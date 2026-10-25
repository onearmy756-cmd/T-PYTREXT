/**
 * useLiveRtc — WebRTC halisi (mesh) kwa madarasa ya ELIMUPRO.
 *
 * Signaling hupitia WS ya kikao (`/api/live/session/:id/ws`) — hiyo ni relay ya
 * offer/answer/ICE. Vyumba vya LAN hutumia host candidates (hakuna internet).
 * Mshiriki mwenye `peer` ndogo husukuma offer (perfect-negotiation nyepesi).
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { liveService, liveWsUrl, type LvPeer } from "#/api/education-service/live.api";

const ICE: RTCIceServer[] = [
  { urls: "stun:stun.l.google.com:19302" },
];

export interface LiveRtcState {
  peers: LvPeer[];
  streams: Record<string, MediaStream>;
  connected: string[];
  micOn: boolean;
  error: string;
  toggleMic: () => void;
  closeAll: () => void;
}

export function useLiveRtc(
  sessionId: string,
  myPeer: string,
  localStream: MediaStream | null,
  onEvent?: (v: Record<string, unknown>) => void,
): LiveRtcState {
  const [peers, setPeers] = useState<LvPeer[]>([]);
  const [streams, setStreams] = useState<Record<string, MediaStream>>({});
  const [connected, setConnected] = useState<string[]>([]);
  const [micOn, setMicOn] = useState(true);
  const [error, setError] = useState("");

  const pcs = useRef<Map<string, RTCPeerConnection>>(new Map());
  const pendingIce = useRef<Map<string, RTCIceCandidateInit[]>>(new Map());
  const seenSignals = useRef<Set<string>>(new Set());
  const wsRef = useRef<WebSocket | null>(null);
  const localRef = useRef<MediaStream | null>(null);
  const onEventRef = useRef(onEvent);
  onEventRef.current = onEvent;

  const closeAll = useCallback(() => {
    pcs.current.forEach((pc) => {
      try {
        pc.close();
      } catch {
        /* imefungwa */
      }
    });
    pcs.current.clear();
    pendingIce.current.clear();
    seenSignals.current.clear();
    setStreams({});
    setConnected([]);
  }, []);

  const send = useCallback((v: Record<string, unknown>) => {
    const ws = wsRef.current;
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(v));
    }
  }, []);

  const flushIce = useCallback(async (pc: RTCPeerConnection, from: string) => {
    const q = pendingIce.current.get(from);
    if (!q || q.length === 0) return;
    pendingIce.current.delete(from);
    for (const c of q) {
      try {
        await pc.addIceCandidate(new RTCIceCandidate(c));
      } catch {
        /* candidate imeshindwa */
      }
    }
  }, []);

  const ensurePc = useCallback(
    (remote: string): RTCPeerConnection => {
      const existing = pcs.current.get(remote);
      if (existing) return existing;
      const pc = new RTCPeerConnection({ iceServers: ICE });
      pcs.current.set(remote, pc);
      localRef.current?.getTracks().forEach((t) => {
        try {
          pc.addTrack(t, localRef.current!);
        } catch {
          /* track imeshindwa */
        }
      });
      pc.onicecandidate = (e) => {
        if (e.candidate) {
          void liveService
            .signal(sessionId, {
              from_peer: myPeer,
              to_peer: remote,
              kind: "ice",
              payload: e.candidate.toJSON(),
            })
            .catch(() => {});
        }
      };
      pc.ontrack = (e) => {
        const [stream] = e.streams;
        if (stream) setStreams((prev) => ({ ...prev, [remote]: stream }));
      };
      pc.onconnectionstatechange = () => {
        if (pc.connectionState === "connected") {
          setConnected((prev) => (prev.includes(remote) ? prev : [...prev, remote]));
        } else if (
          pc.connectionState === "failed" ||
          pc.connectionState === "closed" ||
          pc.connectionState === "disconnected"
        ) {
          setConnected((prev) => prev.filter((p) => p !== remote));
        }
      };
      return pc;
    },
    [sessionId, myPeer],
  );

  const makeOffer = useCallback(
    async (remote: string) => {
      try {
        const pc = ensurePc(remote);
        const offer = await pc.createOffer();
        await pc.setLocalDescription(offer);
        await liveService.signal(sessionId, {
          from_peer: myPeer,
          to_peer: remote,
          kind: "offer",
          payload: offer,
        });
      } catch (e) {
        setError(String(e));
      }
    },
    [ensurePc, sessionId, myPeer],
  );

  // Sauti ya ndani: weka ref KABLA ya signaling (effects hupangwa kwa mfuatano).
  useEffect(() => {
    localRef.current = localStream;
    if (!localStream) return;
    pcs.current.forEach((pc) => {
      const senders = pc.getSenders();
      let added = false;
      localStream.getTracks().forEach((t) => {
        if (!senders.some((s) => s.track === t)) {
          try {
            pc.addTrack(t, localStream);
            added = true;
          } catch {
            /* track imeshindwa */
          }
        }
      });
      // Track iliongezwa baada ya offer → renegotiate ili sauti ipite.
      if (added && pc.signalingState === "stable") {
        void pc
          .createOffer()
          .then((o) => pc.setLocalDescription(o))
          .then(() => {
            const remote = Array.from(pcs.current.entries()).find(([, x]) => x === pc)?.[0];
            if (remote) {
              void liveService.signal(sessionId, {
                from_peer: myPeer,
                to_peer: remote,
                kind: "offer",
                payload: pc.localDescription,
              });
            }
          })
          .catch(() => {});
      }
    });
  }, [localStream, sessionId, myPeer]);

  // Signaling: WS + polling fallback (WS ikiwa imefungwa).
  useEffect(() => {
    if (!sessionId || !myPeer) return;
    closeAll();
    let ws: WebSocket | null = null;
    let poll: ReturnType<typeof setInterval> | null = null;

    const handleSignal = async (from: string, kind: string, payload: unknown) => {
      try {
        const pc = ensurePc(from);
        if (kind === "offer") {
          // Perfect-negotiation nyepesi: mwenye peer ndogo HUANZISHA offer ya
          // kwanza; mwingine huipuuza ili kuepuka mgongano (glare). Offer za
          // baadaye (renegotiation) zinakubaliwa.
          const hasRemote = Boolean(pc.remoteDescription && pc.remoteDescription.type);
          if (!hasRemote && myPeer < from) return;
          await pc.setRemoteDescription(
            new RTCSessionDescription(payload as RTCSessionDescriptionInit),
          );
          await flushIce(pc, from);
          const answer = await pc.createAnswer();
          await pc.setLocalDescription(answer);
          await liveService.signal(sessionId, {
            from_peer: myPeer,
            to_peer: from,
            kind: "answer",
            payload: answer,
          });
        } else if (kind === "answer") {
          if (pc.signalingState !== "stable") {
            await pc.setRemoteDescription(
              new RTCSessionDescription(payload as RTCSessionDescriptionInit),
            );
            await flushIce(pc, from);
          }
        } else if (kind === "ice") {
          const cand = payload as RTCIceCandidateInit;
          // ICE inaweza kufika kabla ya remote description — hifadhi kwanza.
          if (!pc.remoteDescription || !pc.remoteDescription.type) {
            const q = pendingIce.current.get(from) ?? [];
            q.push(cand);
            pendingIce.current.set(from, q);
          } else {
            await pc.addIceCandidate(new RTCIceCandidate(cand));
          }
        }
      } catch (e) {
        setError(String(e));
      }
    };

    const onMessage = (raw: string) => {
      let v: Record<string, unknown>;
      try {
        v = JSON.parse(raw) as Record<string, unknown>;
      } catch {
        return;
      }
      onEventRef.current?.(v);
      const type = String(v.type ?? "");
      if (type === "signal") {
        const to = String(v.to ?? "");
        const sid = String(v.id ?? "");
        if (to === myPeer) {
          if (sid && seenSignals.current.has(sid)) return;
          if (sid) seenSignals.current.add(sid);
          void handleSignal(String(v.from ?? ""), String(v.kind ?? ""), v.payload);
        }
      } else if (type === "peer_join") {
        const other = String(v.peer ?? "");
        setPeers((prev) =>
          prev.some((p) => p.peer === other)
            ? prev
            : [...prev, { peer: other, name: String(v.name ?? ""), role: String(v.role ?? "") }],
        );
        // Peer mdogo husukuma offer (epuka offer zote mbili).
        if (other && myPeer < other) void makeOffer(other);
      } else if (type === "peer_leave") {
        const other = String(v.peer ?? "");
        const pc = pcs.current.get(other);
        if (pc) {
          try {
            pc.close();
          } catch {
            /* imefungwa */
          }
          pcs.current.delete(other);
        }
        setPeers((prev) => prev.filter((p) => p.peer !== other));
        setStreams((prev) => {
          const n = { ...prev };
          delete n[other];
          return n;
        });
      }
    };

    const refreshPeers = async () => {
      try {
        const r = await liveService.peers(sessionId);
        setPeers(r.peers);
        // Kwa kila peer mpya, mwenye id ndogo huanzisha.
        r.peers.forEach((p) => {
          if (p.peer !== myPeer && myPeer < p.peer && !pcs.current.has(p.peer)) {
            void makeOffer(p.peer);
          }
        });
      } catch {
        /* hakuna peers */
      }
    };

    const drain = async () => {
      try {
        const r = await liveService.signals(sessionId, myPeer);
        for (const s of r.signals) {
          if (s.id) {
            if (seenSignals.current.has(s.id)) continue;
            seenSignals.current.add(s.id);
          }
          await handleSignal(s.from, s.kind, s.payload);
        }
      } catch {
        /* hakuna signals */
      }
    };

    try {
      ws = new WebSocket(liveWsUrl(sessionId));
      wsRef.current = ws;
      ws.onmessage = (ev) => onMessage(String(ev.data));
      ws.onerror = () => setError("WS ya kikao imeshindwa");
      ws.onopen = () => {
        void refreshPeers();
      };
      ws.onclose = () => {
        wsRef.current = null;
      };
    } catch {
      setError("WebSocket haipatikani — natumia polling");
    }

    // Polling HAIKATIKI: signaling iliyotumwa kabla ya WS kusubscribe (au
    // iliyopotea kwenye broadcast) hupatikana hapa (DB drain).
    poll = setInterval(() => {
      void drain();
      void refreshPeers();
    }, 2000);

    void refreshPeers();
    return () => {
      if (poll) clearInterval(poll);
      try {
        ws?.close();
      } catch {
        /* imefungwa */
      }
      wsRef.current = null;
      closeAll();
    };
  }, [sessionId, myPeer, ensurePc, makeOffer, closeAll]);

  // Sauti ya ndani: kufuatilia mkondo.
  useEffect(() => {
    localRef.current = localStream;
    if (!localStream) return;
    pcs.current.forEach((pc) => {
      const senders = pc.getSenders();
      localStream.getTracks().forEach((t) => {
        if (!senders.some((s) => s.track === t)) {
          try {
            pc.addTrack(t, localStream);
          } catch {
            /* track imeshindwa */
          }
        }
      });
    });
  }, [localStream]);

  const toggleMic = useCallback(() => {
    const st = localRef.current;
    if (!st) return;
    const next = !micOn;
    st.getAudioTracks().forEach((t) => {
      t.enabled = next;
    });
    setMicOn(next);
  }, [micOn]);

  return { peers, streams, connected, micOn, error, toggleMic, closeAll };
}

/** Husaidia kupata sauti ya kipaza sauti (mic) kwa somo la moja kwa moja. */
export async function getMicStream(): Promise<MediaStream | null> {
  try {
    return await navigator.mediaDevices.getUserMedia({ audio: true, video: false });
  } catch {
    return null;
  }
}

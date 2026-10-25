import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Mic,
  MicOff,
  Play,
  Plus,
  Radio,
  RefreshCw,
  Sparkles,
  Square,
  Upload,
  Users,
  Wand2,
} from "lucide-react";
import {
  liveService,
  recordingUrl,
  type LvLine,
  type LvSession,
  type LvSlide,
  type LvStatus,
} from "#/api/education-service/live.api";
import { elimuproService, type EpLevel, type EpSubject } from "#/api/education-service/elimupro.api";
import { getMicStream, useLiveRtc } from "./use-live-rtc";
import {
  EduBadge,
  EduButton,
  EduCard,
  EduEmpty,
  EduError,
  EduField,
  EduInput,
  EduSelect,
} from "./edu-ui";

/** Aina ndogo ya Web Speech API (si kwenye TS lib ya kawaida). */
type SpeechCtor = new () => {
  lang: string;
  continuous: boolean;
  interimResults: boolean;
  onresult: ((e: { resultIndex: number; results: { length: number; [i: number]: { isFinal: boolean; 0: { transcript: string } } } }) => void) | null;
  onend: (() => void) | null;
  start: () => void;
  stop: () => void;
};

function speechCtor(): SpeechCtor | null {
  const w = window as unknown as { SpeechRecognition?: SpeechCtor; webkitSpeechRecognition?: SpeechCtor };
  return w.SpeechRecognition ?? w.webkitSpeechRecognition ?? null;
}

/**
 * ELIMUPRO — Awamu 3 & 4: madarasa ya moja kwa moja na uwasilishaji.
 * WebRTC halisi (mesh), AI Scribe, muhtasari wa AI, slides, alama.
 */
export function LiveTab() {
  const [status, setStatus] = useState<LvStatus | null>(null);
  const [sessions, setSessions] = useState<LvSession[]>([]);
  const [active, setActive] = useState<LvSession | null>(null);
  const [err, setErr] = useState("");
  const [kind, setKind] = useState<"live" | "presentation">("live");

  const refresh = useCallback(async () => {
    try {
      setStatus(await liveService.status());
      setSessions((await liveService.sessions()).sessions);
    } catch (e) {
      setErr(String(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  if (active) {
    return (
      <RoomView
        session={active}
        onExit={() => {
          setActive(null);
          void refresh();
        }}
      />
    );
  }

  return (
    <div data-testid="live" className="flex flex-col gap-4">
      <EduCard testId="live-status">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <Radio className="size-5 text-primary" aria-hidden />
            <span className="text-sm font-semibold text-content">
              Madarasa ya moja kwa moja (Awamu 3)
            </span>
          </div>
          <div className="flex flex-wrap gap-1">
            <EduBadge>{status?.sessions ?? 0} vikao</EduBadge>
            <EduBadge>{status?.live_now ?? 0} live sasa</EduBadge>
            <EduBadge>{status?.participants ?? 0} washiriki</EduBadge>
            <EduBadge>{status?.slides ?? 0} slides</EduBadge>
            <EduButton variant="ghost" onClick={() => void refresh()}>
              <RefreshCw className="size-3.5" aria-hidden />
            </EduButton>
          </div>
        </div>
      </EduCard>

      <CreateSession
        kind={kind}
        setKind={setKind}
        onCreated={() => {
          void refresh();
        }}
      />

      {err ? <EduError message={err} /> : null}

      <EduCard>
        <div className="flex flex-col gap-2">
          <span className="text-sm font-semibold text-content">Vikao ({sessions.length})</span>
          {sessions.length === 0 ? (
            <EduEmpty>Hakuna vikao bado. Unda kikao kipya juu.</EduEmpty>
          ) : (
            sessions.map((s) => (
              <button
                key={s.id}
                type="button"
                data-testid={`live-item-${s.id}`}
                onClick={() => setActive(s)}
                className="rounded-md border border-border p-2 text-left transition hover:border-primary/60"
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="text-sm font-medium text-content">{s.title}</span>
                  <div className="flex gap-1">
                    <EduBadge>{s.kind === "presentation" ? "uwasilishaji" : "live"}</EduBadge>
                    <EduBadge>{s.status}</EduBadge>
                    <EduBadge>{s.participants} washiriki</EduBadge>
                  </div>
                </div>
              </button>
            ))
          )}
        </div>
      </EduCard>
    </div>
  );
}

// ------------------------------------------------------------
// Kikao kipya
// ------------------------------------------------------------

function CreateSession({
  kind,
  setKind,
  onCreated,
}: {
  kind: "live" | "presentation";
  setKind: (k: "live" | "presentation") => void;
  onCreated: () => void;
}) {
  const [levels, setLevels] = useState<EpLevel[]>([]);
  const [levelId, setLevelId] = useState("");
  const [subjects, setSubjects] = useState<EpSubject[]>([]);
  const [subjectId, setSubjectId] = useState("");
  const [title, setTitle] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");
  const [msg, setMsg] = useState("");

  useEffect(() => {
    void (async () => {
      try {
        const r = await elimuproService.levels("");
        setLevels(r.levels);
        if (r.levels[0]) setLevelId(r.levels[0].id);
      } catch {
        /* hakuna levels */
      }
    })();
  }, []);

  useEffect(() => {
    if (!levelId) return;
    void (async () => {
      try {
        const r = await elimuproService.subjects(levelId);
        setSubjects(r.subjects);
        setSubjectId(r.subjects[0]?.id ?? "");
      } catch {
        setSubjects([]);
      }
    })();
  }, [levelId]);

  const create = async () => {
    if (!title.trim()) {
      setErr("Kichwa cha kikao kinahitajika.");
      return;
    }
    setBusy(true);
    setErr("");
    setMsg("");
    try {
      const r = await liveService.createSession({
        title: title.trim(),
        subject_id: subjectId,
        kind,
      });
      setMsg(`Kikao kimeundwa (${r.kind}). Fungua kuianza.`);
      setTitle("");
      onCreated();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <EduCard>
      <div className="grid gap-3 md:grid-cols-2">
        <EduField label="Aina">
          <EduSelect
            testId="lv-kind"
            value={kind}
            onChange={(e) => setKind(e.target.value as "live" | "presentation")}
          >
            <option value="live">Somo la moja kwa moja</option>
            <option value="presentation">Uwasilishaji</option>
          </EduSelect>
        </EduField>
        <EduField label="Level">
          <EduSelect
            testId="lv-level"
            value={levelId}
            onChange={(e) => setLevelId(e.target.value)}
          >
            {levels.map((l) => (
              <option key={l.id} value={l.id}>
                {l.name}
              </option>
            ))}
          </EduSelect>
        </EduField>
        <EduField label="Somo">
          <EduSelect
            testId="lv-subject"
            value={subjectId}
            onChange={(e) => setSubjectId(e.target.value)}
          >
            {subjects.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </EduSelect>
        </EduField>
        <EduField label="Kichwa">
          <EduInput
            testId="lv-title"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="Algoriti — somo la 3"
          />
        </EduField>
        <div className="md:col-span-2 flex flex-col gap-2">
          {err ? <EduError message={err} /> : null}
          {msg ? <p className="text-xs text-primary">{msg}</p> : null}
          <EduButton testId="lv-create" onClick={() => void create()} disabled={busy}>
            <Plus className="size-4" aria-hidden />
            {busy ? "Inaunda..." : "Unda kikao"}
          </EduButton>
        </div>
      </div>
    </EduCard>
  );
}

// ------------------------------------------------------------
// Chumba (room)
// ------------------------------------------------------------

function RoomView({ session, onExit }: { session: LvSession; onExit: () => void }) {
  const [detail, setDetail] = useState<{ status: string; kind: string; recording: string } | null>(null);
  const [myPeer, setMyPeer] = useState("");
  const [mic, setMic] = useState<MediaStream | null>(null);
  const [lines, setLines] = useState<LvLine[]>([]);
  const [slides, setSlides] = useState<LvSlide[]>([]);
  const [current, setCurrent] = useState(0);
  const [summary, setSummary] = useState<unknown>(null);
  const [summaryMode, setSummaryMode] = useState("");
  const [scribe, setScribe] = useState(false);
  const [name, setName] = useState("Mwalimu");
  const [err, setErr] = useState("");
  const [busy, setBusy] = useState(false);
  const recRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);

  const onEvent = useCallback((v: Record<string, unknown>) => {
    if (v.type === "slide_go") setCurrent(Number(v.page ?? 0));
    if (v.type === "transcript") {
      setLines((prev) => [...prev, { speaker: String(v.speaker ?? ""), text: String(v.text ?? ""), at: String(v.at ?? "") }]);
    }
    if (v.type === "status") setDetail((d) => (d ? { ...d, status: String(v.status ?? d.status) } : d));
  }, []);

  const rtc = useLiveRtc(session.id, myPeer, mic, onEvent);

  const load = useCallback(async () => {
    try {
      const r = await liveService.session(session.id);
      setDetail({
        status: r.session.status,
        kind: r.session.kind,
        recording: r.session.recording,
      });
      setCurrent(r.session.current_slide);
      setLines((await liveService.transcript(session.id)).lines);
      setSlides((await liveService.slides(session.id)).slides);
      const s = await liveService.summaryGet(session.id).catch(() => null);
      if (s?.ok) {
        setSummary(s.summary);
        setSummaryMode(s.mode);
      }
    } catch (e) {
      setErr(String(e));
    }
  }, [session.id]);

  useEffect(() => {
    void load();
  }, [load]);

  const join = async () => {
    setBusy(true);
    setErr("");
    try {
      const r = await liveService.join(session.id, {
        name: name.trim() || "Mshiriki",
        role: "teacher",
      });
      setMyPeer(r.peer);
      const stream = await getMicStream();
      setMic(stream);
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const leave = async () => {
    try {
      if (myPeer) await liveService.leave(session.id, myPeer);
    } catch {
      /* tayari ameondoka */
    }
    rtc.closeAll();
    mic?.getTracks().forEach((t) => t.stop());
    setMic(null);
    setMyPeer("");
  };

  const startSession = async () => {
    await liveService.start(session.id);
    await load();
  };

  const endSession = async () => {
    setBusy(true);
    try {
      const r = await liveService.end(session.id);
      setSummaryMode(r.summary?.mode ?? "");
      await load();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const makeSummary = async () => {
    setBusy(true);
    try {
      const r = await liveService.summaryMake(session.id);
      setSummaryMode(r.mode);
      if (r.summary) setSummary(r.summary);
      if (!r.ok) setErr(r.error ?? "LLM haikujibu");
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  // AI Scribe: Web Speech API → transcript (huenda kwa wote kwa WS).
  useEffect(() => {
    if (!scribe || !myPeer) return;
    const Ctor = speechCtor();
    if (!Ctor) {
      setErr("Kivinjari hiki hakitumii Web Speech API — tumia Chrome/Edge.");
      setScribe(false);
      return;
    }
    const rec = new Ctor();
    rec.lang = "sw-TZ";
    rec.continuous = true;
    rec.interimResults = false;
    rec.onresult = (e) => {
      for (let i = e.resultIndex; i < e.results.length; i += 1) {
        const res = e.results[i];
        if (res.isFinal) {
          const text = res[0].transcript.trim();
          if (text) void liveService.transcriptAdd(session.id, { speaker: name || "Mwalimu", text }).catch(() => {});
        }
      }
    };
    rec.onend = () => {
      if (scribe) {
        try {
          rec.start();
        } catch {
          /* imekwisha */
        }
      }
    };
    try {
      rec.start();
    } catch {
      /* imeshindwa kuanza */
    }
    return () => {
      rec.onend = null;
      try {
        rec.stop();
      } catch {
        /* imekwisha */
      }
    };
  }, [scribe, myPeer, session.id, name]);

  const toggleRecord = async () => {
    if (recRef.current) {
      recRef.current.stop();
      recRef.current = null;
      return;
    }
    if (!mic) {
      setErr("Washa mic kwanza ili kurekodi.");
      return;
    }
    const rec = new MediaRecorder(mic);
    chunksRef.current = [];
    rec.ondataavailable = (e) => {
      if (e.data.size > 0) chunksRef.current.push(e.data);
    };
    rec.onstop = async () => {
      const blob = new Blob(chunksRef.current, { type: "video/webm" });
      try {
        await liveService.record(session.id, blob);
        await load();
      } catch (e) {
        setErr(String(e));
      }
    };
    rec.start();
    recRef.current = rec;
  };

  const uploadSlides = async (files: FileList | null) => {
    if (!files) return;
    for (const f of Array.from(files)) {
      const data = await new Promise<string>((resolve) => {
        const fr = new FileReader();
        fr.onload = () => resolve(String(fr.result));
        fr.readAsDataURL(f);
      });
      await liveService.slideAdd(session.id, { title: f.name, data });
    }
    setSlides((await liveService.slides(session.id)).slides);
  };

  const goSlide = async (page: number) => {
    await liveService.slideGo(session.id, page);
    setCurrent(page);
  };

  const liveSlide = useMemo(
    () => slides.find((s) => s.page === current) ?? slides[0] ?? null,
    [slides, current],
  );

  return (
    <div data-testid="live-room" className="flex flex-col gap-4">
      <EduCard testId="live-room-head">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <Radio className="size-5 text-primary" aria-hidden />
            <span className="text-sm font-semibold text-content">{session.title}</span>
            <EduBadge>{detail?.kind === "presentation" ? "uwasilishaji" : "live"}</EduBadge>
            <EduBadge>{detail?.status ?? session.status}</EduBadge>
          </div>
          <div className="flex flex-wrap gap-1">
            <EduButton variant="ghost" testId="live-back" onClick={() => void leave().then(onExit)}>
              Toka
            </EduButton>
            {detail?.status !== "live" ? (
              <EduButton testId="live-start" onClick={() => void startSession()}>
                <Play className="size-3.5" aria-hidden /> Anza
              </EduButton>
            ) : (
              <EduButton testId="live-end" onClick={() => void endSession()} disabled={busy}>
                <Square className="size-3.5" aria-hidden /> Maliza
              </EduButton>
            )}
          </div>
        </div>
      </EduCard>

      {err ? <EduError message={err} /> : null}

      {!myPeer ? (
        <EduCard>
          <div className="flex flex-col gap-3">
            <EduField label="Jina lako">
              <EduInput testId="live-name" value={name} onChange={(e) => setName(e.target.value)} />
            </EduField>
            <EduButton testId="live-join" onClick={() => void join()} disabled={busy}>
              <Users className="size-4" aria-hidden />
              {busy ? "Inajiunga..." : "Jiunge na kikao"}
            </EduButton>
            <p className="text-xs text-muted">
              Sauti hutumwa moja kwa moja kati ya vifaa (WebRTC) — kwenye LAN ya chuo
              haihitaji internet.
            </p>
          </div>
        </EduCard>
      ) : (
        <div className="grid gap-4 lg:grid-cols-2">
          <EduCard testId="live-peers">
            <div className="flex flex-col gap-2">
              <div className="flex items-center justify-between">
                <span className="text-sm font-semibold text-content">
                  Washiriki ({rtc.peers.length})
                </span>
                <div className="flex gap-1">
                  <EduButton variant="ghost" testId="live-mic" onClick={rtc.toggleMic}>
                    {rtc.micOn ? <Mic className="size-3.5" aria-hidden /> : <MicOff className="size-3.5" aria-hidden />}
                  </EduButton>
                  <EduButton variant="ghost" testId="live-record" onClick={() => void toggleRecord()}>
                    <Radio className="size-3.5" aria-hidden />
                  </EduButton>
                </div>
              </div>
              <p className="text-xs text-muted">Peer yako: {myPeer}</p>
              {rtc.peers.length === 0 ? (
                <EduEmpty>Hakuna mshiriki mwingine bado.</EduEmpty>
              ) : (
                rtc.peers
                  .filter((p) => p.peer !== myPeer)
                  .map((p) => (
                    <div key={p.peer} className="flex items-center justify-between rounded-md border border-border p-2">
                      <span className="text-sm text-content">{p.name || p.peer}</span>
                      <EduBadge>{rtc.connected.includes(p.peer) ? "imeunganishwa" : "inaunganisha"}</EduBadge>
                    </div>
                  ))
              )}
              {/* Sauti ya wenzake (WebRTC) */}
              {Object.entries(rtc.streams).map(([peer, stream]) => (
                <RemoteAudio key={peer} stream={stream} />
              ))}
              {detail?.recording ? (
                <a
                  className="text-xs text-primary underline"
                  href={recordingUrl(session.id)}
                  target="_blank"
                  rel="noreferrer"
                >
                  Pakua recording
                </a>
              ) : null}
            </div>
          </EduCard>

          <EduCard testId="live-scribe">
            <div className="flex flex-col gap-2">
              <div className="flex items-center justify-between">
                <span className="text-sm font-semibold text-content">AI Scribe</span>
                <EduButton
                  variant="ghost"
                  testId="live-scribe-toggle"
                  onClick={() => setScribe((v) => !v)}
                >
                  <Sparkles className="size-3.5" aria-hidden />
                  {scribe ? "Zima" : "Washa"}
                </EduButton>
              </div>
              <div
                data-testid="live-transcript"
                className="max-h-56 overflow-y-auto rounded-md border border-border p-2"
              >
                {lines.length === 0 ? (
                  <span className="text-xs text-muted">Transcript itaonekana hapa.</span>
                ) : (
                  lines.map((l, i) => (
                    <div key={`${l.at}-${i}`} className="text-sm">
                      <span className="font-medium text-content">{l.speaker}: </span>
                      <span className="text-muted">{l.text}</span>
                    </div>
                  ))
                )}
              </div>
              <EduButton testId="live-summary" onClick={() => void makeSummary()} disabled={busy}>
                <Wand2 className="size-4" aria-hidden /> Tengeneza muhtasari
              </EduButton>
              {summaryMode ? <EduBadge>muhtasari: {summaryMode}</EduBadge> : null}
              {summary ? (
                <pre
                  data-testid="live-summary-out"
                  className="max-h-40 overflow-auto rounded-md bg-surface-secondary p-2 text-xs text-muted"
                >
                  {JSON.stringify(summary, null, 2)}
                </pre>
              ) : null}
            </div>
          </EduCard>

          {detail?.kind === "presentation" ? (
            <EduCard testId="live-slides" className="lg:col-span-2">
              <div className="flex flex-col gap-2">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <span className="text-sm font-semibold text-content">
                    Slides ({slides.length})
                  </span>
                  <label className="inline-flex cursor-pointer items-center gap-1 rounded-md border border-border px-2 py-1 text-xs text-muted hover:text-content">
                    <Upload className="size-3.5" aria-hidden /> Ongeza slides
                    <input
                      data-testid="live-slide-upload"
                      type="file"
                      accept="image/*,application/pdf"
                      multiple
                      className="hidden"
                      onChange={(e) => void uploadSlides(e.target.files)}
                    />
                  </label>
                </div>
                {liveSlide ? (
                  <div className="flex flex-col items-center gap-2">
                    {liveSlide.data.startsWith("data:image") ? (
                      <img
                        data-testid="live-slide-img"
                        src={liveSlide.data}
                        alt={liveSlide.title}
                        className="max-h-80 rounded-md border border-border"
                      />
                    ) : (
                      <div className="w-full rounded-md border border-border p-3 text-sm text-muted">
                        {liveSlide.title || liveSlide.data}
                      </div>
                    )}
                    <div className="flex flex-wrap gap-1">
                      {slides.map((s) => (
                        <button
                          key={s.page}
                          type="button"
                          data-testid={`live-slide-${s.page}`}
                          onClick={() => void goSlide(s.page)}
                          className={
                            "rounded-md border px-2 py-1 text-xs " +
                            (s.page === current ? "border-primary text-primary" : "border-border text-muted")
                          }
                        >
                          {s.page}
                        </button>
                      ))}
                    </div>
                  </div>
                ) : (
                  <EduEmpty>Hakuna slides bado. Ongeza picha za slides.</EduEmpty>
                )}
              </div>
            </EduCard>
          ) : null}
        </div>
      )}
    </div>
  );
}

function RemoteAudio({ stream }: { stream: MediaStream }) {
  const ref = useRef<HTMLAudioElement | null>(null);
  useEffect(() => {
    if (ref.current) ref.current.srcObject = stream;
  }, [stream]);
  return <audio ref={ref} autoPlay data-testid="live-audio" />;
}

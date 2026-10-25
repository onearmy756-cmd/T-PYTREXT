/**
 * useProctor — ulinzi halisi wa mtihani (ELIMUPRO Awamu 5).
 *
 * Hukamata matukio YA KWELI kwenye kivinjari:
 *   - copy / paste / cut → huzuiwa na kuripotiwa
 *   - contextmenu (right-click) → huzuiwa
 *   - devtools / print screen keys → huripotiwa
 *   - kubadilisha dirisha (visibilitychange) + blur → huripotiwa
 *   - kutoka fullscreen → huripotiwa
 *   - picha za mwanafunzi kila `photoEveryMs` (getUserMedia)
 *   - kiwango cha sauti (mic) — kelele hurekodiwa
 *   - AI suspect: maandishi yanayofanana na mtindo wa AI (hoja pekee)
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { proctorService } from "#/api/education-service/proctor.api";

export interface ProctorState {
  violations: number;
  events: { kind: string; detail: string; at: string }[];
  photos: number;
  audioLevel: number;
  cameraOn: boolean;
  micOn: boolean;
  report: (kind: string, detail?: string) => void;
}

export function useProctor(
  attemptId: string,
  enabled: boolean,
  opts?: { photoEveryMs?: number; onFlag?: (kind: string) => void },
): ProctorState {
  const [violations, setViolations] = useState(0);
  const [events, setEvents] = useState<{ kind: string; detail: string; at: string }[]>([]);
  const [photos, setPhotos] = useState(0);
  const [audioLevel, setAudioLevel] = useState(0);
  const [cameraOn, setCameraOn] = useState(false);
  const [micOn, setMicOn] = useState(false);

  const lastRef = useRef<Record<string, number>>({});
  const onFlagRef = useRef(opts?.onFlag);
  onFlagRef.current = opts?.onFlag;

  const report = useCallback(
    (kind: string, detail = "") => {
      if (!attemptId || !enabled) return;
      // Zuia mafuriko: kila aina angalau sekunde 3 tofauti.
      const t = Date.now();
      if (t - (lastRef.current[kind] ?? 0) < 3000) return;
      lastRef.current[kind] = t;
      setEvents((prev) => [...prev, { kind, detail, at: new Date().toISOString() }]);
      onFlagRef.current?.(kind);
      void proctorService
        .addEvent(attemptId, kind, detail)
        .then((r) => setViolations(r.violations))
        .catch(() => {});
    },
    [attemptId, enabled],
  );

  // Anti-copy / anti-AI: copy, paste, cut, right-click.
  useEffect(() => {
    if (!attemptId || !enabled) return;
    const block = (kind: string) => (e: Event) => {
      e.preventDefault();
      report(kind, "imezuiwa kwenye dirisha la mtihani");
    };
    const onCopy = block("copy");
    const onPaste = block("paste");
    const onCut = block("cut");
    const onCtx = block("contextmenu");
    document.addEventListener("copy", onCopy);
    document.addEventListener("paste", onPaste);
    document.addEventListener("cut", onCut);
    document.addEventListener("contextmenu", onCtx);
    return () => {
      document.removeEventListener("copy", onCopy);
      document.removeEventListener("paste", onPaste);
      document.removeEventListener("cut", onCut);
      document.removeEventListener("contextmenu", onCtx);
    };
  }, [attemptId, enabled, report]);

  // DevTools / print keys.
  useEffect(() => {
    if (!attemptId || !enabled) return;
    const onKey = (e: KeyboardEvent) => {
      const k = e.key.toUpperCase();
      if (k === "F12") {
        e.preventDefault();
        report("devtools", "F12");
      } else if ((e.ctrlKey || e.metaKey) && e.shiftKey && ["I", "J", "C"].includes(k)) {
        e.preventDefault();
        report("devtools", `Ctrl+Shift+${k}`);
      } else if ((e.ctrlKey || e.metaKey) && k === "P") {
        e.preventDefault();
        report("devtools", "Ctrl+P");
      }
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [attemptId, enabled, report]);

  // Kutoka dirisha (kama kuangalia kando) + fullscreen.
  useEffect(() => {
    if (!attemptId || !enabled) return;
    const onVis = () => {
      if (document.hidden) report("focus_lost", "dirisha limefichwa");
    };
    const onBlur = () => report("focus_lost", "dirisha haliko mbele");
    const onFs = () => {
      if (!document.fullscreenElement) report("fullscreen_exit", "umetoka fullscreen");
    };
    document.addEventListener("visibilitychange", onVis);
    window.addEventListener("blur", onBlur);
    document.addEventListener("fullscreenchange", onFs);
    return () => {
      document.removeEventListener("visibilitychange", onVis);
      window.removeEventListener("blur", onBlur);
      document.removeEventListener("fullscreenchange", onFs);
    };
  }, [attemptId, enabled, report]);

  // Picha (camera) + sauti (mic) — hukamatwa kila mara.
  useEffect(() => {
    if (!attemptId || !enabled) return;
    let stream: MediaStream | null = null;
    let timer: ReturnType<typeof setInterval> | null = null;
    let raf = 0;
    let audioCtx: AudioContext | null = null;

    void (async () => {
      try {
        stream = await navigator.mediaDevices.getUserMedia({
          video: { width: 320, height: 240 },
          audio: true,
        });
        setCameraOn(stream.getVideoTracks().length > 0);
        setMicOn(stream.getAudioTracks().length > 0);

        const video = document.createElement("video");
        video.srcObject = stream;
        video.muted = true;
        await video.play().catch(() => {});

        const canvas = document.createElement("canvas");
        canvas.width = 320;
        canvas.height = 240;
        const every = opts?.photoEveryMs ?? 30000;
        const snap = () => {
          const ctx = canvas.getContext("2d");
          if (!ctx) return;
          ctx.drawImage(video, 0, 0, canvas.width, canvas.height);
          const data = canvas.toDataURL("image/jpeg", 0.5);
          setPhotos((n) => n + 1);
          void proctorService.addEvent(attemptId, "photo", "picha ya mara kwa mara", data).catch(() => {});
        };
        // Picha ya kwanza mara moja (uthibitisho wa utambulisho), kisha kila `every`.
        snap();
        timer = setInterval(snap, every);

        // Kipimo cha sauti halisi.
        audioCtx = new AudioContext();
        const src = audioCtx.createMediaStreamSource(stream);
        const an = audioCtx.createAnalyser();
        an.fftSize = 512;
        src.connect(an);
        const buf = new Uint8Array(an.frequencyBinCount);
        const tick = () => {
          an.getByteFrequencyData(buf);
          let sum = 0;
          for (const v of buf) sum += v;
          const lvl = sum / buf.length / 255;
          setAudioLevel(lvl);
          if (lvl > 0.45) report("audio", "kelele kubwa");
          raf = requestAnimationFrame(tick);
        };
        tick();
      } catch {
        /* hakuna camera/mic */
      }
    })();

    return () => {
      if (timer) clearInterval(timer);
      if (raf) cancelAnimationFrame(raf);
      if (audioCtx) void audioCtx.close().catch(() => {});
      stream?.getTracks().forEach((t) => t.stop());
    };
  }, [attemptId, enabled, opts?.photoEveryMs, report]);

  return { violations, events, photos, audioLevel, cameraOn, micOn, report };
}

/** Hukagua kama maandishi yana alama za mtindo wa AI (hoja, si hukumu). */
export function aiStyleSuspect(text: string): boolean {
  if (text.trim().length < 240) return false;
  const markers = [
    "kwa ufupi",
    "kwa kumalizia",
    "kwanza,",
    "pili,",
    "tatu,",
    "kwa hivyo,",
    "inashauriwa",
    "muhimu kuzingatia",
    "hapa kuna",
    "hatua zifuatazo",
  ];
  const lower = text.toLowerCase();
  const hits = markers.filter((m) => lower.includes(m)).length;
  const bullets = (text.match(/^\s*[-*•\d]+[.)]?\s/gm) ?? []).length;
  return hits >= 2 || bullets >= 6;
}

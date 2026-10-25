import { useCallback, useEffect, useRef, useState } from "react";
import { Mic, Square, Sparkles, FileText, Trash2, Upload } from "lucide-react";
import {
  scribeService,
  denoiseAudio,
  diarizeAudio,
  blobToPcm16k,
  pcmToWav,
  type DenoiseResult,
  type DiarizeResult,
  type ScribeJob,
  type ScribeProcessResult,
  type ScribeStatus,
} from "#/api/education-service/scribe.api";
import {
  EduBadge,
  EduButton,
  EduCard,
  EduEmpty,
  EduError,
  EduField,
  EduInput,
  EduSelect,
  EduSpinner,
  EduTextarea,
} from "./edu-ui";

/**
 * MTECHOS — Sehemu 1: AI Scribe PRO (moduli 21).
 * Kuchuja sauti (VAD + spectral subtraction halisi), kukili (Web Speech API),
 * uchambuzi wa agentic (SOAP / dakika / notes) na PDF safi ya kurasa nyingi.
 */
export function ScribeProTab() {
  const [status, setStatus] = useState<ScribeStatus | null>(null);
  const [err, setErr] = useState("");
  const [busy, setBusy] = useState(false);

  const [title, setTitle] = useState("");
  const [mode, setMode] = useState("meeting");
  const [language, setLanguage] = useState("Kiswahili");
  const [transcript, setTranscript] = useState("");
  const [recording, setRecording] = useState(false);
  const [listening, setListening] = useState(false);

  const [denoise, setDenoise] = useState<DenoiseResult | null>(null);
  const [cleanUrl, setCleanUrl] = useState("");
  const [diar, setDiar] = useState<DiarizeResult | null>(null);
  const [diarSpeakers, setDiarSpeakers] = useState(0);
  const [lastWav, setLastWav] = useState<Blob | null>(null);
  const [result, setResult] = useState<ScribeProcessResult | null>(null);
  const [jobs, setJobs] = useState<ScribeJob[]>([]);

  const recorderRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  const recogRef = useRef<{ start: () => void; stop: () => void } | null>(null);

  const refresh = useCallback(async () => {
    try {
      setStatus(await scribeService.status());
      setJobs((await scribeService.jobs()).jobs);
      setErr("");
    } catch (e) {
      setErr(String(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  /** Hutenganisha wazungumzaji kwa sauti iliyosafishwa (backend: MFCC + F0 + k-means). */
  const runDiarize = useCallback(async (wav: Blob, speakers: number) => {
    try {
      const d = await diarizeAudio(wav, 16000, speakers);
      setDiar(d);
      setErr("");
    } catch (e) {
      setErr(`Utenganishaji wa wazungumzaji umeshindwa: ${e}`);
    }
  }, []);

  /** Kurekodi sauti, kisha kuisafisha kwa DSP halisi ya backend. */
  const startRecording = async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const rec = new MediaRecorder(stream);
      chunksRef.current = [];
      rec.ondataavailable = (e) =>
        e.data.size > 0 && chunksRef.current.push(e.data);
      rec.onstop = async () => {
        stream.getTracks().forEach((t) => t.stop());
        const blob = new Blob(chunksRef.current, { type: "audio/webm" });
        setBusy(true);
        try {
          const pcm = await blobToPcm16k(blob);
          const wav = pcmToWav(pcm, 16000);
          setLastWav(wav);
          const d = await denoiseAudio(wav, 16000, 0.8);
          setDenoise(d);
          setDiar(d.diarization ?? null);
          const bytes = Uint8Array.from(atob(d.wav_base64), (c) =>
            c.charCodeAt(0),
          );
          const url = URL.createObjectURL(
            new Blob([bytes], { type: "audio/wav" }),
          );
          setCleanUrl((old) => {
            if (old) URL.revokeObjectURL(old);
            return url;
          });
          setErr("");
        } catch (e) {
          setErr(`Usafishaji umeshindwa: ${e}`);
        } finally {
          setBusy(false);
        }
      };
      rec.start();
      recorderRef.current = rec;
      setRecording(true);
      setErr("");
    } catch (e) {
      setErr(`Kivinjari hakiruhusiwi kurekodi: ${e}`);
    }
  };

  const stopRecording = () => {
    recorderRef.current?.stop();
    recorderRef.current = null;
    setRecording(false);
  };

  /** Kunakili kwa Web Speech API (bila seva) — huongeza kwenye transcript. */
  const toggleDictation = () => {
    type SR = {
      new (): {
        lang: string;
        continuous: boolean;
        interimResults: boolean;
        onresult: (e: {
          resultIndex: number;
          results: {
            length: number;
            [i: number]: { isFinal: boolean; 0: { transcript: string } };
          };
        }) => void;
        start: () => void;
        stop: () => void;
      };
    };
    const w = window as unknown as {
      SpeechRecognition?: SR;
      webkitSpeechRecognition?: SR;
    };
    const Ctor = w.SpeechRecognition || w.webkitSpeechRecognition;
    if (!Ctor) {
      setErr(
        "Kivinjari hiki hakina Web Speech API. Andika maandishi kwa mkono au tumia Chrome.",
      );
      return;
    }
    if (listening) {
      recogRef.current?.stop();
      setListening(false);
      return;
    }
    const r = new Ctor();
    r.lang = language.toLowerCase().startsWith("kisw") ? "sw-KE" : "en-US";
    r.continuous = true;
    r.interimResults = false;
    r.onresult = (e) => {
      let add = "";
      for (let i = e.resultIndex; i < e.results.length; i += 1) {
        if (e.results[i].isFinal) add += `${e.results[i][0].transcript} `;
      }
      if (add)
        setTranscript(
          (t) => `${t}${t.endsWith(" ") || t === "" ? "" : " "}${add}`,
        );
    };
    r.start();
    recogRef.current = r;
    setListening(true);
    setErr("");
  };

  const process = async () => {
    if (!transcript.trim()) {
      setErr("Weka au nakili maandishi kwanza.");
      return;
    }
    setBusy(true);
    try {
      const r = await scribeService.process({
        transcript: transcript.trim(),
        mode,
        title: title.trim(),
        language,
      });
      setResult(r);
      setErr("");
      await refresh();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const uploadAudio = async (f: File) => {
    setBusy(true);
    try {
      const d = await denoiseAudio(f, 16000, 0.8);
      setDenoise(d);
      setDiar(d.diarization ?? null);
      setLastWav(f);
      const bytes = Uint8Array.from(atob(d.wav_base64), (c) => c.charCodeAt(0));
      const url = URL.createObjectURL(new Blob([bytes], { type: "audio/wav" }));
      setCleanUrl((old) => {
        if (old) URL.revokeObjectURL(old);
        return url;
      });
      setErr("");
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const modeLabel = (m: string) =>
    m === "hospital"
      ? "Hospitali (SOAP)"
      : m === "classroom"
        ? "Darasa (notes)"
        : "Mkutano (dakika)";

  return (
    <div className="flex flex-col gap-3" data-testid="scribepro-tab">
      <EduCard testId="scribepro-status">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="text-sm font-semibold text-content">
            AI Scribe PRO — kuchuja sauti · agentic · PDF safi
          </span>
          <div className="flex flex-wrap gap-1">
            <EduBadge testId="scribepro-llm">
              {status?.agentic.llm ? "LLM ipo" : "LLM haipo"}
            </EduBadge>
            <EduBadge testId="scribepro-jobs">
              {status?.jobs ?? 0} vikao
            </EduBadge>
            <EduBadge testId="scribepro-pdfs">{status?.pdfs ?? 0} PDF</EduBadge>
          </div>
        </div>
        <p className="mt-1 text-xs text-muted">
          DSP halisi: VAD ya nishati + spectral subtraction (FFT radix-2) kwa
          Rust. Kunakili: Web Speech API. Uchambuzi: LLM (au muundo wa
          kimaandishi bila LLM).
        </p>
      </EduCard>

      {err ? <EduError message={err} /> : null}

      <EduCard testId="scribepro-capture">
        <span className="text-sm font-semibold text-content">
          1. Rekodi au pakia sauti
        </span>
        <div className="mt-2 flex flex-wrap items-center gap-2">
          {recording ? (
            <EduButton testId="scribepro-stop" onClick={stopRecording}>
              <Square className="size-4" aria-hidden /> Simamisha
            </EduButton>
          ) : (
            <EduButton
              testId="scribepro-record"
              disabled={busy}
              onClick={() => void startRecording()}
            >
              <Mic className="size-4" aria-hidden /> Rekodi
            </EduButton>
          )}
          <label className="inline-flex cursor-pointer items-center gap-2 rounded-md border border-border px-3 py-1.5 text-sm text-content">
            <Upload className="size-4" aria-hidden /> Pakia WAV
            <input
              data-testid="scribepro-file"
              type="file"
              accept="audio/*"
              className="hidden"
              onChange={(e) => {
                const f = e.target.files?.[0];
                if (f) void uploadAudio(f);
              }}
            />
          </label>
          {busy ? <EduSpinner label="Inachakata…" /> : null}
        </div>
        {denoise ? (
          <div
            className="mt-2 flex flex-col gap-1"
            data-testid="scribepro-denoise"
          >
            <div className="flex flex-wrap gap-1">
              <EduBadge testId="scribepro-speech-ratio">
                sauti {Math.round(denoise.speech_ratio * 100)}%
              </EduBadge>
              <EduBadge testId="scribepro-noise">
                {denoise.noise_removed_db} dB kelele imeondolewa
              </EduBadge>
              <EduBadge>{denoise.duration_s} s</EduBadge>
              <EduBadge>{denoise.sample_rate} Hz</EduBadge>
            </div>
            <p className="text-xs text-muted">{denoise.note_sw}</p>
            {cleanUrl ? (
              <audio
                data-testid="scribepro-clean-audio"
                controls
                src={cleanUrl}
                className="mt-1 w-full"
              />
            ) : null}
          </div>
        ) : null}
        {diar ? (
          <div
            className="mt-3 flex flex-col gap-2"
            data-testid="scribepro-diar"
          >
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-xs font-semibold text-content">
                Wazungumzaji walioaguliwa
              </span>
              <EduBadge testId="scribepro-diar-count">
                {diar.speakers}{" "}
                {diar.speakers === 1 ? "mzungumzaji" : "wazungumzaji"}
              </EduBadge>
              <EduBadge>{diar.turns} zamu</EduBadge>
              <EduBadge>{diar.total_speech_s} s za usemi</EduBadge>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <EduField label="Idadi (0 = igundue yenyewe)">
                <EduInput
                  testId="scribepro-diar-speakers"
                  type="number"
                  min={0}
                  max={6}
                  value={String(diarSpeakers)}
                  onChange={(e) => setDiarSpeakers(Number(e.target.value) || 0)}
                  className="w-24"
                />
              </EduField>
              <EduButton
                testId="scribepro-diar-run"
                disabled={busy || !lastWav}
                onClick={() =>
                  lastWav && void runDiarize(lastWav, diarSpeakers)
                }
              >
                Tenganisha tena
              </EduButton>
            </div>
            <div
              className="flex flex-col gap-1"
              data-testid="scribepro-diar-segments"
            >
              {diar.segments.map((s, i) => (
                <div
                  key={`seg-${i}`}
                  data-testid={`scribepro-diar-seg-${i}`}
                  className="flex items-center gap-2 text-xs text-content"
                >
                  <span className="inline-flex min-w-16 justify-center rounded-md border border-border px-2 py-0.5 font-medium">
                    S{s.speaker}
                  </span>
                  <span className="tabular-nums text-muted">
                    {s.t_start.toFixed(1)}–{s.t_end.toFixed(1)} s
                  </span>
                  <span className="tabular-nums text-muted">
                    ({s.duration_s.toFixed(1)} s)
                  </span>
                </div>
              ))}
            </div>
            <div className="flex flex-wrap gap-1">
              {diar.per_speaker.map((p) => (
                <EduBadge
                  key={`sp-${p.speaker}`}
                  testId={`scribepro-diar-sp-${p.speaker}`}
                >
                  S{p.speaker}: {p.turns} zamu · {p.total_s} s · {p.share_pct}%
                </EduBadge>
              ))}
            </div>
            <p className="text-xs text-muted">{diar.method}</p>
          </div>
        ) : null}
      </EduCard>

      <EduCard testId="scribepro-input">
        <span className="text-sm font-semibold text-content">
          2. Maandishi ya maongezi
        </span>
        <div className="mt-2 grid gap-2 md:grid-cols-3">
          <EduField label="Kichwa">
            <EduInput
              testId="scribepro-title"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="mf. Mkutano wa wazazi"
            />
          </EduField>
          <EduField label="Aina ya kikao">
            <EduSelect
              testId="scribepro-mode"
              value={mode}
              onChange={(e) => setMode(e.target.value)}
            >
              <option value="meeting">Mkutano (dakika)</option>
              <option value="hospital">Hospitali (SOAP)</option>
              <option value="classroom">Darasa (notes)</option>
            </EduSelect>
          </EduField>
          <EduField label="Lugha">
            <EduInput
              testId="scribepro-language"
              value={language}
              onChange={(e) => setLanguage(e.target.value)}
            />
          </EduField>
        </div>
        <div className="mt-2">
          <EduField
            label="Manukuu"
            hint="Rekodi kisha nakili, au andika moja kwa moja. Kwa wazungumzaji tumia `Jina: maandishi`."
          >
            <EduTextarea
              testId="scribepro-transcript"
              value={transcript}
              onChange={(e) => setTranscript(e.target.value)}
              placeholder={
                "Daktari: Habari, unaumwa nini?\nMgonjwa: Naumwa kichwa siku tatu."
              }
              className="min-h-40"
            />
          </EduField>
        </div>
        <div className="mt-2 flex flex-wrap items-center gap-2">
          <EduButton testId="scribepro-dictate" onClick={toggleDictation}>
            <Mic className="size-4" aria-hidden />{" "}
            {listening ? "Acha kunakili" : "Nakili kwa sauti"}
          </EduButton>
          <EduButton
            testId="scribepro-process"
            disabled={busy}
            onClick={() => void process()}
          >
            <Sparkles className="size-4" aria-hidden /> Changanua
          </EduButton>
        </div>
      </EduCard>

      {result ? (
        <EduCard testId="scribepro-result">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <span className="text-sm font-semibold text-content">
              Matokeo — {modeLabel(result.mode)}
            </span>
            <div className="flex flex-wrap items-center gap-1">
              <EduBadge testId="scribepro-engine">{result.engine}</EduBadge>
              <EduBadge>{result.speakers.length} wazungumzaji</EduBadge>
              <a
                data-testid="scribepro-pdf"
                href={scribeService.pdfUrl(result.id)}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-1 rounded-md border border-border px-3 py-1.5 text-sm text-content"
              >
                <FileText className="size-4" aria-hidden /> PDF safi
              </a>
            </div>
          </div>
          <div className="mt-2 flex flex-col gap-2">
            {Object.entries(result.analysis).map(([k, v]) => (
              <div key={k} data-testid={`scribepro-a-${k}`}>
                <span className="text-xs font-semibold uppercase text-muted">
                  {k}
                </span>
                {Array.isArray(v) ? (
                  <ul className="ml-4 list-disc text-sm text-content">
                    {v.map((x, i) => (
                      <li key={`${k}-${i}`}>
                        {typeof x === "object" ? JSON.stringify(x) : String(x)}
                      </li>
                    ))}
                  </ul>
                ) : (
                  <p className="text-sm text-content">{String(v)}</p>
                )}
              </div>
            ))}
          </div>
          <p className="mt-2 text-xs text-muted">{result.note_sw}</p>
        </EduCard>
      ) : null}

      <EduCard testId="scribepro-history">
        <span className="text-sm font-semibold text-content">
          Vikao vilivyohifadhiwa ({jobs.length})
        </span>
        <div className="mt-2 flex flex-col gap-1">
          {jobs.length === 0 ? (
            <EduEmpty>Hakuna vikao bado.</EduEmpty>
          ) : (
            jobs.map((j) => (
              <div
                key={j.id}
                data-testid={`scribepro-job-${j.id}`}
                className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2"
              >
                <div className="min-w-0">
                  <div className="text-sm text-content">{j.title}</div>
                  <div className="truncate text-xs text-muted">{j.preview}</div>
                </div>
                <div className="flex items-center gap-1">
                  <EduBadge>{j.mode}</EduBadge>
                  <EduBadge>{j.engine}</EduBadge>
                  <a
                    data-testid={`scribepro-job-pdf-${j.id}`}
                    href={scribeService.pdfUrl(j.id)}
                    target="_blank"
                    rel="noreferrer"
                    className="rounded-md border border-border px-2 py-1 text-xs text-content"
                  >
                    PDF
                  </a>
                  <EduButton
                    testId={`scribepro-job-del-${j.id}`}
                    onClick={() =>
                      void scribeService.remove(j.id).then(refresh)
                    }
                  >
                    <Trash2 className="size-4" aria-hidden />
                  </EduButton>
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>
    </div>
  );
}

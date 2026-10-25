/**
 * AI Scribe PRO — MTECHOS Sehemu 1 (moduli 21).
 * Hupiga `/api/scribe/*` (server/agent/src/scribe.rs): DSP halisi + agentic + PDF.
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

function base(): string {
  const b = getAgentServerBaseUrl();
  if (!b) throw new Error("Backend base URL haijulikani");
  return b.replace(/\/+$/, "");
}

async function call<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetch(`${base()}/api/scribe${path}`, {
    method: body !== undefined ? "POST" : "GET",
    headers: {
      ...getAgentServerHeaders(),
      ...(body !== undefined ? { "Content-Type": "application/json" } : {}),
    },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  const data = (await res.json().catch(() => ({}))) as T & { error?: string };
  if (!res.ok) throw new Error(data?.error || `HTTP ${res.status}`);
  return data;
}

export interface ScribeStatus {
  ok: boolean;
  module: string;
  part: string;
  dsp: Record<string, unknown>;
  transcription: { whisper_api: boolean; web_speech_fallback: boolean };
  agentic: { llm: boolean; modes: string[] };
  pdf: Record<string, unknown>;
  jobs: number;
  pdfs: number;
}

export interface DenoiseResult {
  ok: boolean;
  sample_rate: number;
  samples_in: number;
  samples_out: number;
  duration_s: number;
  speech_frames: number;
  total_frames: number;
  speech_ratio: number;
  rms_before: number;
  rms_after: number;
  noise_removed_db: number;
  strength: number;
  wav_base64: string;
  /** Diarization hufanywa na backend kwenye sauti iliyosafishwa (bila ombi la ziada). */
  diarization?: DiarizeResult;
  note_sw: string;
}

export interface ScribeSpeaker {
  speaker: string;
  text: string;
}

export interface ScribeJob {
  id: string;
  mode: string;
  title: string;
  language: string;
  engine: string;
  created_at: string;
  preview: string;
}

export interface ScribeProcessResult {
  ok: boolean;
  id: string;
  mode: string;
  engine: string;
  speakers: ScribeSpeaker[];
  analysis: Record<string, unknown>;
  pdf_url: string;
  note_sw: string;
}

/** Kuchuja sauti: hupokea WAV (au PCM f32) na kurudisha WAV safi + ripoti. */
export async function denoiseAudio(
  wav: Blob | ArrayBuffer,
  rate = 16000,
  strength = 0.8,
): Promise<DenoiseResult> {
  const buf = wav instanceof Blob ? await wav.arrayBuffer() : wav;
  const res = await fetch(
    `${base()}/api/scribe/denoise?rate=${rate}&strength=${strength}`,
    {
      method: "POST",
      headers: {
        ...getAgentServerHeaders(),
        "Content-Type": "application/octet-stream",
      },
      body: buf,
    },
  );
  const data = (await res.json().catch(() => ({}))) as DenoiseResult & {
    error?: string;
  };
  if (!res.ok) throw new Error(data?.error || `HTTP ${res.status}`);
  return data;
}

export interface DiarizeSegment {
  speaker: number;
  t_start: number;
  t_end: number;
  duration_s: number;
  mean_rms: number;
}

export interface DiarizeSpeakerStat {
  speaker: number;
  turns: number;
  total_s: number;
  share_pct: number;
}

export interface DiarizeResult {
  ok: boolean;
  speakers: number;
  turns: number;
  total_speech_s: number;
  segments: DiarizeSegment[];
  per_speaker: DiarizeSpeakerStat[];
  method: string;
}

/** Hutenganisha wazungumzaji (diarization) kwa sifa za akustika + k-means.
 * `speakers=0` (default) = igundue yenyewe; `rate` ni sampuli ya PCM ghafi. */
export async function diarizeAudio(
  audio: Blob | ArrayBuffer,
  rate = 16000,
  speakers = 0,
): Promise<DiarizeResult> {
  const buf = audio instanceof Blob ? await audio.arrayBuffer() : audio;
  const res = await fetch(
    `${base()}/api/scribe/diarize?rate=${rate}&speakers=${speakers}`,
    {
      method: "POST",
      headers: {
        ...getAgentServerHeaders(),
        "Content-Type": "application/octet-stream",
      },
      body: buf,
    },
  );
  const data = (await res.json().catch(() => ({}))) as DiarizeResult & {
    error?: string;
  };
  if (!res.ok) throw new Error(data?.error || `HTTP ${res.status}`);
  return data;
}

/** Hupokea sauti kutoka kivinjari (MediaRecorder → webm) na kuibadilisha kuwa
 * PCM f32 ya 16kHz kwa Web Audio API — bila kuongeza dependency. */
export async function blobToPcm16k(blob: Blob): Promise<Float32Array> {
  const arrayBuf = await blob.arrayBuffer();
  const Ctx =
    window.AudioContext ||
    (window as unknown as { webkitAudioContext: typeof AudioContext })
      .webkitAudioContext;
  const ctx = new Ctx();
  const decoded = await ctx.decodeAudioData(arrayBuf);
  await ctx.close();
  // Unganisha channels → mono, kisha resample kwa 16000 (linear).
  const ch = decoded.numberOfChannels;
  const src = decoded.getChannelData(0);
  const mono = new Float32Array(src.length);
  for (let c = 0; c < ch; c += 1) {
    const d = decoded.getChannelData(c);
    for (let i = 0; i < d.length; i += 1) mono[i] += d[i] / ch;
  }
  const target = 16000;
  if (decoded.sampleRate === target) return mono;
  const ratio = decoded.sampleRate / target;
  const outLen = Math.floor(mono.length / ratio);
  const out = new Float32Array(outLen);
  for (let i = 0; i < outLen; i += 1) {
    const pos = i * ratio;
    const i0 = Math.floor(pos);
    const i1 = Math.min(i0 + 1, mono.length - 1);
    const frac = pos - i0;
    out[i] = mono[i0] * (1 - frac) + mono[i1] * frac;
  }
  return out;
}

export function pcmToWav(pcm: Float32Array, rate = 16000): Blob {
  const dataLen = pcm.length * 2;
  const buf = new ArrayBuffer(44 + dataLen);
  const view = new DataView(buf);
  const w = (o: number, s: string) => {
    for (let i = 0; i < s.length; i += 1) view.setUint8(o + i, s.charCodeAt(i));
  };
  w(0, "RIFF");
  view.setUint32(4, 36 + dataLen, true);
  w(8, "WAVE");
  w(12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 1, true);
  view.setUint32(24, rate, true);
  view.setUint32(28, rate * 2, true);
  view.setUint16(32, 2, true);
  view.setUint16(34, 16, true);
  w(36, "data");
  view.setUint32(40, dataLen, true);
  let o = 44;
  for (let i = 0; i < pcm.length; i += 1) {
    const s = Math.max(-1, Math.min(1, pcm[i]));
    view.setInt16(o, s < 0 ? s * 0x8000 : s * 0x7fff, true);
    o += 2;
  }
  return new Blob([buf], { type: "audio/wav" });
}

export const scribeService = {
  status: () => call<ScribeStatus>("/status"),
  process: (body: {
    transcript: string;
    mode?: string;
    title?: string;
    language?: string;
  }) => call<ScribeProcessResult>("/process", body),
  jobs: () => call<{ ok: boolean; jobs: ScribeJob[] }>("/jobs"),
  job: (id: string) =>
    call<{
      ok: boolean;
      job: {
        id: string;
        mode: string;
        title: string;
        transcript: string;
        speakers: ScribeSpeaker[];
        analysis: Record<string, unknown>;
      };
      pdf_url: string;
    }>(`/job/${id}`),
  remove: (id: string) => call<{ ok: boolean }>(`/job/${id}/remove`, {}),
  pdfUrl: (id: string) => `${base()}/api/scribe/pdf/${id}`,
};

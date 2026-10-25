/**
 * Live (ELIMUPRO Awamu 3 & 4) — madarasa ya moja kwa moja + uwasilishaji.
 * Hupiga `/api/live/*` kwenye fundi-deploy (server/agent/src/live.rs).
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

async function lvCall<T>(
  path: string,
  init?: { method?: string; body?: unknown; raw?: BodyInit },
): Promise<T> {
  const base = getAgentServerBaseUrl();
  if (!base) throw new Error("Backend base URL haijulikani");
  const url = `${base.replace(/\/+$/, "")}/api/live${path}`;
  const res = await fetch(url, {
    method: init?.method ?? "GET",
    headers: {
      ...getAgentServerHeaders(),
      ...(init?.body !== undefined || init?.raw !== undefined
        ? { "Content-Type": "application/json" }
        : {}),
    },
    body:
      init?.raw !== undefined
        ? init.raw
        : init?.body !== undefined
          ? JSON.stringify(init.body)
          : undefined,
  });
  const data = (await res.json().catch(() => ({}))) as T & { error?: string };
  if (!res.ok) throw new Error(data?.error || `HTTP ${res.status}`);
  return data;
}

/** URL ya WebSocket ya kikao (signaling + presence + slides live). */
export function liveWsUrl(sessionId: string): string {
  const base = getAgentServerBaseUrl() ?? "";
  return `${base.replace(/^http/, "ws").replace(/\/+$/, "")}/api/live/session/${sessionId}/ws`;
}

export function recordingUrl(sessionId: string): string {
  const base = getAgentServerBaseUrl() ?? "";
  return `${base.replace(/\/+$/, "")}/api/live/session/${sessionId}/recording`;
}

export interface LvStatus {
  ok: boolean;
  sessions: number;
  live_now: number;
  participants: number;
  transcript_lines: number;
  slides: number;
}

export interface LvSession {
  id: string;
  title: string;
  kind: "live" | "presentation";
  status: string;
  subject_id: string;
  started_at: string;
  participants: number;
}

export interface LvPeer {
  peer: string;
  name: string;
  role: string;
  student_id?: string;
}

export interface LvSlide {
  page: number;
  title: string;
  data: string;
}

export interface LvLine {
  speaker: string;
  text: string;
  at: string;
}

export interface LvMark {
  student_id: string;
  student: string;
  score: number;
  feedback: string;
}

export const liveService = {
  status: () => lvCall<LvStatus>("/status"),

  createSession: (body: {
    title: string;
    subject_id?: string;
    teacher_id?: string;
    kind?: "live" | "presentation";
    presenter?: string;
  }) =>
    lvCall<{ ok: boolean; id: string; kind: string }>("/session/create", {
      method: "POST",
      body,
    }),
  sessions: (params?: { subject_id?: string; kind?: string }) => {
    const qs = new URLSearchParams();
    if (params?.subject_id) qs.set("subject_id", params.subject_id);
    if (params?.kind) qs.set("kind", params.kind);
    const q = qs.toString();
    return lvCall<{ ok: boolean; sessions: LvSession[] }>(
      `/sessions${q ? `?${q}` : ""}`,
    );
  },
  session: (id: string) =>
    lvCall<{
      ok: boolean;
      session: {
        id: string;
        title: string;
        kind: string;
        status: string;
        presenter: string;
        started_at: string;
        recording: string;
        current_slide: number;
        summary_mode: string;
      };
      peers: LvPeer[];
    }>(`/session/${id}`),
  start: (id: string) =>
    lvCall<{ ok: boolean; status: string; at: string }>(`/session/${id}/start`, {
      method: "POST",
    }),
  end: (id: string) =>
    lvCall<{ ok: boolean; status: string; summary: { mode: string } }>(
      `/session/${id}/end`,
      { method: "POST" },
    ),
  join: (id: string, body: { name: string; role?: string; student_id?: string }) =>
    lvCall<{ ok: boolean; peer: string; role: string; name: string }>(
      `/session/${id}/join`,
      { method: "POST", body },
    ),
  leave: (id: string, peer: string) =>
    lvCall<{ ok: boolean }>(`/session/${id}/leave`, {
      method: "POST",
      body: { peer },
    }),
  peers: (id: string) =>
    lvCall<{ ok: boolean; peers: LvPeer[] }>(`/session/${id}/peers`),

  signal: (
    id: string,
    body: { from_peer: string; to_peer: string; kind: string; payload: unknown },
  ) => lvCall<{ ok: boolean }>(`/session/${id}/signal`, { method: "POST", body }),
  signals: (id: string, peer: string) =>
    lvCall<{ ok: boolean; signals: { id: string; from: string; kind: string; payload: unknown }[] }>(
      `/session/${id}/signals?peer=${encodeURIComponent(peer)}`,
    ),

  transcriptAdd: (id: string, body: { speaker: string; text: string }) =>
    lvCall<{ ok: boolean }>(`/session/${id}/transcript`, { method: "POST", body }),
  transcript: (id: string) =>
    lvCall<{ ok: boolean; lines: LvLine[] }>(`/session/${id}/transcript/list`),
  summaryMake: (id: string) =>
    lvCall<{ ok: boolean; mode: string; summary?: unknown; error?: string }>(
      `/session/${id}/summary`,
      { method: "POST" },
    ),
  summaryGet: (id: string) =>
    lvCall<{ ok: boolean; mode: string; summary?: unknown }>(
      `/session/${id}/summary/get`,
    ),

  slideAdd: (id: string, body: { title?: string; data?: string }) =>
    lvCall<{ ok: boolean; page: number }>(`/session/${id}/slide`, {
      method: "POST",
      body,
    }),
  slides: (id: string) =>
    lvCall<{ ok: boolean; slides: LvSlide[] }>(`/session/${id}/slides`),
  slideGo: (id: string, page: number) =>
    lvCall<{ ok: boolean; page: number }>(`/session/${id}/slide/go`, {
      method: "POST",
      body: { page },
    }),

  mark: (id: string, body: { student_id: string; score: number; feedback?: string }) =>
    lvCall<{ ok: boolean }>(`/session/${id}/mark`, { method: "POST", body }),
  marks: (id: string) =>
    lvCall<{ ok: boolean; marks: LvMark[] }>(`/session/${id}/marks`),

  record: (id: string, blob: Blob) =>
    lvCall<{ ok: boolean; name: string; bytes: number }>(`/session/${id}/record`, {
      method: "POST",
      raw: blob,
    }),
};

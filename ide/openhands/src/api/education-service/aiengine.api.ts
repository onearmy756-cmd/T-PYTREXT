/**
 * AI Engine — ELIMUPRO Awamu 17 (moduli 17).
 * Hupiga `/api/ai/*` (server/agent/src/aiengine.rs).
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

async function aiCall<T>(path: string, body?: unknown): Promise<T> {
  const base = getAgentServerBaseUrl();
  if (!base) throw new Error("Backend base URL haijulikani");
  const url = `${base.replace(/\/+$/, "")}/api/ai${path}`;
  const res = await fetch(url, {
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

export interface AiStatus {
  ok: boolean;
  jobs: number;
  jobs_llm: number;
  jobs_offline: number;
  flags: number;
  flags_unreviewed: number;
  capabilities: Record<string, boolean>;
}

export interface AiFlag {
  id: string;
  subject_id: string;
  kind: string;
  ref_id: string;
  score: number;
  reasons: string[];
  risk: string;
  reviewed: boolean;
  reviewer: string;
  created_at: string;
}

export interface AiJob {
  id: string;
  kind: string;
  ref_id: string;
  output: string;
  mode: string;
  created_at: string;
}

export interface AiResult<T = unknown> {
  ok: boolean;
  mode: "llm" | "offline";
  result: T | string;
  error?: string;
}

export interface AntiCheatResult {
  ok: boolean;
  score: number;
  risk: string;
  reasons: string[];
  flagged: boolean;
  flag_id: string;
  note: string;
}

export const aiEngineService = {
  status: () => aiCall<AiStatus>("/status"),
  scribe: (body: { transcript: string; language?: string; subject?: string }) =>
    aiCall<AiResult>("/scribe", body),
  lesson: (body: { notes: string; subject?: string; level?: string }) =>
    aiCall<AiResult>("/lesson", body),
  anticheat: (body: { text: string; kind?: string; ref_id?: string; subject_id?: string }) =>
    aiCall<AntiCheatResult>("/anticheat", body),
  tutor: (body: { question: string; notes?: string; subject?: string }) =>
    aiCall<AiResult>("/tutor", body),
  flags: (params?: { reviewed?: string; kind?: string }) => {
    const q = new URLSearchParams();
    if (params?.reviewed) q.set("reviewed", params.reviewed);
    if (params?.kind) q.set("kind", params.kind);
    const qs = q.toString();
    return aiCall<{ ok: boolean; flags: AiFlag[] }>(`/flags${qs ? `?${qs}` : ""}`);
  },
  reviewFlag: (id: string, reviewer: string) =>
    aiCall<{ ok: boolean; error?: string }>(`/flag/${id}/review`, { reviewer }),
  jobs: (kind?: string) =>
    aiCall<{ ok: boolean; jobs: AiJob[] }>(`/jobs${kind ? `?kind=${kind}` : ""}`),
};

/**
 * Zana za IT — ELIMUPRO Awamu 16 (moduli 16).
 * Hupiga `/api/ittools/*` (server/agent/src/ittools.rs).
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

async function itCall<T>(path: string, body?: unknown): Promise<T> {
  const base = getAgentServerBaseUrl();
  if (!base) throw new Error("Backend base URL haijulikani");
  const url = `${base.replace(/\/+$/, "")}/api/ittools${path}`;
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

export interface ItStatus {
  ok: boolean;
  languages: string[];
  sessions_active: number;
  sessions: number;
  submissions: number;
  submissions_ok: number;
  labs: number;
  attempts: number;
  tools: Record<string, boolean>;
}

export interface ItSession {
  id: string;
  student_id: string;
  kind: string;
  language: string;
  status: string;
  started_at: string;
  ended_at: string | null;
  live: boolean;
}

export interface ItSubmission {
  id: string;
  student_id: string;
  language: string;
  ok: boolean;
  score: number;
  feedback: string;
  created_at: string;
}

export interface ItLab {
  id: string;
  title: string;
  category: string;
  brief: string;
  points: number;
}

export interface RunResult {
  ok: boolean;
  language: string;
  result: {
    ok?: boolean;
    exit?: number;
    stdout?: string;
    stderr?: string;
    ms?: number;
    error?: string;
  };
}

export interface DbQueryResult {
  ok: boolean;
  columns?: string[];
  rows?: unknown[][];
  count?: number;
  error?: string;
}

export const itToolsService = {
  status: () => itCall<ItStatus>("/status"),
  startSession: (body: { student_id: string; kind?: string; language?: string }) =>
    itCall<{ ok: boolean; id?: string; kind?: string; error?: string }>("/session/start", body),
  sessions: (params?: { student_id?: string; kind?: string; status?: string }) => {
    const q = new URLSearchParams();
    if (params?.student_id) q.set("student_id", params.student_id);
    if (params?.kind) q.set("kind", params.kind);
    if (params?.status) q.set("status", params.status);
    const qs = q.toString();
    return itCall<{ ok: boolean; sessions: ItSession[] }>(`/sessions${qs ? `?${qs}` : ""}`);
  },
  endSession: (id: string) => itCall<{ ok: boolean; error?: string }>(`/session/${id}/end`, {}),
  run: (body: { code: string; language?: string; session_id?: string; student_id?: string }) =>
    itCall<RunResult>("/run", body),
  submit: (body: {
    student_id: string;
    code: string;
    language?: string;
    session_id?: string;
    expect_stdout?: string;
  }) =>
    itCall<{
      ok: boolean;
      id?: string;
      passed?: boolean;
      score?: number;
      feedback?: string;
      stdout?: string;
      stderr?: string;
      exit?: number;
      error?: string;
    }>("/submit", body),
  submissions: (studentId?: string) =>
    itCall<{ ok: boolean; submissions: ItSubmission[] }>(
      `/submissions${studentId ? `?student_id=${studentId}` : ""}`,
    ),
  addLab: (body: {
    title: string;
    category?: string;
    brief?: string;
    expected: string;
    points?: number;
  }) => itCall<{ ok: boolean; id?: string; category?: string; error?: string }>("/lab/add", body),
  labs: (category?: string) =>
    itCall<{ ok: boolean; labs: ItLab[] }>(`/labs${category ? `?category=${category}` : ""}`),
  attemptLab: (body: { lab_id: string; student_id: string; answer: string }) =>
    itCall<{ ok: boolean; correct?: boolean; points?: number; error?: string }>("/lab/attempt", body),
  labAttempts: (params?: { student_id?: string; lab_id?: string }) => {
    const q = new URLSearchParams();
    if (params?.student_id) q.set("student_id", params.student_id);
    if (params?.lab_id) q.set("lab_id", params.lab_id);
    const qs = q.toString();
    return itCall<{
      ok: boolean;
      attempts: { id: string; lab_id: string; student_id: string; correct: boolean; points: number; created_at: string }[];
    }>(`/lab/attempts${qs ? `?${qs}` : ""}`);
  },
  dbQuery: (sql: string) => itCall<DbQueryResult>("/db/query", { sql }),
  dbSchema: () =>
    itCall<{
      ok: boolean;
      tables: { table: string; columns: { name: string; type: string; pk: boolean }[] }[];
    }>("/db/schema"),
  monitor: () =>
    itCall<{
      ok: boolean;
      live: { id: string; student_id: string; kind: string; language: string; started_at: string }[];
      recent: { student_id: string; language: string; feedback: string; ok: boolean; score: number }[];
    }>("/monitor"),
};

/**
 * Marks (ELIMUPRO Awamu 6) — alama za mtindo wa Excel.
 * Hupiga `/api/marks/*` kwenye fundi-deploy (server/agent/src/marks.rs).
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

async function mkCall<T>(path: string, body?: unknown): Promise<T> {
  const base = getAgentServerBaseUrl();
  if (!base) throw new Error("Backend base URL haijulikani");
  const url = `${base.replace(/\/+$/, "")}/api/marks${path}`;
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

export interface MkStatus {
  ok: boolean;
  columns: number;
  marks: number;
  levels: number;
}

export interface MkColumn {
  id: string;
  kind: string;
  label: string;
  subject_id: string;
  max_marks: number;
  weight: number;
  position: number;
}

export interface MkRow {
  student_id: string;
  student: string;
  reg_no: string;
  marks: { column_id: string; score: number }[];
  total: number;
  max: number;
  percent: number;
  grade: string;
}

export interface MkGradebook {
  ok: boolean;
  columns: MkColumn[];
  rows: MkRow[];
}

export interface MkStudentReport {
  ok: boolean;
  student: { id: string; name: string; reg_no: string; level_id: string; course_id: string };
  entries: {
    column_id: string;
    subject_id: string;
    label: string;
    score: number;
    max_marks: number;
    weight: number;
  }[];
  subjects: { subject_id: string; subject: string; percent: number; grade: string }[];
}

export const marksService = {
  status: () => mkCall<MkStatus>("/status"),

  columns: (p: { level_id?: string; subject_id?: string }) => {
    const qs = new URLSearchParams();
    if (p.level_id) qs.set("level_id", p.level_id);
    if (p.subject_id) qs.set("subject_id", p.subject_id);
    return mkCall<{ ok: boolean; columns: MkColumn[] }>(`/columns?${qs}`);
  },
  addColumn: (body: {
    level_id: string;
    subject_id: string;
    kind: string;
    label?: string;
    max_marks?: number;
    weight?: number;
  }) => mkCall<{ ok: boolean; id: string; label: string }>("/column/add", body),
  deleteColumn: (id: string) => mkCall<{ ok: boolean }>("/column/delete", { id }),
  editColumn: (body: { id: string; label?: string; max_marks?: number; weight?: number }) =>
    mkCall<{ ok: boolean; label?: string; max_marks?: number; weight?: number; error?: string }>(
      "/column/edit",
      body,
    ),

  gradebook: (p: { level_id: string; subject_id?: string }) => {
    const qs = new URLSearchParams({ level_id: p.level_id });
    if (p.subject_id) qs.set("subject_id", p.subject_id);
    return mkCall<MkGradebook>(`/gradebook?${qs}`);
  },
  setMark: (body: { column_id: string; student_id: string; score: number }) =>
    mkCall<{ ok: boolean; score: number }>("/set", body),
  bulk: (body: {
    level_id: string;
    subject_id?: string;
    marks: { column_id: string; student_id: string; score: number }[];
  }) => mkCall<{ ok: boolean; saved: number }>("/bulk", body),

  studentReport: (id: string) => mkCall<MkStudentReport>(`/student/${id}`),

  exportUrl: (p: { level_id: string; subject_id?: string }) => {
    const base = (getAgentServerBaseUrl() ?? "").replace(/\/+$/, "");
    const qs = new URLSearchParams({ level_id: p.level_id });
    if (p.subject_id) qs.set("subject_id", p.subject_id);
    return `${base}/api/marks/export.csv?${qs}`;
  },
};

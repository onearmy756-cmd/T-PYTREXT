/**
 * Matokeo, transkripti na vyeti — ELIMUPRO Awamu 11.
 * Hupiga `/api/results/*` kwenye fundi-deploy (server/agent/src/transcript.rs).
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

async function resCall<T>(path: string, body?: unknown): Promise<T> {
  const base = getAgentServerBaseUrl();
  if (!base) throw new Error("Backend base URL haijulikani");
  const url = `${base.replace(/\/+$/, "")}/api/results${path}`;
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

export interface ResultsStatus {
  ok: boolean;
  marks: number;
  students_with_marks: number;
  transcripts: number;
  certificates: number;
  grade_scale: { min: number; letter: string; points: number }[];
}

export interface SubjectResult {
  subject_id: string;
  subject: string;
  code: string;
  score: number | null;
  letter: string;
  points: number;
  credits: number;
  entered: boolean;
}

export interface StudentResults {
  ok: boolean;
  student: { id: string; name: string; reg_no: string; level_id: string };
  subjects: SubjectResult[];
  total: number;
  average: number;
  subjects_entered: number;
  gpa: number;
  classification: string;
  issued_transcript?: { id: string; gpa: number; classification: string; issued_at: string };
}

export interface LevelResult {
  rank: number;
  student_id: string;
  student: string;
  reg_no: string;
  average: number;
  gpa: number;
  subjects: number;
  classification: string;
}

export interface Certificate {
  id: string;
  student: string;
  reg_no: string;
  kind: string;
  title: string;
  serial: string;
  verify_code: string;
  revoked: boolean;
}

export const resultsService = {
  status: () => resCall<ResultsStatus>("/status"),
  student: (student_id: string) =>
    resCall<StudentResults>(`/student?student_id=${encodeURIComponent(student_id)}`),
  level: (level_id: string) =>
    resCall<{ ok: boolean; level_id: string; count: number; results: LevelResult[] }>(
      `/level?level_id=${encodeURIComponent(level_id)}`,
    ),
  transcript: (student_id: string) =>
    resCall<StudentResults>(`/transcript?student_id=${encodeURIComponent(student_id)}`),
  issueTranscript: (body: { student_id: string; issued_by?: string }) =>
    resCall<{ ok: boolean; id?: string; gpa?: number; classification?: string; error?: string }>(
      "/transcript/issue",
      body,
    ),
  issueCertificate: (body: { student_id: string; kind?: string; title?: string }) =>
    resCall<{ ok: boolean; id?: string; serial?: string; verify_code?: string; title?: string; error?: string }>(
      "/certificate/issue",
      body,
    ),
  certificates: (p: { student_id?: string; kind?: string }) => {
    const qs = new URLSearchParams();
    if (p.student_id) qs.set("student_id", p.student_id);
    if (p.kind) qs.set("kind", p.kind);
    return resCall<{ ok: boolean; certificates: Certificate[] }>(`/certificates?${qs}`);
  },
  certificate: (id: string) =>
    resCall<{ ok: boolean; certificate: Certificate & { issued_at: string } }>(`/certificate/${id}`),
  verify: (serial: string, code: string) =>
    resCall<{
      ok: boolean;
      valid: boolean;
      revoked?: boolean;
      error?: string;
      certificate?: { student: string; kind: string; title: string; issued_at: string };
    }>(`/verify?serial=${encodeURIComponent(serial)}&code=${encodeURIComponent(code)}`),
};

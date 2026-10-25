/**
 * Proctor (ELIMUPRO Awamu 5) — mitihani ya chuo + ulinzi mkali.
 * Hupiga `/api/proctor/*` kwenye fundi-deploy (server/agent/src/proctor.rs).
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

async function pcCall<T>(path: string, body?: unknown): Promise<T> {
  const base = getAgentServerBaseUrl();
  if (!base) throw new Error("Backend base URL haijulikani");
  const url = `${base.replace(/\/+$/, "")}/api/proctor${path}`;
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

export interface PcStatus {
  ok: boolean;
  exams: number;
  published: number;
  attempts: number;
  active: number;
  proctor_events: number;
}

export interface PcExam {
  id: string;
  title: string;
  status: string;
  created_at: string;
  minutes: number;
  questions: number;
  attempts: number;
}

export interface PcQuestion {
  id: string;
  n: number;
  kind: "mcq" | "text" | "code";
  prompt: string;
  options: string[];
  marks: number;
  answer_key?: string;
}

export interface PcAttempt {
  id: string;
  student_id: string;
  student: string;
  score: number;
  violations: number;
  status: string;
  grade_mode: string;
}

export interface PcAnswer {
  question_id: string;
  body: string;
  locked: boolean;
  auto_score: number;
  ai_score: number;
  ai_feedback: string;
  run_out: string;
}

export const proctorService = {
  status: () => pcCall<PcStatus>("/status"),

  createExam: (body: {
    title: string;
    subject_id?: string;
    level_id?: string;
    teacher_id?: string;
    instructions?: string;
    minutes?: number;
    proctor?: boolean;
    pass_mark?: number;
  }) => pcCall<{ ok: boolean; id: string }>("/exam/create", body),
  exams: (params?: { subject_id?: string; status?: string }) => {
    const qs = new URLSearchParams();
    if (params?.subject_id) qs.set("subject_id", params.subject_id);
    if (params?.status) qs.set("status", params.status);
    const q = qs.toString();
    return pcCall<{ ok: boolean; exams: PcExam[] }>(`/exams${q ? `?${q}` : ""}`);
  },
  exam: (id: string) =>
    pcCall<{
      ok: boolean;
      exam: {
        id: string;
        title: string;
        status: string;
        instructions: string;
        minutes: number;
        proctor: boolean;
        pass_mark: number;
      };
    }>(`/exam/${id}`),
  publish: (id: string) =>
    pcCall<{ ok: boolean; questions: number }>(`/exam/${id}/publish`, {}),
  close: (id: string) => pcCall<{ ok: boolean }>(`/exam/${id}/close`, {}),
  attempts: (id: string) =>
    pcCall<{ ok: boolean; attempts: PcAttempt[] }>(`/exam/${id}/attempts`),

  addQuestion: (
    id: string,
    body: {
      kind: string;
      prompt: string;
      options?: string[];
      answer_key?: string;
      language?: string;
      marks?: number;
    },
  ) => pcCall<{ ok: boolean; id: string; n: number }>(`/exam/${id}/question`, body),
  questions: (id: string, withKey = false) =>
    pcCall<{ ok: boolean; questions: PcQuestion[] }>(
      `/exam/${id}/questions${withKey ? "?with_key=true" : ""}`,
    ),

  start: (examId: string, studentId: string) =>
    pcCall<{ ok: boolean; attempt_id: string; proctor: boolean; max_score: number }>(
      `/exam/${examId}/start`,
      { student_id: studentId },
    ),
  attempt: (aid: string) =>
    pcCall<{
      ok: boolean;
      attempt: {
        id: string;
        exam_id: string;
        student_id: string;
        status: string;
        started_at: string;
        score: number;
        violations: number;
        grade_mode: string;
      };
      answers: PcAnswer[];
    }>(`/attempt/${aid}`),
  saveAnswer: (aid: string, questionId: string, body: string) =>
    pcCall<{ ok: boolean; locked: boolean; error?: string; body?: string }>(
      `/attempt/${aid}/answer`,
      { question_id: questionId, body },
    ),
  runCode: (aid: string, questionId: string, language: string, body: string) =>
    pcCall<{ ok: boolean; run: unknown }>(`/attempt/${aid}/run`, {
      question_id: questionId,
      language,
      body,
    }),
  submit: (aid: string) =>
    pcCall<{
      ok: boolean;
      score: number;
      max_score: number;
      grade_mode: string;
      results: { question_id: string; kind: string; score: number; ai_feedback: string }[];
    }>(`/attempt/${aid}/submit`, {}),
  grade: (aid: string, questionId: string, score: number, feedback = "") =>
    pcCall<{ ok: boolean; score: number }>(`/attempt/${aid}/grade`, {
      question_id: questionId,
      score,
      feedback,
    }),

  addEvent: (aid: string, kind: string, detail = "", data = "") =>
    pcCall<{ ok: boolean; weight: number; violations: number }>(
      `/attempt/${aid}/event`,
      { kind, detail, data },
    ),
  events: (aid: string) =>
    pcCall<{
      ok: boolean;
      events: { kind: string; weight: number; detail: string; data: string; at: string }[];
    }>(`/attempt/${aid}/events`),
};

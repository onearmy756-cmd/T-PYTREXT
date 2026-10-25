/**
 * Attendance + Timetable (ELIMUPRO Awamu 7).
 * Hupiga `/api/attendance/*` kwenye fundi-deploy (server/agent/src/attendance.rs).
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

async function atCall<T>(path: string, body?: unknown): Promise<T> {
  const base = getAgentServerBaseUrl();
  if (!base) throw new Error("Backend base URL haijulikani");
  const url = `${base.replace(/\/+$/, "")}/api/attendance${path}`;
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

export interface AtStatus {
  ok: boolean;
  periods: number;
  sessions: number;
  marks: number;
  open: number;
}

export interface AtPeriod {
  id: string;
  level_id: string;
  level: string;
  subject_id: string;
  subject: string;
  teacher_id: string;
  teacher: string;
  day: number;
  start: string;
  end: string;
  start_min: number;
  end_min: number;
  room: string;
}

export interface AtSession {
  id: string;
  level_id: string;
  subject_id: string;
  subject: string;
  date: string;
  open: boolean;
  total: number;
  present: number;
  code: string;
}

export interface AtStudentMark {
  student_id: string;
  student: string;
  reg_no: string;
  status: string;
  note: string;
}

export const attendanceService = {
  status: () => atCall<AtStatus>("/status"),

  periods: (p: { level_id?: string; teacher_id?: string; day?: number }) => {
    const qs = new URLSearchParams();
    if (p.level_id) qs.set("level_id", p.level_id);
    if (p.teacher_id) qs.set("teacher_id", p.teacher_id);
    if (p.day) qs.set("day", String(p.day));
    return atCall<{ ok: boolean; periods: AtPeriod[] }>(`/timetable?${qs}`);
  },
  addPeriod: (body: {
    level_id: string;
    subject_id: string;
    teacher_id?: string;
    day: number;
    start: string;
    end: string;
    room?: string;
  }) =>
    atCall<{ ok: boolean; id?: string; error?: string; kind?: string }>(
      "/timetable/add",
      body,
    ),
  deletePeriod: (id: string) => atCall<{ ok: boolean }>("/timetable/delete", { id }),

  openSession: (body: { timetable_id: string; date: string; created_by?: string }) =>
    atCall<{ ok: boolean; id: string; code: string; open: boolean; existing?: boolean; students?: number }>(
      "/session/open",
      body,
    ),
  sessions: (p: { level_id?: string; date?: string }) => {
    const qs = new URLSearchParams();
    if (p.level_id) qs.set("level_id", p.level_id);
    if (p.date) qs.set("date", p.date);
    return atCall<{ ok: boolean; sessions: AtSession[] }>(`/sessions?${qs}`);
  },
  session: (id: string) =>
    atCall<{
      ok: boolean;
      session: {
        id: string;
        level_id: string;
        subject_id: string;
        date: string;
        open: boolean;
        code: string;
        percent: number;
      };
      students: AtStudentMark[];
    }>(`/session/${id}`),
  mark: (body: { session_id: string; student_id: string; status: string; note?: string }) =>
    atCall<{ ok: boolean; status: string }>("/mark", body),
  markBulk: (body: {
    session_id: string;
    marks: { student_id: string; status: string; note?: string }[];
  }) => atCall<{ ok: boolean; saved: number }>("/mark/bulk", body),
  checkin: (body: { code: string; student_id: string }) =>
    atCall<{ ok: boolean; status?: string; error?: string }>("/checkin", body),
  closeSession: (id: string) => atCall<{ ok: boolean }>("/session/close", { id }),

  summaryStudent: (id: string) =>
    atCall<{
      ok: boolean;
      student: { id: string; name: string; reg_no: string; level_id: string };
      overall: number;
      sessions: number;
      subjects: { subject_id: string; subject: string; percent: number; sessions: number }[];
    }>(`/summary/student/${id}`),
  summaryLevel: (level_id: string) =>
    atCall<{
      ok: boolean;
      students: { student_id: string; student: string; reg_no: string; percent: number; sessions: number }[];
    }>(`/summary/level?level_id=${level_id}`),
};

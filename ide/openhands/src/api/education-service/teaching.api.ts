/**
 * Ufundishaji (ELIMUPRO Awamu 2) service — hupiga endpoints halisi za
 * `/api/teaching/*` kwenye fundi-deploy (server/agent/src/teaching.rs).
 *
 * Notes/materiala → wanafunzi, assignments (individual/group) + AI-check,
 * groups + chat (WebSocket), matangazo.
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

async function tpCall<T>(
  path: string,
  init?: { method?: string; body?: unknown },
): Promise<T> {
  const base = getAgentServerBaseUrl();
  if (!base) throw new Error("Backend base URL haijulikani");
  const url = `${base.replace(/\/+$/, "")}/api/teaching${path}`;
  const res = await fetch(url, {
    method: init?.method ?? "GET",
    headers: {
      ...getAgentServerHeaders(),
      ...(init?.body !== undefined
        ? { "Content-Type": "application/json" }
        : {}),
    },
    body: init?.body !== undefined ? JSON.stringify(init.body) : undefined,
  });
  const data = (await res.json().catch(() => ({}))) as T & { error?: string };
  if (!res.ok) throw new Error(data?.error || `HTTP ${res.status}`);
  return data;
}

/** URL ya WebSocket ya chat ya kikundi. */
export function groupWsUrl(groupId: string): string {
  const base = getAgentServerBaseUrl() ?? "";
  return `${base.replace(/^http/, "ws").replace(/\/+$/, "")}/api/teaching/group/${groupId}/ws`;
}

export interface TpStatus {
  ok: boolean;
  notes: number;
  assignments: number;
  submissions: number;
  groups: number;
  group_messages: number;
  announcements: number;
}

export interface TpNote {
  id: string;
  title: string;
  body: string;
  kind: string;
  url: string;
  subject_id: string;
  shared_levels: number;
}

export interface TpAssignment {
  id: string;
  title: string;
  instructions: string;
  mode: "individual" | "group";
  subject_id: string;
  teacher_id: string;
  marks: number;
  due_at: string;
  created_at: string;
  submissions: number;
}

export interface TpSubmission {
  id: string;
  student_id: string;
  student: string;
  ai_score: number;
  ai_feedback: string;
  score: number;
  feedback: string;
  status: string;
}

export interface TpGroup {
  id: string;
  name: string;
  subject_id: string;
  members: number;
}

export interface TpMessage {
  id: string;
  author: string;
  author_type: string;
  body: string;
  at: string;
}

export interface TpAnnouncement {
  id: string;
  title: string;
  body: string;
  target: string;
  target_id: string;
  created_at: string;
}

export const teachingService = {
  status: () => tpCall<TpStatus>("/status"),

  // Notes
  addNote: (body: {
    subject_id: string;
    teacher_id?: string;
    title: string;
    body?: string;
    kind?: string;
    url?: string;
    level_ids?: string[];
  }) => tpCall<{ ok: boolean; id: string; shared_levels: number }>("/note/add", {
    method: "POST",
    body,
  }),
  notes: (params?: { subject_id?: string; level_id?: string }) => {
    const qs = new URLSearchParams();
    if (params?.subject_id) qs.set("subject_id", params.subject_id);
    if (params?.level_id) qs.set("level_id", params.level_id);
    const q = qs.toString();
    return tpCall<{ ok: boolean; notes: TpNote[] }>(`/notes${q ? `?${q}` : ""}`);
  },

  // Assignments
  addAssignment: (body: {
    subject_id: string;
    teacher_id?: string;
    title: string;
    instructions?: string;
    mode?: "individual" | "group";
    marks?: number;
    due_at?: string;
  }) =>
    tpCall<{ ok: boolean; id: string; mode: string; marks: number }>(
      "/assignment/add",
      { method: "POST", body },
    ),
  assignments: (subjectId?: string) => {
    const qs = subjectId ? `?subject_id=${encodeURIComponent(subjectId)}` : "";
    return tpCall<{ ok: boolean; assignments: TpAssignment[] }>(
      `/assignments${qs}`,
    );
  },
  assignment: (id: string) =>
    tpCall<{
      ok: boolean;
      assignment: TpAssignment;
      submissions: TpSubmission[];
    }>(`/assignment/${id}`),
  submit: (id: string, body: { student_id: string; group_id?: string; content: string }) =>
    tpCall<{
      ok: boolean;
      id: string;
      status: string;
      ai_score: number;
      ai_feedback: string;
      ai_mode: "graded" | "pending";
    }>(`/assignment/${id}/submit`, { method: "POST", body }),
  submissions: (id: string) =>
    tpCall<{ ok: boolean; submissions: TpSubmission[] }>(
      `/assignment/${id}/submissions`,
    ),
  grade: (submissionId: string, body: { score: number; feedback?: string }) =>
    tpCall<{ ok: boolean; score: number }>(`/submission/${submissionId}/grade`, {
      method: "POST",
      body,
    }),

  // Groups + chat
  addGroup: (body: {
    subject_id: string;
    name: string;
    created_by?: string;
    member_ids?: string[];
  }) => tpCall<{ ok: boolean; id: string; members: number }>("/group/add", {
    method: "POST",
    body,
  }),
  addGroupMember: (groupId: string, studentId: string) =>
    tpCall<{ ok: boolean }>(`/group/${groupId}/member/add`, {
      method: "POST",
      body: { student_id: studentId },
    }),
  groups: () => tpCall<{ ok: boolean; groups: TpGroup[] }>("/groups"),
  group: (id: string) =>
    tpCall<{
      ok: boolean;
      group: { id: string; name: string; subject_id: string };
      members: { id: string; name: string }[];
    }>(`/group/${id}`),
  sendMessage: (
    groupId: string,
    body: { author: string; author_type?: string; body: string },
  ) =>
    tpCall<{ ok: boolean; message: TpMessage }>(`/group/${groupId}/message`, {
      method: "POST",
      body,
    }),
  messages: (groupId: string) =>
    tpCall<{ ok: boolean; messages: TpMessage[] }>(`/group/${groupId}/messages`),

  // Matangazo
  addAnnouncement: (body: {
    title: string;
    body?: string;
    target?: string;
    target_id?: string;
    author?: string;
  }) => tpCall<{ ok: boolean; id: string; target: string }>("/announcement/add", {
    method: "POST",
    body,
  }),
  announcements: (params?: { level_id?: string; course_id?: string }) => {
    const qs = new URLSearchParams();
    if (params?.level_id) qs.set("level_id", params.level_id);
    if (params?.course_id) qs.set("course_id", params.course_id);
    const q = qs.toString();
    return tpCall<{ ok: boolean; announcements: TpAnnouncement[] }>(
      `/announcements${q ? `?${q}` : ""}`,
    );
  },
};

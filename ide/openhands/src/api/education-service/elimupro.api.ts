/**
 * ELIMUPRO (Awamu 1) service — hupiga endpoints halisi za `/api/elimupro/*`
 * kwenye fundi-deploy (server/agent/src/elimupro.rs).
 *
 * Taasisi → courses/levels/subjects → walimu → usajili (NECTA/RITA) →
 * enrollment → ID card.
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

async function epCall<T>(
  path: string,
  init?: { method?: string; body?: unknown },
): Promise<T> {
  const base = getAgentServerBaseUrl();
  if (!base) throw new Error("Backend base URL haijulikani");
  const url = `${base.replace(/\/+$/, "")}/api/elimupro${path}`;
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

/** URL ya ID card (SVG) kwa mwanafunzi — hupigwa moja kwa moja kwenye <img>. */
export function idCardUrl(studentId: string): string {
  const base = getAgentServerBaseUrl() ?? "";
  return `${base.replace(/\/+$/, "")}/api/elimupro/student/${studentId}/idcard.svg`;
}

export interface EpStatus {
  ok: boolean;
  institutions: number;
  courses: number;
  levels: number;
  subjects: number;
  teachers: number;
  students: number;
  enrollments: number;
  necta_api: boolean;
  rita_api: boolean;
}

export interface EpInstitution {
  id: string;
  name: string;
  code: string;
  motto: string;
  address: string;
  phone: string;
  email: string;
  website: string;
  description: string;
  logo: string;
}

export interface EpCourse {
  id: string;
  name: string;
  code: string;
  award: string;
  years: number;
  description: string;
  levels: number;
}

export interface EpLevel {
  id: string;
  year: number;
  semester: number;
  name: string;
  subjects: number;
}

export interface EpSubject {
  id: string;
  name: string;
  code: string;
  credits: number;
  description: string;
  level: string;
}

export interface EpTeacher {
  id: string;
  name: string;
  email: string;
  phone: string;
  staff_no: string;
  username: string;
  subjects: number;
}

export interface EpTeacherDetail {
  ok: boolean;
  teacher: {
    id: string;
    name: string;
    email: string;
    phone: string;
    staff_no: string;
    qualification: string;
    specialization: string;
  };
  subjects: { id: string; name: string; code: string; academic_year: string }[];
}

export interface EpStudent {
  id: string;
  full_name: string;
  gender: string;
  reg_no: string;
  username: string;
  necta_verified: string;
  rita_verified: string;
  level: string;
  subjects: number;
}

export interface EpStudentDetail {
  ok: boolean;
  student: {
    id: string;
    full_name: string;
    reg_no: string;
    username: string;
    gender: string;
    dob: string;
    phone: string;
    email: string;
    form4_index: string;
    birth_cert_no: string;
    necta_verified: string;
    rita_verified: string;
  };
  subjects: { name: string; code: string; teacher: string }[];
}

export interface EpVerifyResult {
  ok: boolean;
  mode: "api" | "api_error" | "manual";
  verified: boolean;
  result?: unknown;
  detail?: unknown;
  message?: string;
  error?: string;
}

export interface EpAdmitResult {
  ok: boolean;
  id?: string;
  reg_no?: string;
  username?: string;
  password?: string;
  enrolled_subjects?: number;
  necta_verified?: string;
  rita_verified?: string;
  institution?: string;
  error?: string;
}

export const elimuproService = {
  status: () => epCall<EpStatus>("/status"),

  // Taasisi
  institution: () =>
    epCall<{ ok: boolean; institution: EpInstitution | null }>("/institution"),
  saveInstitution: (body: Partial<EpInstitution>) =>
    epCall<{ ok: boolean; id: string; code: string }>("/institution/save", {
      method: "POST",
      body,
    }),

  // Courses / levels / subjects
  courses: () => epCall<{ ok: boolean; courses: EpCourse[] }>("/courses"),
  addCourse: (body: {
    name: string;
    code?: string;
    award?: string;
    years?: number;
    description?: string;
    auto_levels?: boolean;
  }) => epCall<{ ok: boolean; id: string; levels_created: number }>("/course/add", {
    method: "POST",
    body,
  }),

  levels: (courseId: string) => {
    const qs = courseId ? `?course_id=${encodeURIComponent(courseId)}` : "";
    return epCall<{ ok: boolean; levels: EpLevel[] }>(`/levels${qs}`);
  },

  subjects: (levelId: string) => {
    const qs = levelId ? `?level_id=${encodeURIComponent(levelId)}` : "";
    return epCall<{ ok: boolean; subjects: EpSubject[] }>(`/subjects${qs}`);
  },
  addSubject: (body: {
    level_id: string;
    name: string;
    code?: string;
    credits?: number;
    description?: string;
  }) => epCall<{ ok: boolean; id: string; code: string; error?: string }>(
    "/subject/add",
    { method: "POST", body },
  ),

  // Walimu
  teachers: () => epCall<{ ok: boolean; teachers: EpTeacher[] }>("/teachers"),
  addTeacher: (body: {
    name: string;
    email?: string;
    phone?: string;
    staff_no?: string;
    qualification?: string;
    specialization?: string;
    username?: string;
    password?: string;
  }) => epCall<{ ok: boolean; id: string; username: string; password: string }>(
    "/teacher/add",
    { method: "POST", body },
  ),
  teacher: (id: string) => epCall<EpTeacherDetail>(`/teacher/${id}`),
  assignSubject: (body: {
    teacher_id: string;
    subject_id: string;
    academic_year?: string;
  }) => epCall<{ ok: boolean; academic_year: string }>("/teacher/assign", {
    method: "POST",
    body,
  }),

  // Uthibitisho
  verifyNecta: (body: { form4_index: string; full_name?: string }) =>
    epCall<EpVerifyResult>("/verify/necta", { method: "POST", body }),
  verifyRita: (body: { birth_cert_no: string; full_name?: string }) =>
    epCall<EpVerifyResult>("/verify/rita", { method: "POST", body }),

  // Usajili + enrollment
  admit: (body: {
    full_name: string;
    course_id: string;
    level_id: string;
    gender?: string;
    dob?: string;
    phone?: string;
    email?: string;
    form4_index?: string;
    birth_cert_no?: string;
    necta_result?: string;
    necta_verified?: string;
    rita_verified?: string;
    photo?: string;
    username?: string;
    password?: string;
    verified_by?: string;
    allow_unverified?: boolean;
  }) => epCall<EpAdmitResult>("/student/admit", { method: "POST", body }),

  students: (levelId?: string) => {
    const qs = levelId ? `?level_id=${encodeURIComponent(levelId)}` : "";
    return epCall<{ ok: boolean; students: EpStudent[] }>(`/students${qs}`);
  },
  student: (id: string) => epCall<EpStudentDetail>(`/student/${id}`),
};

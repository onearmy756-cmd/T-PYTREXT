/**
 * Clearance — ELIMUPRO Awamu 14 (moduli 14).
 * Hupiga `/api/clearance/*` (server/agent/src/clearance.rs).
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

async function clrCall<T>(path: string, body?: unknown): Promise<T> {
  const base = getAgentServerBaseUrl();
  if (!base) throw new Error("Backend base URL haijulikani");
  const url = `${base.replace(/\/+$/, "")}/api/clearance${path}`;
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

export interface ClearanceStatus {
  ok: boolean;
  templates: number;
  departments: number;
  clearances: number;
  pending: number;
  cleared: number;
  rejected: number;
  core_departments: { code: string; name: string }[];
}

export interface ClearanceTemplate {
  id: string;
  name: string;
  pdf_path: string;
  seal_text: string;
  active: number;
  created_at: string;
}

export interface ClearanceSection {
  department_code: string;
  status: string;
  comment: string;
  signed_by: string;
  signed_at: string | null;
}

export interface Clearance {
  id: string;
  student_id: string;
  template_id: string;
  academic_year: string;
  status: string;
  seal_text?: string;
  cleared_at: string | null;
  sections: ClearanceSection[];
}

export const clearanceService = {
  status: () => clrCall<ClearanceStatus>("/status"),
  templates: () => clrCall<{ ok: boolean; templates: ClearanceTemplate[] }>("/templates"),
  addTemplate: (body: { name: string; pdf_path?: string; seal_text?: string }) =>
    clrCall<{ ok: boolean; id?: string; error?: string }>("/template/add", body),
  addDepartment: (body: {
    template_id: string;
    code: string;
    name?: string;
    blocking?: boolean;
    sort_order?: number;
  }) => clrCall<{ ok: boolean; id?: string; error?: string }>("/department/add", body),
  templateGet: (id: string) =>
    clrCall<{
      ok: boolean;
      template?: ClearanceTemplate;
      departments?: { code: string; name: string; blocking: boolean; sort_order: number }[];
    }>(`/template/${id}`),
  request: (body: { student_id: string; template_id: string; academic_year?: string }) =>
    clrCall<{ ok: boolean; id?: string; academic_year?: string; sections?: number; error?: string }>(
      "/request",
      body,
    ),
  list: (params?: { student_id?: string; status?: string; department?: string }) => {
    const q = new URLSearchParams();
    if (params?.student_id) q.set("student_id", params.student_id);
    if (params?.status) q.set("status", params.status);
    if (params?.department) q.set("department", params.department);
    const qs = q.toString();
    return clrCall<{ ok: boolean; clearances: Clearance[] }>(`/list${qs ? `?${qs}` : ""}`);
  },
  get: (id: string) =>
    clrCall<{ ok: boolean; clearance: Clearance; sections: ClearanceSection[] }>(`/${id}`),
  sign: (body: {
    clearance_id: string;
    department: string;
    signed_by?: string;
    comment?: string;
    force?: boolean;
  }) =>
    clrCall<{ ok: boolean; status?: string; clearance_status?: string; error?: string; blocked?: boolean }>(
      "/section/sign",
      body,
    ),
  reject: (body: { clearance_id: string; department: string; signed_by?: string; comment?: string }) =>
    clrCall<{ ok: boolean; error?: string }>("/section/reject", body),
  verify: (id: string) =>
    clrCall<{
      ok: boolean;
      valid: boolean;
      student_id: string;
      academic_year: string;
      status: string;
      seal: string;
      sections: ClearanceSection[];
    }>(`/${id}/verify`),
};

/**
 * MTECHOS GROUP — biashara, malipo (Stripe + escrow), ajira, SMS na kampuni ya
 * wakala. Hupiga endpoints halisi za `/api/biashara/*`, `/api/escrow/*`,
 * `/api/ajira/*`, `/api/sms/*` na `/api/agents/company/*` kwenye fundi-deploy.
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

async function call<T>(base: string, path: string, body?: unknown): Promise<T> {
  const url = `${base.replace(/\/+$/, "")}${path}`;
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

function base(): string {
  const b = getAgentServerBaseUrl();
  if (!b) throw new Error("Backend base URL haijulikani");
  return b;
}

const bz = <T,>(path: string, body?: unknown) =>
  call<T>(base(), `/api/biashara${path}`, body);
const esc = <T,>(path: string, body?: unknown) =>
  call<T>(base(), `/api/escrow${path}`, body);
const aj = <T,>(path: string, body?: unknown) =>
  call<T>(base(), `/api/ajira${path}`, body);
const sms = <T,>(path: string, body?: unknown) =>
  call<T>(base(), `/api/sms${path}`, body);
const ak = <T,>(path: string, body?: unknown) =>
  call<T>(base(), `/api/agents/company${path}`, body);

export interface BizStatus {
  ok: boolean;
  services: number;
  employees: number;
  requests: number;
  contracts: number;
  currencies: number;
  tools: number;
  regions: number;
  countries: number;
  pending_approval: number;
  active: number;
  completed: number;
}

export interface Service {
  id: string;
  code: string;
  name: string;
  name_sw: string;
  category: string;
  audience: string;
  base_price: string;
  currency: string;
}

export interface Employee {
  id: string;
  user_id: string;
  full_name: string;
  photo: string;
  profession: string;
  level: string;
  department_id: string;
  phone: string;
  email: string;
  tools: string[];
  status: string;
}

export interface BizRequest {
  id: string;
  ref: string;
  client_name: string;
  service_id: string;
  title: string;
  country: string;
  region: string;
  currency: string;
  amount: string;
  amount_cents: number;
  status: string;
  progress: number;
  priority: string;
  assignee_id: string;
  created_at: string;
  needs_admin: boolean;
  in_flight: boolean;
}

export interface TrackEvent {
  actor: string;
  kind: string;
  note: string;
  progress: number;
  at: string;
}

export interface Track {
  ok: boolean;
  ref: string;
  title: string;
  status: string;
  progress: number;
  stage: string;
  timeline: TrackEvent[];
}

/** Mwonekano wa mteja: HAUNA simu/barua pepe za mfanyakazi (umma). */
export interface ClientTrack {
  ok: boolean;
  ref: string;
  title: string;
  service: string;
  service_name: string;
  status: string;
  stage: string;
  progress: number;
  amount: string;
  currency: string;
  assignee: { full_name: string; profession: string } | null;
  paid: boolean;
  contracts: {
    title: string;
    status: string;
    currency: string;
    amount_cents: number;
  }[];
  timeline: TrackEvent[];
  error?: string;
}

export const biasharaService = {
  status: () => bz<BizStatus>("/status"),
  currencies: () =>
    bz<{ currencies: { code: string; name: string; rate_tzs: number; stripe: boolean }[] }>(
      "/currencies",
    ),
  convert: (amount: string, from: string, to: string) =>
    bz<{ ok: boolean; amount: string; from: string; converted: string; to: string; rate: number }>(
      `/convert?amount=${encodeURIComponent(amount)}&from=${from}&to=${to}`,
    ),
  services: (params?: { category?: string; country?: string }) => {
    const q = new URLSearchParams();
    if (params?.category) q.set("category", params.category);
    if (params?.country) q.set("country", params.country);
    const qs = q.toString();
    return bz<{ total: number; services: Service[] }>(`/services${qs ? `?${qs}` : ""}`);
  },
  departments: () =>
    bz<{ departments: { id: string; code: string; name: string; tier: string }[] }>("/departments"),
  tools: () =>
    bz<{ total: number; tools: { code: string; name: string; category: string }[] }>("/tools"),

  employees: () => bz<{ employees: Employee[] }>("/employees"),
  addEmployee: (body: {
    full_name: string;
    profession?: string;
    level?: string;
    department?: string;
    phone?: string;
    email?: string;
    photo?: string;
    tools?: string[];
  }) => bz<{ ok: boolean; id?: string; error?: string }>("/employee/add", body),
  employee: (id: string) =>
    bz<{ ok: boolean; employee: Employee; granted_tools: string[] }>(`/employee/${id}`),
  setEmployeeTools: (id: string, tools: string[]) =>
    bz<{ ok: boolean; tools: string[]; error?: string }>(`/employee/${id}/tools`, { tools }),
  setEmployeeStatus: (id: string, status: string) =>
    bz<{ ok: boolean; status: string }>(`/employee/${id}/status`, { status }),

  requests: (params?: { status?: string }) => {
    const q = params?.status ? `?status=${encodeURIComponent(params.status)}` : "";
    return bz<{ requests: BizRequest[] }>(`/requests${q}`);
  },
  request: (id: string) =>
    bz<{ ok: boolean; request: BizRequest; events: TrackEvent[] }>(`/request/${id}`),
  createRequest: (body: {
    client_name: string;
    client_phone: string;
    client_email?: string;
    service: string;
    title: string;
    description?: string;
    country?: string;
    region?: string;
    currency?: string;
    amount: string;
    priority?: string;
  }) =>
    bz<{
      ok: boolean;
      ref?: string;
      id?: string;
      amount?: string;
      usd_equivalent?: string;
      error?: string;
    }>("/request/create", body),
  approve: (id: string) => bz<{ ok: boolean; error?: string }>(`/request/${id}/approve`, {}),
  reject: (id: string) => bz<{ ok: boolean; error?: string }>(`/request/${id}/reject`, {}),
  assign: (id: string, employee: string) =>
    bz<{ ok: boolean; assignee_name?: string; error?: string }>(`/request/${id}/assign`, {
      employee,
    }),
  autoAssign: (id: string) =>
    bz<{ ok: boolean; assignee?: string; assignee_name?: string; error?: string }>(
      `/request/${id}/auto-assign`,
      {},
    ),
  employeeTasks: (id: string) =>
    bz<{
      ok: boolean;
      employee: { id: string; full_name: string; level: string; tools: string[] };
      open: number;
      completed: number;
      tasks: BizRequest[];
      error?: string;
    }>(`/employee/${id}/tasks`),
  /** Umma: mteja hufuatilia ombi kwa kumbukumbu + namba ya simu. */
  clientTrack: (reference: string, phone: string) =>
    bz<ClientTrack>("/client/track", { reference, phone }),
  progress: (id: string, body: { progress: number; note?: string; actor?: string }) =>
    bz<{ ok: boolean; progress: number }>(`/request/${id}/progress`, body),
  review: (id: string) => bz<{ ok: boolean; error?: string }>(`/request/${id}/review`, {}),
  deliver: (id: string) => bz<{ ok: boolean; error?: string }>(`/request/${id}/deliver`, {}),
  close: (id: string) => bz<{ ok: boolean; error?: string }>(`/request/${id}/close`, {}),
  track: (id: string) => bz<Track>(`/request/${id}/track`),

  contracts: () =>
    bz<{
      contracts: {
        id: string;
        request_id: string;
        title: string;
        amount: string;
        currency: string;
        status: string;
      }[];
    }>("/contracts"),
  createContract: (body: {
    request: string;
    title: string;
    scope?: string;
    amount: string;
    currency?: string;
  }) => bz<{ ok: boolean; id?: string; seal_hash?: string; error?: string }>("/contract/create", body),
  signContract: (id: string, party: "admin" | "client") =>
    bz<{ ok: boolean; status: string; alg: string; error?: string }>(`/contract/${id}/sign`, {
      party,
    }),
  verifyContract: (id: string) =>
    bz<{
      ok: boolean;
      client_signature_valid: boolean;
      admin_signature_valid: boolean;
      fully_signed: boolean;
      alg: string;
      key_fingerprint: string;
    }>(`/contract/${id}/verify`),

  coverage: () =>
    bz<{
      regions_total: number;
      countries_total: number;
      regions: { code: string; name: string; status: string; requests: number }[];
      countries: { code: string; name: string; status: string; requests: number }[];
    }>("/coverage"),
  audit: () =>
    bz<{ audit: { actor: string; action: string; target: string; detail: string; at: string }[] }>(
      "/audit",
    ),

  branding: () => bz<{ ok: boolean; branding: Branding }>("/branding"),
  setBranding: (body: {
    company_name?: string;
    tagline?: string;
    primary_color?: string;
    accent_color?: string;
    surface_color?: string;
  }) => bz<{ ok: boolean; changed?: string[]; error?: string }>("/branding/set", body),
  setLogo: (data: string, kind?: "logo" | "favicon") =>
    bz<{ ok: boolean; key?: string; bytes?: number; error?: string }>("/branding/logo", {
      data,
      kind,
    }),
  resetBranding: () =>
    bz<{ ok: boolean; note?: string }>("/branding/reset", {}),
};

export interface Branding {
  company_name: string;
  tagline: string;
  primary_color: string;
  accent_color: string;
  surface_color: string;
  logo: string;
  favicon: string;
}

export const escrowService = {
  status: () =>
    esc<{
      stripe_configured: boolean;
      company_account: Record<string, unknown>;
      payments: number;
      paid: number;
      in_escrow: string;
      released: string;
    }>("/status"),
  gateways: () =>
    esc<{
      stripe_configured: boolean;
      gateways: { code: string; name: string; currencies: string[] }[];
    }>("/gateways"),
  checkout: (body: {
    request: string;
    client_name: string;
    amount: string;
    currency?: string;
    country?: string;
  }) =>
    esc<{
      ok: boolean;
      checkout_url?: string;
      gateway?: string;
      note?: string;
      error?: string;
    }>("/checkout", body),
  confirm: (payment: string, provider_ref?: string) =>
    esc<{ ok: boolean; status: string; held?: string; error?: string }>("/confirm", {
      payment,
      provider_ref,
    }),
  release: (request: string, amount?: string, note?: string) =>
    esc<{ ok: boolean; amount?: string; to_account?: Record<string, unknown>; error?: string }>(
      "/release",
      { request, amount, note },
    ),
  refund: (request: string, amount?: string, note?: string) =>
    esc<{ ok: boolean; amount?: string; error?: string }>("/refund", { request, amount, note }),
  payments: () =>
    esc<{
      payments: {
        id: string;
        client_name: string;
        gateway: string;
        currency: string;
        amount: string;
        status: string;
        created_at: string;
      }[];
    }>("/payments"),
  ledger: () =>
    esc<{
      ledger: {
        id: string;
        request_id: string;
        kind: string;
        amount: string;
        currency: string;
        status: string;
        at: string;
      }[];
    }>("/ledger"),
  balance: (request: string) => esc<{ held: string }>(`/balance/${request}`),
};

export interface JobRole {
  id: string;
  code: string;
  title: string;
  department_id: string;
  service_code: string;
  level: string;
  location: string;
  kind: string;
  salary_min: string;
  salary_max: string;
  currency: string;
  status: string;
}

export interface Application {
  id: string;
  ref: string;
  role_id: string;
  full_name: string;
  email: string;
  phone: string;
  education: string;
  experience_years: number;
  status: string;
  score_theory: number;
  score_practical: number;
  total_score: number;
  passed: boolean;
}

export const ajiraService = {
  status: () =>
    aj<{
      open_roles: number;
      applications: number;
      assessments: number;
      questions: number;
      pass_mark: number;
      stages: string[];
    }>("/status"),
  roles: (status?: string) => {
    const q = status ? `?status=${status}` : "";
    return aj<{ roles: JobRole[] }>(`/roles${q}`);
  },
  addRole: (body: {
    title: string;
    department?: string;
    service?: string;
    level?: string;
    location?: string;
    kind?: string;
    description?: string;
    requirements?: string;
    salary_min?: string;
    salary_max?: string;
    currency?: string;
  }) => aj<{ ok: boolean; id?: string; error?: string }>("/role/add", body),
  closeRole: (id: string) => aj<{ ok: boolean }>(`/role/${id}/close`, {}),
  applications: (params?: { status?: string; role?: string }) => {
    const q = new URLSearchParams();
    if (params?.status) q.set("status", params.status);
    if (params?.role) q.set("role", params.role);
    const qs = q.toString();
    return aj<{ applications: Application[] }>(`/applications${qs ? `?${qs}` : ""}`);
  },
  apply: (body: {
    role: string;
    full_name: string;
    email: string;
    phone: string;
    education?: string;
    experience_years?: number;
    skills?: string[];
    country?: string;
    region?: string;
  }) => aj<{ ok: boolean; ref?: string; error?: string }>("/apply", body),
  application: (id: string) =>
    aj<{
      ok: boolean;
      application: Application;
      assessments: { id: string; kind: string; task: string; score: number; status: string }[];
    }>(`/application/${id}`),
  setStage: (id: string, stage: string) =>
    aj<{ ok: boolean; error?: string }>(`/application/${id}/stage`, { stage }),
  startTheory: (id: string) =>
    aj<{
      ok: boolean;
      assessment_id?: string;
      questions?: { id: string; question: string; options: string[]; marks: number }[];
      error?: string;
    }>(`/application/${id}/theory/start`, {}),
  submitTheory: (id: string, assessment: string, answers: string[]) =>
    aj<{ ok: boolean; score: number; passed: boolean; error?: string }>(
      `/application/${id}/theory/submit`,
      { assessment, answers },
    ),
  assignPractical: (id: string, task: string, criteria?: string[]) =>
    aj<{ ok: boolean; assessment_id?: string; criteria?: string[]; error?: string }>(
      `/application/${id}/practical/assign`,
      { task, criteria },
    ),
  submitPractical: (id: string, assessment: string, submission: string) =>
    aj<{ ok: boolean; status: string; error?: string }>(
      `/application/${id}/practical/submit`,
      { assessment, submission },
    ),
  gradePractical: (id: string, assessment: string, scores: number[], grader?: string) =>
    aj<{ ok: boolean; score: number; passed: boolean; error?: string }>(
      `/application/${id}/practical/grade`,
      { assessment, scores, grader },
    ),
  hire: (id: string, body: { department?: string; tools?: string[]; level?: string }) =>
    aj<{ ok: boolean; employee_id?: string; error?: string }>(`/application/${id}/hire`, body),
  reject: (id: string) => aj<{ ok: boolean }>(`/application/${id}/reject`, {}),
  questions: () =>
    aj<{ questions: { id: string; family: string; question: string; answer: string }[] }>(
      "/questions",
    ),
};

export interface CompanyAgent {
  id: string;
  name: string;
  kind: string;
  department: string;
  specialty: string;
  level: string;
  manager_id: string;
  model: string;
  status: string;
  autonomy: boolean;
  tasks_done: number;
}

export const agentCompanyService = {
  status: () =>
    ak<{
      agents: number;
      managers: number;
      workers: number;
      busy: number;
      tasks: number;
      pending_approval: number;
      denied_runs: number;
      autonomy: { enabled: boolean; mode: string; max_concurrent: number };
    }>("/status"),
  org: () => ak<{ managers: CompanyAgent[]; workers: CompanyAgent[]; total: number }>("/org"),
  agents: (department?: string) => {
    const q = department ? `?department=${department}` : "";
    return ak<{ agents: CompanyAgent[] }>(`/agents${q}`);
  },
  agent: (id: string) =>
    ak<{
      ok: boolean;
      agent: CompanyAgent & {
        tools: string[];
        tasks: { id: string; ref: string; title: string; status: string }[];
      };
    }>(`/agent/${id}`),
  setTools: (id: string, tools: string[]) =>
    ak<{ ok: boolean; error?: string }>(`/agent/${id}/tools`, { tools }),
  setStatus: (id: string, status: string) => ak<{ ok: boolean }>(`/agent/${id}/status`, { status }),
  run: (id: string, tool: string, input: string) =>
    ak<{ ok: boolean; allowed: boolean; output?: string; error?: string }>(`/agent/${id}/run`, {
      tool,
      input,
    }),
  tasks: (status?: string) => {
    const q = status ? `?status=${status}` : "";
    return ak<{
      tasks: {
        id: string;
        ref: string;
        title: string;
        tool_code: string;
        status: string;
        priority: string;
        can_start: boolean;
      }[];
    }>(`/tasks${q}`);
  },
  createTask: (body: {
    title: string;
    detail?: string;
    request?: string;
    agent?: string;
    tool?: string;
    priority?: string;
  }) => ak<{ ok: boolean; ref?: string; error?: string }>("/task/create", body),
  approveTask: (id: string) => ak<{ ok: boolean; error?: string }>(`/task/${id}/approve`, {}),
  assignTask: (id: string, agent: string) =>
    ak<{ ok: boolean; error?: string }>(`/task/${id}/assign`, { agent }),
  runTask: (id: string) =>
    ak<{ ok: boolean; status: string; result?: string; error?: string }>(`/task/${id}/run`, {}),
  completeTask: (id: string) => ak<{ ok: boolean; error?: string }>(`/task/${id}/complete`, {}),
  reviewTask: (id: string) => ak<{ ok: boolean; error?: string }>(`/task/${id}/review`, {}),
  autonomy: () =>
    ak<{ autonomy: { enabled: boolean; mode: string; max_concurrent: number } }>("/autonomy"),
  setAutonomy: (enabled: boolean, mode?: string, max_concurrent?: number) =>
    ak<{ ok: boolean; autonomy: { enabled: boolean; mode: string } }>("/autonomy", {
      enabled,
      mode,
      max_concurrent,
    }),
  runs: () =>
    ak<{
      runs: {
        agent_id: string;
        task_id: string;
        tool_code: string;
        allowed: boolean;
        error: string;
        at: string;
      }[];
    }>("/runs"),
};

export const smsService = {
  status: () =>
    sms<{
      configured: boolean;
      sender: string;
      admin_phone: string;
      queued: number;
      sent: number;
      failed: number;
      inbox: number;
      pending_commands: number;
    }>("/status"),
  send: (to: string, body: string, kind?: string, ref?: string) =>
    sms<{ ok: boolean; sent: boolean; status: string; note: string; error?: string }>("/send", {
      to,
      body,
      kind,
      ref,
    }),
  outbox: () =>
    sms<{
      messages: {
        id: string;
        to: string;
        body: string;
        kind: string;
        ref: string;
        status: string;
        created_at: string;
      }[];
    }>("/outbox"),
  inbox: () =>
    sms<{ inbox: { id: string; from: string; body: string; status: string; created_at: string }[] }>(
      "/inbox",
    ),
  commands: () =>
    sms<{
      commands: {
        id: string;
        from: string;
        action: string;
        ref: string;
        raw: string;
        status: string;
        created_at: string;
      }[];
    }>("/commands"),
  applyCommand: (id: string) =>
    sms<{ ok: boolean; action: string; ref: string; status: string; result?: unknown; error?: string }>(
      `/command/${id}/apply`,
      {},
    ),
};

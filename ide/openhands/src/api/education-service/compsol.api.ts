/**
 * Computer Solutions — MTECHOS Sehemu 6 (moduli 22).
 * Hupiga `/api/compsol/*` (server/agent/src/compsol.rs).
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

function base(): string {
  const b = getAgentServerBaseUrl();
  if (!b) throw new Error("Backend base URL haijulikani");
  return b.replace(/\/+$/, "");
}

async function call<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetch(`${base()}/api/compsol${path}`, {
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

export interface CompSolStatus {
  ok: boolean;
  module: string;
  part: string;
  os: string;
  capabilities: Record<string, unknown>;
  diagnostics: number;
  fixes: number;
}

export interface Finding {
  severity: "ok" | "info" | "warning" | "critical";
  code: string;
  message: string;
  hint: string;
}

export interface TopProcess {
  name: string;
  cpu: number;
  mem: number;
}

export interface Diagnostic {
  ok: boolean;
  id: string;
  os: string;
  hostname: string;
  cpu_pct: number;
  ram_pct: number;
  disk_pct: number;
  load1: number;
  processes: number;
  top_processes: TopProcess[];
  severity: string;
  findings: Finding[];
  note_sw: string;
}

export interface CommandEntry {
  topic: string;
  windows: string;
  macos: string;
  linux: string;
  maelezo: string;
}

export interface SafeAction {
  id: string;
  label: string;
  cmd: string;
  risk: string;
}

export interface FixPlan {
  ok: boolean;
  diag_id: string;
  plan: { ufafanuzi?: string; hatua?: string[]; amri_salama?: string[] };
  available_actions: SafeAction[];
  note_sw: string;
}

export interface DiagRow {
  id: string;
  os: string;
  hostname: string;
  cpu_pct: number;
  ram_pct: number;
  disk_pct: number;
  processes: number;
  severity: string;
  created_at: string;
}

export const compsolService = {
  status: () => call<CompSolStatus>("/status"),
  diagnose: () => call<Diagnostic>("/diagnose", {}),
  commands: () =>
    call<{ ok: boolean; os_yako: string; catalog: CommandEntry[]; note_sw: string }>(
      "/commands",
    ),
  vision: (image_url: string, question?: string) =>
    call<{ ok: boolean; analysis?: unknown; error?: string }>("/vision", {
      image_url,
      question: question ?? "",
    }),
  fixPlan: (body: { diag_id?: string; problem?: string }) =>
    call<FixPlan>("/fix/plan", body),
  fixApply: (action: string, approved: boolean) =>
    call<{ ok: boolean; action: string; status: string; output: string; note_sw: string }>(
      "/fix/apply",
      { action, approved },
    ),
  diagnostics: () => call<{ ok: boolean; diagnostics: DiagRow[] }>("/diagnostics"),
  fixes: () =>
    call<{
      ok: boolean;
      fixes: { id: string; action: string; status: string; output: string; created_at: string }[];
    }>("/fixes"),
};

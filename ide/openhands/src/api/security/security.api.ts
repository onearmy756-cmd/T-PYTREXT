/**
 * MTECHOS CYBER SECURITY — mteja wa API wa `/api/security/*` (fundi-deploy).
 *
 * Hupiga endpoints halisi: katalogi za zana (Kali/Windows/macOS/Android/UOS),
 * vikundi vya zana (ClamAV n.k.), finshield (kadi/IBAN/PCI/rate-limit),
 * apiauth (jwt/oauth), gateprobe (uchunguzi wa lango la API), threatswarm
 * (kill-chain ya wakala), agentbind (ufungaji wa wakala↔jukumu), digital
 * forensics (DFIR) na utekelezaji wa kazi zenye HITL.
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

async function call<T>(path: string, body?: unknown): Promise<T> {
  const b = getAgentServerBaseUrl();
  if (!b) throw new Error("Backend base URL haijulikani");
  const res = await fetch(`${b.replace(/\/+$/, "")}/api/security${path}`, {
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

export interface SecTask {
  id: string;
  name_sw: string;
  cat: string;
  high_risk: boolean;
  needs_target: boolean;
}

export interface SecGroup {
  id: string;
  title: string;
  desc: string;
  cat: string;
  tool_count: number;
  tools: string[];
  tasks: SecTask[];
}

export interface SecRun {
  id: string;
  task: string;
  target: string;
  status: string;
  at: string;
  output?: string;
}

export interface SecPlatformCategory {
  id: string;
  title: string;
  commands: {
    name: string;
    desc: string;
    usage: string;
    risk: string;
    needs_root: boolean;
    target: string;
  }[];
}

/** `/platforms/full` — haina `platform` id; huunganishwa kwa `title` na `/platforms`. */
export interface SecPlatform {
  title: string;
  shell: string;
  categories: SecPlatformCategory[];
}

/** `/platforms` (muhtasari) — ndiyo chanzo cha `platform` id. */
export interface SecPlatformSummary {
  platform: string;
  title: string;
  shell: string;
  categories: number;
  commands: number;
}

export interface SwarmAgent {
  name: string;
  title: string;
  description: string;
  class: string;
  tools: string[];
  tasks: string[];
}

export interface DfirAgent {
  agent: string;
  title: string;
  tools: string[];
  tasks: string[];
}

export interface AgentBinding {
  agent: string;
  class: string;
  platform: string;
  task: string;
}

export interface FinOp {
  op: string;
  desc: string;
}

export const securityService = {
  /** Vikundi vya zana (ClamAV, Kali n.k.) pamoja na kazi zake. */
  groups: () => call<{ ok: boolean; groups: SecGroup[] }>("/groups"),
  /** Upatikanaji halisi wa zana kwenye mashine hii. */
  availability: () =>
    call<{
      ok: boolean;
      groups: {
        id: string;
        title: string;
        tools_available: number;
        tools_total: number;
        available: string[];
      }[];
    }>("/availability"),
  /** Hali ya ngao ya AI (patterns, matukio). */
  shield: () =>
    call<{
      ok: boolean;
      active: boolean;
      patterns: number;
      total_events: number;
      recent: unknown[];
      last_event: string | null;
    }>("/shield"),
  /** Hali ya injini: modules, swarm, platforms, finshield ops. */
  engineStatus: () =>
    call<{
      ok: boolean;
      engine: string;
      modules: number;
      platforms: number;
      platform_commands: number;
      swarm_agents: number;
      swarm_languages: string[];
      apiauth_ops: string[];
      /** Idadi ya ops za finshield (namba, si orodha). */
      finshield_ops: number;
      finshield_catalog: { ops: string[]; capabilities: FinOp[]; rules?: Record<string, string> };
      hitl: unknown;
      scope: unknown;
      surveillance: unknown;
      gateprobe: unknown;
      forensics: unknown;
    }>("/engine/status"),
  /** Katalogi ya majukwaa (Kali/Windows/macOS/Android/UOS). */
  platforms: () =>
    call<{ ok: boolean; total_commands?: number; platforms: SecPlatformSummary[] }>(
      "/platforms",
    ),
  platformsFull: () => call<{ ok: boolean; platforms: SecPlatform[] }>("/platforms/full"),
  /** Panga amri kwa jukwaa (haitekelezi — katalogi pekee). */
  platformPlan: (platform: string, task: string, target: string) =>
    call<Record<string, unknown>>("/platforms/plan", { platform, task, target }),
  /** Finshield: luhn/pan/mask/iban/pci/secrets/bucket/strength/txn_risk n.k. */
  finshield: (op: string, data: unknown) =>
    call<Record<string, unknown>>("/finshield", { op, data }),
  /** Apiauth: jwt/oauth/nacos. */
  apiauth: (op: string, data: unknown) =>
    call<Record<string, unknown>>("/apiauth", { op, data }),
  /** Uchunguzi wa lango la API (huruhusiwa tu ukiwa umeidhinishwa). */
  gateprobe: (url: string, authorized: boolean) =>
    call<Record<string, unknown>>("/gateprobe", { url, authorized }),
  /** Katalogi ya threatswarm (wakala wa kill-chain). */
  swarmCatalog: () => call<{ ok: boolean; agents: SwarmAgent[] }>("/swarm/catalog"),
  swarmPlan: (mode: string, target: string, scope: string) =>
    call<Record<string, unknown>>("/swarm/plan", { mode, target, scope }),
  swarmConduct: (targets: string[], ports: number[], mode: string, scope: string) =>
    call<Record<string, unknown>>("/swarm/conduct", { targets, ports, mode, scope }),
  /** Ufungaji wa wakala ↔ jukumu ↔ jukwaa. */
  agentbind: () => call<{ ok: boolean; bindings: AgentBinding[] }>("/agentbind"),
  agentbindFinops: () => call<Record<string, unknown>>("/agentbind/finops"),
  agentbindAuthops: () => call<Record<string, unknown>>("/agentbind/authops"),
  agentbindPlan: (agent: string, target: string) =>
    call<Record<string, unknown>>("/agentbind/plan", { agent, target }),
  agentbindConduct: (target: string) =>
    call<Record<string, unknown>>(
      `/agentbind/conduct?target=${encodeURIComponent(target)}`,
    ),
  /** Digital forensics (DFIR): wakala + zana. */
  forensics: () =>
    call<{ ok: boolean; dfir_agent_count: number; dfir_agents: DfirAgent[] }>("/forensics"),
  /** Endesha kazi ya usalama (HITL kwa high_risk). */
  runTask: (task: string, target: string, human_approved: boolean) =>
    call<Record<string, unknown>>("/task", { task, target, human_approved }),
  /** Endesha kazi kwa malengo mengi. */
  mass: (task: string, targets: string[], limit: number, human_approved: boolean) =>
    call<Record<string, unknown>>("/mass", { task, targets, limit, human_approved }),
  /** Kumbukumbu za kazi zilizotekelezwa. */
  runs: () => call<{ ok: boolean; count: number; runs: SecRun[] }>("/runs"),
  /** Ripoti ya HTML (hupigwa kwa kichupo kipya). */
  reportUrl: () => {
    const b = getAgentServerBaseUrl();
    return b ? `${b.replace(/\/+$/, "")}/api/security/report` : "";
  },
};

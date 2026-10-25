/**
 * Offline mode / LAN — ELIMUPRO Awamu 12 (moduli 12).
 * Hupiga `/api/offline/*` (server/agent/src/offline.rs).
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

async function offCall<T>(path: string, body?: unknown): Promise<T> {
  const base = getAgentServerBaseUrl();
  if (!base) throw new Error("Backend base URL haijulikani");
  const url = `${base.replace(/\/+$/, "")}/api/offline${path}`;
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

export interface OfflineStatus {
  ok: boolean;
  mode: string;
  state: "online" | "offline";
  internet: boolean;
  server_role: string;
  lan_ips: string[];
  nodes: number;
  nodes_live: number;
  queue_pending: number;
  queue_failed: number;
  queue_total: number;
}

export interface SyncItem {
  id: string;
  entity: string;
  op: string;
  payload: string;
  status: string;
  attempts: number;
  last_error: string;
  created_at: string;
  synced_at: string | null;
}

export interface LanNode {
  id: string;
  name: string;
  role: string;
  ip: string;
  kind: string;
  last_seen: string;
  state: "live" | "idle" | "gone";
}

export const offlineService = {
  status: () => offCall<OfflineStatus>("/status"),
  settings: () => offCall<{ ok: boolean; mode: string; cloud_url: string }>("/settings"),
  setSettings: (body: { mode: string; cloud_url?: string }) =>
    offCall<{ ok: boolean; mode: string }>("/settings/set", body),
  queueAdd: (body: { entity: string; op?: string; payload?: unknown }) =>
    offCall<{ ok: boolean; id?: string; error?: string }>("/queue/add", body),
  queue: (status?: string) =>
    offCall<{ ok: boolean; queue: SyncItem[] }>(`/queue${status ? `?status=${status}` : ""}`),
  sync: () =>
    offCall<{
      ok: boolean;
      synced: number;
      failed: number;
      local_commits: number;
      cloud: string;
      internet: boolean;
    }>("/sync", {}),
  registerNode: (body: { name?: string; ip?: string; role?: string; kind?: string }) =>
    offCall<{ ok: boolean; id?: string; ip?: string; error?: string }>("/node/register", body),
  nodes: () => offCall<{ ok: boolean; nodes: LanNode[] }>("/nodes"),
  leaveNode: (id: string) => offCall<{ ok: boolean }>(`/node/${id}/leave`, {}),
};

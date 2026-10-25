/**
 * VoIP ya LAN — ELIMUPRO Awamu 13 (moduli 13).
 * Hupiga `/api/voip/*` (server/agent/src/voip.rs).
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

async function voipCall<T>(path: string, body?: unknown): Promise<T> {
  const base = getAgentServerBaseUrl();
  if (!base) throw new Error("Backend base URL haijulikani");
  const url = `${base.replace(/\/+$/, "")}/api/voip${path}`;
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

export interface VoipStatus {
  ok: boolean;
  mode: string;
  users: number;
  online: number;
  ringing: number;
  active: number;
  unread: number;
  messages: number;
  files: number;
  max_file_bytes: number;
}

export interface VoipUser {
  id: string;
  user_id: string;
  name: string;
  kind: string;
  ip: string;
  extension: string;
  status: string;
}

export interface VoipCall {
  id: string;
  caller_id: string;
  callee_id: string;
  kind: string;
  status: string;
  started_at: string;
  answered_at: string | null;
  ended_at: string | null;
  duration_s: number;
}

export interface VoipMessage {
  id: string;
  from_id: string;
  to_id: string;
  body: string;
  status: string;
  created_at: string;
}

export interface VoipFile {
  id: string;
  from_id: string;
  to_id: string;
  name: string;
  size: number;
  mime: string;
  lan_url: string;
  status: string;
  created_at: string;
}

export const voipService = {
  status: () => voipCall<VoipStatus>("/status"),
  presence: (body: {
    user_id: string;
    name?: string;
    ip?: string;
    kind?: string;
    extension?: string;
    status?: string;
  }) => voipCall<{ ok: boolean; id?: string; status?: string; error?: string }>("/presence", body),
  users: () => voipCall<{ ok: boolean; users: VoipUser[] }>("/users"),
  startCall: (body: { caller_id: string; callee_id: string; kind?: string }) =>
    voipCall<{ ok: boolean; id?: string; status?: string; kind?: string; error?: string }>("/call", body),
  answer: (id: string) => voipCall<{ ok: boolean; status?: string; error?: string }>(`/call/${id}/answer`, {}),
  reject: (id: string) => voipCall<{ ok: boolean; status?: string; error?: string }>(`/call/${id}/reject`, {}),
  end: (id: string) => voipCall<{ ok: boolean; status?: string; error?: string }>(`/call/${id}/end`, {}),
  calls: (userId?: string) =>
    voipCall<{ ok: boolean; calls: VoipCall[] }>(`/calls${userId ? `?user_id=${userId}` : ""}`),
  sendMessage: (body: { from_id: string; to_id: string; body: string }) =>
    voipCall<{ ok: boolean; id?: string; error?: string }>("/message", body),
  messages: (userId?: string, withId?: string) => {
    const q = new URLSearchParams();
    if (userId) q.set("user_id", userId);
    if (withId) q.set("with_id", withId);
    const qs = q.toString();
    return voipCall<{ ok: boolean; messages: VoipMessage[] }>(`/messages${qs ? `?${qs}` : ""}`);
  },
  markRead: (id: string) => voipCall<{ ok: boolean; error?: string }>(`/message/${id}/read`, {}),
  offerFile: (body: {
    from_id: string;
    to_id: string;
    name: string;
    size: number;
    mime?: string;
    lan_url?: string;
  }) => voipCall<{ ok: boolean; id?: string; error?: string }>("/file", body),
  files: (userId?: string) =>
    voipCall<{ ok: boolean; files: VoipFile[] }>(`/files${userId ? `?user_id=${userId}` : ""}`),
  acceptFile: (id: string) => voipCall<{ ok: boolean; error?: string }>(`/file/${id}/accept`, {}),
  doneFile: (id: string) => voipCall<{ ok: boolean; error?: string }>(`/file/${id}/done`, {}),
};

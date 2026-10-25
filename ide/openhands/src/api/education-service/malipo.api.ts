/**
 * Malipo ya simu — ELIMUPRO Awamu 19 (moduli 19).
 * Hupiga `/api/malipo/*` (server/agent/src/malipo.rs).
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

async function payCall<T>(path: string, body?: unknown): Promise<T> {
  const base = getAgentServerBaseUrl();
  if (!base) throw new Error("Backend base URL haijulikani");
  const url = `${base.replace(/\/+$/, "")}/api/malipo${path}`;
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

export interface Provider {
  code: string;
  name: string;
  prefix: string;
}

export interface MalipoStatus {
  ok: boolean;
  intents: number;
  pending: number;
  confirmed: number;
  failed: number;
  collected_cents: number;
  collected: string;
  providers: Provider[];
}

export interface Intent {
  id: string;
  invoice_id: string;
  student_id: string;
  provider: string;
  phone: string;
  amount_cents: number;
  amount: string;
  tx_ref: string;
  status: string;
  provider_ref: string;
  confirmed_at: string | null;
}

export const malipoService = {
  status: () => payCall<MalipoStatus>("/status"),
  providers: () => payCall<{ ok: boolean; providers: Provider[] }>("/providers"),
  createIntent: (body: { invoice_id: string; provider?: string; phone: string; amount: string }) =>
    payCall<{
      ok: boolean;
      id?: string;
      tx_ref?: string;
      provider?: string;
      amount?: string;
      status?: string;
      push?: string;
      error?: string;
      balance?: string;
    }>("/intent", body),
  intents: (params?: { student_id?: string; status?: string; provider?: string }) => {
    const q = new URLSearchParams();
    if (params?.student_id) q.set("student_id", params.student_id);
    if (params?.status) q.set("status", params.status);
    if (params?.provider) q.set("provider", params.provider);
    const qs = q.toString();
    return payCall<{ ok: boolean; intents: Intent[] }>(`/intents${qs ? `?${qs}` : ""}`);
  },
  intent: (id: string) => payCall<{ ok: boolean; intent?: Intent; error?: string }>(`/intent/${id}`),
  confirm: (body: { intent_id: string; provider_ref?: string; received_by?: string }) =>
    payCall<{
      ok: boolean;
      receipt_no?: string;
      payment_id?: string;
      invoice_status?: string;
      balance_cents?: number;
      error?: string;
    }>("/confirm", body),
  fail: (body: { intent_id: string; reason?: string; cancelled?: boolean }) =>
    payCall<{ ok: boolean; status?: string; error?: string }>("/fail", body),
  reconcile: () =>
    payCall<{
      ok: boolean;
      intents_confirmed_cents: number;
      mobile_payments_cents: number;
      balanced: boolean;
      difference_cents: number;
      note: string;
    }>("/reconcile"),
};

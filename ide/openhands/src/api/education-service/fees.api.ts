/**
 * Fees + payments (ELIMUPRO Awamu 8).
 * Hupiga `/api/fees/*` kwenye fundi-deploy (server/agent/src/fees.rs).
 * Fedha zote huwasilishwa kama senti (integers) + kamba iliyopangwa.
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

async function feeCall<T>(path: string, body?: unknown): Promise<T> {
  const base = getAgentServerBaseUrl();
  if (!base) throw new Error("Backend base URL haijulikani");
  const url = `${base.replace(/\/+$/, "")}/api/fees${path}`;
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

export interface FeeStatus {
  ok: boolean;
  items: number;
  invoices: number;
  payments: number;
  billed: number;
  collected: number;
  outstanding: number;
}

export interface FeeItem {
  id: string;
  level_id: string;
  level: string;
  name: string;
  amount_cents: number;
  amount: string;
  mandatory: boolean;
  code: string;
}

export interface FeeInvoice {
  id: string;
  student_id: string;
  student: string;
  reg_no: string;
  term: string;
  total_cents: number;
  total: string;
  paid_cents: number;
  paid: string;
  balance_cents: number;
  balance: string;
  status: string;
}

export interface FeeInvoiceDetail {
  id: string;
  student: string;
  reg_no: string;
  term: string;
  total: string;
  paid: string;
  balance: string;
  status: string;
  lines: { name: string; amount_cents: number; amount: string }[];
  payments: { receipt_no: string; amount: string; method: string; paid_at: string; reference: string }[];
}

export interface FeeSummary {
  ok: boolean;
  billed: string;
  collected: string;
  outstanding: string;
  collection_rate: number;
  students: {
    student_id: string;
    student: string;
    reg_no: string;
    term: string;
    total_cents: number;
    paid_cents: number;
    balance_cents: number;
    status: string;
  }[];
}

export const feesService = {
  status: () => feeCall<FeeStatus>("/status"),

  items: (level_id?: string) =>
    feeCall<{ ok: boolean; items: FeeItem[] }>(`/items${level_id ? `?level_id=${level_id}` : ""}`),
  addItem: (body: { level_id: string; name: string; amount: string; code?: string; mandatory?: boolean }) =>
    feeCall<{ ok: boolean; id?: string; error?: string }>("/items/add", body),
  deleteItem: (id: string) => feeCall<{ ok: boolean }>("/items/delete", { id }),

  createInvoice: (body: { student_id: string; term: string; item_ids?: string[] }) =>
    feeCall<{ ok: boolean; id?: string; total_cents?: number; lines?: number; existing?: boolean; error?: string }>(
      "/invoice/create",
      body,
    ),
  invoices: (p: { level_id?: string; student_id?: string; term?: string; status?: string }) => {
    const qs = new URLSearchParams();
    Object.entries(p).forEach(([k, v]) => {
      if (v) qs.set(k, String(v));
    });
    return feeCall<{ ok: boolean; invoices: FeeInvoice[] }>(`/invoices?${qs}`);
  },
  invoice: (id: string) => feeCall<{ ok: boolean; invoice: FeeInvoiceDetail }>(`/invoice/${id}`),

  pay: (body: { invoice_id: string; amount: string; method?: string; reference?: string; received_by?: string }) =>
    feeCall<{ ok: boolean; receipt_no?: string; amount_cents?: number; status?: string; balance_cents?: number; error?: string }>(
      "/pay",
      body,
    ),
  receipt: (id: string) =>
    feeCall<{
      ok: boolean;
      receipt: { receipt_no: string; student: string; reg_no: string; amount: string; method: string; paid_at: string; reference: string };
    }>(`/receipt/${id}`),

  summary: (p: { level_id?: string; student_id?: string }) => {
    const qs = new URLSearchParams();
    Object.entries(p).forEach(([k, v]) => {
      if (v) qs.set(k, String(v));
    });
    return feeCall<FeeSummary>(`/summary?${qs}`);
  },
};

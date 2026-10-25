/**
 * Maktaba (library) — ELIMUPRO Awamu 9.
 * Hupiga `/api/library/*` kwenye fundi-deploy (server/agent/src/library.rs).
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

async function libCall<T>(path: string, body?: unknown): Promise<T> {
  const base = getAgentServerBaseUrl();
  if (!base) throw new Error("Backend base URL haijulikani");
  const url = `${base.replace(/\/+$/, "")}/api/library${path}`;
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

export interface LibStatus {
  ok: boolean;
  books: number;
  copies: number;
  loans: number;
  on_loan: number;
  outstanding_fines: string;
  max_loans: number;
  fine_per_day_cents: number;
  loan_days: number;
}

export interface LibBook {
  id: string;
  title: string;
  author: string;
  isbn: string;
  category: string;
  copies: number;
  available: number;
}

export interface LibLoan {
  id: string;
  book: string;
  student: string;
  reg_no: string;
  borrowed_at: string;
  due_at: string;
  returned_at: string | null;
  active: boolean;
  late_days: number;
  fine_cents: number;
  fine: string;
  fine_paid: boolean;
  projected_fine_cents: number;
}

export interface LibOverdue {
  loan_id: string;
  book: string;
  student: string;
  reg_no: string;
  student_id: string;
  due_at: string;
  late_days: number;
  fine_cents: number;
  fine: string;
}

export const libraryService = {
  status: () => libCall<LibStatus>("/status"),

  books: (search?: string) =>
    libCall<{ ok: boolean; books: LibBook[] }>(`/books${search ? `?search=${encodeURIComponent(search)}` : ""}`),
  addBook: (body: { title: string; author?: string; isbn?: string; category?: string; copies?: number }) =>
    libCall<{ ok: boolean; id?: string; copies?: string[]; error?: string }>("/book/add", body),
  deleteBook: (id: string) => libCall<{ ok: boolean; error?: string }>("/book/delete", { id }),
  book: (id: string) =>
    libCall<{
      ok: boolean;
      book: { id: string; title: string; author: string; isbn: string; category: string; copies: number };
      items: { id: string; barcode: string; status: string }[];
    }>(`/book/${id}`),

  borrow: (body: { student_id: string; book_id?: string; barcode?: string }) =>
    libCall<{ ok: boolean; loan_id?: string; due_at?: string; error?: string; max?: number; fine?: string }>(
      "/borrow",
      body,
    ),
  returnBook: (body: { loan_id: string; today?: string }) =>
    libCall<{ ok: boolean; late_days?: number; fine_cents?: number; fine?: string; fine_due?: boolean; error?: string }>(
      "/return",
      body,
    ),
  loans: (p: { student_id?: string; active?: boolean }) => {
    const qs = new URLSearchParams();
    if (p.student_id) qs.set("student_id", p.student_id);
    if (p.active) qs.set("active", "true");
    return libCall<{ ok: boolean; loans: LibLoan[] }>(`/loans?${qs}`);
  },
  overdue: () =>
    libCall<{ ok: boolean; count: number; total_fine: string; overdue: LibOverdue[] }>("/overdue"),
  payFine: (loan_id: string) =>
    libCall<{ ok: boolean; paid?: string; error?: string }>("/fine/pay", { loan_id }),
};

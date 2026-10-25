/**
 * Akademia ya AI — MTECHOS Sehemu 2 & 4 (moduli 23).
 * Hupiga `/api/akademia/*` (server/agent/src/akademia.rs): RAG halisi + web.
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
  const res = await fetch(`${base()}/api/akademia${path}`, {
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

export interface Domain {
  id: string;
  label: string;
  part: string;
}

export interface AkStatus {
  ok: boolean;
  module: string;
  parts: string[];
  domains: { id: string; label: string; docs: number }[];
  rag: string;
  web: string;
  llm: boolean;
  docs_total: number;
  asks: number;
  security_basics: { q: string; a: string }[];
}

export interface AskResult {
  ok: boolean;
  domain: string;
  engine: string;
  answer: string;
  doc_sources: { title: string; score: number; type: string }[];
  web_sources: { title: string; url: string; type: string }[];
  basics: { q: string; a: string; score: number }[];
  note_sw: string;
}

export interface DocRow {
  id: string;
  domain: string;
  title: string;
  preview: string;
  source: string;
  created_at: string;
}

export const akademiaService = {
  status: () => call<AkStatus>("/status"),
  domains: () => call<{ ok: boolean; domains: Domain[] }>("/domains"),
  docs: (domain?: string) =>
    call<{ ok: boolean; docs: DocRow[] }>(
      domain ? `/docs?domain=${encodeURIComponent(domain)}` : "/docs",
    ),
  docAdd: (body: { domain: string; title: string; body: string; source?: string }) =>
    call<{ ok: boolean; id?: string; domain?: string; error?: string }>("/doc/add", body),
  docRemove: (id: string) => call<{ ok: boolean }>(`/doc/${id}/remove`, {}),
  ask: (body: { question: string; domain?: string; web?: boolean }) =>
    call<AskResult>("/ask", body),
  teach: (body: { topic: string; domain?: string }) =>
    call<{
      ok: boolean;
      domain: string;
      engine: string;
      lesson: {
        utangulizi?: string;
        hatua?: string[];
        mifano?: string[];
        maswali?: string[];
      };
      sources: { title: string; score: number }[];
    }>("/teach", body),
  securityBrief: (body: { region?: string; topic?: string }) =>
    call<{
      ok: boolean;
      topic: string;
      region: string;
      news: { title: string; url: string; snippet?: string }[];
      cves: { id: string; desc: string; published: string }[];
      note_sw: string;
    }>("/security/brief", body),
  asks: () =>
    call<{ ok: boolean; asks: { id: string; domain: string; question: string; engine: string; created_at: string }[] }>(
      "/asks",
    ),
};

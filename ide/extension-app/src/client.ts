import type { AgentServerRequest, CanvasExtensionHost } from "./types";

export interface ApiResult<T = unknown> {
  ok: boolean;
  status: number;
  data: T | null;
  error?: string;
}

// Configuration is read from the extension metadata when the host exposes it,
// then falls back to window.__MTECHOS__ so an operator can point the IDE at the
// MTECHOS Rust agent even when only the Agent Server API is reachable.
export interface IdeConfig {
  mt: string; // MTECHOS Rust agent base (Monaco APIs, Autopilot, AKILI, swarm)
  forwardToAgentServer: boolean;
  providerUrl: string;
  providerModel: string;
  providerKey: string;
}

function defaultMtBase(): string {
  const w = window as unknown as { __MTECHOS__?: { apiBase?: string } };
  return w.__MTECHOS__?.apiBase || "http://127.0.0.1:8080";
}

export class IdeClient {
  cfg: IdeConfig;

  constructor(private readonly host: CanvasExtensionHost) {
    const w = window as unknown as {
      __MTECHOS__?: Partial<IdeConfig> & { apiBase?: string };
      localStorage?: Storage;
    };
    const ls = w.localStorage;
    this.cfg = {
      mt: w.__MTECHOS__?.mt || w.__MTECHOS__?.apiBase || ls?.getItem("mt.ide.mtBase") || defaultMtBase(),
      forwardToAgentServer: w.__MTECHOS__?.forwardToAgentServer ?? true,
      providerUrl: ls?.getItem("mt.ide.providerUrl") || "",
      providerModel: ls?.getItem("mt.ide.providerModel") || "",
      providerKey: ls?.getItem("mt.ide.providerKey") || "",
    };
  }

  setMt(base: string): void {
    this.cfg.mt = base.replace(/\/$/, "");
    (window as unknown as { localStorage?: Storage }).localStorage?.setItem("mt.ide.mtBase", this.cfg.mt);
  }

  setProvider(url: string, model: string, key: string): void {
    this.cfg.providerUrl = url;
    this.cfg.providerModel = model;
    this.cfg.providerKey = key;
    const ls = (window as unknown as { localStorage?: Storage }).localStorage;
    ls?.setItem("mt.ide.providerUrl", url);
    ls?.setItem("mt.ide.providerModel", model);
    ls?.setItem("mt.ide.providerKey", key);
  }

  // Calls the MTECHOS Rust agent directly. Falls back to the Agent Server when
  // configured, so the same page works with or without a direct MTECHOS origin.
  async mt<T = unknown>(req: AgentServerRequest): Promise<ApiResult<T>> {
    const path = req.path.startsWith("/") ? req.path : `/${req.path}`;
    const headers: Record<string, string> = { "Content-Type": "application/json", ...(req.headers || {}) };
    const opts: RequestInit = {
      method: req.method || "GET",
      headers,
      body: req.body !== undefined ? JSON.stringify(req.body) : undefined,
    };
    try {
      const res = await fetch(`${this.cfg.mt}${path}`, opts);
      let data: unknown = null;
      const text = await res.text();
      if (text) {
        try {
          data = JSON.parse(text);
        } catch {
          data = text;
        }
      }
      return { ok: res.ok, status: res.status, data: data as T };
    } catch (err) {
      // Direct origin unreachable: try the owning Agent Server as a proxy.
      if (this.cfg.forwardToAgentServer && this.host.agentServer?.request) {
        try {
          const data = await this.host.agentServer.request<T>({ ...req, path });
          return { ok: true, status: 200, data };
        } catch (e2) {
          return { ok: false, status: 0, data: null, error: String((e2 as Error)?.message || e2) };
        }
      }
      return { ok: false, status: 0, data: null, error: String((err as Error)?.message || err) };
    }
  }
}

import { describe, it, expect, vi, beforeEach } from "vitest";
import type { CanvasExtensionHost, Dispose, PageMountContext } from "../src/types";

// Minimal Canvas Extensions API host test double: records page registrations and
// drives the real mount callback against a real DOM container.
export interface HostDouble {
  host: CanvasExtensionHost;
  registered: Map<string, (ctx: PageMountContext) => void | Dispose | Promise<void | Dispose>>;
  navigations: string[];
  requests: Array<{ method?: string; path: string; body?: unknown }>;
  mounts: number;
}

export function makeHostDouble(
  responder?: (path: string) => { status?: number; body?: unknown; reject?: boolean },
): HostDouble {
  const registered = new Map<string, (ctx: PageMountContext) => void | Dispose | Promise<void | Dispose>>();
  const navigations: string[] = [];
  const requests: Array<{ method?: string; path: string; body?: unknown }> = [];
  const state = { mounts: 0 };

  const host: CanvasExtensionHost = {
    apiVersion: "1",
    extension: { name: "mbilinyitech-ide", version: "0.1.0", resolvedRef: "test" },
    backend: { id: "local-1", kind: "local", orgId: null },
    registerPage: (id, mount) => {
      registered.set(id, mount);
      return () => registered.delete(id);
    },
    navigate: (p) => navigations.push(p),
    agentServer: {
      request: async (req) => {
        requests.push({ method: req.method, path: req.path, body: req.body });
        const r = responder?.(req.path);
        if (r?.reject) throw new Error("agent-server unreachable");
        return (r?.body ?? { ok: true }) as never;
      },
    },
  };

  return {
    host,
    registered,
    navigations,
    requests,
    get mounts() {
      return state.mounts;
    },
  };
}

export function makeContext(path = ""): { ctx: PageMountContext; container: HTMLElement } {
  const container = document.createElement("div");
  document.body.appendChild(container);
  const ctx: PageMountContext = {
    container,
    path,
    navigate: vi.fn(),
  };
  return { ctx, container };
}

beforeEach(() => {
  document.body.innerHTML = "";
  document.head.querySelectorAll("style").forEach((s) => s.remove());
});

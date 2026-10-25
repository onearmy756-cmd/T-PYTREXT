import { describe, it, expect, vi } from "vitest";
import { activate } from "../src/extension";
import { makeHostDouble, makeContext } from "./host-double";
import type { CanvasExtensionHost } from "../src/types";

// The real Monaco editor is a 5 MB third-party rendering engine that cannot
// render under jsdom; mock only that boundary so the app's own logic
// (activation, routing, requests, disposal) runs as real code.
vi.mock("../src/monaco-loader", () => {
  const model = { setValue: vi.fn(), getValue: vi.fn(() => "") };
  const state: { runner?: (sel: string, lang: string, apply: (c: string) => void) => void } = {};
  return {
    createEditor: () => ({
      getModel: () => model,
      getValue: () => "",
      getAction: () => ({
        run: () => state.runner?.("sel()", "typescript", vi.fn()),
      }),
      dispose: vi.fn(),
    }),
    monaco: { editor: { setModelLanguage: vi.fn() } },
    registerInlineCompletions: () => ({ dispose: vi.fn() }),
    addInlineEditAction: (_editor: unknown, run: (sel: string, lang: string, apply: (c: string) => void) => void) => {
      state.runner = run;
      return { dispose: vi.fn() };
    },
  };
});

function mockFetch(handler: (url: string, init?: RequestInit) => unknown): void {
  global.fetch = vi.fn(async (url: unknown, init?: RequestInit) => {
    const body = handler(String(url), init);
    return {
      ok: true,
      status: 200,
      text: async () => JSON.stringify(body ?? {}),
    } as Response;
  }) as unknown as typeof fetch;
}

describe("activate", () => {
  it("registers the ide page exactly once and unregisters on dispose", () => {
    const h = makeHostDouble();
    const dispose = activate(h.host);
    expect(h.registered.has("ide")).toBe(true);
    expect(h.registered.size).toBe(1);
    dispose();
    expect(h.registered.has("ide")).toBe(false);
  });

  it("throws on an unsupported host API version", () => {
    const h = makeHostDouble();
    const bad = { ...h.host, apiVersion: "2" } as CanvasExtensionHost;
    expect(() => activate(bad)).toThrow(/host API 1/);
  });

  it("throws when registerPage is unavailable", () => {
    const h = makeHostDouble();
    const bad = { ...h.host, registerPage: undefined } as unknown as CanvasExtensionHost;
    expect(() => activate(bad)).toThrow(/registerPage/);
  });
});

const flush = () => new Promise((r) => setTimeout(r, 0));

describe("mount", () => {
  it("renders the IDE root inside the supplied container", async () => {
    mockFetch((url) => (url.includes("/api/ide/tree") ? { entries: [] } : {}));
    const h = makeHostDouble();
    activate(h.host);
    const mount = h.registered.get("ide")!;
    const { ctx, container } = makeContext();
    await mount(ctx);
    expect(container.querySelector(".mbide-root")).toBeTruthy();
    expect(container.querySelector(".mbide-editor-host")).toBeTruthy();
  });

  it("requests the MTECHOS file tree and agent health on mount", async () => {
    const seen: string[] = [];
    mockFetch((url) => {
      seen.push(url);
      return url.includes("/api/ide/tree") ? { entries: [] } : {};
    });
    const h = makeHostDouble();
    activate(h.host);
    await h.registered.get("ide")!(makeContext().ctx);
    expect(seen.some((u) => u.includes("/api/ide/tree"))).toBe(true);
    expect(seen.some((u) => u.includes("/api/agent/health"))).toBe(true);
  });

  it("opens a file through /api/ide/file with an encoded path", async () => {
    const seen: string[] = [];
    mockFetch((url) => {
      seen.push(url);
      if (url.includes("/api/ide/tree")) {
        return { entries: [{ path: "src/main.rs", name: "main.rs" }] };
      }
      if (url.includes("/api/ide/file")) return { content: "fn main() {}" };
      return {};
    });
    const h = makeHostDouble();
    activate(h.host);
    const { ctx, container } = makeContext();
    await h.registered.get("ide")!(ctx);
    await flush();
    const fileBtn = container.querySelector<HTMLButtonElement>(".mbide-tree button")!;
    expect(fileBtn).toBeTruthy();
    fileBtn.click();
    await flush();
    expect(seen.some((u) => u.includes("/api/ide/file?path=src%2Fmain.rs"))).toBe(true);
  });

  it("renders safely when the MTECHOS agent is unreachable", async () => {
    global.fetch = vi.fn(async () => {
      throw new Error("ECONNREFUSED");
    }) as unknown as typeof fetch;
    const h = makeHostDouble(() => ({ reject: true }));
    activate(h.host);
    const { ctx, container } = makeContext();
    await h.registered.get("ide")!(ctx);
    await Promise.resolve();
    await Promise.resolve();
    expect(container.querySelector(".mbide-root")).toBeTruthy();
    expect(container.textContent).toMatch(/offline|agent/i);
  });

  it("sends the agent prompt as the question field", async () => {
    const bodies: Array<{ path: string; body?: unknown }> = [];
    global.fetch = vi.fn(async (url: unknown, init?: RequestInit) => {
      const body = init?.body ? JSON.parse(String(init.body)) : undefined;
      bodies.push({ path: String(url), body });
      const payload = String(url).includes("/api/agent/chat") ? { reply: "sawa" } : {};
      return { ok: true, status: 200, text: async () => JSON.stringify(payload) } as Response;
    }) as unknown as typeof fetch;
    const h = makeHostDouble();
    activate(h.host);
    const { ctx, container } = makeContext();
    await h.registered.get("ide")!(ctx);
    const input = container.querySelector<HTMLTextAreaElement>(".mbide-prompt textarea")!;
    input.value = "jenga website";
    container.querySelector<HTMLButtonElement>(".mbide-prompt button")!.click();
    await flush();
    const chat = bodies.find((b) => b.path.includes("/api/agent/chat"));
    expect(chat).toBeTruthy();
    expect((chat!.body as { question?: string }).question).toBe("jenga website");
  });

  it("disposes DOM and injected style on unmount", async () => {
    mockFetch(() => ({}));
    const h = makeHostDouble();
    activate(h.host);
    const { ctx, container } = makeContext();
    const dispose = await h.registered.get("ide")!(ctx);
    expect(container.querySelector(".mbide-root")).toBeTruthy();
    (dispose as () => void)();
    expect(container.querySelector(".mbide-root")).toBeFalsy();
    expect(document.head.querySelector('[data-mbilinyitech-ide-style]')).toBeFalsy();
  });

  it("renders a 2NYI chart from a ```chart block in the agent reply", async () => {
    const seen: string[] = [];
    global.fetch = vi.fn(async (url: unknown, init?: RequestInit) => {
      seen.push(String(url));
      const payload = String(url).includes("/api/agent/chat")
        ? { reply: 'Haya:\n```chart\n{"kind":"bar","title":"Mauzo","data":[{"label":"a","value":3}]}\n```' }
        : String(url).includes("/api/nyi/chart")
          ? { ok: true, svg: "<svg></svg>", data_url: "data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=" }
          : {};
      return { ok: true, status: 200, text: async () => JSON.stringify(payload) } as Response;
    }) as unknown as typeof fetch;
    const h = makeHostDouble();
    activate(h.host);
    const { ctx, container } = makeContext();
    await h.registered.get("ide")!(ctx);
    const input = container.querySelector<HTMLTextAreaElement>(".mbide-prompt textarea")!;
    input.value = "onyesha chati";
    container.querySelector<HTMLButtonElement>(".mbide-prompt button")!.click();
    await flush();
    await flush();
    expect(seen.some((u) => u.includes("/api/nyi/chart"))).toBe(true);
    const img = container.querySelector<HTMLImageElement>(".mbide-media");
    expect(img).toBeTruthy();
    expect(img!.src.startsWith("data:image/svg+xml;base64,")).toBe(true);
  });

  it("renders image and video URLs from the agent reply as media", async () => {
    global.fetch = vi.fn(async (url: unknown) => {
      const payload = String(url).includes("/api/agent/chat")
        ? { reply: "Hapa: https://x.dev/a.png na https://x.dev/b.mp4" }
        : {};
      return { ok: true, status: 200, text: async () => JSON.stringify(payload) } as Response;
    }) as unknown as typeof fetch;
    const h = makeHostDouble();
    activate(h.host);
    const { ctx, container } = makeContext();
    await h.registered.get("ide")!(ctx);
    const input = container.querySelector<HTMLTextAreaElement>(".mbide-prompt textarea")!;
    input.value = "onyesha media";
    container.querySelector<HTMLButtonElement>(".mbide-prompt button")!.click();
    await flush();
    expect(container.querySelector("img.mbide-media")).toBeTruthy();
    expect(container.querySelector("video.mbide-media")).toBeTruthy();
  });

  it("toggles the MONOCODE AI autocomplete switch", async () => {
    mockFetch(() => ({}));
    const h = makeHostDouble();
    activate(h.host);
    const { ctx, container } = makeContext();
    await h.registered.get("ide")!(ctx);
    const ai = Array.from(container.querySelectorAll<HTMLButtonElement>(".mbide-bar button")).find((b) =>
      b.textContent?.includes("AI:"),
    )!;
    expect(ai).toBeTruthy();
    expect(ai.textContent).toContain("on");
    ai.click();
    expect(ai.textContent).toContain("off");
    ai.click();
    expect(ai.textContent).toContain("on");
  });

  it("runs the Cmd+K inline edit against /api/ide/edit", async () => {
    const bodies: Array<{ path: string; body?: unknown }> = [];
    global.fetch = vi.fn(async (url: unknown, init?: RequestInit) => {
      const body = init?.body ? JSON.parse(String(init.body)) : undefined;
      bodies.push({ path: String(url), body });
      const payload = String(url).includes("/api/ide/edit") ? { ok: true, code: "improved()" } : {};
      return { ok: true, status: 200, text: async () => JSON.stringify(payload) } as Response;
    }) as unknown as typeof fetch;
    window.prompt = vi.fn(() => "boresha");
    const h = makeHostDouble();
    activate(h.host);
    const { ctx, container } = makeContext();
    await h.registered.get("ide")!(ctx);
    const edit = Array.from(container.querySelectorAll<HTMLButtonElement>(".mbide-bar button")).find((b) =>
      b.textContent?.includes("Edit"),
    )!;
    edit.click();
    await flush();
    await flush();
    const call = bodies.find((b) => b.path.includes("/api/ide/edit"));
    expect(call).toBeTruthy();
    expect((call!.body as { instruction?: string }).instruction).toBe("boresha");
  });

  it("runs the Composer plan and renders a diff per file", async () => {
    const seen: string[] = [];
    global.fetch = vi.fn(async (url: unknown, init?: RequestInit) => {
      const u = String(url);
      seen.push(u);
      let payload: unknown = {};
      if (u.includes("/api/ide/composer/plan")) {
        payload = {
          ok: true,
          edits: [
            { path: "src/a.rs", action: "write", old: "", new: "fn a() {}\nfn b() {}\n" },
            { path: "src/b.rs", action: "replace", old: "x", new: "y" },
          ],
        };
      } else if (u.includes("/api/ide/composer/diff")) {
        payload = { ok: true, diff: "--- a/x\n+++ b/x\n+x\n", added: 1, removed: 0 };
      } else if (u.includes("/api/ide/file")) {
        payload = { content: "fn a() {}\n" };
      }
      return { ok: true, status: 200, text: async () => JSON.stringify(payload) } as Response;
    }) as unknown as typeof fetch;
    const h = makeHostDouble();
    activate(h.host);
    const { ctx, container } = makeContext();
    await h.registered.get("ide")!(ctx);
    const composer = Array.from(container.querySelectorAll<HTMLButtonElement>(".mbide-bar button")).find(
      (b) => b.textContent?.includes("Composer"),
    )!;
    expect(composer).toBeTruthy();
    composer.click();
    const plan = Array.from(container.querySelectorAll<HTMLButtonElement>(".mbide-modal button")).find(
      (b) => b.textContent?.includes("Panga edits"),
    )!;
    plan.click();
    await flush();
    await flush();
    await flush();
    expect(seen.some((u) => u.includes("/api/ide/composer/plan"))).toBe(true);
    expect(seen.some((u) => u.includes("/api/ide/composer/diff"))).toBe(true);
    expect(container.querySelectorAll(".mbide-composer-card").length).toBe(2);
    expect(container.querySelector(".mbide-diffline.add")).toBeTruthy();
  });

  it("applies Composer edits via /api/ide/composer/apply", async () => {
    const bodies: Array<{ path: string; body?: unknown }> = [];
    global.fetch = vi.fn(async (url: unknown, init?: RequestInit) => {
      const u = String(url);
      const body = init?.body ? JSON.parse(String(init.body)) : undefined;
      bodies.push({ path: u, body });
      let payload: unknown = {};
      if (u.includes("/api/ide/composer/plan")) {
        payload = { ok: true, edits: [{ path: "src/a.rs", action: "write", old: "", new: "fn a() {}" }] };
      } else if (u.includes("/api/ide/composer/diff")) {
        payload = { ok: true, diff: "+fn a() {}", added: 1, removed: 0 };
      } else if (u.includes("/api/ide/composer/apply")) {
        payload = { ok: true, applied_count: 1, skipped: [] };
      }
      return { ok: true, status: 200, text: async () => JSON.stringify(payload) } as Response;
    }) as unknown as typeof fetch;
    const h = makeHostDouble();
    activate(h.host);
    const { ctx, container } = makeContext();
    await h.registered.get("ide")!(ctx);
    Array.from(container.querySelectorAll<HTMLButtonElement>(".mbide-bar button"))
      .find((b) => b.textContent?.includes("Composer"))!
      .click();
    Array.from(container.querySelectorAll<HTMLButtonElement>(".mbide-modal button"))
      .find((b) => b.textContent?.includes("Panga edits"))!
      .click();
    await flush();
    await flush();
    await flush();
    Array.from(container.querySelectorAll<HTMLButtonElement>(".mbide-modal button"))
      .find((b) => b.textContent?.includes("Kubali zote"))!
      .click();
    await flush();
    const apply = bodies.find((b) => b.path.includes("/api/ide/composer/apply"));
    expect(apply).toBeTruthy();
    expect((apply!.body as { edits?: unknown[] }).edits?.length).toBe(1);
  });
});

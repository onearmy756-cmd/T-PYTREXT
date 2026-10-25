import styles from "./styles.css?inline";
import { IdeClient } from "./client";
import {
  createEditor,
  monaco,
  registerInlineCompletions,
  addInlineEditAction,
  type InlineCompletionContextData,
} from "./monaco-loader";
import type { CanvasExtensionHost, Dispose, PageMountContext } from "./types";

const PAGE_ID = "ide";
const ROOT_CLASS = "mbide-root";
const STYLE_MARKER = "data-mbilinyitech-ide-style";

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  className?: string,
  text?: string,
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function languageFor(name: string): string {
  const ext = name.split(".").pop()?.toLowerCase() || "";
  const map: Record<string, string> = {
    rs: "rust",
    js: "javascript",
    mjs: "javascript",
    cjs: "javascript",
    ts: "typescript",
    tsx: "typescript",
    jsx: "javascript",
    py: "python",
    r: "r",
    html: "html",
    htm: "html",
    css: "css",
    json: "json",
    md: "markdown",
    toml: "ini",
    yml: "yaml",
    yaml: "yaml",
    sh: "shell",
    bash: "shell",
    c: "c",
    h: "c",
    cpp: "cpp",
    hpp: "cpp",
    go: "go",
    java: "java",
    sql: "sql",
    xml: "xml",
  };
  return map[ext] || "plaintext";
}

export function activate(host: CanvasExtensionHost): Dispose {
  if (host.apiVersion !== "1") {
    throw new Error(`MBILINYITECH IDE requires Canvas host API 1, got ${host.apiVersion}.`);
  }
  if (typeof host.registerPage !== "function") {
    throw new Error("MBILINYITECH IDE requires host.registerPage from Canvas host API 1.");
  }

  const unregister = host.registerPage(PAGE_ID, (ctx) => mount(host, ctx));
  return () => {
    unregister();
  };
}

function mount(host: CanvasExtensionHost, ctx: PageMountContext): Dispose {
  const client = new IdeClient(host);
  const style = document.createElement("style");
  style.setAttribute(STYLE_MARKER, "");
  style.textContent = styles;
  document.head.appendChild(style);

  const root = el("div", ROOT_CLASS);

  // ---- top bar ----
  const bar = el("div", "mbide-bar");
  const logo = el("span", "mbide-logo", "MBILINYITECH IDE");
  const btnTree = el("button", "mbide-btn", "↻ Files");
  const btnAutopilot = el("button", "mbide-btn primary", "🚀 Autopilot");
  const btnAI = el("button", "mbide-btn", "✨ AI: on");
  const btnEdit = el("button", "mbide-btn", "⌘K Edit");
  const btnComposer = el("button", "mbide-btn", "✍️ Composer");
  const btnTerminal = el("button", "mbide-btn", ">_ Terminal");
  const btnFull = el("button", "mbide-btn", "🧩 Full IDE");
  const btnSettings = el("button", "mbide-btn", "⚙️ Settings");
  const spacer = el("div", "mbide-spacer");
  const chip = el("span", "mbide-btn", "● agent…");
  bar.append(logo, btnTree, btnAutopilot, btnAI, btnEdit, btnComposer, btnTerminal, btnFull, btnSettings, spacer, chip);

  // ---- main grid ----
  const main = el("div", "mbide-main");

  // file tree panel
  const left = el("div", "mbide-panel");
  left.append(el("div", "mbide-hd", "Project"));
  const treeBox = el("div", "mbide-tree");
  left.append(treeBox);

  // editor column
  const center = el("div", "mbide-editor-col");
  const editorHost = el("div", "mbide-editor-host");
  const status = el("div", "mbide-status");
  const stFile = el("span", undefined, "—");
  const stPos = el("span");
  const stLang = el("span");
  const stSaved = el("span", undefined, "saved");
  status.append(stFile, stPos, stLang, stSaved);
  center.append(editorHost, status);

  // agent panel
  const right = el("div", "mbide-panel dock");
  right.append(el("div", "mbide-hd", "Agent"));
  const logBox = el("div", "mbide-log");
  const promptRow = el("div", "mbide-prompt");
  const input = el("textarea", "mbide-input");
  input.placeholder = "Uliza agent… (Enter = send)";
  input.setAttribute("aria-label", "Agent prompt");
  const sendBtn = el("button", "mbide-btn primary", "➤");
  promptRow.append(input, sendBtn);
  right.append(logBox, promptRow);

  main.append(left, center, right);
  root.append(bar, main);
  ctx.container.append(root);

  // ---- editor ----
  const editor = createEditor({
    host: editorHost,
    value: "// MBILINYITECH IDE — fungua faili upande wa kushoto, au tumia Autopilot.\n",
    language: "plaintext",
    onChange: () => {
      stSaved.textContent = "unsaved";
    },
    onCursor: (line, col) => {
      stPos.textContent = `Ln ${line}, Col ${col}`;
    },
    onSave: () => void saveFile(),
  });

  let currentFile = "";
  let disposed = false;
  const timers: number[] = [];

  function log(msg: string, kind: "info" | "user" | "err" = "info"): void {
    const line = el("div", `mbide-msg ${kind === "info" ? "" : kind}`, msg);
    logBox.append(line);
    logBox.scrollTop = logBox.scrollHeight;
  }

  // ---- MONOCODE: AI ghost-text autocomplete + Cmd+K inline edit ----
  // Nyuma ni endpoints halisi za MTECHOS: /api/ide/complete (FIM) na
  // /api/ide/edit (inline edit). Kitufe ✨ AI hudhibiti ghost-text.
  let aiEnabled = true;
  btnAI.addEventListener("click", () => {
    aiEnabled = !aiEnabled;
    btnAI.textContent = aiEnabled ? "✨ AI: on" : "✨ AI: off";
    btnAI.classList.toggle("primary", aiEnabled);
    log(`AI autocomplete ${aiEnabled ? "imewashwa" : "imezimwa"}.`);
  });

  const completeViaAgent = async (ctx: InlineCompletionContextData): Promise<string | null> => {
    if (!aiEnabled || !currentFile) return null;
    const res = await client.mt<{ completion?: string }>({
      method: "POST",
      path: "/api/ide/complete",
      body: { path: currentFile, prefix: ctx.prefix, suffix: ctx.suffix, language: ctx.language },
    });
    const completion = res.data && typeof res.data.completion === "string" ? res.data.completion : "";
    return res.ok && completion.trim() ? completion : null;
  };

  const inlineCompletions = registerInlineCompletions(editor, completeViaAgent);

  const runInlineEdit = (
    selection: string,
    language: string,
    apply: (code: string) => void,
  ): void => {
    const instruction = window.prompt(
      "AI inline edit — maelekezo (Cmd/Ctrl+K):",
      "boresha code hii, kaa API zilizopo",
    );
    if (!instruction || !instruction.trim()) return;
    log(`⌘K edit: ${instruction}`, "user");
    void client
      .mt<{ code?: string }>({
        method: "POST",
        path: "/api/ide/edit",
        body: { selection, instruction: instruction.trim(), path: currentFile, language },
      })
      .then((res) => {
        const code = res.data && typeof res.data.code === "string" ? res.data.code : "";
        if (res.ok && code.trim()) {
          apply(code);
          log("⌘K edit imetumika.");
        } else {
          log(`⌘K edit: ${res.error || res.status || "haipatikani (weka provider kwenye ⚙️ Settings)"}`, "err");
        }
      });
  };

  const inlineEditAction = addInlineEditAction(editor, runInlineEdit);

  btnEdit.addEventListener("click", () => editor.getAction("mbide.inlineEdit")?.run());

  // ---- 2NYI rich rendering: chati / picha / video ----
  const MEDIA_RE = /(https?:\/\/\S+\.(?:png|jpe?g|gif|webp|svg|mp4|webm|mov|m4v))(\?\S*)?/gi;

  function chartBlock(jsonText: string): HTMLElement {
    const wrap = el("div", "mbide-msg media");
    const info = el("div", "mbide-media-cap", "2NYI chart");
    wrap.append(info);
    try {
      const spec = JSON.parse(jsonText) as { kind?: string; title?: string; data?: unknown };
      void client
        .mt<{ svg?: string; data_url?: string }>({
          method: "POST",
          path: "/api/nyi/chart",
          body: {
            kind: spec.kind || "line",
            title: spec.title || "Chati",
            data: spec.data ?? [],
          },
        })
        .then((res) => {
          const url = res.data?.data_url;
          if (res.ok && url) {
            const img = el("img", "mbide-media") as HTMLImageElement;
            img.src = url;
            img.alt = spec.title || "chart";
            wrap.append(img);
          } else {
            wrap.append(el("div", "mbide-msg err", `chart error: ${res.error || res.status}`));
          }
          logBox.scrollTop = logBox.scrollHeight;
        });
    } catch {
      wrap.append(el("div", "mbide-msg err", "chart: JSON si sahihi"));
    }
    return wrap;
  }

  function mediaNode(url: string): HTMLElement {
    const isVideo = /\.(mp4|webm|mov|m4v)(\?|$)/i.test(url);
    const wrap = el("div", "mbide-msg media");
    if (isVideo) {
      const v = el("video", "mbide-media") as HTMLVideoElement;
      v.src = url;
      v.controls = true;
      v.muted = true;
      wrap.append(v);
    } else {
      const img = el("img", "mbide-media") as HTMLImageElement;
      img.src = url;
      img.alt = "media";
      wrap.append(img);
    }
    return wrap;
  }

  /** Onyesha jibu la LLM: chati (```chart {…}```), picha na video (URL). */
  function logRich(text: string): void {
    const chartRe = /```chart\s*([\s\S]*?)```/gi;
    let last = 0;
    let m: RegExpExecArray | null;
    const parts: Array<string | HTMLElement> = [];
    while ((m = chartRe.exec(text)) !== null) {
      parts.push(text.slice(last, m.index));
      parts.push(chartBlock(m[1].trim()));
      last = m.index + m[0].length;
    }
    parts.push(text.slice(last));

    for (const part of parts) {
      if (typeof part !== "string") {
        logBox.append(part);
        continue;
      }
      // Gawa maandishi kwa URL za media; zingine zibaki kama maandishi.
      let idx = 0;
      let mm: RegExpExecArray | null;
      MEDIA_RE.lastIndex = 0;
      const tail: string[] = [];
      while ((mm = MEDIA_RE.exec(part)) !== null) {
        if (mm.index > idx) tail.push(part.slice(idx, mm.index));
        tail.push("\u0000" + mm[0]);
        idx = mm.index + mm[0].length;
      }
      tail.push(part.slice(idx));
      for (const seg of tail) {
        if (seg.startsWith("\u0000")) {
          logBox.append(mediaNode(seg.slice(1)));
        } else if (seg.trim()) {
          log(seg);
        }
      }
    }
    logBox.scrollTop = logBox.scrollHeight;
  }

  async function openFile(path: string): Promise<void> {
    const res = await client.mt<{ content?: string }>({
      path: `/api/ide/file?path=${encodeURIComponent(path)}`,
    });
    const content = res.ok && res.data && typeof res.data.content === "string" ? res.data.content : "";
    if (!res.ok) {
      log(`Failed to open ${path}: ${res.error || res.status}`, "err");
      return;
    }
    currentFile = path;
    const model = editor.getModel();
    if (model) {
      monaco.editor.setModelLanguage(model, languageFor(path));
      model.setValue(content);
    }
    stFile.textContent = path;
    stLang.textContent = languageFor(path);
    stSaved.textContent = "saved";
  }

  async function saveFile(): Promise<void> {
    if (!currentFile) return;
    const value = editor.getValue();
    const res = await client.mt({
      method: "POST",
      path: "/api/ide/file",
      body: { path: currentFile, content: value },
    });
    stSaved.textContent = res.ok ? "saved" : "error";
    log(res.ok ? `Saved ${currentFile}` : `Save failed: ${res.error || res.status}`, res.ok ? "info" : "err");
  }

  async function loadTree(): Promise<void> {
    treeBox.replaceChildren(el("div", "mbide-hd", "loading…"));
    const res = await client.mt<{ entries?: Array<{ path: string; name?: string } | string> }>({
      path: "/api/ide/tree",
    });
    treeBox.replaceChildren();
    if (!res.ok || !res.data) {
      treeBox.append(el("div", "mbide-hd", res.error ? `offline: ${res.error}` : "no data"));
      return;
    }
    const entries = Array.isArray(res.data.entries)
      ? res.data.entries
      : Array.isArray(res.data)
        ? (res.data as unknown as string[])
        : [];
    for (const entry of entries) {
      const path = typeof entry === "string" ? entry : entry.path;
      const name = typeof entry === "string" ? entry.split("/").pop() || entry : entry.name || entry.path;
      const b = el("button", undefined, `📄 ${name}`);
      b.addEventListener("click", () => void openFile(path));
      treeBox.append(b);
    }
    if (entries.length === 0) treeBox.append(el("div", "mbide-hd", "empty"));
  }

  // ---- MONOCODE COMPOSER: maaelezo MOJA → edits za faili NYINGI + diff ----
  // Cursor/Devin Composer: /api/ide/composer/plan hupanga edits; kila faili
  // inaonyeshwa kama diff; mtumiaji anakubali (apply) au anakataa (reject).
  interface ComposerEdit {
    path: string;
    action: string;
    old: string;
    new: string;
  }

  function openComposer(): void {
    const backdrop = el("div", "mbide-modal-backdrop");
    const modal = el("div", "mbide-modal mbide-composer");
    modal.append(el("h2", undefined, "✍️ Composer — hariri faili nyingi (Cursor/Devin)"));

    const ideaRow = el("div", "mbide-row");
    ideaRow.append(el("label", undefined, "Maelezo (kazi)"));
    const idea = el("textarea");
    idea.value = "Ongeza uthibitisho wa pembejeo kwenye faili husika";
    ideaRow.append(idea);

    const ctxRow = el("div", "mbide-row");
    ctxRow.append(el("label", undefined, "Faili za muktadha (mtiririko, si lazima)"));
    const ctxFiles = el("input");
    ctxFiles.placeholder = "src/a.rs, src/b.rs";
    ctxRow.append(ctxFiles);

    const actions = el("div", "mbide-bar");
    const planBtn = el("button", "mbide-btn primary", "🔍 Panga edits");
    const applyAll = el("button", "mbide-btn", "✅ Kubali zote");
    const rejectAll = el("button", "mbide-btn", "✖ Kataa zote");
    const close = el("button", "mbide-btn", "Funga");
    actions.append(planBtn, applyAll, rejectAll, close);

    const results = el("div", "mbide-composer-list");
    modal.append(ideaRow, ctxRow, actions, results);
    backdrop.append(modal);
    root.append(backdrop);

    let pending: ComposerEdit[] = [];

    const renderDiff = (host: HTMLElement, path: string, diffText: string): void => {
      host.replaceChildren();
      for (const line of diffText.split("\n")) {
        let cls = "ctx";
        if (line.startsWith("+") && !line.startsWith("+++")) cls = "add";
        else if (line.startsWith("-") && !line.startsWith("---")) cls = "del";
        else if (line.startsWith("@@")) cls = "hunk";
        host.append(el("div", `mbide-diffline ${cls}`, line || " "));
      }
    };

    const showEdits = async (edits: ComposerEdit[]): Promise<void> => {
      pending = edits;
      results.replaceChildren();
      for (const e of edits) {
        const card = el("div", "mbide-composer-card");
        card.append(el("div", "mbide-hd", `${e.action === "write" ? "＋" : "±"} ${e.path}`));
        let oldText = e.old;
        if (e.action !== "replace") {
          const res = await client.mt<{ content?: string }>({
            path: `/api/ide/file?path=${encodeURIComponent(e.path)}`,
          });
          oldText = res.ok && res.data && typeof res.data.content === "string" ? res.data.content : "";
        }
        const d = await client.mt<{ diff?: string; added?: number; removed?: number }>({
          method: "POST",
          path: "/api/ide/composer/diff",
          body: { path: e.path, old: oldText, new: e.new },
        });
        card.append(
          el("div", "mbide-media-cap", `+${d.data?.added ?? 0} / -${d.data?.removed ?? 0}`),
        );
        const box = el("div", "mbide-diffbox");
        renderDiff(box, e.path, d.data?.diff || "(hakuna tofauti)");
        card.append(box);
        results.append(card);
      }
    };

    planBtn.addEventListener("click", async () => {
      const instruction = idea.value.trim();
      if (!instruction) return;
      results.replaceChildren(el("div", "mbide-hd", "inafikiri…"));
      const files = ctxFiles.value
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean);
      const body: Record<string, unknown> = { instruction, files };
      if (currentFile) {
        body.path = currentFile;
        body.language = languageFor(currentFile);
      }
      const res = await client.mt<{ edits?: ComposerEdit[]; error?: string }>({
        method: "POST",
        path: "/api/ide/composer/plan",
        body,
      });
      if (res.ok && res.data?.edits?.length) {
        log(`Composer: edits ${res.data.edits.length}`, "info");
        await showEdits(res.data.edits);
      } else {
        results.replaceChildren(
          el("div", "mbide-msg err", `Composer: ${res.data?.error || res.error || res.status}`),
        );
      }
    });

    applyAll.addEventListener("click", async () => {
      if (!pending.length) {
        log("Composer: hakuna edits za kukubali.", "err");
        return;
      }
      const res = await client.mt<{ applied_count?: number; skipped?: unknown[] }>({
        method: "POST",
        path: "/api/ide/composer/apply",
        body: { edits: pending },
      });
      log(
        `Composer: zilizowekwa ${res.data?.applied_count ?? 0}` +
          (res.data?.skipped?.length ? `, zilizorukwa ${res.data.skipped.length}` : ""),
        res.ok ? "info" : "err",
      );
      pending = [];
      backdrop.remove();
      void loadTree();
    });

    rejectAll.addEventListener("click", async () => {
      await client.mt({ method: "POST", path: "/api/ide/composer/reject", body: { edits: pending } });
      log("Composer: edits zimekataliwa.", "info");
      pending = [];
      backdrop.remove();
    });

    close.addEventListener("click", () => backdrop.remove());
  }

  function openAutopilot(): void {
    const backdrop = el("div", "mbide-modal-backdrop");
    const modal = el("div", "mbide-modal");
    modal.append(el("h2", undefined, "🚀 Autopilot — panga na uendeshe mradi"));

    const kindRow = el("div", "mbide-row");
    kindRow.append(el("label", undefined, "Aina ya mradi"));
    const kind = el("select");
    for (const k of ["website", "mobile", "iot", "gaming"]) {
      const o = el("option", undefined, k);
      o.value = k;
      kind.append(o);
    }
    kindRow.append(kind);

    const ideaRow = el("div", "mbide-row");
    ideaRow.append(el("label", undefined, "Wazo / maelezo"));
    const idea = el("textarea");
    idea.value = "Mradi wangu wa kwanza";
    ideaRow.append(idea);

    const actions = el("div", "mbide-bar");
    const run = el("button", "mbide-btn primary", "Anza");
    const close = el("button", "mbide-btn", "Funga");
    const out = el("div", "mbide-log");
    actions.append(run, close);

    modal.append(kindRow, ideaRow, actions, out);
    backdrop.append(modal);
    root.append(backdrop);

    close.addEventListener("click", () => backdrop.remove());
    run.addEventListener("click", async () => {
      out.textContent = "inaanza…";
      const res = await client.mt({
        method: "POST",
        path: "/api/ide/agent/run",
        body: { kind: kind.value, idea: idea.value, use_llm: true },
      });
      out.textContent = JSON.stringify(res.data ?? res.error, null, 2);
      log(`Autopilot (${kind.value}) → ${res.ok ? "ok" : "error"}`, res.ok ? "info" : "err");
    });
  }

  async function sendPrompt(): Promise<void> {
    const text = input.value.trim();
    if (!text) return;
    input.value = "";
    log(`you: ${text}`, "user");
    const res = await client.mt<{ reply?: string }>({
      method: "POST",
      path: "/api/agent/chat",
      body: { question: text, mode: "agent", prefer_agent: true },
    });
    const reply =
      (res.data && res.data.reply) ||
      (res.ok ? "(empty response)" : `error: ${res.error || res.status}`);
    if (res.ok && res.data && res.data.reply) {
      logRich(String(reply));
    } else {
      log(String(reply), res.ok ? "info" : "err");
    }
  }

  function openSettings(): void {
    const backdrop = el("div", "mbide-modal-backdrop");
    const modal = el("div", "mbide-modal");
    modal.append(el("h2", undefined, "⚙️ Settings"));
    const mk = (label: string, val: string, type = "text") => {
      const row = el("div", "mbide-row");
      row.append(el("label", undefined, label));
      const inp = el("input");
      inp.type = type;
      inp.value = val;
      row.append(inp);
      modal.append(row);
      return inp;
    };
    const mtBase = mk("MTECHOS agent URL", client.cfg.mt);
    const pUrl = mk("Provider URL", client.cfg.providerUrl);
    const pModel = mk("Provider model", client.cfg.providerModel);
    const pKey = mk("Provider API key", client.cfg.providerKey, "password");
    const actions = el("div", "mbide-bar");
    const save = el("button", "mbide-btn primary", "💾 Save");
    const close = el("button", "mbide-btn", "Funga");
    actions.append(save, close);
    modal.append(actions);
    backdrop.append(modal);
    root.append(backdrop);
    close.addEventListener("click", () => backdrop.remove());
    save.addEventListener("click", async () => {
      client.setMt(mtBase.value.trim());
      client.setProvider(pUrl.value.trim(), pModel.value.trim(), pKey.value);
      chip.textContent = "● configured";
      const results: string[] = [];
      if (pKey.value.trim()) {
        const key = await client.mt({ method: "POST", path: "/api/ai/key", body: { key: pKey.value.trim() } });
        results.push(key.ok ? "key saved" : `key error: ${key.error || key.status}`);
      }
      if (pUrl.value.trim() && pModel.value.trim()) {
        const custom = await client.mt({
          method: "POST",
          path: "/api/ai/custom",
          body: { url: pUrl.value.trim(), model: pModel.value.trim() },
        });
        results.push(custom.ok ? "model endpoint set" : `model error: ${custom.error || custom.status}`);
      }
      backdrop.remove();
      log(`Settings saved. ${results.join(" · ")}`);
      // Apply the model to confirm the Autopilot can reach an LLM.
      const test = await client.mt({ method: "POST", path: "/api/ai/test" });
      log(`AI test: ${JSON.stringify(test.data ?? test.error)}`, test.ok ? "info" : "err");
      void loadTree();
    });
  }

  async function checkHealth(): Promise<void> {
    const res = await client.mt({ path: "/api/agent/health" });
    chip.textContent = res.ok ? "● agent online" : "● agent offline";
  }

  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey && document.activeElement === input) {
      e.preventDefault();
      void sendPrompt();
    }
  };

  input.addEventListener("keydown", onKey);
  sendBtn.addEventListener("click", () => void sendPrompt());
  btnTree.addEventListener("click", () => void loadTree());
  btnAutopilot.addEventListener("click", openAutopilot);
  btnComposer.addEventListener("click", openComposer);
  btnSettings.addEventListener("click", openSettings);
  btnTerminal.addEventListener("click", async () => {
    const cmd = window.prompt("Amri ya terminal:", "ls -la");
    if (!cmd) return;
    const res = await client.mt<{ stdout?: string; stderr?: string }>({
      method: "POST",
      path: "/api/ide/terminal",
      body: { cmd },
    });
    log(String(res.data?.stdout || res.data?.stderr || res.error || ""), res.ok ? "info" : "err");
  });

  btnFull.addEventListener("click", async () => {
    // "Full IDE" loads the complete MTECHOS canvas (Monaco + swarm 50, LSP,
    // DAP, E2E, emulators, AKILI, providers, cyber) so that this app
    // never drops a feature. The iframe keeps its own document/origin-isolated
    // runtime; the native panel above stays the Monaco + Autopilot surface.
    if (document.querySelector(".mbide-fullframe")) {
      document.querySelector(".mbide-fullframe")!.remove();
      log("Full IDE imefungwa.");
      return;
    }
    const frame = el("iframe", "mbide-fullframe");
    frame.setAttribute("title", "MBILINYITECH IDE — full");
    const url = `${client.cfg.mt}/ide.html`;
    frame.src = url;
    frame.style.cssText = "width:100%;height:70vh;border:1px solid var(--mbide-border);border-radius:8px;background:#0d1117";
    const wrap = el("div");
    wrap.style.cssText = "padding:8px;border-top:1px solid var(--mbide-border)";
    const note = el("div", "mbide-hd", `Full IDE · ${url}`);
    wrap.append(note, frame);
    center.append(wrap);
    const health = await client.mt({ path: "/api/agent/health" });
    log(
      health.ok
        ? `Full IDE imepakiwa kutoka ${url}`
        : `Full IDE: MTECHOS agent haipatikani kwenye ${url} — weka URL kwenye ⚙️ Settings.`,
      health.ok ? "info" : "err",
    );
  });

  log("Karibu MBILINYITECH IDE. Agent + Autopilot + Monaco viko tayari.");
  void loadTree();
  void checkHealth();

  return () => {
    disposed = true;
    for (const t of timers) window.clearTimeout(t);
    inlineCompletions.dispose();
    inlineEditAction.dispose();
    editor.dispose();
    root.remove();
    style.remove();
    void disposed;
  };
}

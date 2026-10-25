/* extension.js — MTECHOS kama ukurasa wa ndani wa MBILINYI IDE.
 *
 * Pattern: ile ile ya demo-page fixture (src/fixtures/canvas-extensions/demo-page).
 * trusted same-realm JavaScript — inaandikwa ndani ya Agent Canvas kama page
 * moja ya ndani (si ukurasa mpya wa browser, si tab mpya).
 *
 * Vitu 2 vinapachikwa:
 *   1. MTECHOS FORGE — page /mtechos: zalisha MRADI MZIMA (faili nyingi kwa
 *      parallel) kwa SSE kutoka POST /api/forge/run. Inaonyesha blueprint,
 *      kila faili (lint ok/rounds/bytes), na muhtasari (muda, engineer score).
 *   2. OLLAMA BOOSTER — page /ollama: models halisi za Ollama + MTECHOS
 *      (llm_setup) + kuingiza provider-connection ya MBILINYI.
 *
 * Kanuni: hakuna mock — kila thamani inatoka API halisi ya backend ya MTECHOS.
 * Hakuna innerHTML kwa data ya mtumiaji (usalama: XSS).
 */
export function activate(host) {
  // Session key ya Canvas (X-Session-API-Key) — Canvas huiweka kwenye window.
  // Endpoints za agent-server (mfano provider-connections) zinahitaji uthibitisho.
  const authHeaders = (extra) => {
    const k = window.__AGENT_CANVAS_SESSION_API_KEY__;
    return k ? { ...(extra || {}), "X-Session-API-Key": k } : (extra || {});
  };
  // ═══ UKURASA: MTECHOS IDE — Monaco kamili (editor = uso mkuu) + chat na zana za mradi ═══
  host.registerPage("studio", async ({ container }) => {
    const el = (tag, css, text) => {
      const n = document.createElement(tag);
      if (css) n.style.cssText = css;
      if (text != null) n.textContent = text;
      return n;
    };
    const BTN =
      "padding:8px 14px;border-radius:8px;border:0;cursor:pointer;font-weight:600;font-size:12px;" +
      "background:var(--oh-brand,#5865f2);color:#fff;";
    const BTN2 =
      "padding:8px 14px;border-radius:8px;cursor:pointer;font-weight:600;font-size:12px;" +
      "background:transparent;color:inherit;border:1px solid var(--oh-border-variant,#2a2f3a);";
    const INP =
      "width:100%;padding:9px 12px;border-radius:8px;font-size:13px;" +
      "background:rgba(255,255,255,.05);color:inherit;border:1px solid var(--oh-border-variant,#2a2f3a);";
    const CARD =
      "background:rgba(255,255,255,.04);border-radius:10px;padding:12px;" +
      "border:1px solid var(--oh-border-variant,#2a2f3a);";

    const shell = el("section",
      "height:100%;min-height:100%;padding:.75rem;color:var(--oh-text-primary,#fff);" +
      "background:var(--oh-color-base,#0b0f14);font-family:system-ui,sans-serif;" +
      "display:flex;flex-direction:column;gap:8px;box-sizing:border-box;overflow:hidden;");
    // Route ya canvas huweka container `min-h-full` (height auto), hivyo `height:100%`
    // ya shell haitatatuliwa. Ipe container urefu dhahiri ili editor ijae skrini.
    container.style.height = "100%";
    container.style.minHeight = "0";
    container.append(shell);

    let uiLang = localStorage.getItem("mtechos_lang") || "sw";
    const L = {
      sw: {
        title: "MTECHOS IDE", sub: "Monaco kamili — faili nyingi, ghost-text, Cmd+K, LSP, terminal. Kama Cursor/Devin.",
        tabs: { chat: "💬 Chat", editor: "📝 Editor", forge: "⚡ Kujenga", sdlc: "🧭 SDLC", models: "🧩 Models", providers: "🔑 Providers", engine: "⚙️ Injini", agentic: "🧠 Ubongo", mcp: "🔌 MCP", acp: "🤖 ACP", plugins: "🧱 Plugins", connectors: "🔗 Connectors", memory: "🧠 Kumbukumbu", emulator: "🖥️ Emulator", ai: "🧰 AI" },
        send: "Tuma", thinking: "Inafikiri…", ph: "Uliza kitu, au eleza unachotaka kujenga…",
        gen: "⚡ Zalisha mradi", ready: "Tayari", building: "Inajenga…",
        assess: "🧭 Pima ugumu", plan: "📐 Panga", run: "🚀 Endesha SDLC",
        key: "API Key", saveKey: "Hifadhi Key", test: "Jaribu (verify)", saved: "Imehifadhiwa",
        download: "⬇️ Pakua", use: "Tumia",
        emulate: "🖥️ Anzisha emulator", sessions: "Sessions",
        quick: "Anza haraka", newFile: "➕ Faili", newFolder: "➕ Folder", refresh: "🔄", save: "💾 Hifadhi",
        open: "Fungua", lang: "Lugha", detect: "tambua kiotomatiki", close: "Funga",
        agents: "Agents", plugins: "Plugins", connectors: "Connectors",
      },
      en: {
        title: "MTECHOS IDE", sub: "Full Monaco editor — multi-file, ghost-text, Cmd+K, LSP, terminal. Like Cursor/Devin.",
        tabs: { chat: "💬 Chat", editor: "📝 Editor", forge: "⚡ Build", sdlc: "🧭 SDLC", models: "🧩 Models", providers: "🔑 Providers", engine: "⚙️ Engine", agentic: "🧠 Brain", mcp: "🔌 MCP", acp: "🤖 ACP", plugins: "🧱 Plugins", connectors: "🔗 Connectors", memory: "🧠 Memory", emulator: "🖥️ Emulator", ai: "⚙️ AI" },
        send: "Send", thinking: "Thinking…", ph: "Ask something, or describe what you want to build…",
        gen: "⚡ Generate project", ready: "Ready", building: "Building…",
        assess: "🧭 Assess", plan: "📐 Plan", run: "🚀 Run SDLC",
        key: "API Key", saveKey: "Save key", test: "Test (verify)", saved: "Saved",
        download: "⬇️ Download", use: "Use",
        emulate: "🖥️ Start emulator", sessions: "Sessions",
        quick: "Quick start", newFile: "➕ File", newFolder: "➕ Folder", refresh: "🔄", save: "💾 Save",
        open: "Open", lang: "Language", detect: "auto-detect", close: "Close",
        agents: "Agents", plugins: "Plugins", connectors: "Connectors",
      },
    };
    const t = () => L[uiLang] || L.sw;

    const header = el("div", "display:flex;gap:10px;align-items:center;justify-content:space-between;flex-wrap:wrap;");
    const brand = el("div", "display:flex;align-items:center;gap:10px;");
    const logo = el("div",
      "width:34px;height:34px;border-radius:9px;flex:none;display:flex;align-items:center;justify-content:center;" +
      "font-weight:800;font-size:13px;letter-spacing:.5px;color:#fff;" +
      "background:linear-gradient(135deg,var(--oh-brand,#5865f2),#a855f7);" +
      "box-shadow:0 2px 10px rgba(88,101,242,.35);", "MT");
    const hbox = el("div", "display:flex;flex-direction:column;gap:2px;");
    hbox.append(el("h2", "font-size:1.2rem;margin:0;font-weight:700;", t().title));
    hbox.append(el("p", "margin:0;font-size:12px;opacity:.7;", t().sub));
    brand.append(logo, hbox);
    header.append(brand);
    const langSel = el("select",
      "padding:6px 10px;border-radius:8px;background:rgba(255,255,255,.05);color:inherit;" +
      "border:1px solid var(--oh-border-variant,#2a2f3a);font-size:12px;");
    for (const [code, name] of [["sw", "Kiswahili"], ["en", "English"]]) {
      const o = el("option", null, name); o.value = code;
      if (code === uiLang) o.selected = true; langSel.append(o);
    }
    header.append(langSel);
    shell.append(header);

    const tabBar = el("div", "display:flex;gap:6px;flex-wrap:wrap;");
    const tabNames = ["editor", "chat", "forge", "sdlc", "models", "providers", "engine", "agentic", "mcp", "acp", "plugins", "connectors", "memory", "emulator", "ai"];
    // Tabs zinazoonekana: ACP/MCP/Models/Providers/AI zimehamia Settings (LLM,
    // MCP, Agent/ACP) — IDE inabaki na editor (uso mkuu), chat, ujenzi, na zana za mradi.
    const visibleTabs = ["editor", "chat", "forge", "sdlc", "engine", "agentic", "plugins", "connectors", "memory", "emulator"];
    const tabBtns = {};
    const panels = {};
    const body = el("div", "display:flex;flex-direction:column;gap:12px;flex:1;min-height:0;");
    shell.append(tabBar, body);

    function showTab(name) {
      for (const k of tabNames) {
        panels[k].style.display = k === name ? "flex" : "none";
        tabBtns[k].style.background = k === name ? "var(--oh-brand,#5865f2)" : "transparent";
        tabBtns[k].style.color = k === name ? "#fff" : "inherit";
        tabBtns[k].style.borderColor = k === name ? "transparent" : "var(--oh-border-variant,#2a2f3a)";
      }
    }
    for (const name of tabNames) {
      const b = el("button", BTN2, t().tabs[name]);
      b.onclick = () => showTab(name);
      tabBtns[name] = b; tabBar.append(b);
      const p = el("div", "flex-direction:column;gap:10px;flex:1;min-height:0;");
      panels[name] = p; body.append(p);
    }
    // Ficha tabs zilizohamishwa Settings (bado zinasajiliwa ili showTab/panels
    // zisivunjike, lakini hazionekani kwenye tab bar).
    for (const name of tabNames) {
      if (!visibleTabs.includes(name)) tabBtns[name].style.display = "none";
    }

    const mkLog = (p) => {
      const log = el("pre",
        "flex:1;min-height:180px;max-height:46vh;overflow:auto;white-space:pre-wrap;" +
        "font-size:12px;line-height:1.6;background:rgba(255,255,255,.04);" +
        "border-radius:8px;padding:12px;margin:0;", t().ready);
      p.append(log);
      return (s) => {
        if (s === null) { log.textContent = ""; return; }
        log.textContent += (log.textContent ? "\n" : "") + s;
        log.scrollTop = log.scrollHeight;
      };
    };
    const sse = async (url, payload, onEvent) => {
      const res = await fetch(url, {
        method: "POST",
        headers: authHeaders({ "Content-Type": "application/json" }),
        body: JSON.stringify(payload),
      });
      if (!res.ok || !res.body) throw new Error("HTTP " + res.status);
      const reader = res.body.getReader(); const dec = new TextDecoder(); let buf = "";
      for (;;) {
        const { value, done } = await reader.read(); if (done) break;
        buf += dec.decode(value, { stream: true });
        const parts = buf.split("\n\n"); buf = parts.pop();
        for (const part of parts) {
          const line = part.split("\n").find((l) => l.startsWith("data:"));
          if (!line) continue;
          let ev; try { ev = JSON.parse(line.slice(5).trim()); } catch { continue; }
          onEvent(ev);
        }
      }
    };

    // ══════════ TAB: CHAT (MOJA — chat + engineer + autopilot) ══════════
    // Chat MOJA inaunganisha: 💬 Chat (ask), 🚀 Engineer (agentic stream),
    // 🧠 Super (Super Intellectual Engineer — hatua 10), 🛰️ Autopilot (forge run → build).
    // Kumbukumbu huhifadhiwa kwenye DB.
    (() => {
      const p = panels.chat;
      const wrap = el("div", "display:flex;gap:12px;align-items:flex-start;flex-wrap:wrap;");
      // sidebar: kumbukumbu (conversations)
      const side = el("div", "display:flex;flex-direction:column;gap:6px;min-width:190px;max-width:230px;flex:1;");
      const sideHead = el("div", "display:flex;gap:6px;align-items:center;justify-content:space-between;");
      sideHead.append(el("span", "font-size:11px;opacity:.6;font-weight:600;", t().memory));
      const newB = el("button", BTN2, "➕"); newB.title = "Mazungumzo mapya";
      sideHead.append(newB); side.append(sideHead);
      const convList = el("div", "display:flex;flex-direction:column;gap:4px;max-height:44vh;overflow:auto;");
      side.append(convList);
      // main
      const main = el("div", "display:flex;flex-direction:column;gap:8px;flex:2;min-width:280px;");
      const modeRow = el("div", "display:flex;gap:6px;flex-wrap:wrap;align-items:center;");
      const modes = [["chat", "💬 Chat"], ["engineer", "🚀 Engineer"], ["super", "🧠 Super"], ["autopilot", "🛰️ Autopilot"]];
      let mode = "chat"; const modeBtns = {};
      for (const [id, label] of modes) {
        const b = el("button", BTN2, label);
        b.onclick = () => { mode = id; for (const k in modeBtns) {
          modeBtns[k].style.background = k === id ? "var(--oh-brand,#5865f2)" : "transparent";
          modeBtns[k].style.color = k === id ? "#fff" : "inherit";
        } };
        modeBtns[id] = b; modeRow.append(b);
      }
      modeBtns.chat.style.background = "var(--oh-brand,#5865f2)"; modeBtns.chat.style.color = "#fff";
      main.append(modeRow);
      const row = el("div", "display:flex;gap:8px;flex-wrap:wrap;align-items:center;");
      row.append(el("span", "font-size:11px;opacity:.6;", t().quick));
      for (const [label, seed] of [["Website", "tovuti ya kuuza bidhaa"], ["Mobile App", "app ya simu ya kumbukumbu"],
        ["IoT", "kifaa cha IoT cha kupima joto"], ["Game Dev", "mchezo mdogo wa 2D"],
        ["Autopilot", "jenga na uendeshe mradi"], ["Emulator", "endesha kwenye emulator"]]) {
        const c = el("button", BTN2, label);
        c.onclick = () => { ta.value = seed; ta.focus(); };
        row.append(c);
      }
      main.append(row);
      const ta = el("textarea", INP + "resize:vertical;"); ta.rows = 3; ta.placeholder = t().ph;
      main.append(ta);
      const brow = el("div", "display:flex;gap:10px;align-items:center;");
      const send = el("button", BTN, t().send);
      const status = el("span", "font-size:12px;opacity:.75;", t().ready);
      brow.append(send, status); main.append(brow);
      const say = mkLog(main);
      // Mtiririko: delta zinaandikwa kwenye mstari mmoja (bila newline).
      let streamLog = null;
      const sayDelta = (txt) => {
        if (!streamLog) { streamLog = el("pre", "white-space:pre-wrap;font-size:12px;line-height:1.6;background:rgba(255,255,255,.04);border-radius:8px;padding:12px;margin:0;", "🤖 "); main.append(streamLog); }
        streamLog.textContent += txt;
        streamLog.scrollTop = streamLog.scrollHeight;
      };
      const endStream = () => { streamLog = null; };
      wrap.append(side, main); p.append(wrap);

      let convId = null;
      const post = (url, body) => fetch(url, { method: "POST", headers: authHeaders({ "Content-Type": "application/json" }), body: JSON.stringify(body) }).then((r) => r.json());
      async function ensureConv(firstMsg) {
        if (convId) return convId;
        const j = await post("/api/ide/conversations", { title: (firstMsg || "Mazungumzo").slice(0, 40), lang: uiLang });
        convId = j.id || null; await loadConvs(); return convId;
      }
      async function loadConvs() {
        const j = await (await fetch("/api/ide/conversations", { headers: authHeaders() })).json();
        convList.textContent = "";
        for (const c of j.conversations || []) {
          const it = el("div", CARD + "display:flex;gap:6px;align-items:center;justify-content:space-between;cursor:pointer;padding:8px;");
          it.append(el("span", "font-size:11px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;", c.title));
          const del = el("button", BTN2, "✕"); del.style.padding = "2px 6px";
          del.onclick = async (ev) => { ev.stopPropagation(); await fetch("/api/ide/conversations/" + c.id, { method: "DELETE", headers: authHeaders() }); if (convId === c.id) convId = null; await loadConvs(); };
          it.append(del);
          it.onclick = async () => {
            convId = c.id;
            const d = await (await fetch("/api/ide/conversations/" + c.id, { headers: authHeaders() })).json();
            say(null); // clear
            for (const m of d.messages || []) say((m.role === "user" ? "🧑 " : "🤖 ") + m.content);
          };
          convList.append(it);
        }
      }
      newB.onclick = () => { convId = null; say(null); };
      loadConvs();

      let busy = false;
      send.onclick = async () => {
        const prompt = ta.value.trim(); if (!prompt || busy) return;
        busy = true; send.disabled = true; status.textContent = t().thinking;
        say("🧑 " + prompt); ta.value = "";
        await ensureConv(prompt);
        if (convId) post("/api/ide/conversations/" + convId + "/message", { role: "user", content: prompt });
        const record = (txt) => { say("🤖 " + txt); if (convId) post("/api/ide/conversations/" + convId + "/message", { role: "assistant", content: txt }); };
        try {
          if (mode === "chat") {
            // Chat safi kwa MTIRIRIKO (SSE): maandishi yanaonekana mara moja.
            let acc = "";
            await sse("/api/ide/agent/stream", { prompt, ide_context: { language: uiLang } }, (ev) => {
              if (ev.kind === "delta") { acc += ev.text; sayDelta(ev.text); }
              else if (ev.kind === "status") status.textContent = ev.text;
              else if (ev.kind === "done") {
                endStream();
                // Kilichotiririka kiko tayari kwenye skrini; hifadhi tu (bila
                // kurudia). Bila deltas (mf. jibu la haraka), andika mara moja.
                const final = (ev.text && ev.text.trim()) || acc;
                if (!acc && final) say("🤖 " + final);
                if (convId && final) post("/api/ide/conversations/" + convId + "/message", { role: "assistant", content: final });
              }
            });
          } else if (mode === "engineer") {
            await sse("/api/ide/agent/agentic/stream", { idea: prompt, kind: "auto", maxSteps: 18 }, (ev) => {
              if (ev.type === "start") say("📁 " + ev.project + " · model " + ev.model);
              else if (ev.type === "write") say("   " + (ev.status === "ok" ? "✅" : "⚠️") + " write " + ev.path);
              else if (ev.type === "verify") { if (ev.ok !== null) say((ev.ok ? "✅" : "⛔") + " VERIFY " + ev.ok); }
              else if (ev.type === "done") record(ev.summary || "imekamilika");
              else if (ev.type === "error") say("⚠️ " + ev.error);
            });
          } else if (mode === "super") {
            // SUPER INTELLECTUAL ENGINEER: hatua 10 za injini zote halisi.
            const j = await post("/api/super/engineer", { goal: prompt, kind: "auto", execute: true, max_steps: 18 });
            if (!j || j.ok === false) { say("⚠️ " + ((j && j.error) || "kosa")); }
            else {
              for (const st of j.steps || []) {
                const d = st.detail || {};
                if (st.step === "recon") say("🔎 RECON · " + (d.workspace || "") + " · lang " + (d.lang || "") + " · " + ((d.complexity && d.complexity.level) || ""));
                else if (st.step === "policy") say("🛡️ SERA · ethics " + (d.ethics_allowed ? "✅" : "⛔") + " · HITL " + (d.requires_hitl ? "ndio" : "hapana"));
                else if (st.step === "plan") say("📐 MPANGO · awamu " + ((d.phases || []).length) + " · vigezo " + ((d.acceptance_criteria || []).length));
                else if (st.step === "ideate") say("💡 HOJA · dunia " + (d.orthogonal && d.orthogonal.worlds_explored));
                else if (st.step === "descend") say("🔧 DESCEND · score " + d.best_score);
                else if (st.step === "execute") say("🚀 UTENDAJI · " + st.status + (d.project ? " · " + d.project : ""));
                else if (st.step === "review") say("🧪 UKAGUZI · engineer " + d.engineer_score + " · usalama " + d.security_grade);
                else if (st.step === "megaflow") say("🌊 MEGAFLOW · throughput " + d.throughput_tasks_per_tick);
              }
              say("🏁 " + j.verdict + " · score " + j.score);
              record("Super Engineer: " + j.verdict + " (score " + j.score + ")");
            }
          } else { // autopilot
            let summary = "";
            await sse("/api/forge/run", { spec: prompt, engineer: true }, (ev) => {
              if (ev.type === "blueprint") say("📐 Blueprint: " + ev.project + " [" + ev.kind + "/" + ev.lang + "]");
              else if (ev.type === "file") say("   " + (ev.ok ? "✅" : "⚠️") + " " + ev.path + " (" + ev.bytes + "B)");
              else if (ev.type === "done") { summary = ev.project + " — faili safi " + ev.files_clean + "/" + ev.files_total; say("🏁 " + summary); }
              else if (ev.type === "error") say("⚠️ " + (ev.error || "kosa"));
            });
            if (summary) record(summary);
          }
        } catch (e) { say("⚠️ " + e.message); }
        finally { busy = false; send.disabled = false; status.textContent = t().ready; await loadConvs(); }
      };
    })();

    // ══════════ TAB: EDITOR (Monaco KAMILI — tabs, ghost-text, Cmd+K, LSP, terminal, tafuta) ══════════
    // Editor KAMILI (kama Cursor/Devin): Monaco + faili nyingi (tabs), ghost-text
    // (inline completion ya AI), Cmd+K inline edit, LSP (diagnostics/hover/
    // completion), terminal halisi, tafuta kwenye mradi, na vitendo vya AI.
    (() => {
      const p = panels.editor;
      let monaco = null, editor = null;
      const models = {};        // path → Monaco model
      const tabs = [];          // paths zilizofunguliwa
      const dirty = {};         // path → bool
      let active = null;
      let langsCatalog = { languages: [], by_extension: {} };
      let lspServers = null;
      let diagTimer = null;
      let toastTimer = null;

      const MONO_BASE = "/ide/monaco/vs";
      const status = el("span", "font-size:12px;opacity:.75;", "Monaco…");

      const postJson = async (url, body) => {
        try {
          const r = await fetch(url, { method: "POST", headers: authHeaders({ "Content-Type": "application/json" }), body: JSON.stringify(body) });
          return await r.json();
        } catch (e) { return { ok: false, error: String((e && e.message) || e) }; }
      };
      const getJson = async (url) => {
        try { return await (await fetch(url, { headers: authHeaders() })).json(); }
        catch (e) { return { ok: false, error: String((e && e.message) || e) }; }
      };
      function toast(msg) {
        status.textContent = msg;
        if (toastTimer) clearTimeout(toastTimer);
        toastTimer = setTimeout(() => { status.textContent = t().ready; }, 2800);
      }

      // ---------- Monaco (offline kutoka /ide/monaco/vs) ----------
      function loadMonaco(cb) {
        if (monaco) return cb();
        if (window.monaco) { monaco = window.monaco; defineThemes(); registerProviders(); return cb(); }
        const s = document.createElement("script");
        s.src = MONO_BASE + "/loader.js";
        s.onload = () => {
          window.require.config({ paths: { vs: MONO_BASE } });
          window.require(["vs/editor/editor.main"], () => {
            monaco = window.monaco; defineThemes(); registerProviders(); cb();
          });
        };
        s.onerror = () => toast("⚠️ Monaco haipatikani (/ide/monaco/vs/loader.js)");
        document.head.append(s);
      }
      function defineThemes() {
        monaco.editor.defineTheme("mbilinyi-dark", {
          base: "vs-dark", inherit: true,
          rules: [
            { token: "comment", foreground: "5c6370", fontStyle: "italic" },
            { token: "keyword", foreground: "c678dd" },
            { token: "string", foreground: "98c379" },
            { token: "number", foreground: "d19a66" },
            { token: "type", foreground: "e5c07b" },
            { token: "function", foreground: "61afef" },
          ],
          colors: { "editor.background": "#0d1117", "editor.lineHighlightBackground": "#161b22" },
        });
      }
      const relOf = (model) => model.uri.path.replace(/^\/+/, "");
      function hoverText(r) {
        if (!r) return "";
        if (typeof r === "string") return r;
        const c = r.contents;
        if (typeof c === "string") return c;
        if (Array.isArray(c)) return c.map((x) => (typeof x === "string" ? x : ((x && x.value) || ""))).join("\n\n");
        if (c && c.value) return c.value;
        return "";
      }
      function lspItems(r) {
        const raw = Array.isArray(r) ? r : (r && Array.isArray(r.items) ? r.items : []);
        return raw.map((it) => ({
          label: it.label || it.insertText || "",
          kind: monaco.languages.CompletionItemKind.Text,
          insertText: it.insertText || it.label || "",
          detail: it.detail || "",
        })).filter((x) => x.label);
      }
      function registerProviders() {
        // Ghost text (inline completion) — kama "Cursor Tab".
        monaco.languages.registerInlineCompletionsProvider({ pattern: "**" }, {
          provideInlineCompletions: async (model, position) => {
            const prefix = model.getValueInRange({ startLineNumber: 1, startColumn: 1, endLineNumber: position.lineNumber, endColumn: position.column });
            const last = model.getLineCount();
            const suffix = model.getValueInRange({ startLineNumber: position.lineNumber, startColumn: position.column, endLineNumber: last, endColumn: model.getLineMaxColumn(last) });
            const j = await postJson("/api/ide/complete", { path: relOf(model), prefix: prefix.slice(-4000), suffix: suffix.slice(0, 1000), language: model.getLanguageId(), maxTokens: 64 });
            if (!j || !j.ok || !j.completion) return { items: [] };
            return { items: [{ insertText: j.completion }] };
          },
          freeInlineCompletions: () => {},
        });
        monaco.languages.registerHoverProvider({ pattern: "**" }, {
          provideHover: async (model, position) => {
            const j = await postJson("/api/ide/lsp/hover", { path: relOf(model), line: position.lineNumber - 1, character: position.column - 1 });
            const txt = hoverText(j && j.result);
            return txt ? { contents: [{ value: txt }] } : null;
          },
        });
        monaco.languages.registerCompletionItemProvider({ pattern: "**" }, {
          provideCompletionItems: async (model, position) => {
            const j = await postJson("/api/ide/lsp/completion", { path: relOf(model), line: position.lineNumber - 1, character: position.column - 1 });
            return { suggestions: lspItems(j && j.result) };
          },
        });
      }
      function detectLang(path) {
        const name = (path || "").split("/").pop() || "";
        if (name === "Dockerfile") return "dockerfile";
        if (name === "Makefile") return "shell";
        const ext = name.includes(".") ? name.split(".").pop().toLowerCase() : "";
        return (langsCatalog.by_extension || {})[ext] || "plaintext";
      }

      // ---------- DOM ----------
      const bar = el("div", "display:flex;gap:6px;flex-wrap:wrap;align-items:center;");
      const newFile = el("button", BTN2, t().newFile);
      const newFolder = el("button", BTN2, t().newFolder);
      const refreshB = el("button", BTN2, t().refresh);
      const saveB = el("button", BTN, t().save);
      const searchBox = el("input", "padding:6px 10px;border-radius:8px;background:rgba(255,255,255,.05);color:inherit;border:1px solid var(--oh-border-variant,#2a2f3a);font-size:12px;min-width:150px;flex:1;");
      searchBox.placeholder = "🔍 Tafuta kwenye mradi…";
      bar.append(newFile, newFolder, refreshB, saveB, searchBox, status);
      p.append(bar);

      const grid = el("div", "display:flex;gap:10px;align-items:stretch;flex-wrap:wrap;flex:1;min-height:0;");
      const leftCol = el("div", CARD + "min-width:220px;max-width:300px;flex:1;height:100%;overflow:auto;font-size:12px;display:flex;flex-direction:column;gap:2px;");
      leftCol.append(el("div", "font-weight:700;font-size:11px;opacity:.7;", "EXPLORER"));
      const treeBox = el("div", "display:flex;flex-direction:column;gap:1px;");
      leftCol.append(treeBox);

      const centerCol = el("div", "flex:2;min-width:340px;display:flex;flex-direction:column;gap:6px;height:100%;min-height:0;");
      const tabBar = el("div", "display:flex;gap:4px;flex-wrap:wrap;align-items:center;min-height:28px;");
      const editorBox = el("div", "flex:1;min-height:300px;border-radius:10px;overflow:hidden;border:1px solid var(--oh-border-variant,#2a2f3a);");
      const statusBar = el("div", "display:flex;gap:12px;font-size:11px;opacity:.75;padding:2px 4px;");
      centerCol.append(tabBar, editorBox, statusBar);

      const rightCol = el("div", "min-width:240px;max-width:340px;flex:1;display:flex;flex-direction:column;gap:8px;height:100%;min-height:0;overflow:auto;");
      const aiCard = el("div", CARD);
      aiCard.append(el("div", "font-weight:700;font-size:11px;opacity:.7;margin-bottom:6px;", "AI (Cmd+K)"));
      const aiRow = el("div", "display:flex;gap:6px;flex-wrap:wrap;");
      for (const pair of [["Eleza", "eleza code hii kwa ufupi"], ["Rekebisha", "rekebisha hitilafu zote kwenye code hii"], ["Boresha", "boresha ubora wa code hii bila kubadilisha tabia"], ["Testi", "ongeza testi za unit kwa code hii"], ["Nyaraka", "ongeza docstrings/nyaraka kwa code hii"]]) {
        const b = el("button", BTN2, pair[0]);
        b.onclick = () => inlineEdit(pair[1]);
        aiRow.append(b);
      }
      const askInput = el("input", "width:100%;margin-top:6px;padding:6px 8px;border-radius:8px;background:rgba(255,255,255,.05);color:inherit;border:1px solid var(--oh-border-variant,#2a2f3a);font-size:12px;");
      askInput.placeholder = "Uliza kuhusu faili… (Enter)";
      askInput.onkeydown = (e) => { if (e.key === "Enter") askAgent(); };
      const aiOut = el("pre", "max-height:18vh;overflow:auto;white-space:pre-wrap;font-size:11px;background:rgba(0,0,0,.2);border-radius:8px;padding:8px;margin:6px 0 0;", "");
      aiCard.append(aiRow, askInput, aiOut);

      const termCard = el("div", CARD);
      termCard.append(el("div", "font-weight:700;font-size:11px;opacity:.7;margin-bottom:6px;", "TERMINAL"));
      const termIn = el("input", "flex:1;padding:6px 8px;border-radius:8px;background:rgba(255,255,255,.05);color:inherit;border:1px solid var(--oh-border-variant,#2a2f3a);font-size:12px;");
      termIn.placeholder = "amri (mfano: ls -la)";
      const termRun = el("button", BTN, "▶");
      const termRow = el("div", "display:flex;gap:6px;");
      termRow.append(termIn, termRun);
      const termOut = el("pre", "max-height:18vh;overflow:auto;white-space:pre-wrap;font-size:11px;background:rgba(0,0,0,.25);border-radius:8px;padding:8px;margin:6px 0 0;", "");
      termCard.append(termRow, termOut);

      const searchCard = el("div", CARD);
      searchCard.append(el("div", "font-weight:700;font-size:11px;opacity:.7;margin-bottom:6px;", "TAFUTA"));
      const searchOut = el("div", "max-height:22vh;overflow:auto;font-size:11px;");
      searchCard.append(searchOut);

      rightCol.append(aiCard, termCard, searchCard);
      grid.append(leftCol, centerCol, rightCol);
      p.append(grid);

      // ---------- tabs ----------
      function renderTabs() {
        tabBar.replaceChildren();
        for (const path of tabs) {
          const b = el("button", BTN2, (dirty[path] ? "● " : "") + path.split("/").pop());
          b.style.padding = "4px 8px"; b.style.fontSize = "11px";
          if (path === active) { b.style.background = "var(--oh-brand,#5865f2)"; b.style.color = "#fff"; }
          b.title = path;
          b.onclick = () => activateTab(path);
          const x = el("span", "margin-left:6px;opacity:.7;", "✕");
          x.onclick = (ev) => { ev.stopPropagation(); closeTab(path); };
          b.append(x);
          tabBar.append(b);
        }
        if (!tabs.length) tabBar.append(el("span", "font-size:11px;opacity:.5;", "Fungua faili kutoka Explorer →"));
      }
      function activateTab(path) {
        active = path;
        loadMonaco(() => {
          if (!editor) {
            editor = monaco.editor.create(editorBox, {
              theme: "mbilinyi-dark", automaticLayout: true, fontSize: 13,
              minimap: { enabled: true }, scrollBeyondLastLine: false, smoothScrolling: true,
              fontFamily: "'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, monospace",
              inlineSuggest: { enabled: true },
            });
            bindEditor();
          }
          editor.setModel(models[path] || null);
          editor.focus();
          renderTabs(); refreshStatus(); scheduleDiagnostics(path);
        });
      }
      function closeTab(path) {
        if (dirty[path] && !window.confirm("Faili " + path + " haijahifadhiwa. Funga bila kuhifadhi?")) return;
        const idx = tabs.indexOf(path);
        if (idx >= 0) tabs.splice(idx, 1);
        if (models[path]) { models[path].dispose(); delete models[path]; }
        delete dirty[path];
        if (active === path) active = tabs[idx] || tabs[idx - 1] || null;
        renderTabs();
        if (active) activateTab(active);
        else { if (editor) editor.setModel(null); refreshStatus(); }
      }
      function openPath(path) {
        return new Promise((resolve) => {
          if (models[path]) { activateTab(path); return resolve(true); }
          loadMonaco(async () => {
            const j = await getJson("/api/ide/file?path=" + encodeURIComponent(path));
            if (!j.ok) { toast("⚠️ " + (j.error || "kosa")); return resolve(false); }
            const lang = j.syntax || detectLang(path);
            const model = monaco.editor.createModel(j.content, lang, monaco.Uri.file("/" + path));
            model.onDidChangeContent(() => { markDirty(path); scheduleDiagnostics(path); });
            models[path] = model;
            if (!tabs.includes(path)) tabs.push(path);
            renderTabs(); activateTab(path); resolve(true);
          });
        });
      }
      async function openPathAt(path, line) {
        const ok = await openPath(path);
        if (!ok || !editor) return;
        activateTab(path);
        editor.revealLineInCenter(line);
        editor.setPosition({ lineNumber: line, column: 1 });
        editor.focus();
      }
      function markDirty(path) { dirty[path] = true; renderTabs(); refreshStatus(); }
      function refreshStatus() {
        const parts = [];
        if (editor && editor.getPosition()) { const pos = editor.getPosition(); parts.push("Ln " + pos.lineNumber + ", Col " + pos.column); }
        if (active && models[active]) parts.push(models[active].getLanguageId());
        parts.push(active ? (dirty[active] ? "● haijahifadhiwa" : "imehifadhiwa") : "hakuna faili");
        parts.push("LSP " + (lspServers ? (lspServers.ready + "/" + lspServers.total) : "…"));
        statusBar.replaceChildren(...parts.map((x) => el("span", null, x)));
      }
      function bindEditor() {
        editor.addAction({ id: "mbilinyi.aiEdit", label: "AI: Hariri kwa maelekezo (Cmd+K)", keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyK], contextMenuGroupId: "1_modification", run: () => inlineEdit(null) });
        editor.addAction({ id: "mbilinyi.save", label: "Hifadhi (Cmd+S)", keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS], run: () => saveActive() });
        editor.addAction({ id: "mbilinyi.explain", label: "AI: Eleza kipande", run: () => inlineEdit("eleza code hii kwa ufupi") });
        editor.onDidChangeCursorPosition(() => refreshStatus());
      }
      async function saveActive() {
        if (!active || !models[active]) return;
        const j = await postJson("/api/ide/file", { path: active, content: models[active].getValue() });
        if (j && j.ok) { dirty[active] = false; renderTabs(); refreshStatus(); toast("✅ " + active); scheduleDiagnostics(active); }
        else toast("⚠️ " + ((j && j.error) || "hifadhi imeshindwa"));
      }
      function scheduleDiagnostics(path) {
        if (diagTimer) clearTimeout(diagTimer);
        diagTimer = setTimeout(() => runDiagnostics(path), 900);
      }
      async function runDiagnostics(path) {
        if (!models[path] || !monaco) return;
        const j = await postJson("/api/ide/lsp/diagnostics", { path, code: models[path].getValue() });
        const diags = (j && j.result) || [];
        const markers = (Array.isArray(diags) ? diags : []).map((d) => {
          const st = (d.range && d.range.start) || {};
          const en = (d.range && d.range.end) || {};
          return {
            severity: d.severity === 1 ? monaco.MarkerSeverity.Error : d.severity === 2 ? monaco.MarkerSeverity.Warning : monaco.MarkerSeverity.Info,
            message: d.message || "",
            startLineNumber: (st.line || 0) + 1, startColumn: (st.character || 0) + 1,
            endLineNumber: (en.line || 0) + 1, endColumn: (en.character || 0) + 2,
          };
        });
        monaco.editor.setModelMarkers(models[path], "lsp", markers);
      }

      // ---------- tree ----------
      async function buildTree() {
        const j = await getJson("/api/ide/tree");
        treeBox.replaceChildren();
        const entries = (j && (j.entries || j.tree)) || [];
        if (!entries.length) { treeBox.append(el("div", "opacity:.6;padding:6px;", "Workspace tupu — tengeneza faili au folder.")); return; }
        for (const n of entries) {
          const depth = (n.path || "").split("/").length - 1;
          const row = el("div", "display:flex;align-items:center;gap:6px;padding:3px 4px;cursor:pointer;border-radius:6px;padding-left:" + (4 + depth * 12) + "px;");
          row.append(el("span", null, (n.type === "dir" ? "📁 " : "📄 ") + n.name));
          row.title = n.path;
          if (n.type === "file") row.onclick = () => openPath(n.path);
          const del = el("button", BTN2, "✕");
          del.style.cssText = "margin-left:auto;padding:1px 5px;font-size:10px;opacity:.5;";
          del.onclick = async (ev) => {
            ev.stopPropagation();
            if (!window.confirm("Futa " + n.path + "?")) return;
            const r = await postJson("/api/ide/delete", { path: n.path });
            if (r && r.ok) { if (models[n.path]) closeTab(n.path); buildTree(); toast("🗑 " + n.path); }
            else toast("⚠️ " + ((r && r.error) || "kufuta imeshindwa"));
          };
          row.append(del);
          treeBox.append(row);
        }
      }
      newFile.onclick = async () => {
        const path = window.prompt("Jina la faili (mfano src/app.py):"); if (!path) return;
        const j = await postJson("/api/ide/file", { path, content: "" });
        if (!j.ok) return toast("⚠️ " + (j.error || "kosa"));
        await buildTree(); openPath(path);
      };
      newFolder.onclick = async () => {
        const path = window.prompt("Jina la folder (mfano src/components):"); if (!path) return;
        const j = await postJson("/api/ide/mkdir", { path });
        if (!j.ok) return toast("⚠️ " + (j.error || "kosa"));
        await buildTree(); toast("📁 " + path);
      };
      refreshB.onclick = () => buildTree();
      saveB.onclick = () => saveActive();

      // ---------- tafuta ----------
      async function runSearch() {
        const q = searchBox.value.trim(); if (!q) return;
        searchOut.textContent = "…";
        const j = await postJson("/api/ide/search", { query: q });
        searchOut.replaceChildren();
        const results = (j && j.results) || [];
        if (!results.length) { searchOut.append(el("div", "opacity:.6;", "hakuna matokeo")); return; }
        searchOut.append(el("div", "opacity:.6;margin-bottom:4px;", results.length + " matokeo"));
        for (const r of results) {
          const row = el("div", "padding:3px 4px;cursor:pointer;border-radius:6px;");
          row.append(el("div", "opacity:.85;", r.path + ":" + r.line));
          row.append(el("div", "opacity:.55;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;", r.text));
          row.onclick = () => openPathAt(r.path, r.line);
          searchOut.append(row);
        }
      }
      searchBox.onkeydown = (e) => { if (e.key === "Enter") runSearch(); };

      // ---------- terminal ----------
      async function runTerm() {
        const cmd = termIn.value.trim(); if (!cmd) return;
        termIn.value = "";
        termOut.textContent += (termOut.textContent ? "\n" : "") + "$ " + cmd;
        const j = await postJson("/api/ide/terminal", { command: cmd, workdir: active ? active.split("/").slice(0, -1).join("/") : "" });
        const out = (j && (j.output || j.error)) || "(hakuna matokeo)";
        termOut.textContent += "\n" + out;
        termOut.scrollTop = termOut.scrollHeight;
      }
      termIn.onkeydown = (e) => { if (e.key === "Enter") runTerm(); };
      termRun.onclick = () => runTerm();

      // ---------- AI ----------
      async function askAgent() {
        const q = askInput.value.trim(); if (!q) return;
        askInput.value = "";
        aiOut.textContent = "…";
        const ctx = active && models[active] ? { file: active, language: models[active].getLanguageId() } : {};
        const j = await postJson("/api/ide/agent", { prompt: q + (active ? " (faili: " + active + ")" : ""), ide_context: ctx });
        const reply = j && j.reply ? (typeof j.reply === "string" ? j.reply : (j.reply.answer_sw || JSON.stringify(j.reply))) : (j && j.error ? "⚠️ " + j.error : "(hakuna jibu)");
        aiOut.textContent = reply;
      }
      async function inlineEdit(preset) {
        if (!editor || !active || !models[active]) return toast("⚠️ fungua faili kwanza");
        const model = models[active];
        const sel = editor.getSelection();
        const hasSel = sel && !sel.isEmpty();
        const range = hasSel ? sel : model.getFullModelRange();
        const instruction = preset || window.prompt("Maelekezo ya AI (Cmd+K):", "boresha code hii");
        if (!instruction) return;
        const original = model.getValueInRange(range);
        const prefix = model.getValueInRange({ startLineNumber: 1, startColumn: 1, endLineNumber: range.startLineNumber, endColumn: range.startColumn });
        const suffix = model.getValueInRange({ startLineNumber: range.endLineNumber, startColumn: range.endColumn, endLineNumber: model.getLineCount(), endColumn: model.getLineMaxColumn(model.getLineCount()) });
        toast("🤖 AI inahariri…");
        const j = await postJson("/api/ide/edit", { selection: original, instruction, path: active, language: model.getLanguageId(), prefix, suffix });
        if (!j || !j.ok) return toast("⚠️ " + ((j && j.error) || "inline edit imeshindwa"));
        showDiff(original, j.code, () => {
          editor.executeEdits("ai-edit", [{ range, text: j.code, forceMoveMarkers: true }]);
          editor.pushUndoStop();
          markDirty(active);
          toast("✅ AI imehariri (Ctrl+Z kurudisha)");
        });
      }
      function showDiff(before, after, apply) {
        const backdrop = el("div", "position:fixed;inset:0;background:rgba(0,0,0,.6);display:flex;align-items:center;justify-content:center;z-index:9999;");
        const modal = el("div", "background:var(--oh-color-base,#0b0f14);border:1px solid var(--oh-border-variant,#2a2f3a);border-radius:12px;padding:16px;max-width:92vw;max-height:88vh;overflow:auto;display:flex;flex-direction:column;gap:10px;");
        modal.append(el("h3", "margin:0;font-size:1rem;", "🤖 AI inline edit — hakiki mabadiliko"));
        const cols = el("div", "display:flex;gap:10px;flex-wrap:wrap;");
        const mk = (title, text) => {
          const c = el("div", "flex:1;min-width:280px;");
          c.append(el("div", "font-size:11px;opacity:.7;margin-bottom:4px;", title));
          const pre = el("pre", "max-height:60vh;overflow:auto;white-space:pre-wrap;font-size:12px;background:rgba(0,0,0,.3);border-radius:8px;padding:10px;margin:0;");
          pre.textContent = text;
          c.append(pre);
          return c;
        };
        cols.append(mk("KABLA", before), mk("BAADA", after));
        modal.append(cols);
        const row = el("div", "display:flex;gap:8px;justify-content:flex-end;");
        const ok = el("button", BTN, "✅ Tumia");
        const no = el("button", BTN2, t().close);
        ok.onclick = () => { backdrop.remove(); apply(); };
        no.onclick = () => backdrop.remove();
        row.append(ok, no);
        modal.append(row);
        backdrop.append(modal);
        document.body.append(backdrop);
      }

      // ---------- init ----------
      buildTree();
      renderTabs();
      refreshStatus();
      getJson("/api/ide/languages").then((j) => { if (j && j.languages) langsCatalog = j; });
      getJson("/api/ide/lsp/servers").then((j) => { if (j && j.ok) { lspServers = j; refreshStatus(); } });
      loadMonaco(() => { status.textContent = t().ready; refreshStatus(); });
    })();

    // ══════════ TAB: FORGE ══════════
    (() => {
      const p = panels.forge;
      const ta = el("textarea", INP + "resize:vertical;"); ta.rows = 2;
      ta.placeholder = "Mradi wako… (mfano: REST API ya TODO yenye tests)";
      p.append(ta);
      const row = el("div", "display:flex;gap:10px;align-items:center;flex-wrap:wrap;");
      const btn = el("button", BTN, t().gen);
      const status = el("span", "font-size:12px;opacity:.75;", t().ready);
      row.append(btn, status); p.append(row);
      const say = mkLog(p);
      // Quick buttons: miradi KAMILI (website/mobile/iot/gaming) yenye self-test —
      // chanzo kimoja na emulator (`/api/ide/pipeline/project`).
      const qrow = el("div", "display:flex;gap:8px;flex-wrap:wrap;align-items:center;");
      qrow.append(el("span", "font-size:11px;opacity:.6;", "Mradi kamili (halisi + self-test):"));
      for (const [kind, label] of [["website", "🌐 Website"], ["mobile", "📱 Mobile App"], ["iot", "📡 IoT"], ["gaming", "🎮 Game"]]) {
        const c = el("button", BTN2, label);
        c.onclick = async () => {
          const proj = (ta.value.trim().split("\n")[0] || kind).slice(0, 40).replace(/[^a-zA-Z0-9 _-]/g, "").trim().replace(/\s+/g, "-").toLowerCase() || kind;
          c.disabled = true; status.textContent = t().building;
          say("📦 " + kind + " → " + proj);
          try {
            const j = await (await fetch("/api/ide/pipeline/project", {
              method: "POST", headers: authHeaders({ "Content-Type": "application/json" }),
              body: JSON.stringify({ kind, project: proj }),
            })).json();
            if (!j.ok) say("⚠️ " + (j.error || "kosa"));
            else {
              say("✅ " + j.kind + " · " + j.files_written + "/" + j.files_total + " faili → " + j.path);
              say("   " + j.note);
              say("   self-test [" + j.test_cmd + "] " + (j.test && j.test.ok ? "✅ imepita" : "⛔ " + ((j.test && j.test.error) || "imeshindwa")));
              if (j.test && j.test.stdout) say("   " + String(j.test.stdout).trim().split("\n").slice(0, 6).join("\n   "));
            }
          } catch (e) { say("⚠️ " + e.message); }
          finally { c.disabled = false; status.textContent = t().ready; }
        };
        qrow.append(c);
      }
      p.append(qrow);
      let busy = false;
      btn.onclick = async () => {
        const spec = ta.value.trim(); if (!spec || busy) return;
        busy = true; btn.disabled = true; status.textContent = t().building;
        say("📐 " + spec);
        try {
          await sse("/api/forge/run", { spec, engineer: true }, (ev) => {
            if (ev.type === "blueprint") say("📐 Blueprint: " + ev.project + " [" + ev.kind + "/" + ev.lang + "]");
            else if (ev.type === "file") say("   " + (ev.ok ? "✅" : "⚠️") + " " + ev.path + " (" + ev.bytes + "B)");
            else if (ev.type === "done") say("🏁 " + ev.project + " — faili safi " + ev.files_clean + "/" + ev.files_total);
            else if (ev.type === "error") say("⚠️ " + (ev.error || "kosa"));
          });
        } catch (e) { say("⚠️ " + e.message); }
        finally { busy = false; btn.disabled = false; status.textContent = t().ready; }
      };
    })();

    // ══════════ TAB: SDLC ══════════
    (() => {
      const p = panels.sdlc;
      const ta = el("textarea", INP + "resize:vertical;"); ta.rows = 2;
      ta.placeholder = "Eleza mradi… (mfano: tovuti ya kuuza bidhaa)";
      p.append(ta);
      const row = el("div", "display:flex;gap:10px;align-items:center;flex-wrap:wrap;");
      const aB = el("button", BTN2, t().assess), pB = el("button", BTN2, t().plan), rB = el("button", BTN, t().run);
      const status = el("span", "font-size:12px;opacity:.75;", t().ready);
      row.append(aB, pB, rB, status); p.append(row);
      const say = mkLog(p);
      const post = async (url, spec) => (await fetch(url, {
        method: "POST", headers: authHeaders({ "Content-Type": "application/json" }),
        body: JSON.stringify({ spec, lang: uiLang }),
      })).json();
      const run = async (btn, fn) => {
        const spec = ta.value.trim(); if (!spec) return;
        for (const b of [aB, pB, rB]) b.disabled = true;
        try { await fn(spec); } catch (e) { say("⚠️ " + e.message); }
        finally { for (const b of [aB, pB, rB]) b.disabled = false; btn.disabled = false; }
      };
      aB.onclick = () => run(aB, async (spec) => {
        const j = await post("/api/sdlc/assess", spec);
        if (!j.ok) return say("⚠️ " + (j.error || "kosa"));
        say("🧭 Ugumu: " + (j.complexity.label || j.complexity.level) + " (score " + j.complexity.score + ")");
        for (const ph of j.phase_plan || []) say("   ▸ " + ph.label + " · " + ph.depth);
      });
      pB.onclick = () => run(pB, async (spec) => {
        const j = await post("/api/sdlc/plan", spec);
        if (!j.ok) return say("⚠️ " + (j.error || "kosa"));
        say("📐 " + j.project + " [" + j.kind + "/" + j.lang_code + "]");
        for (const f of j.files_planned || []) say("   • " + f.path + " — " + f.purpose);
      });
      rB.onclick = () => run(rB, async (spec) => {
        const j = await post("/api/sdlc/run", spec);
        if (!j.ok) return say("⚠️ " + (j.error || "kosa"));
        for (const ph of j.phases || []) say((ph.status === "ok" ? "✅" : "⚠️") + " " + ph.label + " — " + ph.status);
        say((j.ok ? "🏁" : "⛔") + " SDLC ok=" + j.ok + " · score " + j.score);
      });
    })();

    // ══════════ TAB: MODELS ══════════
    (() => {
      const p = panels.models;
      const row = el("div", "display:flex;gap:10px;align-items:center;flex-wrap:wrap;");
      const cloudB = el("button", BTN2, "🔄 Cloud models (key)");
      const localB = el("button", BTN2, "🔄 Local models");
      const catalogB = el("button", BTN2, "📚 Catalog (zote)");
      const status = el("span", "font-size:12px;opacity:.75;", t().ready);
      row.append(cloudB, localB, catalogB, status); p.append(row);
      const list = el("div", "display:flex;flex-direction:column;gap:6px;");
      p.append(list);
      const say = mkLog(p);
      function rowFor(id, extra) {
        const r = el("div", CARD + "display:flex;justify-content:space-between;align-items:center;gap:8px;");
        r.append(el("span", "font-size:13px;font-weight:600;", id));
        if (extra) r.append(el("span", "font-size:11px;opacity:.6;", extra));
        const dl = el("button", BTN2, t().download);
        dl.onclick = async () => {
          dl.disabled = true;
          const j = await (await fetch("/api/llm/download", {
            method: "POST", headers: authHeaders({ "Content-Type": "application/json" }),
            body: JSON.stringify({ id }),
          })).json();
          say((j.ok ? "✅" : "⚠️") + " download " + id + " → " + (j.error || j.state || "imeanza"));
          dl.disabled = false;
        };
        const use = el("button", BTN, t().use);
        use.onclick = async () => {
          use.disabled = true;
          const j = await (await fetch("/api/llm/use", {
            method: "POST", headers: authHeaders({ "Content-Type": "application/json" }),
            body: JSON.stringify({ id }),
          })).json();
          say((j.ok ? "✅" : "⚠️") + " use " + id + " → " + (j.error || "ok"));
          use.disabled = false;
        };
        const acts = el("div", "display:flex;gap:6px;"); acts.append(dl, use);
        r.append(acts);
        return r;
      }
      cloudB.onclick = async () => {
        status.textContent = "…"; list.textContent = "";
        const j = await (await fetch("/api/ai/cloud-models", { headers: authHeaders() })).json();
        status.textContent = t().ready;
        if (j.ok === false) return say("⚠️ " + (j.error || "kosa"));
        const ms = j.models || [];
        say("☁️ Cloud models: " + ms.length);
        for (const m of ms) list.append(rowFor(typeof m === "string" ? m : m.id || m.name, "cloud"));
      };
      localB.onclick = async () => {
        status.textContent = "…"; list.textContent = "";
        const j = await (await fetch("/api/llm/models", { headers: authHeaders() })).json();
        status.textContent = t().ready;
        const ms = (j.models || []).filter((m) => typeof m === "string" || (m && m.id));
        say("💾 Local models: " + ms.length);
        for (const m of ms) list.append(rowFor(typeof m === "string" ? m : m.id, "local"));
      };
      catalogB.onclick = async () => {
        status.textContent = "…"; list.textContent = "";
        try {
          const j = await (await fetch("/api/ai/catalog", { headers: authHeaders() })).json();
          status.textContent = t().ready;
          const ms = (j.catalog && j.catalog.models) || [];
          say("📚 Catalog: " + ms.length + " (bure " + ms.filter((m) => m.free).length + ")");
          for (const m of ms) {
            list.append(rowFor(m.id, (m.free ? "bure" : "PRO") + " · " + (m.params || "")));
          }
        } catch (e) { status.textContent = t().ready; say("⚠️ " + e.message); }
      };
      cloudB.onclick();
    })();

    // ══════════ TAB: PROVIDERS ══════════
    (() => {
      const p = panels.providers;
      const PROVIDERS = [
        ["fundi_engine", "MTECHOS Injini (GGUF)", "#7c3aed", "ME"],
        ["ollama", "Ollama", "#ffffff", "🦙"],
        ["openai", "OpenAI", "#10a37f", "AI"],
        ["anthropic", "Anthropic", "#d4a27f", "A"],
        ["google", "Google Gemini", "#4285f4", "G"],
        ["groq", "Groq", "#f55036", "G"],
        ["mistral", "Mistral AI", "#ff7000", "M"],
        ["huggingface", "Hugging Face", "#ffd21e", "🤗"],
        ["openrouter", "OpenRouter", "#6467f2", "OR"],
        ["deepseek", "DeepSeek", "#4d6bfe", "DS"],
      ];
      const grid = el("div", "display:grid;grid-template-columns:repeat(auto-fill,minmax(150px,1fr));gap:8px;");
      p.append(grid);
      const keyRow = el("div", "display:flex;gap:10px;align-items:center;flex-wrap:wrap;");
      const keyIn = el("input", INP + "max-width:360px;"); keyIn.type = "password";
      keyIn.placeholder = "Ollama Cloud API key (mfano: <hex>.<base64url>)";
      const saveB = el("button", BTN, t().saveKey);
      const testB = el("button", BTN2, t().test);
      keyRow.append(keyIn, saveB, testB); p.append(keyRow);
      const say = mkLog(p);

      for (const [id, label, color, glyph] of PROVIDERS) {
        const card = el("div", CARD + "display:flex;align-items:center;gap:10px;");
        const badge = el("span",
          "width:30px;height:30px;border-radius:8px;display:flex;align-items:center;justify-content:center;" +
          "font-weight:700;font-size:12px;flex:none;background:" + color + ";color:#0b0f14;", glyph);
        card.append(badge);
        const meta = el("div", "display:flex;flex-direction:column;");
        meta.append(el("span", "font-size:12px;font-weight:600;", label));
        meta.append(el("span", "font-size:10px;opacity:.55;", id));
        card.append(meta);
        card.onclick = () => { keyIn.focus(); };
        grid.append(card);
      }
      saveB.onclick = async () => {
        const key = keyIn.value.trim(); if (!key) return;
        saveB.disabled = true;
        const j = await (await fetch("/api/ai/key", {
          method: "POST", headers: authHeaders({ "Content-Type": "application/json" }),
          body: JSON.stringify({ key }),
        })).json();
        say((j.ok ? "✅" : "⚠️") + " " + t().saved + ": " + (j.error || "key imehifadhiwa; mfumo umejipanga"));
        saveB.disabled = false;
      };
      testB.onclick = async () => {
        testB.disabled = true;
        const j = await (await fetch("/api/ai/test", {
          method: "POST", headers: authHeaders({ "Content-Type": "application/json" }),
          body: JSON.stringify({}),
        })).json();
        say((j.ok ? "✅" : "⚠️") + " verify: " + (j.reply || j.error || JSON.stringify(j)) + (j.ms ? " (" + j.ms + "ms)" : ""));
        testB.disabled = false;
      };
    })();

    // ══════════ TAB: INJINI (fundi-engine — injini YETU ya inference) ══════════
    // Injini yetu ya inference: GGUF moja kwa moja, pure Rust, bila Python/CUDA/vLLM.
    // UI hii inaonyesha hali HALISI: modeli zilizopo, iliyopakiwa, na stream live
    // yenye SPEEDOMETER (tok/s) — hakuna namba za bandia.
    (() => {
      const p = panels.engine;
      const info = el("div", CARD + "font-size:12px;line-height:1.7;");
      info.append(el("div", "font-weight:700;font-size:13px;margin-bottom:4px;", "⚙️ Injini ya MTECHOS — inference ya ndani (GGUF, pure Rust)"));
      info.append(el("div", "opacity:.85;",
        "Usanifu: Monaco (UI) ➤ Rust Axum (orchestrator — hii) ➤ fundi-engine (AI core yetu). " +
        "Inapakia faili za GGUF moja kwa moja — hakuna Python, hakuna CUDA, hakuna vLLM ya nje."));
      info.append(el("div", "opacity:.7;margin-top:6px;",
        "Weka modeli yako (.gguf) kwenye FUNDI_MODELS_DIR au /data/models, kisha pakia hapa. " +
        "Inaunga mkono Llama / Qwen2 / Mistral na quantization Q4_K/Q6_K/Q8_0 n.k."));
      p.append(info);
      const bar = el("div", "display:flex;gap:8px;flex-wrap:wrap;align-items:center;");
      const statusB = el("button", BTN, "🔄 Hali (status)");
      const modelsB = el("button", BTN2, "📋 Modeli");
      const loadB = el("button", BTN2, "⬇️ Pakia modeli");
      const probeB = el("button", BTN2, "🧪 Jaribu stream");
      const st = el("span", "font-size:12px;opacity:.75;", t().ready);
      bar.append(statusB, modelsB, loadB, probeB, st); p.append(bar);
      const cards = el("div", "display:grid;grid-template-columns:repeat(auto-fill,minmax(220px,1fr));gap:8px;");
      p.append(cards);
      const ta = el("textarea", INP + "resize:vertical;"); ta.rows = 2;
      ta.placeholder = "Prompt ya kujaribu stream (mfano: andika fn ya Rust ya Fibonacci)…";
      p.append(ta);
      const say = mkLog(p);
      function card(title, sub) {
        const c = el("div", CARD);
        c.append(el("div", "font-size:12px;font-weight:600;", title));
        c.append(el("div", "font-size:11px;opacity:.7;margin-top:4px;white-space:pre-wrap;", sub || ""));
        return c;
      }
      statusB.onclick = async () => {
        cards.textContent = "";
        const j = await (await fetch("/api/engine/status", { headers: authHeaders() })).json();
        cards.append(card("Injini", j.engine || "fundi-engine"));
        cards.append(card("Iliyopakiwa", j.loaded ? (j.loaded.path + "\n" + j.loaded.arch + " · layers " + j.loaded.layers + " · vocab " + j.loaded.vocab + " · ctx " + j.loaded.ctx) : "— hakuna"));
        cards.append(card("Saraka za modeli", (j.model_dirs || []).join("\n") || "—"));
        if (j.loading) say("⏳ inapakia: " + j.loading);
        say("ℹ️ " + (j.note_sw || "Injini YETU ya inference."));
      };
      modelsB.onclick = async () => {
        const j = await (await fetch("/api/engine/models", { headers: authHeaders() })).json();
        say(null);
        if (!j.ok || !j.count) return say("⛔ hakuna modeli za GGUF. Weka .gguf kwenye: " + (j.dirs || []).join(", "));
        say("📋 Modeli za GGUF (" + j.count + "):");
        for (const m of j.models || []) {
          const i = m.info || {};
          say("   • " + m.name + "  [" + m.size_mb + "MB" + (i.arch ? " · " + i.arch + " · " + i.layers + "L" : "") + "]");
        }
      };
      loadB.onclick = async () => {
        const j0 = await (await fetch("/api/engine/models", { headers: authHeaders() })).json();
        if (!j0.ok || !j0.count) return say("⛔ hakuna modeli. Weka .gguf kwenye " + (j0.dirs || []).join(", "));
        const names = (j0.models || []).map((m) => m.name);
        const pick = window.prompt("Chagua modeli ya kupakia:\n" + names.join("\n"), names[0]);
        if (!pick) return;
        loadB.disabled = true; st.textContent = "inapakia…";
        const j = await (await fetch("/api/engine/load", {
          method: "POST", headers: authHeaders({ "Content-Type": "application/json" }),
          body: JSON.stringify({ model: pick, ctx: 4096 }),
        })).json();
        if (j.ok) say("✅ imepakiwa: " + (j.loaded ? j.loaded.path : pick));
        else say("⚠️ " + (j.error || "kosa"));
        loadB.disabled = false; st.textContent = t().ready;
      };
      probeB.onclick = async () => {
        const prompt = ta.value.trim() || "andika function fupi ya Rust";
        probeB.disabled = true; st.textContent = t().thinking;
        say(null); say("🧑 " + prompt);
        try {
          await sse("/api/engine/stream", { prompt, max_tokens: 256 }, (ev) => {
            if (ev.type === "start") say("▶️ injini " + ev.model + " · ctx " + ev.ctx + " · " + (ev.base || ""));
            else if (ev.type === "tok") say("   " + ev.tok + "  [" + ev.tps + " tok/s · " + ev.total + " tok]");
            else if (ev.type === "done") say("🏁 " + (ev.ok ? "ok" : "kosa") + " · " + ev.total_tokens + " tok · " + ev.elapsed_ms + "ms · wastani " + ev.tps_avg + " tok/s");
            else if (ev.type === "error") say("⚠️ " + ev.error);
          });
        } catch (e) { say("⚠️ " + e.message); }
        finally { probeB.disabled = false; st.textContent = t().ready; }
      };
      statusB.onclick();
    })();

    // ══════════ TAB: UBONGO (agentic bridge — Rust ➤ Go ➤ Elixir) ══════════
    // Uso huu huunganisha UI MOJA KWA MOJA na API za lugha tatu:
    //   • Rust (orchestrator)  — /api/native/agentic (hali ya sidecars)
    //   • Go (agentic/ptyexec) — /api/native/exec (mikono: run commands)
    //   • Elixir (agentic/brain)— /api/native/think (ubongo: maamuzi/HITL)
    // Kila thamani inatoka API halisi — hakuna mock.
    (() => {
      const p = panels.agentic;
      const info = el("div", CARD + "font-size:12px;line-height:1.7;");
      info.append(el("div", "font-weight:700;font-size:13px;margin-bottom:4px;", "🧠 Ubongo wa MTECHOS — Rust ➤ Go ➤ Elixir"));
      info.append(el("div", "opacity:.85;",
        "Rust (orchestrator) huongoza: Go (mikono — utekelezaji wa amri kwa concurrency) " +
        "na Elixir/OTP (ubongo — tabaka 12 za kufikiri, maadili + HITL). Hakuna Python."));
      p.append(info);
      const bar = el("div", "display:flex;gap:8px;flex-wrap:wrap;align-items:center;");
      const statusB = el("button", BTN, "🔎 Hali ya sidecars");
      const execB = el("button", BTN2, "🏃 Endesha amri (Go)");
      const thinkB = el("button", BTN2, "🧠 Fikiri (Elixir)");
      const st = el("span", "font-size:12px;opacity:.75;", t().ready);
      bar.append(statusB, execB, thinkB, st); p.append(bar);
      const cards = el("div", "display:grid;grid-template-columns:repeat(auto-fill,minmax(220px,1fr));gap:8px;");
      p.append(cards);
      const ta = el("textarea", INP + "resize:vertical;"); ta.rows = 2;
      ta.placeholder = "Amri ya kuendesha (Go) au swali la kufikiri (Elixir)… mfano: ls -la";
      p.append(ta);
      const say = mkLog(p);
      function card(title, sub) {
        const c = el("div", CARD);
        c.append(el("div", "font-size:12px;font-weight:600;", title));
        c.append(el("div", "font-size:11px;opacity:.7;margin-top:4px;white-space:pre-wrap;", sub || ""));
        return c;
      }
      statusB.onclick = async () => {
        cards.textContent = "";
        const j = await (await fetch("/api/native/agentic", { headers: authHeaders() })).json();
        const g = (j.components && j.components.go_ptyexec) || {};
        const e = (j.components && j.components.elixir_brain) || {};
        cards.append(card("Runtime", j.runtime || "—"));
        cards.append(card("Go · mikono (ptyexec)", (g.present ? "✅ ipo" : "⛔ haipo") + "\n" + (g.binary || "")));
        cards.append(card("Elixir · ubongo (brain)", (e.present ? "✅ ipo" : "⛔ haipo") + "\n" + (e.binary || "")));
        cards.append(card("Protocol", j.protocol_version || "—"));
        say("ℹ️ Rust orchestrator ameunganisha Go + Elixir kwa subprocess JSON.");
      };
      execB.onclick = async () => {
        const line = ta.value.trim() || "ls -la";
        const parts = line.split(/\s+/);
        const bin = parts.shift();
        say(null); say("🏃 Go inaendesha: " + line);
        const j = await (await fetch("/api/native/exec", {
          method: "POST", headers: authHeaders({ "Content-Type": "application/json" }),
          body: JSON.stringify({ cmds: [{ id: "ui", bin, args: parts, timeout_ms: 15000 }], concurrency: 1 }),
        })).json();
        const r = (j.results && j.results[0]) || {};
        say("exit " + (r.exit_code ?? "?") + " · " + Math.round((r.duration || 0) / 1e6) + "ms");
        if (r.stdout) say(r.stdout);
        if (r.stderr) say("stderr: " + r.stderr);
        if (!j.ok) say("⚠️ " + (j.error || "kosa"));
      };
      thinkB.onclick = async () => {
        const input = ta.value.trim() || "Nikupe deploy ya production?";
        say(null); say("🧠 Elixir inafikiri: " + input);
        const j = await (await fetch("/api/native/think", {
          method: "POST", headers: authHeaders({ "Content-Type": "application/json" }),
          body: JSON.stringify({ input, hitl_for_deploy: true }),
        })).json();
        if (!j.ok) return say("⚠️ " + (j.error || "kosa"));
        const d = j.decision || {}, e = j.emotion || {}, it = j.intuition || {};
        say("maamuzi: " + d.action + " · uhakika " + d.confidence + " · HITL " + d.requires_hitl + " · mfumo " + d.used_system);
        say("hisia: " + e.kind + " (" + e.mood + ") · angavu: " + it.signal + " — " + it.explanation);
        say("tabaka: " + ((d.layers_activated || []).join(", ") || "—"));
      };
      statusB.onclick();
    })();

    // ══════════ TAB: MCP ══════════
    (() => {
      const p = panels.mcp;
      const bar = el("div", "display:flex;gap:6px;flex-wrap:wrap;align-items:center;");
      const addB = el("button", BTN, "➕ Server");
      const healthB = el("button", BTN2, "🩺 Pima (health)");
      const status = el("span", "font-size:12px;opacity:.75;", t().ready);
      bar.append(addB, healthB, status); p.append(bar);
      const list = el("div", "display:flex;flex-direction:column;gap:6px;");
      const say = mkLog(p);
      async function load() {
        list.textContent = "";
        const j = await (await fetch("/api/mcp/servers", { headers: authHeaders() })).json();
        status.textContent = t().ready;
        for (const s of j.servers || []) {
          const card = el("div", CARD + "display:flex;justify-content:space-between;align-items:center;gap:8px;");
          const meta = el("div", "display:flex;flex-direction:column;");
          meta.append(el("span", "font-size:13px;font-weight:600;", s.name + " · " + s.transport));
          meta.append(el("span", "font-size:11px;opacity:.6;", s.url || s.command || ""));
          card.append(meta);
          const del = el("button", BTN2, "✕");
          del.onclick = async () => { await fetch("/api/mcp/servers/" + s.id, { method: "DELETE", headers: authHeaders() }); await load(); };
          card.append(del); list.append(card);
        }
        if (!(j.servers || []).length) list.append(el("div", "opacity:.6;", "Hakuna MCP servers bado."));
      }
      addB.onclick = async () => {
        const name = prompt("Jina la MCP server:"); if (!name) return;
        const transport = (prompt("Transport (stdio/http/sse):", "stdio") || "stdio").toLowerCase();
        let body = { name, transport };
        if (transport === "stdio") { const command = prompt("Command (mfano: npx -y @modelcontextprotocol/server-filesystem /tmp):"); if (!command) return; body.command = command; }
        else { const url = prompt("URL:"); if (!url) return; body.url = url; }
        const j = await (await fetch("/api/mcp/servers", { method: "POST", headers: authHeaders({ "Content-Type": "application/json" }), body: JSON.stringify(body) })).json();
        say((j.ok ? "✅" : "⚠️") + " " + (j.error || "imehifadhiwa")); await load();
      };
      healthB.onclick = async () => {
        healthB.disabled = true;
        const j = await (await fetch("/api/mcp/health", { headers: authHeaders() })).json();
        for (const pr of j.probes || []) say((pr.ok ? "✅" : "⛔") + " " + pr.name + " (" + pr.transport + "): " + pr.detail);
        if (!(j.probes || []).length) say("Hakuna servers wa kupima.");
        healthB.disabled = false;
      };
      load();
    })();

    // ══════════ TAB: ACP ══════════
    (() => {
      const p = panels.acp;
      const bar = el("div", "display:flex;gap:6px;align-items:center;");
      const scanB = el("button", BTN2, "🔄 Pima agents (PATH)");
      const status = el("span", "font-size:12px;opacity:.75;", t().ready);
      bar.append(scanB, status); p.append(bar);
      const list = el("div", "display:flex;flex-direction:column;gap:6px;");
      p.append(list);
      async function load() {
        list.textContent = "";
        const j = await (await fetch("/api/acp/agents", { headers: authHeaders() })).json();
        status.textContent = (j.available || 0) + " / " + (j.agents || []).length + " zipo";
        for (const a of j.agents || []) {
          const card = el("div", CARD + "display:flex;justify-content:space-between;align-items:center;gap:8px;");
          const meta = el("div", "display:flex;flex-direction:column;");
          meta.append(el("span", "font-size:13px;font-weight:600;", (a.available ? "🟢 " : "⚪ ") + a.label));
          meta.append(el("span", "font-size:11px;opacity:.6;", a.binary + (a.path ? " · " + a.path : " · haipo kwenye PATH")));
          card.append(meta); list.append(card);
        }
      }
      scanB.onclick = load; load();
    })();

    // ══════════ TAB: PLUGINS ══════════
    (() => {
      const p = panels.plugins;
      const bar = el("div", "display:flex;gap:6px;align-items:center;");
      const reloadB = el("button", BTN2, t().refresh);
      const status = el("span", "font-size:12px;opacity:.75;", t().ready);
      bar.append(reloadB, status); p.append(bar);
      const list = el("div", "display:flex;flex-direction:column;gap:6px;");
      p.append(list);
      async function load() {
        list.textContent = "";
        const j = await (await fetch("/api/plugins/installed", { headers: authHeaders() })).json();
        status.textContent = (j.count || 0) + " plugins";
        for (const pl of j.plugins || []) {
          const card = el("div", CARD);
          card.append(el("div", "font-size:13px;font-weight:600;", "🧱 " + (pl.name || "?") + " v" + (pl.version || "?")));
          card.append(el("div", "font-size:11px;opacity:.7;margin-top:4px;", pl.description || ""));
          card.append(el("div", "font-size:11px;opacity:.6;margin-top:4px;", "pages: " + (pl.pages || []).map((x) => x.nav_label || x.id).join(", ")));
          list.append(card);
        }
      }
      reloadB.onclick = load; load();
    })();

    // ══════════ TAB: CONNECTORS ══════════
    (() => {
      const p = panels.connectors;
      const bar = el("div", "display:flex;gap:6px;align-items:center;");
      const reloadB = el("button", BTN2, t().refresh);
      const status = el("span", "font-size:12px;opacity:.75;", t().ready);
      bar.append(reloadB, status); p.append(bar);
      const list = el("div", "display:grid;grid-template-columns:repeat(auto-fill,minmax(220px,1fr));gap:8px;");
      p.append(list);
      async function load() {
        list.textContent = "";
        const j = await (await fetch("/api/connectors/status", { headers: authHeaders() })).json();
        for (const c of j.connectors || []) {
          const card = el("div", CARD);
          card.append(el("div", "font-size:13px;font-weight:600;", (c.connected ? "🟢 " : "⚪ ") + c.label));
          card.append(el("div", "font-size:11px;opacity:.7;margin-top:4px;", c.detail || ""));
          list.append(card);
        }
        status.textContent = t().ready;
      }
      reloadB.onclick = load; load();
    })();

    // ══════════ TAB: KUMBUKUMBU (memory) ══════════
    (() => {
      const p = panels.memory;
      const bar = el("div", "display:flex;gap:6px;align-items:center;");
      const reloadB = el("button", BTN2, t().refresh);
      const status = el("span", "font-size:12px;opacity:.75;", t().ready);
      bar.append(reloadB, status); p.append(bar);
      const list = el("div", "display:flex;flex-direction:column;gap:6px;");
      p.append(list);
      const say = mkLog(p);
      async function load() {
        list.textContent = "";
        const j = await (await fetch("/api/ide/conversations", { headers: authHeaders() })).json();
        status.textContent = (j.conversations || []).length + " mazungumzo";
        for (const c of j.conversations || []) {
          const card = el("div", CARD + "display:flex;justify-content:space-between;align-items:center;gap:8px;cursor:pointer;");
          card.append(el("span", "font-size:12px;", "🧠 " + c.title + " (" + c.lang + ")"));
          const view = el("button", BTN2, t().open);
          view.onclick = async () => {
            const d = await (await fetch("/api/ide/conversations/" + c.id, { headers: authHeaders() })).json();
            say(null);
            for (const m of d.messages || []) say((m.role === "user" ? "🧑 " : "🤖 ") + m.content);
          };
          card.append(view); list.append(card);
        }
      }
      reloadB.onclick = load; load();
    })();

    // ══════════ TAB: EMULATOR ══════════
    (() => {
      const p = panels.emulator;
      const row = el("div", "display:flex;gap:10px;align-items:center;flex-wrap:wrap;");
      const sel = el("select",
        "padding:8px 10px;border-radius:8px;background:rgba(255,255,255,.05);color:inherit;" +
        "border:1px solid var(--oh-border-variant,#2a2f3a);font-size:12px;");
      for (const k of ["website", "mobile", "iot", "gaming"]) {
        const o = el("option", null, k); o.value = k; sel.append(o);
      }
      const btn = el("button", BTN, t().emulate);
      const sessB = el("button", BTN2, t().sessions);
      const status = el("span", "font-size:12px;opacity:.75;", t().ready);
      row.append(sel, btn, sessB, status); p.append(row);
      const say = mkLog(p);
      btn.onclick = async () => {
        btn.disabled = true; status.textContent = "…";
        try {
          const j = await (await fetch("/api/ide/emulate/" + sel.value, {
            method: "POST", headers: authHeaders({ "Content-Type": "application/json" }),
            body: JSON.stringify({ project: true }),
          })).json();
          say((j.ok ? "✅" : "⚠️") + " " + sel.value + ": " + JSON.stringify(j).slice(0, 800));
        } catch (e) { say("⚠️ " + e.message); }
        finally { btn.disabled = false; status.textContent = t().ready; }
      };
      sessB.onclick = async () => {
        const j = await (await fetch("/api/ide/emulate/sessions", { headers: authHeaders() })).json();
        say("📋 " + JSON.stringify(j).slice(0, 800));
      };
    })();

    langSel.onchange = () => {
      uiLang = langSel.value;
      localStorage.setItem("mtechos_lang", uiLang);
      for (const name of tabNames) tabBtns[name].textContent = t().tabs[name];
    };

    // ══════════ TAB: AI (models + providers — karibu na Settings) ══════════
    (() => {
      const p = panels.ai;
      const bar = el("div", "display:flex;gap:6px;flex-wrap:wrap;align-items:center;");
      const reloadB = el("button", BTN2, t().refresh);
      const catalogB = el("button", BTN2, "📚 Catalog");
      const status = el("span", "font-size:12px;opacity:.75;", t().ready);
      bar.append(reloadB, catalogB, status); p.append(bar);
      const list = el("div", "display:grid;grid-template-columns:repeat(auto-fill,minmax(240px,1fr));gap:8px;");
      p.append(list);
      const say = mkLog(p);
      function card(title, sub, extra) {
        const c = el("div", CARD);
        c.append(el("div", "font-size:13px;font-weight:600;", title));
        c.append(el("div", "font-size:11px;opacity:.65;margin-top:4px;", sub || ""));
        if (extra) c.append(el("div", "font-size:11px;opacity:.55;margin-top:4px;", extra));
        return c;
      }
      async function loadModels() {
        list.textContent = "";
        const j = await (await fetch("/api/llm/models", { headers: authHeaders() })).json();
        const ms = (j.models || []).filter((m) => typeof m === "string" || (m && m.id));
        status.textContent = ms.length + " models";
        for (const m of ms) {
          const id = typeof m === "string" ? m : m.id;
          const c = card("🧩 " + id, "local (Ollama)");
          const useB = el("button", BTN, t().use); useB.style.marginTop = "8px";
          useB.onclick = async () => { const r = await (await fetch("/api/llm/use", { method: "POST", headers: authHeaders({ "Content-Type": "application/json" }), body: JSON.stringify({ id }) })).json(); say((r.ok ? "✅" : "⚠️") + " use " + id); };
          c.append(useB); list.append(c);
        }
        if (!ms.length) list.append(card("Hakuna models za ndani", "Pakua au tumia Ollama Cloud"));
      }
      async function loadProviders() {
        const j = await (await fetch("/api/llm/providers", { headers: authHeaders() })).json();
        const provs = j.providers || [];
        say(null);
        for (const pr of provs) say("🔑 " + (typeof pr === "string" ? pr : (pr.id || pr.name || JSON.stringify(pr))) + " · inapatikana");
        if (!provs.length) say("Hakuna providers waliopangwa (Ollama ni ya ndani).");
      }
      reloadB.onclick = () => { loadModels(); loadProviders(); };
      catalogB.onclick = async () => {
        const j = await (await fetch("/api/ai/catalog", { headers: authHeaders() })).json();
        const ms = (j.catalog && j.catalog.models) || [];
        say(null); say("📚 Catalog: " + ms.length + " (bure " + ms.filter((m) => m.free).length + ")");
        for (const m of ms.slice(0, 60)) say("   • " + m.id + (m.free ? " (bure)" : " (PRO)") + " " + (m.params || ""));
      };
      loadModels(); loadProviders();
    })();

    showTab("editor");
    return () => shell.remove();
  });
  // ═══ UKURASA 2: OLLAMA BOOSTER (models + connection ya MBILINYI) ═══
  host.registerPage("ollama", async ({ container }) => {
    const shell = document.createElement("section");
    shell.style.cssText =
      "min-height:100%;padding:2rem;color:var(--oh-text-primary,#fff);" +
      "background:var(--oh-color-base,#111);font-family:system-ui,sans-serif;";
    container.append(shell);

    const title = document.createElement("h2");
    title.textContent = "MBILINYI IDE";
    title.style.cssText = "font-size:1.2rem;margin:0 0 .75rem;font-weight:600;";
    shell.append(title);

    const out = document.createElement("pre");
    out.style.cssText =
      "white-space:pre-wrap;font-size:12px;line-height:1.6;opacity:.85;" +
      "background:rgba(255,255,255,.04);border-radius:8px;padding:12px;max-height:60vh;overflow:auto;";
    out.textContent = "Inatafuta Ollama + models za MTECHOS…";
    shell.append(out);

    const btn = document.createElement("button");
    btn.textContent = "🔌 Kuingiza connection ya MBILINYI (base_url: Ollama)";
    btn.style.cssText =
      "margin-top:.75rem;padding:10px 16px;border-radius:8px;border:0;cursor:pointer;" +
      "background:var(--oh-brand,#5865f2);color:#fff;font-weight:600;font-size:13px;";
    btn.disabled = true;
    shell.append(btn);

    let conn = null;
    try {
      const j = await (await fetch("/api/engines/canvas/ollama-provider", {
        headers: authHeaders(),
      })).json();
      conn = j.canvas_connection;
      out.textContent =
        "✅ Provider: " + (j.provider || "ollama") + "\n\n" +
        "base_url: " + ((conn && conn.base_url) || "—") + "\n\n" +
        "Models zinazopendekezwa:\n" +
        (j.recommended_models || []).map((m) => "  • " + m).join("\n") + "\n\n" +
        "Models halisi za MTECHOS (llm_setup):\n" +
        (Array.isArray(j.mtechos_models) ? j.mtechos_models : [])
          .map((m) => "  • " + (m.id || m) + (m.present ? " ✓" : " (westa mtazamaji)"))
          .join("\n") +
        "\n\n" + (j.hint || "");
      btn.disabled = false;
    } catch (e) {
      out.textContent = "⚠️ " + e.message;
    }

    btn.onclick = async () => {
      if (!conn) return;
      btn.disabled = true;
      btn.textContent = "Inaingiza…";
      try {
        const r = await fetch("/api/llm/provider-connections", {
          method: "POST",
          headers: authHeaders({ "Content-Type": "application/json" }),
          body: JSON.stringify({
            display_name: conn.display_name,
            provider: conn.provider,
            api_key: conn.api_key || "ollama",
            base_url: conn.base_url,
          }),
        });
        const jr = await r.json();
        out.textContent =
          (r.ok ? "✅" : "⚠️") + " Connection response:\n\n" +
          JSON.stringify(jr, null, 2);
        btn.textContent = "✅ Imeingizwa — fungua Settings > LLM kuchagua model";
      } catch (e) {
        out.textContent = "⚠️ " + e.message;
        btn.textContent = "Retry — kuingiza connection";
        btn.disabled = false;
      }
    };

    return () => shell.remove();
  });

  // ═══ UKURASA 3: SUPER ENGINEER — agenti wa kweli (ReAct loop, lugha 22) ═══
  host.registerPage("super", async ({ container }) => {
    const el = (tag, css, text) => {
      const n = document.createElement(tag);
      if (css) n.style.cssText = css;
      if (text != null) n.textContent = text;
      return n;
    };
    const shell = el("section",
      "min-height:100%;padding:1.5rem;color:var(--oh-text-primary,#fff);" +
      "background:var(--oh-color-base,#0b0f14);font-family:system-ui,sans-serif;" +
      "display:flex;flex-direction:column;gap:12px;");
    container.append(shell);

    shell.append(el("h2", "font-size:1.15rem;margin:0;font-weight:600;",
      "SUPER ENGINEER — agenti anayejenga kwa kweli"));
    shell.append(el("p", "margin:0;font-size:12px;opacity:.7;line-height:1.6;",
      "Eleza mradi kwa Kiswahili. Agenti huandika faili, HUENDESHA (build/tests), " +
      "hujirekebisha, na kuthibitisha tests HALISI zipite. Lugha 22 (Rust, Node, " +
      "Python, Dart, Go, Java, C#, C/C++…). Uthibitisho wa uongo hauruhusiwi."));

    // Uwezo halisi kutoka backend.
    const caps = el("div", "font-size:11px;opacity:.8;line-height:1.7;" +
      "background:rgba(255,255,255,.04);border-radius:8px;padding:10px;",
      "Inasoma uwezo wa engines…");
    shell.append(caps);

    const ta = el("textarea",
      "width:100%;resize:vertical;padding:10px 12px;border-radius:8px;font-size:13px;" +
      "background:rgba(255,255,255,.05);color:inherit;border:1px solid var(--oh-border-variant,#2a2f3a);");
    ta.rows = 2;
    ta.placeholder = "Mfano: REST API ya TODO yenye tests, au parser ya Rust…";
    shell.append(ta);

    const row = el("div", "display:flex;gap:10px;align-items:center;flex-wrap:wrap;");
    shell.append(row);
    const btn = el("button",
      "padding:10px 18px;border-radius:8px;border:0;cursor:pointer;font-weight:600;font-size:13px;" +
      "background:var(--oh-brand,#5865f2);color:#fff;", "🚀 Jenga mradi");
    row.append(btn);
    const runBtn = el("button",
      "padding:10px 16px;border-radius:8px;border:1px solid var(--oh-border-variant,#2a2f3a);cursor:pointer;" +
      "font-weight:600;font-size:13px;background:transparent;color:inherit;", "▶ Run mradi");
    runBtn.disabled = true;
    row.append(runBtn);
    const kindIn = el("input",
      "padding:8px 10px;border-radius:8px;font-size:12px;width:130px;" +
      "background:rgba(255,255,255,.05);color:inherit;border:1px solid var(--oh-border-variant,#2a2f3a);");
    kindIn.placeholder = "kind (mf. rust-cli)";
    row.append(kindIn);
    const status = el("span", "font-size:12px;opacity:.75;", "Tayari");
    row.append(status);

    const log = el("pre",
      "flex:1;min-height:180px;max-height:55vh;overflow:auto;white-space:pre-wrap;" +
      "font-size:12px;line-height:1.6;background:rgba(255,255,255,.04);" +
      "border-radius:8px;padding:12px;margin:0;", "Tayari.");
    shell.append(log);

    const say = (s) => {
      log.textContent += (log.textContent ? "\n" : "") + s;
      log.scrollTop = log.scrollHeight;
    };

    // Pata uwezo halisi (lugha, skills, engines).
    try {
      const cj = await (await fetch("/api/engines/unified/capabilities", {
        headers: authHeaders(),
      })).json();
      const langs = ((cj.capabilities || []).find((c) => c.id === "code_generation") || {}).languages || [];
      const names = (cj.capabilities || []).filter((c) => c.available).map((c) => c.id);
      caps.textContent = "Lugha (" + langs.length + "): " + langs.join(", ") +
        "\nUwezo unaopatikana: " + names.join(", ");
    } catch (e) {
      caps.textContent = "⚠️ Uwezo: " + e.message;
    }

    let running = false;
    let lastProject = "";
    let gotResult = false;
    let pollTimer = 0;
    const t0Ref = { t: 0 };

    // Onyesha matokeo ya mwisho (kutoka SSE au kupoll).
    function applyResult(j) {
      if (!j || !j.ok) return false;
      gotResult = true;
      lastProject = j.project || lastProject;
      runBtn.disabled = false;
      say("");
      say("📁 Mradi: " + j.project + "  (" + j.workspace + ")");
      say("⏱️  Hatua: " + j.steps_count + " · faili: " + (j.files_written || []).length);
      say("📝 Faili: " + (j.files_written || []).join(", "));
      const v = j.verify || {};
      say((v.ok ? "✅" : "⛔") + " VERIFY: " + v.ok + "  (" + (v.command || "") + ")");
      if (v.output) say("--- uthibitisho ---\n" + String(v.output).slice(-900));
      status.textContent = "Tayari";
      return true;
    }

    // Fallback: poll matokeo (proxy inaweza kuvunja SSE stream).
    function startPoll() {
      if (pollTimer) clearInterval(pollTimer);
      const deadline = Date.now() + 6 * 60 * 1000;
      pollTimer = setInterval(async () => {
        if (gotResult || Date.now() > deadline) { clearInterval(pollTimer); pollTimer = 0; return; }
        try {
          const j = await (await fetch("/api/ide/agent/agentic/result")).json();
          if (j && j.ok && !j.pending) { applyResult(j); clearInterval(pollTimer); pollTimer = 0; }
        } catch { /* endelea kupoll */ }
      }, 2000);
    }

    // SSE: hatua moja-moja LIVE (start/write/run/verify/fix_round/done/result).
    function handle(ev) {
      switch (ev.type) {
        case "start":
          lastProject = ev.project;
          say("📁 Mradi: " + ev.project + "  (" + ev.workspace + ")");
          say("🧠 Model: " + ev.model + " · hatua max: " + ev.max_steps);
          break;
        case "write":
          say("  " + (ev.status === "ok" ? "✅" : "⚠️") + " write " + ev.path +
            " (" + ev.bytes + "B" + (ev.dup ? ", dup" : "") + ")");
          break;
        case "mkdir":
          say("  📂 mkdir " + ev.path + " → " + ev.status);
          break;
        case "run":
          say("  ▶ run `" + ev.command + "` → " + ev.status);
          if (ev.output) say("     " + String(ev.output).split("\n").slice(0, 6).join("\n     "));
          break;
        case "verify":
          if (ev.ok === null) { say("🔎 verify: " + (ev.final ? "mwisho" : "") + " `" + ev.command + "`…"); break; }
          say((ev.ok ? "✅" : "⛔") + " VERIFY: " + ev.ok + "  (" + ev.command + ")");
          if (ev.output) say(String(ev.output).slice(-900));
          break;
        case "fix_round":
          say("🔧 Mzunguko wa kujirekebisha " + ev.round + "/" + ev.max + "…");
          break;
        case "max_steps":
          say("⚠️ Umefikia kikomo cha hatua (" + ev.max + ") — uthibitisho wa mwisho unafanyika…");
          break;
        case "done":
          say((ev.verify_ok ? "🏁" : "⚠️") + " DONE (verify=" + ev.verify_ok + "): " + ev.summary);
          break;
        case "result":
          if (ev.result) applyResult(ev.result);
          runBtn.disabled = false;
          break;
        case "error":
          say("⚠️ " + ev.error);
          break;
      }
    }

    btn.onclick = async () => {
      const idea = ta.value.trim();
      if (!idea || running) return;
      running = true;
      btn.disabled = true;
      runBtn.disabled = true;
      log.textContent = "";
      gotResult = false;
      status.textContent = "Inajenga (live)…";
      t0Ref.t = performance.now();
      startPoll();
      try {
        const res = await fetch("/api/ide/agent/agentic/stream", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ idea, kind: kindIn.value.trim() || "auto", maxSteps: 18 }),
        });
        if (!res.ok || !res.body) throw new Error("HTTP " + res.status);
        const reader = res.body.getReader();
        const dec = new TextDecoder();
        let buf = "";
        for (;;) {
          const { value, done } = await reader.read();
          if (done) break;
          buf += dec.decode(value, { stream: true });
          const parts = buf.split("\n\n");
          buf = parts.pop();
          for (const part of parts) {
            const line = part.split("\n").find((l) => l.startsWith("data:"));
            if (!line) continue;
            let ev;
            try { ev = JSON.parse(line.slice(5).trim()); } catch { continue; }
            handle(ev);
          }
        }
        say("\n⏱️  Muda wote: " + ((performance.now() - t0Ref.t) / 1000).toFixed(1) + "s");
      } catch (e) {
        say("⚠️ " + e.message + " (naendelea kupoll matokeo…)");
      } finally {
        running = false;
        btn.disabled = false;
        if (pollTimer) { clearInterval(pollTimer); pollTimer = 0; }
        if (!gotResult) {
          try {
            const j = await (await fetch("/api/ide/agent/agentic/result")).json();
            if (!applyResult(j)) status.textContent = "Tayari";
          } catch { status.textContent = "Tayari"; }
        }
      }
    };

    runBtn.onclick = async () => {
      if (!lastProject) return;
      runBtn.disabled = true;
      status.textContent = "Inakimbia…";
      say("\n▶ Run: " + lastProject);
      try {
        const r = await fetch("/api/ide/agent/build/run", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ project: lastProject }),
        });
        const j = await r.json();
        say((j.ok ? "✅" : "⛔") + " `" + j.command + "` → ok=" + j.ok + " (" + j.elapsed_ms + "ms)");
        if (j.output) say(String(j.output).slice(-1500));
        if (j.error) say("⚠️ " + j.error);
      } catch (e) {
        say("⚠️ " + e.message);
      } finally {
        runBtn.disabled = false;
        status.textContent = "Tayari";
      }
    };

    return () => shell.remove();
  });

  // ═══ UKURASA 4: SDLC — mzunguko kamili wa maendeleo ═══
  host.registerPage("sdlc", async ({ container }) => {
    const el = (tag, css, text) => {
      const n = document.createElement(tag);
      if (css) n.style.cssText = css;
      if (text != null) n.textContent = text;
      return n;
    };
    const shell = el("section",
      "min-height:100%;padding:1.5rem;color:var(--oh-text-primary,#fff);" +
      "background:var(--oh-color-base,#0b0f14);font-family:system-ui,sans-serif;" +
      "display:flex;flex-direction:column;gap:12px;");
    container.append(shell);

    // Lugha ya mtumiaji (sw/en/fr/…). Jibu hutolewa kwa lugha hii.
    let uiLang = localStorage.getItem("mtechos_lang") || "sw";
    const L = {
      sw: {
        title: "SDLC — mzunguko kamili wa maendeleo",
        desc: "Mahitaji → Usanifu → Ujenzi → Uthibitisho → Uwasilishaji → Ufuatiliaji. " +
          "Kina huchaguliwa kwa ugumu wa mradi (Rahisi/Wastani/Mgumu). Kila awamu " +
          "ina GATE na inahitaji idhini yako (\"Je, tuendelee?\").",
        ph: "Mfano: REST API ya TODO yenye tests, au tovuti ya kuuza bidhaa…",
        plan: "📐 Panga (plan)", assess: "🧭 Pima ugumu", run: "🚀 Endesha SDLC",
        ready: "Tayari", assessing: "Inapima…", planning: "Inapanga…", running: "Inaendesha SDLC…",
        complexity: "Ugumu", depth: "Kina", deliverables: "Deliverables", guardrails: "Vikwazo",
        approve: "Je, tuendelee?", lang: "Lugha",
        approveBtn: "✅ Idhinisha & endelea", runFirst: "Endesha SDLC kwanza",
        nextPhase: "Awamu inayofuata", approved: "Awamu zilizoidhinishwa",
      },
      en: {
        title: "SDLC — full development lifecycle",
        desc: "Requirements → Design → Development → Testing → Deployment → Maintenance. " +
          "Depth is chosen by project complexity (Simple/Medium/Complex). Each phase has " +
          "a GATE and needs your approval (\"Shall we continue?\").",
        ph: "e.g. a TODO REST API with tests, or a shop website…",
        plan: "📐 Plan", assess: "🧭 Assess complexity", run: "🚀 Run SDLC",
        ready: "Ready", assessing: "Assessing…", planning: "Planning…", running: "Running SDLC…",
        complexity: "Complexity", depth: "Depth", deliverables: "Deliverables", guardrails: "Guardrails",
        approve: "Shall we continue?", lang: "Language",
        approveBtn: "✅ Approve & continue", runFirst: "Run SDLC first",
        nextPhase: "Next phase", approved: "Approved phases",
      },
    };
    const t = () => L[uiLang] || L.sw;

    const header = el("div", "display:flex;gap:10px;align-items:center;justify-content:space-between;flex-wrap:wrap;");
    header.append(el("h2", "font-size:1.15rem;margin:0;font-weight:600;", t().title));
    const langSel = el("select",
      "padding:6px 10px;border-radius:8px;background:rgba(255,255,255,.05);color:inherit;" +
      "border:1px solid var(--oh-border-variant,#2a2f3a);font-size:12px;");
    for (const [code, name] of [["sw", "Kiswahili"], ["en", "English"], ["fr", "Français"], ["ar", "العربية"], ["pt", "Português"]]) {
      const o = el("option", null, name);
      o.value = code;
      if (code === uiLang) o.selected = true;
      langSel.append(o);
    }
    header.append(langSel);
    shell.append(header);

    const desc = el("p", "margin:0;font-size:12px;opacity:.7;line-height:1.6;", t().desc);
    shell.append(desc);

    const caps = el("div", "font-size:11px;opacity:.8;line-height:1.7;" +
      "background:rgba(255,255,255,.04);border-radius:8px;padding:10px;",
      "…");
    shell.append(caps);

    const ta = el("textarea",
      "width:100%;resize:vertical;padding:10px 12px;border-radius:8px;font-size:13px;" +
      "background:rgba(255,255,255,.05);color:inherit;border:1px solid var(--oh-border-variant,#2a2f3a);");
    ta.rows = 2;
    ta.placeholder = t().ph;
    shell.append(ta);

    const row = el("div", "display:flex;gap:10px;align-items:center;flex-wrap:wrap;");
    shell.append(row);
    const assessBtn = el("button",
      "padding:10px 16px;border-radius:8px;border:1px solid var(--oh-border-variant,#2a2f3a);cursor:pointer;" +
      "font-weight:600;font-size:13px;background:transparent;color:inherit;", t().assess);
    const planBtn = el("button",
      "padding:10px 16px;border-radius:8px;border:1px solid var(--oh-border-variant,#2a2f3a);cursor:pointer;" +
      "font-weight:600;font-size:13px;background:transparent;color:inherit;", t().plan);
    const runBtn = el("button",
      "padding:10px 18px;border-radius:8px;border:0;cursor:pointer;font-weight:600;font-size:13px;" +
      "background:var(--oh-brand,#5865f2);color:#fff;", t().run);
    row.append(assessBtn, planBtn, runBtn);
    const status = el("span", "font-size:12px;opacity:.75;", t().ready);
    row.append(status);

    const log = el("pre",
      "flex:1;min-height:200px;max-height:58vh;overflow:auto;white-space:pre-wrap;" +
      "font-size:12px;line-height:1.6;background:rgba(255,255,255,.04);" +
      "border-radius:8px;padding:12px;margin:0;", t().ready);
    shell.append(log);

    const say = (s) => {
      log.textContent += (log.textContent ? "\n" : "") + s;
      log.scrollTop = log.scrollHeight;
    };

    function renderCaps(cj) {
      const ph = (cj.phases || []).map((p) => uiLang === "sw" ? p.label_sw : p.label_en);
      caps.textContent = ph.join(" → ") + "  ·  " + t().complexity + ": " +
        (cj.complexity_levels || []).map((c) => uiLang === "sw" ? c.label_sw : c.label_en).join(" / ");
    }

    async function loadCaps() {
      try {
        const cj = await (await fetch("/api/sdlc/capabilities", { headers: authHeaders() })).json();
        renderCaps(cj);
      } catch (e) {
        caps.textContent = "⚠️ " + e.message;
      }
    }

    function renderComplexity(j) {
      const c = j.complexity || {};
      say("🧭 " + t().complexity + ": " + (c.label || c.level) +
        " (score " + c.score + ")");
      for (const r of (c.reasons || []).slice(0, 8)) say("   • " + r);
    }

    function renderPlan(j) {
      say("📐 " + j.project + "  [" + j.kind + "/" + j.lang_code + "]");
      renderComplexity(j);
      say((t().lang) + ": " + j.lang);
      say("✓ " + (j.acceptance_criteria || []).length + " " + t().deliverables + " (acceptance):");
      for (const c of j.acceptance_criteria || []) say("   ✓ " + c);
      say("🗂️ " + (j.files_planned || []).length + " faili:");
      for (const f of j.files_planned || []) say("   • " + f.path + " — " + f.purpose);
      say("— " + t().guardrails + ":");
      for (const g of j.guardrails || []) say("   🛡️ " + g.label);
    }

    function renderRun(j) {
      say("");
      say("📦 " + j.project + "  [" + j.kind + "/" + j.lang + "]  ·  " + t().depth + ": " + j.depth);
      if (j.complexity) renderComplexity(j);
      for (const p of j.phases || []) {
        const mark = p.status === "ok" ? "✅" : p.status === "skipped" ? "⏭️" : "⚠️";
        let line = mark + " " + p.label + " — " + p.status;
        if (p.id === "usanifu") line += " (" + p.files_planned + " faili)";
        if (p.id === "ujenzi") line += " (" + p.written + " zimeandikwa)";
        if (p.id === "uthibitisho") line += " · verify=" + p.verify_ok + " · grade " + Math.round(p.security_grade);
        if (p.id === "uwasilishaji") line += " (" + p.steps + " hatua)";
        say(line);
        if (p.deliverables && p.deliverables.length) {
          say("   " + t().deliverables + ": " + p.deliverables.map((d) => d.name).join(", "));
        }
        if (p.id === "uthibitisho") {
          if (p.output) say("   --- uthibitisho ---\n   " + String(p.output).split("\n").slice(-8).join("\n   "));
          for (const g of p.guardrails || []) say("   " + (g.ok ? "🛡️✅" : "🛡️⛔") + " " + g.id);
          const dod = p.definition_of_done;
          if (dod && !dod.done) for (const b of dod.blockers || []) say("   ⛔ " + b);
        }
        if (p.requires_approval) say("   ❓ " + t().approve);
      }
      say((j.ok ? "🏁" : "⛔") + " SDLC: ok=" + j.ok + " · score " + j.score);
      if (j.workflow) say("ℹ️ " + j.workflow);
      lastProject = j.project || lastProject;
      if (j.approved_phases && j.approved_phases.length) {
        say("✅ " + t().approved + ": " + j.approved_phases.join(", "));
      }
      renderContinue();
    }

    let lastProject = "";
    let lastPhase = "mahitaji";
    const contRow = el("div", "display:flex;gap:8px;align-items:center;flex-wrap:wrap;");
    const phaseSel = el("select",
      "padding:6px 10px;border-radius:8px;background:rgba(255,255,255,.05);color:inherit;" +
      "border:1px solid var(--oh-border-variant,#2a2f3a);font-size:12px;");
    for (const id of ["mahitaji", "usanifu", "ujenzi", "uthibitisho", "uwasilishaji", "ufuatiliaji"]) {
      const o = el("option", null, id);
      o.value = id;
      phaseSel.append(o);
    }
    const contBtn = el("button",
      "padding:8px 14px;border-radius:8px;border:0;cursor:pointer;font-weight:600;font-size:12px;" +
      "background:#22c55e;color:#04120a;", t().approveBtn);
    const contInfo = el("span", "font-size:12px;opacity:.75;", "");
    contRow.append(phaseSel, contBtn, contInfo);
    shell.append(contRow);

    function renderContinue() {
      contInfo.textContent = lastProject ? ("📁 " + lastProject) : "";
    }

    contBtn.onclick = async () => {
      if (!lastProject) return say("⚠️ " + t().runFirst);
      contBtn.disabled = true;
      try {
        const res = await fetch("/api/sdlc/continue", {
          method: "POST",
          headers: authHeaders({ "Content-Type": "application/json" }),
          body: JSON.stringify({ project: lastProject, phase: phaseSel.value, lang: uiLang }),
        });
        const j = await res.json();
        if (!j.ok) { say("⚠️ " + (j.error || "kosa")); return; }
        say("✅ " + t().approved + ": " + (j.approved || []).join(", "));
        if (j.next_label) say("➡️ " + t().nextPhase + ": " + j.next_label);
        say("ℹ️ " + j.message);
      } catch (e) {
        say("⚠️ " + e.message);
      } finally {
        contBtn.disabled = false;
      }
    };

    async function post(url, spec) {
      const res = await fetch(url, {
        method: "POST",
        headers: authHeaders({ "Content-Type": "application/json" }),
        body: JSON.stringify({ spec, lang: uiLang }),
      });
      return res.json();
    }

    let running = false;
    async function go(btn, label, fn) {
      const spec = ta.value.trim();
      if (!spec || running) return;
      running = true;
      for (const b of [assessBtn, planBtn, runBtn]) b.disabled = true;
      log.textContent = "";
      status.textContent = label;
      try {
        await fn(spec);
      } catch (e) {
        say("⚠️ " + e.message);
      } finally {
        running = false;
        for (const b of [assessBtn, planBtn, runBtn]) b.disabled = false;
        status.textContent = t().ready;
        btn.disabled = false;
      }
    }

    assessBtn.onclick = () => go(assessBtn, t().assessing, async (spec) => {
      const j = await post("/api/sdlc/assess", spec);
      if (!j.ok) return say("⚠️ " + (j.error || "kosa"));
      renderComplexity(j);
      say("— " + t().guardrails + ":");
      for (const g of j.guardrails || []) say("   🛡️ " + g.label);
      say("— " + t().depth + " (" + j.complexity.label + "):");
      for (const p of j.phase_plan || []) {
        say("   ▸ " + p.label + " · " + p.depth);
        for (const a of p.activities || []) say("      - " + a);
        say("      " + t().deliverables + ": " + (p.deliverables || []).map((d) => d.name).join(", "));
      }
    });

    planBtn.onclick = () => go(planBtn, t().planning, async (spec) => {
      const j = await post("/api/sdlc/plan", spec);
      if (!j.ok) return say("⚠️ " + (j.error || "kosa"));
      renderPlan(j);
    });

    runBtn.onclick = () => go(runBtn, t().running, async (spec) => {
      const j = await post("/api/sdlc/run", spec);
      renderRun(j);
    });

    langSel.onchange = () => {
      uiLang = langSel.value;
      localStorage.setItem("mtechos_lang", uiLang);
      header.firstChild.textContent = t().title;
      desc.textContent = t().desc;
      ta.placeholder = t().ph;
      assessBtn.textContent = t().assess;
      planBtn.textContent = t().plan;
      runBtn.textContent = t().run;
      status.textContent = t().ready;
      loadCaps();
    };

    await loadCaps();
    return () => shell.remove();
  });

  // ═══ UKURASA 1: MTECHOS FORGE — zalisha mradi mzima (SSE) ═══
  return host.registerPage("mtechos-ide", ({ container }) => {
    const shell = document.createElement("section");
    shell.style.cssText =
      "min-height:100%;padding:1.5rem;color:var(--oh-text-primary,#fff);" +
      "background:var(--oh-color-base,#0b0f14);font-family:system-ui,sans-serif;" +
      "display:flex;flex-direction:column;gap:12px;";
    container.append(shell);

    const title = document.createElement("h2");
    title.textContent = "MTECHOS FORGE — zalisha mradi mzima";
    title.style.cssText = "font-size:1.15rem;margin:0;font-weight:600;";
    shell.append(title);

    const hint = document.createElement("p");
    hint.textContent =
      "Eleza mradi kwa Kiswahili (mfano: \"tovuti ya kuuza bidhaa\", \"REST API ya malipo\", " +
      "\"dashboard ya takwimu\"). FORGE hupanga blueprint, huzalisha faili NYINGI kwa parallel, " +
      "hupitisha linter + usalama, kisha hukagua (engineer).";
    hint.style.cssText = "margin:0;font-size:12px;opacity:.7;line-height:1.6;";
    shell.append(hint);

    const ta = document.createElement("textarea");
    ta.placeholder = "Mradi wako…";
    ta.rows = 2;
    ta.style.cssText =
      "width:100%;resize:vertical;padding:10px 12px;border-radius:8px;font-size:13px;" +
      "background:rgba(255,255,255,.05);color:inherit;border:1px solid var(--oh-border-variant,#2a2f3a);";
    shell.append(ta);

    const row = document.createElement("div");
    row.style.cssText = "display:flex;gap:10px;align-items:center;flex-wrap:wrap;";
    shell.append(row);

    const btn = document.createElement("button");
    btn.textContent = "⚡ Zalisha mradi";
    btn.style.cssText =
      "padding:10px 18px;border-radius:8px;border:0;cursor:pointer;font-weight:600;font-size:13px;" +
      "background:var(--oh-brand,#5865f2);color:#fff;";
    row.append(btn);

    const engLabel = document.createElement("label");
    engLabel.style.cssText = "font-size:12px;display:flex;gap:6px;align-items:center;opacity:.85;";
    const eng = document.createElement("input");
    eng.type = "checkbox";
    eng.checked = true;
    engLabel.append(eng);
    engLabel.append(document.createTextNode("Engineer (testi + build)"));
    row.append(engLabel);

    const status = document.createElement("span");
    status.style.cssText = "font-size:12px;opacity:.75;";
    row.append(status);

    const log = document.createElement("pre");
    log.style.cssText =
      "flex:1;min-height:180px;max-height:55vh;overflow:auto;white-space:pre-wrap;" +
      "font-size:12px;line-height:1.6;background:rgba(255,255,255,.04);" +
      "border-radius:8px;padding:12px;margin:0;";
    log.textContent = "Tayari.";
    shell.append(log);

    let running = false;
    const say = (s) => {
      log.textContent += (log.textContent ? "\n" : "") + s;
      log.scrollTop = log.scrollHeight;
    };

    async function generate() {
      const spec = ta.value.trim();
      if (!spec || running) return;
      running = true;
      btn.disabled = true;
      log.textContent = "";
      status.textContent = "Inazalisha…";
      const t0 = performance.now();

      try {
        const res = await fetch("/api/forge/run", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ spec, engineer: eng.checked }),
        });
        if (!res.ok || !res.body) {
          throw new Error("HTTP " + res.status);
        }
        const reader = res.body.getReader();
        const dec = new TextDecoder();
        let buf = "";
        for (;;) {
          const { value, done } = await reader.read();
          if (done) break;
          buf += dec.decode(value, { stream: true });
          const parts = buf.split("\n\n");
          buf = parts.pop();
          for (const part of parts) {
            const line = part.split("\n").find((l) => l.startsWith("data:"));
            if (!line) continue;
            let ev;
            try { ev = JSON.parse(line.slice(5).trim()); } catch { continue; }
            handle(ev);
          }
        }
      } catch (e) {
        say("⚠️ " + e.message);
      } finally {
        running = false;
        btn.disabled = false;
        status.textContent = "Tayari";
      }

      function handle(ev) {
        if (ev.type === "step") {
          say("⏳ " + ev.stage + (ev.file ? " → " + ev.file : "") + "…");
        } else if (ev.type === "blueprint") {
          say("📐 Blueprint (" + ev.source + "): " + ev.project +
            " [" + ev.kind + "/" + ev.lang + "]");
          for (const f of ev.files || []) say("   • " + f.path + " — " + f.purpose);
        } else if (ev.type === "file") {
          say("   " + (ev.ok ? "✅" : "⚠️") + " " + ev.path +
            " (" + ev.bytes + "B, rounds " + ev.rounds +
            (ev.critical ? ", ⛔ critical " + ev.critical : "") + ")");
        } else if (ev.type === "engineer") {
          say("🔧 Engineer: score " + ev.score);
        } else if (ev.type === "done") {
          const secs = ((performance.now() - t0) / 1000).toFixed(2);
          say("\n🏁 Mradi \"" + ev.project + "\" umekamilika kwa " + secs + "s");
          say("   faili safi: " + ev.files_clean + "/" + ev.files_total +
            " · critical: " + ev.critical_findings +
            (ev.engineer_score != null ? " · engineer: " + ev.engineer_score : ""));
        } else if (ev.type === "error") {
          say("⚠️ " + (ev.error || "kosa"));
        }
      }
    }

    btn.onclick = generate;

    return () => shell.remove();
  });
}

#!/usr/bin/env node
/**
 * Live harness for the built MBILINYITECH IDE extension.
 * Node port — hakuna Python.
 *
 * Hutumikia `extension.js` halisi (kile kinachosafiri kwenye Canvas App) pamoja
 * na mock ya API ya MTECHOS Rust agent, ili app iweze kujaribiwa kwenye browser
 * bila kujenga backend ya Rust. Hii ni preview/proof server PEKEE; si sehemu ya
 * app inayosafiri.
 *
 *     node harness/server.mjs 12000
 *
 * Env: `OLLAMA_API_KEY` / `FUNDI_AI_KEY` / `<repo>/data/ai_key`, `OLLAMA_MODEL`.
 */
import { createServer } from "node:http";
import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const EXTENSION_JS = resolve(HERE, "..", "extension.js");
const INDEX_HTML = join(HERE, "index.html");

let OLLAMA_KEY = process.env.OLLAMA_API_KEY || process.env.FUNDI_AI_KEY || "";
if (!OLLAMA_KEY) {
  const keyfile = resolve(HERE, "..", "..", "..", "data", "ai_key");
  if (existsSync(keyfile)) OLLAMA_KEY = readFileSync(keyfile, "utf-8").trim();
}
const OLLAMA_URL = "https://ollama.com";
const CHAT_MODEL = process.env.OLLAMA_MODEL || "gpt-oss:20b";
const PORT = Number(process.argv[2] || 12000);

/** Ollama Cloud completion halisi; hurudisha text au kurusha kosa. */
async function ollamaChat(messages, model = CHAT_MODEL, timeoutMs = 60_000) {
  const ac = new AbortController();
  const t = setTimeout(() => ac.abort(), timeoutMs);
  try {
    const r = await fetch(`${OLLAMA_URL}/api/chat`, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${OLLAMA_KEY}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ model, messages, stream: false }),
      signal: ac.signal,
    });
    const d = await r.json();
    return (d.message || {}).content || "";
  } finally {
    clearTimeout(t);
  }
}

const TREE = {
  ok: true,
  entries: [
    { path: "src/main.rs", name: "main.rs" },
    { path: "package.json", name: "package.json" },
    { path: "README.md", name: "README.md" },
    { path: "docker-compose.yml", name: "docker-compose.yml" },
  ],
};

const FILES = {
  "src/main.rs": 'fn main() {\n    println!("Habari MTECHOS - kutoka MBILINYITECH IDE!");\n}\n',
  "package.json": '{\n  "name": "demo-app",\n  "version": "1.0.0",\n  "scripts": { "dev": "vite" }\n}\n',
  "README.md": "# Demo App\n\nImeundwa na Autopilot ya MBILINYITECH IDE (mock).\n",
  "docker-compose.yml": "services:\n  app:\n    build: .\n    ports: ['3000:3000']\n",
};

function send(res, code, body, ctype = "application/json") {
  const payload = typeof body === "string" ? body : JSON.stringify(body);
  res.writeHead(code, {
    "Content-Type": typeof body === "string" ? ctype : "application/json",
    "Access-Control-Allow-Origin": "*",
    "Content-Length": Buffer.byteLength(payload),
  });
  res.end(payload);
}

async function handleAgentRun(res, req) {
  const idea = req.idea || "(idea)";
  const kind = req.kind || "website";
  const steps = [
    { step: "akili_v6", status: "ok", action: "scaffold", confidence: 0.86, ethical_allowed: true },
  ];
  let project = "demo-app";
  let files = [];
  try {
    const prompt =
      "Wewe ni Autopilot wa MBILINYITECH IDE. Rejesha JSON MOJA bila markdown yenye " +
      '"project_name" (kebab-case) na "files" (orodha ya {"path","content"}) za mradi ' +
      `aina '${kind}' kwa wazo: ${idea}. Content iwe code halisi inayofanya kazi, files 3-5.`;
    let raw = await ollamaChat([{ role: "user", content: prompt }], CHAT_MODEL, 120_000);
    const clean = raw
      .trim()
      .replace(/^```json/, "")
      .replace(/^```/, "")
      .replace(/```$/, "")
      .trim();
    const plan = JSON.parse(clean);
    project = plan.project_name || project;
    files = plan.files || [];
    steps.push({ step: "plan", status: "ok", project, model: CHAT_MODEL, llm: "ollama-cloud" });
  } catch (e) {
    steps.push({ step: "plan", status: "error", error: `LLM: ${e}` });
  }

  const written = [];
  const ws = join("/tmp/mbide-workspace", project);
  for (const f of files) {
    const path = f.path || "";
    const content = f.content || "";
    if (!path || path.includes("..") || path.startsWith("/")) continue;
    const full = join(ws, path);
    mkdirSync(dirname(full), { recursive: true });
    writeFileSync(full, content);
    written.push({ path, size: content.length });
  }
  steps.push({
    step: "scaffold",
    status: written.length ? "ok" : "empty",
    files_count: written.length,
    workspace: ws,
  });
  steps.push({ step: "report", status: "ok", workspace: ws });

  send(res, 200, {
    ok: written.length > 0,
    steps,
    project,
    kind,
    idea,
    files: written,
    note: "Plan + files HALISI kutoka Ollama Cloud; harness inaandika kwenye /tmp.",
  });
}

async function handleAgentChat(res, req) {
  const q = req.question || "";
  try {
    const reply = await ollamaChat([
      { role: "system", content: "Wewe ni coding agent wa MBILINYITECH IDE (MTECHOS). Jibu kwa Kiswahili kifupi na fasaha." },
      { role: "user", content: q },
    ]);
    send(res, 200, { ok: true, reply, model: CHAT_MODEL, llm: "ollama-cloud" });
  } catch (e) {
    send(res, 200, { ok: false, error: `LLM: ${e}` });
  }
}

const server = createServer((req, res) => {
  const u = new URL(req.url, `http://${req.headers.host}`);
  const p = u.pathname;

  if (req.method === "GET") {
    if (p === "/" || p === "/index.html") {
      return send(res, 200, readFileSync(INDEX_HTML, "utf-8"), "text/html; charset=utf-8");
    }
    if (p === "/extension.js") {
      return send(res, 200, readFileSync(EXTENSION_JS, "utf-8"), "text/javascript; charset=utf-8");
    }
    if (p === "/api/ide/tree") return send(res, 200, TREE);
    if (p === "/api/ide/file") {
      const path = u.searchParams.get("path") || "";
      return send(res, 200, { ok: true, path, content: FILES[path] || `// ${path}\n` });
    }
    if (p === "/api/agent/health") return send(res, 200, { ok: true, status: "online", agent: "mttechos-rust" });
    if (p === "/api/ai/test") return send(res, 200, { ok: true, model: "mock-llm", reachable: true });
    if (p === "/api/ide/agent/status") return send(res, 200, { ok: true, running: false });
    return send(res, 404, { ok: false, error: `no route ${p}` });
  }

  if (req.method === "POST") {
    let raw = "";
    req.on("data", (c) => (raw += c));
    req.on("end", async () => {
      let body = {};
      try {
        body = JSON.parse(raw || "{}");
      } catch {
        body = {};
      }
      try {
        if (p === "/api/ide/agent/run") return await handleAgentRun(res, body);
        if (p === "/api/agent/chat") return await handleAgentChat(res, body);
        if (p === "/api/ai/key") return send(res, 200, { ok: true, note_sw: "key imehifadhiwa (mock)" });
        if (p === "/api/ai/custom") return send(res, 200, { ok: true, url: body.url, model: body.model });
        if (p === "/api/ide/file") return send(res, 200, { ok: true, saved: body.path });
        if (p === "/api/ide/terminal")
          return send(res, 200, { ok: true, stdout: `$ ${body.command || ""}\n(harness mock)\n` });
        return send(res, 404, { ok: false, error: `no route ${p}` });
      } catch (e) {
        return send(res, 500, { ok: false, error: String(e) });
      }
    });
    return;
  }

  send(res, 405, { ok: false, error: "method not allowed" });
});

server.listen(PORT, "0.0.0.0", () => {
  console.log(`MBILINYITECH IDE harness: http://0.0.0.0:${PORT} (extension.js + mock API)`);
});

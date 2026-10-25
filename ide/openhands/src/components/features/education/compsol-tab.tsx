import { useCallback, useEffect, useState } from "react";
import { Stethoscope, Terminal, ShieldCheck, Wrench, ImageIcon } from "lucide-react";
import {
  compsolService,
  type CommandEntry,
  type Diagnostic,
  type DiagRow,
  type FixPlan,
  type SafeAction,
} from "#/api/education-service/compsol.api";
import {
  EduBadge,
  EduButton,
  EduCard,
  EduEmpty,
  EduError,
  EduField,
  EduInput,
  EduSpinner,
} from "./edu-ui";

/**
 * MTECHOS — Sehemu 6: Computer Solutions (moduli 22).
 * Uchunguzi halisi wa mfumo (CPU/RAM/diski/processes), amri za Windows/macOS/Linux,
 * vision ya screen, na kutatua kwa HITL (allowlist pekee).
 */
export function CompSolTab() {
  const [diag, setDiag] = useState<Diagnostic | null>(null);
  const [history, setHistory] = useState<DiagRow[]>([]);
  const [catalog, setCatalog] = useState<CommandEntry[]>([]);
  const [osName, setOsName] = useState("");
  const [plan, setPlan] = useState<FixPlan | null>(null);
  const [applyOut, setApplyOut] = useState("");
  const [visionUrl, setVisionUrl] = useState("");
  const [visionOut, setVisionOut] = useState("");
  const [problem, setProblem] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");

  const refresh = useCallback(async () => {
    try {
      const c = await compsolService.commands();
      setCatalog(c.catalog);
      setOsName(c.os_yako);
      setHistory((await compsolService.diagnostics()).diagnostics);
      setErr("");
    } catch (e) {
      setErr(String(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const runDiagnose = async () => {
    setBusy(true);
    try {
      setDiag(await compsolService.diagnose());
      await refresh();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const getPlan = async () => {
    setBusy(true);
    try {
      setPlan(await compsolService.fixPlan({ diag_id: diag?.id ?? "", problem }));
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const applyAction = async (a: SafeAction) => {
    if (!window.confirm(`Endesha: ${a.label}?\n\n${a.cmd}`)) return;
    setBusy(true);
    try {
      const r = await compsolService.fixApply(a.id, true);
      setApplyOut(`[${r.status}] ${r.action}\n${r.output}`);
      setErr(r.ok ? "" : r.note_sw);
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const runVision = async () => {
    if (!visionUrl.trim()) return;
    setBusy(true);
    try {
      const r = await compsolService.vision(visionUrl.trim());
      setVisionOut(r.ok ? JSON.stringify(r.analysis, null, 2) : (r.error ?? "imeshindwa"));
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const sevColor = (s: string) =>
    s === "critical" ? "text-red-400" : s === "warning" ? "text-amber-400" : "text-primary";

  return (
    <div className="flex flex-col gap-3" data-testid="compsol-tab">
      <EduCard testId="compsol-status">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="text-sm font-semibold text-content">
            Computer Solutions — kutatua matatizo ya kompyuta
          </span>
          <EduBadge testId="compsol-os">OS: {osName}</EduBadge>
        </div>
        <p className="mt-1 text-xs text-muted">
          Vipimo halisi vya mfumo (procfs/ps/df). Utekelezaji wa kutatua ni HITL —
          amri SALAMA pekee kutoka orodha iliyowekwa.
        </p>
      </EduCard>

      {err ? <EduError message={err} /> : null}

      <EduCard testId="compsol-diag">
        <div className="flex flex-wrap items-center gap-2">
          <EduButton testId="compsol-diagnose" disabled={busy} onClick={() => void runDiagnose()}>
            <Stethoscope className="size-4" aria-hidden /> Chunguza mfumo sasa
          </EduButton>
          {busy ? <EduSpinner label="Inachunguza…" /> : null}
        </div>
        {diag ? (
          <div className="mt-2 flex flex-col gap-2" data-testid="compsol-diag-out">
            <div className="flex flex-wrap gap-1">
              <EduBadge testId="compsol-cpu">CPU {diag.cpu_pct}%</EduBadge>
              <EduBadge testId="compsol-ram">RAM {diag.ram_pct}%</EduBadge>
              <EduBadge testId="compsol-disk">Diski {diag.disk_pct}%</EduBadge>
              <EduBadge>load {diag.load1}</EduBadge>
              <EduBadge>{diag.processes} processes</EduBadge>
              <EduBadge testId="compsol-severity">{diag.severity}</EduBadge>
            </div>
            <div className="flex flex-col gap-1">
              {diag.findings.map((f) => (
                <div
                  key={f.code}
                  data-testid={`compsol-finding-${f.code}`}
                  className="rounded-md border border-border p-2"
                >
                  <div className={`text-sm font-semibold ${sevColor(f.severity)}`}>
                    {f.severity.toUpperCase()} — {f.message}
                  </div>
                  <div className="text-xs text-muted">{f.hint}</div>
                </div>
              ))}
            </div>
            {diag.top_processes.length > 0 ? (
              <div>
                <span className="text-xs font-semibold uppercase text-muted">Processes za juu</span>
                <div className="mt-1 flex flex-col gap-1">
                  {diag.top_processes.map((p) => (
                    <div key={p.name} className="flex justify-between rounded border border-border px-2 py-1 text-xs">
                      <span className="text-content">{p.name}</span>
                      <span className="text-muted">
                        CPU {p.cpu}% · RAM {p.mem}%
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            ) : null}
          </div>
        ) : (
          <EduEmpty>Bofya "Chunguza mfumo sasa" kupata vipimo halisi.</EduEmpty>
        )}
      </EduCard>

      <EduCard testId="compsol-plan">
        <span className="text-sm font-semibold text-content">Mpango wa kutatua (agentic)</span>
        <div className="mt-2 flex flex-wrap items-end gap-2">
          <div className="min-w-56 flex-1">
            <EduField label="Eleza tatizo (hiari)">
              <EduInput
                testId="compsol-problem"
                value={problem}
                onChange={(e) => setProblem(e.target.value)}
                placeholder="mf. Kompyuta ni polepole sana na inazima ghafla"
              />
            </EduField>
          </div>
          <EduButton testId="compsol-plan-btn" disabled={busy} onClick={() => void getPlan()}>
            <Wrench className="size-4" aria-hidden /> Pata mpango
          </EduButton>
        </div>
        {plan ? (
          <div className="mt-2 flex flex-col gap-2" data-testid="compsol-plan-out">
            <p className="text-sm text-content">{plan.plan.ufafanuzi}</p>
            {plan.plan.hatua?.length ? (
              <ol className="ml-4 list-decimal text-sm text-content">
                {plan.plan.hatua.map((h, i) => (
                  <li key={`hatua-${i}`}>{h}</li>
                ))}
              </ol>
            ) : null}
            <div>
              <span className="text-xs font-semibold uppercase text-muted">
                Matendo salama (HITL — unahitaji kuidhinisha)
              </span>
              <div className="mt-1 flex flex-col gap-1">
                {plan.available_actions.map((a) => (
                  <div
                    key={a.id}
                    data-testid={`compsol-action-${a.id}`}
                    className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2"
                  >
                    <div className="min-w-0">
                      <div className="text-sm text-content">{a.label}</div>
                      <code className="text-xs text-muted">{a.cmd}</code>
                    </div>
                    <div className="flex items-center gap-1">
                      <EduBadge>{a.risk}</EduBadge>
                      <EduButton testId={`compsol-run-${a.id}`} disabled={busy} onClick={() => void applyAction(a)}>
                        Endesha
                      </EduButton>
                    </div>
                  </div>
                ))}
              </div>
            </div>
            <p className="text-xs text-muted">{plan.note_sw}</p>
          </div>
        ) : null}
        {applyOut ? (
          <pre
            data-testid="compsol-apply-out"
            className="mt-2 max-h-64 overflow-auto rounded-md border border-border bg-surface p-2 text-xs text-content"
          >
            {applyOut}
          </pre>
        ) : null}
      </EduCard>

      <EduCard testId="compsol-vision">
        <span className="text-sm font-semibold text-content">Visioning — chunguza picha ya screen</span>
        <div className="mt-2 flex flex-wrap items-end gap-2">
          <div className="min-w-56 flex-1">
            <EduField label="URL ya picha (error screen)">
              <EduInput
                testId="compsol-vision-url"
                value={visionUrl}
                onChange={(e) => setVisionUrl(e.target.value)}
                placeholder="https://…/screenshot.png"
              />
            </EduField>
          </div>
          <EduButton testId="compsol-vision-btn" disabled={busy} onClick={() => void runVision()}>
            <ImageIcon className="size-4" aria-hidden /> Chunguza
          </EduButton>
        </div>
        {visionOut ? (
          <pre
            data-testid="compsol-vision-out"
            className="mt-2 max-h-64 overflow-auto rounded-md border border-border bg-surface p-2 text-xs text-content"
          >
            {visionOut}
          </pre>
        ) : null}
      </EduCard>

      <EduCard testId="compsol-commands">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="text-sm font-semibold text-content">
            Amri kwa OS zote tatu ({catalog.length})
          </span>
          <EduBadge>
            <ShieldCheck className="size-3" aria-hidden /> Kiswahili
          </EduBadge>
        </div>
        <div className="mt-2 flex flex-col gap-1">
          {catalog.map((c) => (
            <div
              key={c.topic}
              data-testid={`compsol-cmd-${c.topic}`}
              className="rounded-md border border-border p-2"
            >
              <div className="text-sm text-content">{c.topic}</div>
              <div className="mt-1 grid gap-1 text-xs md:grid-cols-3">
                <div>
                  <span className="text-muted">Windows: </span>
                  <code className="text-content">{c.windows}</code>
                </div>
                <div>
                  <span className="text-muted">macOS: </span>
                  <code className="text-content">{c.macos}</code>
                </div>
                <div>
                  <span className="text-muted">Linux: </span>
                  <code className="text-content">{c.linux}</code>
                </div>
              </div>
              <div className="mt-1 text-xs text-muted">{c.maelezo}</div>
            </div>
          ))}
        </div>
      </EduCard>

      <EduCard testId="compsol-history">
        <span className="text-sm font-semibold text-content">
          <Terminal className="size-4 inline" aria-hidden /> Uchunguzi wa nyuma ({history.length})
        </span>
        <div className="mt-2 flex flex-col gap-1">
          {history.length === 0 ? (
            <EduEmpty>Hakuna uchunguzi bado.</EduEmpty>
          ) : (
            history.slice(0, 10).map((h) => (
              <div
                key={h.id}
                data-testid={`compsol-hist-${h.id}`}
                className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2 text-xs"
              >
                <span className="text-content">{h.created_at.slice(0, 19)}</span>
                <div className="flex gap-1">
                  <EduBadge>CPU {h.cpu_pct}%</EduBadge>
                  <EduBadge>RAM {h.ram_pct}%</EduBadge>
                  <EduBadge>Diski {h.disk_pct}%</EduBadge>
                  <EduBadge>{h.severity}</EduBadge>
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>
    </div>
  );
}

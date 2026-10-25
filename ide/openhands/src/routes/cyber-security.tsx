import { useCallback, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import {
  Activity,
  Bug,
  CheckCircle2,
  FileSearch,
  Fingerprint,
  Globe,
  Layers,
  RefreshCw,
  Shield,
  ShieldAlert,
  Siren,
  Terminal,
  Users,
} from "lucide-react";
import { cn } from "#/utils/utils";
import {
  EduBadge,
  EduButton,
  EduCard,
  EduEmpty,
  EduError,
  EduField,
  EduInput,
  EduSelect,
  EduSpinner,
  EduTextarea,
} from "#/components/features/education/edu-ui";
import {
  securityService,
  type AgentBinding,
  type DfirAgent,
  type SecGroup,
  type SecPlatform,
  type SwarmAgent,
} from "#/api/security/security.api";

type TabKey =
  | "dashibodi"
  | "zana"
  | "majukwaa"
  | "swarm"
  | "dfir"
  | "finshield"
  | "apiauth"
  | "gateprobe"
  | "kazi";

const TABS: { key: TabKey; label: string; icon: typeof Shield }[] = [
  { key: "dashibodi", label: "Dashibodi", icon: Shield },
  { key: "zana", label: "Zana (Kali)", icon: Terminal },
  { key: "majukwaa", label: "Majukwaa", icon: Layers },
  { key: "swarm", label: "ThreatSwarm", icon: Siren },
  { key: "dfir", label: "Forensics (DFIR)", icon: FileSearch },
  { key: "finshield", label: "FinShield", icon: Fingerprint },
  { key: "apiauth", label: "API Auth", icon: Globe },
  { key: "gateprobe", label: "GateProbe", icon: Bug },
  { key: "kazi", label: "Kazi + Ripoti", icon: Activity },
];

function useAsync<T>(fn: () => Promise<T>, deps: unknown[]) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const run = useCallback(() => {
    setLoading(true);
    setError(null);
    fn()
      .then(setData)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setLoading(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  useEffect(run, [run]);
  return { data, error, loading, reload: run };
}

function Kpi({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="rounded-md border border-border bg-surface-raised p-3">
      <div className="text-xs text-muted">{label}</div>
      <div className="text-lg font-semibold text-content">{value}</div>
    </div>
  );
}

function Row({ children }: { children: ReactNode }) {
  return (
    <div className="flex flex-wrap items-center gap-2 border-b border-border py-2 last:border-0">
      {children}
    </div>
  );
}

/* ------------------------- Dashibodi ------------------------- */

function DashboardTab() {
  const eng = useAsync(() => securityService.engineStatus(), []);
  const shield = useAsync(() => securityService.shield(), []);
  if (eng.loading) return <EduSpinner label="Inapakia injini ya usalama…" />;
  if (eng.error) return <EduError message={eng.error} />;
  const d = eng.data;
  return (
    <div className="flex flex-col gap-3">
      <EduCard testId="sec-status">
        <div className="mb-3 flex items-center gap-2">
          <Shield className="size-5 text-primary" aria-hidden />
          <h2 className="font-semibold text-content">{d?.engine ?? "akili-security"}</h2>
          <EduBadge>{d?.modules ?? 0} modules</EduBadge>
          <EduBadge>{d?.platforms ?? 0} majukwaa</EduBadge>
          <EduBadge>{d?.platform_commands ?? 0} amri</EduBadge>
        </div>
        <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
          <Kpi label="Wakala wa swarm" value={d?.swarm_agents ?? 0} />
          <Kpi label="Lugha za swarm" value={(d?.swarm_languages ?? []).length} />
          <Kpi label="Finshield ops" value={d?.finshield_ops ?? 0} />
          <Kpi label="Apiauth ops" value={(d?.apiauth_ops ?? []).length} />
        </div>
      </EduCard>

      <EduCard testId="sec-shield">
        <div className="flex flex-wrap items-center gap-2">
          <ShieldAlert className={cn("size-5", shield.data?.active ? "text-primary" : "text-muted")} aria-hidden />
          <h2 className="font-semibold text-content">Ngao ya AI (Threat Shield)</h2>
          <EduBadge>{shield.data?.active ? "inatumika" : "imezimwa"}</EduBadge>
          <EduBadge>{shield.data?.patterns ?? 0} patterns</EduBadge>
          <EduBadge>{shield.data?.total_events ?? 0} matukio</EduBadge>
        </div>
        <p className="mt-2 text-sm text-muted">
          AI haipotezi mamlaka kwa hacker yeyote: kila jaribio la kudanganya linazuiliwa,
          kuandikwa, na kujifunzwa kwenye Brain.
        </p>
      </EduCard>
    </div>
  );
}

/* --------------------------- Zana --------------------------- */

function ToolsTab() {
  const groups = useAsync(() => securityService.groups(), []);
  const avail = useAsync(() => securityService.availability(), []);
  const [open, setOpen] = useState<string | null>(null);
  if (groups.loading) return <EduSpinner label="Inapakia zana…" />;
  if (groups.error) return <EduError message={groups.error} />;
  const availMap = new Map((avail.data?.groups ?? []).map((g) => [g.id, g]));
  return (
    <div className="flex flex-col gap-3">
      <EduCard testId="sec-tools">
        <h2 className="mb-2 font-semibold text-content">
          Zana za usalama ({(groups.data?.groups ?? []).length} vikundi)
        </h2>
        {(groups.data?.groups ?? []).length === 0 ? (
          <EduEmpty>Hakuna vikundi vya zana.</EduEmpty>
        ) : (
          <div className="flex flex-col">
            {(groups.data?.groups ?? []).map((g: SecGroup) => {
              const av = availMap.get(g.id);
              const isOpen = open === g.id;
              return (
                <Row key={g.id}>
                  <button
                    type="button"
                    onClick={() => setOpen(isOpen ? null : g.id)}
                    className="flex flex-1 flex-wrap items-center gap-2 text-left"
                    data-testid={`sec-tool-${g.id}`}
                  >
                    <span className="font-medium text-content">{g.title}</span>
                    <EduBadge>{g.tool_count} zana</EduBadge>
                    <EduBadge>{g.tasks.length} kazi</EduBadge>
                    {av ? (
                      <EduBadge>
                        {av.tools_available}/{av.tools_total} zipo
                      </EduBadge>
                    ) : null}
                  </button>
                  {isOpen ? (
                    <div className="w-full pt-2 text-sm text-muted">
                      <div className="mb-1">{g.desc}</div>
                      <div className="mb-1">
                        <span className="font-medium text-content">Zana: </span>
                        {g.tools.join(", ") || "—"}
                      </div>
                      <div className="flex flex-col">
                        {g.tasks.map((t) => (
                          <div key={t.id} className="flex flex-wrap items-center gap-2 py-0.5">
                            <span className="text-content">{t.name_sw}</span>
                            {t.high_risk ? <EduBadge>hatari kubwa</EduBadge> : null}
                            {t.needs_target ? <EduBadge>inahitaji lengo</EduBadge> : null}
                          </div>
                        ))}
                      </div>
                    </div>
                  ) : null}
                </Row>
              );
            })}
          </div>
        )}
      </EduCard>
    </div>
  );
}

/* ------------------------- Majukwaa ------------------------- */

function PlatformsTab() {
  const summary = useAsync(() => securityService.platforms(), []);
  const full = useAsync(() => securityService.platformsFull(), []);
  const [sel, setSel] = useState<string>("");
  const [plat, setPlat] = useState<string>("kali");
  const [task, setTask] = useState("");
  const [target, setTarget] = useState("");
  const [plan, setPlan] = useState<Record<string, unknown> | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  // `/platforms` hutoa id halisi; `/platforms/full` hutoa amri. Huunganishwa kwa title.
  const summaries = summary.data?.platforms ?? [];
  const fullList: SecPlatform[] = full.data?.platforms ?? [];
  const titleOf = (id: string) =>
    summaries.find((p) => p.platform === id)?.title ?? id;
  const idOfTitle = (title: string) =>
    summaries.find((p) => p.title === title)?.platform ?? title;
  const activeId = sel || plat;
  const current = fullList.find((p) => p.title === titleOf(activeId));
  if (summary.loading || full.loading) return <EduSpinner label="Inapakia majukwaa…" />;
  if (summary.error) return <EduError message={summary.error} />;
  if (full.error) return <EduError message={full.error} />;
  return (
    <div className="flex flex-col gap-3">
      <EduCard testId="sec-platforms">
        <h2 className="mb-2 font-semibold text-content">
          Majukwaa ({summaries.length}) — katalogi ya amri
        </h2>
        <div className="flex flex-wrap gap-2">
          {summaries.map((p) => (
            <button
              key={p.platform}
              type="button"
              onClick={() => {
                setSel(p.platform);
                setPlat(p.platform);
              }}
              data-testid={`sec-plat-${p.platform}`}
              className={cn(
                "rounded-md border px-3 py-1.5 text-sm",
                activeId === p.platform
                  ? "border-primary text-primary"
                  : "border-border text-content hover:bg-surface-raised",
              )}
            >
              {p.title} <span className="text-xs text-muted">({p.commands})</span>
            </button>
          ))}
        </div>
      </EduCard>

      <EduCard testId="sec-platform-plan">
        <h2 className="mb-2 font-semibold text-content">Panga kazi (haitekelezi)</h2>
        <div className="grid gap-2 md:grid-cols-4">
          <EduField label="Jukwaa">
            <EduSelect value={plat} onChange={(e) => setPlat(e.target.value)} testId="sec-plan-platform">
              {summaries.map((p) => (
                <option key={p.platform} value={p.platform}>
                  {p.title}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Kazi">
            <EduInput value={task} onChange={(e) => setTask(e.target.value)} testId="sec-plan-task" />
          </EduField>
          <EduField label="Lengo">
            <EduInput value={target} onChange={(e) => setTarget(e.target.value)} testId="sec-plan-target" />
          </EduField>
          <div className="flex items-end">
            <EduButton
              testId="sec-plan-submit"
              onClick={() => {
                setMsg(null);
                securityService
                  .platformPlan(plat, task, target)
                  .then((r) => {
                    setPlan(r);
                    setMsg("Mpango umetengenezwa");
                  })
                  .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)));
              }}
            >
              Panga
            </EduButton>
          </div>
        </div>
        {msg ? <div className="mt-2 text-sm text-muted">{msg}</div> : null}
        {plan ? (
          <pre className="mt-2 max-h-72 overflow-auto rounded bg-surface-raised p-2 text-xs text-content">
            {JSON.stringify(plan, null, 2)}
          </pre>
        ) : null}
      </EduCard>

      {current ? (
        <EduCard testId="sec-platform-cmds">
          <h2 className="mb-2 font-semibold text-content">
            {current.title} — amri ({idOfTitle(current.title)})
          </h2>
          <div className="flex flex-col gap-3">
            {current.categories.map((c) => (
              <div key={c.id}>
                <div className="mb-1 text-sm font-medium text-content">{c.title}</div>
                <div className="flex flex-col">
                  {c.commands.map((cmd) => (
                    <Row key={`${c.id}-${cmd.name}`}>
                      <span className="w-40 shrink-0 font-mono text-xs text-content">{cmd.name}</span>
                      <span className="flex-1 text-sm text-muted">{cmd.desc}</span>
                      <code className="rounded bg-surface-raised px-1.5 py-0.5 text-xs text-content">
                        {cmd.usage}
                      </code>
                      {cmd.risk === "high" ? <EduBadge>hatari</EduBadge> : null}
                      {cmd.needs_root ? <EduBadge>root</EduBadge> : null}
                    </Row>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </EduCard>
      ) : null}
    </div>
  );
}

/* -------------------------- Swarm -------------------------- */

function SwarmTab() {
  const cat = useAsync(() => securityService.swarmCatalog(), []);
  const bind = useAsync(() => securityService.agentbind(), []);
  const [mode, setMode] = useState("recon");
  const [target, setTarget] = useState("");
  const [scope, setScope] = useState("");
  const [plan, setPlan] = useState<Record<string, unknown> | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const agents: SwarmAgent[] = cat.data?.agents ?? [];
  const bindings: AgentBinding[] = bind.data?.bindings ?? [];
  if (cat.loading) return <EduSpinner label="Inapakia threatswarm…" />;
  if (cat.error) return <EduError message={cat.error} />;
  return (
    <div className="flex flex-col gap-3">
      <EduCard testId="sec-swarm">
        <div className="mb-2 flex items-center gap-2">
          <Siren className="size-5 text-primary" aria-hidden />
          <h2 className="font-semibold text-content">ThreatSwarm — wakala {agents.length}</h2>
        </div>
        <div className="flex flex-col">
          {agents.map((a) => (
            <Row key={a.name}>
              <span className="w-40 shrink-0 font-medium text-content">{a.name}</span>
              <span className="w-48 shrink-0 text-sm text-content">{a.title}</span>
              <EduBadge>{a.class}</EduBadge>
              <span className="flex-1 text-sm text-muted">{a.description}</span>
            </Row>
          ))}
        </div>
      </EduCard>

      <EduCard testId="sec-swarm-plan">
        <h2 className="mb-2 font-semibold text-content">Panga kill-chain</h2>
        <div className="grid gap-2 md:grid-cols-4">
          <EduField label="Hali">
            <EduInput value={mode} onChange={(e) => setMode(e.target.value)} testId="sec-swarm-mode" />
          </EduField>
          <EduField label="Lengo">
            <EduInput value={target} onChange={(e) => setTarget(e.target.value)} testId="sec-swarm-target" />
          </EduField>
          <EduField label="Wigo (scope)">
            <EduInput value={scope} onChange={(e) => setScope(e.target.value)} testId="sec-swarm-scope" />
          </EduField>
          <div className="flex items-end">
            <EduButton
              testId="sec-swarm-submit"
              onClick={() => {
                setMsg(null);
                securityService
                  .swarmPlan(mode, target, scope)
                  .then((r) => {
                    setPlan(r);
                    setMsg("Mpango wa kill-chain umetengenezwa");
                  })
                  .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)));
              }}
            >
              Panga
            </EduButton>
          </div>
        </div>
        {msg ? <div className="mt-2 text-sm text-muted">{msg}</div> : null}
        {plan ? (
          <pre className="mt-2 max-h-72 overflow-auto rounded bg-surface-raised p-2 text-xs text-content">
            {JSON.stringify(plan, null, 2)}
          </pre>
        ) : null}
      </EduCard>

      <EduCard testId="sec-agentbind">
        <div className="mb-2 flex items-center gap-2">
          <Users className="size-5 text-primary" aria-hidden />
          <h2 className="font-semibold text-content">AgentBind — ufungaji wa wakala ({bindings.length})</h2>
        </div>
        <div className="flex flex-col">
          {bindings.map((b) => (
            <Row key={`${b.agent}-${b.task}`}>
              <span className="w-40 shrink-0 font-medium text-content">{b.agent}</span>
              <EduBadge>{b.class}</EduBadge>
              <EduBadge>{b.platform}</EduBadge>
              <span className="text-sm text-muted">{b.task}</span>
            </Row>
          ))}
        </div>
      </EduCard>
    </div>
  );
}

/* --------------------------- DFIR --------------------------- */

function DfirTab() {
  const f = useAsync(() => securityService.forensics(), []);
  if (f.loading) return <EduSpinner label="Inapakia forensics…" />;
  if (f.error) return <EduError message={f.error} />;
  const agents: DfirAgent[] = f.data?.dfir_agents ?? [];
  return (
    <EduCard testId="sec-dfir">
      <div className="mb-2 flex items-center gap-2">
        <FileSearch className="size-5 text-primary" aria-hidden />
        <h2 className="font-semibold text-content">
          Digital Forensics & Incident Response ({f.data?.dfir_agent_count ?? 0} wakala)
        </h2>
      </div>
      <div className="flex flex-col">
        {agents.map((a) => (
          <Row key={a.agent}>
            <span className="w-44 shrink-0 font-medium text-content">{a.agent}</span>
            <span className="w-56 shrink-0 text-sm text-content">{a.title}</span>
            <span className="flex-1 text-xs text-muted">
              zana: {a.tools.join(", ") || "—"}
            </span>
            <EduBadge>{a.tasks.length} kazi</EduBadge>
          </Row>
        ))}
      </div>
    </EduCard>
  );
}

/* ------------------------ FinShield ------------------------ */

function FinShieldTab() {
  const eng = useAsync(() => securityService.engineStatus(), []);
  const [op, setOp] = useState("luhn");
  const [data, setData] = useState("");
  const [out, setOut] = useState<Record<string, unknown> | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const ops = eng.data?.finshield_catalog?.ops ?? [];
  return (
    <div className="flex flex-col gap-3">
      <EduCard testId="sec-finshield">
        <div className="mb-2 flex items-center gap-2">
          <Fingerprint className="size-5 text-primary" aria-hidden />
          <h2 className="font-semibold text-content">FinShield — ulinzi wa kifedha</h2>
        </div>
        <div className="mb-2 flex flex-wrap gap-2">
          {ops.map((o) => (
            <button
              key={o}
              type="button"
              onClick={() => setOp(o)}
              className={cn(
                "rounded-md border px-2.5 py-1 text-xs",
                op === o ? "border-primary text-primary" : "border-border text-content",
              )}
            >
              {o}
            </button>
          ))}
        </div>
        <EduField label="Data (JSON au maandishi)">
          <EduTextarea
            value={data}
            onChange={(e) => setData(e.target.value)}
            testId="sec-fin-data"
            rows={4}
          />
        </EduField>
        <div className="mt-2 flex items-center gap-2">
          <EduButton
            testId="sec-fin-run"
            onClick={() => {
              setMsg(null);
              let parsed: unknown = data;
              try {
                parsed = JSON.parse(data);
              } catch {
                parsed = data;
              }
              securityService
                .finshield(op, parsed)
                .then((r) => {
                  setOut(r);
                  setMsg("Imekamilika");
                })
                .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)));
            }}
          >
            Endesha {op}
          </EduButton>
          {msg ? <span className="text-sm text-muted">{msg}</span> : null}
        </div>
        {out ? (
          <pre className="mt-2 max-h-72 overflow-auto rounded bg-surface-raised p-2 text-xs text-content">
            {JSON.stringify(out, null, 2)}
          </pre>
        ) : null}
      </EduCard>
    </div>
  );
}

/* ------------------------- API Auth ------------------------- */

function ApiAuthTab() {
  const eng = useAsync(() => securityService.engineStatus(), []);
  const [op, setOp] = useState("jwt");
  const [data, setData] = useState("");
  const [out, setOut] = useState<Record<string, unknown> | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const ops = eng.data?.apiauth_ops ?? [];
  return (
    <EduCard testId="sec-apiauth">
      <div className="mb-2 flex items-center gap-2">
        <Globe className="size-5 text-primary" aria-hidden />
        <h2 className="font-semibold text-content">Apiauth — uthibitisho wa API</h2>
      </div>
      <div className="mb-2 flex flex-wrap gap-2">
        {ops.map((o) => (
          <button
            key={o}
            type="button"
            onClick={() => setOp(o)}
            className={cn(
              "rounded-md border px-2.5 py-1 text-xs",
              op === o ? "border-primary text-primary" : "border-border text-content",
            )}
          >
            {o}
          </button>
        ))}
      </div>
      <EduField label="Data (JSON)">
        <EduTextarea
          value={data}
          onChange={(e) => setData(e.target.value)}
          testId="sec-api-data"
          rows={4}
        />
      </EduField>
      <div className="mt-2 flex items-center gap-2">
        <EduButton
          testId="sec-api-run"
          onClick={() => {
            setMsg(null);
            let parsed: unknown = data;
            try {
              parsed = JSON.parse(data);
            } catch {
              parsed = data;
            }
            securityService
              .apiauth(op, parsed)
              .then((r) => {
                setOut(r);
                setMsg("Imekamilika");
              })
              .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)));
          }}
        >
          Endesha {op}
        </EduButton>
        {msg ? <span className="text-sm text-muted">{msg}</span> : null}
      </div>
      {out ? (
        <pre className="mt-2 max-h-72 overflow-auto rounded bg-surface-raised p-2 text-xs text-content">
          {JSON.stringify(out, null, 2)}
        </pre>
      ) : null}
    </EduCard>
  );
}

/* ------------------------- GateProbe ------------------------- */

function GateProbeTab() {
  const [url, setUrl] = useState("");
  const [authorized, setAuthorized] = useState(false);
  const [out, setOut] = useState<Record<string, unknown> | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  return (
    <EduCard testId="sec-gateprobe">
      <div className="mb-2 flex items-center gap-2">
        <Bug className="size-5 text-primary" aria-hidden />
        <h2 className="font-semibold text-content">GateProbe — uchunguzi wa lango</h2>
      </div>
      <p className="mb-2 text-sm text-muted">
        Hufanya uchunguzi wa lango la API kwa ruhusa PEKEE. Bila uthibitisho wa
        umiliki, ombi hukataliwa.
      </p>
      <div className="grid gap-2 md:grid-cols-3">
        <EduField label="URL">
          <EduInput value={url} onChange={(e) => setUrl(e.target.value)} testId="sec-gate-url" />
        </EduField>
        <EduField label="Umeidhinishwa?">
          <EduSelect
            value={authorized ? "yes" : "no"}
            onChange={(e) => setAuthorized(e.target.value === "yes")}
            testId="sec-gate-auth"
          >
            <option value="no">Hapana</option>
            <option value="yes">Ndiyo (nina umiliki)</option>
          </EduSelect>
        </EduField>
        <div className="flex items-end">
          <EduButton
            testId="sec-gate-run"
            onClick={() => {
              setMsg(null);
              securityService
                .gateprobe(url, authorized)
                .then((r) => {
                  setOut(r);
                  setMsg("Imekamilika");
                })
                .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)));
            }}
          >
            Chunguza
          </EduButton>
        </div>
      </div>
      {msg ? <div className="mt-2 text-sm text-muted">{msg}</div> : null}
      {out ? (
        <pre className="mt-2 max-h-72 overflow-auto rounded bg-surface-raised p-2 text-xs text-content">
          {JSON.stringify(out, null, 2)}
        </pre>
      ) : null}
    </EduCard>
  );
}

/* --------------------- Kazi + Ripoti --------------------- */

function RunsTab() {
  const runs = useAsync(() => securityService.runs(), []);
  const [task, setTask] = useState("");
  const [target, setTarget] = useState("");
  const [approved, setApproved] = useState(false);
  const [out, setOut] = useState<Record<string, unknown> | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const reportUrl = useMemo(() => securityService.reportUrl(), []);
  return (
    <div className="flex flex-col gap-3">
      <EduCard testId="sec-run">
        <div className="mb-2 flex items-center gap-2">
          <Activity className="size-5 text-primary" aria-hidden />
          <h2 className="font-semibold text-content">Endesha kazi ya usalama</h2>
        </div>
        <div className="grid gap-2 md:grid-cols-4">
          <EduField label="Kazi">
            <EduInput value={task} onChange={(e) => setTask(e.target.value)} testId="sec-run-task" />
          </EduField>
          <EduField label="Lengo">
            <EduInput value={target} onChange={(e) => setTarget(e.target.value)} testId="sec-run-target" />
          </EduField>
          <EduField label="Idhini ya binadamu">
            <EduSelect
              value={approved ? "yes" : "no"}
              onChange={(e) => setApproved(e.target.value === "yes")}
              testId="sec-run-approve"
            >
              <option value="no">Hapana</option>
              <option value="yes">Ndiyo</option>
            </EduSelect>
          </EduField>
          <div className="flex items-end">
            <EduButton
              testId="sec-run-submit"
              onClick={() => {
                setMsg(null);
                securityService
                  .runTask(task, target, approved)
                  .then((r) => {
                    setOut(r);
                    setMsg("Kazi imeanza");
                    runs.reload();
                  })
                  .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)));
              }}
            >
              Endesha
            </EduButton>
          </div>
        </div>
        {msg ? <div className="mt-2 text-sm text-muted">{msg}</div> : null}
        {out ? (
          <pre className="mt-2 max-h-60 overflow-auto rounded bg-surface-raised p-2 text-xs text-content">
            {JSON.stringify(out, null, 2)}
          </pre>
        ) : null}
      </EduCard>

      <EduCard testId="sec-runs">
        <div className="mb-2 flex items-center gap-2">
          <CheckCircle2 className="size-5 text-primary" aria-hidden />
          <h2 className="font-semibold text-content">
            Kumbukumbu za kazi ({runs.data?.count ?? 0})
          </h2>
          <EduButton variant="ghost" testId="sec-runs-reload" onClick={runs.reload}>
            <RefreshCw className="size-4" aria-hidden />
          </EduButton>
          {reportUrl ? (
            <a
              href={reportUrl}
              target="_blank"
              rel="noreferrer"
              className="text-sm text-primary underline"
              data-testid="sec-report"
            >
              Ripoti ya HTML
            </a>
          ) : null}
        </div>
        {(runs.data?.runs ?? []).length === 0 ? (
          <EduEmpty>Hakuna kazi iliyotekelezwa bado.</EduEmpty>
        ) : (
          <div className="flex flex-col">
            {(runs.data?.runs ?? []).map((r) => (
              <Row key={r.id}>
                <span className="w-40 shrink-0 font-mono text-xs text-content">{r.task}</span>
                <span className="w-40 shrink-0 text-sm text-muted">{r.target}</span>
                <EduBadge>{r.status}</EduBadge>
                <span className="flex-1 text-xs text-muted">{r.at}</span>
              </Row>
            ))}
          </div>
        )}
      </EduCard>
    </div>
  );
}

/* --------------------------- Shell --------------------------- */

export default function CyberSecurityPage() {
  const [tab, setTab] = useState<TabKey>("dashibodi");
  return (
    <div className="flex flex-col gap-4" data-testid="cyber-security-page">
      <div className="flex flex-wrap items-center gap-2">
        <ShieldAlert className="size-6 text-primary" aria-hidden />
        <h1 className="text-xl font-semibold text-content">MTECHOS CYBER SECURITY</h1>
        <EduBadge>Kali · Windows · macOS · Android · UOS</EduBadge>
        <EduBadge>HITL kwa hatari kubwa</EduBadge>
      </div>
      <nav className="flex flex-wrap gap-1 border-b border-border">
        {TABS.map(({ key, label, icon: Icon }) => (
          <button
            key={key}
            type="button"
            onClick={() => setTab(key)}
            data-testid={`cyber-tab-${key}`}
            className={cn(
              "flex items-center gap-1.5 px-3 py-2 text-sm",
              tab === key
                ? "border-b-2 border-primary text-primary"
                : "text-muted hover:text-content",
            )}
          >
            <Icon className="size-4" aria-hidden />
            {label}
          </button>
        ))}
      </nav>
      <div className="pb-8">
        {tab === "dashibodi" ? <DashboardTab /> : null}
        {tab === "zana" ? <ToolsTab /> : null}
        {tab === "majukwaa" ? <PlatformsTab /> : null}
        {tab === "swarm" ? <SwarmTab /> : null}
        {tab === "dfir" ? <DfirTab /> : null}
        {tab === "finshield" ? <FinShieldTab /> : null}
        {tab === "apiauth" ? <ApiAuthTab /> : null}
        {tab === "gateprobe" ? <GateProbeTab /> : null}
        {tab === "kazi" ? <RunsTab /> : null}
      </div>
    </div>
  );
}

export const CYBER_TABS = TABS;

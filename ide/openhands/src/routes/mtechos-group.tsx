import { useCallback, useEffect, useState } from "react";
import type { ReactNode } from "react";
import { buildAgentCanvasPath } from "#/utils/base-path";
import {
  BadgeDollarSign,
  Banknote,
  Briefcase,
  Bot,
  Building2,
  CheckCircle2,
  FileSignature,
  MessageSquare,
  Palette,
  ShieldCheck,
  Users,
  Wrench,
} from "lucide-react";
import { cn } from "#/utils/utils";
import {
  EduBadge,
  EduButton,
  EduCard,
  EduCard3D,
  EduEmpty,
  EduError,
  EduField,
  EduInput,
  EduSelect,
  EduSpinner,
  EduTextarea,
} from "#/components/features/education/edu-ui";
import {
  agentCompanyService,
  ajiraService,
  biasharaService,
  escrowService,
  smsService,
  type Application,
  type BizRequest,
  type BizStatus,
  type CompanyAgent,
  type Employee,
  type JobRole,
  type Service,
  type Track,
} from "#/api/biashara/biashara.api";

type TabKey =
  | "dashibodi"
  | "huduma"
  | "wafanyakazi"
  | "idara"
  | "maombi"
  | "mikataba"
  | "malipo"
  | "ajira"
  | "wakala"
  | "sms"
  | "ufuatiliaji"
  | "chapa";

const TABS: { key: TabKey; label: string; icon: typeof Building2 }[] = [
  { key: "dashibodi", label: "Dashibodi", icon: Building2 },
  { key: "huduma", label: "Huduma", icon: Wrench },
  { key: "wafanyakazi", label: "Wafanyakazi", icon: Users },
  { key: "idara", label: "Idara (Usimamizi)", icon: Briefcase },
  { key: "maombi", label: "Maombi ya Wateja", icon: CheckCircle2 },
  { key: "mikataba", label: "Mikataba (ECDSA)", icon: FileSignature },
  { key: "malipo", label: "Malipo + Escrow", icon: Banknote },
  { key: "ajira", label: "Ajira + Tathmini", icon: Briefcase },
  { key: "wakala", label: "Kampuni ya Wakala", icon: Bot },
  { key: "sms", label: "SMS", icon: MessageSquare },
  { key: "ufuatiliaji", label: "Ufuatiliaji", icon: ShieldCheck },
  { key: "chapa", label: "Chapa (Logo)", icon: Palette },
];

/**
 * MTECHOS GROUP — kituo kimoja cha biashara: huduma, wafanyakazi, maombi ya
 * wateja yenye idhini ya admin na maendeleo yanayoonekana kwa mteja, mikataba
 * yenye sahihi ya ECDSA, malipo (Stripe USD / simu TZS) kupitia escrow, ajira
 * yenye tathmini, kampuni ya wakala 100, na SMS.
 */
export default function MtechosGroupRoute() {
  const [tab, setTab] = useState<TabKey>("dashibodi");
  const [status, setStatus] = useState<BizStatus | null>(null);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const s = await biasharaService.status();
        if (!cancelled) setStatus(s);
      } catch {
        /* takwimu si muhimu kwa matumizi ya tabs */
      }
    };
    void load();
    const timer = window.setInterval(load, 20000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [tab]);

  return (
    <div
      data-testid="mtechos-group-page"
      className="mx-auto flex h-full w-full max-w-6xl flex-col gap-4 overflow-y-auto p-4"
    >
      <header className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <Building2 className="size-6 text-primary" aria-hidden />
          <div>
            <h1 className="text-lg font-semibold text-content">MTECHOS GROUP</h1>
            <p className="text-xs text-muted">
              Huduma za teknolojia duniani — maombi, wafanyakazi, malipo, ajira na wakala
            </p>
          </div>
        </div>
        {status ? (
          <div className="flex flex-wrap items-center gap-1" data-testid="mtechos-status">
            <EduBadge>{status.services} huduma</EduBadge>
            <EduBadge>{status.employees} wafanyakazi</EduBadge>
            <EduBadge>{status.requests} maombi</EduBadge>
            <EduBadge>{status.pending_approval} yanasubiri idhini</EduBadge>
            <EduBadge>{status.regions} mikoa</EduBadge>
            <EduBadge>{status.countries} nchi</EduBadge>
            <a
              href={buildAgentCanvasPath("/mteja")}
              data-testid="mtechos-client-link"
              className="ml-1 text-xs text-primary hover:underline"
            >
              Ukurasa wa mteja
            </a>
          </div>
        ) : null}
      </header>

      <nav className="flex flex-wrap gap-1 border-b border-border pb-1">
        {TABS.map(({ key, label, icon: Icon }) => (
          <button
            key={key}
            type="button"
            data-testid={`mtechos-tab-${key}`}
            onClick={() => setTab(key)}
            className={cn(
              "inline-flex items-center gap-1.5 rounded-t-md px-3 py-1.5 text-sm font-medium transition",
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

      <div className="flex-1 pb-8">
        {tab === "dashibodi" ? <DashboardTab status={status} /> : null}
        {tab === "huduma" ? <ServicesTab /> : null}
        {tab === "wafanyakazi" ? <EmployeesTab /> : null}
        {tab === "idara" ? <DepartmentsTab /> : null}
        {tab === "maombi" ? <RequestsTab /> : null}
        {tab === "mikataba" ? <ContractsTab /> : null}
        {tab === "malipo" ? <PaymentsTab /> : null}
        {tab === "ajira" ? <JobsTab /> : null}
        {tab === "wakala" ? <AgentCompanyTab /> : null}
        {tab === "sms" ? <SmsTab /> : null}
        {tab === "ufuatiliaji" ? <CoverageTab /> : null}
        {tab === "chapa" ? <BrandingTab /> : null}
      </div>
    </div>
  );
}

function useAsync<T>(fn: () => Promise<T>, deps: unknown[] = []) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const reload = useCallback(() => {
    setLoading(true);
    setError(null);
    fn()
      .then((d) => setData(d))
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setLoading(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  useEffect(() => {
    reload();
  }, [reload]);
  return { data, error, loading, reload };
}

function Row({ children, testId }: { children: ReactNode; testId?: string }) {
  return (
    <div
      data-testid={testId}
      className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border bg-surface-raised px-3 py-2 text-sm"
    >
      {children}
    </div>
  );
}

const STATUS_SW: Record<string, string> = {
  submitted: "Imewasilishwa",
  approved: "Imeidhinishwa",
  rejected: "Imekataliwa",
  assigned: "Imepewa mfanyakazi",
  in_progress: "Inaendelea",
  review: "Ukaguzi",
  delivered: "Imewasilishwa",
  closed: "Imefungwa",
  cancelled: "Imesitishwa",
};

function StatusBadge({ status }: { status: string }) {
  return <EduBadge>{STATUS_SW[status] ?? status}</EduBadge>;
}

/* ---------------------------- Dashibodi ---------------------------- */

function DashboardTab({ status }: { status: BizStatus | null }) {
  const pay = useAsync(() => escrowService.status(), []);
  const comp = useAsync(() => agentCompanyService.status(), []);
  const jobs = useAsync(() => ajiraService.status(), []);
  const sms = useAsync(() => smsService.status(), []);
  if (!status) return <EduSpinner label="Inapakia takwimu…" />;
  return (
    <div className="grid gap-3 md:grid-cols-2">
      <EduCard testId="mtechos-card-biashara">
        <h2 className="mb-2 font-semibold text-content">Biashara</h2>
        <div className="grid grid-cols-2 gap-2 text-sm">
          <Kpi label="Huduma" value={status.services} />
          <Kpi label="Wafanyakazi" value={status.employees} />
          <Kpi label="Maombi" value={status.requests} />
          <Kpi label="Mikataba" value={status.contracts} />
          <Kpi label="Zana" value={status.tools} />
          <Kpi label="Sarafu" value={status.currencies} />
          <Kpi label="Mikoa" value={status.regions} />
          <Kpi label="Nchi" value={status.countries} />
        </div>
      </EduCard>
      <EduCard testId="mtechos-card-malipo">
        <h2 className="mb-2 font-semibold text-content">Malipo + Escrow</h2>
        {pay.loading ? <EduSpinner /> : null}
        {pay.error ? <EduError message={pay.error} /> : null}
        {pay.data ? (
          <div className="grid grid-cols-2 gap-2 text-sm">
            <Kpi label="Malipo" value={pay.data.payments} />
            <Kpi label="Yaliyolipwa" value={pay.data.paid} />
            <Kpi label="Escrow" value={pay.data.in_escrow} />
            <Kpi label="Yaliyotolewa" value={pay.data.released} />
            <div className="col-span-2">
              <EduBadge>
                Stripe: {pay.data.stripe_configured ? "iko tayari" : "haijawekwa"}
              </EduBadge>
            </div>
          </div>
        ) : null}
      </EduCard>
      <EduCard testId="mtechos-card-wakala">
        <h2 className="mb-2 font-semibold text-content">Kampuni ya Wakala</h2>
        {comp.data ? (
          <div className="grid grid-cols-2 gap-2 text-sm">
            <Kpi label="Wakala" value={comp.data.agents} />
            <Kpi label="Mameneja" value={comp.data.managers} />
            <Kpi label="Watekelezaji" value={comp.data.workers} />
            <Kpi label="Kazi" value={comp.data.tasks} />
            <Kpi label="Zinasubiri idhini" value={comp.data.pending_approval} />
            <Kpi label="Zilizokataliwa" value={comp.data.denied_runs} />
            <div className="col-span-2">
              <EduBadge>
                Ujisimamiaji: {comp.data.autonomy.enabled ? comp.data.autonomy.mode : "imezimwa"}
              </EduBadge>
            </div>
          </div>
        ) : (
          <EduSpinner />
        )}
      </EduCard>
      <EduCard testId="mtechos-card-ajira">
        <h2 className="mb-2 font-semibold text-content">Ajira na SMS</h2>
        <div className="grid grid-cols-2 gap-2 text-sm">
          <Kpi label="Nafasi wazi" value={jobs.data?.open_roles ?? 0} />
          <Kpi label="Maombi" value={jobs.data?.applications ?? 0} />
          <Kpi label="Tathmini" value={jobs.data?.assessments ?? 0} />
          <Kpi label="Maswali" value={jobs.data?.questions ?? 0} />
          <Kpi label="SMS zilizotumwa" value={sms.data?.sent ?? 0} />
          <Kpi label="SMS foleni" value={sms.data?.queued ?? 0} />
          <Kpi label="Amri za SMS" value={sms.data?.pending_commands ?? 0} />
          <Kpi label="Kiwango cha kufaulu" value={`${jobs.data?.pass_mark ?? 60}%`} />
        </div>
      </EduCard>
    </div>
  );
}

function Kpi({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="rounded-md border border-border bg-surface px-2.5 py-1.5">
      <div className="text-xs text-muted">{label}</div>
      <div className="font-semibold text-content">{value}</div>
    </div>
  );
}

/* ---------------------------- Huduma ---------------------------- */

function ServicesTab() {
  const [country, setCountry] = useState("TZ");
  const { data, error, loading } = useAsync(
    () => biasharaService.services({ country }),
    [country],
  );
  const [amount, setAmount] = useState("1000");
  const [from, setFrom] = useState("TZS");
  const [to, setTo] = useState("USD");
  const [conv, setConv] = useState<string | null>(null);
  return (
    <div className="flex flex-col gap-3">
      <EduCard>
        <div className="flex flex-wrap items-end gap-2">
          <EduField label="Nchi ya mteja" hint="Huduma za TZ (BRELA/TRA) hazionekani kwa wateja wa nje">
            <EduSelect value={country} onChange={(e) => setCountry(e.target.value)} testId="svc-country">
              <option value="TZ">Tanzania</option>
              <option value="US">Marekani</option>
              <option value="KE">Kenya</option>
              <option value="GB">Uingereza</option>
            </EduSelect>
          </EduField>
          <div className="flex items-end gap-1">
            <EduField label="Kiasi">
              <EduInput value={amount} onChange={(e) => setAmount(e.target.value)} className="w-28" />
            </EduField>
            <EduField label="Kutoka">
              <EduSelect value={from} onChange={(e) => setFrom(e.target.value)}>
                {["TZS", "USD", "EUR", "GBP"].map((c) => (
                  <option key={c}>{c}</option>
                ))}
              </EduSelect>
            </EduField>
            <EduField label="Kwenda">
              <EduSelect value={to} onChange={(e) => setTo(e.target.value)}>
                {["TZS", "USD", "EUR", "GBP"].map((c) => (
                  <option key={c}>{c}</option>
                ))}
              </EduSelect>
            </EduField>
            <EduButton
              testId="svc-convert"
              onClick={() => {
                biasharaService
                  .convert(amount, from, to)
                  .then((r) => setConv(`${r.amount} ${r.from} = ${r.converted} ${r.to}`))
                  .catch((e: unknown) => setConv(e instanceof Error ? e.message : String(e)));
              }}
            >
              Badilisha
            </EduButton>
          </div>
        </div>
        {conv ? <p className="mt-2 text-sm text-content" data-testid="svc-conv-result">{conv}</p> : null}
      </EduCard>

      {loading ? <EduSpinner label="Inapakia huduma…" /> : null}
      {error ? <EduError message={error} /> : null}
      <div className="grid gap-2 md:grid-cols-2 lg:grid-cols-3">
        {(data?.services ?? []).map((s: Service) => (
          <EduCard3D
            key={s.code}
            testId={`svc-${s.code}`}
            className="flex flex-col gap-1"
          >
            <div className="flex items-center justify-between gap-2">
              <span className="font-medium text-content">{s.name}</span>
              <EduBadge>{s.category}</EduBadge>
            </div>
            <span className="text-xs text-muted">{s.name_sw}</span>
            {s.audience === "tz" ? <EduBadge>Tanzania pekee</EduBadge> : null}
          </EduCard3D>
        ))}
      </div>
      {data && data.total === 0 ? <EduEmpty>Hakuna huduma kwa nchi hii.</EduEmpty> : null}
    </div>
  );
}

/* ---------------------------- Wafanyakazi ---------------------------- */

function EmployeesTab() {
  const { data, error, loading, reload } = useAsync(() => biasharaService.employees(), []);
  const tools = useAsync(() => biasharaService.tools(), []);
  const deps = useAsync(() => biasharaService.departments(), []);
  const [name, setName] = useState("");
  const [profession, setProfession] = useState("");
  const [level, setLevel] = useState("mid");
  const [department, setDepartment] = useState("ceo");
  const [phone, setPhone] = useState("");
  const [email, setEmail] = useState("");
  const [photo, setPhoto] = useState("");
  const [picked, setPicked] = useState<string[]>([]);
  const [msg, setMsg] = useState<string | null>(null);
  const [tasksFor, setTasksFor] = useState<string | null>(null);
  const tasks = useAsync(
    () =>
      tasksFor
        ? biasharaService.employeeTasks(tasksFor)
        : Promise.resolve(null),
    [tasksFor],
  );

  const toggle = (code: string) =>
    setPicked((p) => (p.includes(code) ? p.filter((x) => x !== code) : [...p, code]));

  return (
    <div className="flex flex-col gap-3">
      <EduCard testId="emp-form">
        <h2 className="mb-2 font-semibold text-content">Ongeza mfanyakazi</h2>
        <div className="grid gap-2 md:grid-cols-3">
          <EduField label="Jina kamili">
            <EduInput value={name} onChange={(e) => setName(e.target.value)} testId="emp-name" />
          </EduField>
          <EduField label="Taaluma">
            <EduInput value={profession} onChange={(e) => setProfession(e.target.value)} testId="emp-prof" />
          </EduField>
          <EduField label="Ngazi">
            <EduSelect value={level} onChange={(e) => setLevel(e.target.value)} testId="emp-level">
              {["intern", "junior", "mid", "senior", "lead", "executive"].map((l) => (
                <option key={l}>{l}</option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Idara">
            <EduSelect value={department} onChange={(e) => setDepartment(e.target.value)} testId="emp-dep">
              {(deps.data?.departments ?? []).map((d) => (
                <option key={d.code} value={d.code}>
                  {d.name}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Simu">
            <EduInput value={phone} onChange={(e) => setPhone(e.target.value)} testId="emp-phone" />
          </EduField>
          <EduField label="Barua pepe">
            <EduInput value={email} onChange={(e) => setEmail(e.target.value)} testId="emp-email" />
          </EduField>
          <EduField label="Picha (URL ya picha ya mfanyakazi)" hint="Itaonekana kwenye wasifu wa mfanyakazi">
            <EduInput value={photo} onChange={(e) => setPhoto(e.target.value)} testId="emp-photo" placeholder="https://…" />
          </EduField>
        </div>
        <div className="mt-3">
          <div className="mb-1 text-sm font-medium text-content">
            Zana za kazi ({picked.length} zimechaguliwa)
          </div>
          <div className="flex flex-wrap gap-1">
            {(tools.data?.tools ?? []).map((t) => (
              <button
                key={t.code}
                type="button"
                onClick={() => toggle(t.code)}
                data-testid={`emp-tool-${t.code}`}
                className={cn(
                  "rounded-full border px-2 py-0.5 text-xs transition",
                  picked.includes(t.code)
                    ? "border-primary bg-primary text-on-primary"
                    : "border-border text-muted hover:text-content",
                )}
              >
                {t.name}
              </button>
            ))}
          </div>
        </div>
        <div className="mt-3 flex items-center gap-2">
          <EduButton
            testId="emp-submit"
            onClick={() => {
              setMsg(null);
              biasharaService
                .addEmployee({ full_name: name, profession, level, department, phone, email, photo, tools: picked })
                .then((r) => {
                  setMsg(r.ok ? "Mfanyakazi ameongezwa" : r.error ?? "Kosa");
                  setName("");
                  setPicked([]);
                  setPhoto("");
                  reload();
                })
                .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)));
            }}
          >
            Ongeza
          </EduButton>
          {msg ? <span className="text-sm text-muted" data-testid="emp-msg">{msg}</span> : null}
        </div>
      </EduCard>

      {loading ? <EduSpinner label="Inapakia wafanyakazi…" /> : null}
      {error ? <EduError message={error} /> : null}
      <div className="flex flex-col gap-2">
        {(data?.employees ?? []).map((e: Employee) => (
          <Row key={e.id} testId={`emp-${e.id}`}>
            <div className="flex items-center gap-2">
              {e.photo ? (
                <img
                  src={e.photo}
                  alt={e.full_name}
                  className="size-8 rounded-full object-cover"
                  data-testid={`emp-photo-${e.id}`}
                />
              ) : (
                <div className="flex size-8 items-center justify-center rounded-full bg-surface-raised text-xs font-semibold text-muted">
                  {e.full_name.slice(0, 2).toUpperCase()}
                </div>
              )}
              <div>
                <div className="font-medium text-content">{e.full_name}</div>
                <div className="text-xs text-muted">
                  {e.profession || "—"} · {e.level}
                </div>
              </div>
            </div>
            <div className="flex flex-wrap items-center gap-1">
              {(e.tools ?? []).slice(0, 4).map((t) => (
                <EduBadge key={t}>{t}</EduBadge>
              ))}
              {(e.tools ?? []).length > 4 ? <EduBadge>+{e.tools.length - 4}</EduBadge> : null}
              <StatusBadge status={e.status} />
              <EduButton
                variant="ghost"
                testId={`emp-tasks-${e.id}`}
                onClick={() => setTasksFor(e.id)}
              >
                Kazi
              </EduButton>
            </div>
          </Row>
        ))}
        {data && data.employees.length === 0 ? (
          <EduEmpty>Hakuna wafanyakazi bado — ongeza wa kwanza juu.</EduEmpty>
        ) : null}
      </div>

      {tasksFor ? (
        <EduCard testId="emp-tasks-panel">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="font-semibold text-content">Kazi za mfanyakazi</h3>
            <EduButton variant="ghost" onClick={() => setTasksFor(null)}>
              Funga
            </EduButton>
          </div>
          {tasks.loading ? <EduSpinner label="Inapakia kazi…" /> : null}
          {tasks.error ? <EduError message={tasks.error} /> : null}
          {tasks.data ? (
            <>
              <div className="mb-2 flex flex-wrap items-center gap-2">
                <span className="font-medium text-content">
                  {tasks.data.employee.full_name}
                </span>
                <EduBadge>{tasks.data.employee.level}</EduBadge>
                <EduBadge>Zinazoendelea: {tasks.data.open}</EduBadge>
                <EduBadge>Zilizokamilika: {tasks.data.completed}</EduBadge>
              </div>
              <div className="flex flex-col gap-2">
                {tasks.data.tasks.map((t) => (
                  <Row key={t.id} testId={`emp-task-${t.ref}`}>
                    <div>
                      <div className="font-medium text-content">
                        {t.ref} · {t.title}
                      </div>
                      <div className="text-xs text-muted">{t.client_name}</div>
                    </div>
                    <div className="flex items-center gap-1">
                      <StatusBadge status={t.status} />
                      <EduBadge>{t.progress}%</EduBadge>
                    </div>
                  </Row>
                ))}
                {tasks.data.tasks.length === 0 ? (
                  <EduEmpty>Mfanyakazi huyu hana kazi bado.</EduEmpty>
                ) : null}
              </div>
            </>
          ) : null}
        </EduCard>
      ) : null}
    </div>
  );
}

/* ---------------------------- Maombi ---------------------------- */

function RequestsTab() {
  const { data, error, loading, reload } = useAsync(() => biasharaService.requests(), []);
  const services = useAsync(() => biasharaService.services(), []);
  const emps = useAsync(() => biasharaService.employees(), []);
  const [client, setClient] = useState("");
  const [phone, setPhone] = useState("");
  const [service, setService] = useState("sw_dev");
  const [title, setTitle] = useState("");
  const [amount, setAmount] = useState("500000");
  const [currency, setCurrency] = useState("TZS");
  const [country, setCountry] = useState("TZ");
  const [region, setRegion] = useState("DAR");
  const [msg, setMsg] = useState<string | null>(null);
  const [track, setTrack] = useState<Track | null>(null);

  const act = (fn: () => Promise<{ ok: boolean; error?: string }>) => {
    setMsg(null);
    fn()
      .then((r) => {
        if (!r.ok) setMsg(r.error ?? "Kosa");
        reload();
      })
      .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)));
  };

  return (
    <div className="flex flex-col gap-3">
      <EduCard testId="req-form">
        <h2 className="mb-2 font-semibold text-content">Ombi jipya la mteja</h2>
        <div className="grid gap-2 md:grid-cols-4">
          <EduField label="Mteja">
            <EduInput value={client} onChange={(e) => setClient(e.target.value)} testId="req-client" />
          </EduField>
          <EduField label="Simu">
            <EduInput value={phone} onChange={(e) => setPhone(e.target.value)} testId="req-phone" />
          </EduField>
          <EduField label="Huduma">
            <EduSelect value={service} onChange={(e) => setService(e.target.value)} testId="req-service">
              {(services.data?.services ?? []).map((s) => (
                <option key={s.code} value={s.code}>
                  {s.name}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Kichwa">
            <EduInput value={title} onChange={(e) => setTitle(e.target.value)} testId="req-title" />
          </EduField>
          <EduField label="Kiasi">
            <EduInput value={amount} onChange={(e) => setAmount(e.target.value)} testId="req-amount" />
          </EduField>
          <EduField label="Sarafu">
            <EduSelect value={currency} onChange={(e) => setCurrency(e.target.value)} testId="req-currency">
              <option value="TZS">TZS</option>
              <option value="USD">USD</option>
              <option value="EUR">EUR</option>
            </EduSelect>
          </EduField>
          <EduField label="Nchi">
            <EduSelect value={country} onChange={(e) => setCountry(e.target.value)} testId="req-country">
              <option value="TZ">Tanzania</option>
              <option value="US">Marekani</option>
              <option value="KE">Kenya</option>
            </EduSelect>
          </EduField>
          <EduField label="Mkoa (TZ)">
            <EduInput value={region} onChange={(e) => setRegion(e.target.value)} testId="req-region" />
          </EduField>
        </div>
        <div className="mt-3 flex items-center gap-2">
          <EduButton
            testId="req-submit"
            onClick={() => {
              setMsg(null);
              biasharaService
                .createRequest({
                  client_name: client,
                  client_phone: phone,
                  service,
                  title,
                  amount,
                  currency,
                  country,
                  region,
                })
                .then((r) => {
                  setMsg(r.ok ? `Ombi ${r.ref} limewasilishwa — linasubiri idhini` : r.error ?? "Kosa");
                  reload();
                })
                .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)));
            }}
          >
            Wasilisha ombi
          </EduButton>
          {msg ? <span className="text-sm text-muted" data-testid="req-msg">{msg}</span> : null}
        </div>
      </EduCard>

      {loading ? <EduSpinner label="Inapakia maombi…" /> : null}
      {error ? <EduError message={error} /> : null}
      <div className="flex flex-col gap-2">
        {(data?.requests ?? []).map((r: BizRequest) => (
          <EduCard key={r.id} testId={`req-${r.ref}`} className="flex flex-col gap-2">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div>
                <div className="font-medium text-content">
                  {r.ref} · {r.title}
                </div>
                <div className="text-xs text-muted">
                  {r.client_name} · {r.amount} {r.currency} · {r.country}
                  {r.region ? `/${r.region}` : ""}
                </div>
              </div>
              <div className="flex flex-wrap items-center gap-1">
                <StatusBadge status={r.status} />
                {r.needs_admin ? <EduBadge>Inahitaji idhini yako</EduBadge> : null}
                <EduBadge>{r.progress}%</EduBadge>
              </div>
            </div>
            <div className="h-1.5 w-full overflow-hidden rounded-full bg-surface-raised">
              <div className="h-full bg-primary" style={{ width: `${r.progress}%` }} />
            </div>
            <div className="flex flex-wrap items-center gap-1">
              {r.needs_admin ? (
                <>
                  <EduButton testId={`req-approve-${r.ref}`} onClick={() => act(() => biasharaService.approve(r.id))}>
                    Idhinisha
                  </EduButton>
                  <EduButton variant="ghost" onClick={() => act(() => biasharaService.reject(r.id))}>
                    Kataa
                  </EduButton>
                </>
              ) : null}
              {r.status === "approved" ? (
                <>
                  <EduButton
                    testId={`req-autoassign-${r.ref}`}
                    onClick={() =>
                      act(async () => {
                        const res = await biasharaService.autoAssign(r.id);
                        if (res.ok) {
                          setMsg(
                            `Imepewa ${res.assignee_name ?? res.assignee ?? "mfanyakazi"} moja kwa moja`,
                          );
                        }
                        return res;
                      })
                    }
                  >
                    Mpe mfanyakazi moja kwa moja
                  </EduButton>
                  <EduSelect
                    defaultValue=""
                    onChange={(e) => {
                      if (e.target.value) act(() => biasharaService.assign(r.id, e.target.value));
                    }}
                    testId={`req-assign-${r.ref}`}
                  >
                    <option value="">— au chagua mwenyewe —</option>
                    {(emps.data?.employees ?? []).map((emp) => (
                      <option key={emp.id} value={emp.id}>
                        {emp.full_name}
                      </option>
                    ))}
                  </EduSelect>
                </>
              ) : null}
              {r.status === "assigned" || r.status === "in_progress" ? (
                <EduButton
                  variant="ghost"
                  onClick={() =>
                    act(() =>
                      biasharaService.progress(r.id, {
                        progress: Math.min(100, r.progress + 25),
                        note: "Maendeleo yamepandishwa",
                        actor: "mfanyakazi",
                      }),
                    )
                  }
                >
                  +25% maendeleo
                </EduButton>
              ) : null}
              {r.status === "in_progress" ? (
                <EduButton variant="ghost" onClick={() => act(() => biasharaService.review(r.id))}>
                  Tuma kwa ukaguzi
                </EduButton>
              ) : null}
              {r.status === "review" ? (
                <EduButton testId={`req-deliver-${r.ref}`} onClick={() => act(() => biasharaService.deliver(r.id))}>
                  Wasilisha kwa mteja
                </EduButton>
              ) : null}
              {r.status === "delivered" ? (
                <EduButton variant="ghost" onClick={() => act(() => biasharaService.close(r.id))}>
                  Funga
                </EduButton>
              ) : null}
              <EduButton
                variant="ghost"
                testId={`req-track-${r.ref}`}
                onClick={() =>
                  biasharaService
                    .track(r.ref)
                    .then(setTrack)
                    .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)))
                }
              >
                Maendeleo ya mteja
              </EduButton>
            </div>
          </EduCard>
        ))}
        {data && data.requests.length === 0 ? <EduEmpty>Hakuna maombi bado.</EduEmpty> : null}
      </div>

      {track ? (
        <EduCard testId="req-track-panel">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="font-semibold text-content">
              {track.ref} — {track.stage}
            </h3>
            <EduButton variant="ghost" onClick={() => setTrack(null)}>
              Funga
            </EduButton>
          </div>
          <ol className="flex flex-col gap-2">
            {track.timeline.map((ev, i) => (
              <li key={i} className="flex gap-2 text-sm">
                <span className="mt-1 size-2 shrink-0 rounded-full bg-primary" />
                <div>
                  <div className="text-content">{ev.note}</div>
                  <div className="text-xs text-muted">
                    {ev.actor} · {ev.progress}% · {ev.at.slice(0, 16)}
                  </div>
                </div>
              </li>
            ))}
          </ol>
        </EduCard>
      ) : null}
    </div>
  );
}

/* ---------------------------- Mikataba ---------------------------- */

function ContractsTab() {
  const { data, error, loading, reload } = useAsync(() => biasharaService.contracts(), []);
  const reqs = useAsync(() => biasharaService.requests(), []);
  const [request, setRequest] = useState("");
  const [title, setTitle] = useState("");
  const [amount, setAmount] = useState("");
  const [msg, setMsg] = useState<string | null>(null);
  const [verify, setVerify] = useState<Record<string, boolean | string>>({});

  return (
    <div className="flex flex-col gap-3">
      <EduCard testId="contract-form">
        <h2 className="mb-2 font-semibold text-content">Mkataba mpya</h2>
        <div className="grid gap-2 md:grid-cols-3">
          <EduField label="Ombi">
            <EduSelect value={request} onChange={(e) => setRequest(e.target.value)} testId="con-request">
              <option value="">— chagua —</option>
              {(reqs.data?.requests ?? []).map((r) => (
                <option key={r.id} value={r.ref}>
                  {r.ref} · {r.title}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Kichwa">
            <EduInput value={title} onChange={(e) => setTitle(e.target.value)} testId="con-title" />
          </EduField>
          <EduField label="Kiasi (acha tupu = cha ombi)">
            <EduInput value={amount} onChange={(e) => setAmount(e.target.value)} testId="con-amount" />
          </EduField>
        </div>
        <div className="mt-3 flex items-center gap-2">
          <EduButton
            testId="con-submit"
            onClick={() => {
              setMsg(null);
              biasharaService
                .createContract({ request, title, amount })
                .then((r) => {
                  setMsg(r.ok ? `Mkataba umeundwa (muhuri ${r.seal_hash?.slice(0, 12)}…)` : r.error ?? "Kosa");
                  reload();
                })
                .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)));
            }}
          >
            Unda mkataba
          </EduButton>
          {msg ? <span className="text-sm text-muted" data-testid="con-msg">{msg}</span> : null}
        </div>
      </EduCard>

      {loading ? <EduSpinner /> : null}
      {error ? <EduError message={error} /> : null}
      <div className="flex flex-col gap-2">
        {(data?.contracts ?? []).map((c) => (
          <Row key={c.id} testId={`con-${c.id}`}>
            <div>
              <div className="font-medium text-content">{c.title}</div>
              <div className="text-xs text-muted">
                {c.amount} {c.currency} · {c.status}
              </div>
            </div>
            <div className="flex flex-wrap items-center gap-1">
              <EduButton
                variant="ghost"
                onClick={() =>
                  biasharaService
                    .signContract(c.id, "admin")
                    .then((r) => setMsg(r.ok ? `Sahihi ya admin (${r.alg})` : r.error ?? "Kosa"))
                    .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)))
                }
              >
                Sahihi (admin)
              </EduButton>
              <EduButton
                variant="ghost"
                onClick={() =>
                  biasharaService
                    .signContract(c.id, "client")
                    .then((r) => setMsg(r.ok ? `Sahihi ya mteja (${r.alg})` : r.error ?? "Kosa"))
                    .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)))
                }
              >
                Sahihi (mteja)
              </EduButton>
              <EduButton
                variant="ghost"
                testId={`con-verify-${c.id}`}
                onClick={() =>
                  biasharaService
                    .verifyContract(c.id)
                    .then((r) =>
                      setVerify((v) => ({
                        ...v,
                        [c.id]: r.fully_signed
                          ? `Sahihi zote ni halali (${r.alg})`
                          : `Mteja: ${r.client_signature_valid ? "✓" : "✗"} · Admin: ${r.admin_signature_valid ? "✓" : "✗"}`,
                      })),
                    )
                    .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)))
                }
              >
                Thibitisha
              </EduButton>
              {verify[c.id] ? <EduBadge>{verify[c.id]}</EduBadge> : null}
            </div>
          </Row>
        ))}
        {data && data.contracts.length === 0 ? <EduEmpty>Hakuna mikataba bado.</EduEmpty> : null}
      </div>
    </div>
  );
}

/* ---------------------------- Malipo ---------------------------- */

function PaymentsTab() {
  const pay = useAsync(() => escrowService.status(), []);
  const gw = useAsync(() => escrowService.gateways(), []);
  const payments = useAsync(() => escrowService.payments(), []);
  const ledger = useAsync(() => escrowService.ledger(), []);
  const reqs = useAsync(() => biasharaService.requests(), []);
  const [request, setRequest] = useState("");
  const [client, setClient] = useState("");
  const [amount, setAmount] = useState("100");
  const [currency, setCurrency] = useState("USD");
  const [msg, setMsg] = useState<string | null>(null);
  const [url, setUrl] = useState<string | null>(null);

  const reloadAll = () => {
    payments.reload();
    ledger.reload();
    pay.reload();
  };

  return (
    <div className="flex flex-col gap-3">
      <EduCard testId="pay-status">
        <div className="flex flex-wrap items-center gap-2">
          <EduBadge>
            Stripe: {gw.data?.stripe_configured ? "iko tayari" : "haijawekwa"}
          </EduBadge>
          <EduBadge>Escrow: {pay.data?.in_escrow ?? "0.00"}</EduBadge>
          <EduBadge>Yaliyotolewa: {pay.data?.released ?? "0.00"}</EduBadge>
          {pay.data?.company_account?.configured === false ? (
            <EduBadge>akaunti ya kampuni haijawekwa</EduBadge>
          ) : null}
        </div>
      </EduCard>

      <EduCard testId="pay-checkout">
        <h2 className="mb-2 font-semibold text-content">Anzisha malipo (Stripe USD / simu TZS)</h2>
        <div className="grid gap-2 md:grid-cols-4">
          <EduField label="Ombi">
            <EduSelect value={request} onChange={(e) => setRequest(e.target.value)} testId="pay-request">
              <option value="">— chagua —</option>
              {(reqs.data?.requests ?? []).map((r) => (
                <option key={r.id} value={r.ref}>
                  {r.ref}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Mteja">
            <EduInput value={client} onChange={(e) => setClient(e.target.value)} testId="pay-client" />
          </EduField>
          <EduField label="Kiasi">
            <EduInput value={amount} onChange={(e) => setAmount(e.target.value)} testId="pay-amount" />
          </EduField>
          <EduField label="Sarafu">
            <EduSelect value={currency} onChange={(e) => setCurrency(e.target.value)} testId="pay-currency">
              <option value="TZS">TZS (simu/benki)</option>
              <option value="USD">USD (Stripe)</option>
              <option value="EUR">EUR (Stripe)</option>
              <option value="GBP">GBP (Stripe)</option>
              <option value="KES">KES (Stripe)</option>
              <option value="ZAR">ZAR (Stripe)</option>
              <option value="INR">INR (Stripe)</option>
            </EduSelect>
          </EduField>
        </div>
        <div className="mt-3 flex items-center gap-2">
          <EduButton
            testId="pay-submit"
            onClick={() => {
              setMsg(null);
              setUrl(null);
              escrowService
                .checkout({ request, client_name: client, amount, currency })
                .then((r) => {
                  setMsg(r.ok ? r.note ?? "Malipo yameanzishwa" : r.error ?? "Kosa");
                  if (r.checkout_url) setUrl(r.checkout_url);
                  reloadAll();
                })
                .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)));
            }}
          >
            Anzisha
          </EduButton>
          {msg ? <span className="text-sm text-muted" data-testid="pay-msg">{msg}</span> : null}
        </div>
        {url ? (
          <a
            href={url}
            target="_blank"
            rel="noreferrer"
            className="mt-2 inline-block text-sm text-primary underline"
            data-testid="pay-url"
          >
            Fungua ukurasa wa malipo wa Stripe
          </a>
        ) : null}
      </EduCard>

      <EduCard testId="pay-release">
        <h2 className="mb-2 font-semibold text-content">Toa pesa kutoka escrow (baada ya kazi)</h2>
        <div className="flex flex-wrap items-end gap-2">
          <EduField label="Ombi">
            <EduSelect
              value={request}
              onChange={(e) => setRequest(e.target.value)}
              testId="release-request"
            >
              <option value="">— chagua —</option>
              {(reqs.data?.requests ?? []).map((r) => (
                <option key={r.id} value={r.ref}>
                  {r.ref} · {r.status}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduButton
            testId="release-btn"
            onClick={() =>
              escrowService
                .release(request)
                .then((r) => {
                  setMsg(r.ok ? `Pesa imetolewa kwenda akaunti ya kampuni (${r.amount})` : r.error ?? "Kosa");
                  reloadAll();
                })
                .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)))
            }
          >
            Toa kwa kampuni
          </EduButton>
          <EduButton
            variant="ghost"
            onClick={() =>
              escrowService
                .refund(request)
                .then((r) => {
                  setMsg(r.ok ? `Pesa imerudishwa kwa mteja (${r.amount})` : r.error ?? "Kosa");
                  reloadAll();
                })
                .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)))
            }
          >
            Rudisha kwa mteja
          </EduButton>
        </div>
      </EduCard>

      <div className="grid gap-3 md:grid-cols-2">
        <EduCard>
          <h3 className="mb-2 font-semibold text-content">Malipo</h3>
          {(payments.data?.payments ?? []).map((p) => (
            <Row key={p.id}>
              <span className="text-content">
                {p.client_name} · {p.amount} {p.currency}
              </span>
              <span className="flex items-center gap-1">
                <EduBadge>{p.gateway}</EduBadge>
                <StatusBadge status={p.status} />
              </span>
            </Row>
          ))}
          {payments.data && payments.data.payments.length === 0 ? <EduEmpty>Hakuna malipo.</EduEmpty> : null}
        </EduCard>
        <EduCard>
          <h3 className="mb-2 font-semibold text-content">Escrow ledger</h3>
          {(ledger.data?.ledger ?? []).map((l) => (
            <Row key={l.id}>
              <span className="text-content">
                {l.kind} · {l.amount} {l.currency}
              </span>
              <EduBadge>{l.status}</EduBadge>
            </Row>
          ))}
          {ledger.data && ledger.data.ledger.length === 0 ? <EduEmpty>Ledger ni tupu.</EduEmpty> : null}
        </EduCard>
      </div>
    </div>
  );
}

/* ---------------------------- Ajira ---------------------------- */

function JobsTab() {
  const roles = useAsync(() => ajiraService.roles(), []);
  const apps = useAsync(() => ajiraService.applications(), []);
  const deps = useAsync(() => biasharaService.departments(), []);
  const tools = useAsync(() => biasharaService.tools(), []);
  const [title, setTitle] = useState("");
  const [department, setDepartment] = useState("employee");
  const [level, setLevel] = useState("mid");
  const [salaryMin, setSalaryMin] = useState("");
  const [salaryMax, setSalaryMax] = useState("");
  const [msg, setMsg] = useState<string | null>(null);
  const [picked, setPicked] = useState<string[]>([]);
  const [theory, setTheory] = useState<Record<string, { id: string; questions: { id: string; question: string; options: string[] }[] }>>({});
  const [answers, setAnswers] = useState<Record<string, string[]>>({});

  const refresh = () => {
    roles.reload();
    apps.reload();
  };

  return (
    <div className="flex flex-col gap-3">
      <EduCard testId="job-form">
        <h2 className="mb-2 font-semibold text-content">Tangaza nafasi ya kazi</h2>
        <div className="grid gap-2 md:grid-cols-4">
          <EduField label="Kichwa">
            <EduInput value={title} onChange={(e) => setTitle(e.target.value)} testId="job-title" />
          </EduField>
          <EduField label="Idara">
            <EduSelect value={department} onChange={(e) => setDepartment(e.target.value)} testId="job-dep">
              {(deps.data?.departments ?? []).map((d) => (
                <option key={d.code} value={d.code}>
                  {d.name}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Ngazi">
            <EduSelect value={level} onChange={(e) => setLevel(e.target.value)}>
              {["intern", "junior", "mid", "senior", "lead"].map((l) => (
                <option key={l}>{l}</option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Mshahara (chini)">
            <EduInput value={salaryMin} onChange={(e) => setSalaryMin(e.target.value)} />
          </EduField>
          <EduField label="Mshahara (juu)">
            <EduInput value={salaryMax} onChange={(e) => setSalaryMax(e.target.value)} />
          </EduField>
        </div>
        <div className="mt-3 flex items-center gap-2">
          <EduButton
            testId="job-submit"
            onClick={() => {
              setMsg(null);
              ajiraService
                .addRole({ title, department, level, salary_min: salaryMin, salary_max: salaryMax })
                .then((r) => {
                  setMsg(r.ok ? "Nafasi imetangazwa" : r.error ?? "Kosa");
                  setTitle("");
                  refresh();
                })
                .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)));
            }}
          >
            Tangaza
          </EduButton>
          {msg ? <span className="text-sm text-muted" data-testid="job-msg">{msg}</span> : null}
        </div>
      </EduCard>

      <EduCard>
        <h3 className="mb-2 font-semibold text-content">Nafasi ({roles.data?.roles.length ?? 0})</h3>
        {(roles.data?.roles ?? []).map((r: JobRole) => (
          <Row key={r.id} testId={`role-${r.code}`}>
            <div>
              <div className="font-medium text-content">{r.title}</div>
              <div className="text-xs text-muted">
                {r.level} · {r.location} · {r.salary_min}–{r.salary_max} {r.currency}
              </div>
            </div>
            <div className="flex items-center gap-1">
              <StatusBadge status={r.status} />
              <EduButton
                variant="ghost"
                onClick={() =>
                  ajiraService.closeRole(r.id).then(refresh).catch(() => undefined)
                }
              >
                Funga
              </EduButton>
            </div>
          </Row>
        ))}
        {roles.data && roles.data.roles.length === 0 ? <EduEmpty>Hakuna nafasi.</EduEmpty> : null}
      </EduCard>

      <EduCard>
        <h3 className="mb-2 font-semibold text-content">Maombi ya ajira</h3>
        {(apps.data?.applications ?? []).map((a: Application) => (
          <EduCard key={a.id} testId={`app-${a.ref}`} className="mb-2 flex flex-col gap-2">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div>
                <div className="font-medium text-content">
                  {a.ref} · {a.full_name}
                </div>
                <div className="text-xs text-muted">
                  nadharia {a.score_theory.toFixed(0)}% · vitendo {a.score_practical.toFixed(0)}% ·
                  jumla {a.total_score.toFixed(0)}%
                </div>
              </div>
              <div className="flex items-center gap-1">
                <StatusBadge status={a.status} />
                {a.passed ? <EduBadge>Amefaulu</EduBadge> : null}
              </div>
            </div>
            <div className="flex flex-wrap items-center gap-1">
              {a.status === "submitted" ? (
                <EduButton variant="ghost" onClick={() => ajiraService.setStage(a.id, "screening").then(refresh)}>
                  Screening
                </EduButton>
              ) : null}
              {a.status === "screening" ? (
                <EduButton
                  testId={`app-theory-${a.ref}`}
                  onClick={() =>
                    ajiraService
                      .startTheory(a.id)
                      .then((r) => {
                        if (r.assessment_id && r.questions) {
                          setTheory((t) => ({ ...t, [a.id]: { id: r.assessment_id!, questions: r.questions! } }));
                          setAnswers((s) => ({ ...s, [a.id]: r.questions!.map(() => "") }));
                        } else {
                          setMsg(r.error ?? "Kosa");
                        }
                      })
                      .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)))
                  }
                >
                  Anzisha nadharia
                </EduButton>
              ) : null}
              {a.status === "theory" ? (
                <EduButton
                  variant="ghost"
                  onClick={() =>
                    ajiraService
                      .assignPractical(a.id, "Tengeneza suluhisho la kiufundi kwa kazi halisi")
                      .then((r) => setMsg(r.ok ? "Kazi ya vitendo imetolewa" : r.error ?? "Kosa"))
                      .then(refresh)
                  }
                >
                  Toa kazi ya vitendo
                </EduButton>
              ) : null}
              {a.status === "practical" ? (
                <EduButton variant="ghost" onClick={() => ajiraService.setStage(a.id, "interview").then(refresh)}>
                  Mwito wa ana kwa ana
                </EduButton>
              ) : null}
              {a.status === "interview" ? (
                <EduButton variant="ghost" onClick={() => ajiraService.setStage(a.id, "offered").then(refresh)}>
                  Mpe nafasi
                </EduButton>
              ) : null}
              {a.status === "offered" ? (
                <EduButton
                  testId={`app-hire-${a.ref}`}
                  onClick={() =>
                    ajiraService
                      .hire(a.id, { department, tools: picked, level })
                      .then((r) => {
                        setMsg(r.ok ? `Ameajiriwa — mfanyakazi ${r.employee_id}` : r.error ?? "Kosa");
                        refresh();
                      })
                      .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)))
                  }
                >
                  Ajiri
                </EduButton>
              ) : null}
              <EduButton variant="ghost" onClick={() => ajiraService.reject(a.id).then(refresh)}>
                Kataa
              </EduButton>
            </div>
            {theory[a.id] ? (
              <div className="mt-1 flex flex-col gap-2 rounded-md border border-border bg-surface-raised p-2">
                {theory[a.id].questions.map((q, i) => (
                  <div key={q.id} className="text-sm">
                    <div className="text-content">
                      {i + 1}. {q.question}
                    </div>
                    <div className="mt-1 flex flex-wrap gap-1">
                      {q.options.map((o) => (
                        <button
                          key={o}
                          type="button"
                          onClick={() =>
                            setAnswers((s) => {
                              const copy = [...(s[a.id] ?? [])];
                              copy[i] = o;
                              return { ...s, [a.id]: copy };
                            })
                          }
                          className={cn(
                            "rounded-md border px-2 py-0.5 text-xs",
                            answers[a.id]?.[i] === o
                              ? "border-primary bg-primary text-on-primary"
                              : "border-border text-muted hover:text-content",
                          )}
                        >
                          {o}
                        </button>
                      ))}
                    </div>
                  </div>
                ))}
                <EduButton
                  testId={`app-theory-submit-${a.ref}`}
                  onClick={() =>
                    ajiraService
                      .submitTheory(a.id, theory[a.id].id, answers[a.id] ?? [])
                      .then((r) => {
                        setMsg(
                          r.ok
                            ? `Nadharia: ${r.score.toFixed(0)}% — ${r.passed ? "amefaulu" : "hajafaulu"}`
                            : r.error ?? "Kosa",
                        );
                        refresh();
                      })
                      .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)))
                  }
                >
                  Wasilisha nadharia
                </EduButton>
              </div>
            ) : null}
          </EduCard>
        ))}
        {apps.data && apps.data.applications.length === 0 ? <EduEmpty>Hakuna maombi.</EduEmpty> : null}
      </EduCard>

      <EduCard>
        <h3 className="mb-1 font-semibold text-content">Zana za kumpa aliyeajiriwa</h3>
        <div className="flex flex-wrap gap-1">
          {(tools.data?.tools ?? []).map((t) => (
            <button
              key={t.code}
              type="button"
              onClick={() =>
                setPicked((p) => (p.includes(t.code) ? p.filter((x) => x !== t.code) : [...p, t.code]))
              }
              className={cn(
                "rounded-full border px-2 py-0.5 text-xs",
                picked.includes(t.code)
                  ? "border-primary bg-primary text-on-primary"
                  : "border-border text-muted hover:text-content",
              )}
            >
              {t.name}
            </button>
          ))}
        </div>
      </EduCard>
    </div>
  );
}

/* ---------------------------- Kampuni ya Wakala ---------------------------- */

function AgentCompanyTab() {
  const org = useAsync(() => agentCompanyService.org(), []);
  const tasks = useAsync(() => agentCompanyService.tasks(), []);
  const tools = useAsync(() => biasharaService.tools(), []);
  const [selected, setSelected] = useState<CompanyAgent | null>(null);
  const [detail, setDetail] = useState<{ tools: string[] } | null>(null);
  const [taskTitle, setTaskTitle] = useState("");
  const [taskTool, setTaskTool] = useState("");
  const [taskAgent, setTaskAgent] = useState("");
  const [msg, setMsg] = useState<string | null>(null);

  const refresh = () => {
    org.reload();
    tasks.reload();
  };

  const pick = (a: CompanyAgent) => {
    setSelected(a);
    agentCompanyService
      .agent(a.id)
      .then((r) => setDetail({ tools: r.agent.tools }))
      .catch(() => setDetail(null));
  };

  return (
    <div className="flex flex-col gap-3">
      <EduCard testId="agent-company-status">
        <div className="flex flex-wrap items-center gap-2">
          <EduBadge>Wakala {org.data?.total ?? 0}</EduBadge>
          <EduBadge>Mameneja {org.data?.managers.length ?? 0}</EduBadge>
          <EduBadge>Watekelezaji {org.data?.workers.length ?? 0}</EduBadge>
          <EduButton
            variant="ghost"
            testId="autonomy-on"
            onClick={() =>
              agentCompanyService
                .setAutonomy(true, "supervised", 8)
                .then((r) => setMsg(`Ujisimamiaji: ${r.autonomy.mode}`))
                .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)))
            }
          >
            Washa ujisimamiaji
          </EduButton>
          <EduButton
            variant="ghost"
            onClick={() =>
              agentCompanyService
                .setAutonomy(false)
                .then(() => setMsg("Ujisimamiaji umezimwa"))
                .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)))
            }
          >
            Zima
          </EduButton>
          {msg ? <span className="text-sm text-muted">{msg}</span> : null}
        </div>
      </EduCard>

      <EduCard testId="task-form">
        <h2 className="mb-2 font-semibold text-content">Kazi mpya (inasubiri idhini yako)</h2>
        <div className="grid gap-2 md:grid-cols-3">
          <EduField label="Kichwa">
            <EduInput value={taskTitle} onChange={(e) => setTaskTitle(e.target.value)} testId="task-title" />
          </EduField>
          <EduField label="Wakala">
            <EduSelect value={taskAgent} onChange={(e) => setTaskAgent(e.target.value)} testId="task-agent">
              <option value="">— chagua —</option>
              {(org.data?.workers ?? []).slice(0, 30).map((w) => (
                <option key={w.id} value={w.id}>
                  {w.id} · {w.specialty}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Zana">
            <EduSelect value={taskTool} onChange={(e) => setTaskTool(e.target.value)} testId="task-tool">
              <option value="">— chagua —</option>
              {(tools.data?.tools ?? []).map((t) => (
                <option key={t.code} value={t.code}>
                  {t.name}
                </option>
              ))}
            </EduSelect>
          </EduField>
        </div>
        <div className="mt-3">
          <EduButton
            testId="task-submit"
            onClick={() =>
              agentCompanyService
                .createTask({ title: taskTitle, agent: taskAgent, tool: taskTool })
                .then((r) => {
                  setMsg(r.ok ? `Kazi ${r.ref} imeundwa — idhinisha` : r.error ?? "Kosa");
                  setTaskTitle("");
                  refresh();
                })
                .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)))
            }
          >
            Unda kazi
          </EduButton>
        </div>
      </EduCard>

      <EduCard>
        <h3 className="mb-2 font-semibold text-content">Kazi za wakala</h3>
        {(tasks.data?.tasks ?? []).map((t) => (
          <Row key={t.id} testId={`task-${t.ref}`}>
            <div>
              <div className="font-medium text-content">
                {t.ref} · {t.title}
              </div>
              <div className="text-xs text-muted">
                zana: {t.tool_code || "—"} · {t.priority}
              </div>
            </div>
            <div className="flex items-center gap-1">
              <StatusBadge status={t.status} />
              {t.status === "pending" ? (
                <EduButton
                  testId={`task-approve-${t.ref}`}
                  onClick={() => agentCompanyService.approveTask(t.id).then(refresh)}
                >
                  Idhinisha
                </EduButton>
              ) : null}
              {t.can_start ? (
                <EduButton
                  variant="ghost"
                  testId={`task-run-${t.ref}`}
                  onClick={() =>
                    agentCompanyService
                      .runTask(t.id)
                      .then((r) => {
                        setMsg(r.ok ? `Kazi imekamilika: ${r.result}` : r.error ?? "Kosa");
                        refresh();
                      })
                      .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)))
                  }
                >
                  Kimbiza
                </EduButton>
              ) : null}
              {t.status === "done" ? (
                <EduButton variant="ghost" onClick={() => agentCompanyService.reviewTask(t.id).then(refresh)}>
                  Kagua
                </EduButton>
              ) : null}
            </div>
          </Row>
        ))}
        {tasks.data && tasks.data.tasks.length === 0 ? <EduEmpty>Hakuna kazi bado.</EduEmpty> : null}
      </EduCard>

      <div className="grid gap-3 md:grid-cols-2">
        <EduCard>
          <h3 className="mb-2 font-semibold text-content">Mameneja</h3>
          {(org.data?.managers ?? []).map((m) => (
            <Row key={m.id} testId={`mgr-${m.id}`}>
              <button type="button" className="text-left" onClick={() => pick(m)}>
                <div className="font-medium text-content">{m.name}</div>
                <div className="text-xs text-muted">
                  {m.id} · {m.department}
                </div>
              </button>
              <EduBadge>{m.tasks_done} kazi</EduBadge>
            </Row>
          ))}
        </EduCard>
        <EduCard>
          <h3 className="mb-2 font-semibold text-content">
            Watekelezaji ({org.data?.workers.length ?? 0})
          </h3>
          <div className="flex max-h-72 flex-wrap gap-1 overflow-y-auto">
            {(org.data?.workers ?? []).map((w) => (
              <button
                key={w.id}
                type="button"
                onClick={() => pick(w)}
                data-testid={`agent-${w.id}`}
                className={cn(
                  "rounded-md border px-2 py-1 text-xs",
                  selected?.id === w.id
                    ? "border-primary text-primary"
                    : "border-border text-muted hover:text-content",
                )}
              >
                {w.id} · {w.specialty}
              </button>
            ))}
          </div>
        </EduCard>
      </div>

      {selected && detail ? (
        <EduCard testId="agent-detail">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="font-semibold text-content">
              {selected.id} · {selected.name}
            </h3>
            <EduButton variant="ghost" onClick={() => setSelected(null)}>
              Funga
            </EduButton>
          </div>
          <div className="mb-2 text-xs text-muted">
            idara {selected.department} · ngazi {selected.level} · meneja {selected.manager_id || "—"}
          </div>
          <div className="flex flex-wrap gap-1">
            {detail.tools.map((t) => (
              <EduBadge key={t}>{t}</EduBadge>
            ))}
          </div>
        </EduCard>
      ) : null}
    </div>
  );
}

/* ---------------------------- SMS ---------------------------- */

function SmsTab() {
  const st = useAsync(() => smsService.status(), []);
  const outbox = useAsync(() => smsService.outbox(), []);
  const inbox = useAsync(() => smsService.inbox(), []);
  const cmds = useAsync(() => smsService.commands(), []);
  const [to, setTo] = useState("");
  const [body, setBody] = useState("");
  const [msg, setMsg] = useState<string | null>(null);

  const refresh = () => {
    outbox.reload();
    inbox.reload();
    cmds.reload();
    st.reload();
  };

  return (
    <div className="flex flex-col gap-3">
      <EduCard testId="sms-status">
        <div className="flex flex-wrap items-center gap-2">
          <EduBadge>Mtoa huduma: {st.data?.configured ? "amewekwa" : "hajawekwa"}</EduBadge>
          <EduBadge>Foleni {st.data?.queued ?? 0}</EduBadge>
          <EduBadge>Zilizotumwa {st.data?.sent ?? 0}</EduBadge>
          <EduBadge>Zilizoshindikana {st.data?.failed ?? 0}</EduBadge>
          <EduBadge>Amri zinazosubiri {st.data?.pending_commands ?? 0}</EduBadge>
        </div>
        {st.data && !st.data.configured ? (
          <p className="mt-2 text-xs text-muted">
            Ujumbe huingia kwenye foleni. Weka <code>SMS_API_URL</code> na <code>SMS_API_KEY</code> ili
            utumwe kwa kweli.
          </p>
        ) : null}
      </EduCard>

      <EduCard testId="sms-send">
        <h2 className="mb-2 font-semibold text-content">Tuma SMS</h2>
        <div className="grid gap-2 md:grid-cols-2">
          <EduField label="Kwa namba">
            <EduInput value={to} onChange={(e) => setTo(e.target.value)} testId="sms-to" />
          </EduField>
          <EduField label="Ujumbe">
            <EduInput value={body} onChange={(e) => setBody(e.target.value)} testId="sms-body" />
          </EduField>
        </div>
        <div className="mt-3 flex items-center gap-2">
          <EduButton
            testId="sms-submit"
            onClick={() =>
              smsService
                .send(to, body, "info")
                .then((r) => {
                  setMsg(r.ok ? r.note : r.error ?? "Kosa");
                  refresh();
                })
                .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)))
            }
          >
            Tuma
          </EduButton>
          {msg ? <span className="text-sm text-muted" data-testid="sms-msg">{msg}</span> : null}
        </div>
      </EduCard>

      <EduCard>
        <h3 className="mb-2 font-semibold text-content">Amri zilizoingia (APPROVE / REJECT / SIGN)</h3>
        {(cmds.data?.commands ?? []).map((c) => (
          <Row key={c.id} testId={`sms-cmd-${c.id}`}>
            <div>
              <div className="font-medium text-content">
                {c.action.toUpperCase()} {c.ref}
              </div>
              <div className="text-xs text-muted">
                kutoka {c.from} · {c.raw}
              </div>
            </div>
            <div className="flex items-center gap-1">
              <StatusBadge status={c.status} />
              {c.status === "pending" ? (
                <EduButton
                  testId={`sms-apply-${c.id}`}
                  onClick={() =>
                    smsService
                      .applyCommand(c.id)
                      .then((r) => {
                        setMsg(r.ok ? `Amri imetekelezwa: ${r.action} ${r.ref}` : r.error ?? "Kosa");
                        refresh();
                      })
                      .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)))
                  }
                >
                  Tekeleza
                </EduButton>
              ) : null}
            </div>
          </Row>
        ))}
        {cmds.data && cmds.data.commands.length === 0 ? (
          <EduEmpty>Hakuna amri zilizoingia.</EduEmpty>
        ) : null}
      </EduCard>

      <div className="grid gap-3 md:grid-cols-2">
        <EduCard>
          <h3 className="mb-2 font-semibold text-content">Zilizotumwa</h3>
          {(outbox.data?.messages ?? []).slice(0, 20).map((m) => (
            <Row key={m.id}>
              <span className="text-content">{m.to}</span>
              <span className="flex items-center gap-1">
                <span className="text-xs text-muted">{m.kind}</span>
                <StatusBadge status={m.status} />
              </span>
            </Row>
          ))}
          {outbox.data && outbox.data.messages.length === 0 ? <EduEmpty>Hakuna.</EduEmpty> : null}
        </EduCard>
        <EduCard>
          <h3 className="mb-2 font-semibold text-content">Zilizoingia</h3>
          {(inbox.data?.inbox ?? []).slice(0, 20).map((m) => (
            <Row key={m.id}>
              <span className="text-content">{m.from}</span>
              <span className="text-xs text-muted">{m.body.slice(0, 40)}</span>
            </Row>
          ))}
          {inbox.data && inbox.data.inbox.length === 0 ? <EduEmpty>Hakuna.</EduEmpty> : null}
        </EduCard>
      </div>
    </div>
  );
}

/* ---------------------------- Idara (Usimamizi) ---------------------------- */

/**
 * Idara tano za kampuni (CEO, Employee, Finance, Marketing, HR). CEO ndiye
 * mmiliki (Admin Mkuu). Kila idara huonyesha wafanyakazi wake na ngazi zao —
 * hivyo admin huona kwa haraka nani yupo wapi na nani anahitaji kuajiriwa.
 */
function DepartmentsTab() {
  const deps = useAsync(() => biasharaService.departments(), []);
  const emps = useAsync(() => biasharaService.employees(), []);
  if (deps.loading || emps.loading) return <EduSpinner label="Inapakia idara…" />;
  if (deps.error) return <EduError message={deps.error} />;
  if (emps.error) return <EduError message={emps.error} />;
  const list = deps.data?.departments ?? [];
  const all = emps.data?.employees ?? [];
  const byDep = (code: string) =>
    all.filter((e) => e.department_id === code || e.department_id === idFor(list, code));
  return (
    <div className="flex flex-col gap-3">
      <EduCard testId="deps-card">
        <div className="mb-2 flex flex-wrap items-center gap-2">
          <EduBadge>{list.length} idara</EduBadge>
          <EduBadge>{all.length} wafanyakazi</EduBadge>
        </div>
        <div className="grid gap-2 md:grid-cols-2">
          {list.map((d) => {
            const members = byDep(d.code);
            return (
              <div
                key={d.id}
                data-testid={`dep-${d.code}`}
                className="rounded-md border border-border bg-surface-raised p-3"
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="font-semibold text-content">{d.name}</span>
                  <EduBadge>{d.tier}</EduBadge>
                </div>
                <div className="mt-1 text-xs text-muted">
                  {members.length} wafanyakazi
                </div>
                <div className="mt-2 flex flex-wrap gap-1">
                  {members.map((e) => (
                    <span
                      key={e.id}
                      data-testid={`dep-${d.code}-emp-${e.id}`}
                      className="rounded border border-border px-2 py-0.5 text-xs text-content"
                    >
                      {e.full_name}
                      <span className="ml-1 text-muted">({e.level})</span>
                    </span>
                  ))}
                  {members.length === 0 ? (
                    <span className="text-xs text-muted">Hakuna mfanyakazi bado.</span>
                  ) : null}
                </div>
              </div>
            );
          })}
        </div>
      </EduCard>
    </div>
  );
}

function idFor(
  list: { id: string; code: string }[],
  code: string,
): string {
  return list.find((d) => d.code === code)?.id ?? "";
}

/* ---------------------------- Ufuatiliaji ---------------------------- */

function CoverageTab() {
  const { data, error, loading } = useAsync(() => biasharaService.coverage(), []);
  const audit = useAsync(() => biasharaService.audit(), []);
  const [kind, setKind] = useState<"regions" | "countries">("regions");
  if (loading) return <EduSpinner label="Inapakia ufuatiliaji…" />;
  if (error) return <EduError message={error} />;
  const list = kind === "regions" ? data?.regions ?? [] : data?.countries ?? [];
  return (
    <div className="flex flex-col gap-3">
      <EduCard>
        <div className="mb-2 flex flex-wrap items-center gap-2">
          <EduBadge>{data?.regions_total ?? 0} mikoa ya Tanzania</EduBadge>
          <EduBadge>{data?.countries_total ?? 0} nchi</EduBadge>
          <EduButton
            variant={kind === "regions" ? "primary" : "ghost"}
            onClick={() => setKind("regions")}
          >
            Mikoa
          </EduButton>
          <EduButton
            variant={kind === "countries" ? "primary" : "ghost"}
            onClick={() => setKind("countries")}
          >
            Nchi
          </EduButton>
        </div>
        <div className="flex flex-wrap gap-1">
          {list.map((r) => (
            <span
              key={r.code}
              className="rounded-md border border-border px-2 py-1 text-xs text-content"
              data-testid={`cov-${r.code}`}
            >
              {r.name}
              {r.requests > 0 ? <span className="ml-1 text-primary">·{r.requests}</span> : null}
            </span>
          ))}
        </div>
      </EduCard>
      <EduCard>
        <h3 className="mb-2 font-semibold text-content">Kumbukumbu ya vitendo (audit)</h3>
        {(audit.data?.audit ?? []).slice(0, 40).map((a, i) => (
          <Row key={i}>
            <span className="text-content">
              {a.actor} · {a.action}
            </span>
            <span className="text-xs text-muted">
              {a.detail} · {a.at.slice(0, 16)}
            </span>
          </Row>
        ))}
        {audit.data && audit.data.audit.length === 0 ? <EduEmpty>Hakuna vitendo bado.</EduEmpty> : null}
      </EduCard>
    </div>
  );
}

/* ---------------------------- Chapa (Logo) ---------------------------- */

function BrandingTab() {
  const { data, error, loading, reload } = useAsync(() => biasharaService.branding(), []);
  const [name, setName] = useState("");
  const [tagline, setTagline] = useState("");
  const [primary, setPrimary] = useState("#FF6B2C");
  const [accent, setAccent] = useState("#22D3EE");
  const [surface, setSurface] = useState("#130A07");
  const [msg, setMsg] = useState<string | null>(null);
  const [preview, setPreview] = useState<string | null>(null);
  const [seeded, setSeeded] = useState(false);

  useEffect(() => {
    if (data?.branding && !seeded) {
      setName(data.branding.company_name);
      setTagline(data.branding.tagline);
      setPrimary(data.branding.primary_color);
      setAccent(data.branding.accent_color);
      setSurface(data.branding.surface_color);
      setPreview(data.branding.logo || null);
      setSeeded(true);
    }
  }, [data, seeded]);

  const upload = (file: File) => {
    if (file.size > 512 * 1024) {
      setMsg("Picha ni kubwa mno (kikomo 512 KB)");
      return;
    }
    const reader = new FileReader();
    reader.onload = () => {
      const uri = String(reader.result ?? "");
      biasharaService
        .setLogo(uri, "logo")
        .then((r) => {
          setMsg(r.ok ? `Logo imehifadhiwa (${r.bytes} bytes)` : r.error ?? "Kosa");
          if (r.ok) setPreview(uri);
          reload();
        })
        .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)));
    };
    reader.readAsDataURL(file);
  };

  if (loading) return <EduSpinner label="Inapakia chapa…" />;
  if (error) return <EduError message={error} />;

  return (
    <div className="flex flex-col gap-3">
      <EduCard testId="branding-preview">
        <div
          className="flex items-center gap-3 rounded-md p-3"
          style={{ background: surface, color: "#fff" }}
        >
          {preview ? (
            <img src={preview} alt="Logo" className="size-12 rounded-md object-contain" data-testid="branding-logo-img" />
          ) : (
            <div
              className="flex size-12 items-center justify-center rounded-md text-lg font-bold"
              style={{ background: primary }}
              data-testid="branding-logo-placeholder"
            >
              {(name || "M").slice(0, 1)}
            </div>
          )}
          <div>
            <div className="text-base font-semibold" data-testid="branding-name">
              {name || "MTECHOS GROUP"}
            </div>
            <div className="text-xs opacity-80" data-testid="branding-tagline">
              {tagline || "Teknolojia ya Dunia, Kasi ya Kipekee"}
            </div>
          </div>
          <span className="ml-auto rounded-full px-2 py-0.5 text-xs" style={{ background: accent, color: "#000" }}>
            {primary}
          </span>
        </div>
      </EduCard>

      <EduCard testId="branding-form">
        <h2 className="mb-2 font-semibold text-content">Chapa ya kampuni (Admin Mkuu)</h2>
        <div className="grid gap-2 md:grid-cols-2">
          <EduField label="Jina la kampuni">
            <EduInput value={name} onChange={(e) => setName(e.target.value)} testId="branding-input-name" />
          </EduField>
          <EduField label="Kauli mbiu">
            <EduInput value={tagline} onChange={(e) => setTagline(e.target.value)} testId="branding-input-tagline" />
          </EduField>
          <EduField label="Rangi kuu">
            <div className="flex items-center gap-2">
              <input
                type="color"
                value={primary}
                onChange={(e) => setPrimary(e.target.value)}
                data-testid="branding-input-primary"
                className="h-8 w-12 rounded border border-border bg-transparent"
              />
              <EduInput value={primary} onChange={(e) => setPrimary(e.target.value)} className="w-28" />
            </div>
          </EduField>
          <EduField label="Rangi ya pili">
            <div className="flex items-center gap-2">
              <input
                type="color"
                value={accent}
                onChange={(e) => setAccent(e.target.value)}
                data-testid="branding-input-accent"
                className="h-8 w-12 rounded border border-border bg-transparent"
              />
              <EduInput value={accent} onChange={(e) => setAccent(e.target.value)} className="w-28" />
            </div>
          </EduField>
          <EduField label="Rangi ya mandhari">
            <EduInput value={surface} onChange={(e) => setSurface(e.target.value)} testId="branding-input-surface" />
          </EduField>
          <EduField label="Logo (PNG/JPEG/SVG, ≤512 KB)">
            <input
              type="file"
              accept="image/png,image/jpeg,image/svg+xml,image/webp"
              data-testid="branding-file"
              onChange={(e) => {
                const f = e.target.files?.[0];
                if (f) upload(f);
              }}
              className="text-sm text-muted"
            />
          </EduField>
        </div>
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <EduButton
            testId="branding-save"
            onClick={() => {
              setMsg(null);
              biasharaService
                .setBranding({
                  company_name: name,
                  tagline,
                  primary_color: primary,
                  accent_color: accent,
                  surface_color: surface,
                })
                .then((r) => {
                  setMsg(r.ok ? `Imehifadhiwa: ${r.changed?.join(", ")}` : r.error ?? "Kosa");
                  reload();
                })
                .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)));
            }}
          >
            Hifadhi chapa
          </EduButton>
          <EduButton
            variant="ghost"
            testId="branding-reset"
            onClick={() =>
              biasharaService
                .resetBranding()
                .then((r) => {
                  setMsg(r.ok ? r.note ?? "Imerudishwa" : "Kosa");
                  setSeeded(false);
                  reload();
                })
                .catch((e: unknown) => setMsg(e instanceof Error ? e.message : String(e)))
            }
          >
            Rudisha default
          </EduButton>
          {msg ? <span className="text-sm text-muted" data-testid="branding-msg">{msg}</span> : null}
        </div>
      </EduCard>
    </div>
  );
}

export const MTECHOS_TABS = TABS;

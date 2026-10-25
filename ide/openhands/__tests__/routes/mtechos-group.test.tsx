import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import MtechosGroupRoute from "#/routes/mtechos-group";

const fetchMock = vi.fn();

vi.mock("#/api/agent-server-config", async (importOriginal) => ({
  ...(await importOriginal<typeof import("#/api/agent-server-config")>()),
  getAgentServerBaseUrl: () => "http://agent.example.com",
  getAgentServerHeaders: () => ({}),
}));

const statusPayload = {
  ok: true,
  services: 24,
  employees: 3,
  requests: 2,
  contracts: 1,
  currencies: 7,
  tools: 20,
  regions: 31,
  countries: 14,
  pending_approval: 1,
  active: 1,
  completed: 0,
};

const requestPayload = {
  id: "req-1",
  ref: "REQ-2026-000001",
  client_name: "Asha",
  service_id: "SVC-sw_dev",
  title: "Tovuti ya duka",
  country: "TZ",
  region: "DAR",
  currency: "TZS",
  amount: "500,000.00",
  amount_cents: 50_000_000,
  status: "submitted",
  progress: 0,
  priority: "normal",
  assignee_id: "",
  created_at: "2026-10-23T07:00:00Z",
  needs_admin: true,
  in_flight: false,
};

const trackPayload = {
  ok: true,
  ref: "REQ-2026-000001",
  title: "Tovuti ya duka",
  status: "in_progress",
  progress: 50,
  stage: "Inaendelea (50%)",
  timeline: [
    { actor: "mteja", kind: "created", note: "Ombi limewasilishwa", progress: 0, at: "2026-10-23T07:00:00Z" },
    { actor: "admin", kind: "approved", note: "Idhini ya admin", progress: 5, at: "2026-10-23T07:05:00Z" },
  ],
};

const employeePayload = {
  id: "emp-1",
  user_id: "",
  full_name: "Juma Mwakyusa",
  photo: "",
  profession: "Mhandisi wa Programu",
  level: "senior",
  department_id: "DEP-ceo",
  phone: "0712345678",
  email: "juma@mtechos.tz",
  tools: ["ide_code_run", "ide_build"],
  status: "active",
};

const toolsPayload = {
  total: 2,
  tools: [
    { code: "ide_code_run", name: "Kukimbiza Code", category: "software" },
    { code: "secpack", name: "Pakiti ya Ulinzi", category: "security" },
  ],
};

function routeByUrl(url: string): unknown {
  const u = String(url);
  if (u.includes("/api/biashara/status")) return statusPayload;
  if (u.includes("/api/biashara/requests")) return { ok: true, requests: [requestPayload] };
  if (u.includes("/api/biashara/employees")) return { ok: true, employees: [employeePayload] };
  if (u.includes("/api/biashara/tools")) return toolsPayload;
  if (u.includes("/api/biashara/departments"))
    return {
      ok: true,
      departments: [
        { id: "DEP-ceo", code: "ceo", name: "CEO Management", tier: "executive" },
        { id: "DEP-emp", code: "employee", name: "Employee Management", tier: "people" },
        { id: "DEP-fin", code: "finance", name: "Finance Management", tier: "finance" },
        { id: "DEP-mkt", code: "marketing", name: "Marketing", tier: "growth" },
        { id: "DEP-hr", code: "hr", name: "HR Management", tier: "people" },
      ],
    };
  if (u.includes("/api/biashara/services")) return { ok: true, total: 0, services: [] };
  if (u.includes("/api/biashara/contracts")) return { ok: true, contracts: [] };
  if (u.includes("/api/biashara/coverage"))
    return { ok: true, regions_total: 31, countries_total: 14, regions: [], countries: [] };
  if (u.includes("/api/biashara/audit")) return { ok: true, audit: [] };
  if (u.includes("/api/biashara/branding"))
    return {
      ok: true,
      branding: {
        company_name: "MTECHOS GROUP",
        tagline: "Teknolojia ya Dunia",
        primary_color: "#FF6B2C",
        accent_color: "#22D3EE",
        surface_color: "#130A07",
        logo: "",
        favicon: "",
      },
    };
  if (u.includes("/api/biashara/request/") && u.includes("/track")) return trackPayload;
  if (u.includes("/api/biashara/request/") && u.includes("/auto-assign"))
    return { ok: true, id: "req-1", assignee: "emp-1", assignee_name: "Juma Mwakyusa", status: "assigned" };
  if (u.includes("/api/biashara/employee/") && u.includes("/tasks"))
    return {
      ok: true,
      employee: { id: "emp-1", full_name: "Juma Mwakyusa", level: "senior", tools: ["ide_code_run"] },
      open: 1,
      completed: 0,
      tasks: [{ ...requestPayload, status: "assigned", assignee_id: "emp-1", progress: 10 }],
    };
  if (u.includes("/api/escrow/status"))
    return { ok: true, stripe_configured: false, company_account: {}, payments: 0, paid: 0, in_escrow: "0.00", released: "0.00" };
  if (u.includes("/api/escrow/gateways")) return { ok: true, stripe_configured: false, gateways: [] };
  if (u.includes("/api/escrow/payments")) return { ok: true, payments: [] };
  if (u.includes("/api/escrow/ledger")) return { ok: true, ledger: [] };
  if (u.includes("/api/ajira/status"))
    return { ok: true, open_roles: 0, applications: 0, assessments: 0, questions: 0, pass_mark: 60, stages: [] };
  if (u.includes("/api/ajira/roles")) return { ok: true, roles: [] };
  if (u.includes("/api/ajira/applications")) return { ok: true, applications: [] };
  if (u.includes("/api/agents/company/status"))
    return {
      ok: true,
      agents: 100,
      managers: 6,
      workers: 94,
      busy: 0,
      tasks: 0,
      pending_approval: 0,
      denied_runs: 0,
      autonomy: { enabled: false, mode: "supervised", max_concurrent: 8 },
    };
  if (u.includes("/api/agents/company/org")) return { ok: true, managers: [], workers: [], total: 0 };
  if (u.includes("/api/agents/company/tasks")) return { ok: true, tasks: [] };
  if (u.includes("/api/sms/status"))
    return { configured: false, sender: "", admin_phone: "", queued: 0, sent: 0, failed: 0, inbox: 0, pending_commands: 0 };
  if (u.includes("/api/sms/outbox")) return { ok: true, messages: [] };
  if (u.includes("/api/sms/inbox")) return { ok: true, inbox: [] };
  if (u.includes("/api/sms/commands")) return { ok: true, commands: [] };
  return {};
}

describe("MTECHOS GROUP route", () => {
  beforeEach(() => {
    fetchMock.mockReset();
    fetchMock.mockImplementation((url: string) =>
      Promise.resolve({
        ok: true,
        status: 200,
        json: async () => routeByUrl(url),
      }),
    );
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("renders the dashboard with live business counters", async () => {
    render(<MtechosGroupRoute />);
    expect(screen.getByTestId("mtechos-group-page")).toBeInTheDocument();
    await waitFor(() =>
      expect(screen.getByTestId("mtechos-status")).toHaveTextContent("24 huduma"),
    );
    expect(screen.getByTestId("mtechos-status")).toHaveTextContent("31 mikoa");
    expect(screen.getByTestId("mtechos-card-wakala")).toHaveTextContent("100");
  });

  it("shows admin approval controls on a pending request", async () => {
    const user = userEvent.setup();
    render(<MtechosGroupRoute />);
    await user.click(screen.getByTestId("mtechos-tab-maombi"));
    const card = await screen.findByTestId("req-REQ-2026-000001");
    expect(card).toHaveTextContent("Inahitaji idhini yako");
    expect(screen.getByTestId("req-approve-REQ-2026-000001")).toBeInTheDocument();
  });

  it("reveals the client-facing progress timeline", async () => {
    const user = userEvent.setup();
    render(<MtechosGroupRoute />);
    await user.click(screen.getByTestId("mtechos-tab-maombi"));
    await user.click(await screen.findByTestId("req-track-REQ-2026-000001"));
    const panel = await screen.findByTestId("req-track-panel");
    expect(panel).toHaveTextContent("Inaendelea (50%)");
    expect(panel).toHaveTextContent("Idhini ya admin");
  });

  it("lists employees with their granted tools", async () => {
    const user = userEvent.setup();
    render(<MtechosGroupRoute />);
    await user.click(screen.getByTestId("mtechos-tab-wafanyakazi"));
    const row = await screen.findByTestId("emp-emp-1");
    expect(row).toHaveTextContent("Juma Mwakyusa");
    expect(row).toHaveTextContent("ide_code_run");
  });

  it("exposes admin branding controls with the default logo", async () => {
    const user = userEvent.setup();
    render(<MtechosGroupRoute />);
    await user.click(screen.getByTestId("mtechos-tab-chapa"));
    const preview = await screen.findByTestId("branding-preview");
    expect(preview).toHaveTextContent("MTECHOS GROUP");
    expect(screen.getByTestId("branding-logo-placeholder")).toBeInTheDocument();
    expect(screen.getByTestId("branding-save")).toBeInTheDocument();
    expect(screen.getByTestId("branding-reset")).toBeInTheDocument();
  });

  it("links to the public client progress page", async () => {
    render(<MtechosGroupRoute />);
    await waitFor(() =>
      expect(screen.getByTestId("mtechos-status")).toHaveTextContent("24 huduma"),
    );
    const link = screen.getByTestId("mtechos-client-link");
    expect(link).toHaveAttribute("href", expect.stringContaining("/mteja"));
  });

  it("approving a request hits the approve endpoint that auto-assigns", async () => {
    const user = userEvent.setup();
    render(<MtechosGroupRoute />);
    await user.click(screen.getByTestId("mtechos-tab-maombi"));
    await user.click(await screen.findByTestId("req-approve-REQ-2026-000001"));
    await waitFor(() =>
      expect(
        fetchMock.mock.calls.some(([url]) =>
          String(url).includes("/api/biashara/request/req-1/approve"),
        ),
      ).toBe(true),
    );
  });

  it("shows a worker's task queue with open/completed counts", async () => {
    const user = userEvent.setup();
    render(<MtechosGroupRoute />);
    await user.click(screen.getByTestId("mtechos-tab-wafanyakazi"));
    await user.click(await screen.findByTestId("emp-tasks-emp-1"));
    const panel = await screen.findByTestId("emp-tasks-panel");
    expect(panel).toHaveTextContent("Juma Mwakyusa");
    expect(panel).toHaveTextContent("Zinazoendelea: 1");
    expect(await screen.findByTestId("emp-task-REQ-2026-000001")).toHaveTextContent(
      "Tovuti ya duka",
    );
  });

  it("offers one-click auto-assign on an approved request", async () => {
    const user = userEvent.setup();
    fetchMock.mockImplementation((url: string) => {
      const u = String(url);
      if (u.includes("/api/biashara/requests")) {
        return Promise.resolve({
          ok: true,
          status: 200,
          json: async () => ({
            ok: true,
            requests: [{ ...requestPayload, status: "approved", needs_admin: false }],
          }),
        });
      }
      return Promise.resolve({
        ok: true,
        status: 200,
        json: async () => routeByUrl(u),
      });
    });
    render(<MtechosGroupRoute />);
    await user.click(screen.getByTestId("mtechos-tab-maombi"));
    await user.click(await screen.findByTestId("req-autoassign-REQ-2026-000001"));
    await waitFor(() =>
      expect(
        fetchMock.mock.calls.some(([url]) =>
          String(url).includes("/api/biashara/request/req-1/auto-assign"),
        ),
      ).toBe(true),
    );
    expect(await screen.findByTestId("req-msg")).toHaveTextContent("Juma Mwakyusa");
  });

  it("shows all five management departments with their staff", async () => {
    const user = userEvent.setup();
    render(<MtechosGroupRoute />);
    await user.click(screen.getByTestId("mtechos-tab-idara"));
    const card = await screen.findByTestId("deps-card");
    expect(card).toHaveTextContent("5 idara");
    for (const code of ["ceo", "employee", "finance", "marketing", "hr"]) {
      expect(await screen.findByTestId(`dep-${code}`)).toBeInTheDocument();
    }
    // Juma yupo CEO (DEP-ceo); idara nyingine hazina mtu bado.
    const ceo = await screen.findByTestId("dep-ceo");
    expect(ceo).toHaveTextContent("Juma Mwakyusa");
    expect(await screen.findByTestId("dep-hr")).toHaveTextContent("Hakuna mfanyakazi bado");
  });
});

import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";

vi.mock("#/api/agent-server-config", () => ({
  getAgentServerBaseUrl: () => "http://localhost:9999",
  getAgentServerHeaders: () => ({}),
}));

import CyberSecurityPage, { CYBER_TABS } from "#/routes/cyber-security";

const engineStatus = {
  ok: true,
  engine: "akili-security (AGENT-ENGINE)",
  modules: 12,
  platforms: 5,
  platform_commands: 271,
  swarm_agents: 13,
  swarm_languages: ["rust", "python"],
  apiauth_ops: ["jwt", "oauth", "nacos"],
  finshield_ops: 12,
  finshield_catalog: {
    ops: ["luhn", "pan", "mask", "iban"],
    capabilities: [
      { op: "luhn", desc: "Uthibitisho wa namba ya kadi (mod-10)" },
      { op: "pan", desc: "Aina ya kadi (BIN) + masked PAN" },
    ],
  },
};

const groups = {
  ok: true,
  groups: [
    {
      id: "clamav",
      title: "ClamAV — antivirus ya mfumo",
      desc: "Malware scan, kusasisha saini, daemon",
      cat: "system",
      tool_count: 5,
      tools: ["clamav", "clamscan"],
      tasks: [
        {
          id: "clam-scan",
          name_sw: "Kuchunguza malware (clamscan)",
          cat: "system",
          high_risk: false,
          needs_target: false,
        },
      ],
    },
  ],
};

const platformsSummary = {
  ok: true,
  total_commands: 271,
  platforms: [
    { platform: "kali", title: "Kali Linux", shell: "bash", categories: 13, commands: 129 },
  ],
};

const platformsFull = {
  ok: true,
  platforms: [
    {
      title: "Kali Linux",
      shell: "bash",
      categories: [
        {
          id: "recon",
          title: "Recon",
          commands: [
            {
              name: "nmap",
              desc: "Service and version scan of the target",
              usage: "nmap -sV -Pn $T",
              risk: "low",
              needs_root: false,
              target: "host",
            },
          ],
        },
      ],
    },
  ],
};

const swarmCatalog = {
  ok: true,
  agents: [
    {
      name: "recon",
      title: "Reconnaissance & Enumeration",
      description: "Scans, enumerates ports",
      class: "offensive",
      tools: ["nmap"],
      tasks: ["t10-scan"],
    },
  ],
};

const agentbind = {
  ok: true,
  bindings: [{ agent: "recon", class: "offensive", platform: "kali", task: "recon" }],
};

const forensics = {
  ok: true,
  dfir_agent_count: 7,
  dfir_agents: [
    { agent: "dfir", title: "Digital Forensics", tools: ["volatility"], tasks: ["fore-vol"] },
  ],
};

const shield = { ok: true, active: true, patterns: 36, total_events: 0, recent: [], last_event: null };
const runs = { ok: true, count: 0, runs: [] };

function jsonRes(body: unknown) {
  return Promise.resolve({
    ok: true,
    status: 200,
    json: () => Promise.resolve(body),
  } as Response);
}

beforeEach(() => {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url.includes("/api/security/engine/status")) return jsonRes(engineStatus);
      if (url.includes("/api/security/shield")) return jsonRes(shield);
      if (url.includes("/api/security/groups")) return jsonRes(groups);
      if (url.includes("/api/security/availability")) return jsonRes({ ok: true, groups: [] });
      if (url.includes("/api/security/platforms/full")) return jsonRes(platformsFull);
      if (url.includes("/api/security/platforms")) return jsonRes(platformsSummary);
      if (url.includes("/api/security/swarm/catalog")) return jsonRes(swarmCatalog);
      if (url.includes("/api/security/agentbind")) return jsonRes(agentbind);
      if (url.includes("/api/security/forensics")) return jsonRes(forensics);
      if (url.includes("/api/security/runs")) return jsonRes(runs);
      return jsonRes({ ok: true });
    }),
  );
});

describe("CyberSecurityPage", () => {
  it("huonyesha tabs zote 9", () => {
    render(<CyberSecurityPage />);
    expect(CYBER_TABS).toHaveLength(9);
    expect(screen.getByTestId("cyber-tab-dashibodi")).toBeInTheDocument();
    expect(screen.getByTestId("cyber-tab-finshield")).toBeInTheDocument();
    expect(screen.getByTestId("cyber-tab-kazi")).toBeInTheDocument();
  });

  it("dashibodi huonyesha KPI za injini na ngao", async () => {
    render(<CyberSecurityPage />);
    await waitFor(() => {
      expect(screen.getByTestId("sec-status")).toBeInTheDocument();
    });
    expect(screen.getByTestId("sec-shield")).toBeInTheDocument();
  });

  it("tab ya zana huorodhesha vikundi", async () => {
    render(<CyberSecurityPage />);
    fireEvent.click(screen.getByTestId("cyber-tab-zana"));
    await waitFor(() => {
      expect(screen.getByTestId("sec-tool-clamav")).toBeInTheDocument();
    });
  });

  it("tab ya swarm huonyesha wakala na bindings", async () => {
    render(<CyberSecurityPage />);
    fireEvent.click(screen.getByTestId("cyber-tab-swarm"));
    await waitFor(() => {
      expect(screen.getByTestId("sec-swarm")).toBeInTheDocument();
    });
    expect(screen.getByTestId("sec-agentbind")).toBeInTheDocument();
  });

  it("tab ya DFIR huonyesha wakala wa uchunguzi", async () => {
    render(<CyberSecurityPage />);
    fireEvent.click(screen.getByTestId("cyber-tab-dfir"));
    await waitFor(() => {
      expect(screen.getByTestId("sec-dfir")).toBeInTheDocument();
    });
  });
});

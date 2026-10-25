import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getAcpProvider as getClientAcpProvider } from "@openhands/typescript-client";
import {
  AgentSettingsScreen,
  type AgentSettingsSaveControl,
} from "#/routes/agent-settings";
import SettingsService from "#/api/settings-service/settings-service.api";
import { SecretsService } from "#/api/secrets-service";
import { MOCK_DEFAULT_USER_SETTINGS } from "#/mocks/handlers";
import { Settings } from "#/types/settings";
import { ACP_VERTEX_SAFE_MODEL } from "#/constants/acp-providers";

import { parseCommand } from "#/utils/acp-command";
const CLAUDE_COMMAND = getClientAcpProvider("claude-code")!.default_command;

// Stub the login-detection probe so the ACP credentials section doesn't spin a
// subprocess; default to no detected session so existing tests are unaffected.
const acpAuthStatusMock = vi.hoisted(() => vi.fn());
vi.mock("#/hooks/query/use-acp-auth-status", () => ({
  useAcpAuthStatus: (...args: unknown[]) => acpAuthStatusMock(...args),
}));

// What the provider's own server reports; empty unless a test sets it.
const acpModelDiscoveryMock = vi.hoisted(() => vi.fn());
vi.mock("#/hooks/query/use-acp-model-discovery", () => ({
  useAcpModelDiscovery: (...args: unknown[]) => acpModelDiscoveryMock(...args),
}));
const NO_DISCOVERY = {
  discovery: null,
  models: [],
  source: "none",
  defaultModelId: null,
  isDiscovering: false,
};
const CLAUDE_MODELS = [
  { id: "default", label: "Default (recommended)" },
  { id: "haiku", label: "Haiku" },
];

const profileSupportsSecretRefsMock = vi.hoisted(() => vi.fn(() => true));
const profileSupportsInstructionsMock = vi.hoisted(() => vi.fn(() => false));
vi.mock("#/api/agent-profiles-service/profile-field-support", () => ({
  agentProfileSupportsSecretRefs: () => profileSupportsSecretRefsMock(),
  agentProfileSupportsInstructions: () => profileSupportsInstructionsMock(),
  agentProfileSupportsTools: () => true,
}));

// The secret picker lists the user's saved secrets; stub the query so these
// tests don't need a live secrets store.
const savedSecretsMock = vi.hoisted(() =>
  vi.fn<() => { name: string; description?: string }[]>(),
);
vi.mock("#/hooks/query/use-get-secrets", () => ({
  useSearchSecrets: () => ({ data: savedSecretsMock() }),
}));

// Observe toasts so a silent credential save can be asserted as silent.
const toastMocks = vi.hoisted(() => ({
  success: vi.fn(),
  error: vi.fn(),
  warning: vi.fn(),
}));
vi.mock("#/utils/custom-toast-handlers", () => ({
  displaySuccessToast: toastMocks.success,
  displayErrorToast: toastMocks.error,
  displayWarningToast: toastMocks.warning,
}));

function buildSettings(overrides: Partial<Settings> = {}): Settings {
  return {
    ...MOCK_DEFAULT_USER_SETTINGS,
    ...overrides,
    agent_settings:
      overrides.agent_settings ?? MOCK_DEFAULT_USER_SETTINGS.agent_settings,
  };
}

function renderAgentSettingsScreen(
  props: Partial<React.ComponentProps<typeof AgentSettingsScreen>> = {},
) {
  let control: AgentSettingsSaveControl | null = null;
  const view = render(
    <AgentSettingsScreen
      onSaveControlChange={(next) => {
        control = next;
      }}
      {...props}
    />,
    {
      wrapper: ({ children }) => (
        <MemoryRouter>
          <QueryClientProvider
            client={
              new QueryClient({ defaultOptions: { queries: { retry: false } } })
            }
          >
            {children}
          </QueryClientProvider>
        </MemoryRouter>
      ),
    },
  );
  return { ...view, control: () => control! };
}

const CLAUDE_PROFILE = {
  agent_kind: "acp",
  acp_server: "claude-code",
  acp_command: [],
  acp_args: [],
  acp_model: "",
};

describe("AgentSettingsScreen", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.spyOn(SettingsService, "getSettings").mockResolvedValue(buildSettings());
    // The form owns the ACP credential form, which reads secrets on mount.
    vi.spyOn(SecretsService, "getSecrets").mockResolvedValue([]);
    vi.spyOn(SecretsService, "createSecret").mockResolvedValue();
    acpAuthStatusMock.mockReturnValue({
      status: "unknown",
      isChecking: false,
      isSupported: true,
    });
    acpModelDiscoveryMock.mockReturnValue(NO_DISCOVERY);
    toastMocks.success.mockClear();
    toastMocks.error.mockClear();
    toastMocks.warning.mockClear();
    profileSupportsSecretRefsMock.mockReturnValue(true);
    savedSecretsMock.mockReturnValue([
      { name: "GITHUB_TOKEN", description: "repo access" },
      { name: "DATADOG_API_KEY" },
      { name: "PROD_DB_URL" },
    ]);
  });

  it("builds tool_concurrency_limit from the input on the OpenHands path", async () => {
    const user = userEvent.setup();
    const { control } = renderAgentSettingsScreen({
      agentSettingsOverride: {
        agent_kind: "openhands",
        tool_concurrency_limit: 1,
      },
    });
    await screen.findByTestId("agent-settings-screen");

    const input = screen.getByTestId("sdk-settings-tool_concurrency_limit");
    await user.clear(input);
    await user.type(input, "4");

    // Coerced to a number (not the raw input string) via the shared
    // schema-driven coercion.
    expect(control().buildAgentProfileFields()).toMatchObject({
      tool_concurrency_limit: 4,
    });
  });

  it("keeps Gemini on its Vertex-safe model instead of offering its own default", async () => {
    const user = userEvent.setup();
    renderAgentSettingsScreen({
      agentSettingsOverride: { ...CLAUDE_PROFILE, acp_server: "gemini-cli" },
    });

    await screen.findByTestId("agent-command-input");
    expect(screen.getByLabelText("SETTINGS$AGENT_MODEL")).toHaveValue(
      ACP_VERTEX_SAFE_MODEL,
    );
    await user.click(screen.getByLabelText("SETTINGS$AGENT_MODEL"));
    expect(
      screen.queryByRole("option", {
        name: "SETTINGS$AGENT_MODEL_AGENT_DEFAULT",
      }),
    ).toBeNull();
  });

  it("builds the selected built-in ACP model", async () => {
    acpModelDiscoveryMock.mockReturnValue({
      ...NO_DISCOVERY,
      models: CLAUDE_MODELS,
      source: "live",
    });
    const user = userEvent.setup();
    const { control } = renderAgentSettingsScreen({
      agentSettingsOverride: CLAUDE_PROFILE,
    });

    await screen.findByTestId("agent-command-input");
    await user.click(screen.getByLabelText("SETTINGS$AGENT_MODEL"));
    await user.click(await screen.findByText("Haiku"));

    expect(control().buildAgentProfileFields()).toMatchObject({
      acp_model: "haiku",
    });
  });

  it("does not ask a custom command for its models", async () => {
    renderAgentSettingsScreen({
      agentSettingsOverride: {
        ...CLAUDE_PROFILE,
        acp_server: "custom",
        acp_command: ["my-acp-wrapper"],
      },
    });

    await screen.findByTestId("agent-command-input");
    expect(acpModelDiscoveryMock).toHaveBeenLastCalledWith(null, {
      secretRefs: null,
    });
  });

  it("drops the ACP fields when switching back to OpenHands", async () => {
    const user = userEvent.setup();
    const { control } = renderAgentSettingsScreen({
      agentSettingsOverride: CLAUDE_PROFILE,
    });
    await screen.findByTestId("agent-command-input");

    await user.click(screen.getByTestId("agent-type-selector"));
    await user.click(
      await screen.findByRole("option", {
        name: "SETTINGS$AGENT_TYPE_OPENHANDS",
      }),
    );

    expect(control().buildAgentProfileFields()).toEqual({
      agent_kind: "openhands",
      mcp_server_refs: null,
      secret_refs: null,
      tools: null,
      tool_concurrency_limit: 1,
    });
  });

  it("round-trips a Custom command with quoted args", async () => {
    // ``bash -c "echo hi"`` must not tokenise as
    // ``["bash","-c","\"echo","hi\""]``.
    const user = userEvent.setup();
    const { control } = renderAgentSettingsScreen({
      agentSettingsOverride: CLAUDE_PROFILE,
    });
    const cmd = await screen.findByTestId("agent-command-input");
    await user.clear(cmd);
    await user.type(cmd, 'bash -c "echo hi"');

    const fields = control().buildAgentProfileFields();
    expect(fields).toMatchObject({ acp_server: "custom" });
    expect(
      parseCommand((fields as { acp_command: string }).acp_command),
    ).toEqual(["bash", "-c", "echo hi"]);
  });

  it("is clean again after reverting an agent-type change", async () => {
    const user = userEvent.setup();
    const { control } = renderAgentSettingsScreen({
      agentSettingsOverride: { agent_kind: "openhands" },
    });
    await screen.findByTestId("agent-type-selector");
    expect(control().isDirty).toBe(false);

    await user.click(screen.getByTestId("agent-type-selector"));
    await user.click(
      await screen.findByRole("option", { name: "SETTINGS$AGENT_TYPE_ACP" }),
    );
    expect(control().isDirty).toBe(true);

    await user.click(screen.getByTestId("agent-type-selector"));
    await user.click(
      await screen.findByRole("option", {
        name: "SETTINGS$AGENT_TYPE_OPENHANDS",
      }),
    );
    expect(control().isDirty).toBe(false);
  });
});

describe("AgentSettingsScreen — MCP server scope", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  function seedWithMcp(mcpConfig: Record<string, unknown>) {
    // `useSettings` prefers `agent_settings.mcp_config` over the top-level
    // field, so seed it where the hook actually reads.
    vi.spyOn(SettingsService, "getSettings").mockResolvedValue(
      buildSettings({
        agent_settings: {
          ...MOCK_DEFAULT_USER_SETTINGS.agent_settings,
          agent_kind: "openhands",
          mcp_config: mcpConfig,
        },
      } as never),
    );
  }

  const TWO_SERVERS = {
    github: { url: "https://mcp.example/github", transport: "shttp" },
    postgres: { url: "https://mcp.example/pg", transport: "shttp" },
  };

  it("lists configured servers read-only and persists null by default", async () => {
    seedWithMcp(TWO_SERVERS);
    let control: AgentSettingsSaveControl | null = null;
    renderAgentSettingsScreen({
      agentSettingsOverride: {
        agent_kind: "openhands",
        mcp_server_refs: null,
      },
      onSaveControlChange: (next) => {
        control = next;
      },
    });
    await screen.findByTestId("agent-settings-screen");

    const github = screen.getByTestId("agent-settings-mcp-github");
    expect(github).toBeChecked();
    expect(github).toBeDisabled();
    expect(control!.buildAgentProfileFields()).toMatchObject({
      mcp_server_refs: null,
    });
  });

  it("seeds from a stored scope and persists the selection", async () => {
    seedWithMcp(TWO_SERVERS);
    let control: AgentSettingsSaveControl | null = null;
    renderAgentSettingsScreen({
      agentSettingsOverride: {
        agent_kind: "openhands",
        mcp_server_refs: ["github"],
      },
      onSaveControlChange: (next) => {
        control = next;
      },
    });
    await screen.findByTestId("agent-settings-screen");

    expect(screen.getByTestId("agent-settings-mcp-github")).toBeChecked();
    expect(screen.getByTestId("agent-settings-mcp-postgres")).not.toBeChecked();
    expect(control!.buildAgentProfileFields()).toMatchObject({
      mcp_server_refs: ["github"],
    });
  });

  it("seeds a switch to custom with every configured server", async () => {
    // Turning the control on should narrow from the default rather than cut
    // the agent off from every server at once.
    seedWithMcp(TWO_SERVERS);
    let control: AgentSettingsSaveControl | null = null;
    renderAgentSettingsScreen({
      agentSettingsOverride: {
        agent_kind: "openhands",
        mcp_server_refs: null,
      },
      onSaveControlChange: (next) => {
        control = next;
      },
    });
    await screen.findByTestId("agent-settings-screen");

    const user = userEvent.setup();
    const combo = screen.getByTestId("agent-settings-mcp-mode");
    combo.focus();
    await user.keyboard("{ArrowDown}");
    await user.click(
      await screen.findByRole("option", {
        name: "SETTINGS$AGENT_PROFILE_MCP_CHOOSE",
      }),
    );

    await waitFor(() => {
      expect(control!.buildAgentProfileFields()).toMatchObject({
        mcp_server_refs: ["github", "postgres"],
      });
    });
    expect(screen.getByTestId("agent-settings-mcp-github")).toBeChecked();
    expect(screen.getByTestId("agent-settings-mcp-postgres")).toBeChecked();
  });

  it("warns about a ref whose server is gone, which would fail the launch", async () => {
    seedWithMcp(TWO_SERVERS);
    renderAgentSettingsScreen({
      agentSettingsOverride: {
        agent_kind: "openhands",
        mcp_server_refs: ["github", "deleted-server"],
      },
    });
    await screen.findByTestId("agent-settings-screen");

    expect(
      screen.getByTestId("agent-settings-mcp-deleted-server"),
    ).toBeChecked();
    expect(
      screen.getByTestId("agent-settings-mcp-dangling"),
    ).toBeInTheDocument();
  });

  it("scopes an ACP profile too, since the field lives on the profile base", async () => {
    seedWithMcp(TWO_SERVERS);
    let control: AgentSettingsSaveControl | null = null;
    renderAgentSettingsScreen({
      agentSettingsOverride: {
        agent_kind: "acp",
        acp_server: "claude-code",
        mcp_server_refs: ["github"],
      },
      onSaveControlChange: (next) => {
        control = next;
      },
    });
    await screen.findByTestId("agent-settings-screen");

    expect(screen.getByTestId("agent-settings-mcp-github")).toBeChecked();
    expect(control!.buildAgentProfileFields()).toMatchObject({
      agent_kind: "acp",
      mcp_server_refs: ["github"],
    });
  });

  it("explains the empty state when no server is configured", async () => {
    seedWithMcp({});
    renderAgentSettingsScreen({
      agentSettingsOverride: {
        agent_kind: "openhands",
      },
    });
    await screen.findByTestId("agent-settings-screen");
    expect(
      screen.queryByTestId("agent-settings-mcp-list"),
    ).not.toBeInTheDocument();
    expect(
      screen.getByText("SETTINGS$AGENT_PROFILE_MCP_NONE"),
    ).toBeInTheDocument();
  });
});

describe("AgentSettingsScreen — MCP scope dirty tracking", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.spyOn(SettingsService, "getSettings").mockResolvedValue(
      buildSettings({
        agent_settings: {
          ...MOCK_DEFAULT_USER_SETTINGS.agent_settings,
          agent_kind: "openhands",
          mcp_config: {
            github: { url: "https://mcp.example/github", transport: "shttp" },
            postgres: { url: "https://mcp.example/pg", transport: "shttp" },
          },
        },
      } as never),
    );
  });

  async function dirtyForStoredRefs(refs: string[]) {
    let control: AgentSettingsSaveControl | null = null;
    renderAgentSettingsScreen({
      agentSettingsOverride: {
        agent_kind: "openhands",
        mcp_server_refs: refs,
      },
      onSaveControlChange: (next) => {
        control = next;
      },
    });
    await screen.findByTestId("agent-settings-screen");
    await screen.findByTestId("agent-settings-mcp-list");
    return () => control!.isDirty;
  }

  it("is clean on load when the stored order matches the config order", async () => {
    const isDirty = await dirtyForStoredRefs(["github", "postgres"]);
    await waitFor(() => expect(isDirty()).toBe(false));
  });

  it("is clean on load when the stored order differs from the config order", async () => {
    const isDirty = await dirtyForStoredRefs(["postgres", "github"]);
    await waitFor(() => expect(isDirty()).toBe(false));
  });
  describe("secret scope", () => {
    beforeEach(() => {
      acpAuthStatusMock.mockReturnValue({
        status: "unknown",
        isChecking: false,
        isSupported: true,
      });
      profileSupportsSecretRefsMock.mockReturnValue(true);
      savedSecretsMock.mockReturnValue([
        { name: "GITHUB_TOKEN", description: "repo access" },
        { name: "DATADOG_API_KEY" },
        { name: "PROD_DB_URL" },
      ]);
    });
    function seedOpenHandsSettings() {
      vi.spyOn(SettingsService, "getSettings").mockResolvedValue(
        buildSettings({
          agent_settings: {
            ...MOCK_DEFAULT_USER_SETTINGS.agent_settings,
            agent_kind: "openhands",
          },
        }),
      );
    }

    it("lists every saved secret read-only and persists null by default", async () => {
      seedOpenHandsSettings();
      let control: AgentSettingsSaveControl | null = null;
      renderAgentSettingsScreen({
        agentSettingsOverride: {
          agent_kind: "openhands",
          secret_refs: null,
        },
        onSaveControlChange: (next) => {
          control = next;
        },
      });
      await screen.findByTestId("agent-settings-screen");

      expect(
        screen.getByTestId("agent-settings-secret-list"),
      ).toBeInTheDocument();
      const github = screen.getByTestId("agent-settings-secret-GITHUB_TOKEN");
      expect(github).toBeChecked();
      expect(github).toBeDisabled();
      expect(
        screen.getByTestId("agent-settings-secret-PROD_DB_URL"),
      ).toBeChecked();
      expect(control!.buildAgentProfileFields()).toMatchObject({
        secret_refs: null,
      });
    });

    it("seeds from a stored scope and persists the selection", async () => {
      seedOpenHandsSettings();
      let control: AgentSettingsSaveControl | null = null;
      renderAgentSettingsScreen({
        agentSettingsOverride: {
          agent_kind: "openhands",
          secret_refs: ["DATADOG_API_KEY"],
        },
        onSaveControlChange: (next) => {
          control = next;
        },
      });
      await screen.findByTestId("agent-settings-screen");

      expect(
        screen.getByTestId("agent-settings-secret-DATADOG_API_KEY"),
      ).toBeChecked();
      expect(
        screen.getByTestId("agent-settings-secret-PROD_DB_URL"),
      ).not.toBeChecked();
      expect(control!.buildAgentProfileFields()).toMatchObject({
        secret_refs: ["DATADOG_API_KEY"],
      });
    });

    it("keeps a stored ref whose secret no longer exists", async () => {
      // The save is a whole-profile overwrite, so dropping it here would
      // silently rewrite the user's scope.
      seedOpenHandsSettings();
      let control: AgentSettingsSaveControl | null = null;
      renderAgentSettingsScreen({
        agentSettingsOverride: {
          agent_kind: "openhands",
          secret_refs: ["DELETED_SECRET"],
        },
        onSaveControlChange: (next) => {
          control = next;
        },
      });
      await screen.findByTestId("agent-settings-screen");

      expect(
        screen.getByTestId("agent-settings-secret-DELETED_SECRET"),
      ).toBeChecked();
      expect(control!.buildAgentProfileFields()).toMatchObject({
        secret_refs: ["DELETED_SECRET"],
      });
    });

    it.each([{ secretRefs: [] }, { secretRefs: ["PROD_DB_URL"] }])(
      "preserves an existing ACP secret scope $secretRefs through an unrelated edit",
      async ({ secretRefs }) => {
        savedSecretsMock.mockReturnValue([
          { name: "ANTHROPIC_API_KEY" },
          { name: "PROD_DB_URL" },
        ]);
        seedOpenHandsSettings();
        let control: AgentSettingsSaveControl | null = null;
        renderAgentSettingsScreen({
          agentSettingsOverride: {
            agent_kind: "acp",
            acp_server: "claude-code",
            acp_command: [...CLAUDE_COMMAND],
            acp_args: [],
            acp_model: "haiku",
            secret_refs: secretRefs,
          },
          onSaveControlChange: (next) => {
            control = next;
          },
        });
        await screen.findByTestId("agent-command-input");
        expect(control!.isDirty).toBe(false);
        expect(
          screen.getByTestId("agent-settings-secret-ANTHROPIC_API_KEY"),
        ).not.toBeChecked();

        const user = userEvent.setup();
        await user.click(screen.getByTestId("agent-settings-mcp-mode"));
        await user.click(
          await screen.findByRole("option", {
            name: "SETTINGS$AGENT_PROFILE_MCP_CHOOSE",
          }),
        );
        expect(control!.buildAgentProfileFields()).toMatchObject({
          mcp_server_refs: [],
          secret_refs: secretRefs,
        });
      },
    );

    it("keeps a provider credential deselected when the saved ACP profile is reopened", async () => {
      savedSecretsMock.mockReturnValue([
        { name: "ANTHROPIC_API_KEY" },
        { name: "PROD_DB_URL" },
      ]);
      seedOpenHandsSettings();
      let control: AgentSettingsSaveControl | null = null;
      const onSaveControlChange = (next: AgentSettingsSaveControl | null) => {
        control = next;
      };
      const view = renderAgentSettingsScreen({
        agentSettingsOverride: {
          agent_kind: "acp",
          acp_server: "claude-code",
          acp_command: [...CLAUDE_COMMAND],
          acp_args: [],
          acp_model: "haiku",
          secret_refs: ["ANTHROPIC_API_KEY", "PROD_DB_URL"],
        },
        onSaveControlChange,
      });
      await screen.findByTestId("agent-command-input");
      const user = userEvent.setup();
      await user.click(
        screen.getByTestId("agent-settings-secret-ANTHROPIC_API_KEY"),
      );
      const saved = control!.buildAgentProfileFields();
      expect(saved).toMatchObject({ secret_refs: ["PROD_DB_URL"] });
      view.unmount();
      renderAgentSettingsScreen({
        agentSettingsOverride: saved,
        onSaveControlChange,
      });
      await screen.findByTestId("agent-command-input");
      expect(
        screen.getByTestId("agent-settings-secret-ANTHROPIC_API_KEY"),
      ).not.toBeChecked();
      expect(control!.isDirty).toBe(false);
      expect(control!.buildAgentProfileFields()).toMatchObject({
        secret_refs: ["PROD_DB_URL"],
      });
    });

    it("leaves an OpenHands profile's scope empty when scoping starts", async () => {
      // Nothing an OpenHands agent needs rides this channel, so there is
      // nothing to seed.
      seedOpenHandsSettings();
      let control: AgentSettingsSaveControl | null = null;
      renderAgentSettingsScreen({
        agentSettingsOverride: {
          agent_kind: "openhands",
        },
        onSaveControlChange: (next) => {
          control = next;
        },
      });
      await screen.findByTestId("agent-settings-screen");

      const user = userEvent.setup();
      await user.click(screen.getByTestId("agent-settings-secrets-mode"));
      await user.click(
        await screen.findByRole("option", {
          name: "SETTINGS$AGENT_PROFILE_SECRETS_CHOOSE",
        }),
      );

      await waitFor(() => {
        expect(control!.buildAgentProfileFields()).toMatchObject({
          secret_refs: [],
        });
      });
    });

    it("omits the key on a backend whose profile model predates it", async () => {
      profileSupportsSecretRefsMock.mockReturnValue(false);
      seedOpenHandsSettings();
      let control: AgentSettingsSaveControl | null = null;
      renderAgentSettingsScreen({
        agentSettingsOverride: {
          agent_kind: "openhands",
        },
        onSaveControlChange: (next) => {
          control = next;
        },
      });
      await screen.findByTestId("agent-settings-screen");

      expect(
        screen.queryByTestId("agent-settings-secrets-mode"),
      ).not.toBeInTheDocument();
      expect(control!.buildAgentProfileFields()).not.toHaveProperty(
        "secret_refs",
      );
    });

    it("explains the empty state when nothing is saved", async () => {
      savedSecretsMock.mockReturnValue([]);
      seedOpenHandsSettings();
      renderAgentSettingsScreen({
        agentSettingsOverride: {
          agent_kind: "openhands",
        },
      });
      await screen.findByTestId("agent-settings-screen");
      expect(
        screen.queryByTestId("agent-settings-secret-list"),
      ).not.toBeInTheDocument();
      expect(
        screen.getByText("SETTINGS$AGENT_PROFILE_SECRETS_NONE"),
      ).toBeInTheDocument();
      // The control itself stays, so the user can still see the scope mode.
      expect(
        screen.getByTestId("agent-settings-secrets-mode"),
      ).toBeInTheDocument();
    });
  });
});

describe("AgentSettingsScreen — system prompt", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.spyOn(SettingsService, "getSettings").mockResolvedValue(buildSettings());
    vi.spyOn(SecretsService, "getSecrets").mockResolvedValue([]);
    profileSupportsInstructionsMock.mockReturnValue(true);
  });

  const OPENHANDS_PROFILE = {
    agent_kind: "openhands",
    mcp_server_refs: null,
  };

  async function openModes() {
    const user = userEvent.setup();
    const combo = screen.getByTestId("agent-settings-system-prompt-mode");
    combo.focus();
    await user.keyboard("{ArrowDown}");
    return user;
  }

  async function chooseMode(name: string) {
    const user = await openModes();
    await user.click(await screen.findByRole("option", { name }));
    return user;
  }

  it("is hidden where launches would not apply it", async () => {
    profileSupportsInstructionsMock.mockReturnValue(false);
    const { control } = renderAgentSettingsScreen({
      agentSettingsOverride: OPENHANDS_PROFILE,
    });
    await screen.findByTestId("agent-settings-screen");

    expect(
      screen.queryByTestId("agent-settings-system-prompt-mode"),
    ).not.toBeInTheDocument();
    const fields = control().buildAgentProfileFields();
    expect(fields).not.toHaveProperty("persona");
    expect(fields).not.toHaveProperty("system_message_suffix");
  });

  it("persists null for both fields on the OpenHands default", async () => {
    const { control } = renderAgentSettingsScreen({
      agentSettingsOverride: OPENHANDS_PROFILE,
    });
    await screen.findByTestId("agent-settings-system-prompt-mode");

    expect(
      screen.queryByTestId("agent-settings-system-prompt"),
    ).not.toBeInTheDocument();
    expect(control().buildAgentProfileFields()).toMatchObject({
      persona: null,
      system_message_suffix: null,
    });
  });

  it.each([
    ["system_message_suffix", "append"],
    ["persona", "custom"],
  ])("opens a stored %s in %s mode, clean", async (field, mode) => {
    const { control } = renderAgentSettingsScreen({
      agentSettingsOverride: { ...OPENHANDS_PROFILE, [field]: "Stored text." },
    });

    expect(
      await screen.findByTestId("agent-settings-system-prompt"),
    ).toHaveValue("Stored text.");
    expect(
      screen.getByTestId("agent-settings-system-prompt-hint"),
    ).toHaveTextContent(
      mode === "append"
        ? "SETTINGS$AGENT_PROFILE_SYSTEM_PROMPT_APPEND_HINT"
        : "SETTINGS$AGENT_PROFILE_SYSTEM_PROMPT_CUSTOM_HINT",
    );
    expect(control().isDirty).toBe(false);
    expect(control().buildAgentProfileFields()).toMatchObject({
      [field]: "Stored text.",
    });
  });

  it("saves typed instructions as the suffix, not an override", async () => {
    const { control } = renderAgentSettingsScreen({
      agentSettingsOverride: OPENHANDS_PROFILE,
    });
    await screen.findByTestId("agent-settings-system-prompt-mode");

    const user = await chooseMode(
      "SETTINGS$AGENT_PROFILE_SYSTEM_PROMPT_APPEND",
    );
    await user.type(
      screen.getByTestId("agent-settings-system-prompt"),
      "Be terse.",
    );
    await waitFor(() => expect(control().isDirty).toBe(true));
    expect(control().buildAgentProfileFields()).toMatchObject({
      persona: null,
      system_message_suffix: "Be terse.",
    });
  });

  it("blocks saving empty text, then saves the typed custom persona", async () => {
    const { control } = renderAgentSettingsScreen({
      agentSettingsOverride: OPENHANDS_PROFILE,
    });
    await screen.findByTestId("agent-settings-system-prompt-mode");

    const user = await chooseMode(
      "SETTINGS$AGENT_PROFILE_SYSTEM_PROMPT_CUSTOM",
    );
    await waitFor(() => expect(control().isValid).toBe(false));
    expect(
      screen.getByTestId("agent-settings-system-prompt-required"),
    ).toBeInTheDocument();

    await user.type(
      screen.getByTestId("agent-settings-system-prompt"),
      "You review PRs.",
    );
    await waitFor(() => expect(control().isValid).toBe(true));
    expect(control().buildAgentProfileFields()).toMatchObject({
      persona: "You review PRs.",
      system_message_suffix: null,
    });
  });

  it("clears a stored persona when switched back to the default", async () => {
    const { control } = renderAgentSettingsScreen({
      agentSettingsOverride: {
        ...OPENHANDS_PROFILE,
        persona: "You triage issues.",
      },
    });
    await screen.findByTestId("agent-settings-system-prompt");

    await chooseMode("SETTINGS$AGENT_PROFILE_SYSTEM_PROMPT_STANDARD");
    await waitFor(() => expect(control().isDirty).toBe(true));
    expect(control().buildAgentProfileFields()).toMatchObject({
      persona: null,
      system_message_suffix: null,
    });
  });

  it("explains instead of editing on the default profile, and leaves both fields alone", async () => {
    const { control } = renderAgentSettingsScreen({
      agentSettingsOverride: {
        ...OPENHANDS_PROFILE,
        persona: "Set through the API.",
        system_message_suffix: "Also set through the API.",
      },
      isDefaultProfile: true,
    });

    expect(
      await screen.findByTestId("agent-settings-system-prompt-default-profile"),
    ).toBeInTheDocument();
    expect(
      screen.queryByTestId("agent-settings-system-prompt-mode"),
    ).not.toBeInTheDocument();
    const fields = control().buildAgentProfileFields();
    expect(fields).not.toHaveProperty("persona");
    expect(fields).not.toHaveProperty("system_message_suffix");
  });
});

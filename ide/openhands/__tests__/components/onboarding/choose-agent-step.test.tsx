import React from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  AgentOptionIcon,
  ChooseAgentStep,
  type OnboardingAgentId,
} from "#/components/features/onboarding/steps/choose-agent-step";
import SettingsService from "#/api/settings-service/settings-service.api";
import {
  __resetActiveStoreForTests,
  setActiveSelection,
  setRegisteredBackends,
} from "#/api/backend-registry/active-store";
import { ActiveBackendProvider } from "#/contexts/active-backend-context";

function renderStep(initial: OnboardingAgentId = "openhands") {
  const onSelect = vi.fn();
  const onNext = vi.fn();
  render(
    <QueryClientProvider
      client={
        new QueryClient({ defaultOptions: { queries: { retry: false } } })
      }
    >
      <ChooseAgentStep
        selectedAgentId={initial}
        onSelect={onSelect}
        onNext={onNext}
      />
    </QueryClientProvider>,
  );
  return { onSelect, onNext };
}

describe("ChooseAgentStep", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.spyOn(SettingsService, "saveSettings").mockResolvedValue(true);
  });

  it("renders only the MBILINYI tile, selected by default", () => {
    renderStep();

    const openhands = screen.getByTestId("onboarding-agent-option-openhands");
    expect(openhands).toHaveAttribute("aria-checked", "true");
    expect(openhands).not.toBeDisabled();

    // The IDE is self-contained: no external ACP harness tiles are surfaced.
    expect(
      screen.queryByTestId("onboarding-agent-option-claude-code"),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByTestId("onboarding-agent-option-codex"),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByTestId("onboarding-agent-option-gemini-cli"),
    ).not.toBeInTheDocument();
  });

  it("falls back to the generic CLI icon for an unknown provider key", () => {
    render(<AgentOptionIcon id="unknown-cli" muted={false} />);

    expect(
      screen.getByTestId("onboarding-agent-icon-cli-generic"),
    ).toBeInTheDocument();
  });

  it("persists agent_kind:'openhands' and advances on Next", async () => {
    const save = vi.spyOn(SettingsService, "saveSettings");
    const { onNext } = renderStep("openhands");
    const user = userEvent.setup();

    await user.click(screen.getByTestId("onboarding-agent-next"));

    await waitFor(() => {
      expect(save).toHaveBeenCalledTimes(1);
      expect(onNext).toHaveBeenCalledTimes(1);
    });
    const call = save.mock.calls[0]?.[0] as {
      agent_settings_diff?: Record<string, unknown>;
    };
    expect(call.agent_settings_diff).toEqual({ agent_kind: "openhands" });
  });

  it("does not advance when the save mutation fails", async () => {
    vi.spyOn(SettingsService, "saveSettings").mockRejectedValueOnce(
      new Error("boom"),
    );
    const { onNext } = renderStep("openhands");
    const user = userEvent.setup();

    await user.click(screen.getByTestId("onboarding-agent-next"));

    await waitFor(
      () => {
        expect(SettingsService.saveSettings).toHaveBeenCalledTimes(1);
      },
      { timeout: 1000 },
    );
    expect(onNext).not.toHaveBeenCalled();
  });
});

describe("ChooseAgentStep on a cloud backend", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    __resetActiveStoreForTests();
    setRegisteredBackends([
      {
        id: "cloud-1",
        name: "Cloud",
        host: "https://app.example.com",
        apiKey: "key",
        kind: "cloud",
      },
    ]);
    setActiveSelection({ backendId: "cloud-1", orgId: null });
  });
  afterEach(() => {
    window.localStorage.clear();
    __resetActiveStoreForTests();
  });

  it("still offers the self-contained MBILINYI agent on Cloud", () => {
    const onSelect = vi.fn();
    render(
      <QueryClientProvider
        client={
          new QueryClient({ defaultOptions: { queries: { retry: false } } })
        }
      >
        <ActiveBackendProvider>
          <ChooseAgentStep
            selectedAgentId="openhands"
            onSelect={onSelect}
            onNext={vi.fn()}
          />
        </ActiveBackendProvider>
      </QueryClientProvider>,
    );

    expect(
      screen.getByTestId("onboarding-agent-option-openhands"),
    ).toBeInTheDocument();
    expect(onSelect).not.toHaveBeenCalled();
  });
});

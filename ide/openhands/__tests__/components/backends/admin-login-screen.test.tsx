import React from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { __resetActiveStoreForTests } from "#/api/backend-registry/active-store";
import { ActiveBackendProvider } from "#/contexts/active-backend-context";
import AdminLoginScreen from "#/components/features/backends/admin-login-screen";

// Stub cloud org service used by ActiveBackendProvider
vi.mock("#/api/cloud/organization-service.api", () => ({
  getCurrentCloudApiKey: vi.fn().mockResolvedValue({
    orgId: null,
    isLegacyKey: true,
  }),
}));

// The API-key fallback is a separate screen with its own test; stub it here.
vi.mock("#/components/features/backends/api-key-entry-screen", () => ({
  default: () => <div data-testid="api-key-entry-screen" />,
}));

const reloadMock = vi.fn();

function renderScreen() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <ActiveBackendProvider>
        <AdminLoginScreen />
      </ActiveBackendProvider>
    </QueryClientProvider>,
  );
}

const ORIGINAL_LOCATION = window.location;

beforeEach(() => {
  window.localStorage.clear();
  __resetActiveStoreForTests();
  reloadMock.mockReset();
  Object.defineProperty(window, "location", {
    configurable: true,
    value: {
      ...ORIGINAL_LOCATION,
      origin: "http://localhost:8000",
      hostname: "localhost",
      reload: reloadMock,
    },
  });
});

afterEach(() => {
  vi.restoreAllMocks();
  Object.defineProperty(window, "location", {
    configurable: true,
    value: ORIGINAL_LOCATION,
  });
  window.localStorage.clear();
});

describe("AdminLoginScreen — MTECHOS login halisi", () => {
  it("huonyesha fomu ya login", () => {
    renderScreen();
    expect(screen.getByTestId("admin-login-screen")).toBeInTheDocument();
    expect(screen.getByTestId("admin-login-username")).toBeInTheDocument();
    expect(screen.getByTestId("admin-login-password")).toBeInTheDocument();
  });

  it("huingia kwa username/password na kuhifadhi token, kisha reload", async () => {
    const user = userEvent.setup();
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ ok: true, session: { token: "tok-123" } }),
    });
    vi.stubGlobal("fetch", fetchMock);

    renderScreen();
    await user.type(screen.getByTestId("admin-login-username"), "MBILINYI");
    await user.type(screen.getByTestId("admin-login-password"), "MKINGAHALISI");
    await user.click(screen.getByTestId("admin-login-submit"));

    await waitFor(() => expect(reloadMock).toHaveBeenCalled());
    const stored = window.localStorage.getItem("mbilinyi-backends");
    expect(stored).toContain("tok-123");
  });

  it("huonyesha kosa kwa credentials mbovu", async () => {
    const user = userEvent.setup();
    const fetchMock = vi.fn().mockResolvedValue({
      ok: false,
      status: 401,
      json: async () => ({ ok: false, error: "Password si sahihi" }),
    });
    vi.stubGlobal("fetch", fetchMock);

    renderScreen();
    await user.type(screen.getByTestId("admin-login-username"), "MBILINYI");
    await user.type(screen.getByTestId("admin-login-password"), "nope");
    await user.click(screen.getByTestId("admin-login-submit"));

    expect(await screen.findByTestId("admin-login-error")).toHaveTextContent(
      "Password si sahihi",
    );
    expect(reloadMock).not.toHaveBeenCalled();
  });

  it("hubadilisha kwenda skrini ya API key", async () => {
    const user = userEvent.setup();
    renderScreen();
    await user.click(screen.getByTestId("admin-login-use-api-key"));
    expect(await screen.findByTestId("api-key-entry-screen")).toBeInTheDocument();
  });
});

/**
 * `/mteja` — ukurasa wa umma wa kufuatilia maendeleo ya ombi.
 *
 * Vipimo hivi huhakikisha: (1) fomu hutumia kumbukumbu + simu, (2) maendeleo,
 * hatua, mfanyakazi, mikataba na malipo huonyeshwa, (3) kosa la uthibitisho
 * hufichua KILA KITU — ref sahihi + simu isiyo sahihi HAIruhusiwi kuona kitu.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import MtejaTrack from "#/routes/mteja-track";

const fetchMock = vi.fn();

vi.mock("#/api/agent-server-config", async (importOriginal) => ({
  ...(await importOriginal<typeof import("#/api/agent-server-config")>()),
  getAgentServerBaseUrl: () => "http://agent.example.com",
  getAgentServerHeaders: () => ({}),
}));

const trackPayload = {
  ok: true,
  ref: "REQ-2026-000001",
  title: "Tovuti ya duka",
  service: "Uundaji wa Programu",
  service_name: "Software Development",
  status: "in_progress",
  stage: "Inaendelea (60%)",
  progress: 60,
  amount: "500,000.00",
  currency: "TZS",
  assignee: { full_name: "Juma Mwakyusa", profession: "Mhandisi wa Programu" },
  paid: true,
  contracts: [
    { title: "Mkataba wa Tovuti", status: "signed", currency: "TZS", amount_cents: 50_000_000 },
  ],
  timeline: [
    { actor: "mteja", kind: "created", note: "Ombi limewasilishwa", progress: 0, at: "2026-10-23T07:00:00Z" },
    { actor: "admin", kind: "assigned", note: "Imepewa Juma Mwakyusa", progress: 10, at: "2026-10-23T07:05:00Z" },
  ],
};

describe("MTEA public track page", () => {
  beforeEach(() => {
    fetchMock.mockReset();
    fetchMock.mockImplementation((url: string) => {
      const u = String(url);
      if (u.includes("/api/biashara/client/track")) {
        const body = JSON.parse(
          (fetchMock.mock.calls.at(-1)?.[1] as { body?: string })?.body ?? "{}",
        );
        // Server policy: a wrong phone yields the same generic error.
        if (body.phone === "0712345678") {
          return Promise.resolve({
            ok: true,
            status: 200,
            json: async () => trackPayload,
          });
        }
        return Promise.resolve({
          ok: true,
          status: 200,
          json: async () => ({ ok: false, error: "kumbukumbu au namba ya simu si sahihi" }),
        });
      }
      return Promise.resolve({ ok: true, status: 200, json: async () => ({}) });
    });
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("renders the lookup form without requiring a login", () => {
    render(<MtejaTrack />);
    expect(screen.getByTestId("mteja-form")).toBeInTheDocument();
    expect(screen.getByTestId("mteja-ref")).toBeInTheDocument();
    expect(screen.getByTestId("mteja-phone")).toBeInTheDocument();
  });

  it("shows progress, stage, assignee, contracts and payment state", async () => {
    const user = userEvent.setup();
    render(<MtejaTrack />);
    await user.type(screen.getByTestId("mteja-ref"), "REQ-2026-000001");
    await user.type(screen.getByTestId("mteja-phone"), "0712345678");
    await user.click(screen.getByTestId("mteja-submit"));

    const result = await screen.findByTestId("mteja-result");
    expect(result).toHaveTextContent("REQ-2026-000001");
    expect(result).toHaveTextContent("Tovuti ya duka");
    expect(result).toHaveTextContent("Inaendelea (60%)");
    expect(await screen.findByTestId("mteja-assignee")).toHaveTextContent("Juma Mwakyusa");
    expect(screen.getByTestId("mteja-timeline")).toHaveTextContent("Imepewa Juma Mwakyusa");
    expect(screen.getByText("Malipo yamefanyika")).toBeInTheDocument();
    expect(screen.getByText(/Mkataba wa Tovuti/)).toBeInTheDocument();
    expect(screen.getByTestId("mteja-steps")).toHaveTextContent("Kazi inaendelea");
  });

  it("rejects a mismatched reference/phone pair", async () => {
    const user = userEvent.setup();
    render(<MtejaTrack />);
    await user.type(screen.getByTestId("mteja-ref"), "REQ-2026-000001");
    await user.type(screen.getByTestId("mteja-phone"), "0799999999");
    await user.click(screen.getByTestId("mteja-submit"));

    await waitFor(() =>
      expect(screen.getByText("kumbukumbu au namba ya simu si sahihi")).toBeInTheDocument(),
    );
    expect(screen.queryByTestId("mteja-result")).not.toBeInTheDocument();
  });

  it("sends the reference and phone to the public endpoint", async () => {
    const user = userEvent.setup();
    render(<MtejaTrack />);
    await user.type(screen.getByTestId("mteja-ref"), "REQ-2026-000001");
    await user.type(screen.getByTestId("mteja-phone"), "0712345678");
    await user.click(screen.getByTestId("mteja-submit"));

    await waitFor(() =>
      expect(
        fetchMock.mock.calls.some(([url]) =>
          String(url).includes("/api/biashara/client/track"),
        ),
      ).toBe(true),
    );
  });
});

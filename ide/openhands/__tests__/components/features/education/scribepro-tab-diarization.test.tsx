import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ScribeProTab } from "#/components/features/education/scribepro-tab";
import type { DiarizeResult } from "#/api/education-service/scribe.api";

const diar: DiarizeResult = {
  ok: true,
  speakers: 2,
  turns: 3,
  total_speech_s: 18.48,
  segments: [
    { speaker: 1, t_start: 0.1, t_end: 7.2, duration_s: 7.1, mean_rms: 0.031 },
    { speaker: 2, t_start: 7.3, t_end: 13.2, duration_s: 5.9, mean_rms: 0.028 },
    { speaker: 1, t_start: 13.2, t_end: 18.7, duration_s: 5.5, mean_rms: 0.03 },
  ],
  per_speaker: [
    { speaker: 1, turns: 2, total_s: 12.59, share_pct: 68.1 },
    { speaker: 2, turns: 1, total_s: 5.89, share_pct: 31.9 },
  ],
  method: "MFCC + F0",
};

const denoiseResult = {
  ok: true,
  sample_rate: 16000,
  samples_in: 16000,
  samples_out: 16000,
  duration_s: 1,
  speech_frames: 10,
  total_frames: 20,
  speech_ratio: 0.5,
  rms_before: 0.01,
  rms_after: 0.01,
  noise_removed_db: 3,
  strength: 0.8,
  wav_base64: btoa("RIFF"),
  diarization: diar,
  note_sw: "imesafishwa",
};

const fetchMock = vi.fn();

vi.mock("#/api/agent-server-config", async (importOriginal) => ({
  ...(await importOriginal<typeof import("#/api/agent-server-config")>()),
  getAgentServerBaseUrl: () => "http://agent.example.com",
  getAgentServerHeaders: () => ({}),
}));

function routeByUrl(url: string) {
  if (url.includes("/api/scribe/status")) {
    return {
      ok: true,
      module: "scribe",
      part: "1",
      dsp: {},
      transcription: { whisper_api: false, web_speech_fallback: true },
      agentic: { llm: false, modes: [] },
      pdf: {},
      jobs: 0,
      pdfs: 0,
    };
  }
  if (url.includes("/api/scribe/jobs")) return { ok: true, jobs: [] };
  if (url.includes("/api/scribe/denoise")) return denoiseResult;
  if (url.includes("/api/scribe/diarize")) return { ...diar, speakers: 3 };
  return {};
}

describe("ScribeProTab — diarization", () => {
  beforeEach(() => {
    fetchMock.mockReset();
    fetchMock.mockImplementation((url: string) =>
      Promise.resolve({
        ok: true,
        status: 200,
        json: async () => routeByUrl(String(url)),
      }),
    );
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("shows the diarization panel after uploading audio", async () => {
    const user = userEvent.setup();
    render(<ScribeProTab />);

    const file = new File([new Uint8Array([1, 2, 3])], "mkutano.wav", {
      type: "audio/wav",
    });
    const input = screen.getByTestId("scribepro-file") as HTMLInputElement;
    await user.upload(input, file);

    const panel = await screen.findByTestId("scribepro-diar");
    expect(panel).toBeInTheDocument();
    expect(screen.getByTestId("scribepro-diar-count")).toHaveTextContent(
      "2 wazungumzaji",
    );
    expect(screen.getByTestId("scribepro-diar-sp-1")).toHaveTextContent(
      "S1: 2 zamu · 12.59 s · 68.1%",
    );
    expect(screen.getByTestId("scribepro-diar-seg-0")).toHaveTextContent("S1");
    expect(screen.getByTestId("scribepro-diar-seg-0")).toHaveTextContent(
      "0.1–7.2 s",
    );
  });

  it("re-runs diarization with the requested speaker count", async () => {
    const user = userEvent.setup();
    render(<ScribeProTab />);

    const input = screen.getByTestId("scribepro-file") as HTMLInputElement;
    await user.upload(
      input,
      new File([new Uint8Array([1])], "a.wav", { type: "audio/wav" }),
    );
    await screen.findByTestId("scribepro-diar");

    const count = screen.getByTestId("scribepro-diar-speakers");
    await user.clear(count);
    await user.type(count, "3");
    await user.click(screen.getByTestId("scribepro-diar-run"));

    await waitFor(() => {
      const calls = fetchMock.mock.calls.map((c) => String(c[0]));
      expect(
        calls.some((u) => u.includes("diarize?rate=16000&speakers=3")),
      ).toBe(true);
    });
    await waitFor(() =>
      expect(screen.getByTestId("scribepro-diar-count")).toHaveTextContent(
        "3 wazungumzaji",
      ),
    );
  });

  it("shows no diarization panel when denoise omits it", async () => {
    fetchMock.mockImplementation((url: string) =>
      Promise.resolve({
        ok: true,
        status: 200,
        json: async () => {
          if (String(url).includes("/api/scribe/denoise")) {
            return { ...denoiseResult, diarization: undefined };
          }
          return routeByUrl(String(url));
        },
      }),
    );
    const user = userEvent.setup();
    render(<ScribeProTab />);

    const input = screen.getByTestId("scribepro-file") as HTMLInputElement;
    await user.upload(
      input,
      new File([new Uint8Array([1])], "a.wav", { type: "audio/wav" }),
    );

    await waitFor(() =>
      expect(screen.getByTestId("scribepro-denoise")).toBeInTheDocument(),
    );
    expect(screen.queryByTestId("scribepro-diar")).not.toBeInTheDocument();
  });
});

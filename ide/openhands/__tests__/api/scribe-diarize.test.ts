import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  diarizeAudio,
  type DiarizeResult,
} from "#/api/education-service/scribe.api";

vi.mock("#/api/agent-server-config", async (importOriginal) => ({
  ...(await importOriginal<typeof import("#/api/agent-server-config")>()),
  getAgentServerBaseUrl: () => "http://agent.example.com",
  getAgentServerHeaders: () => ({ "X-Test": "1" }),
}));

const sample: DiarizeResult = {
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

describe("diarizeAudio", () => {
  const fetchMock = vi.fn();

  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("posts raw bytes to /api/scribe/diarize with rate and speakers", async () => {
    fetchMock.mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => sample,
    });

    const buf = new Uint8Array([1, 2, 3, 4]).buffer;
    const out = await diarizeAudio(buf, 16000, 2);

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe(
      "http://agent.example.com/api/scribe/diarize?rate=16000&speakers=2",
    );
    expect(init.method).toBe("POST");
    expect(init.headers["Content-Type"]).toBe("application/octet-stream");
    expect(init.headers["X-Test"]).toBe("1");
    expect(init.body).toBe(buf);
    expect(out).toEqual(sample);
  });

  it("defaults speakers to 0 (auto-detect) and accepts a Blob", async () => {
    fetchMock.mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ ...sample, speakers: 1 }),
    });

    const out = await diarizeAudio(new Blob([new Uint8Array([9, 9])]));

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe(
      "http://agent.example.com/api/scribe/diarize?rate=16000&speakers=0",
    );
    expect(init.body).toBeInstanceOf(ArrayBuffer);
    expect(out.speakers).toBe(1);
  });

  it("surfaces the server error message on a failed response", async () => {
    fetchMock.mockResolvedValue({
      ok: false,
      status: 422,
      json: async () => ({ error: "sauti ni fupi mno" }),
    });

    await expect(diarizeAudio(new ArrayBuffer(8))).rejects.toThrow(
      "sauti ni fupi mno",
    );
  });

  it("falls back to an HTTP status message when the body is not JSON", async () => {
    fetchMock.mockResolvedValue({
      ok: false,
      status: 500,
      json: async () => {
        throw new Error("not json");
      },
    });

    await expect(diarizeAudio(new ArrayBuffer(8))).rejects.toThrow("HTTP 500");
  });
});

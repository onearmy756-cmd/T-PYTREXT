import { describe, expect, it } from "vitest";
import {
  ACP_CUSTOM_PRESET_KEY,
  ACP_PROVIDERS,
  ACP_VERTEX_SAFE_MODEL,
  buildAcpAgentSettingsDiff,
  getAcpCredentialConflicts,
  getAcpPreferredDefaultModel,
  getAcpProviderDisplayName,
  getAcpProviderSecrets,
} from "#/constants/acp-providers";

describe("getAcpProviderDisplayName", () => {
  it("returns null for keys with no surfaced external harness", () => {
    // MBILINYI IDE ships only its own harness, so the external registry is
    // empty and every lookup falls back to the generic "ACP" chip label.
    expect(getAcpProviderDisplayName("claude-code")).toBeNull();
    expect(getAcpProviderDisplayName("codex")).toBeNull();
    expect(getAcpProviderDisplayName("gemini-cli")).toBeNull();
    expect(getAcpProviderDisplayName("pi")).toBeNull();
    expect(getAcpProviderDisplayName("opencode")).toBeNull();
  });

  it("returns null for the Custom-command preset so callers can fall back to the generic 'ACP' label", () => {
    // The custom preset has no canonical brand name — the registry
    // resolver intentionally returns null so the conversation card renders
    // ``CONVERSATION$ACP_AGENT_GENERIC`` ("ACP") instead.
    expect(getAcpProviderDisplayName("custom")).toBeNull();
  });

  it("returns null for unknown / forward-compatible keys", () => {
    // A future ACP server Canvas's registry doesn't know about yet
    // shouldn't crash or render a random fragment of the key — fall back
    // to the generic chip.
    expect(getAcpProviderDisplayName("future-acp-server")).toBeNull();
  });

  it("returns null for empty / null / undefined input", () => {
    expect(getAcpProviderDisplayName(null)).toBeNull();
    expect(getAcpProviderDisplayName(undefined)).toBeNull();
    expect(getAcpProviderDisplayName("")).toBeNull();
  });
});

describe("ACP provider registry", () => {
  it("surfaces no external ACP harness", () => {
    // MBILINYI IDE is self-contained: the registry is intentionally empty, so
    // there is no SDK-derived data to mirror.
    expect(ACP_PROVIDERS).toEqual([]);
  });

  it("builds no diff for a harness it does not surface", () => {
    expect(buildAcpAgentSettingsDiff("claude-code")).toBeNull();
    expect(buildAcpAgentSettingsDiff("gemini-cli")).toBeNull();
  });

  it("keeps custom ACP diffs model-optional", () => {
    expect(buildAcpAgentSettingsDiff(ACP_CUSTOM_PRESET_KEY)).toMatchObject({
      agent_kind: "acp",
      acp_server: ACP_CUSTOM_PRESET_KEY,
      acp_model: null,
    });
  });
});

describe("getAcpProviderSecrets — containerized credentials", () => {
  // With no external harness surfaced, no provider yields a credentials step:
  // the onboarding flow never prompts for Claude/Codex/Gemini/Pi/OpenCode keys.
  it("returns no credential fields for any external harness", () => {
    for (const key of [
      "codex",
      "claude-code",
      "gemini-cli",
      "pi",
      "opencode",
    ]) {
      expect(getAcpProviderSecrets(key), key).toEqual([]);
    }
  });

  it("returns [] for MBILINYI / custom / unknown / empty", () => {
    expect(getAcpProviderSecrets("openhands")).toEqual([]);
    expect(getAcpProviderSecrets(ACP_CUSTOM_PRESET_KEY)).toEqual([]);
    expect(getAcpProviderSecrets("future-acp-server")).toEqual([]);
    expect(getAcpProviderSecrets(null)).toEqual([]);
  });
});

describe("getAcpPreferredDefaultModel", () => {
  it("keeps the Vertex-safe Gemini default even with no surfaced harness", () => {
    // The default-model map is independent of the UI registry; gemini-cli's own
    // default 404s on many Vertex projects, so the safe model is still returned.
    expect(getAcpPreferredDefaultModel("gemini-cli")).toBe(
      ACP_VERTEX_SAFE_MODEL,
    );
    expect(getAcpPreferredDefaultModel("claude-code")).toBeNull();
    expect(getAcpPreferredDefaultModel("codex")).toBeNull();
  });

  it("pins a NON-flash Gemini model", () => {
    // gemini-cli 0.45.x re-resolves any *-flash id at generation time to its
    // current default flash (software-agent-sdk#3532), so a flash pin is not
    // honored — only a non-flash id (e.g. gemini-2.5-pro) sticks.
    expect(ACP_VERTEX_SAFE_MODEL).not.toMatch(/flash/);
  });

  it("returns null for MBILINYI / custom / unknown", () => {
    expect(getAcpPreferredDefaultModel("openhands")).toBeNull();
    expect(getAcpPreferredDefaultModel(ACP_CUSTOM_PRESET_KEY)).toBeNull();
    expect(getAcpPreferredDefaultModel("future-acp-server")).toBeNull();
  });
});

describe("getAcpCredentialConflicts", () => {
  const has =
    (...names: string[]) =>
    (name: string) =>
      names.includes(name);

  it("flags the Claude OAuth token + base URL pair when both are set", () => {
    expect(
      getAcpCredentialConflicts(
        "claude-code",
        has("CLAUDE_CODE_OAUTH_TOKEN", "ANTHROPIC_BASE_URL"),
      ),
    ).toEqual([["CLAUDE_CODE_OAUTH_TOKEN", "ANTHROPIC_BASE_URL"]]);
  });

  it("has no conflicts for other providers / null", () => {
    expect(
      getAcpCredentialConflicts(
        "codex",
        has("CLAUDE_CODE_OAUTH_TOKEN", "ANTHROPIC_BASE_URL"),
      ),
    ).toEqual([]);
    expect(getAcpCredentialConflicts(null, () => true)).toEqual([]);
  });
});

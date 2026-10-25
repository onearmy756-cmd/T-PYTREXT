import { ACP_PROVIDERS as CLIENT_ACP_PROVIDERS } from "@openhands/typescript-client";
import { describe, expect, it } from "vitest";
import {
  ACP_MANAGED_SENTINEL,
  ACP_PROVIDERS,
  getAcpProvidersForBackend,
  getAcpProviderSecrets,
  SURFACED_ACP_PROVIDERS,
  resolveEffectiveAcpModel,
} from "./acp-providers";

describe("ACP_PROVIDERS", () => {
  it("surfaces no external ACP harness on the self-contained MBILINYI IDE", () => {
    expect(ACP_PROVIDERS).toEqual([]);
  });
});

describe("resolveEffectiveAcpModel", () => {
  it("surfaces the real claude-agent-acp 0.44+ 'default' model", () => {
    // ``default`` ("Default (recommended)") is a real, selectable Claude model
    // in the configOptions select — the server reports it as the current model.
    // It must NOT be suppressed as a placeholder (regression: the chip would
    // otherwise show no model for a session genuinely running on 'default').
    expect(resolveEffectiveAcpModel({ runtimeId: "default" })).toBe("default");
    expect(
      resolveEffectiveAcpModel({ runtimeName: "Default (recommended)" }),
    ).toBe("Default (recommended)");
  });

  it("follows the runtime → configured → sdkLlm precedence", () => {
    expect(
      resolveEffectiveAcpModel({
        runtimeName: "Sonnet",
        runtimeId: "sonnet",
        configured: "haiku",
      }),
    ).toBe("Sonnet");
    expect(resolveEffectiveAcpModel({ configured: "haiku" })).toBe("haiku");
    expect(resolveEffectiveAcpModel({ sdkLlm: "gpt-5.5/medium" })).toBe(
      "gpt-5.5/medium",
    );
  });

  it("still suppresses the legacy acp-managed sentinel and blanks", () => {
    expect(
      resolveEffectiveAcpModel({ sdkLlm: ACP_MANAGED_SENTINEL }),
    ).toBeNull();
    expect(resolveEffectiveAcpModel({ runtimeId: "   " })).toBeNull();
    expect(resolveEffectiveAcpModel({})).toBeNull();
  });

  it("falls back to providerDefault only when no concrete model resolves", () => {
    expect(
      resolveEffectiveAcpModel({
        sdkLlm: ACP_MANAGED_SENTINEL,
        providerDefault: "opus[1m]",
      }),
    ).toBe("opus[1m]");
    // A real 'default' wins over providerDefault — it is a concrete model.
    expect(
      resolveEffectiveAcpModel({
        runtimeId: "default",
        providerDefault: "opus[1m]",
      }),
    ).toBe("default");
  });
});

describe("surfaced ACP providers", () => {
  it("surfaces no external ACP harness", () => {
    expect([...SURFACED_ACP_PROVIDERS]).toEqual([]);
  });

  it("offers no external harness on either backend kind", () => {
    expect(getAcpProvidersForBackend("local")).toEqual([]);
    expect(getAcpProvidersForBackend("cloud")).toEqual([]);
  });

  it("surfaces nothing the pinned client registry has dropped", () => {
    // A rename or removal upstream must break loudly rather than leave a tile
    // whose command and models resolve to nothing. An addition stays a no-op.
    expect(
      SURFACED_ACP_PROVIDERS.filter((key) => !(key in CLIENT_ACP_PROVIDERS)),
    ).toEqual([]);
  });

  it("offers no credential fields for a harness it does not surface", () => {
    Object.keys(CLIENT_ACP_PROVIDERS).forEach((key) => {
      expect(getAcpProviderSecrets(key)).toEqual([]);
    });
  });
});

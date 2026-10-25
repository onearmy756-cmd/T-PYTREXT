import { describe, expect, it } from "vitest";

import { backendRequiresAcpCredentials } from "#/components/features/onboarding/steps/setup-acp-secrets-step";

describe("backendRequiresAcpCredentials", () => {
  it("never requires credentials when a login is already detected", () => {
    expect(backendRequiresAcpCredentials("local", "authenticated")).toBe(false);
    expect(backendRequiresAcpCredentials("cloud", "authenticated")).toBe(false);
  });

  it("always requires credentials on a cloud backend (no host login)", () => {
    expect(backendRequiresAcpCredentials("cloud", "unauthenticated")).toBe(
      true,
    );
    expect(backendRequiresAcpCredentials("cloud", "unknown")).toBe(true);
  });

  it("requires credentials on a logged-out local backend (a fresh container)", () => {
    expect(backendRequiresAcpCredentials("local", "unauthenticated")).toBe(
      true,
    );
  });

  it("stays permissive on a local backend the probe can't classify", () => {
    expect(backendRequiresAcpCredentials("local", "unknown")).toBe(false);
  });
});

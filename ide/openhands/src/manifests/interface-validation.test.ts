import { describe, expect, it } from "vitest";
import { validateInterfaceManifest } from "./interface-validation";

const context = {
  catalogIds: new Set<string>(),
  mountedRoutes: {
    list: "/automations",
    setup: "/automations/new/:automationId",
    detail: "/automations/:automationId",
    templates: "/automations/templates",
  },
} as const;

// The rest of the manifest is irrelevant to the docsUrl check: validation runs
// every check and collects errors, so an otherwise-empty manifest still reports
// the docsUrl verdict we assert on.
function docsUrlErrors(docsUrl: unknown): string[] {
  const result = validateInterfaceManifest(
    {
      version: "1.0",
      routes: context.mountedRoutes,
      navigation: {},
      pages: {},
      docsUrl,
      attributes: {},
      importExport: {},
      endpoints: {},
      featuredAutomationIds: [],
      responderIntegrationIds: [],
    },
    context,
  );
  return result.errors.filter((error) => error.startsWith("docsUrl"));
}

describe("validateInterfaceManifest docsUrl admission", () => {
  it("accepts the rebranded documentation domain", () => {
    expect(docsUrlErrors("https://docs.mbilinyi.dev/openhands/usage")).toEqual(
      [],
    );
  });

  it("accepts the pre-rebrand domain the pinned package still publishes", () => {
    expect(
      docsUrlErrors("https://docs.openhands.dev/openhands/usage"),
    ).toEqual([]);
  });

  it("rejects an unrelated domain", () => {
    expect(docsUrlErrors("https://evil.example.com/")).not.toEqual([]);
  });
});

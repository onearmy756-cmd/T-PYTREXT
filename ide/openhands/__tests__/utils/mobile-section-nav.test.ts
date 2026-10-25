import { describe, expect, it } from "vitest";
import { getMobileTopBarState } from "#/utils/mobile-section-nav";
import { I18nKey } from "#/i18n/declaration";

describe("getMobileTopBarState", () => {
  it("shows the menu on the settings hub and main routes", () => {
    expect(getMobileTopBarState("/settings")).toEqual({ mode: "menu" });
    expect(getMobileTopBarState("/conversations")).toEqual({ mode: "menu" });
  });

  it("backs from settings detail pages to the settings hub", () => {
    expect(getMobileTopBarState("/settings/llm")).toEqual({
      mode: "back",
      backTo: "/settings",
      backLabelKey: I18nKey.SETTINGS$TITLE,
    });
  });

  it("shows the menu on every non-settings route", () => {
    // The Customize hub was removed, so its former detail pages are now
    // rail-level destinations with no back affordance.
    expect(getMobileTopBarState("/skills")).toEqual({ mode: "menu" });
    expect(getMobileTopBarState("/mcp")).toEqual({ mode: "menu" });
    expect(getMobileTopBarState("/apps")).toEqual({ mode: "menu" });
    expect(getMobileTopBarState("/extensions/demo-page/hello")).toEqual({
      mode: "menu",
    });
  });
});

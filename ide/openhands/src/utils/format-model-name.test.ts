import { describe, expect, it } from "vitest";
import { formatModelPillLabel } from "./format-model-name";

describe("formatModelPillLabel", () => {
  it("keeps the pretty MBILINYI label without a free suffix unless backend marks it free", () => {
    expect(formatModelPillLabel("openhands/deepseek-v4-flash")).toBe(
      "MBILINYI DeepSeek V4 Flash",
    );

    expect(
      formatModelPillLabel(
        "openhands/deepseek-v4-flash",
        new Set(["openhands/deepseek-v4-flash"]),
      ),
    ).toBe("MBILINYI DeepSeek V4 Flash (free)");
  });
});

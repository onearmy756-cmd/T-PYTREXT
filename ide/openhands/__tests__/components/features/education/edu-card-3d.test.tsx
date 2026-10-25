import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { fireEvent } from "@testing-library/react";
import { EduCard3D } from "#/components/features/education/edu-ui";

describe("EduCard3D", () => {
  it("huonyesha maudhui na huzunguka kwa mwendo wa kursa", () => {
    render(
      <EduCard3D testId="c3d">
        <span>Huduma ya AI</span>
      </EduCard3D>,
    );
    const card = screen.getByTestId("c3d");
    expect(card).toBeInTheDocument();
    expect(screen.getByText("Huduma ya AI")).toBeInTheDocument();

    // Awali hakuna mzunguko.
    expect(card.style.getPropertyValue("--c3d-ry")).toBe("");

    fireEvent.mouseMove(card, { clientX: 200, clientY: 50 });
    expect(card.style.getPropertyValue("--c3d-ry")).not.toBe("");

    fireEvent.mouseLeave(card);
    expect(card.style.getPropertyValue("--c3d-ry")).toBe("0deg");
    expect(card.style.getPropertyValue("--c3d-rx")).toBe("0deg");
  });

  it("huweka accent ya primary kwenye border", () => {
    render(<EduCard3D testId="c3d-accent" accent>maudhui</EduCard3D>);
    expect(screen.getByTestId("c3d-accent").className).toContain(
      "border-primary/60",
    );
  });
});

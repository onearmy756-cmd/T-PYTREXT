import type { AgentServerUICssVariableName } from "#/styles/agent-server-ui-style-scope";
import type { ColorThemeDefinition } from "../types";
import { MBILINYI_HEROUI, MBILINYI_SCALE } from "../palettes/mbilinyi";

/**
 * MBILINYI — obsidian ember identity.
 *
 * The signature look of MBILINYI IDE: a warm obsidian-black canvas heated by
 * an ember (orange-red) core, with cold forged-steel cyan as the secondary
 * signal. Deliberately not a blue-grey coding-agent skin.
 */
const MBILINYI_TOKENS: Partial<Record<AgentServerUICssVariableName, string>> = {
  // Ember drives buttons and focus; steel cyan is the cold counter-accent.
  "--oh-color-primary": "#FF6B2C",
  "--oh-accent": "#38BDF8",
  "--oh-accent-foreground": "#0A0503",
  "--oh-warning": "#FBBF24",
  "--oh-color-logo": "#FF6B2C",
  "--oh-focus": "#FF6B2C",
  "--oh-link": "#7DD3FC",
  "--oh-success": "#4ADE80",
  "--oh-danger": "#F0716B",
  // Sharper, more architectural corners than the stock soft skin.
  "--oh-radius": "6px",
  "--oh-field-radius": "8px",
};

export const mbilinyi: ColorThemeDefinition = {
  label: "MBILINYI",
  appearance: "dark",
  scale: MBILINYI_SCALE,
  heroui: MBILINYI_HEROUI,
  tokens: MBILINYI_TOKENS,
};

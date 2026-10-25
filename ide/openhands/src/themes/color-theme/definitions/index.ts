import type { ColorThemeDefinition, ColorThemeKey } from "../types";
import { lightPlus } from "./light-plus";
import { mbilinyi } from "./mbilinyi";
import { mbilinyiDeepsea } from "./mbilinyi-deepsea";
import { mbilinyiNeutral } from "./mbilinyi-neutral";
import { mbilinyiNeo } from "./mbilinyi-neo";
import { solarizedLight } from "./solarized-light";

/**
 * The theme registry.
 *
 * To add a theme: create a sibling module exporting a `ColorThemeDefinition`,
 * add its key to `ColorThemeKey` in `../types.ts`, and register it below.
 */
export const COLOR_THEMES: Record<ColorThemeKey, ColorThemeDefinition> = {
  mbilinyi,
  "mbilinyi-deepsea": mbilinyiDeepsea,
  "mbilinyi-neutral": mbilinyiNeutral,
  "mbilinyi-neo": mbilinyiNeo,
  "light-plus": lightPlus,
  "solarized-light": solarizedLight,
};

export const DEFAULT_COLOR_THEME: ColorThemeKey = "mbilinyi";

export const AVAILABLE_COLOR_THEMES = Object.entries(COLOR_THEMES).map(
  ([key, def]) => ({ key: key as ColorThemeKey, label: def.label }),
);

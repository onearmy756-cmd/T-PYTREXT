import {
  solarizedlight,
  vs,
  vscDarkPlus,
} from "react-syntax-highlighter/dist/esm/styles/prism";
import type { ColorThemeKey } from "#/themes/color-themes";

// Exhaustive: adding a Canvas theme requires an explicit syntax palette.
const SYNTAX_THEMES: Record<ColorThemeKey, typeof vs> = {
  mbilinyi: vscDarkPlus,
  "mbilinyi-deepsea": vscDarkPlus,
  "mbilinyi-neutral": vscDarkPlus,
  "mbilinyi-neo": vscDarkPlus,
  "light-plus": vs,
  "solarized-light": solarizedlight,
};

export function getSyntaxHighlighterTheme(theme: ColorThemeKey) {
  return SYNTAX_THEMES[theme];
}

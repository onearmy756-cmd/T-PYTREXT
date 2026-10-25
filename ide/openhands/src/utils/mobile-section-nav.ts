import { I18nKey } from "#/i18n/declaration";

const SETTINGS_PREFIX = "/settings";

export type MobileTopBarMode = "menu" | "back";

export interface MobileTopBarState {
  mode: MobileTopBarMode;
  backTo?: string;
  backLabelKey?: I18nKey;
}

export function getMobileTopBarState(pathname: string): MobileTopBarState {
  if (pathname === SETTINGS_PREFIX) {
    return { mode: "menu" };
  }

  if (
    pathname.startsWith(`${SETTINGS_PREFIX}/`) &&
    pathname.length > SETTINGS_PREFIX.length
  ) {
    return {
      mode: "back",
      backTo: SETTINGS_PREFIX,
      backLabelKey: I18nKey.SETTINGS$TITLE,
    };
  }

  return { mode: "menu" };
}

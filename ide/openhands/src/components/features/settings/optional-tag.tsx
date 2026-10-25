import { useTranslation } from "react-i18next";
import { I18nKey } from "#/i18n/declaration";

export function OptionalTag() {
  const { t } = useTranslation("mbilinyi");
  return (
    <span className="text-xs text-tertiary-alt">
      {t(I18nKey.COMMON$OPTIONAL)}
    </span>
  );
}

import { useTranslation } from "react-i18next";
import { Typography } from "#/ui/typography";
import { I18nKey } from "#/i18n/declaration";

export function HomeHeaderTitle() {
  const { t } = useTranslation("mbilinyi");

  return (
    <div className="flex w-full flex-col items-center gap-3 py-2">
      <span className="mb-hero-kicker">{t(I18nKey.HOME$HERO_KICKER)}</span>
      <Typography.H1 className="mb-hero-title w-full text-center text-[34px] leading-tight">
        {t(I18nKey.HOME$WHAT_TO_WORK_ON)}
      </Typography.H1>
      <span className="mb-hero-rule" aria-hidden />
      <p
        data-testid="home-header-subtitle"
        className="max-w-xl text-center text-[var(--oh-text-tertiary)]"
      >
        {t(I18nKey.HOME$ENGINEERING_TASKS_SUBHEADER)}
      </p>
    </div>
  );
}

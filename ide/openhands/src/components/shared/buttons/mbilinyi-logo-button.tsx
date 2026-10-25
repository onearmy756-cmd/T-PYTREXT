import { useTranslation } from "react-i18next";
import MbilinyiLogo from "#/assets/branding/mbilinyi-logo.svg?react";
import { NavigationLink } from "#/components/shared/navigation-link";
import { I18nKey } from "#/i18n/declaration";
import { cn } from "#/utils/utils";

const DEFAULT_LOGO_WIDTH = 46;
const DEFAULT_LOGO_HEIGHT = 30;

export type MbilinyiLogoButtonProps = {
  className?: string;
  /** Applied to the root `<svg>` (e.g. `max-w-none` so Tailwind preflight doesn’t clamp wide marks inside a narrow flex slot). */
  logoClassName?: string;
  logoWidth?: number;
  logoHeight?: number;
};

export function MbilinyiLogoButton({
  className,
  logoClassName,
  logoWidth = DEFAULT_LOGO_WIDTH,
  logoHeight = DEFAULT_LOGO_HEIGHT,
}: MbilinyiLogoButtonProps = {}) {
  const { t } = useTranslation("mbilinyi");

  const ariaLabel = t(I18nKey.BRANDING$OPENHANDS_LOGO);

  return (
    <NavigationLink
      to="/conversations"
      aria-label={ariaLabel}
      className={cn(className)}
    >
      <MbilinyiLogo
        width={logoWidth}
        height={logoHeight}
        className={cn("shrink-0 text-logo", logoClassName)}
      />
    </NavigationLink>
  );
}

import { useTranslation } from "react-i18next";
import {
  gitControlBarActionButtonClassName,
  gitControlBarActionIconColor,
  gitControlBarActionLabelClassName,
} from "#/utils/git-control-bar-classes";
import { MTECHOS_PLATFORM_ACTIONS } from "#/utils/mtechos-platform-actions";

/**
 * MTECHOS quick actions — vitufe vya platform ndogo ndani ya chat input bar ya
 * MBILINYI (MTECHOS bridge). Orodha inatoka kwenye CHANZO KIMOJA
 * (utils/mtechos-platform-actions.tsx) — kila action inatuma prompt halisi
 * ndani ya chat — hakuna UI mpya, hakuna iframe, hakuna ukurasa mwingine.
 * Mfumo mmoja; ongeza action mpya kwenye chanzo kimoja pekee.
 */
interface MtechosQuickActionsProps {
  /** Prompts hupelekwa chat yenyewe — handler ni ile ile ya suggestions. */
  onSuggestionsClick: (value: string) => void;
  isConversationReady?: boolean;
}

export function MtechosQuickActions({
  onSuggestionsClick,
  isConversationReady = true,
}: MtechosQuickActionsProps) {
  const { t } = useTranslation("mbilinyi");

  return (
    <div
      className="flex flex-row gap-2.5 items-center overflow-x-auto flex-nowrap relative scrollbar-hide"
      data-testid="mtechos-quick-actions"
    >
      {MTECHOS_PLATFORM_ACTIONS.map((action) => (
        <button
          key={action.testId}
          type="button"
          data-testid={action.testId}
          disabled={!isConversationReady}
          className={gitControlBarActionButtonClassName(isConversationReady)}
          onClick={() => onSuggestionsClick(action.prompt)}
        >
          <span
            style={{ color: gitControlBarActionIconColor(isConversationReady) }}
          >
            {action.icon}
          </span>
          <span className={gitControlBarActionLabelClassName}>
            {t(action.key)}
          </span>
        </button>
      ))}
    </div>
  );
}

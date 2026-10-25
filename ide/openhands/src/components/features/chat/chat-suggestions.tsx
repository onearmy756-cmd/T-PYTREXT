import { useTranslation } from "react-i18next";
import { motion, AnimatePresence } from "framer-motion";
import { Suggestions } from "#/components/features/suggestions/suggestions";
import { I18nKey } from "#/i18n/declaration";
import { SUGGESTIONS } from "#/utils/suggestions";
import { MTECHOS_PLATFORM_ACTIONS } from "#/utils/mtechos-platform-actions";
import { useConversationStore } from "#/stores/conversation-store";

interface ChatSuggestionsProps {
  onSuggestionsClick: (value: string) => void;
}

export function ChatSuggestions({ onSuggestionsClick }: ChatSuggestionsProps) {
  const { t } = useTranslation("mbilinyi");
  const { shouldHideSuggestions } = useConversationStore();

  return (
    <AnimatePresence>
      {!shouldHideSuggestions && (
        <motion.div
          data-testid="chat-suggestions"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.3, ease: "easeInOut" }}
          className="pointer-events-auto absolute inset-x-4 bottom-37.75 top-0 flex flex-col items-center justify-center md:inset-x-8"
        >
          <div className="flex flex-col items-center p-4 rounded-xl w-full">
            <span className="pb-6 text-[32px] font-medium leading-5 text-contrast">
              {t(I18nKey.LANDING$TITLE)}
            </span>
          </div>
          <Suggestions
            suggestions={[
              // MTECHOS platform actions (CHANZO KIMOJA): kila button inaingia
              // chat ile ile — hakuna UI mpya, kazi inatekelezwa na agent na
              // API za MBILINYI IDE (/api/forge/*, /api/sdlc/*, /api/ide/*…).
              ...MTECHOS_PLATFORM_ACTIONS.map((action) => ({
                label: t(action.key),
                value: action.prompt,
              })),
              ...Object.entries(SUGGESTIONS.repo)
                .slice(0, 4)
                .map(([label, value]) => ({
                  label,
                  value,
                })),
            ]}
            onSuggestionClick={onSuggestionsClick}
          />
        </motion.div>
      )}
    </AnimatePresence>
  );
}

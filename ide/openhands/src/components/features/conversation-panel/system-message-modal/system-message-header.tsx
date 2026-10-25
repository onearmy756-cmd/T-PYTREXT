import { useTranslation } from "react-i18next";
import { BaseModalTitle } from "#/components/shared/modals/confirmation-modals/base-modal";
import { ModalCloseButton } from "#/components/shared/modals/modal-close-button";
import { Typography } from "#/ui/typography";
import { I18nKey } from "#/i18n/declaration";

interface SystemMessageHeaderProps {
  agentClass: string | null;
  mbilinyiVersion: string | null;
  onClose: () => void;
}

export function SystemMessageHeader({
  agentClass,
  mbilinyiVersion,
  onClose,
}: SystemMessageHeaderProps) {
  const { t } = useTranslation("mbilinyi");

  return (
    <>
      <ModalCloseButton onClose={onClose} testId="close-system-message-modal" />
      <div className="flex w-full min-w-0 flex-col gap-2 pr-6">
        <BaseModalTitle title={t(I18nKey.SYSTEM_MESSAGE_MODAL$TITLE)} />
        {(agentClass || mbilinyiVersion) && (
          <div className="flex flex-col gap-2">
            {agentClass && (
              <div className="text-sm">
                <Typography.Text className="font-semibold text-text-tertiary">
                  {t(I18nKey.SYSTEM_MESSAGE_MODAL$AGENT_CLASS)}
                </Typography.Text>{" "}
                <Typography.Text className="font-medium text-content-2">
                  {agentClass}
                </Typography.Text>
              </div>
            )}
            {mbilinyiVersion && (
              <div className="text-sm">
                <Typography.Text className="font-semibold text-text-tertiary">
                  {t(I18nKey.SYSTEM_MESSAGE_MODAL$OPENHANDS_VERSION)}
                </Typography.Text>{" "}
                <Typography.Text className="text-content-2">
                  {mbilinyiVersion}
                </Typography.Text>
              </div>
            )}
          </div>
        )}
      </div>
    </>
  );
}

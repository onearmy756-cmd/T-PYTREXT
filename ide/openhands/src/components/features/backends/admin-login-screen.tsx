import React from "react";
import { loginMtechosAdmin } from "#/api/auth/mtechos-auth.api";
import { isNoBackend } from "#/api/backend-registry/active-store";
import { useActiveBackendContext } from "#/contexts/active-backend-context";
import { BrandButton } from "#/components/features/settings/brand-button";
import { EduInput } from "#/components/features/education/edu-ui";
import {
  MODAL_MAX_WIDTH_VIEWPORT,
  modalWidthClassName,
} from "#/components/shared/modals/modal-body";
import { cn } from "#/utils/utils";

const ApiKeyEntryScreen = React.lazy(
  () => import("#/components/features/backends/api-key-entry-screen"),
);

/**
 * Skrini ya kuingia ya MTECHOS — login HALISI (username/password).
 *
 * Hii hutumika wakati server HAIIngizi key kwenye HTML
 * (`AGENT_CANVAS_ALLOW_LAN_SESSION_KEY` haipo/false), ambayo ni hali salama
 * ya default: mgeni hawezi kuiba token kwa `curl /ide`.
 *
 * Mtiririko: `POST /api/auth/login` → token ya `sessions` → hifadhi kama
 * `apiKey` ya backend ya local (localStorage) → reload. Token hiyo inakubalika
 * na `auth_mw` (protocol) na `rbac_mw` (kazi za Admin Mkuu).
 *
 * Kwa mipangilio ya hali ya juu (backend ya mbali / cloud), mtumiaji anaweza
 * kubadilisha kwenda skrini ya API key.
 */
export default function AdminLoginScreen() {
  const { active, addBackend, updateBackend } = useActiveBackendContext();
  const [username, setUsername] = React.useState("");
  const [password, setPassword] = React.useState("");
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [showApiKey, setShowApiKey] = React.useState(false);

  const host = window.location.origin;

  const submit = React.useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();
      if (!username.trim() || !password) return;
      setBusy(true);
      setError(null);
      try {
        const data = await loginMtechosAdmin(username.trim(), password);
        const token = data.session?.token;
        if (!data.ok || !token) {
          setError(data.error || "Jina la mtumiaji au password si sahihi.");
          setBusy(false);
          return;
        }
        const payload = {
          name: "MTECHOS",
          host,
          apiKey: token,
          kind: "local" as const,
        };
        if (isNoBackend(active.backend)) {
          addBackend(payload);
        } else {
          updateBackend(active.backend.id, payload);
        }
        window.location.reload();
      } catch (err) {
        setError(
          err instanceof Error
            ? `Imeshindwa kuunganisha: ${err.message}`
            : "Imeshindwa kuunganisha na server.",
        );
        setBusy(false);
      }
    },
    [username, password, host, active.backend, addBackend, updateBackend],
  );

  if (showApiKey) {
    return (
      <React.Suspense fallback={null}>
        <ApiKeyEntryScreen />
      </React.Suspense>
    );
  }

  return (
    <div
      data-testid="admin-login-screen"
      className="flex min-h-screen items-center justify-center bg-canvas-base px-6"
    >
      <form
        onSubmit={submit}
        className={cn(
          "relative rounded-xl border border-border bg-base-secondary",
          modalWidthClassName("md"),
          MODAL_MAX_WIDTH_VIEWPORT,
        )}
      >
        <div className="px-6 pt-6 pb-2">
          <h2 className="text-lg font-semibold">Ingia MTECHOS</h2>
          <p className="mt-1 text-xs text-muted">
            Tumia akaunti yako ya MTECHOS (Admin Mkuu: MBILINYI).
          </p>
        </div>

        <div className="flex flex-col gap-3 px-6 pt-2 pb-6">
          <EduInput
            testId="admin-login-username"
            autoComplete="username"
            placeholder="Jina la mtumiaji"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
          />
          <EduInput
            testId="admin-login-password"
            type="password"
            autoComplete="current-password"
            placeholder="Password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />

          {error ? (
            <div
              data-testid="admin-login-error"
              className="text-sm text-red-400"
            >
              {error}
            </div>
          ) : null}

          <BrandButton
            type="submit"
            variant="secondary"
            isDisabled={busy || !username.trim() || !password}
            testId="admin-login-submit"
            className="w-full text-center"
          >
            {busy ? "Inaingia…" : "Ingia"}
          </BrandButton>

          <button
            type="button"
            data-testid="admin-login-use-api-key"
            onClick={() => setShowApiKey(true)}
            className="text-xs text-muted underline hover:text-content"
          >
            Weka API key / backend ya mbali
          </button>
        </div>
      </form>
    </div>
  );
}

/**
 * Source of truth for "is a newer MBILINYI IDE published?".
 *
 * The npm registry `latest` dist-tag endpoint is CORS-open and returns the
 * abbreviated packument for that version as JSON. Isolated here so the
 * release channel can change (e.g. GitHub releases) without touching the
 * query hook or the settings update card.
 */
const NPM_LATEST_VERSION_URL =
  "https://registry.npmjs.org/@openhands/agent-canvas/latest";

/**
 * MTECHOS is self-hosted and white-labelled: the upstream npm/GitHub release
 * feed is not the user's release channel, and on an air-gapped LAN the request
 * hangs until the network times out, leaving a spurious "new version" banner.
 * Release checks stay available when a deployment explicitly opts in.
 */
export function releaseUpdateCheckEnabled(): boolean {
  return (
    typeof window !== "undefined" &&
    (window as unknown as Record<string, unknown>)
      .__MBILINYI_ENABLE_UPDATE_CHECK__ === true
  );
}

export const AGENT_CANVAS_RELEASE_NOTES_URL =
  "https://github.com/mbilinyitech/MBILINYI/releases";

/** Literal shell commands — intentionally not localized. */
export const AGENT_CANVAS_UPDATE_COMMANDS = {
  npm: "npm install -g @openhands/agent-canvas@latest",
  docker: "docker pull ghcr.io/openhands/agent-canvas:latest",
} as const;

export async function fetchLatestAgentCanvasVersion(
  signal?: AbortSignal,
): Promise<string> {
  if (!releaseUpdateCheckEnabled()) {
    throw new Error("release update check is disabled on this deployment");
  }
  const response = await fetch(NPM_LATEST_VERSION_URL, {
    signal,
    headers: { Accept: "application/json" },
  });
  if (!response.ok) {
    throw new Error(`npm registry responded ${response.status}`);
  }
  const body: unknown = await response.json();
  const version = (body as { version?: unknown } | null)?.version;
  if (typeof version !== "string" || !version.trim()) {
    throw new Error("npm registry response missing version");
  }
  return version.trim();
}

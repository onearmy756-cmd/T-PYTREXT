import { McpSettingsContent } from "./mcp";

/**
 * Settings → MCP (`/settings/mcp`). Reuses the same editor as the standalone
 * `/mcp` route, inside the settings layout so MCP servers are configured from
 * Settings alongside LLM, Models and ACP.
 */
export default function McpSettingsPage() {
  return (
    <div data-testid="mcp-settings-screen" className="flex flex-col gap-6">
      <McpSettingsContent showHeader={false} />
    </div>
  );
}

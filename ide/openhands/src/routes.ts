import {
  type RouteConfig,
  layout,
  index,
  route,
} from "@react-router/dev/routes";

export default [
  layout("routes/root-layout.tsx", [
    index("routes/index-home.tsx"),
    route("conversations", "routes/home.tsx"),
    route(
      "conversations/:conversationId/panel",
      "routes/conversation-panel.tsx",
    ),
    route("conversations/:conversationId", "routes/conversation.tsx"),
    route("launch", "routes/launch.tsx"),
    route("elimu", "routes/education.tsx"),
    route("mtechos", "routes/mtechos-group.tsx"),
    route("cyber", "routes/cyber-security.tsx"),
    route("mtihani", "routes/mtihani.tsx"),
    route("extensions/:extensionName/*", "routes/canvas-extension-page.tsx"),
    route("mcp", "routes/mcp.tsx"),
    route("settings", "routes/settings.tsx", [
      index("routes/settings-index.tsx"),
      route("llm", "routes/llm-settings.tsx"),
      route("meta-llm", "routes/meta-llm-settings.tsx"),
      route("mcp", "routes/settings-mcp.tsx"),
      route("agent", "routes/agent-settings.tsx"),
      route("agents", "routes/agent-profiles-settings.tsx"),
      route("condenser", "routes/condenser-settings.tsx"),
      route("agent-context", "routes/agent-context-settings.tsx"),
      route("verification", "routes/verification-settings.tsx"),
      route("app", "routes/app-settings.tsx"),
      route("secrets", "routes/secrets-settings.tsx"),
    ]),
    route("oauth/device/verify", "routes/device-verify.tsx"),
    route("automations", "routes/automations-list.tsx"),
    route("automations/git-sync", "routes/automation-git-sync.tsx"),
    route("automations/templates", "routes/automation-templates.tsx"),
    route("automations/new/:automationId", "routes/automation-setup-route.tsx"),
    route("automations/setup", "routes/automation-setup-new.tsx"),
    route("automations/:automationId", "routes/automation-detail.tsx"),
    // Unknown URLs (including /settings/<x>) render inside the app shell.
    route("*", "routes/not-found.tsx"),
  ]),
  route(
    "shared/conversations/:conversationId",
    "routes/shared-conversation.tsx",
  ),
  // Umma: mteja hana login — hufuatilia ombi kwa kumbukumbu + simu. Haipo
  // chini ya root-layout kwa sababu layout hiyo inahitaji uthibitisho.
  route("mteja", "routes/mteja-track.tsx"),
] satisfies RouteConfig;

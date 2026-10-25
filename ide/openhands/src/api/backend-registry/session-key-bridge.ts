/**
 * Daraja la token ya login kati ya backend registry na `agent-server-config`.
 *
 * Hali ya usalama ya default (`AGENT_CANVAS_ALLOW_LAN_SESSION_KEY` haipo)
 * HAIINGIZI key kwenye HTML; badala yake mtumiaji huingia kwa username/password
 * na token huhifadhiwa kwenye registry (`mbilinyi-backends`). `active-store`
 * huiandikisha hapa, na `getAgentServerHeaders()` huitumia — vinginevyo kila
 * wito wa `/api/biashara/*` ungekataliwa (401) ingawa mtumiaji ameingia.
 *
 * Moduli hii HAINA imports nyingine kwa makusudi: inge-import
 * `agent-server-config` au `active-store` ingezalisha mzunguko wa import.
 */
type SessionApiKeyResolver = () => string | null;

let resolver: SessionApiKeyResolver | null = null;

export function registerRegistrySessionApiKeyResolver(
  next: SessionApiKeyResolver,
): void {
  resolver = next;
}

export function resolveRegistrySessionApiKey(): string | null {
  return resolver?.() ?? null;
}

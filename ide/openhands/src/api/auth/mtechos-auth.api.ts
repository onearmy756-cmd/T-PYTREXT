/**
 * MTECHOS — uthibitishaji wa Admin Mkuu (`/api/auth/login`).
 *
 * Hali salama ya default HAIINGIZI session key kwenye HTML, hivyo skrini ya
 * login (`components/features/backends/admin-login-screen.tsx`) hupata token
 * kupitia hapa na kuiweka kwenye backend registry kama `apiKey`.
 */

import { getAgentServerBaseUrl } from "#/api/agent-server-config";

export interface MtechosLoginSession {
  token: string;
  username?: string;
  role?: string;
  expires_at?: string;
}

export interface MtechosLoginResult {
  ok: boolean;
  error?: string;
  session?: MtechosLoginSession;
}

export async function loginMtechosAdmin(
  username: string,
  password: string,
): Promise<MtechosLoginResult> {
  const base = getAgentServerBaseUrl();
  if (!base) throw new Error("Backend base URL haijulikani");

  const url = `${base.replace(/\/+$/, "")}/api/auth/login`;
  const res = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ username, password }),
  });

  return (await res.json().catch(() => ({ ok: false }))) as MtechosLoginResult;
}

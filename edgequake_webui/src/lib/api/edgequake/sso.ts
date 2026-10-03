/**
 * SPEC-158 — SSO API module (provider discovery, handoff redemption, login URL).
 *
 * The browser never sees an IdP token: it is redirected to the API, which talks to the IdP and
 * returns here with an opaque single-use `?code=` redeemed by {@link redeemSsoHandoff}.
 */

import { getRuntimeApiBaseUrl } from "@/lib/runtime-config";
import type { LoginResponse } from "@/types";
import { api } from "../client";

export interface SsoProvider {
  slug: string;
  display_name: string;
  /** `keycloak` | `google` | `microsoft` | `aws` | `github` | `generic` | ... */
  kind: string;
  login_path: string;
}

export interface SsoHandoffResponse extends LoginResponse {
  tenant_id: string;
  workspace_id?: string | null;
  /** Server-validated same-origin path; never an absolute URL. */
  redirect_after?: string | null;
}

/** Same-origin relative path only (blocks open redirects). Shared by login + callback. */
export function safeRedirectPath(raw: string | null | undefined): string | null {
  if (!raw || !raw.startsWith("/") || raw.startsWith("//")) return null;
  if (raw.includes("://") || raw.includes("\\")) return null;
  return raw;
}

export async function listSsoProviders(): Promise<SsoProvider[]> {
  const res = await api.get<{ providers: SsoProvider[] }>("/auth/sso/providers");
  return res.providers ?? [];
}

export async function redeemSsoHandoff(code: string): Promise<SsoHandoffResponse> {
  return api.post<SsoHandoffResponse>("/auth/handoff", { code });
}

export interface SsoLoginTarget {
  /** Provider slug; omitted → the server's default provider. */
  provider?: string;
  /** Keycloak Organization alias hint (maps to a tenant slug). */
  org?: string;
  /** Same-origin path to land on after login. */
  redirect?: string | null;
}

/** Absolute-or-relative URL that starts the SSO flow (full-page navigation, not fetch). */
export function buildSsoLoginUrl(target: SsoLoginTarget): string {
  const params = new URLSearchParams();
  if (target.provider) params.set("provider", target.provider);
  const org = target.org?.trim();
  if (org) params.set("org", org);
  const redirect = safeRedirectPath(target.redirect);
  if (redirect && redirect !== "/") params.set("redirect", redirect);
  const qs = params.toString();
  return `${getRuntimeApiBaseUrl()}/auth/oidc/login${qs ? `?${qs}` : ""}`;
}

/** Persisted provider definition (admin API; the client secret is only ever an env-var *name*). */
export interface StoredIdentityProvider {
  slug: string;
  kind: string;
  display_name: string;
  issuer: string;
  client_id: string;
  client_secret_ref?: string | null;
  redirect_uri: string;
  trust_email: boolean;
  link_policy: string;
  jit_enabled: boolean;
  role_claim: string;
  max_role: string;
  enabled: boolean;
}

export async function listIdentityProviders(): Promise<StoredIdentityProvider[]> {
  return api.get<StoredIdentityProvider[]>("/admin/identity-providers");
}

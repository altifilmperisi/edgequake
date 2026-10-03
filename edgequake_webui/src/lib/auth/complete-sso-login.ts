/**
 * SPEC-158 — finish an SSO login: redeem the handoff code, adopt the session, pin the tenant.
 *
 * SRP: this is the only place that turns a handoff response into client state, so the callback
 * page stays presentational and the sequence is unit-testable.
 */

import { redeemSsoHandoff, safeRedirectPath, type SsoHandoffResponse } from "@/lib/api/edgequake/sso";
import { useAuthStore } from "@/stores/use-auth-store";
import { useTenantStore } from "@/stores/use-tenant-store";

export interface SsoLoginOutcome {
  response: SsoHandoffResponse;
  /** Validated same-origin landing path. */
  redirectTo: string;
}

export async function completeSsoLogin(code: string): Promise<SsoLoginOutcome> {
  const response = await redeemSsoHandoff(code);
  useAuthStore.getState().login(response);

  // The token is already scoped to this tenant (LAW-158-2); mirror it in the UI selection so the
  // tenant provider never auto-selects a different one.
  const tenants = useTenantStore.getState();
  tenants.selectTenant(response.tenant_id);
  if (response.workspace_id) tenants.selectWorkspace(response.workspace_id);

  return { response, redirectTo: safeRedirectPath(response.redirect_after) ?? "/" };
}

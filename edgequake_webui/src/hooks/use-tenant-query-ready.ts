"use client";

/**
 * @module use-tenant-query-ready
 * @description Gate for tenant-scoped React Query hooks.
 *
 * WHY: The backend rejects tenant-scoped routes (`/folders`, `/conversations`,
 * ...) with 400 "Missing X-Tenant-ID header" when the request is sent before
 * the tenant store has hydrated and published its context to the API client.
 * Firing those requests first is pure noise: a guaranteed 4xx, a WARN in the
 * server log and a flash of error UI. Waiting for hydration + a selected tenant
 * makes the request correct by construction instead of fixing it by retry.
 */

import { useTenantStore } from "@/stores/use-tenant-store";

/** True once the tenant store hydrated and a tenant is selected. */
export function useTenantQueryReady(): boolean {
  const hydrated = useTenantStore((s) => s._hasHydrated);
  const tenantId = useTenantStore((s) => s.selectedTenantId);
  return hydrated && !!tenantId;
}

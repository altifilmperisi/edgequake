'use client';

import { listSsoProviders, type SsoProvider } from '@/lib/api/edgequake/sso';
import { useQuery } from '@tanstack/react-query';

/** SPEC-158 — public, secret-free provider list for the login page (empty when SSO is off). */
export function useSsoProviders() {
  return useQuery<SsoProvider[]>({
    queryKey: ['auth', 'sso-providers'],
    queryFn: listSsoProviders,
    staleTime: 60_000,
    retry: 0,
  });
}

'use client';

import { DynamicBreadcrumb } from '@/components/layout/dynamic-breadcrumb';
import { Header } from '@/components/layout/header';
import { Sidebar } from '@/components/layout/sidebar';
import { SkipLink } from '@/components/shared/skip-link';

/**
 * Layout for workspace deeplink routes.
 *
 * SPEC-155: shortcuts live only in KeyboardShortcutsProvider (no duplicate mount).
 * Full AuthGuard parity is W7; this wave only removes the double keydown listener.
 */
export default function WorkspaceDeeplinkLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="flex h-dvh max-h-dvh min-h-0 overflow-clip bg-background">
      <SkipLink />
      <Sidebar />
      <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-clip">
        <Header />
        <DynamicBreadcrumb />
        <main id="main-content" className="min-h-0 flex-1 overflow-hidden" tabIndex={-1}>
          {children}
        </main>
      </div>
    </div>
  );
}

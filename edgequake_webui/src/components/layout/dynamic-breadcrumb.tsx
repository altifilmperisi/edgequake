'use client';

import {
    Breadcrumb,
    BreadcrumbItem,
    BreadcrumbLink,
    BreadcrumbList,
    BreadcrumbPage,
    BreadcrumbSeparator,
} from '@/components/ui/breadcrumb';
import { getDocument } from '@/lib/api/edgequake';
import {
    extractDocumentIdFromPath,
    formatGuidShort,
    isDocumentIdSegment,
    resolveDocumentBreadcrumbLabel,
} from '@/lib/layout/breadcrumb-document-label';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useQuery } from '@tanstack/react-query';
import {
  BookOpen,
  Building2,
  ChevronRight,
  FileText,
  GitBranch,
  Home,
  MessageSquare,
  Network,
  Settings,
  Terminal,
  Wallet,
} from 'lucide-react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import React from 'react';

interface PathConfig {
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  description?: string;
}

const pathConfig: Record<string, PathConfig> = {
  '': { label: 'Dashboard', icon: Home, description: 'Overview and statistics' },
  'graph': { label: 'Knowledge Graph', icon: Network, description: 'Visualize entities and relationships' },
  'documents': { label: 'Documents', icon: FileText, description: 'Manage your documents' },
  'query': { label: 'Query', icon: MessageSquare, description: 'Ask questions' },
  'pipeline': { label: 'Pipeline', icon: GitBranch, description: 'Ingestion pipeline' },
  'costs': { label: 'Costs', icon: Wallet, description: 'Usage and spend' },
  'knowledge': { label: 'Knowledge', icon: BookOpen, description: 'Knowledge injections' },
  'workspace': { label: 'Workspace', icon: Building2, description: 'Workspace settings' },
  'api-explorer': { label: 'API Explorer', icon: Terminal, description: 'Test API endpoints' },
  'settings': { label: 'Settings', icon: Settings, description: 'Configure preferences' },
};

interface CrumbItem {
  label: string;
  href: string;
  icon?: React.ComponentType<{ className?: string }>;
  /** Full document id — always shown alongside label for document slugs */
  guid?: string;
}

interface DynamicBreadcrumbProps {
  /** Additional custom segments to append */
  customSegments?: Array<{ label: string; href?: string }>;
}

export function DynamicBreadcrumb({ customSegments }: DynamicBreadcrumbProps) {
  const pathname = usePathname();
  const { selectedWorkspaceId } = useTenantStore();
  const documentId = extractDocumentIdFromPath(pathname);

  // Share cache with document detail page — no extra round-trip when already loaded.
  const { data: document } = useQuery({
    queryKey: ['document', documentId, selectedWorkspaceId],
    queryFn: () => getDocument(documentId!),
    enabled: Boolean(documentId && selectedWorkspaceId),
    staleTime: 30_000,
  });
  const documentLabel = resolveDocumentBreadcrumbLabel(document);

  const segments = pathname.split('/').filter(Boolean);

  const items: CrumbItem[] = [
    { label: 'EdgeQuake', href: '/', icon: Home },
  ];

  let currentPath = '';
  segments.forEach((segment, index) => {
    currentPath += `/${segment}`;
    // Skip workspace-slug mount prefix `/w/:slug` — not a breadcrumb crumb.
    if (segments[0] === 'w' && (index === 0 || index === 1)) {
      return;
    }

    const config = pathConfig[segment];
    if (config) {
      items.push({
        label: config.label,
        href: currentPath,
        icon: config.icon,
      });
      return;
    }

    if (isDocumentIdSegment(segment)) {
      // Always show human label + GUID for document slugs (never GUID-only).
      items.push({
        label: documentLabel ?? 'Document',
        href: currentPath,
        guid: segment.replace(/^staging:/i, ''),
      });
      return;
    }

    // Other dynamic segments (entity ids, etc.) — keep short fallback.
    items.push({
      label: decodeURIComponent(segment).slice(0, 12) + (segment.length > 12 ? '...' : ''),
      href: currentPath,
    });
  });

  if (customSegments) {
    customSegments.forEach((seg) => {
      items.push({
        label: seg.label,
        href: seg.href || '#',
      });
    });
  }

  // Depth ≤ 1 (/, /documents, /query, …): no band — sidebar already marks
  // location, and an empty h-9 spacer read as a layout hole above the page
  // title. Depth ≥ 2 (e.g. /documents/:id): paint the breadcrumb bar.
  // WHY no reserved spacer: list→detail navigation is user-initiated, so the
  // brief band appearance is expected (web.dev CLS: hadRecentInput).
  if (items.length <= 2) {
    return null;
  }

  return (
    <div
      className="h-9 shrink-0 border-b px-4 bg-muted/20 flex items-center"
      data-testid="breadcrumb-bar"
    >
    <Breadcrumb>
      <BreadcrumbList>
        {items.map((item, index) => {
          const isLast = index === items.length - 1;
          const Icon = item.icon;
          const crumbTitle = item.guid
            ? `${item.label} (${item.guid})`
            : item.label;

          const crumbBody = (
            <>
              {Icon && <Icon className="h-3 w-3 shrink-0" />}
              <span
                className="truncate max-w-56"
                data-testid={item.guid ? 'breadcrumb-doc-label' : undefined}
                data-full-name={item.guid ? item.label : undefined}
              >
                {item.label}
              </span>
              {item.guid ? (
                <span
                  className="shrink-0 font-mono text-xs font-normal text-muted-foreground"
                  data-testid="breadcrumb-doc-guid"
                  data-full-guid={item.guid}
                  title={item.guid}
                >
                  {formatGuidShort(item.guid)}
                </span>
              ) : null}
            </>
          );

          return (
            <React.Fragment key={`${item.href}-${index}`}>
              <BreadcrumbItem>
                {!isLast ? (
                  <BreadcrumbLink asChild>
                    <Link
                      href={item.href}
                      title={crumbTitle}
                      className="flex min-w-0 items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
                    >
                      {crumbBody}
                    </Link>
                  </BreadcrumbLink>
                ) : (
                  <BreadcrumbPage
                    title={crumbTitle}
                    className="flex min-w-0 items-center gap-1.5 text-xs font-medium"
                    data-testid={item.guid ? 'breadcrumb-document-crumb' : undefined}
                  >
                    {crumbBody}
                  </BreadcrumbPage>
                )}
              </BreadcrumbItem>
              {!isLast && (
                <BreadcrumbSeparator>
                  <ChevronRight className="h-3 w-3" />
                </BreadcrumbSeparator>
              )}
            </React.Fragment>
          );
        })}
      </BreadcrumbList>
    </Breadcrumb>
    </div>
  );
}

export default DynamicBreadcrumb;

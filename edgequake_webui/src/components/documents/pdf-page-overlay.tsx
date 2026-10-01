/**
 * SPEC-128 layout overlay surface (LAW-128-10).
 * Host (PDFViewer) owns fetch/chips; this paints bbox_norm % boxes.
 */
'use client';

import type { DocumentPageLayoutRegion } from '@/lib/api/edgequake/documents';
import { cn } from '@/lib/utils';
import { useTranslation } from 'react-i18next';
import { layoutAssetStem } from './layout-asset';

export { layoutAssetBasename, layoutAssetStem } from './layout-asset';

export type OverlayChips = {
  figures: boolean;
  charts: boolean;
  tables: boolean;
  paragraphs: boolean;
  columns: boolean;
  noise: boolean;
};

export const OVERLAY_CHIP_CLASSES: Record<keyof OverlayChips, string[]> = {
  figures: ['figure', 'formula'],
  charts: ['chart'],
  tables: ['table'],
  paragraphs: ['paragraph', 'title', 'caption'],
  columns: ['column'],
  noise: ['abandon', 'header', 'footer', 'other'],
};

export function overlayBoxColor(cls: string): string {
  switch (cls) {
    case 'figure':
    case 'formula':
      return 'rgba(37, 99, 235, 0.28)';
    case 'chart':
      return 'rgba(147, 51, 234, 0.28)';
    case 'table':
      return 'rgba(22, 163, 74, 0.28)';
    case 'paragraph':
    case 'title':
    case 'caption':
      return 'rgba(234, 179, 8, 0.22)';
    case 'column':
      return 'rgba(249, 115, 22, 0.22)';
    default:
      return 'rgba(239, 68, 68, 0.28)';
  }
}

export function regionChipVisible(cls: string, chips: OverlayChips): boolean {
  return (Object.keys(chips) as (keyof OverlayChips)[]).some(
    (key) => chips[key] && OVERLAY_CHIP_CLASSES[key].includes(cls),
  );
}

/** Scroll markdown pane to the img that matches a layout asset_path. */
export function focusMarkdownAsset(assetPath: string): void {
  const stem = layoutAssetStem(assetPath);
  const nodes = document.querySelectorAll('[data-layout-asset]');
  for (const node of nodes) {
    if (!(node instanceof HTMLElement)) continue;
    if (node.getAttribute('data-layout-asset') !== stem) continue;
    node.scrollIntoView({ block: 'center', behavior: 'smooth' });
    node.setAttribute('data-layout-asset-focused', 'true');
    window.setTimeout(() => node.removeAttribute('data-layout-asset-focused'), 2500);
    return;
  }
}

type PdfPageOverlayProps = {
  regions: DocumentPageLayoutRegion[];
  chips: OverlayChips;
  empty: boolean;
};

export function PdfPageOverlay({ regions, chips, empty }: PdfPageOverlayProps) {
  const { t } = useTranslation();
  const visible = regions.filter((r) => regionChipVisible(r.class, chips));

  return (
    <div
      data-testid="pdf-layout-overlay"
      className="pointer-events-none absolute inset-0 z-20"
    >
      {empty ? (
        <div
          data-testid="pdf-layout-empty"
          className="pointer-events-none absolute inset-x-0 top-2 flex justify-center"
        >
          <span className="rounded bg-background/80 px-2 py-1 text-xs text-muted-foreground">
            {t('documents.viewer.layout.empty', 'No regions on this page')}
          </span>
        </div>
      ) : null}
      {visible.map((r) => {
        const clickable = Boolean(r.asset_path);
        const isColumn = r.class === 'column';
        return (
          <div
            key={r.region_id}
            data-testid="pdf-layout-box"
            data-layout-class={r.class}
            data-layout-asset={r.asset_path ? layoutAssetStem(r.asset_path) : undefined}
            className={cn(
              'absolute border-2',
              isColumn && 'z-0 border-dashed',
              !isColumn && 'z-[1]',
              clickable && 'pointer-events-auto cursor-pointer',
            )}
            title={
              r.confidence != null
                ? `${r.class} (${r.confidence.toFixed(2)})`
                : r.class
            }
            style={{
              left: `${r.bbox_norm.x * 100}%`,
              top: `${r.bbox_norm.y * 100}%`,
              width: `${r.bbox_norm.w * 100}%`,
              height: `${r.bbox_norm.h * 100}%`,
              background: overlayBoxColor(r.class),
              borderColor: overlayBoxColor(r.class)
                .replace('0.28', '0.9')
                .replace('0.22', '0.9'),
            }}
            onClick={
              clickable
                ? (e) => {
                    e.stopPropagation();
                    focusMarkdownAsset(r.asset_path!);
                  }
                : undefined
            }
          >
            <span
              data-testid="pdf-layout-label"
              className="pointer-events-none absolute left-0 top-0 max-w-full truncate px-0.5 text-xs leading-tight text-foreground"
              style={{ background: overlayBoxColor(r.class).replace('0.28', '0.85').replace('0.22', '0.85') }}
            >
              {r.class}
            </span>
          </div>
        );
      })}
    </div>
  );
}

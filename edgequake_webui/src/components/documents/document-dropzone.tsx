'use client';

import { cn } from '@/lib/utils';
import { Upload } from 'lucide-react';
import type React from 'react';
import type { DropzoneInputProps, DropzoneRootProps } from 'react-dropzone';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  shouldShowVisionExtractControls,
  VisionSettingsPanel,
  type VisionExtractDraft,
} from '@/components/settings/vision-extract-controls';
import { useLlmModels } from '@/hooks/use-providers';
import { useTranslation } from 'react-i18next';
import { MAX_UPLOAD_LABEL } from '@/lib/api/upload-limits';
import { formatWorkspaceDefaultPdfParserLabel } from '@/lib/pdf/resolve-pdf-parser-backend';
import {
  effectiveEffortWhenAuto,
  modelSupportsThinking,
  supportedReasoningEffortsForModel,
} from '@/lib/settings/reasoning-effort-supported';
import type { PdfParserBackend } from '@/types/graph';
import { useEffect, useMemo, useRef, useState } from 'react';
import {
  resolveDropzoneFillLayout,
  type DropzoneFillLayout,
} from '@/lib/documents/dropzone-fill-layout';

/**
 * Props for the DocumentDropzone component.
 */
export interface DocumentDropzoneProps {
  /** Props to spread on the dropzone container */
  getRootProps: <T extends DropzoneRootProps>(props?: T) => T;
  /** Props to spread on the hidden file input */
  getInputProps: <T extends DropzoneInputProps>(props?: T) => T;
  /** Whether a drag operation is currently active over the zone */
  isDragActive: boolean;
  /** Function to programmatically open file dialog (explicit click handler) */
  openFileDialog: () => void;
  /** Per-upload PDF parser backend override. */
  pdfParserBackend: 'default' | 'vision' | 'edgeparse' | 'auto';
  /** Change handler for the PDF parser override selector. */
  onPdfParserBackendChange: (
    value: 'default' | 'vision' | 'edgeparse' | 'auto',
  ) => void;
  /**
   * Workspace default `pdf_parser_backend` — shown in the inherit option label
   * (e.g. Workspace Default (Vision)). Falls back to server → Vision when unset.
   */
  workspacePdfParserBackend?: PdfParserBackend | null;
  /** SPEC-109: optional vision reasoning effort for VLM convert. */
  visionReasoningEffort?: string;
  onVisionReasoningEffortChange?: (value: string | undefined) => void;
  /** SPEC-015V */
  visionExtract?: VisionExtractDraft;
  onVisionExtractChange?: (value: VisionExtractDraft) => void;
  /** SPEC-113: vision model identity for thinking capability honesty. */
  visionProvider?: string | null;
  visionModel?: string | null;
  /**
   * SPEC-048: compact chrome while ingestion is working so progress UI stays primary.
   */
  quiet?: boolean;
  /**
   * SPEC-099 LAW-099-4: denser band when feedback zone has live work.
   * Always remains a full-width drop target (never removed).
   */
  collapsed?: boolean;
  /**
   * Fill the parent panel (docking Upload zone). Stretches to 100% width/height
   * with content centered inside the dashed frame.
   */
  fill?: boolean;
}

function ParserSelect({
  pdfParserBackend,
  onPdfParserBackendChange,
  workspacePdfParserBackend,
  compact,
  /** When true, omit the side label — used inside the Vision combo row. */
  hideSideLabel,
  triggerClassName,
}: {
  pdfParserBackend: 'default' | 'vision' | 'edgeparse' | 'auto';
  onPdfParserBackendChange: (
    value: 'default' | 'vision' | 'edgeparse' | 'auto',
  ) => void;
  workspacePdfParserBackend?: PdfParserBackend | null;
  compact: boolean;
  hideSideLabel?: boolean;
  triggerClassName?: string;
}) {
  const { t } = useTranslation();
  const workspaceDefaultLabel = formatWorkspaceDefaultPdfParserLabel(
    t,
    workspacePdfParserBackend,
  );
  return (
    <div
      className={cn(
        'flex items-center gap-2',
        hideSideLabel ? 'min-w-0 flex-1' : 'shrink-0',
      )}
      onClick={(event) => event.stopPropagation()}
      onKeyDown={(event) => event.stopPropagation()}
    >
      {/* Always labelled so the select reads as "applies to the next upload",
          not a free-floating control (compact density keeps the short form). */}
      {!hideSideLabel && (
        <span className="text-xs text-muted-foreground whitespace-nowrap">
          {compact
            ? t('documents.upload.pdfParserShort', 'Parser')
            : t('documents.upload.pdfParser', 'Parser for this upload')}
        </span>
      )}
      <Select
        value={pdfParserBackend}
        onValueChange={(value: 'default' | 'vision' | 'edgeparse' | 'auto') =>
          onPdfParserBackendChange(value)
        }
      >
        <SelectTrigger
          className={cn(
            'bg-background',
            triggerClassName ??
              (compact
                ? 'min-w-[10.5rem] w-auto max-w-[14rem] h-7 text-xs'
                : 'min-w-[13.5rem] w-auto max-w-[18rem] h-9'),
          )}
          data-testid="spec038-upload-parser-select"
          title={
            pdfParserBackend === 'default' ? workspaceDefaultLabel : undefined
          }
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="default">{workspaceDefaultLabel}</SelectItem>
          <SelectItem value="vision">
            {t('documents.upload.pdfParserVision', 'Vision')}
          </SelectItem>
          <SelectItem value="edgeparse">
            {t('documents.upload.pdfParserEdgeParse', 'EdgeParse')}
          </SelectItem>
          <SelectItem value="auto">
            {t('documents.upload.pdfParserAuto', 'Auto')}
          </SelectItem>
        </SelectContent>
      </Select>
    </div>
  );
}

/**
 * Always-on file upload drop zone (idle expand / busy collapse).
 *
 * SPEC-099: the drop zone is never removed — collapse only shrinks chrome.
 * Drag-and-drop, click, and keyboard activation remain available.
 *
 * @implements FEAT0001 - Document ingestion with entity extraction
 * @implements SPEC-099 F-099-04 - collapse when feedback zone has live work
 */
export function DocumentDropzone({
  getRootProps,
  getInputProps,
  isDragActive,
  openFileDialog,
  pdfParserBackend,
  onPdfParserBackendChange,
  workspacePdfParserBackend,
  visionReasoningEffort,
  onVisionReasoningEffortChange,
  visionExtract,
  onVisionExtractChange,
  visionProvider,
  visionModel,
  quiet = false,
  collapsed = false,
  fill = false,
}: DocumentDropzoneProps) {
  const { t } = useTranslation();
  const { data: llmCatalog } = useLlmModels();
  const visionThinkingSupported = useMemo(
    () => modelSupportsThinking(llmCatalog?.models, visionProvider, visionModel),
    [llmCatalog?.models, visionProvider, visionModel],
  );
  const visionEffortSupported = useMemo(
    () =>
      supportedReasoningEffortsForModel(
        llmCatalog?.models,
        visionProvider,
        visionModel,
      ),
    [llmCatalog?.models, visionProvider, visionModel],
  );
  const compact = quiet || collapsed;
  const workspaceIsVision =
    !workspacePdfParserBackend ||
    workspacePdfParserBackend === 'vision' ||
    workspacePdfParserBackend === 'auto';
  const showVisionPanel =
    !compact &&
    !collapsed &&
    typeof onVisionExtractChange === 'function' &&
    visionExtract &&
    shouldShowVisionExtractControls(pdfParserBackend, workspaceIsVision);
  const showVisionEffort =
    showVisionPanel && typeof onVisionReasoningEffortChange === 'function';

  const rootRef = useRef<HTMLDivElement | null>(null);
  const [fillLayout, setFillLayout] = useState<DropzoneFillLayout>('hero');

  useEffect(() => {
    if (!fill) return;
    const el = rootRef.current;
    if (!el || typeof ResizeObserver === 'undefined') return;
    const apply = () => {
      const { width, height } = el.getBoundingClientRect();
      setFillLayout(resolveDropzoneFillLayout(width, height));
    };
    apply();
    const ro = new ResizeObserver(apply);
    ro.observe(el);
    return () => ro.disconnect();
  }, [fill]);

  const fillIsRow = fill && fillLayout === 'row';
  const fillIsStack = fill && fillLayout === 'stack';
  const fillIsHero = fill && fillLayout === 'hero';

  const rootProps = getRootProps({
    onClick: (e: React.MouseEvent) => {
      e.stopPropagation();
      openFileDialog();
    },
    onKeyDown: (e: React.KeyboardEvent) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        e.stopPropagation();
        openFileDialog();
      }
    },
    // WHY group, not button: the root hosts nested selects (axe nested-interactive).
    role: 'group' as const,
    'aria-label': collapsed || fill
      ? t('documents.upload.uploadCollapsed', 'Add files — click or drop')
      : t('documents.upload.uploadDrop', 'Upload files by clicking or dragging'),
    tabIndex: 0,
  });

  const { ref: dropRef, ...dropRootProps } = rootProps as typeof rootProps & {
    ref?: React.Ref<HTMLDivElement>;
  };
  const setRefs = (node: HTMLDivElement | null) => {
    rootRef.current = node;
    if (typeof dropRef === 'function') dropRef(node);
    else if (dropRef && typeof dropRef === 'object') {
      (dropRef as React.MutableRefObject<HTMLDivElement | null>).current = node;
    }
  };

  return (
    <div
      {...dropRootProps}
      ref={setRefs}
      data-testid="document-dropzone"
      data-upload="true"
      data-quiet={quiet ? 'true' : 'false'}
      data-collapsed={collapsed ? 'true' : 'false'}
      data-fill={fill ? 'true' : 'false'}
      data-fill-layout={fill ? fillLayout : undefined}
      className={cn(
        'w-full border-dashed cursor-pointer transition-colors duration-200',
        'flex min-w-0',
        fill
          ? cn(
              'absolute inset-0 rounded-none border border-dashed border-muted-foreground/35 bg-muted/5',
              fillIsRow && 'flex-row items-center gap-2 px-3 py-1.5',
              fillIsStack && 'flex-col items-stretch justify-center gap-2 px-2.5 py-2',
              fillIsHero && 'flex-col items-center justify-center gap-2 px-4 py-4',
            )
          : cn(
              'flex-wrap items-center gap-3',
              collapsed
                ? 'rounded-md border px-2.5 py-1.5 gap-2'
                : compact
                  ? 'rounded-lg border px-3 py-2 gap-2'
                  : 'rounded-lg border-2 px-4 py-2.5 gap-3',
            ),
        isDragActive
          ? fill
            ? 'border-primary bg-primary/5'
            : 'border-primary bg-primary/5 ring-2 ring-primary/20'
          : fill
            ? 'hover:border-primary/40 hover:bg-muted/15'
            : collapsed || quiet
              ? 'border-muted-foreground/25 bg-muted/10 hover:border-primary/40 hover:bg-muted/20'
              : 'border-muted-foreground/20 hover:border-primary/50 hover:bg-muted/30',
      )}
    >
      <input
        {...getInputProps()}
        aria-label={t('documents.upload.uploadDrop', 'Upload files by clicking or dragging')}
        data-testid="document-dropzone-input"
      />
      <div
        className={cn(
          'flex min-w-0 items-center gap-2',
          fillIsHero && 'max-w-full flex-col text-center',
          fillIsStack && 'w-full flex-col items-start text-left',
          fillIsRow && 'min-w-0 flex-1',
          !fill && 'flex-1 flex-wrap',
        )}
      >
        <div
          className={cn(
            'rounded-lg transition-all shrink-0',
            fillIsHero ? 'p-2.5' : fill || compact || collapsed ? 'p-1.5' : 'p-2',
            isDragActive ? 'bg-primary/10' : 'bg-muted/50',
          )}
        >
          <Upload
            className={cn(
              'transition-all duration-200',
              fillIsHero ? 'h-6 w-6' : fill || compact || collapsed ? 'h-4 w-4' : 'h-5 w-5',
              isDragActive ? 'text-primary scale-110' : 'text-muted-foreground',
            )}
          />
        </div>
        <div
          className={cn(
            'min-w-0 overflow-hidden',
            fillIsHero || fillIsStack ? 'w-full' : fillIsRow ? 'min-w-0 flex-1' : 'flex-1 basis-[12rem]',
          )}
        >
          {isDragActive ? (
            <p
              className={cn(
                'font-medium text-primary',
                fill ? 'text-sm' : 'truncate text-sm',
              )}
            >
              {t('documents.upload.uploadDropActive', 'Drop files here')}
            </p>
          ) : fill || collapsed ? (
            <p
              className={cn(
                'text-muted-foreground',
                fillIsHero ? 'text-sm' : 'truncate text-xs',
              )}
            >
              {t('documents.upload.addFilesDrop', 'Drop files or click to upload')}
            </p>
          ) : quiet ? (
            <p className="truncate text-xs text-muted-foreground">
              {t(
                'documents.upload.uploadWhileWorking',
                'Add more files anytime · max {{limit}}',
                { limit: MAX_UPLOAD_LABEL },
              )}
            </p>
          ) : (
            <p
              className="truncate text-sm text-muted-foreground"
              title={t(
                'documents.upload.uploadDropWithLimit',
                'Drag & drop or click to upload • TXT, MD, JSON, PDF, PNG, JPG, GIF, WEBP (max {{limit}}) · DOCX/Excel not supported',
                { limit: MAX_UPLOAD_LABEL },
              )}
            >
              {t(
                'documents.upload.uploadDropWithLimit',
                'Drag & drop or click to upload • TXT, MD, JSON, PDF, PNG, JPG, GIF, WEBP (max {{limit}}) · DOCX/Excel not supported',
                { limit: MAX_UPLOAD_LABEL },
              )}
            </p>
          )}
          {fillIsHero ? (
            <p className="mt-0.5 text-[11px] text-muted-foreground/80">
              TXT, MD, JSON, PDF, images · max {MAX_UPLOAD_LABEL}
            </p>
          ) : null}
        </div>
      </div>
      <div
        className={cn(
          'shrink-0 flex items-center gap-2',
          fillIsHero && 'justify-center',
          fillIsStack && 'w-full justify-stretch',
          fillIsRow && 'shrink-0',
          !fill &&
            'sm:border-l sm:border-border/70 sm:pl-3 max-sm:basis-full max-sm:border-t max-sm:border-border/50 max-sm:pt-1.5 max-sm:pl-0',
        )}
        data-testid="upload-parser-vision-combo"
        onClick={(event) => event.stopPropagation()}
        onKeyDown={(event) => event.stopPropagation()}
      >
        <ParserSelect
          pdfParserBackend={pdfParserBackend}
          onPdfParserBackendChange={onPdfParserBackendChange}
          workspacePdfParserBackend={workspacePdfParserBackend}
          compact={compact || collapsed || Boolean(fill)}
          hideSideLabel={collapsed || fillIsRow || fillIsStack}
          triggerClassName={
            compact || collapsed || fill
              ? cn(
                  'h-7 text-xs',
                  fillIsStack ? 'w-full max-w-none' : 'min-w-[9.5rem] w-auto max-w-[13rem]',
                )
              : 'min-w-[13.5rem] w-auto max-w-[18rem] h-9'
          }
        />
        {showVisionPanel && visionExtract && onVisionExtractChange ? (
          <VisionSettingsPanel
            value={visionExtract}
            onChange={onVisionExtractChange}
            showInheritHint={pdfParserBackend === 'default'}
            compact={compact || collapsed}
            effort={
              showVisionEffort && onVisionReasoningEffortChange
                ? {
                    value: visionReasoningEffort,
                    onChange: onVisionReasoningEffortChange,
                    supported: visionEffortSupported,
                    thinkingSupported: visionThinkingSupported,
                    effectiveWhenAuto: effectiveEffortWhenAuto(
                      llmCatalog?.models,
                      visionProvider,
                      visionModel,
                      'structured',
                    ),
                  }
                : undefined
            }
          />
        ) : null}
      </div>
    </div>
  );
}

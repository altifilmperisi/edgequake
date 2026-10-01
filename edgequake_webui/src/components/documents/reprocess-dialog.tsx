/**
 * @fileoverview Choice dialog for reprocessing an existing document.
 *
 * WHY: Reprocessing a completed PDF can mean two very different things:
 *  - "entities": reuse the cached markdown and only re-run the KG pipeline
 *    (chunking, entity/relationship extraction, embedding). Fast and cheap.
 *  - "full": re-run PDF -> markdown conversion from the stored PDF bytes
 *    (spends vision tokens), then re-run the KG pipeline. Slower and costly,
 *    but necessary when the markdown itself is wrong or stale.
 *
 * Previously the Reprocess button always queued the same opaque task, which
 * silently hit the "resume shortcut" in the backend and skipped the PDF
 * re-conversion entirely. This dialog makes the intent explicit and maps it
 * to the backend `mode` query param of POST /documents/reprocess.
 *
 * @implements FEAT-reprocess-choice - Reprocess intent selection
 * @enforces BR-reprocess-full - Full re-conversion on demand
 */
'use client';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { resolveDetailLifecycle } from '@/lib/documents/detail-lifecycle';
import { FileSearch, Layers, RefreshCw, Zap } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { Document } from '@/types';
import type { ReprocessMode } from '@/lib/api/edgequake';

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/** Full-doc API modes plus SPEC-151 partial pages handoff. */
export type ReprocessDialogIntent = ReprocessMode | 'pages';

export interface ReprocessChoice {
  mode: ReprocessDialogIntent;
}

interface ReprocessDialogProps {
  /** Whether the dialog is visible. */
  open: boolean;
  /** Target document (used to tailor copy: PDF vs. text, status-aware hints). */
  document: Pick<
    Document,
    | 'id'
    | 'title'
    | 'file_name'
    | 'source_type'
    | 'status'
    | 'document_type'
    | 'mime_type'
    | 'error_message'
    | 'stage_message'
    | 'failure_code'
  > | null;
  /** Called with the user's choice when they confirm. */
  onConfirm: (choice: ReprocessChoice) => void;
  /** Called when the user cancels or dismisses the dialog. */
  onCancel: () => void;
}

// ---------------------------------------------------------------------------
// Helpers (exported for unit testing — kept pure and side-effect free)
// ---------------------------------------------------------------------------

/** A document is PDF-like if its source/type/mime indicate a PDF. */
export function isPdfDocument(
  doc: Pick<Document, 'source_type' | 'document_type' | 'mime_type'> | null | undefined,
): boolean {
  if (!doc) return false;
  if (doc.source_type === 'pdf' || doc.document_type === 'pdf') return true;
  const mime = (doc.mime_type ?? '').toLowerCase();
  return mime === 'application/pdf' || mime.endsWith('pdf');
}

/** A document is still being processed and should not be re-queued. */
export function isInflight(
  doc: Pick<Document, 'status'> | null | undefined,
): boolean {
  return doc?.status === 'processing' || doc?.status === 'pending';
}

const INTERRUPTED_RESTART_RE =
  /interrupted by server restart|use reprocess to resume|server restart/i;

/** ISSUE-304: Interrupted-after-restart docs should default to Full reprocess. */
export function isInterruptedRestartDoc(
  doc:
    | Pick<Document, 'status' | 'error_message' | 'stage_message' | 'failure_code'>
    | null
    | undefined,
): boolean {
  if (!doc) return false;
  if (doc.status !== 'failed' && doc.status !== 'partial_failure') return false;
  if (doc.failure_code === 'server_restart_interrupted') return true;
  // Legacy message fallback for docs written before structured failure_code.
  const msg = `${doc.error_message ?? ''} ${doc.stage_message ?? ''}`;
  return INTERRUPTED_RESTART_RE.test(msg);
}

// ---------------------------------------------------------------------------

export function ReprocessDialog({
  open,
  document: target,
  onConfirm,
  onCancel,
}: ReprocessDialogProps) {
  const { t } = useTranslation();
  const pdf = isPdfDocument(target);
  const inflight = isInflight(target);
  const canPages =
    pdf &&
    Boolean(target) &&
    resolveDetailLifecycle(target!).canReprocessPages;

  // Default: entity-only is the cheap, safe default — except Interrupted
  // (server restart) docs, which need Full to avoid soft Entities no-ops.
  const [mode, setMode] = useState<ReprocessDialogIntent>('entities');

  useEffect(() => {
    if (!open) return;
    setMode(isInterruptedRestartDoc(target) ? 'full' : 'entities');
  }, [
    open,
    target?.id,
    target?.status,
    target?.error_message,
    target?.stage_message,
    target?.failure_code,
  ]);

  useEffect(() => {
    if (!canPages && mode === 'pages') setMode('entities');
  }, [canPages, mode]);

  const title = useMemo(() => {
    const name = target?.file_name || target?.title || target?.id?.slice(0, 8) || '';
    return t('documents.reprocessDialog.title', 'Reprocess "{{name}}"', { name });
  }, [t, target]);

  const handleConfirm = useCallback(() => {
    onConfirm({ mode });
  }, [mode, onConfirm]);

  const confirmLabel =
    mode === 'pages'
      ? t('documents.reprocessDialog.pagesContinue', 'Choose pages…')
      : t('documents.reprocessDialog.confirm', 'Reprocess');

  // Block confirm while the document is already processing — re-queuing would
  // race the in-flight task and corrupt the pipeline checkpoint.
  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && onCancel()}>
      <DialogContent className="max-w-lg" data-testid="reprocess-dialog">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <RefreshCw className="h-5 w-5 text-primary" />
            {title}
          </DialogTitle>
          <DialogDescription>
            {t(
              'documents.reprocessDialog.description',
              'Choose how this document should be reprocessed.',
            )}
          </DialogDescription>
        </DialogHeader>

        {inflight && (
          <p className="text-sm rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-amber-700 dark:text-amber-300">
            {t(
              'documents.reprocessDialog.inflightWarning',
              'This document is currently processing. Wait for it to finish before reprocessing.',
            )}
          </p>
        )}

        <RadioGroup
          value={mode}
          onValueChange={(v) => setMode(v as ReprocessDialogIntent)}
          className="gap-3"
        >
          {canPages ? (
            <ReprocessOption
              value="pages"
              icon={<Layers className="h-5 w-5" />}
              label={t(
                'documents.pageHealth.menuAction',
                'Reprocess specific pages',
              )}
              description={t(
                'documents.reprocessDialog.pagesDescription',
                'Pick pages and a starting stage (Parse / Figures / Entities). Other pages stay untouched.',
              )}
              badge={t(
                'documents.reprocessDialog.pagesBadge',
                'Partial · PDF only',
              )}
              disabled={inflight}
            />
          ) : null}
          <ReprocessOption
            value="full"
            icon={<FileSearch className="h-5 w-5" />}
            label={t(
              'documents.reprocessDialog.fullLabel',
              'Re-convert from PDF (slower, uses vision tokens)',
            )}
            description={
              pdf
                ? t(
                    'documents.reprocessDialog.fullDescriptionPdf',
                    'Re-runs PDF -> markdown conversion from the stored PDF bytes, then re-extracts entities and relationships. Use this when the markdown itself is wrong or stale.',
                  )
                : t(
                    'documents.reprocessDialog.fullDescriptionNonPdf',
                    'Re-ingests the source content from scratch and re-runs the full knowledge-graph pipeline.',
                  )
            }
            badge={t('documents.reprocessDialog.fullBadge', 'Slowest · costs tokens')}
            disabled={inflight}
          />
          <ReprocessOption
            value="entities"
            icon={<Zap className="h-5 w-5" />}
            label={t(
              'documents.reprocessDialog.entitiesLabel',
              'Re-extract entities only (reuse existing markdown)',
            )}
            description={
              pdf
                ? t(
                    'documents.reprocessDialog.entitiesDescriptionPdf',
                    'Keeps the cached markdown and only re-runs chunking, entity/relationship extraction, and embedding. Fast and cheap.',
                  )
                : t(
                    'documents.reprocessDialog.entitiesDescriptionNonPdf',
                    'Re-runs the knowledge-graph pipeline over the existing content.',
                  )
            }
            badge={t('documents.reprocessDialog.entitiesBadge', 'Fast · no vision')}
            disabled={inflight}
          />
        </RadioGroup>

        <DialogFooter className="gap-2">
          <Button variant="outline" onClick={onCancel}>
            {t('documents.reprocessDialog.cancel', 'Cancel')}
          </Button>
          <Button
            onClick={handleConfirm}
            disabled={inflight}
            data-testid="reprocess-dialog-confirm"
          >
            {confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ---------------------------------------------------------------------------
// Option subcomponent
// ---------------------------------------------------------------------------

interface ReprocessOptionProps {
  value: ReprocessDialogIntent;
  icon: React.ReactNode;
  label: string;
  description: string;
  badge: string;
  disabled?: boolean;
}

function ReprocessOption({
  value,
  icon,
  label,
  description,
  badge,
  disabled,
}: ReprocessOptionProps) {
  return (
    <label
      htmlFor={`reprocess-${value}`}
      data-testid={`reprocess-option-${value}`}
      className={[
        'flex items-start gap-3 rounded-lg border p-3 transition-colors',
        'cursor-pointer select-none',
        disabled ? 'opacity-60 cursor-not-allowed' : 'hover:bg-muted/40',
      ].join(' ')}
    >
      <RadioGroupItem
        id={`reprocess-${value}`}
        value={value}
        disabled={disabled}
        className="mt-1"
      />
      <div className="mt-0.5 shrink-0 text-muted-foreground">{icon}</div>
      <div className="flex-1 min-w-0 space-y-1">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="text-sm font-medium leading-tight">{label}</span>
          <span className="text-xs uppercase tracking-wide rounded-full border px-1.5 py-0.5 text-muted-foreground">
            {badge}
          </span>
        </div>
        <p className="text-xs text-muted-foreground leading-relaxed">{description}</p>
      </div>
    </label>
  );
}

export default ReprocessDialog;

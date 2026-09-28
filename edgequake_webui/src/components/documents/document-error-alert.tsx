/**
 * @module DocumentErrorAlert
 * @description Error alert for document loading failures.
 * Extracted from DocumentManager for SRP compliance (OODA-25).
 *
 * @implements FEAT0601 - Error display
 */
'use client';

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { ApiRequestError } from '@/lib/api/client';
import { AlertCircle } from 'lucide-react';

/**
 * Props for DocumentErrorAlert component.
 */
export interface DocumentErrorAlertProps {
  /** Error object or message */
  error: Error | unknown;
  /** Handler to retry loading */
  onRetry: () => void;
}

/** GH-400: map machine-readable read_path_busy reasons to short UI hints. */
export function readPathBusyHint(reason: unknown): string | null {
  if (typeof reason !== 'string') return null;
  switch (reason) {
    case 'work_deadline':
      return 'list timed out under load';
    case 'permit_wait':
      return 'too many list requests';
    case 'permit_closed':
      return 'read path unavailable';
    default:
      return null;
  }
}

function formatDocumentLoadError(error: Error | unknown): string {
  if (error instanceof ApiRequestError && error.code === 'read_path_busy') {
    const hint = readPathBusyHint(error.details?.reason);
    return hint ? `${error.message} — ${hint}` : error.message;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return 'Failed to load documents';
}

/**
 * Error alert displayed when document loading fails.
 */
export function DocumentErrorAlert({ error, onRetry }: DocumentErrorAlertProps) {
  return (
    <div className="p-6">
      <Alert variant="destructive">
        <AlertCircle className="h-4 w-4" />
        <AlertTitle>Error loading documents</AlertTitle>
        <AlertDescription>
          {formatDocumentLoadError(error)}
          <Button variant="link" className="ml-2 p-0" onClick={onRetry}>
            Try again
          </Button>
        </AlertDescription>
      </Alert>
    </div>
  );
}

export default DocumentErrorAlert;

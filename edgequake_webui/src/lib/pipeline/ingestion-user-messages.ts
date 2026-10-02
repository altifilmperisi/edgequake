/**
 * Translate backend stage_message / failure hints into user-facing copy.
 *
 * WHY: Backend messages are operator-oriented; UI must answer
 * "what is happening?" and "what should I do?" (SPEC-045).
 */

import type { Document } from '@/types';
import { documentStalledForMs } from '@/lib/documents/document-run-state';
import { formatSilence } from './run-liveness';
import {
  formatGraphMergeUserDetail,
  parseGraphMergeStageMessage,
} from './graph-merge-progress';

export type IngestionMessageContext = 'active' | 'queued' | 'stuck';

const AUTO_RECOVERED_RE =
  /auto-recovered after server restart/i;

/**
 * Human-readable detail line for a document in the ingestion banner/dialog.
 */
export function translateIngestionDetail(
  doc: Document,
  context: IngestionMessageContext,
): string {
  const raw = doc.stage_message?.trim();
  const fileName = doc.title || doc.file_name || 'Document';

  // SPEC-155: the last stage_message is stale history, not what is happening.
  const stalledMs = context === 'stuck' ? documentStalledForMs(doc) : null;
  if (stalledMs !== null) {
    return `${fileName}: No progress for ${formatSilence(stalledMs)} — cancel it or reprocess to continue.`;
  }

  if (raw && AUTO_RECOVERED_RE.test(raw)) {
    if (context === 'stuck') {
      return `${fileName}: Recovered after restart but no worker is processing it — reprocess to continue.`;
    }
    if (context === 'queued') {
      return `${fileName}: Resuming from checkpoint after restart.`;
    }
    return `${fileName}: Resuming after server restart.`;
  }

  if (raw) {
    const graphMerge = parseGraphMergeStageMessage(raw);
    if (graphMerge) {
      return formatGraphMergeUserDetail(fileName, graphMerge);
    }
    return `${fileName}: ${raw}`;
  }

  if (context === 'stuck') {
    return `${fileName}: Waiting with no active worker — reprocess to continue.`;
  }
  if (context === 'queued') {
    return `${fileName}: In queue — processing will start automatically.`;
  }

  return `${fileName}: Processing…`;
}

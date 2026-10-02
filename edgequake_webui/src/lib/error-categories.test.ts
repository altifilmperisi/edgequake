/**
 * @module error-categories.test
 * @description Unit tests for error categorization utility
 *
 * @implements OODA-10 - Test coverage for error categorization
 */

import { describe, expect, it } from 'vitest';

import {
  categorizeError,
  getCategoryColor,
  type ErrorCategory,
} from '@/lib/error-categories';

describe('categorizeError', () => {
  describe('LLM rate limit errors', () => {
    it('detects rate limit messages', () => {
      const errors = [
        'API rate limit exceeded',
        'Rate-limit reached for requests',
        'Too many requests, please slow down',
        'Error 429: Quota exceeded',
        'TPM limit exceeded for model',
        'RPM limit hit',
      ];

      for (const msg of errors) {
        const result = categorizeError(msg);
        expect(result.category).toBe('llm');
        expect(result.isTransient).toBe(true);
      }
    });
  });

  describe('LLM API/auth errors', () => {
    it('detects API key errors', () => {
      const errors = [
        'Invalid API key provided',
        'Authentication failed',
        'Unauthorized: check your credentials',
        'Invalid token for OpenAI',
      ];

      for (const msg of errors) {
        const result = categorizeError(msg);
        expect(result.category).toBe('llm');
        expect(result.isTransient).toBe(false);
      }
    });

    it('detects provider errors', () => {
      const errors = [
        'OpenAI API error',
        'Ollama connection failed',
        'LLM error during extraction',
      ];

      for (const msg of errors) {
        const result = categorizeError(msg);
        expect(result.category).toBe('llm');
      }
    });
  });

  describe('LLM context length errors', () => {
    it('detects context too long errors', () => {
      const errors = [
        'Context length exceeded',
        'Input too long for model',
        'Maximum tokens exceeded',
        'Token limit reached',
        'Maximum context window exceeded',
      ];

      for (const msg of errors) {
        const result = categorizeError(msg);
        expect(result.category).toBe('llm');
        expect(result.isTransient).toBe(false);
      }
    });
  });

  describe('embedding errors', () => {
    it('detects embedding dimension errors', () => {
      const errors = [
        'Embedding dimension mismatch',
        'Vector dimension does not match',
        'expected 1536 dimensions, not 768',
      ];

      for (const msg of errors) {
        const result = categorizeError(msg);
        expect(result.category).toBe('embedding');
        expect(result.isTransient).toBe(false);
        expect(result.suggestion).toMatch(/Embedding dimension mismatch/i);
      }
    });

    it('detects embedding provider unreachable before dimension mismatch', () => {
      const msg =
        'Pipeline processing failed: Embedding error: All embedding sub-batches failed: Network error: error sending request for url (http://localhost:11434/api/embeddings)';
      const result = categorizeError(msg);
      expect(result.category).toBe('embedding');
      expect(result.isTransient).toBe(true);
      expect(result.suggestion).toMatch(/unreachable/i);
      expect(result.suggestion).not.toMatch(/dimension mismatch/i);
    });

    it('does not label generic sub-batch failure as dimension mismatch', () => {
      const msg =
        'Pipeline processing failed: Embedding error: All embedding sub-batches failed [failure_class=unknown]';
      const result = categorizeError(msg);
      expect(result.category).toBe('embedding');
      expect(result.suggestion).not.toMatch(/dimension mismatch/i);
    });

    it('still classifies generic embedding process errors', () => {
      const errors = ['Error in embedding process', 'Failed to encode text'];

      for (const msg of errors) {
        const result = categorizeError(msg);
        expect(result.category).toBe('embedding');
        expect(result.isTransient).toBe(true);
      }
    });

    it('classifies pgvector CheckExpectedDim before storage', () => {
      const msg =
        'Knowledge graph persist failed: Storage error: expected 1536 dimensions, not 1024';
      const result = categorizeError(msg);
      expect(result.category).toBe('embedding');
      expect(result.isTransient).toBe(false);
      expect(result.suggestion).toMatch(/Embedding dimension mismatch/i);
      expect(result.suggestion).not.toMatch(/temporarily unavailable/i);
    });

    it('classifies typed fleet mirror before storage', () => {
      const msg =
        'Knowledge graph persist failed: Graph error: 1 knowledge-graph merge error(s) during persist: Storage error: Database error: SPEC-091: typed fleet mirror resolved 0/25 rows';
      const result = categorizeError(msg);
      expect(result.category).toBe('pipeline');
      expect(result.isTransient).toBe(false);
      expect(result.suggestion).not.toMatch(/temporarily unavailable/i);
    });

    it('classifies documents_valid_status before storage', () => {
      const msg =
        'Storage error: Database error: typed document shell batch write failed: documents_valid_status';
      const result = categorizeError(msg);
      expect(result.category).toBe('pipeline');
      expect(result.isTransient).toBe(false);
    });
  });

  describe('storage/database errors', () => {
    it('detects database errors', () => {
      const errors = [
        'Database connection failed',
        'PostgreSQL error',
        'Connection refused to database',
        'Unique constraint violation',
        'Storage error occurred',
        'Deadlock detected',
        'Transaction aborted',
      ];

      for (const msg of errors) {
        const result = categorizeError(msg);
        expect(result.category).toBe('storage');
        expect(result.isTransient).toBe(true);
      }
    });
  });

  describe('LLM extraction timeout errors', () => {
    it('classifies pipeline chunk timeouts before generic format hints', () => {
      const errors = [
        'Timeout after 180s (attempt 3/3) — All 4 chunks failed extraction.',
        'All 4 chunks failed extraction. Timeout after 180s',
        'Entity extraction timeout while processing chunk',
        'Vision extraction timed out after 120s',
      ];

      for (const msg of errors) {
        const result = categorizeError(msg);
        expect(result.category).toBe('llm_timeout');
        expect(result.isTransient).toBe(true);
        expect(result.suggestion).toMatch(/not a document-format problem/i);
      }
    });

    it('does not steal generic network timeouts', () => {
      const result = categorizeError('Request timeout');
      expect(result.category).toBe('network');
    });
  });

  describe('pipeline/parsing errors', () => {
    it('detects parse errors', () => {
      const errors = [
        'Parse error in document',
        'Invalid format detected',
        'Chunk error during processing',
        'Failed to extract entities',
        'Malformed input',
        'Invalid content type',
        'Empty content in document',
        'No text found in PDF',
        'PDF error: corrupt file',
      ];

      for (const msg of errors) {
        const result = categorizeError(msg);
        expect(result.category).toBe('pipeline');
        expect(result.isTransient).toBe(false);
      }
    });
  });

  describe('network errors', () => {
    it('detects timeout errors', () => {
      const errors = [
        'Request timeout',
        'Connection timed out',
        'Network error occurred',
        'Connection reset by peer',
        'ECONNREFUSED',
        'ETIMEDOUT',
        'Failed to fetch resource',
        'Host unreachable',
      ];

      for (const msg of errors) {
        const result = categorizeError(msg);
        expect(result.category).toBe('network');
        expect(result.isTransient).toBe(true);
      }
    });
  });

  describe('unknown errors', () => {
    it('falls back to unknown for unrecognized errors', () => {
      const errors = [
        'Something went wrong',
        'Unexpected error',
        'Internal server error',
        '',
      ];

      for (const msg of errors) {
        const result = categorizeError(msg);
        expect(result.category).toBe('unknown');
      }
    });
  });

  describe('summary extraction', () => {
    it('extracts first line as summary', () => {
      const result = categorizeError('First line\nSecond line\nThird line');
      expect(result.summary).toBe('First line');
    });

    it('removes Error: prefix', () => {
      const result = categorizeError('Error: Something bad happened');
      expect(result.summary).toBe('Something bad happened');
    });

    it('truncates long summaries', () => {
      const longMsg = 'A'.repeat(200);
      const result = categorizeError(longMsg);
      expect(result.summary.length).toBeLessThanOrEqual(100);
      expect(result.summary).toContain('...');
    });

    it('provides fallback for empty message', () => {
      const result = categorizeError('');
      expect(result.summary).toBe('An error occurred');
    });
  });

  describe('suggestions', () => {
    it('provides appropriate suggestion for each category', () => {
      const categories: ErrorCategory[] = [
        'llm',
        'llm_timeout',
        'embedding',
        'storage',
        'pipeline',
        'network',
        'unknown',
      ];

      for (const cat of categories) {
        // Force category by using known pattern
        let msg: string;
        switch (cat) {
          case 'llm':
            msg = 'rate limit';
            break;
          case 'llm_timeout':
            msg = 'Timeout after 180s (attempt 3/3)';
            break;
          case 'embedding':
            msg = 'embedding error';
            break;
          case 'storage':
            msg = 'database error';
            break;
          case 'pipeline':
            msg = 'parse error';
            break;
          case 'network':
            msg = 'timeout';
            break;
          default:
            msg = 'something happened';
        }

        const result = categorizeError(msg);
        expect(result.suggestion).toBeTruthy();
        expect(result.suggestion.length).toBeGreaterThan(10);
      }
    });
  });
});

describe('getCategoryColor', () => {
  it('returns colors for all categories', () => {
    const categories: ErrorCategory[] = [
      'llm',
      'llm_timeout',
      'embedding',
      'storage',
      'pipeline',
      'network',
      'lifecycle',
      'unknown',
    ];

    for (const cat of categories) {
      const colors = getCategoryColor(cat);
      expect(colors).toHaveProperty('bg');
      expect(colors).toHaveProperty('text');
      expect(colors).toHaveProperty('border');
      expect(colors.bg).toContain('bg-');
      expect(colors.text).toContain('text-');
    }
  });

  it('returns different colors for each category', () => {
    const llmColors = getCategoryColor('llm');
    const storageColors = getCategoryColor('storage');
    const pipelineColors = getCategoryColor('pipeline');

    expect(llmColors.bg).not.toBe(storageColors.bg);
    expect(storageColors.bg).not.toBe(pipelineColors.bg);
  });
});

describe('lifecycle (tombstoned document) errors', () => {
  const MSG =
    'Knowledge graph persist failed: Storage error: Conflict: cannot ingest into a tombstoned document';

  it('is not mislabelled as a transient storage outage', () => {
    const r = categorizeError(MSG);
    expect(r.category).toBe('lifecycle');
    expect(r.isTransient).toBe(false);
    expect(r.retryable).toBe(false);
    expect(r.suggestion).not.toMatch(/try again shortly/i);
  });

  it('recognises the typed worker failure message', () => {
    expect(categorizeError('Document 01a0f632 was deleted but its cleanup did not finish [failure_class=document_deleted]').category).toBe('lifecycle');
  });

  it('keeps ordinary storage errors transient and retryable', () => {
    const r = categorizeError('Storage error: connection pool timed out');
    expect(r.category).toBe('storage');
    expect(r.retryable).toBe(true);
  });
});

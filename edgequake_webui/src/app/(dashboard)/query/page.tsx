/**
 * @module QueryPage
 * @description RAG query interface page route.
 *
 * @implements FEAT0007 - Natural language query processing
 * @see QueryInterface component for full implementation
 */
import { CompanionUrlSync } from '@/components/query/companion/companion-url-sync';
import { QueryInterface } from '@/components/query/query-interface';
import { Suspense } from 'react';

export default function QueryPage() {
  return (
    <>
      {/* SPEC-157: `?pane=` deep links; useSearchParams needs a Suspense boundary. */}
      <Suspense fallback={null}>
        <CompanionUrlSync />
      </Suspense>
      <QueryInterface />
    </>
  );
}

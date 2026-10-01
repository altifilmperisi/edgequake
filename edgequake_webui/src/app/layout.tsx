/**
 * Root layout — SPEC-155 W1: Geist Sans/Mono via next/font (LAW-155-13 / F-155-D01).
 */
import { getRuntimeConfig } from '@/lib/runtime-config';
import { resolveRuntimeApiUrlForInjection } from '@/lib/server/resolve-runtime-api-url';
import { AppProviders } from '@/providers';
import { GeistMono } from 'geist/font/mono';
import { GeistSans } from 'geist/font/sans';
import type { Metadata } from 'next';
import './globals.css';

export const dynamic = 'force-dynamic';

export const metadata: Metadata = {
  title: 'EdgeQuake - Knowledge Graph RAG Platform',
  description:
    'Advanced Retrieval-Augmented Generation with graph-based knowledge representation',
  keywords: ['RAG', 'Knowledge Graph', 'LLM', 'AI', 'Graph Database'],
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  const runtimeConfig = {
    ...getRuntimeConfig(),
    apiUrl: resolveRuntimeApiUrlForInjection() || getRuntimeConfig().apiUrl,
  };

  return (
    <html
      lang="en"
      className={`${GeistSans.variable} ${GeistMono.variable} h-full overflow-hidden`}
      suppressHydrationWarning
    >
      <body className="h-full overflow-hidden font-sans antialiased" suppressHydrationWarning>
        <script
          suppressHydrationWarning
          dangerouslySetInnerHTML={{
            __html: `window.__EDGEQUAKE_RUNTIME_CONFIG__ = ${JSON.stringify(runtimeConfig)};`,
          }}
        />
        <AppProviders>{children}</AppProviders>
      </body>
    </html>
  );
}

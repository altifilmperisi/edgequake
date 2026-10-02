/**
 * SPEC-157 e2e helpers — hermetic companion-pane fixtures (@spec157).
 * Screenshots land in specs/157-side-by-side-query/e2e/screenshots/.
 */
import { expect, type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { prepareSpec155Page } from "../spec155/helpers/mock-api";
import type { MockChatScenario } from "../spec155/helpers/mock-chat-sse";
import { buildMockPdf, mockRangePdfRoute } from "../spec155/helpers/mock-pdf";

export const DOC_ID = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
export const TEXT_DOC_ID = "dddddddd-dddd-4ddd-8ddd-dddddddddddd";
export const MSG_ID = "msg-asst-001";
export const PDF_PAGES = 12;

const SHOTS_ROOT = path.resolve(
  __dirname,
  "../../../specs/157-side-by-side-query/e2e/screenshots",
);

export async function shot(page: Page, name: string): Promise<void> {
  fs.mkdirSync(SHOTS_ROOT, { recursive: true });
  // Let layout/animation settle so the PNG is representative.
  await page.mouse.move(0, 0); // no stray hover popovers in the PNG
  await page.waitForTimeout(500);
  await page.screenshot({ path: path.join(SHOTS_ROOT, `${name}.png`) });
}

const PDF_DOC = {
  id: DOC_ID,
  pdf_id: DOC_ID,
  title: "LightRAG: Simple and Fast Retrieval-Augmented Generation",
  file_name: "lightrag.pdf",
  status: "completed",
  source_type: "pdf",
  mime_type: "application/pdf",
  page_count: PDF_PAGES,
  content: "",
  created_at: "2026-10-01T06:00:00Z",
  updated_at: "2026-10-01T06:30:00Z",
};

const TEXT_DOC = {
  id: TEXT_DOC_ID,
  title: "Release notes",
  file_name: "release-notes.md",
  status: "completed",
  source_type: "markdown",
  mime_type: "text/markdown",
  content:
    "# Release notes\n\nLightRAG combines graph structures with vector retrieval.\n\n## Highlights\n\n- Dual-level retrieval\n- Incremental updates\n",
  created_at: "2026-10-01T06:00:00Z",
  updated_at: "2026-10-01T06:30:00Z",
};

const PASSAGE =
  "LightRAG integrates graph structures into text indexing and retrieval, using a dual-level retrieval system that enables comprehensive information retrieval from both low-level and high-level knowledge discovery.";

const ENTITIES = [
  ["LIGHTRAG", "METHOD", 0.95, 6],
  ["GRAPH_INDEXING", "CONCEPT", 0.9, 4],
  ["DUAL_LEVEL_RETRIEVAL", "CONCEPT", 0.88, 3],
  ["VECTOR_STORE", "TECHNOLOGY", 0.8, 2],
  ["KNOWLEDGE_GRAPH", "CONCEPT", 0.78, 5],
  ["OPENAI", "ORGANIZATION", 0.6, 2],
] as const;

export const QA_SCENARIO: MockChatScenario = {
  events: [
    { type: "conversation", conversation_id: "conv-spec157-001", user_message_id: "msg-user-001" },
    { type: "stage", stage: "retrieving" },
    {
      type: "context",
      sources: [
        {
          source_type: "chunk",
          id: "chunk-1",
          score: 0.92,
          snippet: PASSAGE,
          document_id: DOC_ID,
          file_path: "lightrag.pdf",
          page_start: 3,
          page_end: 3,
          reference_id: 1,
        },
        {
          source_type: "chunk",
          id: "chunk-2",
          score: 0.81,
          snippet: "Incremental updates let the index grow without a full rebuild.",
          document_id: DOC_ID,
          file_path: "lightrag.pdf",
          page_start: 7,
          page_end: 7,
          reference_id: 2,
        },
        {
          source_type: "chunk",
          id: "chunk-3",
          score: 0.7,
          snippet: "LightRAG combines graph structures with vector retrieval.",
          document_id: TEXT_DOC_ID,
          file_path: "release-notes.md",
          start_line: 2,
          end_line: 3,
          reference_id: 3,
        },
      ],
      subgraph: {
        entities: ENTITIES.map(([name, type, score, degree]) => ({
          id: `ent:${name}`,
          name,
          entity_type: type,
          description: `${name.replace(/_/g, " ").toLowerCase()} as described in the paper.`,
          score,
          degree,
          graph_node_id: name,
        })),
        relationships: [
          ["LIGHTRAG", "GRAPH_INDEXING", "USES"],
          ["LIGHTRAG", "DUAL_LEVEL_RETRIEVAL", "IMPLEMENTS"],
          ["GRAPH_INDEXING", "KNOWLEDGE_GRAPH", "BUILDS"],
          ["LIGHTRAG", "VECTOR_STORE", "COMBINES_WITH"],
          ["DUAL_LEVEL_RETRIEVAL", "KNOWLEDGE_GRAPH", "QUERIES"],
          ["LIGHTRAG", "OPENAI", "CALLS"],
        ].map(([source, target, relation_type], i) => ({
          id: `rel-${i}`,
          source,
          target,
          relation_type,
          description: "",
          score: 0.7,
        })),
      },
      query_mode: "mix",
      retrieval_time_ms: 14,
    },
    { type: "stage", stage: "generating" },
    {
      type: "token",
      content:
        "## How LightRAG retrieves\n\nLightRAG indexes documents as a knowledge graph and queries it at two levels [source:1]. New content is merged incrementally instead of rebuilding the index [source:2]. A short changelog summarises the same idea [source:3].\n",
    },
    {
      type: "done",
      assistant_message_id: MSG_ID,
      tokens_used: 64,
      duration_ms: 900,
      answer:
        "## How LightRAG retrieves\n\nLightRAG indexes documents as a knowledge graph and queries it at two levels [source:1]. New content is merged incrementally instead of rebuilding the index [source:2]. A short changelog summarises the same idea [source:3].\n",
      llm_provider: "mock",
      llm_model: "mock-model",
    },
    { type: "title_update", conversation_id: "conv-spec157-001", title: "How LightRAG retrieves" },
  ],
};

/**
 * What the real backend persists with the assistant message: chunk `content`,
 * flat entities/relationships (no subgraph bundle, ids are entity names).
 */
function persistedConversation() {
  const ctxEvt = QA_SCENARIO.events.find((e) => e.type === "context") as {
    sources: Array<Record<string, unknown>>;
    subgraph: {
      entities: Array<Record<string, unknown>>;
      relationships: Array<Record<string, unknown>>;
    };
  };
  const now = new Date().toISOString();
  return {
    id: "conv-spec157-001",
    title: "How LightRAG retrieves",
    mode: "mix",
    created_at: now,
    updated_at: now,
    message_count: 2,
    messages: [
      { id: "msg-user-001", role: "user", content: "How does LightRAG retrieve information?", created_at: now },
      {
        id: MSG_ID,
        role: "assistant",
        content: (QA_SCENARIO.events.find((e) => e.type === "done") as { answer: string }).answer,
        created_at: now,
        mode: "mix",
        context: {
          sources: ctxEvt.sources.map((s) => ({ ...s, content: s.snippet, title: s.file_path })),
          entities: ctxEvt.subgraph.entities.map((e) => ({
            name: e.name,
            entity_type: e.entity_type,
            score: e.score,
            degree: e.degree,
          })),
          relationships: ctxEvt.subgraph.relationships.map((r) => ({
            source: r.source,
            target: r.target,
            relation_type: r.relation_type,
            score: r.score,
          })),
        },
      },
    ],
  };
}

export type PrepareOptions = {
  viewport?: { width: number; height: number };
  /** Disable the companion kill switch before the app boots. */
  flagOff?: boolean;
  scenario?: MockChatScenario;
};

/** Mock the API + PDF binary, then boot at `url`. */
export async function openQuery(
  page: Page,
  url = "/query",
  opts: PrepareOptions = {},
): Promise<void> {
  await prepareSpec155Page(page, {
    documents: [PDF_DOC, TEXT_DOC],
    chatScenario: opts.scenario ?? QA_SCENARIO,
  });
  await page.route(/\/api\/v1\/documents\/pdf\/[^/?]+$/, (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        pdf_id: DOC_ID,
        document_id: DOC_ID,
        filename: "lightrag.pdf",
        file_size_bytes: 1,
        content_type: "application/pdf",
        markdown_content: "# LightRAG\n\nPage 3 text.",
        is_processed: true,
      }),
    }),
  );
  await mockRangePdfRoute(page, buildMockPdf(PDF_PAGES));
  // Registered after the generic mock so persisted messages carry full context.
  await page.route(/\/api\/v1\/conversations\/conv-spec157-001\/?(\?.*)?$/, (route) =>
    route.request().method() === "GET"
      ? route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify(persistedConversation()),
        })
      : route.fallback(),
  );
  // Neighbourhood expansion: two new entities around the selected node.
  await page.route(/\/graph\/entities\/[^/]+\/neighborhood/, (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        nodes: [
          { id: "RAG_BASELINE", label: "RAG_BASELINE", entity_type: "METHOD" },
          { id: "BENCHMARK", label: "BENCHMARK", entity_type: "CONCEPT" },
        ],
        edges: [
          { id: "nx1", source: "LIGHTRAG", target: "RAG_BASELINE", relationship_type: "OUTPERFORMS", weight: 1, source_ids: [], created_at: "" },
          { id: "nx2", source: "LIGHTRAG", target: "BENCHMARK", relationship_type: "EVALUATED_ON", weight: 1, source_ids: [], created_at: "" },
        ],
      }),
    }),
  );
  await page.setViewportSize(opts.viewport ?? { width: 1600, height: 900 });
  if (opts.flagOff) {
    await page.addInitScript(() =>
      localStorage.setItem("edgequake.query.companion.enabled", "0"),
    );
  }
  await page.goto(url, { waitUntil: "domcontentloaded" });
}

/** Ask the canned question and wait for the answer to finish. */
export async function askQuestion(page: Page): Promise<void> {
  const input = page.locator("textarea.query-input").first();
  await expect(input).toBeVisible({ timeout: 30_000 });
  await input.fill("How does LightRAG retrieve information?");
  await input.press("Enter");
  await expect(cite(page, 1)).toBeVisible({ timeout: 30_000 });
}

/** Inline `[n]` chip inside the answer text (the source-chip row reuses the id). */
export const cite = (page: Page, n: number) =>
  page
    .getByRole("region", { name: "Response content" })
    .getByTestId(`query-inline-citation-${n}`);

export async function openFirstCitation(page: Page): Promise<void> {
  await cite(page, 1).click();
  await expect(page.getByTestId("query-companion")).toBeVisible();
  // The pane opens from the store first; wait for the URL write to land.
  await expect(page).toHaveURL(/pane=pdf/);
}

export const pane = (page: Page) => page.getByTestId("query-companion");

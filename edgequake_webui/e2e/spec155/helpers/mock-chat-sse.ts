/**
 * Scripted SSE builder for query chat Playwright mocks (SPEC-155 W7Q).
 */
export type MockChatEvent =
  | { type: "conversation"; conversation_id: string; user_message_id: string }
  | {
      type: "stage";
      stage: "retrieving" | "reading" | "generating";
      detail?: string;
    }
  | { type: "thinking"; content: string }
  | {
      type: "context";
      sources: Array<Record<string, unknown>>;
      subgraph?: Record<string, unknown>;
      query_mode?: string;
      retrieval_time_ms?: number;
    }
  | { type: "token"; content: string }
  | {
      type: "done";
      assistant_message_id: string;
      tokens_used: number;
      duration_ms: number;
      answer?: string;
      llm_provider?: string;
      llm_model?: string;
    }
  | { type: "error"; message: string; code: string }
  | { type: "title_update"; conversation_id: string; title: string };

export type MockChatScenario = {
  events: MockChatEvent[];
  /** Delay between events in ms (default 30) */
  delayMs?: number;
};

export function sseBodyFromEvents(events: MockChatEvent[]): string {
  return events
    .map((data) => `data: ${JSON.stringify(data)}\n\n`)
    .join("");
}

export const SCENARIO_HAPPY: MockChatScenario = {
  events: [
    {
      type: "conversation",
      conversation_id: "conv-spec155-001",
      user_message_id: "msg-user-001",
    },
    { type: "stage", stage: "retrieving" },
    {
      type: "thinking",
      content: "Retrieved context via mix (2 sources in 12ms)",
    },
    {
      type: "stage",
      stage: "reading",
      detail: "2 sources",
    },
    {
      type: "context",
      sources: [
        {
          source_type: "chunk",
          id: "chunk-1",
          score: 0.92,
          snippet: "LightRAG uses dual-level retrieval.",
          document_id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
          file_path: "lightrag.pdf",
          page_start: 3,
          page_end: 3,
          reference_id: 1,
        },
        {
          source_type: "entity",
          id: "LIGHTRAG",
          score: 0.88,
          entity_type: "METHOD",
          document_id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
        },
      ],
      subgraph: {
        entities: [
          {
            id: "ent:LIGHTRAG",
            name: "LIGHTRAG",
            entity_type: "METHOD",
            score: 0.9,
            degree: 4,
            graph_node_id: "n1",
          },
        ],
        relationships: [],
      },
      query_mode: "mix",
      retrieval_time_ms: 12,
    },
    { type: "stage", stage: "generating" },
    { type: "token", content: "## Summary\n\n" },
    {
      type: "token",
      content:
        "LightRAG uses dual-level retrieval ([lightrag p.3](/documents/aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa?page=3 \"LightRAG Paper\")).\n",
    },
    {
      type: "done",
      assistant_message_id: "msg-asst-001",
      tokens_used: 42,
      duration_ms: 800,
      answer:
        "## Summary\n\nLightRAG uses dual-level retrieval ([lightrag p.3](/documents/aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa?page=3 \"LightRAG Paper\")).\n",
      llm_provider: "mock",
      llm_model: "mock-model",
    },
    {
      type: "title_update",
      conversation_id: "conv-spec155-001",
      title: "LightRAG summary",
    },
  ],
};

export const SCENARIO_EMPTY_SOURCES: MockChatScenario = {
  events: [
    {
      type: "conversation",
      conversation_id: "conv-empty",
      user_message_id: "u1",
    },
    { type: "stage", stage: "retrieving" },
    {
      type: "context",
      sources: [],
      query_mode: "mix",
      retrieval_time_ms: 5,
    },
    { type: "stage", stage: "generating" },
    { type: "token", content: "I could not find grounded sources." },
    {
      type: "done",
      assistant_message_id: "a1",
      tokens_used: 10,
      duration_ms: 100,
      answer: "I could not find grounded sources.",
    },
  ],
};

export const SCENARIO_ERROR: MockChatScenario = {
  events: [
    {
      type: "conversation",
      conversation_id: "conv-err",
      user_message_id: "u1",
    },
    { type: "stage", stage: "retrieving" },
    { type: "error", message: "Upstream LLM failed", code: "LLM_ERROR" },
  ],
};

/** Long token stream for stop/queue tests */
export function scenarioLongStream(tokenCount = 40): MockChatScenario {
  const tokens: MockChatEvent[] = Array.from({ length: tokenCount }, (_, i) => ({
    type: "token" as const,
    content: `word${i} `,
  }));
  return {
    delayMs: 50,
    events: [
      {
        type: "conversation",
        conversation_id: "conv-long",
        user_message_id: "u1",
      },
      { type: "stage", stage: "retrieving" },
      {
        type: "context",
        sources: [
          {
            source_type: "chunk",
            id: "c1",
            score: 0.5,
            snippet: "x",
            document_id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
            file_path: "lightrag.pdf",
            page_start: 1,
          },
        ],
      },
      { type: "stage", stage: "generating" },
      ...tokens,
      {
        type: "done",
        assistant_message_id: "a1",
        tokens_used: tokenCount,
        duration_ms: 2000,
      },
    ],
  };
}

/**
 * Reasoning-model stream: retrieval thinking event, then chunked <think>
 * tags (including a split tag at the boundary), then the answer.
 */
export const SCENARIO_THINKING: MockChatScenario = {
  delayMs: 40,
  events: [
    {
      type: "conversation",
      conversation_id: "conv-think-001",
      user_message_id: "msg-user-think",
    },
    { type: "stage", stage: "retrieving" },
    {
      type: "thinking",
      content: "Retrieved context via mix (1 sources in 8ms)",
    },
    {
      type: "context",
      sources: [
        {
          source_type: "chunk",
          id: "chunk-t1",
          score: 0.9,
          snippet: "Graph RAG retrieves structured neighbors.",
          document_id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
          file_path: "lightrag.pdf",
          page_start: 2,
          page_end: 2,
          reference_id: 1,
        },
      ],
      query_mode: "mix",
      retrieval_time_ms: 8,
    },
    // Split opening tag across tokens to exercise heldPartial
    { type: "token", content: "<thi" },
    { type: "token", content: "nk>I should cite the paper carefully.\n" },
    {
      type: "token",
      content: "Check page 2 for the dual-level claim.</think>\n\n",
    },
    {
      type: "token",
      content:
        "Graph RAG retrieves structured neighbors ([lightrag p.2](/documents/aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa?page=2 \"LightRAG Paper\")).\n",
    },
    {
      type: "done",
      assistant_message_id: "msg-asst-think",
      tokens_used: 64,
      duration_ms: 1200,
      answer:
        "<think>I should cite the paper carefully.\nCheck page 2 for the dual-level claim.</think>\n\nGraph RAG retrieves structured neighbors ([lightrag p.2](/documents/aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa?page=2 \"LightRAG Paper\")).\n",
      llm_provider: "mock",
      llm_model: "mock-reasoner",
    },
  ],
};

/** Context with many chunks so the chip row shows "+N more". */
export function scenarioManySources(count = 9): MockChatScenario {
  const sources = Array.from({ length: count }, (_, i) => ({
    source_type: "chunk",
    id: `chunk-${i + 1}`,
    score: 0.9 - i * 0.02,
    snippet: `Passage number ${i + 1}`,
    content: `Passage number ${i + 1}`,
    document_id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
    file_path: "lightrag.pdf",
    page_start: i + 1,
    page_end: i + 1,
    reference_id: i + 1,
  }));
  return {
    events: [
      {
        type: "conversation",
        conversation_id: "conv-spec155-001",
        user_message_id: "msg-user-001",
      },
      { type: "stage", stage: "retrieving" },
      {
        type: "context",
        sources,
        subgraph: { entities: [], relationships: [] },
        query_mode: "mix",
        retrieval_time_ms: 9,
      },
      { type: "stage", stage: "generating" },
      { type: "token", content: "Many sources answer." },
      {
        type: "done",
        assistant_message_id: "msg-asst-001",
        tokens_used: 10,
        duration_ms: 100,
        answer: "Many sources answer.",
        llm_provider: "mock",
        llm_model: "mock-model",
      },
    ],
  };
}

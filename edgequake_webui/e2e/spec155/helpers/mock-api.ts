/**
 * SPEC-155 mocked API for UI-only Playwright (no live backend / Ollama).
 * Extends the SPEC-017 tenant seed pattern with full surface coverage.
 */
import type { Page } from "@playwright/test";
import {
  FIXTURE_100,
  FIXTURE_EMPTY,
  toKnowledgeGraphResponse,
  type FixtureGraph,
} from "../../../src/lib/fixtures/graph";

export const SPEC155_TENANT = {
  id: "spec155-tenant-001",
  name: "SPEC155 Tenant",
  slug: "spec155",
  plan: "pro",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

export const SPEC155_WORKSPACE = {
  id: "spec155-ws-001",
  tenant_id: SPEC155_TENANT.id,
  name: "SPEC155 Workspace",
  slug: "spec155-ws",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

export type Spec155MockOptions = {
  graph?: FixtureGraph;
  documents?: Array<Record<string, unknown>>;
  emptyDocs?: boolean;
};

const DEFAULT_DOCS = [
  {
    id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
    title: "LightRAG Paper",
    file_name: "lightrag.pdf",
    status: "completed",
    created_at: "2026-01-15T10:00:00Z",
    updated_at: "2026-01-15T11:00:00Z",
    content_type: "application/pdf",
    size_bytes: 1_200_000,
  },
  {
    id: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
    title: "Processing Doc",
    file_name: "processing.pdf",
    status: "processing",
    created_at: "2026-01-16T10:00:00Z",
    updated_at: "2026-01-16T10:30:00Z",
    content_type: "application/pdf",
    size_bytes: 800_000,
  },
];

function json(data: unknown, status = 200) {
  return {
    status,
    contentType: "application/json",
    body: JSON.stringify(data),
  };
}

function sseBody(events: Array<{ event?: string; data: unknown }>): string {
  return events
    .map((e) => {
      const lines: string[] = [];
      if (e.event) lines.push(`event: ${e.event}`);
      lines.push(`data: ${JSON.stringify(e.data)}`);
      return lines.join("\n") + "\n\n";
    })
    .join("");
}

export async function seedSpec155Tenant(page: Page): Promise<void> {
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await page.evaluate(
    ({ tenant, workspace }) => {
      localStorage.clear();
      sessionStorage.clear();
      const userId = "spec155-user-001";
      localStorage.setItem("userId", userId);
      localStorage.setItem("tenantId", tenant.id);
      localStorage.setItem("workspaceId", workspace.id);
      localStorage.setItem(
        "edgequake-tenant",
        JSON.stringify({
          state: {
            selectedTenantId: tenant.id,
            selectedWorkspaceId: workspace.id,
            workspaces: [workspace],
            tenants: [tenant],
          },
          version: 1,
        }),
      );
      localStorage.setItem(
        "edgequake-auth",
        JSON.stringify({
          state: {
            accessToken: "spec155-mock-access",
            user: { id: userId, username: "spec155", role: "admin" },
            isAuthenticated: true,
          },
          version: 1,
        }),
      );
    },
    { tenant: SPEC155_TENANT, workspace: SPEC155_WORKSPACE },
  );
}

export async function mockSpec155Api(
  page: Page,
  options: Spec155MockOptions = {},
): Promise<void> {
  const graph = options.graph ?? FIXTURE_100;
  const kg = toKnowledgeGraphResponse(graph);
  const docs = options.emptyDocs ? [] : (options.documents ?? DEFAULT_DOCS);

  await page.route("**/health**", (route) =>
    route.fulfill(
      json({
        status: "healthy",
        version: "0.28.5",
        storage_mode: "postgresql",
        workspace_id: SPEC155_WORKSPACE.id,
        components: {
          kv_storage: true,
          vector_storage: true,
          graph_storage: true,
          llm_provider: true,
        },
        llm_provider_name: "mock",
      }),
    ),
  );
  await page.route("**/api/health**", (route) =>
    route.fulfill(json({ status: "healthy", version: "0.28.5" })),
  );
  await page.route("**/live", (route) => route.fulfill({ status: 200, body: "OK" }));
  await page.route("**/ready", (route) => route.fulfill({ status: 200, body: "OK" }));

  // Prevent “Connection lost” toast + 500 proxy noise when no live backend.
  await page.route("**/api/v1/setup/status**", (route) =>
    route.fulfill(
      json({
        status: "ready",
        setup_complete: true,
        auth_enabled: false,
        demo_mode: true,
      }),
    ),
  );
  await page.route("**/api/v1/settings/llm-defaults**", (route) =>
    route.fulfill(json({ providers: [], default_provider: "mock" })),
  );
  await page.route("**/api/v1/tasks**", (route) =>
    route.fulfill(json({ items: [], total: 0, page: 1, page_size: 50 })),
  );

  await page.route("**/api/v1/tenants/*/workspaces**", (route) =>
    route.fulfill(
      json({ items: [SPEC155_WORKSPACE], total: 1, offset: 0, limit: 20 }),
    ),
  );
  // Register tenants AFTER workspaces — Playwright matches last-registered first.
  await page.route("**/api/v1/tenants**", async (route) => {
    const url = route.request().url();
    if (/\/tenants\/[^/]+\/workspaces/.test(url)) {
      await route.fulfill(
        json({ items: [SPEC155_WORKSPACE], total: 1, offset: 0, limit: 20 }),
      );
      return;
    }
    await route.fulfill(
      json({ items: [SPEC155_TENANT], total: 1, offset: 0, limit: 20 }),
    );
  });

  await page.route("**/api/v1/workspaces/*/stats**", (route) =>
    route.fulfill(
      json({
        workspace_id: SPEC155_WORKSPACE.id,
        document_count: docs.length,
        entity_count: graph.total_nodes,
        relationship_count: graph.total_edges,
        entity_type_count: 6,
        chunk_count: docs.length * 12,
        embedding_count: docs.length * 12,
        storage_bytes: 2_500_000,
        stale: false,
      }),
    ),
  );

  await page.route("**/api/v1/documents**", async (route) => {
    const url = route.request().url();
    if (route.request().method() !== "GET") {
      await route.fulfill(json({ ok: true }));
      return;
    }
    const match = url.match(/documents\/([0-9a-f-]{36})/i);
    if (match) {
      const doc = docs.find((d) => d.id === match[1]);
      await route.fulfill(doc ? json(doc) : json({ error: "not found" }, 404));
      return;
    }
    await route.fulfill(
      json({
        items: docs,
        documents: docs,
        total: docs.length,
        offset: 0,
        limit: 50,
        page: 1,
        page_size: 50,
        total_pages: 1,
        has_more: false,
      }),
    );
  });

  await page.route("**/api/v1/graph/communities**", (route) =>
    route.fulfill(
      json({
        items: Array.from(
          new Set(graph.nodes.map((n) => n.community_id).filter(Boolean)),
        ).map((id, i) => ({
          id,
          size: graph.nodes.filter((n) => n.community_id === id).length,
          label: `Community ${i + 1}`,
        })),
      }),
    ),
  );

  await page.route("**/api/v1/graph/facets**", (route) => {
    const entity_types: Record<string, number> = {};
    const relationship_types: Record<string, number> = {};
    for (const n of graph.nodes) {
      entity_types[n.node_type] = (entity_types[n.node_type] ?? 0) + 1;
    }
    for (const e of graph.edges) {
      relationship_types[e.relationship_type] =
        (relationship_types[e.relationship_type] ?? 0) + 1;
    }
    return route.fulfill(json({ entity_types, relationship_types }));
  });

  await page.route("**/api/v1/graph/stream**", async (route) => {
    const batch = 50;
    const events: Array<{ event?: string; data: unknown }> = [
      {
        event: "metadata",
        data: {
          type: "metadata",
          total_nodes: kg.total_nodes,
          total_edges: kg.total_edges,
          max_nodes: kg.max_nodes,
          is_truncated: kg.is_truncated,
        },
      },
    ];
    for (let i = 0; i < kg.nodes.length; i += batch) {
      events.push({
        event: "nodes",
        data: { type: "nodes", nodes: kg.nodes.slice(i, i + batch) },
      });
    }
    events.push({
      event: "edges",
      data: { type: "edges", edges: kg.edges },
    });
    events.push({
      event: "done",
      data: {
        type: "done",
        nodes_count: kg.nodes.length,
        edges_count: kg.edges.length,
        total_nodes: kg.total_nodes,
        total_edges: kg.total_edges,
        is_truncated: kg.is_truncated,
      },
    });
    await route.fulfill({
      status: 200,
      contentType: "text/event-stream",
      headers: { "Cache-Control": "no-cache", "X-Accel-Buffering": "no" },
      body: sseBody(events),
    });
  });

  await page.route("**/api/v1/graph/nodes/search**", (route) => {
    const url = new URL(route.request().url());
    const q = (url.searchParams.get("q") ?? "").toLowerCase();
    const matches = kg.nodes
      .filter((n) => n.label.toLowerCase().includes(q) || n.id.toLowerCase().includes(q))
      .slice(0, 20);
    return route.fulfill(json({ items: matches, total: matches.length }));
  });

  await page.route("**/api/v1/graph**", (route) => {
    if (route.request().url().includes("/stream")) return route.fallback();
    return route.fulfill(json(kg));
  });

  await page.route("**/api/v1/conversations**", (route) =>
    route.fulfill(
      json({
        items: [],
        pagination: {
          has_more: false,
          next_cursor: null,
          prev_cursor: null,
          total: 0,
        },
      }),
    ),
  );

  await page.route("**/api/v1/chat/completions/stream**", async (route) => {
    const subgraph = {
      entities: kg.nodes.slice(0, 5).map((n) => ({
        id: `ent:${n.label}`,
        graph_node_id: n.id,
        name: n.label,
        entity_type: n.node_type,
        score: 0.9,
        degree: n.degree,
      })),
      relationships: kg.edges.slice(0, 4).map((e) => ({
        source: e.source,
        target: e.target,
        relation_type: e.relationship_type,
        score: 0.8,
      })),
    };
    const body = sseBody([
      { event: "context", data: { type: "context", subgraph, sources: [] } },
      {
        event: "token",
        data: { type: "token", content: "Mock answer about " },
      },
      {
        event: "token",
        data: { type: "token", content: kg.nodes[0]?.label ?? "entities" },
      },
      {
        event: "done",
        data: {
          type: "done",
          answer: `Mock answer about ${kg.nodes[0]?.label ?? "entities"}.`,
          tokens_used: 42,
          duration_ms: 120,
          llm_provider: "mock",
          llm_model: "mock",
          subgraph,
        },
      },
    ]);
    await route.fulfill({
      status: 200,
      contentType: "text/event-stream",
      body,
    });
  });

  await page.route("**/api/v1/costs**", (route) =>
    route.fulfill(
      json({
        total_cost_usd: 12.34,
        total_calls: 100,
        period: "30d",
        by_model: [{ model: "mock", cost_usd: 12.34, calls: 100 }],
        history: [
          { date: "2026-01-01", cost_usd: 1.2 },
          { date: "2026-01-02", cost_usd: 2.1 },
        ],
        budget: { limit_usd: 100, used_usd: 12.34, status: "ok" },
      }),
    ),
  );

  await page.route("**/api/v1/pipeline/**", (route) =>
    route.fulfill(
      json({
        stages: [],
        queue: { pending: 0, running: 0, failed: 0 },
        activity: [],
      }),
    ),
  );

  await page.route("**/api/v1/models**", (route) =>
    route.fulfill(json({ items: [{ id: "mock", provider: "mock", name: "Mock" }] })),
  );

  await page.route("**/api/v1/providers**", (route) =>
    route.fulfill(json({ items: [{ id: "mock", name: "Mock", status: "ready" }] })),
  );

  await page.route("**/ws/**", (route) => route.fulfill({ status: 200, body: "" }));

  // Fallback empty graph for unexpected empty workspace probes
  void FIXTURE_EMPTY;
}

export async function prepareSpec155Page(
  page: Page,
  options: Spec155MockOptions = {},
): Promise<void> {
  await mockSpec155Api(page, options);
  await seedSpec155Tenant(page);
  await page.reload({ waitUntil: "domcontentloaded" });
}

/**
 * MCP server — stdio bridge to EdgeQuake POST /mcp (SPEC-152).
 */
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { callRemoteTool } from "./gateway-bridge.js";

const QUERY_INSTRUCTIONS = `EdgeQuake is a Graph-RAG store. Do not invent document lists.
1. eq_document_list before answering "what's in the workspace".
2. Scope with document_ids when the user names a paper.
3. eq_search → eq_fetch(view=toc). Escalate view only if needed.
4. Treat entity names in ALL_CAPS as slugs; show Title Case to the user.
5. If truncation.truncated is true, fetch next_cursor before concluding the corpus is small.
6. Ignore Artifact/DRAWING entities unless the user asks about a figure.
7. Never claim an LLM "answer" from EdgeQuake; the tools return evidence.
8. This connector is query-only unless the remote profile is memory.`;

/** Accept any JSON object; gateway enforces inputSchema. */
const anyArgs = {
  query: z.string().optional(),
  q: z.string().optional(),
  mode: z.string().optional(),
  budget: z.string().optional(),
  limit: z.number().optional(),
  cursor: z.string().optional(),
  workspace_id: z.string().optional(),
  document_id: z.string().optional(),
  document_ids: z.array(z.string()).optional(),
  document_pattern: z.string().optional(),
  retrieval_id: z.string().optional(),
  entity_id: z.string().optional(),
  ids: z.array(z.string()).optional(),
  view: z.string().optional(),
  include_subgraph: z.boolean().optional(),
  include_artifacts: z.boolean().optional(),
  include_weak_edges: z.boolean().optional(),
  max_hops: z.number().optional(),
  status: z.string().optional(),
  include: z.array(z.string()).optional(),
  type: z.string().optional(),
  content: z.string().optional(),
  title: z.string().optional(),
  task_id: z.string().optional(),
  confirm: z.boolean().optional(),
  max_results: z.number().optional(),
};

function registerBridgeTool(
  server: McpServer,
  name: string,
  description: string,
): void {
  server.tool(name, description, anyArgs, async (args) => {
    try {
      const cleaned: Record<string, unknown> = {};
      for (const [k, v] of Object.entries(args ?? {})) {
        if (v !== undefined) cleaned[k] = v;
      }
      const result = await callRemoteTool(name, cleaned);
      const text =
        result.content?.[0]?.text ??
        (result.structuredContent
          ? JSON.stringify(result.structuredContent)
          : "");
      return {
        content: [{ type: "text" as const, text }],
        isError: result.isError,
      };
    } catch (err) {
      return {
        content: [
          {
            type: "text" as const,
            text: `Error: ${err instanceof Error ? err.message : String(err)}`,
          },
        ],
        isError: true,
      };
    }
  });
}

export function createServer(): McpServer {
  const server = new McpServer({
    name: "edgequake",
    version: "0.3.0",
  });

  // instructions via server API when supported; keep constant for hosts that read it
  void QUERY_INSTRUCTIONS;

  registerBridgeTool(
    server,
    "eq_document_list",
    "List documents in the workspace (not semantic search).",
  );
  registerBridgeTool(
    server,
    "eq_search",
    "Search Graph-RAG; returns hits[] + retrieval_id.",
  );
  registerBridgeTool(
    server,
    "eq_fetch",
    "Fetch bodies for a retrieval_id (default view=toc).",
  );
  registerBridgeTool(
    server,
    "eq_retrieve",
    "Search then fetch chunks; still returns hits.",
  );
  registerBridgeTool(
    server,
    "eq_entity_search",
    "Search entities (compact one_liner).",
  );
  registerBridgeTool(
    server,
    "eq_neighborhood",
    "Entity neighborhood with typed edges.",
  );
  registerBridgeTool(
    server,
    "eq_workspace_list",
    "List visible workspaces.",
  );
  registerBridgeTool(
    server,
    "eq_workspace_stats",
    "Workspace document/entity/chunk counts.",
  );
  registerBridgeTool(
    server,
    "eq_document_get",
    "Document metadata (text via eq:// resource).",
  );
  registerBridgeTool(
    server,
    "eq_entity_get",
    "Entity details with lineage resource.",
  );

  return server;
}

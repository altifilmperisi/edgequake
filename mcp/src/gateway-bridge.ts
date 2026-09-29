/**
 * Thin HTTP client for EdgeQuake remote MCP gateway (SPEC-152 stdio bridge).
 */
import { resolveConfig } from "./config.js";

export interface JsonRpcResult {
  result?: unknown;
  error?: { code: number; message: string };
}

export async function mcpRpc(
  method: string,
  params?: Record<string, unknown>,
): Promise<JsonRpcResult> {
  const cfg = resolveConfig();
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    Accept: "application/json, text/event-stream",
    "MCP-Protocol-Version": "2026-07-28",
  };
  if (cfg.apiKey) {
    headers.Authorization = `Bearer ${cfg.apiKey}`;
  }
  if (cfg.defaultWorkspace) {
    headers["X-Workspace-Id"] = cfg.defaultWorkspace;
  }

  const body = {
    jsonrpc: "2.0",
    id: 1,
    method,
    params: params ?? {},
  };

  const res = await fetch(`${cfg.baseUrl.replace(/\/$/, "")}/mcp`, {
    method: "POST",
    headers,
    body: JSON.stringify(body),
  });
  const json = (await res.json()) as JsonRpcResult;
  return json;
}

export async function listRemoteTools(): Promise<
  Array<{
    name: string;
    description?: string;
    inputSchema?: Record<string, unknown>;
  }>
> {
  const rpc = await mcpRpc("tools/list");
  if (rpc.error) {
    throw new Error(rpc.error.message);
  }
  const result = rpc.result as { tools?: Array<Record<string, unknown>> };
  return (result.tools ?? []).map((t) => ({
    name: String(t.name),
    description: t.description ? String(t.description) : undefined,
    inputSchema: (t.inputSchema as Record<string, unknown>) ?? {
      type: "object",
      properties: {},
    },
  }));
}

export async function callRemoteTool(
  name: string,
  args: Record<string, unknown>,
): Promise<{
  content: Array<{ type: string; text?: string }>;
  structuredContent?: unknown;
  isError?: boolean;
}> {
  const rpc = await mcpRpc("tools/call", { name, arguments: args });
  if (rpc.error) {
    return {
      content: [{ type: "text", text: rpc.error.message }],
      isError: true,
    };
  }
  return rpc.result as {
    content: Array<{ type: string; text?: string }>;
    structuredContent?: unknown;
    isError?: boolean;
  };
}

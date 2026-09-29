//! MCP profile: query (default) vs memory (write tools).

/// Which tool surface the gateway advertises.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpProfile {
    /// Read-only catalog + retrieval + graph (remote / demo default).
    Query,
    /// Query + ingest / task / deletes.
    Memory,
}

/// Resolve profile from `EDGEQUAKE_MCP_PROFILE` (`query` | `memory`). Default: query.
pub fn mcp_profile() -> McpProfile {
    match std::env::var("EDGEQUAKE_MCP_PROFILE")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "memory" => McpProfile::Memory,
        _ => McpProfile::Query,
    }
}

pub const QUERY_INSTRUCTIONS: &str = r#"EdgeQuake is a Graph-RAG store. Do not invent document lists.
1. eq_document_list before answering "what's in the workspace".
2. Scope with document_ids when the user names a paper.
3. eq_search → eq_fetch(view=toc). Escalate view only if needed.
4. Treat entity names in ALL_CAPS as slugs; show Title Case to the user.
5. If truncation.truncated is true, fetch next_cursor before concluding the corpus is small.
6. Ignore Artifact/DRAWING entities unless the user asks about a figure.
7. Never claim an LLM "answer" from EdgeQuake; the tools return evidence.
8. This connector is query-only; ingest and delete are unavailable."#;

pub const MEMORY_INSTRUCTIONS: &str = r#"EdgeQuake is a Graph-RAG store. Do not invent document lists.
1. eq_document_list before answering "what's in the workspace".
2. Scope with document_ids when the user names a paper.
3. eq_search → eq_fetch(view=toc). Escalate view only if needed.
4. Treat entity names in ALL_CAPS as slugs; show Title Case to the user.
5. If truncation.truncated is true, fetch next_cursor before concluding the corpus is small.
6. Ignore Artifact/DRAWING entities unless the user asks about a figure.
7. Never claim an LLM "answer" from EdgeQuake; the tools return evidence.
8. Ingest via eq_ingest and poll eq_task_get. Deletes require confirm: true."#;

pub fn instructions_for_profile(profile: McpProfile) -> &'static str {
    match profile {
        McpProfile::Query => QUERY_INSTRUCTIONS,
        McpProfile::Memory => MEMORY_INSTRUCTIONS,
    }
}

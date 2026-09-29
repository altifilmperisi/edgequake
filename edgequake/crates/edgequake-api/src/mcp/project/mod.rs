//! EQ-MCP-1.0 AgentView projection (SPEC-152).
//!
//! Maps REST/query DTOs into the agent envelope. Transport stays in `gateway/`.

pub mod budget;
pub mod catalog;
pub mod envelope;
pub mod errors;
pub mod fetch;
pub mod graph;
pub mod ids;
pub mod profile;
pub mod scores;
pub mod search;
pub mod summary;

pub use budget::{apply_budget, BudgetClass};
pub use envelope::{truncation_ok, EnvelopeBuilder};
pub use errors::{eq_error, ErrorCode};
pub use profile::{mcp_profile, McpProfile, MEMORY_INSTRUCTIONS, QUERY_INSTRUCTIONS};
pub use summary::{call_tool_error_structured, call_tool_result};

//! Typed EQ-MCP error codes (SPEC-152 §11).

use serde_json::{json, Value};

#[derive(Debug, Clone, Copy)]
pub enum ErrorCode {
    InvalidId,
    NotFound,
    ScopeRequired,
    NotReady,
    BudgetExceeded,
    TruncateInvalid,
    Forbidden,
    ConfirmRequired,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidId => "eq/invalid_id",
            Self::NotFound => "eq/not_found",
            Self::ScopeRequired => "eq/scope_required",
            Self::NotReady => "eq/not_ready",
            Self::BudgetExceeded => "eq/budget_exceeded",
            Self::TruncateInvalid => "eq/truncate_invalid",
            Self::Forbidden => "eq/forbidden",
            Self::ConfirmRequired => "eq/confirm_required",
        }
    }
}

/// Build an error envelope (`ok: false`) for structuredContent.
pub fn eq_error(code: ErrorCode, message: impl Into<String>, task_id: Option<&str>) -> Value {
    let mut err = json!({
        "code": code.as_str(),
        "message": message.into(),
    });
    if let Some(tid) = task_id {
        err["task_id"] = json!(tid);
    }
    json!({
        "ok": false,
        "view": "error",
        "budget_used": "standard",
        "truncation": { "truncated": false },
        "error": err
    })
}

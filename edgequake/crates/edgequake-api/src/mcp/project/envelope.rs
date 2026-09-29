//! Shared read envelope helpers (SPEC-152 §6 / 05-envelope-budget-errors).

use serde_json::{json, Map, Value};

use super::budget::BudgetClass;

/// Build truncation object with `truncated: false`.
pub fn truncation_ok() -> Value {
    json!({ "truncated": false })
}

pub struct EnvelopeBuilder {
    view: String,
    budget: BudgetClass,
    fields: Map<String, Value>,
}

impl EnvelopeBuilder {
    pub fn new(view: impl Into<String>, budget: BudgetClass) -> Self {
        Self {
            view: view.into(),
            budget,
            fields: Map::new(),
        }
    }

    pub fn insert(mut self, key: impl Into<String>, value: Value) -> Self {
        self.fields.insert(key.into(), value);
        self
    }

    pub fn truncation(mut self, truncation: Value) -> Self {
        self.fields.insert("truncation".into(), truncation);
        self
    }

    pub fn build(mut self) -> Value {
        let mut out = Map::new();
        out.insert("ok".into(), json!(true));
        out.insert("view".into(), json!(self.view));
        out.insert("budget_used".into(), json!(self.budget.as_str()));
        if !self.fields.contains_key("truncation") {
            out.insert("truncation".into(), truncation_ok());
        }
        out.append(&mut self.fields);
        Value::Object(out)
    }
}

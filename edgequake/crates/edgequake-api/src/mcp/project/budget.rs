//! Server-enforced context budgets (SPEC-152 §3).

use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetClass {
    Cheap,
    Standard,
    Deep,
}

impl BudgetClass {
    pub fn parse(s: Option<&str>) -> Self {
        match s.map(|v| v.to_ascii_lowercase()).as_deref() {
            Some("cheap") => Self::Cheap,
            Some("deep") => Self::Deep,
            _ => Self::Standard,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cheap => "cheap",
            Self::Standard => "standard",
            Self::Deep => "deep",
        }
    }

    pub fn max_chunks(self) -> usize {
        match self {
            Self::Cheap => 5,
            Self::Standard => 12,
            Self::Deep => 30,
        }
    }

    pub fn max_entities(self) -> usize {
        match self {
            Self::Cheap => 8,
            Self::Standard => 16,
            Self::Deep => 40,
        }
    }

    pub fn max_one_liner(self) -> usize {
        match self {
            Self::Cheap => 160,
            Self::Standard => 280,
            Self::Deep => 800,
        }
    }

    pub fn max_kib(self) -> usize {
        match self {
            Self::Cheap => 8,
            Self::Standard => 24,
            Self::Deep => 80,
        }
    }

    pub fn max_bytes(self) -> usize {
        self.max_kib() * 1024
    }
}

/// Truncate arrays by score and enforce KiB cap on the full envelope value.
///
/// Expects `chunks` / `entities` / `relationships` arrays with optional `score`.
/// Returns `(value, still_over_cap)`.
pub fn apply_budget(mut envelope: Value, budget: BudgetClass) -> (Value, bool) {
    let omitted_chunks = truncate_scored_array(&mut envelope, "chunks", budget.max_chunks());
    let omitted_entities = truncate_scored_array(&mut envelope, "entities", budget.max_entities());
    let omitted_relationships =
        truncate_scored_array(&mut envelope, "relationships", budget.max_entities());

    let mut truncated = omitted_chunks > 0 || omitted_entities > 0 || omitted_relationships > 0;
    let mut next_cursor: Option<String> = None;

    if truncated {
        // Cursor = offset into original chunk list after keep.
        let kept = envelope
            .get("chunks")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        next_cursor = Some(format!("chunks:{kept}"));
    }

    // Shrink string fields if still over bytes.
    let mut bytes = serde_json::to_vec(&envelope).map(|v| v.len()).unwrap_or(0);
    if bytes > budget.max_bytes() {
        shrink_text_fields(&mut envelope, budget);
        bytes = serde_json::to_vec(&envelope).map(|v| v.len()).unwrap_or(0);
        truncated = true;
    }

    let still_over = bytes > budget.max_bytes();
    if still_over {
        // Last resort: clear heavy arrays.
        if let Some(obj) = envelope.as_object_mut() {
            obj.insert("chunks".into(), json!([]));
            obj.insert("entities".into(), json!([]));
            obj.insert("relationships".into(), json!([]));
        }
        truncated = true;
    }

    let final_over = serde_json::to_vec(&envelope)
        .map(|v| v.len() > budget.max_bytes())
        .unwrap_or(false);

    if let Some(obj) = envelope.as_object_mut() {
        let mut trunc = json!({ "truncated": truncated });
        if let Some(c) = next_cursor {
            trunc["next_cursor"] = json!(c);
        }
        if omitted_chunks > 0 {
            trunc["omitted_chunks"] = json!(omitted_chunks);
        }
        if omitted_entities > 0 {
            trunc["omitted_entities"] = json!(omitted_entities);
        }
        if omitted_relationships > 0 {
            trunc["omitted_relationships"] = json!(omitted_relationships);
        }
        obj.insert("truncation".into(), trunc);
        obj.insert("budget_used".into(), json!(budget.as_str()));
    }

    (envelope, final_over)
}

fn truncate_scored_array(envelope: &mut Value, key: &str, max: usize) -> usize {
    let Some(arr) = envelope.get_mut(key).and_then(|v| v.as_array_mut()) else {
        return 0;
    };
    if arr.len() <= max {
        return 0;
    }
    arr.sort_by(|a, b| {
        let sa = a.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let sb = b.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        sb.partial_cmp(&sa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let ia = a.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let ib = b.get("id").and_then(|v| v.as_str()).unwrap_or("");
                ia.cmp(ib)
            })
    });
    let omitted = arr.len() - max;
    arr.truncate(max);
    omitted
}

fn shrink_text_fields(envelope: &mut Value, budget: BudgetClass) {
    let cap = budget.max_one_liner();
    for key in ["chunks", "entities", "hits", "citations"] {
        let Some(arr) = envelope.get_mut(key).and_then(|v| v.as_array_mut()) else {
            continue;
        };
        for item in arr.iter_mut() {
            for field in [
                "text",
                "one_liner",
                "description",
                "snippet",
                "quote",
                "content",
            ] {
                if let Some(Value::String(s)) = item.get_mut(field) {
                    if s.chars().count() > cap {
                        *s = s.chars().take(cap).collect();
                    }
                }
            }
        }
    }
}

/// Resume fetch from `chunks:N` cursor without duplicating ids already returned.
pub fn apply_chunk_cursor(chunks: &mut Vec<Value>, cursor: Option<&str>) -> Vec<Value> {
    let offset = cursor
        .and_then(|c| c.strip_prefix("chunks:"))
        .and_then(|n| n.parse::<usize>().ok())
        .unwrap_or(0);
    if offset == 0 {
        return std::mem::take(chunks);
    }
    if offset >= chunks.len() {
        return Vec::new();
    }
    chunks.split_off(offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_chunks_by_score() {
        let env = json!({
            "ok": true,
            "view": "chunks",
            "budget_used": "cheap",
            "chunks": [
                {"id": "a", "score": 0.1, "text": "x"},
                {"id": "b", "score": 0.9, "text": "y"},
                {"id": "c", "score": 0.5, "text": "z"},
                {"id": "d", "score": 0.8, "text": "w"},
                {"id": "e", "score": 0.2, "text": "v"},
                {"id": "f", "score": 0.7, "text": "u"},
            ],
            "truncation": {"truncated": false}
        });
        let (out, over) = apply_budget(env, BudgetClass::Cheap);
        assert!(!over);
        assert_eq!(out["chunks"].as_array().unwrap().len(), 5);
        assert_eq!(out["truncation"]["truncated"], true);
        assert_eq!(out["truncation"]["omitted_chunks"], 1);
    }
}

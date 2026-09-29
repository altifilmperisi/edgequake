//! ≤2 KiB human summary for CallToolResult content[0].text (SPEC-152).

use serde_json::{json, Value};

const MAX_SUMMARY_CHARS: usize = 2048;
const MAX_ITEM_LINES: usize = 8;

/// Build CallToolResult with summary text + structuredContent (no JSON clone).
pub fn call_tool_result(structured: Value) -> Value {
    let is_error = structured
        .get("ok")
        .and_then(|v| v.as_bool())
        .is_some_and(|ok| !ok);
    let text = summarize(&structured);
    let mut result = json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": structured
    });
    if is_error {
        result["isError"] = json!(true);
    }
    result
}

/// Error CallToolResult with typed structuredContent when available.
pub fn call_tool_error_structured(structured: Value) -> Value {
    let text = structured
        .pointer("/error/message")
        .and_then(|v| v.as_str())
        .unwrap_or("tool error")
        .to_string();
    json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": structured,
        "isError": true
    })
}

fn summarize(structured: &Value) -> String {
    let mut lines: Vec<String> = Vec::new();

    let ok = structured
        .get("ok")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let view = structured
        .get("view")
        .and_then(|v| v.as_str())
        .unwrap_or("result");
    let budget = structured
        .get("budget_used")
        .and_then(|v| v.as_str())
        .unwrap_or("standard");

    if !ok {
        let code = structured
            .pointer("/error/code")
            .and_then(|v| v.as_str())
            .unwrap_or("eq/error");
        let msg = structured
            .pointer("/error/message")
            .and_then(|v| v.as_str())
            .unwrap_or("error");
        lines.push(format!("error {code}: {msg}"));
        return truncate_join(&lines);
    }

    let mut status = format!("ok {view} budget={budget}");
    if let Some(mode) = structured.get("mode_used").and_then(|v| v.as_str()) {
        status.push_str(&format!(" mode={mode}"));
    }
    if let Some(rid) = structured.get("retrieval_id").and_then(|v| v.as_str()) {
        status.push_str(&format!(" retrieval={rid}"));
    }
    lines.push(status);

    push_hit_lines(&mut lines, structured);
    push_doc_lines(&mut lines, structured);
    push_entity_lines(&mut lines, structured);
    push_chunk_lines(&mut lines, structured);

    if structured
        .pointer("/truncation/truncated")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        let cursor = structured
            .pointer("/truncation/next_cursor")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let omitted_c = structured
            .pointer("/truncation/omitted_chunks")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let omitted_e = structured
            .pointer("/truncation/omitted_entities")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        lines.push(format!(
            "truncated=true omitted_chunks={omitted_c} omitted_entities={omitted_e} next_cursor={cursor}"
        ));
    }

    truncate_join(&lines)
}

fn push_hit_lines(lines: &mut Vec<String>, structured: &Value) {
    let Some(hits) = structured.get("hits").and_then(|v| v.as_array()) else {
        return;
    };
    for hit in hits.iter().take(MAX_ITEM_LINES) {
        let kind = hit.get("kind").and_then(|v| v.as_str()).unwrap_or("hit");
        let id = hit.get("id").and_then(|v| v.as_str()).unwrap_or("-");
        let score = hit
            .get("score")
            .and_then(|v| v.as_f64())
            .map(|s| format!("{s:.2}"))
            .unwrap_or_else(|| "-".into());
        let title = hit
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .chars()
            .take(80)
            .collect::<String>();
        lines.push(format!("{kind} {id} score={score} {title}"));
    }
}

fn push_doc_lines(lines: &mut Vec<String>, structured: &Value) {
    let docs = structured
        .get("documents")
        .or_else(|| structured.get("items"))
        .and_then(|v| v.as_array());
    let Some(docs) = docs else {
        return;
    };
    // Prefer hits when present for line budget.
    if structured.get("hits").and_then(|v| v.as_array()).is_some() {
        return;
    }
    for doc in docs.iter().take(MAX_ITEM_LINES) {
        let id = doc.get("id").and_then(|v| v.as_str()).unwrap_or("-");
        let name = doc
            .get("file_name")
            .or_else(|| doc.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let status = doc.get("status").and_then(|v| v.as_str()).unwrap_or("");
        lines.push(format!("doc {id} {name} {status}").trim().to_string());
    }
}

fn push_entity_lines(lines: &mut Vec<String>, structured: &Value) {
    if lines.len() > MAX_ITEM_LINES + 1 {
        return;
    }
    let Some(entities) = structured.get("entities").and_then(|v| v.as_array()) else {
        return;
    };
    if structured.get("hits").and_then(|v| v.as_array()).is_some() {
        return;
    }
    for ent in entities
        .iter()
        .take(MAX_ITEM_LINES.saturating_sub(lines.len().saturating_sub(1)))
    {
        let id = ent.get("id").and_then(|v| v.as_str()).unwrap_or("-");
        let name = ent.get("name").and_then(|v| v.as_str()).unwrap_or("");
        lines.push(format!("entity {id} {name}"));
    }
}

fn push_chunk_lines(lines: &mut Vec<String>, structured: &Value) {
    if lines.len() > MAX_ITEM_LINES + 1 {
        return;
    }
    let Some(chunks) = structured.get("chunks").and_then(|v| v.as_array()) else {
        return;
    };
    if structured.get("hits").and_then(|v| v.as_array()).is_some() {
        return;
    }
    for chunk in chunks
        .iter()
        .take(MAX_ITEM_LINES.saturating_sub(lines.len().saturating_sub(1)))
    {
        let id = chunk.get("id").and_then(|v| v.as_str()).unwrap_or("-");
        let score = chunk
            .get("score")
            .and_then(|v| v.as_f64())
            .map(|s| format!("{s:.2}"))
            .unwrap_or_else(|| "-".into());
        let file = chunk
            .get("file_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        lines.push(format!("chunk {id} score={score} {file}"));
    }
}

fn truncate_join(lines: &[String]) -> String {
    let mut text = lines.join("\n");
    if text.chars().count() > MAX_SUMMARY_CHARS {
        text = text
            .chars()
            .take(MAX_SUMMARY_CHARS.saturating_sub(1))
            .collect();
        text.push('…');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn summary_is_not_json_clone() {
        let structured = json!({
            "ok": true,
            "view": "search",
            "budget_used": "standard",
            "truncation": { "truncated": false },
            "hits": [{
                "kind": "chunk",
                "id": "doc-chunk-1",
                "title": "Hello",
                "score": 0.9
            }]
        });
        let result = call_tool_result(structured.clone());
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.starts_with("ok search"));
        assert_ne!(text, serde_json::to_string(&structured).unwrap());
        assert!(text.len() <= MAX_SUMMARY_CHARS);
    }
}

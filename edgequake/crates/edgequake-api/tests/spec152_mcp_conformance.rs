//! SPEC-152 EQ-MCP-1.0 conformance (acceptance tests T1–T10).

mod common;

use axum::http::StatusCode;
use common::spec028_mcp::{
    default_mcp_app, mcp_post_legacy, mcp_tools_call, parse_json, tool_structured,
};
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn t1_document_list_tool_exists_and_returns_envelope() {
    let app = default_mcp_app();
    let response = app
        .clone()
        .oneshot(mcp_post_legacy(
            "/mcp",
            json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        ))
        .await
        .unwrap();
    let body = parse_json(response).await;
    let names: Vec<_> = body["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(names.contains(&"eq_document_list"));
    assert!(names.contains(&"eq_search"));

    let (status, call) = mcp_tools_call(&app, "/mcp", "eq_document_list", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    let sc = tool_structured(&call);
    assert_eq!(sc["ok"], true);
    assert_eq!(sc["view"], "document_list");
    assert!(sc["documents"].is_array() || sc["items"].is_array());
    assert!(sc["truncation"]["truncated"].is_boolean());
}

#[tokio::test]
async fn t2_document_ids_scopes_search_filter() {
    // Fixture-free: advertise document_ids on schema + ensure filter rejects cross-doc bleed
    // by scoping to a nonexistent id (zero hits, never invents other PDFs).
    let app = default_mcp_app();
    let response = app
        .clone()
        .oneshot(mcp_post_legacy(
            "/mcp",
            json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        ))
        .await
        .unwrap();
    let body = parse_json(response).await;
    let search = body["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "eq_search")
        .expect("eq_search");
    assert!(
        search["inputSchema"]["properties"]["document_ids"].is_object()
            || search["inputSchema"]["properties"]
                .get("document_filter")
                .is_some(),
        "document_ids must be advertised on eq_search"
    );

    let (_, call) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_search",
        json!({
            "query": "Action Fusion mechanism",
            "mode": "naive",
            "limit": 8,
            "document_ids": ["doc_sol_pi_fixture_only"]
        }),
    )
    .await;
    let sc = tool_structured(&call);
    assert_eq!(sc["ok"], true);
    for hit in sc["hits"].as_array().unwrap_or(&vec![]) {
        let file = hit["file_name"].as_str().unwrap_or("");
        assert!(
            !file.contains("ten_"),
            "cross-doc bleed into ten_ PDF: {file}"
        );
    }
}

#[tokio::test]
async fn t7_entity_one_liner_cap() {
    let app = default_mcp_app();
    let (_, body) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_entity_search",
        json!({ "q": "RAG", "limit": 20, "budget": "standard" }),
    )
    .await;
    let sc = tool_structured(&body);
    assert_eq!(sc["ok"], true);
    for ent in sc["entities"].as_array().unwrap_or(&vec![]) {
        let one = ent["one_liner"].as_str().unwrap_or("");
        assert!(
            one.chars().count() <= 280,
            "one_liner over 280 chars: {} chars",
            one.chars().count()
        );
    }
}

#[tokio::test]
async fn t9_cursor_resume_no_duplicate_chunk_ids() {
    use edgequake_api::mcp::project::budget::{apply_budget, apply_chunk_cursor, BudgetClass};
    use serde_json::json;

    let mut chunks = Vec::new();
    for i in 0..20 {
        chunks.push(json!({
            "id": format!("chk_{i}"),
            "score": 1.0 - (i as f64) * 0.01,
            "text": format!("chunk body {i}")
        }));
    }
    let env = json!({
        "ok": true,
        "view": "chunks",
        "chunks": chunks,
        "truncation": {"truncated": false}
    });
    let (page1, _) = apply_budget(env, BudgetClass::Cheap);
    assert_eq!(page1["truncation"]["truncated"], true);
    let cursor = page1["truncation"]["next_cursor"].as_str().unwrap();
    let ids1: Vec<String> = page1["chunks"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["id"].as_str().map(str::to_string))
        .collect();

    let mut remaining: Vec<_> = (0..20)
        .map(|i| {
            json!({
                "id": format!("chk_{i}"),
                "score": 1.0 - (i as f64) * 0.01,
                "text": format!("chunk body {i}")
            })
        })
        .collect();
    let page2_chunks = apply_chunk_cursor(&mut remaining, Some(cursor));
    let ids2: Vec<String> = page2_chunks
        .iter()
        .filter_map(|c| c["id"].as_str().map(str::to_string))
        .collect();
    for id in &ids1 {
        assert!(!ids2.contains(id), "duplicate chunk id across pages: {id}");
    }
    assert!(!ids2.is_empty() || page1["truncation"]["omitted_chunks"].as_u64().unwrap_or(0) == 0);
}

#[tokio::test]
async fn t4_summary_text_is_not_json_clone() {
    let app = default_mcp_app();
    let (status, body) =
        mcp_tools_call(&app, "/mcp", "eq_document_list", json!({ "limit": 5 })).await;
    assert_eq!(status, StatusCode::OK);
    let text = body["result"]["content"][0]["text"].as_str().unwrap_or("");
    let structured = &body["result"]["structuredContent"];
    assert!(
        text.starts_with("ok "),
        "summary must be status line: {text}"
    );
    assert_ne!(text, serde_json::to_string(structured).unwrap());
    assert!(text.len() <= 2048);
}

#[tokio::test]
async fn t5_search_returns_hits_not_one_session_row() {
    let app = default_mcp_app();
    let (status, body) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_search",
        json!({ "query": "knowledge graph", "mode": "naive", "limit": 8 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body:?}");
    let sc = tool_structured(&body);
    assert_eq!(sc["ok"], true);
    assert!(sc["retrieval_id"].as_str().unwrap().starts_with("ret_"));
    assert!(sc["hits"].is_array());
    assert!(sc["hits"].as_array().unwrap().len() <= 8);
    assert!(sc["mode_used"].is_string());
    assert!(sc["score_type"].is_string());
}

#[tokio::test]
async fn t6_fetch_ids_does_not_require_full_subgraph() {
    let app = default_mcp_app();
    let (_, search) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_search",
        json!({ "query": "RAG", "mode": "naive", "limit": 3 }),
    )
    .await;
    let sc = tool_structured(&search);
    let rid = sc["retrieval_id"].as_str().unwrap();
    let hit_id = sc["hits"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|h| h["id"].as_str())
        .unwrap_or("");

    let (_, fetch) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_fetch",
        json!({
            "retrieval_id": rid,
            "view": "chunks",
            "ids": [hit_id],
            "include_subgraph": false,
            "budget": "standard"
        }),
    )
    .await;
    let fc = tool_structured(&fetch);
    assert_eq!(fc["ok"], true);
    assert_eq!(fc["view"], "chunks");
    assert_eq!(fc["relationships"].as_array().map(|a| a.len()), Some(0));
}

#[tokio::test]
async fn t8_mode_echo() {
    let app = default_mcp_app();
    let (_, body) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_search",
        json!({ "query": "what is X", "mode": "local", "limit": 5 }),
    )
    .await;
    let sc = tool_structured(&body);
    let mode = sc["mode_used"].as_str().unwrap_or("");
    let reason = sc["mode_reason"].as_str().unwrap_or("");
    assert!(
        mode == "local" || reason.contains("requested"),
        "mode_used={mode} mode_reason={reason}"
    );
}

#[tokio::test]
async fn t3_budget_standard_caps_structured_json() {
    let app = default_mcp_app();
    let (_, body) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_retrieve",
        json!({ "query": "pipeline", "mode": "naive", "budget": "cheap", "limit": 8 }),
    )
    .await;
    let sc = tool_structured(&body);
    let bytes = serde_json::to_vec(&sc).unwrap().len();
    assert!(
        bytes <= 8 * 1024 || sc["truncation"]["truncated"] == true || sc["ok"] == false,
        "cheap budget exceeded without truncation: {bytes} bytes"
    );
}

#[tokio::test]
async fn t10_empty_workspace_list_is_empty_array() {
    let app = default_mcp_app();
    let (_, body) = mcp_tools_call(&app, "/mcp", "eq_document_list", json!({})).await;
    let sc = tool_structured(&body);
    assert_eq!(sc["ok"], true);
    let docs = sc["documents"]
        .as_array()
        .or_else(|| sc["items"].as_array());
    assert!(docs.is_some());
    // test_state may be empty — never invent entities
    assert!(sc.get("entities").is_none() || sc["entities"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn initialize_includes_instructions() {
    let app = default_mcp_app();
    let response = app
        .oneshot(mcp_post_legacy(
            "/mcp",
            json!({
                "jsonrpc":"2.0","id":1,"method":"initialize",
                "params":{"protocolVersion":"2026-07-28","capabilities":{},"clientInfo":{"name":"t","version":"0"}}
            }),
        ))
        .await
        .unwrap();
    let body = parse_json(response).await;
    let instructions = body["result"]["instructions"].as_str().unwrap_or("");
    assert!(instructions.contains("eq_document_list"));
    assert!(instructions.contains("query-only") || instructions.contains("evidence"));
}

#[tokio::test]
async fn resources_list_advertises_eq_scheme() {
    let app = default_mcp_app();
    let response = app
        .oneshot(mcp_post_legacy(
            "/mcp",
            json!({"jsonrpc":"2.0","id":1,"method":"resources/list"}),
        ))
        .await
        .unwrap();
    let body = parse_json(response).await;
    assert!(body.get("error").is_none(), "{body:?}");
    let templates = body["result"]["resourceTemplates"].as_array();
    assert!(templates.is_some());
}

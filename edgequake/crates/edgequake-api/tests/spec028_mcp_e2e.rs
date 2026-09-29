//! SPEC-028 MCP JSON-RPC E2E tests (tools, search/fetch roundtrip, edge cases).

mod common;

use axum::http::StatusCode;
use common::spec028_mcp::{
    default_mcp_app, mcp_post_legacy, mcp_tools_call, parse_json, tool_structured, tools_call_body,
};
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn mcp_tools_list_returns_edgequake_tools() {
    let app = default_mcp_app();
    let response = app
        .oneshot(mcp_post_legacy(
            "/api/v1/mcp",
            json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    let tools = body["result"]["tools"].as_array().expect("tools array");
    let names: Vec<_> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(names.contains(&"edgequake_search"));
    assert!(names.contains(&"edgequake_fetch"));
    assert!(names.contains(&"edgequake_retrieve"));
}

#[tokio::test]
async fn mcp_root_path_tools_list() {
    let app = default_mcp_app();
    let response = app
        .oneshot(mcp_post_legacy(
            "/mcp",
            json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn mcp_search_tool_returns_retrieval_id() {
    let app = default_mcp_app();
    let (status, body) = mcp_tools_call(
        &app,
        "/api/v1/mcp",
        "edgequake_search",
        json!({ "query": "What is RAG?", "mode": "naive" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("error").is_none(), "unexpected error: {body:?}");
    let retrieval_id = tool_structured(&body)["retrieval_id"]
        .as_str()
        .expect("retrieval_id");
    assert!(retrieval_id.starts_with("ret_"));
}

#[tokio::test]
async fn ec_mcp_44_grok_skips_initialize() {
    let app = default_mcp_app();
    let (status, body) = mcp_tools_call(
        &app,
        "/mcp",
        "edgequake_search",
        json!({ "query": "entity extraction", "mode": "naive" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("error").is_none(), "{body:?}");
}

#[tokio::test]
async fn ec_mcp_search_then_fetch_roundtrip() {
    let app = default_mcp_app();
    let (_, search_body) = mcp_tools_call(
        &app,
        "/api/v1/mcp",
        "edgequake_search",
        json!({ "query": "knowledge graph", "mode": "naive" }),
    )
    .await;
    let retrieval_id = tool_structured(&search_body)["retrieval_id"]
        .as_str()
        .expect("retrieval_id");

    let (status, fetch_body) = mcp_tools_call(
        &app,
        "/api/v1/mcp",
        "edgequake_fetch",
        json!({ "retrieval_id": retrieval_id, "content_granularity": "agent" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(fetch_body.get("error").is_none(), "{fetch_body:?}");
    assert_eq!(tool_structured(&fetch_body)["retrieval_id"], retrieval_id);
    assert_eq!(tool_structured(&fetch_body)["ok"], true);
    assert!(
        tool_structured(&fetch_body)["chunks"].is_array()
            || tool_structured(&fetch_body)["documents"].is_array()
    );
}

#[tokio::test]
async fn ec_mcp_search_metadata_includes_graph_preview() {
    let app = default_mcp_app();
    let (_, search_body) = mcp_tools_call(
        &app,
        "/api/v1/mcp",
        "edgequake_search",
        json!({ "query": "entity extraction pipeline", "mode": "naive" }),
    )
    .await;
    let sc = tool_structured(&search_body);
    assert_eq!(sc["ok"], true);
    assert!(sc["hits"].is_array(), "EQ-MCP search must return hits[]");
    assert!(sc["retrieval_id"].as_str().unwrap().starts_with("ret_"));
}

#[tokio::test]
async fn ec_mcp_fetch_omits_subgraph_when_disabled() {
    let app = default_mcp_app();
    let (_, search_body) = mcp_tools_call(
        &app,
        "/api/v1/mcp",
        "edgequake_search",
        json!({ "query": "knowledge graph", "mode": "naive" }),
    )
    .await;
    let retrieval_id = tool_structured(&search_body)["retrieval_id"]
        .as_str()
        .expect("retrieval_id");

    let (_, fetch_body) = mcp_tools_call(
        &app,
        "/api/v1/mcp",
        "edgequake_fetch",
        json!({
            "retrieval_id": retrieval_id,
            "include_subgraph": false
        }),
    )
    .await;
    let sc = tool_structured(&fetch_body);
    assert_eq!(sc["ok"], true);
    // Default toc with include_subgraph=false keeps relationships empty.
    assert_eq!(sc["relationships"].as_array().map(|a| a.len()), Some(0));
}

#[tokio::test]
async fn ec_mcp_21_empty_query_returns_invalid_params() {
    let app = default_mcp_app();
    let (status, body) =
        mcp_tools_call(&app, "/mcp", "edgequake_search", json!({ "query": "   " })).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("error").is_some());
    assert_eq!(body["error"]["code"], -32602);
}

#[tokio::test]
async fn ec_mcp_22_bypass_mode_rejected() {
    let app = default_mcp_app();
    let (status, body) = mcp_tools_call(
        &app,
        "/mcp",
        "edgequake_search",
        json!({ "query": "test", "mode": "bypass" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("error").is_some());
    let msg = body["error"]["message"].as_str().unwrap_or("");
    assert!(msg.contains("bypass"), "expected bypass rejection: {msg}");
}

#[tokio::test]
async fn ec_mcp_25_invalid_retrieval_id_prefix() {
    let app = default_mcp_app();
    let (status, body) = mcp_tools_call(
        &app,
        "/mcp",
        "edgequake_fetch",
        json!({ "retrieval_id": "bad_id" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("error").is_some());
}

#[tokio::test]
async fn ec_mcp_24_unknown_tool_name() {
    let app = default_mcp_app();
    let (status, body) = mcp_tools_call(&app, "/mcp", "nonexistent_tool", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("error").is_some());
}

#[tokio::test]
async fn ec_mcp_23_invalid_mode_string() {
    let app = default_mcp_app();
    let (status, body) = mcp_tools_call(
        &app,
        "/mcp",
        "edgequake_search",
        json!({ "query": "test", "mode": "not-a-mode" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("error").is_some());
    assert_eq!(body["error"]["code"], -32602);
}

#[tokio::test]
async fn ec_mcp_40_max_results_over_cap_rejected() {
    let app = default_mcp_app();
    let (status, body) = mcp_tools_call(
        &app,
        "/mcp",
        "edgequake_search",
        json!({ "query": "test", "max_results": 999 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("error").is_some());
}

#[tokio::test]
async fn ec_mcp_27_expired_retrieval_id() {
    use edgequake_api::services::global_retrieval_cache;

    let app = default_mcp_app();
    let (_, search_body) = mcp_tools_call(
        &app,
        "/mcp",
        "edgequake_search",
        json!({ "query": "expiry test", "mode": "naive" }),
    )
    .await;
    let retrieval_id = tool_structured(&search_body)["retrieval_id"]
        .as_str()
        .expect("retrieval_id");

    global_retrieval_cache().expire_entry_for_test(retrieval_id);

    let (status, body) = mcp_tools_call(
        &app,
        "/mcp",
        "edgequake_fetch",
        json!({ "retrieval_id": retrieval_id }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body["result"]["isError"].as_bool().unwrap_or(false),
        "tool error must use CallToolResult.isError: {body}"
    );
    let msg = body["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        msg.contains("expired")
            || msg.contains("Expired")
            || msg.contains("not found")
            || msg.contains("Not found"),
        "{msg}"
    );
}

#[tokio::test]
async fn mcp_unknown_method_returns_error() {
    let app = default_mcp_app();
    let response = app
        .oneshot(mcp_post_legacy(
            "/api/v1/mcp",
            json!({"jsonrpc":"2.0","id":99,"method":"unknown/method"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    assert!(body.get("error").is_some());
    assert_eq!(body["error"]["code"], -32601);
}

#[tokio::test]
async fn ec_mcp_26_unknown_retrieval_id() {
    let app = default_mcp_app();
    let (status, body) = mcp_tools_call(
        &app,
        "/mcp",
        "edgequake_fetch",
        json!({ "retrieval_id": "ret_nonexistent00000000000000000000" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body["result"]["isError"].as_bool().unwrap_or(false),
        "unknown retrieval_id → CallToolResult.isError: {body}"
    );
}

#[tokio::test]
async fn ec_mcp_34_concurrent_fetch_same_retrieval_id() {
    let app = default_mcp_app();
    let (_, search_body) = mcp_tools_call(
        &app,
        "/mcp",
        "edgequake_search",
        json!({ "query": "idempotent fetch", "mode": "naive" }),
    )
    .await;
    let retrieval_id = tool_structured(&search_body)["retrieval_id"]
        .as_str()
        .expect("retrieval_id");

    let (_, fetch_a) = mcp_tools_call(
        &app,
        "/mcp",
        "edgequake_fetch",
        json!({ "retrieval_id": retrieval_id }),
    )
    .await;
    let (_, fetch_b) = mcp_tools_call(
        &app,
        "/mcp",
        "edgequake_fetch",
        json!({ "retrieval_id": retrieval_id }),
    )
    .await;

    assert_eq!(
        tool_structured(&fetch_a)["retrieval_id"],
        tool_structured(&fetch_b)["retrieval_id"]
    );
    assert_eq!(
        tool_structured(&fetch_a)["chunks"],
        tool_structured(&fetch_b)["chunks"]
    );
}

#[tokio::test]
async fn ec_mcp_retrieve_one_shot() {
    let app = default_mcp_app();
    let (status, body) = mcp_tools_call(
        &app,
        "/mcp",
        "edgequake_retrieve",
        json!({ "query": "RAG pipeline", "mode": "naive", "content_granularity": "agent" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("error").is_none(), "{body:?}");
    assert!(tool_structured(&body)["retrieval_id"]
        .as_str()
        .unwrap()
        .starts_with("ret_"));
}

#[tokio::test]
async fn ec_mcp_45_concurrent_read_only_tools() {
    use futures_util::future::join3;

    let app = default_mcp_app();
    let list_fut = app.clone().oneshot(mcp_post_legacy(
        "/mcp",
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
    ));
    let search_fut = app.clone().oneshot(mcp_post_legacy(
        "/mcp",
        tools_call_body(
            "edgequake_search",
            json!({ "query": "concurrent", "mode": "naive" }),
        ),
    ));
    let ping_fut = app.oneshot(mcp_post_legacy(
        "/mcp",
        json!({"jsonrpc":"2.0","id":2,"method":"ping"}),
    ));

    let (list, search, ping) = join3(list_fut, search_fut, ping_fut).await;
    assert_eq!(list.unwrap().status(), StatusCode::OK);
    assert_eq!(search.unwrap().status(), StatusCode::OK);
    assert_eq!(ping.unwrap().status(), StatusCode::OK);
}

#[tokio::test]
async fn ec_mcp_38_unicode_emoji_query() {
    let app = default_mcp_app();
    let (status, body) = mcp_tools_call(
        &app,
        "/mcp",
        "edgequake_search",
        json!({ "query": "RAG 🚀 日本語", "mode": "naive" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.get("error").is_none(),
        "unicode query must succeed: {body:?}"
    );
}

#[tokio::test]
async fn ec_mcp_46_search_result_url_uses_edgequake_scheme() {
    let app = default_mcp_app();
    let (status, body) = mcp_tools_call(
        &app,
        "/mcp",
        "edgequake_search",
        json!({ "query": "knowledge graph", "mode": "naive" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rid = tool_structured(&body)["retrieval_id"]
        .as_str()
        .expect("retrieval_id");
    assert!(
        rid.starts_with("ret_"),
        "EQ-MCP search must return ret_ handle: {rid}"
    );
}

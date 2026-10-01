//! SPEC-155 W3 B09 — chat `done` includes subgraph; entities carry `graph_node_id`
//! and `source_document_ids` arrays.

use edgequake_api::handlers::context_types::{ContentGranularity, SubgraphBundle};
use edgequake_api::handlers::chat_types::ChatStreamEvent;
use edgequake_api::services::context_bundle_mapper::{map_query_context_to_subgraph, MappingOptions};
use edgequake_query::{QueryContext, RetrievedEntity};
use serde_json::Value;
use uuid::Uuid;

#[tokio::test]
async fn e2e_spec155_chat_done_subgraph_ids() {
    let mut context = QueryContext::default();
    context.entities.push(
        RetrievedEntity::new("SARAH_CHEN", "PERSON", "researcher")
            .with_source_document_ids(vec!["doc-a".into(), "doc-b".into()])
            .with_source_document_id("doc-a"),
    );

    let subgraph = map_query_context_to_subgraph(
        &context,
        &MappingOptions {
            granularity: ContentGranularity::Citation,
            include_lineage: true,
            include_documents: false,
            include_agent_hints: false,
            include_subgraph: true,
            rerank_top_k: None,
            reranked: false,
        },
    );

    assert_eq!(subgraph.entities.len(), 1);
    let ent = &subgraph.entities[0];
    assert_eq!(ent.graph_node_id, "SARAH_CHEN");
    assert_eq!(ent.source_document_ids, vec!["doc-a", "doc-b"]);
    let lineage = ent.lineage.as_ref().expect("lineage");
    assert_eq!(lineage.source_document_ids, vec!["doc-a", "doc-b"]);

    let done = ChatStreamEvent::Done {
        assistant_message_id: Uuid::nil(),
        tokens_used: 10,
        duration_ms: 5,
        llm_provider: Some("mock".into()),
        llm_model: Some("mock-model".into()),
        answer: Some("ok".into()),
        subgraph: Some(subgraph),
    };

    let json: Value = serde_json::to_value(&done).expect("serialize");
    assert_eq!(json["type"], "done");
    assert!(json.get("subgraph").is_some());
    assert_eq!(
        json["subgraph"]["entities"][0]["graph_node_id"],
        "SARAH_CHEN"
    );
    assert_eq!(
        json["subgraph"]["entities"][0]["source_document_ids"]
            .as_array()
            .map(|a| a.len()),
        Some(2)
    );

    // Empty subgraph still serializes cleanly (optional field present when Some).
    let empty = ChatStreamEvent::Done {
        assistant_message_id: Uuid::nil(),
        tokens_used: 1,
        duration_ms: 1,
        llm_provider: None,
        llm_model: None,
        answer: None,
        subgraph: Some(SubgraphBundle::default()),
    };
    let empty_json = serde_json::to_value(&empty).unwrap();
    assert!(empty_json["subgraph"]["entities"].as_array().unwrap().is_empty());
}

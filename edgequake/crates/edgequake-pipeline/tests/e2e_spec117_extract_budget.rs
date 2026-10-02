//! SPEC-117 — Truncation + gleaning continue e2e (mock LLM).
//!
//! Run:
//!   cargo test -p edgequake-pipeline --test e2e_spec117_extract_budget -- --nocapture

use std::collections::HashSet;
use std::sync::Arc;

use edgequake_llm::MockProvider;
use edgequake_pipeline::extractor::{
    EntityExtractor, ExtractedEntity, ExtractedRelationship, ExtractionResult, GleaningConfig,
    GleaningExtractor, LLMExtractor,
};
use edgequake_pipeline::prompts::{
    apply_extraction_caps_with_strategy, json_extraction_prompt_with_caps,
    json_gleaning_prompt_with_caps, CapsSelectionStrategy, EntityExtractionPrompts,
    EntityExtractionSchema, ExtractionCaps, HybridExtractionParser,
};
use edgequake_pipeline::TextChunk;

fn over_budget_json(n: usize, prefix: &str) -> String {
    let ents: Vec<String> = (0..n)
        .map(|i| format!(r#"{{"name":"{prefix}{i}","type":"CONCEPT","description":"d{i}"}}"#))
        .collect();
    format!(r#"{{"entities":[{}],"relationships":[]}}"#, ents.join(","))
}

#[test]
fn spec117_prompt_continue_section_when_truncated() {
    let caps = ExtractionCaps {
        max_entities: 10,
        max_total_records: 20,
    };
    let schema = EntityExtractionSchema::server_default();
    let prompt = json_gleaning_prompt_with_caps(
        "text",
        &["A".into(), "B".into()],
        &schema,
        "English",
        caps,
        true,
    );
    assert!(prompt.contains("Continue After Budget Truncation"));
    assert!(prompt.contains("ADDITIONAL"));
    assert!(prompt.contains("10"));
}

#[test]
fn spec117_hard_truncate_sets_metadata() {
    let caps = ExtractionCaps {
        max_entities: 10,
        max_total_records: 20,
    };
    let mut result = ExtractionResult::new("c1");
    for i in 0..45 {
        result.add_entity(ExtractedEntity::new(format!("E{i}"), "CONCEPT", "d"));
    }
    apply_extraction_caps_with_strategy(&mut result, caps, CapsSelectionStrategy::Fifo);
    assert_eq!(result.entities.len(), 10);
    assert!(result.metadata.contains_key("extract_caps_applied"));
    assert_eq!(result.metadata["extract_caps_applied"]["selection"], "fifo");
}

#[test]
fn spec117_relation_aware_keeps_tail_bridges() {
    let caps = ExtractionCaps {
        max_entities: 5,
        max_total_records: 20,
    };
    let mut result = ExtractionResult::new("c1");
    for i in 0..20 {
        result.add_entity(ExtractedEntity::new(format!("E{i}"), "CONCEPT", "d"));
    }
    result.add_relationship(
        ExtractedRelationship::new("E17", "E18", "RELATED").with_description("bridge"),
    );
    result.add_relationship(
        ExtractedRelationship::new("E18", "E19", "RELATED")
            .with_description("bridge")
            .with_weight(0.95),
    );

    apply_extraction_caps_with_strategy(&mut result, caps, CapsSelectionStrategy::RelationAware);

    assert_eq!(result.entities.len(), 5);
    let names: HashSet<_> = result.entities.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains("E17") && names.contains("E18") && names.contains("E19"));
    assert_eq!(result.relationships.len(), 2);
    assert_eq!(
        result.metadata["extract_caps_applied"]["selection"],
        "relation_aware"
    );
}

#[test]
fn spec117_json_and_sota_prompts_embed_resolved_caps() {
    let caps = ExtractionCaps {
        max_entities: 11,
        max_total_records: 33,
    };
    let schema = EntityExtractionSchema::server_default();
    let json = json_extraction_prompt_with_caps("t", &schema, "English", caps);
    assert!(json.contains("11") && json.contains("33"), "{json}");
    let sota = EntityExtractionPrompts::default().system_prompt_with_caps(&schema, "English", caps);
    assert!(sota.contains("11") && sota.contains("33"), "{sota}");
}

#[test]
fn spec117_hybrid_parser_hard_truncates_json_path() {
    let caps = ExtractionCaps {
        max_entities: 5,
        max_total_records: 10,
    };
    let parser = HybridExtractionParser::new(false).with_extraction_caps(caps);
    let ents: Vec<String> = (0..20)
        .map(|i| format!(r#"{{"name":"E{i}","type":"CONCEPT","description":"d"}}"#))
        .collect();
    let response = format!(r#"{{"entities":[{}],"relationships":[]}}"#, ents.join(","));
    let result = parser.parse(&response, "c1").expect("parse");
    assert_eq!(result.entities.len(), 5);
    assert!(result.metadata.contains_key("extract_caps_applied"));
}

#[tokio::test]
async fn spec117_gleaning_continue_after_truncate() {
    let caps = ExtractionCaps {
        max_entities: 10,
        max_total_records: 20,
    };
    let mock = Arc::new(MockProvider::new());
    mock.add_response(&over_budget_json(45, "Entity")).await;
    mock.add_response(&over_budget_json(3, "Extra")).await;

    let base = Arc::new(
        LLMExtractor::new(mock.clone() as Arc<dyn edgequake_llm::LLMProvider>)
            .with_extraction_caps(caps),
    );
    let glean = GleaningExtractor::new(mock.clone() as Arc<dyn edgequake_llm::LLMProvider>, base)
        .with_extraction_caps(caps)
        .with_config(GleaningConfig { max_gleaning: 1 });

    let chunk = TextChunk::new(
        "c1",
        "Alpha works with Beta on Gamma. ".repeat(20),
        0,
        0,
        100,
    );
    let result = glean.extract(&chunk).await.expect("extract");
    assert!(
        result.entities.len() >= 10,
        "expected truncated base + optional extras, got {}",
        result.entities.len()
    );
    let names: Vec<_> = result
        .entities
        .iter()
        .map(|e| e.name.to_uppercase())
        .collect();
    assert!(
        result.metadata.contains_key("extract_caps_applied")
            || names.iter().any(|n| n.starts_with("EXTRA")),
        "expected caps applied or gleaned extras; names={names:?} meta={:?}",
        result.metadata
    );
}

#[test]
fn spec117_parser_relation_aware_preserves_bridges_end_to_end() {
    let caps = ExtractionCaps {
        max_entities: 4,
        max_total_records: 20,
    };
    // Orphans first, bridges last — FIFO drops bridges; relation-aware keeps them.
    let mut result = ExtractionResult::new("c1");
    for i in 0..12 {
        result.add_entity(ExtractedEntity::new(format!("N{i}"), "CONCEPT", "d"));
    }
    result.add_relationship(
        ExtractedRelationship::new("N10", "N11", "RELATED")
            .with_description("bridge")
            .with_weight(0.9),
    );

    let mut fifo = result.clone();
    apply_extraction_caps_with_strategy(&mut fifo, caps, CapsSelectionStrategy::Fifo);
    assert!(
        fifo.relationships.is_empty(),
        "FIFO must drop tail bridges under K"
    );

    apply_extraction_caps_with_strategy(&mut result, caps, CapsSelectionStrategy::RelationAware);
    let names: HashSet<_> = result.entities.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains("N10") && names.contains("N11"));
    assert_eq!(result.relationships.len(), 1);
    assert_eq!(
        result.metadata["extract_caps_applied"]["selection"],
        "relation_aware"
    );

    // Parser path uses env strategy (default relation-aware).
    let ents: Vec<String> = (0..12)
        .map(|i| format!(r#"{{"name":"N{i}","type":"CONCEPT","description":"d"}}"#))
        .collect();
    let response = format!(
        r#"{{"entities":[{}],"relationships":[
          {{"source":"N10","target":"N11","type":"RELATED","description":"bridge","weight":0.9}}
        ]}}"#,
        ents.join(",")
    );
    let parser = HybridExtractionParser::new(false).with_extraction_caps(caps);
    let parsed = parser.parse(&response, "c2").expect("parse");
    if CapsSelectionStrategy::from_env() == CapsSelectionStrategy::RelationAware {
        let names: HashSet<_> = parsed.entities.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains("N10") && names.contains("N11"));
        assert_eq!(parsed.relationships.len(), 1);
    }
}

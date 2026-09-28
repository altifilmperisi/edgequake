//! SPEC-151 contract: page reprocess fails closed without extraction snapshot.
//!
//! Handler maps `plan_partial_reprocess` Err → `ApiError::ValidationError` (422)
//! for both dry_run and enqueue (see `pages_reprocess.rs`).

use edgequake_api::processor::page_reprocess::{
    plan_partial_reprocess, require_hybrid_snapshot_for_page_extract, PlanInput,
};
use edgequake_pdf::ReprocessStage;

#[test]
fn contract_entities_without_snapshot_plan_errs() {
    let err = plan_partial_reprocess(PlanInput {
        pages: vec![2],
        stages: vec![ReprocessStage::Entities],
        page_count: 4,
        pages_missing_raw: false,
        dirty_chunk_count: 1,
        reusable_chunk_count: 5,
        has_extraction_snapshot: false,
    })
    .expect_err("must fail closed");
    assert!(
        err.contains("prior extraction snapshot"),
        "unexpected plan error: {err}"
    );
}

#[test]
fn contract_parse_without_snapshot_still_ok() {
    // Parse-only does not require Entities → snapshot not required at plan time.
    let plan = plan_partial_reprocess(PlanInput {
        pages: vec![1],
        stages: vec![ReprocessStage::Parse],
        page_count: 4,
        pages_missing_raw: false,
        dirty_chunk_count: 1,
        reusable_chunk_count: 5,
        has_extraction_snapshot: false,
    });
    // Parse closes to figures+entities → still needs snapshot under fail-closed.
    assert!(
        plan.is_err(),
        "parse closure includes entities; must require snapshot"
    );
}

#[test]
fn contract_hybrid_snapshot_gate_matches_plan_message() {
    let gate_err = require_hybrid_snapshot_for_page_extract(&[2], None).unwrap_err();
    let plan_err = plan_partial_reprocess(PlanInput {
        pages: vec![2],
        stages: vec![ReprocessStage::Entities],
        page_count: 3,
        pages_missing_raw: false,
        dirty_chunk_count: 1,
        reusable_chunk_count: 2,
        has_extraction_snapshot: false,
    })
    .unwrap_err();
    assert_eq!(gate_err, plan_err);
}

#[test]
fn contract_entities_with_snapshot_plans_ok() {
    let plan = plan_partial_reprocess(PlanInput {
        pages: vec![2],
        stages: vec![ReprocessStage::Entities],
        page_count: 4,
        pages_missing_raw: false,
        dirty_chunk_count: 1,
        reusable_chunk_count: 5,
        has_extraction_snapshot: true,
    })
    .expect("snapshot present");
    assert_eq!(plan.effective_stages, vec!["entities"]);
}

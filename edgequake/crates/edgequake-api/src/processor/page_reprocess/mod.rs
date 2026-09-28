//! SPEC-151 — Partial page reprocess executor modules.

pub mod execute;
pub mod plan;
pub mod record;

pub use plan::{
    page_reprocess_actions, page_scope_from_plan, plan_partial_reprocess,
    require_hybrid_snapshot_for_page_extract, PageReprocessActions, PartialReprocessPlan,
    PlanInput,
};
pub use record::{
    make_parse_page_sink, record_entities_from_chunks, record_figures_from_markdown,
    record_figures_from_markdown_scoped, record_parse_from_markdown, salvage_checkpoint_pages,
};

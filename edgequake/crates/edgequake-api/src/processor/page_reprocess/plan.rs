//! SPEC-151 — Pure partial-reprocess planner (no I/O).

use edgequake_pdf::{
    effective_stages, ensure_parse_when_figures_need_raw, normalize_pages, PageScope,
    ReprocessStage,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Dry-run / enqueue plan returned to clients.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct PartialReprocessPlan {
    pub pages: Vec<u32>,
    pub requested_stages: Vec<String>,
    pub effective_stages: Vec<String>,
    pub dirty_chunk_count: usize,
    pub reusable_chunk_count: usize,
    pub estimated_vision_calls: usize,
    pub warnings: Vec<String>,
    pub suggest_full_reprocess: bool,
}

/// Inputs gathered by the HTTP handler before planning.
#[derive(Debug, Clone)]
pub struct PlanInput {
    pub pages: Vec<u32>,
    pub stages: Vec<ReprocessStage>,
    pub page_count: u32,
    /// True when any selected page lacks stored raw_markdown.
    pub pages_missing_raw: bool,
    pub dirty_chunk_count: usize,
    pub reusable_chunk_count: usize,
    pub has_extraction_snapshot: bool,
}

/// Build an immutable plan (LAW-151-3 closure + estimates).
pub fn plan_partial_reprocess(input: PlanInput) -> Result<PartialReprocessPlan, String> {
    let pages = normalize_pages(&input.pages, Some(input.page_count)).map_err(|e| e.to_string())?;
    if input.stages.is_empty() {
        return Err("at least one reprocess stage is required".into());
    }
    let mut effective = ensure_parse_when_figures_need_raw(&input.stages, input.pages_missing_raw);
    // ensure_parse may insert Parse; also apply normal closure
    let closed = effective_stages(&input.stages);
    for s in closed {
        if !effective.contains(&s) {
            effective.push(s);
        }
    }
    effective.sort_by_key(|s| s.order());
    effective.dedup();

    // Fail closed early (dry_run + enqueue): Entities needs a prior snapshot so
    // clean pages can be reused. Do not 202 then fail late on Insert.
    if !input.has_extraction_snapshot && effective.contains(&ReprocessStage::Entities) {
        return Err(
            "Partial page reprocess requires a prior extraction snapshot \
             to reuse clean pages. Run a full document reprocess once, \
             then retry selected-page extract."
                .into(),
        );
    }

    let mut warnings = Vec::new();
    if input.pages_missing_raw && effective.contains(&ReprocessStage::Figures) {
        warnings.push("Some pages have no stored OCR; Parse was added automatically.".into());
    }
    let suggest_full = pages.len() as u32 == input.page_count && input.page_count > 1;
    if suggest_full {
        warnings
            .push("You selected every page — consider a full document reprocess instead.".into());
    }

    // Vision OCR calls only for Parse. Figures-without-Parse uses asset writers
    // (build_page_asset_bundle), not the vision convert path.
    let estimated_vision_calls = if effective.contains(&ReprocessStage::Parse) {
        pages.len()
    } else {
        0
    };

    Ok(PartialReprocessPlan {
        pages: pages.clone(),
        requested_stages: input
            .stages
            .iter()
            .map(|s| s.as_str().to_string())
            .collect(),
        effective_stages: effective.iter().map(|s| s.as_str().to_string()).collect(),
        dirty_chunk_count: input.dirty_chunk_count,
        reusable_chunk_count: input.reusable_chunk_count,
        estimated_vision_calls,
        warnings,
        suggest_full_reprocess: suggest_full,
    })
}

/// Build a [`PageScope`] from a validated plan.
pub fn page_scope_from_plan(plan: &PartialReprocessPlan) -> Result<PageScope, String> {
    let stages: Result<Vec<_>, _> = plan
        .effective_stages
        .iter()
        .map(|s| ReprocessStage::parse(s))
        .collect();
    let stages = stages?;
    PageScope::new(plan.pages.clone(), stages).map_err(|e| e.to_string())
}

/// Pure executor stage gates (testable without PDF I/O). LAW-151-3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageReprocessActions {
    /// Mark parse RUNNING and run Vision OCR for selected pages.
    pub run_parse_ocr: bool,
    /// Figures-only (no Parse): build assets without re-OCR.
    pub run_figures_assets_only: bool,
    /// After OCR or figures-only, record figures health from markdown/assets.
    pub record_figures_health: bool,
    /// Enqueue Insert with hybrid reuse for dirty pages.
    pub enqueue_entities: bool,
}

/// Derive I/O actions from the effective stage closure.
pub fn page_reprocess_actions(stages: &[ReprocessStage]) -> PageReprocessActions {
    let has_parse = stages.contains(&ReprocessStage::Parse);
    let has_figures = stages.contains(&ReprocessStage::Figures);
    let has_entities = stages.contains(&ReprocessStage::Entities);
    PageReprocessActions {
        run_parse_ocr: has_parse,
        run_figures_assets_only: has_figures && !has_parse,
        record_figures_health: has_figures || has_parse,
        enqueue_entities: has_entities,
    }
}

/// Fail-closed gate: page-scope extract needs reusable prior extractions.
///
/// Without a durable snapshot, hybrid reuse cannot preserve clean pages —
/// extracting the whole document would violate the selected-page principle,
/// and retracting without a hybrid set would drop unselected entities.
pub fn require_hybrid_snapshot_for_page_extract(
    excluded_pages: &[u32],
    snapshot_extraction_count: Option<usize>,
) -> Result<(), String> {
    if excluded_pages.is_empty() {
        return Ok(());
    }
    match snapshot_extraction_count {
        Some(n) if n > 0 => Ok(()),
        _ => Err(
            "Partial page reprocess requires a prior extraction snapshot \
             to reuse clean pages. Run a full document reprocess once, \
             then retry selected-page extract."
                .into(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_closes_to_all_stages() {
        let plan = plan_partial_reprocess(PlanInput {
            pages: vec![1, 2],
            stages: vec![ReprocessStage::Parse],
            page_count: 10,
            pages_missing_raw: false,
            dirty_chunk_count: 4,
            reusable_chunk_count: 20,
            has_extraction_snapshot: true,
        })
        .unwrap();
        assert_eq!(plan.effective_stages, vec!["parse", "figures", "entities"]);
        assert_eq!(plan.estimated_vision_calls, 2);
    }

    #[test]
    fn figures_missing_raw_adds_parse() {
        let plan = plan_partial_reprocess(PlanInput {
            pages: vec![3],
            stages: vec![ReprocessStage::Figures],
            page_count: 5,
            pages_missing_raw: true,
            dirty_chunk_count: 1,
            reusable_chunk_count: 10,
            has_extraction_snapshot: true,
        })
        .unwrap();
        assert!(plan.effective_stages.contains(&"parse".to_string()));
    }

    #[test]
    fn figures_only_dry_run_estimates_zero_vision_ocr_calls() {
        let plan = plan_partial_reprocess(PlanInput {
            pages: vec![2, 3],
            stages: vec![ReprocessStage::Figures],
            page_count: 5,
            pages_missing_raw: false,
            dirty_chunk_count: 2,
            reusable_chunk_count: 8,
            has_extraction_snapshot: true,
        })
        .unwrap();
        assert_eq!(plan.effective_stages, vec!["figures", "entities"]);
        assert_eq!(
            plan.estimated_vision_calls, 0,
            "figures-without-parse must not count Vision OCR calls"
        );
    }

    #[test]
    fn entities_only_skips_parse_running_and_ocr() {
        let a = page_reprocess_actions(&[ReprocessStage::Entities]);
        assert!(!a.run_parse_ocr);
        assert!(!a.run_figures_assets_only);
        assert!(!a.record_figures_health);
        assert!(a.enqueue_entities);
    }

    #[test]
    fn figures_without_parse_uses_asset_bundle_not_ocr() {
        let a = page_reprocess_actions(&[ReprocessStage::Figures, ReprocessStage::Entities]);
        assert!(!a.run_parse_ocr);
        assert!(a.run_figures_assets_only);
        assert!(a.record_figures_health);
        assert!(a.enqueue_entities);
    }

    #[test]
    fn parse_closure_runs_ocr_not_figures_only_bundle() {
        let a = page_reprocess_actions(&[
            ReprocessStage::Parse,
            ReprocessStage::Figures,
            ReprocessStage::Entities,
        ]);
        assert!(a.run_parse_ocr);
        assert!(!a.run_figures_assets_only);
        assert!(a.record_figures_health);
        assert!(a.enqueue_entities);
    }

    #[test]
    fn hybrid_page_extract_fails_closed_without_snapshot() {
        assert!(require_hybrid_snapshot_for_page_extract(&[2], None).is_err());
        assert!(require_hybrid_snapshot_for_page_extract(&[2], Some(0)).is_err());
        assert!(require_hybrid_snapshot_for_page_extract(&[2], Some(3)).is_ok());
        assert!(require_hybrid_snapshot_for_page_extract(&[], None).is_ok());
    }

    #[test]
    fn plan_entities_without_snapshot_is_err() {
        let err = plan_partial_reprocess(PlanInput {
            pages: vec![2],
            stages: vec![ReprocessStage::Entities],
            page_count: 5,
            pages_missing_raw: false,
            dirty_chunk_count: 1,
            reusable_chunk_count: 4,
            has_extraction_snapshot: false,
        })
        .unwrap_err();
        assert!(
            err.contains("prior extraction snapshot"),
            "unexpected: {err}"
        );
    }
}

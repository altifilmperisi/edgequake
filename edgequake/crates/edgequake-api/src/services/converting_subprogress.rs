//! Post-OCR converting sub-step progress (SPEC-048 / Pass B VLM analyze).
//!
//! SSOT for human-readable `stage_message` strings and `stage_progress` fractions
//! during figure/chart Vision LLM analysis. Keeps pdf_processing, multimodal
//! analyzer, and progress_facade parsing aligned (DRY).

use std::sync::Arc;

/// Structured converting sub-step update (SPEC-155: typed counters, not message regex).
#[derive(Debug, Clone)]
pub struct ConvertingSubstepUpdate {
    pub message: String,
    /// Legacy absolute/band progress hint (0–1).
    pub progress: f64,
    /// When set: figure analyze counters `(done, total)`.
    pub figures: Option<(usize, usize)>,
}

impl ConvertingSubstepUpdate {
    pub fn message_only(message: impl Into<String>, progress: f64) -> Self {
        Self {
            message: message.into(),
            progress,
            figures: None,
        }
    }

    pub fn with_figures(
        message: impl Into<String>,
        progress: f64,
        done: usize,
        total: usize,
    ) -> Self {
        Self {
            message: message.into(),
            progress,
            figures: Some((done, total)),
        }
    }
}

/// Callback invoked when a converting sub-step advances (typically KV + run_progress).
pub type ConvertingSubstepReporter = Arc<dyn Fn(ConvertingSubstepUpdate) + Send + Sync>;

/// Options for Pass B figure progress copy + emission cadence.
#[derive(Debug, Clone, Copy, Default)]
pub struct VisionFigureProgressOpts {
    /// Emit on every completed figure (local / modest totals).
    pub every_figure: bool,
    /// Local classify-only mode message.
    pub local_classify_only: bool,
    /// Total figures discovered in the document (may exceed analyzed_cap).
    pub discovered_total: usize,
    /// Figures scheduled for analysis (after local cap).
    pub analyzed_cap: usize,
}

/// User-visible message while Vision LLM analyzes extracted figure/chart images.
pub fn vision_figure_analyze_message(completed: usize, total: usize) -> String {
    if total == 0 {
        return "Analyzing figures with Vision LLM…".to_string();
    }
    format!("Analyzing figures with Vision LLM — figure {completed}/{total}")
}

/// Local never-stuck copy: classify-only + cap vs discovered.
pub fn vision_figure_analyze_message_local(
    completed: usize,
    analyzed_cap: usize,
    discovered: usize,
) -> String {
    if analyzed_cap == 0 {
        return "Analyzing figures (local, classify-only)…".to_string();
    }
    if discovered > analyzed_cap {
        format!(
            "Analyzing figures (local, classify-only) — {completed}/{analyzed_cap} of {discovered}"
        )
    } else {
        format!("Analyzing figures (local, classify-only) — {completed}/{analyzed_cap}")
    }
}

/// Map figure analyze completion into the converting stage band (0.98–0.995).
pub fn vision_figure_analyze_progress_01(completed: usize, total: usize) -> f64 {
    const BASE: f64 = 0.98;
    const SPAN: f64 = 0.015;
    if total == 0 {
        return BASE;
    }
    (BASE + SPAN * (completed as f64 / total as f64)).min(0.995)
}

/// Throttle KV writes: first, every 3rd, and last — or every step when total ≤ 5.
/// When `every_figure` / total ≤ 50: emit every completion (honest local UI).
pub fn should_emit_substep_milestone(completed: usize, total: usize) -> bool {
    should_emit_substep_milestone_ex(completed, total, false)
}

/// Milestone policy with explicit every-figure override.
pub fn should_emit_substep_milestone_ex(
    completed: usize,
    total: usize,
    every_figure: bool,
) -> bool {
    if total == 0 {
        return completed == 1;
    }
    if every_figure || total <= 50 {
        return true;
    }
    completed == 1 || completed == total || completed.is_multiple_of(3)
}

/// Emit a converting sub-step if a reporter is configured.
pub fn report_vision_figure_analyze(
    reporter: Option<&ConvertingSubstepReporter>,
    completed: usize,
    total: usize,
) {
    report_vision_figure_analyze_ex(
        reporter,
        completed,
        total,
        VisionFigureProgressOpts {
            every_figure: total <= 50,
            local_classify_only: false,
            discovered_total: total,
            analyzed_cap: total,
        },
    );
}

/// Emit Pass B progress with local/cloud-aware copy.
pub fn report_vision_figure_analyze_ex(
    reporter: Option<&ConvertingSubstepReporter>,
    completed: usize,
    total: usize,
    opts: VisionFigureProgressOpts,
) {
    if let Some(hook) = reporter {
        if should_emit_substep_milestone_ex(completed, total, opts.every_figure) {
            let message = if opts.local_classify_only {
                vision_figure_analyze_message_local(
                    completed,
                    opts.analyzed_cap.max(total),
                    opts.discovered_total.max(total),
                )
            } else {
                vision_figure_analyze_message(completed, total)
            };
            let progress = vision_figure_analyze_progress_01(completed, total);
            hook(ConvertingSubstepUpdate::with_figures(
                message, progress, completed, total,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn vision_figure_message_includes_counts() {
        assert_eq!(
            vision_figure_analyze_message(3, 12),
            "Analyzing figures with Vision LLM — figure 3/12"
        );
    }

    #[test]
    fn local_message_includes_cap_and_discovered() {
        assert_eq!(
            vision_figure_analyze_message_local(4, 12, 46),
            "Analyzing figures (local, classify-only) — 4/12 of 46"
        );
    }

    #[test]
    fn vision_figure_progress_monotonic_in_band() {
        assert!(vision_figure_analyze_progress_01(0, 10) >= 0.98);
        assert!(vision_figure_analyze_progress_01(10, 10) <= 0.995);
        assert!(
            vision_figure_analyze_progress_01(5, 10) > vision_figure_analyze_progress_01(2, 10)
        );
    }

    #[test]
    fn milestone_every_figure_when_total_le_50() {
        assert!(should_emit_substep_milestone(2, 46));
        assert!(should_emit_substep_milestone_ex(2, 100, true));
        assert!(!should_emit_substep_milestone_ex(2, 100, false));
        assert!(should_emit_substep_milestone_ex(3, 100, false));
    }

    #[test]
    fn reporter_called_on_milestones() {
        let calls = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&calls);
        let reporter: ConvertingSubstepReporter = Arc::new(move |update| {
            c.fetch_add(1, Ordering::Relaxed);
            assert!(update.message.contains("figure 3/9"));
            assert!(update.progress > 0.98);
            assert_eq!(update.figures, Some((3, 9)));
        });
        report_vision_figure_analyze(Some(&reporter), 3, 9);
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }
}

//! Contract: local VLM Pass B never-stuck profile (caps, classify-only, cancel, budget).

use std::sync::{Arc, Mutex};

use edgequake_api::services::{
    analyze_multimodal_images, analyze_multimodal_images_with_substep, LocalMmProfile,
    MultimodalProviders,
};
use edgequake_llm::MockProvider;
use serial_test::serial;
use tokio_util::sync::CancellationToken;

mod common;
use common::spec026_multimodal::{write_figure_png_asset, TINY_PNG};

fn drawing_tag(id: &str, path: &str) -> String {
    format!(r#"<drawing id="{id}" format="png" path="{path}" caption="Figure" />"#)
}

#[test]
#[serial]
fn contract_local_profile_caps_and_classify_only() {
    std::env::set_var("EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY", "1");
    std::env::set_var("EDGEQUAKE_MM_MAX_FIGURES", "12");
    std::env::set_var("EDGEQUAKE_MM_PASS_B_TIMEOUT_SECS", "600");
    let p = LocalMmProfile::resolve("ollama");
    assert!(p.is_local);
    assert!(p.classify_only);
    assert_eq!(p.max_figures, Some(12));
    assert_eq!(p.figures_to_analyze(46), 12);
    assert!(p.pass_b_timeout.is_some());
    std::env::remove_var("EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY");
    std::env::remove_var("EDGEQUAKE_MM_MAX_FIGURES");
    std::env::remove_var("EDGEQUAKE_MM_PASS_B_TIMEOUT_SECS");
}

#[test]
#[serial]
fn contract_cloud_profile_keeps_specialize() {
    std::env::remove_var("EDGEQUAKE_MM_MAX_FIGURES");
    std::env::remove_var("EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY");
    let p = LocalMmProfile::resolve("openai");
    assert!(!p.is_local);
    assert!(!p.classify_only);
    assert_eq!(p.max_figures, None);
}

#[tokio::test]
#[serial]
async fn contract_local_max_figures_skips_remainder() {
    std::env::set_var("VLM_PROCESS_ENABLE", "true");
    std::env::set_var("VLM_MIN_IMAGE_PIXEL", "1");
    std::env::set_var("EDGEQUAKE_VISION_PROVIDER", "ollama");
    std::env::set_var("EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY", "1");
    std::env::set_var("EDGEQUAKE_MM_MAX_FIGURES", "2");
    std::env::set_var("EDGEQUAKE_MULTIMODAL_FAIL_MODE", "degraded");

    let dir = tempfile::tempdir().unwrap();
    write_figure_png_asset(dir.path(), 1);
    write_figure_png_asset(dir.path(), 2);
    write_figure_png_asset(dir.path(), 3);
    // Reuse page-0001-fig-01 path for all tags by writing three unique files.
    for i in 1..=3 {
        let rel = format!("assets/fig-{i:02}.png");
        let full = dir.path().join(&rel);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(&full, TINY_PNG).unwrap();
    }

    let md = format!(
        "Intro\n{}\n{}\n{}\nBody",
        drawing_tag("im-1", "assets/fig-01.png"),
        drawing_tag("im-2", "assets/fig-02.png"),
        drawing_tag("im-3", "assets/fig-03.png"),
    );

    let mock = MockProvider::new();
    mock.add_response(r#"{"name":"a","type":"Photo","description":"A."}"#)
        .await;
    mock.add_response(r#"{"name":"b","type":"Photo","description":"B."}"#)
        .await;
    // Third figure must not be analyzed (cap=2) — no third response needed.

    let out = analyze_multimodal_images(
        &md,
        Some("i"),
        "doc.pdf",
        MultimodalProviders::single(&mock),
        Some(dir.path()),
        None,
    )
    .await;

    assert!(out.hard_error.is_none());
    assert!(
        out.markdown.contains("skipped 1 figures (local budget)"),
        "expected skip notice, got: {}",
        out.markdown
    );
    // Cap=2 ⇒ at most two replacements; third drawing tag may remain.
    let remaining = out.markdown.matches("<drawing").count();
    assert!(
        remaining >= 1,
        "expected at least one unanalyzed drawing placeholder, markdown={}",
        out.markdown
    );

    std::env::remove_var("VLM_MIN_IMAGE_PIXEL");
    std::env::remove_var("VLM_PROCESS_ENABLE");
    std::env::remove_var("EDGEQUAKE_VISION_PROVIDER");
    std::env::remove_var("EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY");
    std::env::remove_var("EDGEQUAKE_MM_MAX_FIGURES");
    std::env::remove_var("EDGEQUAKE_MULTIMODAL_FAIL_MODE");
}

#[tokio::test]
#[serial]
async fn contract_pass_b_cancel_between_figures() {
    std::env::set_var("VLM_PROCESS_ENABLE", "true");
    std::env::set_var("VLM_MIN_IMAGE_PIXEL", "1");
    std::env::set_var("EDGEQUAKE_VISION_PROVIDER", "ollama");
    std::env::set_var("EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY", "1");
    std::env::set_var("EDGEQUAKE_MM_MAX_FIGURES", "12");
    std::env::set_var("EDGEQUAKE_MULTIMODAL_FAIL_MODE", "degraded");

    let dir = tempfile::tempdir().unwrap();
    for i in 1..=3 {
        let rel = format!("assets/fig-{i:02}.png");
        let full = dir.path().join(&rel);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(&full, TINY_PNG).unwrap();
    }
    let md = format!(
        "Intro\n{}\n{}\n{}\nBody",
        drawing_tag("im-1", "assets/fig-01.png"),
        drawing_tag("im-2", "assets/fig-02.png"),
        drawing_tag("im-3", "assets/fig-03.png"),
    );

    let mock = MockProvider::new();
    mock.add_response(r#"{"name":"a","type":"Photo","description":"A."}"#)
        .await;
    mock.add_response(r#"{"name":"b","type":"Photo","description":"B."}"#)
        .await;
    mock.add_response(r#"{"name":"c","type":"Photo","description":"C."}"#)
        .await;

    let token = CancellationToken::new();
    token.cancel();

    let out = analyze_multimodal_images_with_substep(
        &md,
        Some("i"),
        "doc.pdf",
        MultimodalProviders::single(&mock),
        Some(dir.path()),
        None,
        None,
        Some(token),
    )
    .await;

    assert!(out.hard_error.is_none());
    // Cancelled before first figure ⇒ all drawings remain (or none replaced).
    assert!(
        out.markdown.contains("<drawing"),
        "cancel should leave placeholders; got {}",
        out.markdown
    );

    std::env::remove_var("VLM_MIN_IMAGE_PIXEL");
    std::env::remove_var("VLM_PROCESS_ENABLE");
    std::env::remove_var("EDGEQUAKE_VISION_PROVIDER");
    std::env::remove_var("EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY");
    std::env::remove_var("EDGEQUAKE_MM_MAX_FIGURES");
    std::env::remove_var("EDGEQUAKE_MULTIMODAL_FAIL_MODE");
}

#[tokio::test]
#[serial]
async fn contract_pass_b_budget_does_not_hard_fail() {
    std::env::set_var("VLM_PROCESS_ENABLE", "true");
    std::env::set_var("VLM_MIN_IMAGE_PIXEL", "1");
    std::env::set_var("EDGEQUAKE_VISION_PROVIDER", "ollama");
    std::env::set_var("EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY", "1");
    std::env::set_var("EDGEQUAKE_MM_MAX_FIGURES", "12");
    // Tiny wall budget — Instant check before each figure; first may still run
    // if mock is fast, but expiry must not produce hard_error.
    std::env::set_var("EDGEQUAKE_MM_PASS_B_TIMEOUT_SECS", "30");
    std::env::set_var("EDGEQUAKE_MULTIMODAL_FAIL_MODE", "degraded");

    let dir = tempfile::tempdir().unwrap();
    let rel = "assets/fig-01.png";
    let full = dir.path().join(rel);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(&full, TINY_PNG).unwrap();
    let md = format!("Intro\n{}\nBody", drawing_tag("im-1", rel));

    let mock = MockProvider::new();
    mock.add_response(r#"{"name":"a","type":"Photo","description":"A."}"#)
        .await;

    let out = analyze_multimodal_images(
        &md,
        Some("i"),
        "doc.pdf",
        MultimodalProviders::single(&mock),
        Some(dir.path()),
        None,
    )
    .await;

    assert!(out.hard_error.is_none(), "budget must not hard-fail");
    assert!(!out.markdown.is_empty());

    std::env::remove_var("VLM_MIN_IMAGE_PIXEL");
    std::env::remove_var("VLM_PROCESS_ENABLE");
    std::env::remove_var("EDGEQUAKE_VISION_PROVIDER");
    std::env::remove_var("EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY");
    std::env::remove_var("EDGEQUAKE_MM_MAX_FIGURES");
    std::env::remove_var("EDGEQUAKE_MM_PASS_B_TIMEOUT_SECS");
    std::env::remove_var("EDGEQUAKE_MULTIMODAL_FAIL_MODE");
}

#[tokio::test]
#[serial]
async fn contract_cloud_still_runs_specialize() {
    std::env::set_var("VLM_PROCESS_ENABLE", "true");
    std::env::set_var("VLM_MIN_IMAGE_PIXEL", "1");
    std::env::set_var("EDGEQUAKE_VISION_PROVIDER", "openai");
    std::env::remove_var("EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY");
    std::env::remove_var("EDGEQUAKE_MM_MAX_FIGURES");
    std::env::set_var("EDGEQUAKE_MULTIMODAL_FAIL_MODE", "degraded");

    let dir = tempfile::tempdir().unwrap();
    let rel = "assets/fig-01.png";
    let full = dir.path().join(rel);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(&full, TINY_PNG).unwrap();
    let md = format!(
        "Intro\n{}\nBody",
        drawing_tag("im-1", rel).replace("Figure", "Q4 Revenue chart")
    );

    let mock = MockProvider::new();
    // Classify as Chart → specialize consumes second response.
    mock.add_response(r#"{"name":"rev","type":"Chart","description":"generic"}"#)
        .await;
    mock.add_response(
        r#"{"name":"rev_q4","chart_kind":"bar","title":"Q4","x_axis":"Q","y_axis":"USD","key_values":[{"label":"Q4","value_raw":"42"}],"series":[],"data_table_md":"| Q | V |\n|---|---|\n| Q4 | 42 |","description":"Revenue."}"#,
    )
    .await;

    let out = analyze_multimodal_images(
        &md,
        Some("i"),
        "doc.pdf",
        MultimodalProviders::single(&mock),
        Some(dir.path()),
        None,
    )
    .await;

    assert!(out.hard_error.is_none());
    assert!(
        out.markdown.contains("42") || out.markdown.to_lowercase().contains("revenue"),
        "cloud specialize should land chart numbers; got {}",
        out.markdown
    );

    std::env::remove_var("VLM_MIN_IMAGE_PIXEL");
    std::env::remove_var("VLM_PROCESS_ENABLE");
    std::env::remove_var("EDGEQUAKE_VISION_PROVIDER");
    std::env::remove_var("EDGEQUAKE_MULTIMODAL_FAIL_MODE");
}

#[tokio::test]
#[serial]
async fn contract_local_progress_every_figure() {
    use edgequake_api::services::ConvertingSubstepReporter;

    std::env::set_var("VLM_PROCESS_ENABLE", "true");
    std::env::set_var("VLM_MIN_IMAGE_PIXEL", "1");
    std::env::set_var("EDGEQUAKE_VISION_PROVIDER", "ollama");
    std::env::set_var("EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY", "1");
    std::env::set_var("EDGEQUAKE_MM_MAX_FIGURES", "12");

    let dir = tempfile::tempdir().unwrap();
    for i in 1..=3 {
        let rel = format!("assets/fig-{i:02}.png");
        let full = dir.path().join(&rel);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(&full, TINY_PNG).unwrap();
    }
    let md = format!(
        "Intro\n{}\n{}\n{}\nBody",
        drawing_tag("im-1", "assets/fig-01.png"),
        drawing_tag("im-2", "assets/fig-02.png"),
        drawing_tag("im-3", "assets/fig-03.png"),
    );

    let mock = MockProvider::new();
    for _ in 0..3 {
        mock.add_response(r#"{"name":"a","type":"Photo","description":"A."}"#)
            .await;
    }

    let messages: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let msgs = Arc::clone(&messages);
    let reporter: ConvertingSubstepReporter = Arc::new(move |update| {
        msgs.lock().unwrap().push(update.message);
    });

    let _ = analyze_multimodal_images_with_substep(
        &md,
        Some("i"),
        "doc.pdf",
        MultimodalProviders::single(&mock),
        Some(dir.path()),
        None,
        Some(reporter),
        None,
    )
    .await;

    let logged = messages.lock().unwrap().clone();
    assert!(
        logged.iter().any(|m| m.contains("classify-only")),
        "expected local classify-only progress copy, got {logged:?}"
    );
    assert!(
        logged.iter().any(|m| m.contains("1/3")) && logged.iter().any(|m| m.contains("2/3")),
        "expected every-figure progress, got {logged:?}"
    );

    std::env::remove_var("VLM_MIN_IMAGE_PIXEL");
    std::env::remove_var("VLM_PROCESS_ENABLE");
    std::env::remove_var("EDGEQUAKE_VISION_PROVIDER");
    std::env::remove_var("EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY");
    std::env::remove_var("EDGEQUAKE_MM_MAX_FIGURES");
}

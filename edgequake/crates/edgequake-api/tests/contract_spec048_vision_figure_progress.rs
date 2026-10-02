//! Contract: Vision LLM figure analyze emits converting sub-step progress (SPEC-048).

use std::sync::{Arc, Mutex};

use edgequake_api::services::{
    analyze_multimodal_images_with_substep, ConvertingSubstepReporter, MultimodalProviders,
};
use edgequake_llm::MockProvider;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn contract_vision_figure_analyze_reports_substep_milestones() {
    std::env::set_var("VLM_PROCESS_ENABLE", "true");
    std::env::set_var("VLM_MIN_IMAGE_PIXEL", "1");
    // Pin cloud/mock profile so local never-stuck copy does not alter SSOT assertions.
    std::env::set_var("EDGEQUAKE_VISION_PROVIDER", "mock");
    std::env::remove_var("EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY");
    std::env::remove_var("EDGEQUAKE_MM_MAX_FIGURES");

    let tag = r#"<drawing id="im-1" format="png" path="assets/fig.png" caption="Figure 1" />
<drawing id="im-2" format="png" path="assets/fig2.png" caption="Figure 2" />
<drawing id="im-3" format="png" path="assets/fig3.png" caption="Figure 3" />"#
        .to_string();
    let md = format!("Intro\n{tag}\nBody");

    let mock = MockProvider::new();
    mock.add_response(r#"{"name":"fig","type":"Chart","description":"Chart A."}"#)
        .await;
    mock.add_response(r#"{"name":"fig","type":"Chart","description":"Chart B."}"#)
        .await;
    mock.add_response(r#"{"name":"fig","type":"Chart","description":"Chart C."}"#)
        .await;

    let messages: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let msgs = Arc::clone(&messages);
    let reporter: ConvertingSubstepReporter = Arc::new(move |update| {
        msgs.lock()
            .unwrap()
            .push(format!("{}|{:.4}", update.message, update.progress));
    });

    let out = analyze_multimodal_images_with_substep(
        &md,
        Some("i"),
        "doc.pdf",
        MultimodalProviders::single(&mock),
        None,
        None,
        Some(reporter),
        None,
    )
    .await;

    assert!(out.hard_error.is_none());
    let logged = messages.lock().unwrap().clone();
    assert!(
        logged.iter().any(|m| m.contains("figure 0/3")),
        "expected start milestone, got {logged:?}"
    );
    assert!(
        logged.iter().any(|m| m.contains("figure 3/3")),
        "expected completion milestone, got {logged:?}"
    );
    assert!(
        logged
            .iter()
            .any(|m| m.contains("Vision LLM") && m.contains("figure")),
        "messages must use vision figure SSOT copy"
    );

    std::env::remove_var("VLM_MIN_IMAGE_PIXEL");
    std::env::remove_var("VLM_PROCESS_ENABLE");
    std::env::remove_var("EDGEQUAKE_VISION_PROVIDER");
}

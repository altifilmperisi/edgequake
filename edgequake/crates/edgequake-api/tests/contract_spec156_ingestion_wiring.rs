//! SPEC-156 wiring contracts for the API ingest path.

#[test]
fn contract_spec156_partial_checkpoint_uses_single_writer() {
    let extract = include_str!("../src/processor/text_insert/extraction.rs");
    assert!(
        extract.contains("PartialChunkCheckpointWriter::spawn"),
        "Fresh extract must spawn the coalescing partial-chunk writer"
    );
    assert!(
        extract.contains("writer_for_cb.submit") || extract.contains(".submit(chunk_id"),
        "on_chunk callback must submit to the writer (not fire-and-forget KV RMW)"
    );
    assert!(
        !extract.contains("tokio::spawn(async move {\n                            super::pipeline_checkpoint::save_partial_chunk_extraction"),
        "must not per-chunk tokio::spawn(save_partial_chunk_extraction)"
    );
    assert!(
        extract.contains("partial_writer.flush_now()"),
        "success/error paths must flush the writer before clear/return"
    );
}

#[test]
fn contract_spec156_writer_module_is_single_responsibility() {
    let writer = include_str!("../src/processor/partial_chunk_checkpoint_writer.rs");
    assert!(
        writer.contains("COALESCE_MS"),
        "writer must coalesce KV flushes"
    );
    assert!(
        writer.contains("seed_from_kv_if_matching"),
        "writer must seed prior matching partials (resume-safe)"
    );
    assert!(
        writer.contains("PipelineCheckpoint::compute_content_hash"),
        "content hash must be computed once at spawn (not per chunk)"
    );
}

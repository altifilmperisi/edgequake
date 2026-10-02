use super::super::*;
use super::types::{TextInsertExtracted, TextInsertPrepared};
use tokio_util::sync::CancellationToken;

impl DocumentTaskProcessor {
    pub(super) async fn text_insert_extract(
        &self,
        task: &mut Task,
        prepared: TextInsertPrepared,
        cancel_token: CancellationToken,
    ) -> TaskResult<TextInsertExtracted> {
        let document_id = prepared.document_id.clone();
        let text_content = prepared.text_content.clone();
        let tenant_id = prepared.tenant_id.clone();
        let workspace_id = prepared.workspace_id.as_deref();
        let data = prepared.data.clone();
        let provider_lineage = prepared.provider_lineage.clone();
        let processed_text = prepared.processed_text.clone();
        let is_pdf_source = prepared.is_pdf_source;
        let track_id = prepared.track_id.clone();
        let source_type = prepared.source_type.clone();
        let workspace_id_owned = prepared.workspace_id.clone();
        let chunk_progress_callback = prepared.chunk_progress_callback.clone();
        let pipeline = prepared.pipeline.clone();

        // CHECKPOINT: Try to load a saved pipeline checkpoint before running
        // expensive LLM extraction. This saves minutes of processing when
        // a server crashed after extraction but before storage completed.

        // ── CANCELLATION GATE: before LLM extraction (most expensive stage) ──
        self.check_cancelled(&cancel_token, "pre-extraction", &document_id)
            .await?;

        // PDF re-conversion (Full mode): clear any saved KG pipeline checkpoint
        // so entity extraction re-runs against the freshly converted markdown.
        // WHY: Even though a re-converted PDF usually produces a different
        // content hash (so the checkpoint would not match anyway), clearing it
        // explicitly guarantees no stale extraction results are reused when the
        // user explicitly asked for a full re-conversion.
        let force_fresh_extraction = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("force_fresh_extraction"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let merge_only = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("merge_only"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if force_fresh_extraction {
            info!(
                document_id = %document_id,
                "Fresh extraction requested — clearing KG pipeline checkpoint + extraction snapshot"
            );
            super::pipeline_checkpoint::clear_pipeline_checkpoint(
                &self.kv_storage,
                self.checkpoint_store(),
                &document_id,
            )
            .await;
            super::pipeline_checkpoint::clear_extraction_snapshot(
                &self.kv_storage,
                self.checkpoint_store(),
                &document_id,
            )
            .await;
            super::pipeline_checkpoint::clear_partial_chunk_checkpoint(
                &self.kv_storage,
                &document_id,
            )
            .await;
        }

        let hybrid_dirty_pages = data
            .reuse_excluded_pages
            .as_ref()
            .is_some_and(|pages| !pages.is_empty());

        let checkpoint_result = super::pipeline_checkpoint::load_pipeline_checkpoint(
            &self.kv_storage,
            self.checkpoint_store(),
            &document_id,
            &data.workspace_id,
            &provider_lineage.extraction_provider,
            &provider_lineage.embedding_provider,
            &processed_text,
        )
        .await;

        // SPEC-047 P7e: durable snapshot after successful persist (survives checkpoint clear).
        // SPEC-151: never call RequireMatch snapshot load when hybrid dirty pages are set —
        // splice changes the content hash and RequireMatch would DELETE the blob that
        // ChunkReuseIndex still needs (EC-151-20). Hybrid seeding uses
        // load_extraction_snapshot_for_hybrid_reuse further below.
        let snapshot_result =
            if checkpoint_result.is_none() && !force_fresh_extraction && !hybrid_dirty_pages {
                super::pipeline_checkpoint::load_extraction_snapshot(
                    &self.kv_storage,
                    self.checkpoint_store(),
                    &document_id,
                    &data.workspace_id,
                    &provider_lineage.extraction_provider,
                    &provider_lineage.embedding_provider,
                    &processed_text,
                )
                .await
            } else {
                None
            };

        let reuse_plan = super::pipeline_checkpoint::plan_extraction_reuse(
            checkpoint_result.is_some(),
            snapshot_result.is_some(),
            force_fresh_extraction,
            merge_only,
            hybrid_dirty_pages,
        );

        // SPEC-047 P0: announce extracting *before* LLM work (not after embed).
        self.update_document_status(&document_id, "extracting", None)
            .await?;

        let embed_progress_callback = self.build_embed_progress_callback(&document_id);

        let (mut result, resumed_from_checkpoint) = match reuse_plan {
            super::pipeline_checkpoint::ExtractionReusePlan::MergeOnlyMissing => {
                let error_msg = "merge_only requested but no extraction snapshot/checkpoint found \
                     — run entities reprocess once to create a snapshot, or disable merge_only"
                    .to_string();
                error!(
                    document_id = %document_id,
                    "P7e: merge_only missing durable extractions"
                );
                self.update_document_status(&document_id, "failed", Some(&error_msg))
                    .await?;
                self.pipeline_state
                    .document_failed(&document_id, &error_msg)
                    .await;
                return Err(edgequake_tasks::TaskError::Process(error_msg));
            }
            super::pipeline_checkpoint::ExtractionReusePlan::Reuse(kind) => {
                let reused = match kind {
                    super::pipeline_checkpoint::ExtractionReuseKind::CrashCheckpoint => {
                        checkpoint_result.expect("plan said checkpoint present")
                    }
                    super::pipeline_checkpoint::ExtractionReuseKind::DurableSnapshot => {
                        snapshot_result.expect("plan said snapshot present")
                    }
                };
                info!(
                    document_id = %document_id,
                    chunks = reused.chunks.len(),
                    entities = reused.stats.entity_count,
                    reuse = ?kind,
                    needs_reembed = reused.needs_reembed(),
                    "P7e/CHECKPOINT-RESUME: Skipping LLM extraction — loaded stored extractions"
                );
                (reused, true)
            }
            super::pipeline_checkpoint::ExtractionReusePlan::Fresh => {
                // No valid checkpoint — run the full pipeline (extract + embed).
                // SPEC-091 WP1: cancel before embed-bearing pipeline work.
                self.check_cancelled(&cancel_token, "pre-embed", &document_id)
                    .await?;
                let resume_chunks = super::pipeline_checkpoint::load_partial_chunk_checkpoint(
                    &self.kv_storage,
                    &document_id,
                    &data.workspace_id,
                    &provider_lineage.extraction_provider,
                    &processed_text,
                )
                .await;
                // SPEC-151: content-hash reuse for page reprocess (excluded pages = dirty).
                // Use hybrid loader so splice hash mismatch does not wipe the snapshot (EC-151-20).
                // Fail closed when there is nothing reusable — never LLM the whole document.
                // WHY before writer spawn (SPEC-156 / SRP): do not open a checkpoint
                // consumer for a path that returns before any chunk completes.
                let reuse_index = if let Some(ref excluded) = data.reuse_excluded_pages {
                    if excluded.is_empty() {
                        None
                    } else {
                        let snap =
                            super::pipeline_checkpoint::load_extraction_snapshot_for_hybrid_reuse(
                                &self.kv_storage,
                                self.checkpoint_store(),
                                &document_id,
                                &data.workspace_id,
                                &provider_lineage.extraction_provider,
                                &provider_lineage.embedding_provider,
                                &processed_text,
                            )
                            .await;
                        if let Err(error_msg) =
                            crate::processor::page_reprocess::require_hybrid_snapshot_for_page_extract(
                                excluded,
                                snap.as_ref().map(|s| s.extractions.len()),
                            )
                        {
                            error!(
                                document_id = %document_id,
                                excluded_pages = ?excluded,
                                "SPEC-151: fail-closed — no reusable extractions for hybrid page reprocess"
                            );
                            self.update_document_status(
                                &document_id,
                                "failed",
                                Some(&error_msg),
                            )
                            .await?;
                            self.pipeline_state
                                .document_failed(&document_id, &error_msg)
                                .await;
                            return Err(edgequake_tasks::TaskError::Process(error_msg));
                        }
                        let snap = snap.expect("gated by require_hybrid_snapshot_for_page_extract");
                        info!(
                            document_id = %document_id,
                            prior_chunks = snap.chunks.len(),
                            prior_extractions = snap.extractions.len(),
                            excluded_pages = ?excluded,
                            "SPEC-151: seeding ChunkReuseIndex from durable snapshot (hybrid)"
                        );
                        Some(edgequake_pipeline::ChunkReuseIndex::from_snapshot(
                            &snap.chunks,
                            &snap.extractions,
                            excluded.iter().copied(),
                        ))
                    }
                } else {
                    None
                };
                // SPEC-156: single-writer coalescing partial checkpoint (no
                // per-chunk tokio::spawn RMW races / O(N²) rewrite).
                let partial_writer = crate::processor::partial_chunk_checkpoint_writer::PartialChunkCheckpointWriter::spawn(
                    document_id.clone(),
                    data.workspace_id.clone(),
                    provider_lineage.extraction_provider.clone(),
                    &processed_text,
                    Arc::clone(&self.kv_storage),
                );
                let writer_for_cb = partial_writer.clone();
                let on_chunk: Option<edgequake_pipeline::ChunkExtractedCallback> =
                    Some(std::sync::Arc::new(move |chunk_id, result| {
                        writer_for_cb.submit(chunk_id, result);
                    }));
                let fresh_result = match pipeline
                    .process_with_resilience_cancellable_reuse(
                        &document_id,
                        &processed_text,
                        Some(chunk_progress_callback.clone()),
                        Some(cancel_token.clone()),
                        Some(embed_progress_callback.clone()),
                        // Page-scope: never seed positional id resume (LAW-151-4).
                        if reuse_index.is_some() {
                            None
                        } else {
                            resume_chunks
                        },
                        reuse_index,
                        on_chunk,
                    )
                    .await
                {
                    Ok(result) => {
                        // SPEC-151: clean-page hash miss must abort before checkpoint/retract.
                        let clean_miss = result.stats.chunk_errors.as_ref().is_some_and(|errs| {
                            errs.iter().any(|e| {
                                e.error_message
                                    .contains(edgequake_pipeline::SPEC151_CLEAN_HASH_MISS)
                            })
                        });
                        if clean_miss
                            && data
                                .reuse_excluded_pages
                                .as_ref()
                                .is_some_and(|p| !p.is_empty())
                        {
                            let error_msg = result
                                .stats
                                .chunk_errors
                                .as_ref()
                                .and_then(|errs| {
                                    errs.iter()
                                        .find(|e| {
                                            e.error_message.contains(
                                                edgequake_pipeline::SPEC151_CLEAN_HASH_MISS,
                                            )
                                        })
                                        .map(|e| e.error_message.clone())
                                })
                                .unwrap_or_else(|| {
                                    format!(
                                        "{}: clean page content changed during page reprocess",
                                        edgequake_pipeline::SPEC151_CLEAN_HASH_MISS
                                    )
                                });
                            error!(
                                document_id = %document_id,
                                "SPEC-151: abort Insert before retract — clean hash miss"
                            );
                            self.update_document_status(&document_id, "failed", Some(&error_msg))
                                .await?;
                            self.pipeline_state
                                .document_failed(&document_id, &error_msg)
                                .await;
                            partial_writer.flush_now().await;
                            return Err(edgequake_tasks::TaskError::Process(error_msg));
                        }

                        // SPEC-003: Log partial success if some chunks failed
                        if result.stats.failed_chunks > 0 {
                            warn!(
                                document_id = %document_id,
                                successful_chunks = result.stats.successful_chunks,
                                failed_chunks = result.stats.failed_chunks,
                                total_chunks = result.stats.chunk_count,
                                error.code = "EXTRACTION_PARTIAL_FAILURE",
                                error.source = "pipeline",
                                "Document processed with partial success - some chunks failed extraction"
                            );
                            edgequake_observability::ErrorEvent::log_domain_warn(
                                "pipeline",
                                "partial_extraction",
                                "Some chunks failed extraction",
                                serde_json::json!({
                                    "document_id": document_id,
                                    "successful_chunks": result.stats.successful_chunks,
                                    "failed_chunks": result.stats.failed_chunks,
                                    "total_chunks": result.stats.chunk_count,
                                }),
                            );

                            // Emit WebSocket events for failed chunks
                            if let Some(ref chunk_errors) = result.stats.chunk_errors {
                                for error_info in chunk_errors {
                                    self.pipeline_state.emit_chunk_failure(
                                        document_id.clone(),
                                        task.track_id.clone(),
                                        error_info.chunk_index as u32,
                                        result.stats.chunk_count as u32,
                                        error_info.error_message.clone(),
                                        error_info.was_timeout,
                                        error_info.retry_attempts,
                                    );
                                }

                                // SPEC-046 OPS-P0.4: persist to failed_chunks for retry API
                                #[cfg(feature = "postgres")]
                                if let Some(ref pool) = self.pg_pool {
                                    let ws = workspace_id
                                        .unwrap_or("00000000-0000-0000-0000-000000000000");
                                    crate::handlers::persist_chunk_failures_from_stats(
                                        pool,
                                        &document_id,
                                        ws,
                                        tenant_id.as_deref(),
                                        chunk_errors,
                                    )
                                    .await;
                                }
                            }
                        }
                        result
                    }
                    Err(e) => {
                        // X-30: typed class first; embed failure_class= for string consumers.
                        let class = crate::services::classify_from_pipeline_error(&e);
                        let error_msg = format!(
                            "Pipeline processing failed: {} [failure_class={}]",
                            e.display_with_failure_class(),
                            class.as_str()
                        );
                        if crate::services::task_cancel::is_cancel_error_message(&error_msg) {
                            let _ = crate::services::sync_doc_cancelled_by_document_id(
                                Arc::clone(&self.kv_storage),
                                self.optional_pg_pool(),
                                &document_id,
                                &error_msg,
                            )
                            .await;
                            partial_writer.flush_now().await;
                            return Err(edgequake_tasks::TaskError::Cancelled(error_msg));
                        }
                        error!(
                            document_id = %document_id,
                            workspace_id = ?workspace_id,
                            tenant_id = ?tenant_id,
                            content_length = text_content.len(),
                            error = %e,
                            error.source = "pipeline",
                            error.code = "PIPELINE_PROCESSING_FAILED",
                            "CRITICAL: Pipeline processing failed - document marked as failed"
                        );
                        edgequake_observability::record_document_processing(
                            "text_insert",
                            "pipeline",
                            "failure",
                            0.0,
                        );

                        // Update document status to failed with detailed error
                        self.update_document_status(&document_id, "failed", Some(&error_msg))
                            .await?;

                        // Keep failed staging metadata for list/ActiveRuns; only
                        // free content + hash so re-upload is not blocked (SPEC-086).
                        if let (Some(hash), Some(ws)) = (
                            data.metadata
                                .as_ref()
                                .and_then(|m| m.get("content_hash"))
                                .and_then(|v| v.as_str()),
                            data.metadata
                                .as_ref()
                                .and_then(|m| m.get("workspace_id"))
                                .and_then(|v| v.as_str()),
                        ) {
                            let _ = crate::services::release_staging_reservation(
                                &self.kv_storage,
                                &document_id,
                                ws,
                                hash,
                            )
                            .await;
                            // SPEC-091 W2: typed ingestion_dedup staging release.
                            #[cfg(feature = "postgres")]
                            crate::services::ingestion_dedup_store::dual_release_staging(
                                self.optional_pg_pool(),
                                ws,
                                hash,
                            )
                            .await;
                        }

                        self.pipeline_state
                            .document_failed(&document_id, &error_msg)
                            .await;

                        partial_writer.flush_now().await;
                        return Err(edgequake_tasks::TaskError::Process(error_msg));
                    }
                };

                // CHECKPOINT-SAVE: Persist pipeline results so a crash during
                // storage won't force re-running the expensive LLM extraction.
                // Embeddings are stripped (SPEC-047 P5) — re-embedded on resume.
                if let Err(e) = super::pipeline_checkpoint::save_pipeline_checkpoint(
                    &self.kv_storage,
                    self.checkpoint_store(),
                    &document_id,
                    &fresh_result,
                    &data.workspace_id,
                    &provider_lineage.extraction_provider,
                    &provider_lineage.embedding_provider,
                    &processed_text,
                )
                .await
                {
                    warn!(
                        document_id = %document_id,
                        error = %e,
                        "Failed to save pipeline checkpoint — processing continues without checkpoint"
                    );
                } else {
                    partial_writer.flush_now().await;
                    super::pipeline_checkpoint::clear_partial_chunk_checkpoint(
                        &self.kv_storage,
                        &document_id,
                    )
                    .await;
                }

                (fresh_result, false)
            }
        };

        // SPEC-047 P5 / SPEC-057 P2: slim checkpoints omit embeddings — re-embed
        // before persist and surface an honest stage (not silent "embedding").
        if result.needs_reembed() {
            info!(
                document_id = %document_id,
                resumed = resumed_from_checkpoint,
                embeddings_omitted = true,
                "Re-generating embeddings (slim checkpoint or incomplete embed)"
            );
            self.check_cancelled(&cancel_token, "pre-embed", &document_id)
                .await?;
            self.update_document_status(
                &document_id,
                "re_embedding",
                Some("Re-generating embeddings after slim checkpoint (embeddings_omitted)"),
            )
            .await?;
            if let Err(e) = pipeline
                .ensure_embeddings(&mut result, Some(&embed_progress_callback))
                .await
            {
                let error_msg = format!("Embedding regeneration failed: {e}");
                error!(
                    document_id = %document_id,
                    error = %e,
                    "CRITICAL: ensure_embeddings failed after checkpoint resume"
                );
                self.update_document_status(&document_id, "failed", Some(&error_msg))
                    .await?;
                self.pipeline_state
                    .document_failed(&document_id, &error_msg)
                    .await;
                return Err(edgequake_tasks::TaskError::Process(error_msg));
            }
        }

        // Phase 4k: inject mm entity + association edges when sidecar chunks persisted.
        let mm_metas: Vec<edgequake_pipeline::MmChunkSidecarMeta> = if let Some(mm_chunks) =
            crate::services::load_mm_chunks(
                self.kv_storage.as_ref(),
                self.checkpoint_store(),
                &document_id,
            )
            .await
        {
            mm_chunks
                .iter()
                .filter_map(|c| serde_json::from_value(serde_json::to_value(c).ok()?).ok())
                .collect()
        } else {
            Vec::new()
        };

        if !mm_metas.is_empty() {
            let file_path = data.file_source.as_str();
            // Prefer metadata title; fall back to file stem (066 Drawing display_name).
            let doc_title = data
                .metadata
                .as_ref()
                .and_then(|m| m.get("title"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(file_path);
            edgequake_pipeline::inject_modality_relations(
                &mut result.extractions,
                &result.chunks,
                &mm_metas,
                file_path,
                Some(doc_title),
            );
        }
        edgequake_pipeline::stamp_retrieval_modality_on_chunks(&mut result.chunks, &mm_metas);

        // Log checkpoint usage metrics
        if resumed_from_checkpoint {
            info!(
                document_id = %document_id,
                "CHECKPOINT-STATS: Resumed from checkpoint — saved LLM extraction time"
            );
        }

        // Update task progress — extract+embed done; persist owns indexing.
        self.bump_task_progress(task, "extraction_complete".to_string(), 4, 30)
            .await;

        // ── CANCELLATION GATE: after extraction, before embedding storage ──
        self.check_cancelled(&cancel_token, "post-extraction", &document_id)
            .await?;

        self.pipeline_state
            .info(format!(
                "Generated {} chunks for {}",
                result.chunks.len(),
                document_id
            ))
            .await;

        // SPEC-047 P0: do NOT set status back to "extracting" here — extract+embed
        // already finished. Persist will move to "indexing".

        Ok(TextInsertExtracted {
            prepared: TextInsertPrepared {
                document_id,
                text_content,
                source_type,
                tenant_id,
                workspace_id: workspace_id_owned,
                is_pdf_source,
                track_id,
                data,
                pipeline,
                provider_lineage,
                processed_text,
                chunk_progress_callback,
            },
            result,
        })
    }

    /// Fire-and-forget progress while embedding sub-batches run (SPEC-155 ledger).
    fn build_embed_progress_callback(&self, document_id: &str) -> EmbedProgressCallback {
        let run_progress = crate::services::RunProgressWriter::spawn(
            document_id.to_string(),
            Arc::clone(&self.kv_storage),
        );
        Arc::new(move |update: EmbedProgressUpdate| {
            run_progress.task(
                crate::services::RunTaskId::Embeddings,
                update.current as u64,
                update.total as u64,
                None,
            );
        })
    }
}

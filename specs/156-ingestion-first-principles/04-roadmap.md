# 04 — Roadmap (ordered)

Effort: S ≤ 0.5d · M ≤ 2d · L ≤ 1w.

| # | Item | Owner files | Effort | Status | Test / gate |
|---|------|-------------|--------|--------|-------------|
| 1 | Single-writer partial checkpoint | `partial_chunk_checkpoint_writer.rs`, `text_insert/extraction.rs` | S | **SHIPPED** | Concurrent completes → no lost ids; `contract_spec156_*` |
| 2 | Glean fail-open + early stop | `extractor/gleaning.rs` | S | **SHIPPED** | Mock LLM fail / empty iteration |
| 3 | Parallel embeds under shared semaphore | `pipeline/helpers/embeddings.rs` | S | **SHIPPED** | Peak ≤ cap; `e2e_perf_embed_join` |
| 4 | Heartbeat = TTL/3 | `edgequake-tasks` worker + lease helper | S | **SHIPPED** | Derivation + worker wiring contract |
| 5 | extraction_time_ms + drop redundant Semaphore + PDF env docs | `extraction.rs`, `.env.example`, `Makefile` | S | **SHIPPED** | `e2e_perf_extraction_fanout` + wiring contract |
| 6 | Overlap chunk embed with extraction | `processing.rs`, embeddings | M | roadmap | Cancel + resume still correct |
| 7 | Extraction response cache (chunk hash) | SPEC-103 store + extractor | M | roadmap | Hit on reprocess identical text |
| 8 | Embedding cache hash(model,text) | embeddings helper | M | roadmap | Cross-doc hit |
| 9 | Unified RetryPolicy + chunk budget | sota, glean, chunk, embed, worker | L | roadmap | No substring classifiers; Acc stable |
| 10 | Task `available_at` backoff | tasks claim / orphaned eligibility | M | roadmap | Poll does not steal early |
| 11 | Global cloud RPM/TPM limiter (AIMD) | provider factory + edgequake-llm | L | roadmap | 429 storm bench |
| 12 | Server stall reaper | run_progress ledger janitor | M | roadmap | Stale `updated_at` cancels |
| 13 | Adaptive raster Option A + harness | pdf_processing, manuscript_profile, page_convert_plan | L | roadmap | Eval gate before default |
| 14 | Render-once viewer PNG reuse | pdf2md or page_assets stream | M | roadmap | Peak RSS / render count |
| 15 | Cost SSOT + vision/summarizer/cache tokens | cost tables, stats | M | roadmap | SPEC-153 ledger contract |
| 16 | Persist `try_join!` + merge phase overlap | ingestion_persister, merger | M | roadmap | Idempotent under cancel |
| 17 | Split mega-files; DRY checkpoint + extractor factory | worker, pdf_processing, merger, core orchestrator | L | roadmap | Clippy size / module map |

Items 1–5 ship with this pack. Do not change Acc pins (`fifo`, LLM cache off, local concurrency).

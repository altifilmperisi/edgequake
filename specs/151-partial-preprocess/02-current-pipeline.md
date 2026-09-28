# 02 — Current pipeline (code-grounded)

Parent: [README](README.md) · Why: [00](00-why.md) · Target backend: [06](06-backend-architecture.md)

## End-to-end flow (as-is)

```text
  POST /documents/pdf
       │
       ▼
  pdf_documents + pdf_document_blobs
       │ enqueue TaskType::PdfProcessing
       ▼
  process_pdf_processing_inner
       │  modality plan (SPEC-134) → PageSelection::Set per group
       │  VisionPdfConverter::convert  (OCR + assets + figure filter)
       │  stitch_page_markdown_in_order
       │  empty-page escalate / grounding verify
       │  multimodal Pass B (optional)
       │  persist markdown → enqueue Insert
       ▼
  text_insert: chunk → extract → persist → merge → finalize
```

## Key files

| Concern | Path |
|---------|------|
| Upload + enqueue | `edgequake-api/src/handlers/pdf_upload/upload.rs` |
| PdfProcessing worker | `edgequake-api/src/processor/pdf_processing.rs` |
| Vision convert + assets | `edgequake-pdf/src/backend/vision.rs` |
| Page markers / stitch | `edgequake-pdf/src/vision_markdown.rs`, `page_marker.rs` |
| Empty-page splice | `edgequake-api/src/services/manuscript_verify.rs` (`split_page_sections`) |
| Chunking | `edgequake-pipeline/src/chunker/page_aware.rs` |
| Mid-chunk resume | `edgequake-pipeline/src/pipeline/extraction.rs` (`resume_by_chunk_id`) |
| Checkpoints / snapshot | `edgequake-api/src/processor/pipeline_checkpoint.rs` |
| Full reprocess | `edgequake-api/src/handlers/documents/recovery/reprocess*.rs` |
| Retry chunks | `edgequake-api/src/handlers/documents/recovery/chunks.rs` |
| Layout pages | migration `148_document_pages_layout.sql` |
| MM assets | migration `084_add_document_mm_assets.sql` |

## What exists that SPEC-151 reuses

1. **`PageSelection::Set`** — convert already scopes OCR + asset writers to a page set.
2. **Section splice precedents** — empty-page escalate replaces sections in-place; stitch merges groups by marker.
3. **Extraction snapshot** — durable `ProcessingResult` for entities/merge modes.
4. **Partial chunk checkpoint** — per-chunk mid-extract map (whole-doc hash gated).
5. **`document_pages` / `document_mm_assets`** — page-keyed geometry and binaries.

## What does NOT exist (gaps)

| Gap | Impact |
|-----|--------|
| No `document_page_states` | Cannot show P/F/E health; cannot salvage OCR |
| No `page_scope` on tasks | Cannot enqueue partial convert |
| Duplicate section splitters | DRY debt |
| Assets fused inside `convert()` | Hard to run figures-only |
| Resume keyed only by chunk id | Breaks after re-chunk |
| UI: Full vs Entities only | No page selection |

## Progress channels (as-is)

```text
  pdf2md callback → PipelineProgressCallback
       ├── PipelineEvent::PdfPageProgress → WS
       ├── PdfUploadProgress (in-memory, track_id)
       └── KV stage_message / stage_progress
```

Frontend assumes `completed_pages` is monotonic against `total_pages`. Partial reprocess must use a distinct phase (`page_reprocess`) with `total = selected_pages.len()`.

Cross-refs: [01 laws](01-first-principles.md) · [08 AI](08-ai-pipeline.md).

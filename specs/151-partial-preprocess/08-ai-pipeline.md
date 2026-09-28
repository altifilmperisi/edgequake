# 08 — AI pipeline (AI Engineer)

Parent: [README](README.md) · Laws: [01](01-first-principles.md) · Backend: [06](06-backend-architecture.md)

## WHY

Vision and extraction are expensive. Partial reprocess must minimise tokens while preserving quality guards from SPEC-134 / SPEC-049.

## Parse stage

```text
  classify_pages_from_bytes → filter to selected pages
  group by modality (print vs manuscript)
  for each group:
    conversion_config_for_group(..., PageSelection::Set(pages))
    VisionPdfConverter::convert
    page_result_sink ← raw PageResult per page
  replace_sections(existing_md, new_sections, never_downgrade=true)
```

- Manuscript pages keep `EDGEQUAKE_VISION_*_MANUSCRIPT` overrides.
- Empty-page escalate scoped to selected pages only.
- Grounding verify scoped to selected manuscript pages.

## Figures stage

```text
  build_page_asset_bundle(pdf, pages, cfg)
    ├── embedded figures / caption regions
    ├── page PNGs
    ├── chart crops
    └── FigureFilter (print only)
  assemble_page_sections → splice figure links into page sections
  persist mm_assets for those page_nums only
```

If `raw_markdown` missing for a page → planner adds Parse (LAW-151-3).

## Entities stage

```text
  load extraction snapshot (if any)
  ChunkReuseIndex::from_snapshot(snapshot, excluded_pages=selected)
  chunk full markdown (PageAwareChunking)
  for each new chunk:
    if overlaps selected → extract fresh
    else if content_hash hit → rebind ExtractionResult
    else → extract fresh
  retract_document_indexes
  merge_with_progress(hybrid extractions)
  save new snapshot + update page entity statuses
```

## Cost model (dry_run estimates)

| Stage | Estimated vision calls |
|-------|------------------------|
| Parse | ≈ selected_pages (×2 if empty retry escalates) |
| Figures | ≈ figure_candidates + filter calls (heuristic from prior figures_count) |
| Entities | 0 vision; LLM text calls ≈ dirty_chunk_count × (1+gleaning) |

## Quality guards

- Never downgrade (LAW-151-2).
- Prefer non-placeholder on stitch (`prefer_page_section`).
- Pass B multimodal: run only on sub-document of selected page sections; re-stitch results.
- Suggest full reprocess when `pages.len() == page_count`.

Cross-refs: edge cases [09](09-edge-cases.md).

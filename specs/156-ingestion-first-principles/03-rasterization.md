# 03 — Rasterization honesty and adaptive options

## Honest model (LAW-156-6)

In `edgequake-pdf2md` 0.9.11, `render_pages_blocking` takes `_dpi` (unused). Render size is:

```text
PdfRenderConfig::set_target_width(max_pixels).set_maximum_height(max_pixels)
```

| Class | Effective long edge | Role of `dpi` |
|-------|---------------------|---------------|
| Print | 2000 (`max_rendered_pixels=None` → pdf2md default) | Checkpoint key + logs only |
| Manuscript | 3600 (`DEFAULT_MANUSCRIPT_MAX_PIXELS`) | Same; env can raise DPI but not pixels unless `EDGEQUAKE_PDF_MANUSCRIPT_MAX_PIXELS` |

Adaptive print DPI (96 / 110 / 120 / 150) from `compute_safe_pdf_resource_profile` therefore **does not** reduce OCR cost or memory today.

`ImageGuardProvider` (`EDGEQUAKE_VISION_MAX_IMAGE_BYTES`, default 3.5 MiB) may re-encode to JPEG after the PNG is built — reactive, not proactive.

## Single choke points

| Site | File | Responsibility |
|------|------|----------------|
| Resource profile | `pdf_processing.rs` `compute_safe_pdf_resource_profile` | concurrency + (today) cosmetic dpi |
| Per-group convert | `conversion_config_for_group` | must set real `max_rendered_pixels` for print if DPI tiers matter |
| Manuscript profile | `edgequake-pdf` `ManuscriptProfile::resolve` | manuscript dpi + max_pixels SSOT |
| Grouping | `PageConvertPlan::groups` | print / manuscript / future tiers |

## Option A — quality-first (default)

Keep 2000 / 3600 as **ceilings**.

1. **Per-page text-layer → EdgeParse** using existing `PageSignals.text_chars` (≥ threshold), not document-wide all-or-nothing.
2. **Page-size-aware cap:** `min(ceiling, page_long_pt × target_dpi / 72)` so A6/slides are not upscaled to full box.
3. **Sparse-print tier:** lower long edge (e.g. 1200–1600) when glyph density is low; still below ceiling.
4. **Render once, reuse:** stream viewer PNGs from the OCR render path (or pdf2md optional persist) — today OCR + viewer + chart crop can rasterize three times.

Ship only after an evaluation harness (OmniDocBench / Acc PDF subset) shows no quality regression.

## Option B — cost-first (eval-gated)

Lower print default (e.g. 1200–1600 px). Escalate to 2000+ when empty-page retry or manuscript verify signals fire. Same choke points; different default map.

## Docs quick win (this pack)

- Document in `.env.example`: `EDGEQUAKE_PDF_DPI` is cosmetic for OCR size; `EDGEQUAKE_PDF_MANUSCRIPT_MAX_PIXELS` / print default 2000 are the real knobs; list `EDGEQUAKE_PDF_CONCURRENCY`, `EDGEQUAKE_PDF_VISION_JOBS`.
- Fix Makefile comment claiming `PDF_CONCURRENCY=4` × jobs = 16 in-flight — code clamps cloud page concurrency to 2.

## Checkpoint ID gap (roadmap)

Include `max_rendered_pixels` and prompt fingerprint in the conversion checkpoint id so pixel-cap or prompt changes do not silently resume stale pages.

## Tests / harness (roadmap)

- Unit: `print_long_edge(dpi)` / page-size cap helpers.
- Golden: page-tier routing fixtures.
- Eval: Option A vs current 2000/3600 on a fixed PDF set.

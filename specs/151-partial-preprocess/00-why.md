# 00 — WHY (5-WHY)

Parent: [README](README.md) · Next: [01-first-principles](01-first-principles.md)

## The job to be done

A knowledge worker who uploaded a multi-page PDF must be able to:

1. **See** which pages failed parsing, figures, or entity extraction.
2. **Select** one page or a range of pages.
3. **Re-run** only the broken stage(s) without wiping healthy pages.
4. **Keep** every page that already succeeded (never lose partial work).
5. **Trust** that the knowledge graph is rebuilt correctly from the new page text plus reused extractions.

That is the whole spec. Everything else is a means.

---

## 5-WHY chain A — Lost partial work

| # | Question | Answer |
|---|----------|--------|
| 1 | Why do users re-upload or full-reprocess after a mid-document failure? | Because a failed convert leaves them with nothing durable to resume from the UI. |
| 2 | Why is nothing durable? | pdf2md writes per-page checkpoints to disk, then **deletes them on full success**; on failure the stitched markdown may never land in `pdf_documents.markdown_content`. |
| 3 | Why is the only durable unit the whole markdown string? | The pipeline was designed for whole-document convert → insert. There is no `document_page_states` table. |
| 4 | Why was per-page state skipped? | Early vision conversion treated the document as one job; empty-page retry and modality groups patched sections in memory only. |
| 5 | **Root cause** | **No first-class page state.** Partial OCR exists transiently; the product surface only knows document status. |

```text
  Vision OCR page 1..k succeed ──► disk checkpoint
  Vision OCR page k+1 fails
         |
         v
  User sees "Failed" ──► Retry = Full convert again
         |
         v
  Pages 1..k re-OCR'd (cost + risk of regression)
```

---

## 5-WHY chain B — Coarse reprocess destroys good work

| # | Question | Answer |
|---|----------|--------|
| 1 | Why does fixing one bad page cost a full reprocess? | `POST /documents/reprocess` modes are Full / Entities / Merge — all document-scoped. |
| 2 | Why is there no page field? | `ReprocessFailedRequest` has no pages; `PdfProcessingData` has no `page_scope`. |
| 3 | Why was page selection not wired despite pdf2md supporting `PageSelection::Set`? | Internal modality groups use page sets; the public API and UI never exposed them. |
| 4 | Why does Full clear everything? | Restart deletes `{doc}-content` and every `{doc}-chunk-*`, then retracts the whole graph. |
| 5 | **Root cause** | **Reprocess granularity = document.** Converter already knows pages; product does not. |

---

## 5-WHY chain C — Stages are entangled

| # | Question | Answer |
|---|----------|--------|
| 1 | Why can't a user re-run only figures or only entities for page 7? | UI offers Full vs Entities only; figures live inside Pass-A vision convert. |
| 2 | Why are figures inside convert? | Asset writers (figures, charts, page PNGs, figure filter) run in `VisionPdfConverter::convert` after OCR. |
| 3 | Why is that a problem for repair? | A figure-only bug forces re-OCR of page text the user already accepted. |
| 4 | Why is entity repair also coarse? | `retry-chunks` exists but is unused in UI; checkpoints invalidate on any markdown edit (whole-doc hash). |
| 5 | **Root cause** | **Stages are not product concepts.** Parse / figures / entities must be named, ordered, and independently triggerable (with lawful closure). |

```text
  TODAY (entangled)              TARGET (named stages)
  -----------------              ---------------------
  convert = OCR+assets+filter    Parse  → page markdown
  insert  = chunk+extract+merge  Figures → crops + filter
                                 Entities → extract + rebuild
```

---

## Cost of the status quo

- Vision tokens re-spent on healthy pages.
- Risk of regressing a good OCR when re-running Full.
- Operators cannot answer "which pages are red?" without reading logs.
- Partial_failure / failed_chunks exist but have no page-centric UX.

## Why this spec now

We already have:

- `PageSelection::Set` in convert config.
- Page markers + section splice precedents (`escalate_empty_pages_bounded`, `stitch_page_markdown_in_order`).
- Extraction snapshot + mid-chunk resume.
- Layout table `document_pages` (geometry only).

SPEC-151 closes the product gap: **durable page state + staged partial reprocess + polished UX**.

Cross-refs: laws in [01](01-first-principles.md) · as-is map in [02](02-current-pipeline.md) · REQs in [03](03-requirements.md).

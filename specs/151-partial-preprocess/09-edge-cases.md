# 09 — Edge cases

Parent: [README](README.md) · Laws: [01](01-first-principles.md) · Tests: [11](11-test-plan.md)

| ID | Case | Mitigation | Test |
|----|------|------------|------|
| EC-151-01 | Re-OCR fails for a page that had good text | Never downgrade section | T-151-unit-splice |
| EC-151-02 | Empty / out-of-range / duplicate pages | Validate + dedupe; 400 | T-151-unit-pagescope |
| EC-151-03 | Active task on document | 409 Conflict | T-151-api-busy |
| EC-151-04 | Legacy doc, no raw_markdown, figures-only | Planner adds Parse | T-151-unit-closure |
| EC-151-05 | No extraction snapshot | Fail closed (422 / task error); run full reprocess once | T-151-unit-reuse / T-151-api-422 |
| EC-151-06 | Cross-page chunk (page_end > page_start) | Overlap ⇒ dirty | T-151-unit-reuse |
| EC-151-07 | Original EdgeParse backend | Vision OCR for selected pages | T-151-int-parse |
| EC-151-08 | Manuscript modality pages | Keep modality routing | T-151-int-parse |
| EC-151-09 | Cancel mid-run | Keep completed page states | T-151-int-cancel |
| EC-151-10 | Delete document mid-run | Cancel + CASCADE states | T-151-int-delete |
| EC-151-11 | Select all pages | Warning + suggest full reprocess | T-151-ui-dialog |
| EC-151-12 | Tail after last marker (crop-coverage, mm-chunks) | `replace_sections` preserves tails | T-151-unit-splice |
| EC-151-13 | Missing page markers for some pages | Insert markers in order | T-151-unit-splice |
| EC-151-14 | Workspace isolation | RLS + explicit workspace filter | T-151-int-rls |
| EC-151-15 | Double submit | Admission registry | T-151-api-busy |
| EC-151-16 | Progress phase reset | Distinct `page_reprocess` phase | T-151-e2e-progress |
| EC-151-17 | Placeholder-only page | May replace with new OCR | T-151-unit-splice |
| EC-151-18 | Figures stage with no vision provider | 422 | T-151-api-422 |
| EC-151-19 | Concurrent retry-chunks | 409 from has_active_task | T-151-api-busy |
| EC-151-20 | Snapshot content_hash mismatch after splice | Snapshot invalidated; rebuild reuse from saved extractions before splice or clear | T-151-unit-reuse |

Cross-refs: implementation [10](10-implementation-plan.md).

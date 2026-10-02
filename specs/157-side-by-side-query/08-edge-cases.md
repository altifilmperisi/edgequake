# 08 — Edge cases (EC-157)

Parent: [README](README.md) · Laws: [01](01-first-principles.md) ·
Gates: [10](10-e2e-test-matrix.md)

Every EC has: severity, mitigation, wave, and test id (see matrix).

## Register

| ID | Case | Sev | Mitigation | Wave | Test |
|----|------|-----|------------|------|------|
| **EC-157-01** | Citation has no `page_start` (text/md) | P0 | Source shows text/markdown; scroll to `start_line`/`end_line` or chunk highlight | W2 | `companion_text_doc` |
| **EC-157-02** | PDF 404 / removed / not processed | P0 | Pane error with Retry + humanised copy (reuse PDFViewer messages) | W2 | `companion_pdf_404` |
| **EC-157-03** | Auth expiry mid-view | P1 | Authenticated source failure → re-auth tip / Retry | W2 | `companion_pdf_auth` |
| **EC-157-04** | Huge PDF / many pages | P0 | Keep windowed render; no remount on splitter resize | W2 | `companion_pdf_no_remount` |
| **EC-157-05** | Document deleted while open | P1 | Query invalidation → error state; close still works | W2 | `companion_doc_deleted` |
| **EC-157-06** | Rapid / duplicate citation clicks | P0 | Same `documentId` → update page/chunk props only; debounce optional | W2 | `companion_citation_idempotent` |
| **EC-157-07** | Multi-page chunk (`page_end > page_start`) | P1 | Navigate to `page_start`; badge shows span | W2 | `companion_page_span` |
| **EC-157-08** | Cross-workspace document id | P0 | Block fetch; honest error toast/pane | W2 | `companion_cross_workspace` |
| **EC-157-09** | Streaming while pane open | P0 | Do not remount shell; stick-to-bottom preserved if pinned | W2 | `companion_stream_stable` |
| **EC-157-10** | Switch / new conversation | P1 | Reset companion or restore per-message; clear stale doc | W2 | `companion_conversation_switch` |
| **EC-157-11** | Width starvation (sidebar+history+chat+pane) | P0 | Auto-rail history; clamp width; mins enforced | W1 | `companion_width_rail` |
| **EC-157-12** | Narrow viewport / mobile | P0 | Bottom sheet; state survives breakpoint cross | W1 | `companion_mobile_sheet` |
| **EC-157-13** | Back/forward, deep link, refresh | P0 | URL codec round-trip; bad params → closed | W1 | `companion_url_roundtrip` |
| **EC-157-14** | Persisted layout from other version | P2 | Versioned key; migrate or reset defaults | W1 | `companion_storage_migrate` |
| **EC-157-15** | Answer with 0 entities | P0 | Graph empty state; disable or explain Show on graph | W3 | `companion_graph_empty` |
| **EC-157-16** | Answer with 1 node | P1 | Focus single node; camera fit | W3 | `companion_graph_one` |
| **EC-157-17** | Answer with &gt; N nodes | P1 | Cap + “Show in Studio” | W3 | `companion_graph_cap` |
| **EC-157-18** | Entity ids missing after reload | P0 | Normalised name fallback; list unresolved | W3 | `companion_graph_name_fallback` |
| **EC-157-19** | Graph store clobber `/graph` | P0 | Isolated engine/store; e2e round-trip assertion | W3 | `companion_graph_isolation` |
| **EC-157-20** | WebGL unavailable / context lost | P1 | Fall back to `graph-as-table` | W3 | `companion_graph_webgl_fallback` |
| **EC-157-21** | Keyboard-only + SR | P0 | Focus move + return; splitter keys; live region | W1/W5 | `companion_a11y_keyboard` |
| **EC-157-22** | Modifier / middle-click | P0 | Still opens new tab via `buildCitationHref` | W2 | `companion_modifier_newtab` |
| **EC-157-23** | Esc inside PDF vs close pane | P1 | Esc closes companion when pane chrome focused; PDF internal Esc does not trap forever | W2 | `companion_esc_focus` |
| **EC-157-24** | Dark mode / reduced motion | P2 | Tokens + no motion when reduced | W5 | `companion_theme_motion` |
| **EC-157-25** | Locale en/fr/zh | P0 | Keys + locale-parity script | W5 | `companion_locale_parity` |
| **EC-157-26** | Feature flag off | P0 | Legacy `router.push` behaviour | W1 | `companion_flag_off` |
| **EC-157-27** | Non-stream chat subgraph null | P1 | Map from `context.entities`; optional API fix W5 | W3/W5 | `companion_nonstream_entities` |
| **EC-157-28** | Maximize companion then Esc | P2 | Restore prior split sizes | W1 | `companion_maximize_restore` |
| **EC-157-29** | History panel toggle with companion | P1 | No double-scroll / CLS; mins hold | W1 | `companion_history_toggle` |
| **EC-157-30** | Quote-to-ask without text layer | P2 | Hide/disable action (W4) | W4 | `companion_quote_no_textlayer` |
| **EC-157-31** | Auto scope chip vs user pin | P1 | Auto-added scope clears on close; user pin retained | W4 | `companion_scope_chip_lifecycle` |
| **EC-157-32** | Bypass / no-KG mode Show on graph | P1 | Hide or disable action when no entities | W3 | `companion_bypass_no_graph` |

## Severity legend

- **P0** — Must pass before wave merge.
- **P1** — Same wave or immediate follow-up; blocks release of feature flag on.
- **P2** — Hardening wave acceptable.

## ASCII — citation collision

```text
  click [3] --> openSource(docA, p12)
  click [3] --> openSource(docA, p12)  => props only (no remount)
  click [7] --> openSource(docB, p2)   => swap document (remount OK)
  Cmd-click [3] --> new tab /documents/... (pane unchanged)
```

## ASCII — isolation

```text
  /query EmbeddedAnswerGraph  --local engine-->  (destroyed on leave)
  /graph useGraphStore        --singleton------>  must equal pre-visit snapshot
```

Cross-ref: [10-e2e-test-matrix](10-e2e-test-matrix.md) · [09-implementation-plan](09-implementation-plan.md).

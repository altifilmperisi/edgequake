# 10 — E2E / test matrix

Parent: [README](README.md) · ECs: [08](08-edge-cases.md) · Plan: [09](09-implementation-plan.md)

## Tooling

| Layer | Tool | Location |
|-------|------|----------|
| Pure unit | Vitest | `src/lib/query/__tests__/*` |
| Hook/component | Vitest + jsdom (if available) | `src/**/__tests__/*` |
| Playwright mocked | `@playwright/test` + SPEC-155 helpers | `e2e/spec157/*` |
| Locale | `bun run test:locale-parity` | keys `query.companion.*` |
| Axe | `@axe-core/playwright` | companion open states |
| API (optional W5) | `cargo test -p edgequake-api --test spec027_api_contract` | if contract touched |

Reuse: `e2e/spec155/helpers/mock-api.ts`, `mock-chat-sse.ts`, `mock-pdf.ts`.

Suggested package script:

```bash
bun run test:e2e:spec157   # playwright test e2e/spec157 --project=mock-api
```

## Gate ↔ EC matrix

| Gate id | EC(s) | Type | Assert (summary) |
|---------|-------|------|------------------|
| `companion_url_roundtrip` | 13 | unit + e2e | encode/decode; refresh restores pane |
| `companion_storage_migrate` | 14 | unit | bad version → defaults |
| `companion_width_rail` | 11 | e2e | history railed when viewport tight + pane open |
| `companion_mobile_sheet` | 12 | e2e | sheet visible &lt;768; dismiss works |
| `companion_maximize_restore` | 28 | e2e | Esc restores sizes |
| `companion_history_toggle` | 29 | e2e | toggle history with pane; no crash |
| `companion_flag_off` | 26 | e2e | citation navigates to `/documents` |
| `companion_text_doc` | 01 | e2e | text source scrolls/highlights without page |
| `companion_pdf_404` | 02 | e2e | error UI + Retry |
| `companion_pdf_auth` | 03 | e2e/unit | auth failure path shown |
| `companion_pdf_no_remount` | 04 | unit/e2e | resize does not remount Document root |
| `companion_doc_deleted` | 05 | e2e | error after delete mock |
| `companion_citation_idempotent` | 06 | e2e | double-click same citation; single mount |
| `companion_page_span` | 07 | unit/e2e | badge + start page |
| `companion_cross_workspace` | 08 | e2e | error; no content |
| `companion_stream_stable` | 09 | e2e | stream completes with pane open |
| `companion_conversation_switch` | 10 | e2e | pane resets/restores correctly |
| `companion_modifier_newtab` | 22 | e2e | meta-click → popup/documents URL |
| `companion_esc_focus` | 23 | e2e | Esc closes; focus on trigger |
| `companion_graph_empty` | 15 | e2e | empty copy; no crash |
| `companion_graph_one` | 16 | e2e | one node focused |
| `companion_graph_cap` | 17 | unit/e2e | cap + Studio CTA |
| `companion_graph_name_fallback` | 18 | unit | names resolve without ids |
| `companion_graph_isolation` | 19 | e2e | `/graph` snapshot unchanged after Query |
| `companion_graph_webgl_fallback` | 20 | e2e | table alternative |
| `companion_bypass_no_graph` | 32 | e2e | action hidden/disabled |
| `companion_nonstream_entities` | 27 | unit/(api) | map from context; optional API |
| `companion_a11y_keyboard` | 21 | e2e+axe | tab order, splitter, axe serious=0 |
| `companion_theme_motion` | 24 | e2e | reduced motion class / no transform |
| `companion_locale_parity` | 25 | script | en/fr/zh keys present |
| `companion_quote_no_textlayer` | 30 | e2e | control absent/disabled |
| `companion_scope_chip_lifecycle` | 31 | e2e | auto vs pin |

## Suggested Playwright specs

```text
  e2e/spec157/
    companion-layout.spec.ts    # URL, sheet, rail, flag, maximize, history
    companion-source.spec.ts    # citations, PDF, text, stream, modifiers
    companion-graph.spec.ts     # show on graph, empty, isolation, fallback
    companion-a11y.spec.ts      # axe + keyboard
    companion-ai.spec.ts        # W4 scope + quote
```

## Example assertions (normative sketches)

### Citation stays on Query

```ts
await page.getByTestId("query-inline-citation-1").click();
await expect(page).toHaveURL(/\/query/);
await expect(page.getByTestId("query-companion-source")).toBeVisible();
```

### Isolation

```ts
// snapshot graph store via debug hook or UI marker on /graph
await page.goto("/query");
// open graph companion, interact, leave
await page.goto("/graph");
await expect(page.getByTestId("graph-isolation-ok")).toBeVisible();
```

## CI binding

| Wave | Required green |
|------|----------------|
| W0 | vitest companion-pane; document e2e smoke |
| W1 | layout spec |
| W2 | source spec P0 gates |
| W3 | graph spec P0 gates |
| W4 | ai spec |
| W5 | full matrix P0/P1 + axe + locale |
| W6 | all of the above |

## Coverage checklist

- [x] Every EC-157-01…32 appears in a gate row above
- [x] Every P0 has e2e or unit+e2e
- [x] Isolation explicitly tested
- [x] Flag-off path tested

Cross-ref: [11-cross-ref](11-cross-ref.md).

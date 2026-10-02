# 11 — E2E / test matrix

Parent: [README](README.md) · ECs: [09](09-edge-cases.md) · Plan: [10](10-implementation-plan.md)

## Harness principles (LAW-155-14)

1. **Mocked API Playwright project** for UI — no live Ollama/DB required for
   gates (seed fixtures). Live stack optional tagged `@live`.
2. **Deterministic graph fixtures** — 0 / 1 / 100 / 500 / 2k / hub generators
   shared by vitest + Playwright (`edgequake_webui/src/lib/fixtures/graph/`).
3. **One gate name per EC** — fail the wave if any mapped EC red.
4. **axe + visual + reduced-motion + forced-colors** projects from W2.
5. **Cargo e2e** for data-contract ECs 80–86.

## Tooling to add (W2)

| Tool | Version (2026-10) | Role |
|------|-------------------|------|
| `@axe-core/playwright` | 4.13.0 | Route a11y |
| Playwright `toHaveScreenshot` | 1.58+ | Visual baselines |
| vitest `jsdom` project | 4.x | Component tests |
| ESLint custom / oxlint | — | raw palette + sub-12px |

## Matrix (EC → gate → wave)

| EC | Gate id | Layer | Wave |
|----|---------|-------|------|
| 01 | `e2e_spec155_graph_empty` | PW mock | W4 |
| 02 | `graph_single_node` | PW/vitest | W4 |
| 03 | `graph_disconnected` | vitest layout | W4 |
| 04 | `graph_self_loop` | vitest engine | W4 |
| 05 | `graph_parallel_edges` | vitest+PW | W4 |
| 06 | `graph_hub_cap` | vitest bench | W5 |
| 07 | `graph_stream_2k` | PW mock | W4 |
| 08 | `graph_truncation_truth` | PW+unit | W4 |
| 09 | `graph_label_cjk` | PW | W5 |
| 10 | `graph_unknown_type` | unit colours | W5 |
| 11 | `graph_filter_no_rebuild` | PW engine id | W4 |
| 12 | `graph_theme_stable` | PW | W4 |
| 13 | `graph_expand_stable` | PW | W4 |
| 14 | `graph_ws_switch` | PW | W4 |
| 15 | `graph_time_filter` | unit+PW | W4 |
| 16 | `graph_dim_hover` | unit reducer | W5 |
| 17 | `graph_ego` | PW | W5 |
| 18 | `graph_path_none` | PW | W5 |
| 19 | `graph_lasso` | PW | W5 |
| 20 | `graph_saved_view_stale` | unit | W5 |
| 25 | `graph_export` | PW download | W4 |
| 26 | `graph_minimap` | unit+PW | W4 |
| 27 | `graph_webgl_fallback` | PW flag | W4 |
| 28 | `graph_reduced_motion` | PW project | W2/W4 |
| 30 | `graph_keyboard_roving` | PW | W4 |
| 31 | `graph_cb_palette` | unit | W5 |
| 40 | `shell_font` | PW computed style | W1 |
| 41 | `tokens_status_aa` | unit contrast | W1 |
| 42 | `lint_type_floor` | eslint CI | W2 |
| 43 | `a11y_focus` | axe | W2 |
| 44 | `empty_states` | PW visual | W1 |
| 45 | `cmdk_palette` | PW | W1 |
| 46 | `shortcuts_once` | vitest | W1 |
| 47 | `breadcrumb_paths` | PW | W1 |
| 48 | `mobile_workspace` | PW 375 | W7 |
| 49 | `not_found` | PW | W1 |
| 51 | `costs_period` | PW mock API | W7 |
| 52 | `wslug_auth` | PW | W7 |
| 53 | `dash_deeplink` | PW | W7 |
| 54 | `docs_srp` | size lint | W7 |
| 55 | `detail_errors` | PW | W7 |
| 56 | `pipeline_error` | PW | W7 |
| 57 | `settings_import` | PW | W7 |
| 58 | `knowledge_dz` | PW | W7 |
| 59 | `ws_delete_confirm` | PW | W7 |
| 60 | `login_autocomplete` | PW | W1 |
| 61 | `query_compose_while_stream` | PW mock | W7Q |
| 62 | `query_ime_enter` | PW mock | W7Q |
| 63 | `query_aria_live` | axe+PW | W7Q |
| 65 | `answer_on_graph` | PW | W6 |
| 90 | `query_stop_keeps_partial` | PW mock | W7Q |
| 91 | `query_inline_retry` | PW mock | W7Q |
| 92 | `query_stream_phases` | PW mock | W7Q |
| 93 | `query_jump_to_latest` | PW mock | W7Q |
| 94 | `query_regenerate_safe` | PW mock | W7Q |
| 95 | `query_citation_popover` | PW mock | W7Q |
| 96 | `query_sources_a11y` | axe+PW | W7Q |
| 97 | `query_citations_srp` | size lint | W7Q |
| 98 | `query_persisted_chunk_parity` | vitest | W7Q |
| 99 | `query_mode_menu` | PW mock | W7Q |
| 100 | `query_draft_shortcuts` | PW mock | W7Q |
| 101 | `query_empty_corpus` | PW mock | W7Q |
| 102 | `query_history_xl_dock` | PW 768/1280 | W7Q |
| 103 | `query_history_search_all` | PW mock | W7Q |
| 104 | `query_history_single_mount` | PW mock | W7Q |
| 105 | `query_history_no_token_rerender` | vitest | W7Q |
| 106 | `query_module_size` | size lint | W7Q |
| 107 | `e2e_spec155_chat_stage_events` | cargo | W7Q |
| 108 | `e2e_spec155_message_feedback` | cargo | W7Q |
| 109 | `e2e_spec155_chat_abort_partial` | cargo | W7Q |
| 70 | `lang_switch` | PW | W1 |
| 71 | `html_lang` | PW | W1 |
| 72 | `locale_parity` | node script CI | W2/W8 |
| 73 | `rtl_prep` | eslint | W8 |
| 74 | `a11y_forced` | PW project | W2 |
| 75 | `connection_global` | PW | W7 |
| 76 | `axe_routes` | PW a11y | W2 |
| 77 | `vitest_jsdom` | vitest | W2 |
| 80 | `e2e_spec155_graph_totals_workspace` | cargo | W3 |
| 81 | `e2e_spec155_stream_start_node` | cargo | W3 |
| 82 | `e2e_spec155_degrees_tenant` | cargo | W3 |
| 83 | `e2e_spec155_degree_shape` | cargo | W3 |
| 84 | `e2e_spec155_degree_shape` | cargo | W3 |
| 85 | `e2e_spec155_edge_id_multigraph` | cargo | W3 |
| 86 | `e2e_spec155_communities_endpoint` | cargo | W3 |
| 87 | `e2e_spec155_document_scope_truncation` | cargo | med |

## Playwright projects (target config)

```text
  chromium          # default
  a11y              # axe on route list × light/dark × 375/768/1280
  visual            # toHaveScreenshot baselines
  reduced-motion    # emulate reducedMotion
  forced-colors     # emulate forcedColors
  mock-api          # MSW or route.fulfill graph/query fixtures
```

## Vitest projects

```text
  unit-node     # existing
  unit-jsdom    # GraphEngine reducers, PageHeader, StatusBadge, shortcuts
  bench         # filter applyDelta timing budgets
```

## Wave DoD test commands

```bash
# W1
cd edgequake_webui && pnpm exec playwright test cmdk_palette shell_font login_autocomplete lang_switch not_found

# W2
pnpm exec playwright test --project=a11y --project=visual --project=forced-colors
bun run test --project=unit-jsdom
node scripts/locale-parity.mjs

# W3
cargo test -p edgequake-api --test e2e_spec155_graph_totals_workspace
cargo test -p edgequake-api --test e2e_spec155_stream_start_node
cargo test -p edgequake-api --test e2e_spec155_degrees_tenant
cargo test -p edgequake-api --test e2e_spec155_degree_shape
cargo test -p edgequake-api --test e2e_spec155_edge_id_multigraph
cargo test -p edgequake-api --test e2e_spec155_communities_endpoint

# W4–W6
pnpm exec playwright test graph_filter_no_rebuild graph_truncation_truth graph_export answer_on_graph

# W7–W9
pnpm exec playwright test costs_period wslug_auth query_ime_enter
make release-gates   # after wiring SPEC-155 into release script
```

## Non-regression

Keep green: SPEC-099 docs specs, SPEC-100 CLS, SPEC-101 wizard, SPEC-102
colours, SPEC-154 auth webui, existing graph-responsive (rewrite flaky
worker tests).

Cross-ref: [12-cross-ref](12-cross-ref.md).

# 09 — Edge cases (EC-155-*)

Parent: [README](README.md) · Findings: [04](04-findings.md) · Next: [10-implementation-plan](10-implementation-plan.md)

Each EC has mitigation + named gate (see [11-e2e-test-matrix](11-e2e-test-matrix.md)).

## Graph scale & structure

| EC | Scenario | Mitigation | Gate |
|----|----------|------------|------|
| EC-155-01 | Empty workspace graph | EmptyState + CTA upload; no WebGL error | `e2e_spec155_graph_empty` |
| EC-155-02 | Single node, no edges | Layout stable; details work | `graph_single_node` |
| EC-155-03 | Disconnected components | FA2 still converges; fit camera to all | `graph_disconnected` |
| EC-155-04 | Self-loop | Render or explicitly hide with legend note | `graph_self_loop` |
| EC-155-05 | Parallel edges (multi-rel) | Multigraph + curvature; both selectable | `graph_parallel_edges` |
| EC-155-06 | Hub node ≥5k incident | Per-hop cap; LOD; no UI freeze | `graph_hub_cap` |
| EC-155-07 | 2k nodes streamed | applyDelta; no full rebuild; 60fps pan | `graph_stream_2k` |
| EC-155-08 | Truncation at max_nodes | Banner truthful; Load more appends | `graph_truncation_truth` |
| EC-155-09 | Very long / CJK labels | Truncate + tooltip; font metrics | `graph_label_cjk` |
| EC-155-10 | Unknown entity type | Fallback token colour + shape | `graph_unknown_type` |

## Graph interaction / render

| EC | Scenario | Mitigation | Gate |
|----|----------|------------|------|
| EC-155-11 | Filter keystrokes | Debounce; dim; no Sigma.kill | `graph_filter_no_rebuild` |
| EC-155-12 | Theme toggle mid-session | setTheme; positions kept | `graph_theme_stable` |
| EC-155-13 | Expand then filter | Positions for old nodes kept | `graph_expand_stable` |
| EC-155-14 | Workspace switch mid-stream | Abort stream; clear engine; no leak | `graph_ws_switch` |
| EC-155-15 | Time filter range | Filters nodes by timestamp when present; empty honest | `graph_time_filter` |
| EC-155-16 | Hover focus | Dim not hide; announcer | `graph_dim_hover` |
| EC-155-17 | Ego depth 3 | Cap + performance | `graph_ego` |
| EC-155-18 | Shortest path none | Toast/inline “no path” | `graph_path_none` |
| EC-155-19 | Lasso zero / many | Selection state sync table | `graph_lasso` |
| EC-155-20 | Saved view stale node | Skip missing; warn | `graph_saved_view_stale` |
| EC-155-25 | PNG/SVG export | Non-empty; theme bg | `graph_export` |
| EC-155-26 | Minimap click navigate | Camera centres correctly non-square | `graph_minimap` |
| EC-155-27 | WebGL unavailable | ErrorState + table alt | `graph_webgl_fallback` |
| EC-155-28 | Reduced motion | No layout tween / camera animate | `graph_reduced_motion` |
| EC-155-30 | Keyboard: Tab leaves canvas | No window preventDefault Tab | `graph_keyboard_roving` |
| EC-155-31 | Colour-blind simulation | Distinct shapes/tokens | `graph_cb_palette` |

## Input / query

| EC | Scenario | Mitigation | Gate |
|----|----------|------------|------|
| EC-155-61 | Stream while typing next Q | Textarea enabled; Stop | `query_compose_while_stream` |
| EC-155-62 | IME composition Enter | Ignore Enter when composing | `query_ime_enter` |
| EC-155-63 | Screen reader during tokens | No flood; status updates throttled | `query_aria_live` |
| EC-155-65 | Show on graph from answer | Highlight ≥1 node | `answer_on_graph` |

## A11y / i18n / shell

| EC | Scenario | Mitigation | Gate |
|----|----------|------------|------|
| EC-155-40 | Font load | Computed font-family contains Geist | `shell_font` |
| EC-155-41 | Status badge contrast | axe + contrast sample | `tokens_status_aa` |
| EC-155-42 | No sub-12px in chrome | lint + axe | `lint_type_floor` |
| EC-155-43 | Focus visible | axe focusable | `a11y_focus` |
| EC-155-44 | EmptyState used | visual routes | `empty_states` |
| EC-155-45 | Cmd+K opens palette | playwright | `cmdk_palette` |
| EC-155-46 | Single keydown listener | unit assert | `shortcuts_once` |
| EC-155-47 | Breadcrumb all sections | e2e | `breadcrumb_paths` |
| EC-155-48 | Mobile workspace visible | viewport e2e | `mobile_workspace` |
| EC-155-49 | 404 page | not-found | `not_found` |
| EC-155-60 | Login autocomplete | DOM attrs | `login_autocomplete` |
| EC-155-70 | Language switch applies | i18n + html lang | `lang_switch` |
| EC-155-71 | html lang tracks | mutation | `html_lang` |
| EC-155-72 | Locale parity | CI script | `locale_parity` |
| EC-155-73 | Logical properties sample | lint | `rtl_prep` |
| EC-155-74 | forced-colors / contrast | playwright projects | `a11y_forced` |
| EC-155-75 | Offline banner global | e2e mock offline | `connection_global` |
| EC-155-76 | axe clean routes | playwright a11y | `axe_routes` |
| EC-155-77 | Component DOM tests | vitest jsdom | `vitest_jsdom` |

## Pages / network / tenancy

| EC | Scenario | Mitigation | Gate |
|----|----------|------------|------|
| EC-155-51 | Costs period changes totals | API assert | `costs_period` |
| EC-155-52 | `/w/slug` unauth redirect | e2e | `wslug_auth` |
| EC-155-53 | Dashboard doc deep link | opens detail/preview | `dash_deeplink` |
| EC-155-54 | Document manager modular | file size lint | `docs_srp` |
| EC-155-55 | Detail error ≠ always 404 | mock 500 | `detail_errors` |
| EC-155-56 | Pipeline error banner | mock fail | `pipeline_error` |
| EC-155-57 | Settings import keyboard | tab+enter | `settings_import` |
| EC-155-58 | Knowledge dropzone keyboard | role+key | `knowledge_dz` |
| EC-155-59 | Workspace typed delete | confirm | `ws_delete_confirm` |
| EC-155-80 | Totals workspace-scoped | cargo | `e2e_spec155_graph_totals_workspace` |
| EC-155-81 | Stream start_node | cargo | `e2e_spec155_stream_start_node` |
| EC-155-82 | Degrees tenant isolation | cargo | `e2e_spec155_degrees_tenant` |
| EC-155-83 | start_node degrees | cargo | `e2e_spec155_degree_shape` |
| EC-155-84 | Degree in/out/total | cargo | (same) |
| EC-155-85 | Multigraph edge ids | cargo | `e2e_spec155_edge_id_multigraph` |
| EC-155-86 | Communities endpoint | cargo | `e2e_spec155_communities_endpoint` |
| EC-155-87 | Document scope truncation flag | cargo/API | medium wave |

## Cross-cutting matrix note

```text
  Empty ──► EC-01,44
  Scale ──► EC-06,07,08
  Honesty ► EC-08,15,51,80
  A11y   ──► EC-28,30,31,43,60,74,76
  i18n   ──► EC-62,70–73
  Security► EC-52,82
```

Cross-ref: [11-e2e-test-matrix](11-e2e-test-matrix.md) · [12-cross-ref](12-cross-ref.md).

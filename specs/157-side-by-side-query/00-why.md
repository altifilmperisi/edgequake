# 00 — WHY (5-WHY)

Parent: [README](README.md) · Next: [01-first-principles](01-first-principles.md)

## The job to be done

An analyst using EdgeQuake Query must **verify an answer without leaving the
conversation**. Grounding is only useful if checking a citation or the answer’s
entities costs seconds, not a context switch.

Industry bar (2025–2026): claim-attached citations, in-place preview, and
split-screen source viewers (Multigrid citation UX, agentic source-anchoring
patterns, WAI-ARIA window splitter). EdgeQuake already stores page/chunk
attribution and answer subgraphs — the gap is the **surface**, not the data.

## Five WHYs

```text
WHY-1  Users cannot verify an answer without losing the conversation
  WHY? Citation chips call router.push → /documents/{id}?page&chunk
       (citation-popover.tsx → buildCitationHref).
  WHY? "Show on graph" calls router.push → /graph?answerMessage&focus=answer
       (assistant-message.tsx → useAnswerGraphStore then navigate).
  WHY? QueryInterface has no companion slot — only chat + history panel.
  WHY? Prior specs shipped full-page viewers (SPEC-002 / 033 / 143) and
       answer-on-graph deep links (SPEC-155 LAW-155-11), not in-place panes.
  --> ROOT: Verify-away-from-chat is the default path (LAW-157-1)

WHY-2  Switching surfaces destroys working memory
  WHY? Leaving /query unmounts scroll position, composer draft focus, and
       streaming stick-to-bottom context.
  WHY? Returning requires history / back and re-finding the message.
  WHY? No addressable companion state on /query itself.
  --> ROOT: Chat is not treated as the primary durable surface (LAW-157-2,3)

WHY-3  The same intent is implemented three different ways
  WHY? InlineCitation, DocumentsTab, KnowledgeTab, PassageRow each navigate
       independently (or via onDocumentClick → same router.push).
  WHY? Document detail owns URL/chunk/page resolution in page.tsx; Query
       cannot reuse it without copying.
  WHY? Two answer-graph stores exist (use-answer-graph-store used;
       use-answer-on-graph-store unused / parallel).
  --> ROOT: No single entry point per intent — DRY broken (LAW-157-4,10)

WHY-4  Embedding Graph Studio wholesale would break Query and /graph
  WHY? useGraphStore is a singleton (nodes, edges, sigmaInstance, engineFocus).
  WHY? GraphViewer is ~1200 lines, URL-coupled, 100vh drawers, own chrome.
  WHY? Answer evidence is a small subgraph; full Studio filters are noise.
  --> ROOT: Wrong abstraction for "why this answer?" (LAW-157-8,9)

WHY-5  Width and a11y were never designed for three columns
  WHY? Sidebar (256) + history (280) + chat + companion (~440) ≈ 1400px.
  WHY? No auto-rail / sheet strategy when companion opens.
  WHY? Splitter / focus return / live-region behaviour not specified for Query.
  --> ROOT: Missing layout + a11y contract (LAW-157-6,7,11)
```

## Causal ASCII — verify-in-place chain

```text
  Retrieval context (chunks + entities + pages)
           |
           v
  +--------------------+     click / shortcut
  | Query chat (SSOT)  | --------------------+
  +--------------------+                     |
           ^                                 v
           |                     +-----------------------+
           | Esc / Close         | Companion pane        |
           +---------------------|  Source | Graph tabs  |
                                 +-----------------------+
                                      |            |
                                      v            v
                               DocumentSource   Answer subgraph
                               View (PDF/text)  (GraphRenderer)
                                      |            |
                                      +-----+------+
                                            |
                                   URL codec (pane=…)
                                            |
                                   Share / refresh / back
```

## What “good” looks like vs today

| Capability | Today | Target (SPEC-157) |
|------------|-------|-------------------|
| Citation click | Navigates to `/documents/[id]` | Opens Source companion; chat stays |
| Show on graph | Navigates to `/graph` | Opens Graph companion with answer focus |
| Modifier / middle-click | New tab | Unchanged (escape hatch) |
| Deep link restore | Document / graph only | `/query?pane=…` restores companion |
| Answer subgraph | Store + full Graph Studio | Compact embedded pane + Studio link |
| Width conflict | N/A | History auto-rail; sheet &lt; md |
| Focus return | Lost | Returns to citation / action trigger |

## Non-goals (from WHY)

- Replacing full-page document and graph routes.
- Three-pane simultaneous display.
- Full Graph Studio chrome inside Query.

## Success narrative

When this why is closed:

1. An analyst verifies a citation in &lt; 3 seconds without losing the thread.
2. The same analyst inspects answer entities beside the prose that cited them.
3. Sharing a Query URL with `pane=` reproduces the verification context.
4. CI proves the edge cases (EC-157) and a11y gates.

Cross-ref: [01-first-principles](01-first-principles.md) · [02-surfaces](02-surfaces.md) ·
[08-edge-cases](08-edge-cases.md).

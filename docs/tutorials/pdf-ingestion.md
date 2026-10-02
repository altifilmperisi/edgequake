---
title: 'PDF Ingestion Tutorial'
---

# PDF Ingestion Tutorial

> **Product: v0.23.0** · Contract: [`openapi.snapshot.json`](../../edgequake_webui/openapi/openapi.snapshot.json) · Spec ops: [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md)

EdgeQuake converts PDFs to Markdown (vision LLM or EdgeParse), then ingests the result into the knowledge graph. This tutorial covers upload, progress, cancel, and query using the **lawful API** (OpenAPI snapshot is SSOT).

**Prerequisites**: EdgeQuake running (API `:8080`, WebUI `:3000`). See [Quick Start](/docs/getting-started/quick-start/).

**Time**: ~20 minutes

---

## Endpoint cheat sheet

| Goal | Method | Endpoint | Body |
| ---- | ------ | -------- | ---- |
| Upload PDF (preferred) | POST | `/api/v1/documents/pdf` | `multipart/form-data` |
| Upload any file (incl. PDF) | POST | `/api/v1/documents/upload` | `multipart/form-data` |
| Upload plain text | POST | `/api/v1/documents` | `application/json` only |
| Poll progress | GET | `/api/v1/documents/pdf/progress/{task_id}` | — |
| Cancel | POST | `/api/v1/tasks/{task_id}/cancel` | — |
| Query | POST | `/api/v1/query` | JSON |

**Do not** send `multipart/form-data` to `POST /api/v1/documents` — that route accepts JSON text content only.

---

## Two phases: convert ≠ ingest (SPEC-057)

PDF admission enqueues **convert** (`TaskType::PdfProcessing`). After durable markdown is stored and the PDF row is `Completed`, the worker enqueues a separate **ingest** task (`TaskType::Insert`).

```
┌───────────────────────────────────────────────────────┐
│ Convert then ingest (SPEC-057)                        │
│                                                       │
│  POST /documents/pdf  -->  admit task_id              │
│              |                                        │
│              v                                        │
│  [1] PdfProcessing (convert only)                     │
│      vision / edgeparse --> markdown                  │
│      PDF row --> Completed (artifact)                 │
│              |                                        │
│              v  markdown barrier                      │
│  [2] Insert (KG ingest, new lease)                    │
│      chunk --> extract --> embed --> store            │
│              |                                        │
│              v                                        │
│  document display_status = completed                  │
└───────────────────────────────────────────────────────┘
```

- PDF `Completed` means **convert finished** — the document may still be `extracting` or `embedding`.
- Terminal success for querying: document `display_status` = **`completed`** (not `indexed`).
- Cancel during convert **or** ingest cancels both linked tasks for the same `pdf_id`. See [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md).

---

## Step 1: Upload a PDF

### Default (vision backend)

```bash
curl -X POST http://localhost:8080/api/v1/documents/pdf \
  -H "X-Workspace-ID: default" \
  -F "file=@/path/to/paper.pdf" \
  -F "title=Research Paper"
```

Equivalent generic upload endpoint:

```bash
curl -X POST http://localhost:8080/api/v1/documents/pdf \
  -H "X-Workspace-ID: default" \
  -F "file=@/path/to/paper.pdf" \
  -F "title=Research Paper"
```

**Response** (fields that matter):

```json
{
  "pdf_id": "abc-123",
  "document_id": null,
  "status": "processing",
  "task_id": "pdf-550e8400-e29b-41d4-a716-446655440000",
  "track_id": null,
  "message": "PDF uploaded and processing started",
  "estimated_time_seconds": 120
}
```

| Field | Use |
| ----- | --- |
| `task_id` | **Authoritative** progress/cancel identity — subscribe and cancel with this |
| `track_id` | Optional client correlation echo only — **not** the progress-store key |
| `pdf_id` | PDF row; use for content/cancel-via-PDF routes |
| `document_id` | Populated after ingest creates the doc row |

---

## Step 2: Parser backend (`pdf_parser_backend`)

Runtime backends (see `PdfParserBackend` in `edgequake-pdf`):

| Value | Behavior |
| ----- | -------- |
| `vision` (default) | Render pages → vision LLM markdown (`EDGEQUAKE_VISION_PROVIDER` / `EDGEQUAKE_VISION_MODEL`) |
| `edgeparse` | CPU EdgeParse born-digital extraction (no Tesseract) |
| `edgeparse-ocr` | EdgeParse + Tesseract raster-table OCR |
| `auto` | Vision start; SPEC-038 may fast-path EdgeParse when text density allows |

Per-upload override:

```bash
curl -X POST http://localhost:8080/api/v1/documents/pdf \
  -H "X-Workspace-ID: default" \
  -F "file=@scanned.pdf" \
  -F "title=Scanned Book" \
  -F "pdf_parser_backend=vision" \
  -F "enable_vision=true" \
  -F "vision_provider=ollama" \
  -F "vision_model=gemma4:latest"
```

Global default: `EDGEQUAKE_PDF_PARSER_BACKEND=vision|edgeparse|edgeparse-ocr|auto`.

Vision provider resolution chain: per-request fields → `EDGEQUAKE_VISION_*` env → LLM defaults. Mismatch diagnostics: `GET /api/v1/config/effective`. Details: [FAQ — vision configuration](/docs/faq/#how-does-edgequake-decide-which-vision-provider-and-model-to-use).

---

## Step 3: Track progress

### HTTP poll

```bash
TASK_ID="pdf-550e8400-e29b-41d4-a716-446655440000"

curl -s "http://localhost:8080/api/v1/documents/pdf/progress/${TASK_ID}" \
  -H "X-Workspace-ID: default" | jq .
```

SSE variant: `GET /api/v1/documents/pdf/progress/stream/{task_id}`.

### WebSocket

Connect to the pipeline WebSocket (see OpenAPI / [Pipeline Progress](/docs/deep-dives/pipeline-progress/)). Use **`task_id`** from the upload response — not a client-supplied `track_id` unless you only need batch correlation.

### Document status (UI badges)

List/detail JSON includes SPEC-057 presentation fields:

| Field | Meaning |
| ----- | ------- |
| `display_status` | Badge key: `converting`, `extracting`, `embedding`, `completed`, `failed`, **`cancelled`**, … |
| `ui_phase` | `idle` \| `running` \| `stopping` \| `terminal` — show **Stopping…** when `stopping` |

Prefer `display_status` over re-deriving from raw `status` / `current_stage`.

```bash
curl -s "http://localhost:8080/api/v1/documents?workspace_id=default" \
  -H "X-Workspace-ID: default" | jq '.documents[] | {id, display_status, ui_phase}'
```

**Ready to query** when `display_status` is **`completed`**.

---

## Step 4: Cancel (optional)

Canonical cancel:

```bash
curl -X POST "http://localhost:8080/api/v1/tasks/${TASK_ID}/cancel" \
  -H "X-Workspace-ID: default"
```

Also supported: `DELETE /api/v1/documents/pdf/{pdf_id}/cancel`, WebSocket `{ "type": "cancel", "track_id": "..." }` (uses same SSOT). Cancel is **cooperative** — expect a short delay until the in-flight LLM/vision call aborts.

Terminal cancel: `display_status=cancelled`, `ui_phase=terminal`. PDF cancel maps to `PdfProcessingStatus::Cancelled` (**not** `Failed`).

Full semantics: [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md).

---

## Step 5: Query the content

After `display_status: completed`:

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: default" \
  -d '{
    "query": "What are the key findings?",
    "mode": "hybrid",
    "top_k": 10
  }'
```

**QueryResponse** (no top-level `chunks` / `entities`):

```json
{
  "answer": "The key findings show that…",
  "sources": [
    {
      "document_id": "doc-uuid",
      "snippet": "The results demonstrate…",
      "score": 0.94,
      "file_path": "Research Paper.pdf"
    }
  ],
  "mode": "hybrid",
  "stats": {
    "total_time_ms": 1200,
    "retrieval_time_ms": 400,
    "generation_time_ms": 800
  }
}
```

Prefer the official SDK: `pip install edgequake-sdk` — see [Python SDK](/docs/sdks/python/).

---

## Configuration reference (multipart fields)

| Field | Type | Description |
| ----- | ---- | ----------- |
| `file` | file | Required PDF bytes |
| `title` | string | Display title |
| `metadata` | JSON string | Custom metadata object |
| `enable_vision` | bool | Default `true` for vision path |
| `vision_provider` | string | Override vision LLM provider |
| `vision_model` | string | Override vision model |
| `pdf_parser_backend` | `vision` \| `edgeparse` \| `edgeparse-ocr` \| `auto` | Parser backend |
| `process_options` | string | Multimodal process options tag |
| `force_reindex` | bool | Re-process duplicate checksum |
| `track_id` | string | Client batch correlation only |

Legacy `config={"mode":"Vision",…}` on `/documents/upload` may still appear in older examples; v0.23.0 PDF path uses **`pdf_parser_backend`** + vision env/per-request fields as SSOT.

---

## Troubleshooting

| Symptom | Check | Fix |
| ------- | ----- | --- |
| 400 on upload to `/documents` | Wrong content-type | Use `/documents/pdf` or `/documents/upload` with multipart |
| Stuck on `converting` | Vision provider down | `curl http://localhost:11434/api/tags` or set `pdf_parser_backend=edgeparse` |
| Vision errors / empty markdown | Provider/model mismatch | `GET /api/v1/config/effective` → Vision area |
| `display_status: failed` | Backend logs | `/tmp/edgequake-backend.log`; re-upload or retry |
| Cancel shows `stopping` long | Cooperative abort | Normal — wait for terminal `cancelled` |
| Query returns generic answer | Doc not `completed` | Poll until `display_status=completed` |

More: [PDF Processing Deep Dive](/docs/deep-dives/pdf-processing/) · [Common Issues](/docs/troubleshooting/common-issues/#pdf-extraction-issues)

---

## Next steps

1. [Document Ingestion](/docs/tutorials/document-ingestion/) — text upload and pipeline stages
2. [Pipeline Progress](/docs/deep-dives/pipeline-progress/) — WebSocket/SSE details
3. [Document Upload Quick Reference](/docs/api-reference/document-upload-quick-reference/) — all upload endpoints
4. [REST API Reference](/docs/api-reference/rest-api/) — full contract

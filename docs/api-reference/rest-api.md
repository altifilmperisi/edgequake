---
title: "EdgeQuake REST API Reference"
---

# EdgeQuake REST API Reference

> **Product: v0.23.0** · Contract: [`openapi.snapshot.json`](../../edgequake_webui/openapi/openapi.snapshot.json) · Spec ops: [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md)

> **Base URL**: `http://localhost:8080` (API under `/api/v1`)  
> **Interactive docs**: `/swagger-ui/` when the backend is running

This page is a **guided overlay** for v0.23.0. For the full endpoint catalog, request/response schemas, and Try-it-out, use OpenAPI — it is regenerated on every release and matches the running server.

---

## v0.23.0 quick reference

### Authentication

Most `/api/v1/*` routes require:

- `Authorization: Bearer <JWT>` (from `POST /api/v1/auth/login`), **or**
- `X-API-Key: <key>` (from `POST /api/v1/api-keys`)

Multi-tenant context (required for document/query operations):

| Header | Purpose |
| ------ | ------- |
| `X-Tenant-ID` | Tenant UUID |
| `X-Workspace-ID` | Workspace UUID |

Public (no auth): `/health`, `/ready`, `/live`, `/swagger-ui/*`, `/api-docs/*`.

### Async admission model

Uploads are **accepted asynchronously** — they return `202 Accepted` with a `task_id`, not a synchronous `completed` body.

| Upload | Endpoint | Progress key |
| ------ | -------- | ------------ |
| File (PDF/TXT/MD) | `POST /api/v1/documents/upload` | `FileUploadResponse.task_id` |
| PDF (vision pipeline) | `POST /api/v1/documents/pdf` | `PdfUploadResponse.task_id` |
| JSON text | `POST /api/v1/documents` | track via list/detail `track_id` |

PDF flow is **convert then ingest** (two tasks). See [Pipeline Progress](/docs/deep-dives/pipeline-progress/).

### Status presentation (SPEC-057 P4)

Document list/detail includes `display_status` and `ui_phase` from `IngestionStatusMapper`. **Use these for UI badges** instead of re-deriving from raw `status`.

| `ui_phase` | UI behavior |
| ---------- | ----------- |
| `idle` | Queued / not yet running |
| `running` | Active stage (`display_status`: `converting`, `extracting`, …) |
| `stopping` | Cancel in flight — show **"Stopping…"** even if stage unchanged |
| `terminal` | Done (`completed`, `failed`, `cancelled`, …) |

### Progress, cancel, delete

| Action | Endpoint |
| ------ | -------- |
| Ingest progress (poll) | `GET /api/v1/ingestion/{track_id}/progress` |
| Ingest progress (batch) | `POST /api/v1/ingestion/progress` |
| PDF progress (poll) | `GET /api/v1/documents/pdf/progress/{track_id}` |
| PDF progress (SSE) | `GET /api/v1/documents/pdf/progress/stream/{track_id}` |
| Global WS | `ws://localhost:8080/ws/pipeline/progress` |
| Per-track WS | `ws://localhost:8080/ws/progress/{track_id}` |
| Cancel task | `POST /api/v1/tasks/{track_id}/cancel` |
| PDF cancel | `DELETE /api/v1/documents/pdf/{pdf_id}/cancel` |
| Delete impact preview | `GET /api/v1/documents/{document_id}/deletion-impact` |
| Queue metrics | `GET /api/v1/pipeline/queue-metrics` |

Deletion broadcasts phase events on `/ws/pipeline/progress` (SPEC-050). Details: [Pipeline Progress](/docs/deep-dives/pipeline-progress/) and [Ingestion cancel & fairness](/docs/ingestion-cancel-and-fairness.md).

> **Removed paths:** `/api/v1/rag/upload`, `/api/v1/rag/progress/*` — do not use.

### Entity routes

Entities live under **`/api/v1/graph/entities`**, not `/api/v1/entities`:

- `GET /api/v1/graph/entities` — list
- `GET /api/v1/graph/entities/{entity_name}` — detail
- Provenance: `GET /api/v1/entities/{entity_id}/provenance` (separate lineage route)

### Cost endpoints

- `GET /api/v1/pipeline/costs/pricing`
- `GET /api/v1/costs/summary`, `/costs/history`, `/costs/budget`

See [Cost Tracking](/docs/deep-dives/cost-tracking/).

---

## Table of Contents

- [Authentication](#authentication)
- [Health & Diagnostics](#health--diagnostics)
- [Documents API](#documents-api)
- [Parse API](#parse-api-spec-094)
- [Query API](#query-api)
- [Chat API](#chat-api)
- [Graph API](#graph-api)
- [Workspaces API](#workspaces-api)
- [Knowledge Injection API](#knowledge-injection-api)
- [Conversations API](#conversations-api)
- [Models & Settings](#models--settings)
- [Error Handling](#error-handling)
- [Rate Limiting](#rate-limiting)

---

## Authentication

EdgeQuake supports two authentication methods:

### API Key Authentication

Include your API key in the `X-API-Key` header:

```bash
curl -H "X-API-Key: your-api-key" \
     http://localhost:8080/api/v1/documents
```

### Bearer Token Authentication

Use `Authorization: Bearer` header:

```bash
curl -H "Authorization: Bearer your-api-key" \
     http://localhost:8080/api/v1/documents
```

### Multi-Tenant Headers

For multi-tenant deployments, include workspace context:

| Header           | Description                 | Required                         |
| ---------------- | --------------------------- | -------------------------------- |
| `X-Tenant-ID`    | Tenant identifier (UUID)    | Required for multi-tenant        |
| `X-Workspace-ID` | Workspace identifier (UUID) | Required for workspace isolation |

```bash
curl -H "X-API-Key: your-key" \
     -H "X-Tenant-ID: tenant-uuid" \
     -H "X-Workspace-ID: workspace-uuid" \
     http://localhost:8080/api/v1/documents
```

### Public Endpoints (No Auth Required)

- `GET /health`
- `GET /ready`
- `GET /live`
- `GET /swagger-ui/*`
- `GET /api-docs/*`

---

## Health & Diagnostics

### GET /health

Deep health check with component status for monitoring dashboards.

**Response**:

```json
{
  "status": "healthy",
  "version": "0.23.0",
  "storage_mode": "postgresql",
  "workspace_id": "default",
  "components": {
    "kv_storage": true,
    "vector_storage": true,
    "graph_storage": true,
    "llm_provider": true
  },
  "llm_provider_name": "ollama",
  "attribution": {
    "app_id": "edgequake",
    "app_name": "EdgeQuake",
    "active": true
  },
  "schema": {
    "latest_version": 20240115001,
    "migrations_applied": 12,
    "last_applied_at": "2024-01-15T10:30:00Z"
  }
}
```

### GET /ready

Kubernetes readiness probe. Returns 200 if service can accept traffic.

```bash
curl http://localhost:8080/ready
# Response: 200 OK
```

### GET /live

Kubernetes liveness probe. Returns 200 if process is alive.

```bash
curl http://localhost:8080/live
# Response: 200 OK
```

---

## Documents API

Document ingestion with automatic entity extraction and knowledge graph construction.

### POST /api/v1/documents

Upload document content as JSON text.

**Text Upload (JSON)**:

```bash
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: workspace-uuid" \
  -d '{
    "content": "Your document text here...",
    "title": "Document Title",
    "source": "manual_entry"
  }'
```

### POST /api/v1/documents/upload

Upload a file (PDF, TXT, MD, JSON) via multipart form data.

**File Upload (Multipart)**:

```bash
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -H "X-Workspace-ID: workspace-uuid" \
  -F "file=@document.pdf" \
  -F "title=My PDF Document"
```

**Supported File Types**:

| Extension | MIME Type        | Max Size |
| --------- | ---------------- | -------- |
| `.pdf`    | application/pdf  | 50 MB    |
| `.txt`    | text/plain       | 10 MB    |
| `.md`     | text/markdown    | 10 MB    |
| `.json`   | application/json | 10 MB    |

**Response** (`202 Accepted` — async processing):

```json
{
  "document_id": "doc-uuid",
  "filename": "document.pdf",
  "size": 1024000,
  "content_hash": "sha256:...",
  "status": "pending",
  "chunk_count": 0,
  "entity_count": 0,
  "relationship_count": 0,
  "is_duplicate": false,
  "task_id": "insert-uuid"
}
```

Poll progress via `GET /api/v1/ingestion/{task_id}/progress` or subscribe on `ws://localhost:8080/ws/progress/{task_id}`. When complete, list/detail includes `display_status` and `ui_phase`.

**PDF upload** — use `POST /api/v1/documents/pdf` instead; returns `PdfUploadResponse` with `task_id` (progress key) and optional client `track_id` (correlation only).

### GET /api/v1/documents

List all documents in the workspace.

**Query Parameters**:

| Parameter          | Type    | Default | Description                                                          |
| ------------------ | ------- | ------- | -------------------------------------------------------------------- |
| `limit`            | integer | 50      | Max documents to return                                              |
| `offset`           | integer | 0       | Pagination offset                                                    |
| `status`           | string  | all     | Filter by status (processing, completed, failed)                     |
| `date_from`        | string  | null    | ISO 8601 date. Only include documents created on or after this date  |
| `date_to`          | string  | null    | ISO 8601 date. Only include documents created on or before this date |
| `document_pattern` | string  | null    | Comma-separated title search terms (case-insensitive, OR logic)      |

```bash
curl http://localhost:8080/api/v1/documents?limit=10&status=completed \
  -H "X-Workspace-ID: workspace-uuid"
```

**Response**:

```json
{
  "documents": [
    {
      "id": "doc-uuid-1",
      "title": "Document 1",
      "status": "completed",
      "display_status": "completed",
      "ui_phase": "terminal",
      "chunk_count": 15,
      "created_at": "2024-01-15T10:30:00Z"
    }
  ],
  "total": 42,
  "limit": 10,
  "offset": 0
}
```

The relational scan does not read `documents.content` (preview and length come from metadata). It is capped (`LIMIT`, same envelope as the KV list) and runs under `SET LOCAL statement_timeout`.

**503** `read_path_busy` when the interactive deadline is spent (default 2.5s, `EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS`). Same code on document detail, `GET /api/v1/documents/search`, `GET /api/v1/tenants`, and workspace list. Body is `application/problem+json`:

```json
{
  "code": "read_path_busy",
  "message": "Read path busy",
  "title": "Read Path Busy",
  "status": 503,
  "type": "https://edgequake.dev/problems/read-path-busy",
  "details": {
    "reason": "work_deadline",
    "retry_after_ms": 2500,
    "retryable": true
  }
}
```

`reason` is `permit_wait` (bulkhead queue), `permit_closed`, or `work_deadline` (handler exceeded the shared budget). `Retry-After` is `retry_after_ms` rounded up to seconds. The WebUI retries once, then shows Try again. See [Read path busy](/docs/troubleshooting/common-issues/#10-documents-page-read-path-busy).

### GET /api/v1/documents/:id

Get document details by ID.

```bash
curl http://localhost:8080/api/v1/documents/doc-uuid \
  -H "X-Workspace-ID: workspace-uuid"
```

**Response**:

```json
{
  "id": "doc-uuid",
  "title": "Document Title",
  "status": "completed",
  "display_status": "completed",
  "ui_phase": "terminal",
  "content_hash": "sha256:...",
  "chunk_count": 15,
  "entity_count": 23,
  "relationship_count": 18,
  "file_path": "/uploads/document.pdf",
  "file_size": 1024000,
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:32:40Z"
}
```

### DELETE /api/v1/documents/:id

Delete a document and all associated data (chunks, entities, relationships). Emits SPEC-050 deletion progress on `/ws/pipeline/progress`.

Preview impact first: `GET /api/v1/documents/{document_id}/deletion-impact`.

```bash
curl -X DELETE http://localhost:8080/api/v1/documents/doc-uuid \
  -H "X-Workspace-ID: workspace-uuid"
```

**Response** (`200 OK`):

```json
{
  "document_id": "doc-uuid",
  "deleted": true,
  "chunks_deleted": 15,
  "entities_affected": 8,
  "relationships_affected": 12,
  "embeddings_deleted": 15,
  "partial_failure": false
}
```

### Page health and partial reprocess (SPEC-151)

PDF documents can reprocess selected pages without discarding healthy ones. Migration **160** stores per-page state.

| Endpoint | Description |
| -------- | ----------- |
| `GET /api/v1/documents/{id}/pages/health` | Per-page parse / figures / entities health |
| `POST /api/v1/documents/{id}/pages/reprocess` | `dry_run: true` returns a plan (200). Omit it to enqueue (202) |

`stages` is one or more of `parse`, `figures`, `entities`. `pages` is a JSON array or a range string (`"1-3,7"`). PDF only; an active task on the document returns **409**.

```bash
curl -X POST http://localhost:8080/api/v1/documents/$DOC_ID/pages/reprocess \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: workspace-uuid" \
  -d '{"pages":"1-3","stages":["parse"],"dry_run":true}'
```

---

## Parse API (SPEC-094)

Stateless **PDF → Markdown** conversion with **no document residue** — nothing is stored, embedded, or indexed. Use this when you only need the extracted text (preview, eval, export), not a full ingestion.

| Endpoint | Description |
| -------- | ----------- |
| `POST /api/v1/parse` | Parse a PDF (multipart `file` field or raw `application/pdf` body) |
| `GET /api/v1/parse/backends` | List available parse backends (`vision`, `edgeparse`, `edgeparse-ocr`, …) |
| `GET /api/v1/parse/jobs/{id}` | Poll an async parse job (in-memory TTL) |

**Sync vs async:**

| Input | Sync (≤ 15 pages, ≤ 20 MiB) | Async (up to 1000 pages) |
| ----- | --------------------------- | ------------------------ |
| `Prefer: respond-async` header | Optional — forces async | — |
| Over ceiling | 202 + job id (async) | — |

**Response** (sync 200) — `ParseResponse`: Markdown plus timing/cost metrics only (no `job_id` — that field exists only on async responses):

```json
{
  "markdown": "# Extracted content\n...",
  "backend": "vision",
  "backend_effective": "vision",
  "fallback_applied": false,
  "page_count": 12,
  "metrics": {
    "total_ms": 3450,
    "pages_per_second": 3.5,
    "estimated_cost_usd": 0.0042
  },
  "warnings": [],
  "request_id": "req-uuid"
}
```

Example — parse a PDF synchronously:

```bash
curl -X POST http://localhost:8080/api/v1/parse \
  -F "file=@document.pdf"
```

Example — EdgeParse + Tesseract OCR backend:

```bash
curl -X POST http://localhost:8080/api/v1/parse \
  -F "file=@document.pdf" \
  -F 'options={"backend":"edgeparse-ocr"}'
```

Example — request async processing:

```bash
curl -X POST http://localhost:8080/api/v1/parse \
  -H "Prefer: respond-async" \
  -F "file=@large-document.pdf"
# → 202 { "job_id": "pr_...", "status": "accepted", "request_id": "req-uuid" }
#   poll GET /api/v1/parse/jobs/{job_id} → { "job_id": ..., "status": "completed|processing|failed", "result": {ParseResponse}, "error": {code, message} }
```

Async jobs are held in an **in-memory TTL store** (default 1 hour) — restarting the API loses them.

> The parse API never creates documents or tasks — nothing to cancel, nothing to clean up. Full spec: [`specs/94-api-markdown/00-spec.md`](../../specs/94-api-markdown/00-spec.md).

---

## Query API

Execute RAG queries with multi-mode retrieval.

### POST /api/v1/query

Execute a query with configurable retrieval mode.

**Request**:

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: workspace-uuid" \
  -d '{
    "query": "What are the main themes discussed?",
    "mode": "hybrid",
    "enable_rerank": true,
    "rerank_top_k": 5
  }'
```

**Request Body**:

| Field                  | Type    | Default  | Description                                         |
| ---------------------- | ------- | -------- | --------------------------------------------------- |
| `query`                | string  | required | The question to answer                              |
| `mode`                 | string  | "mix" | Query mode (see below)                              |
| `context_only`         | boolean | false    | Return only retrieved context, no LLM answer        |
| `prompt_only`          | boolean | false    | Return formatted prompt for debugging               |
| `enable_rerank`        | boolean | true     | Apply reranking to improve relevance                |
| `rerank_top_k`         | integer | 5        | Number of top chunks after reranking                |
| `conversation_history` | array   | null     | Previous messages for multi-turn context            |
| `system_prompt`        | string  | null     | Custom instructions prepended to LLM context        |
| `document_filter`      | object  | null     | Optional filter to restrict RAG context (see below) |

**Document Filter Object**:

| Field              | Type   | Description                                                                  |
| ------------------ | ------ | ---------------------------------------------------------------------------- |
| `date_from`        | string | ISO 8601 date. Only include documents created on or after this date          |
| `date_to`          | string | ISO 8601 date. Only include documents created on or before this date         |
| `document_pattern` | string | Comma-separated terms. Matches document titles case-insensitively (OR logic) |

All filter fields are optional and AND-ed together. Omit `document_filter` entirely to query all documents.

**Query Modes** (default when unset is **`mix`** — the code falls back to `QueryMode::Mix`):

| Mode     | Description              | Use Case                          |
| -------- | ------------------------ | --------------------------------- |
| `naive`  | Vector search only       | Fast, simple queries              |
| `local`  | Entity-centric retrieval | Questions about specific entities |
| `global` | Relationship-centric search | Theme/overview questions        |
| `hybrid` | Local + Global + Naive (round-robin) | General queries      |
| `mix`    | Weighted blend of all arms (**default**) | Complex queries      |
| `bypass` | Direct LLM, no RAG       | When context not needed           |

**Response**:

```json
{
  "answer": "The main themes discussed include...",
  "mode": "hybrid",
  "sources": [
    {
      "source_type": "chunk",
      "id": "chunk-uuid",
      "score": 0.89,
      "rerank_score": 0.95,
      "snippet": "The first theme relates to...",
      "reference_id": 1,
      "document_id": "doc-uuid",
      "file_path": "document.pdf",
      "start_line": 45,
      "end_line": 52,
      "chunk_index": 3
    },
    {
      "source_type": "entity",
      "id": "CLIMATE_CHANGE",
      "score": 0.85,
      "snippet": "A global phenomenon affecting...",
      "reference_id": 2,
      "document_id": "doc-uuid"
    }
  ],
  "stats": {
    "embedding_time_ms": 45,
    "retrieval_time_ms": 123,
    "generation_time_ms": 890,
    "total_time_ms": 1058,
    "sources_retrieved": 8,
    "rerank_time_ms": 67,
    "tokens_used": 256,
    "tokens_per_second": 287.6,
    "llm_provider": "ollama",
    "llm_model": "gemma4:latest"
  },
  "reranked": true
}
```

### POST /api/v1/query/stream

Stream query response using Server-Sent Events (SSE).

**Request**:

```bash
curl -X POST http://localhost:8080/api/v1/query/stream \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -d '{"query": "Explain the key findings", "mode": "hybrid"}'
```

**Request Parameters**:

| Field             | Type   | Required | Description                                              |
| ----------------- | ------ | -------- | -------------------------------------------------------- |
| `query`           | string | yes      | Natural language query                                   |
| `mode`            | string | no       | Query mode: `hybrid`, `local`, `global`, `naive`, `mix`  |
| `system_prompt`   | string | no       | System prompt extension                                  |
| `document_filter` | object | no       | Document filter to scope RAG context (SPEC-005)          |
| `llm_provider`    | string | no       | LLM provider override (e.g., `openai`, `ollama`)         |
| `llm_model`       | string | no       | LLM model override (e.g., `gpt-4.1-nano`)                  |
| `stream_format`   | string | no       | `v1` for raw text (backward compat), `v2` for structured |

**SSE Events (v2 format — default)**:

```
data: {"type":"context","sources":[{"source_type":"chunk","id":"...","score":0.89,"entity_type":"PERSON","degree":5}],"query_mode":"hybrid","retrieval_time_ms":120}

data: {"type":"token","content":"The"}

data: {"type":"token","content":" key"}

data: {"type":"token","content":" findings"}

data: {"type":"done","stats":{"retrieval_time_ms":120,"generation_time_ms":800,"total_time_ms":920,"sources_retrieved":8,"tokens_used":256,"tokens_per_second":320.0,"query_mode":"hybrid"},"llm_provider":"ollama","llm_model":"gemma4:latest"}
```

**SSE Events (v1 format — `stream_format: "v1"`)**:

```
data: The

data:  key

data:  findings
```

---

## Chat API

Unified chat completions API with OpenAI-compatible format.

### POST /api/v1/chat/completions

Execute a chat completion with automatic conversation management.

**Request**:

```bash
curl -X POST http://localhost:8080/api/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: workspace-uuid" \
  -d '{
    "message": "What is the relationship between X and Y?",
    "conversation_id": "conv-uuid",
    "mode": "hybrid",
    "stream": false
  }'
```

**Request Body**:

| Field             | Type    | Default  | Description                                                        |
| ----------------- | ------- | -------- | ------------------------------------------------------------------ |
| `message`         | string  | required | User message                                                       |
| `conversation_id` | string  | null     | Existing conversation ID (creates new if null)                     |
| `mode`            | string  | "hybrid" | Query mode                                                         |
| `stream`          | boolean | false    | Enable SSE streaming                                               |
| `system_prompt`   | string  | null     | Custom instructions prepended to LLM context                       |
| `document_filter` | object  | null     | Optional filter to restrict RAG context (same schema as Query API) |

**Response** (Non-streaming):

```json
{
  "id": "msg-uuid",
  "conversation_id": "conv-uuid",
  "role": "assistant",
  "content": "The relationship between X and Y is...",
  "sources": [...],
  "stats": {...},
  "created_at": "2024-01-15T10:30:00Z"
}
```

**Streaming Response**:

```bash
curl -X POST http://localhost:8080/api/v1/chat/completions \
  -H "Accept: text/event-stream" \
  -d '{"message": "...", "stream": true}'
```

```
data: {"type":"conversation","conversation_id":"conv-uuid","user_message_id":"msg-uuid"}

data: {"type":"context","sources":[{"source_type":"entity","id":"X","score":0.95,"entity_type":"PERSON","degree":12}],"query_mode":"hybrid","retrieval_time_ms":85}

data: {"type":"token","content":"The"}

data: {"type":"token","content":" relationship"}

data: {"type":"done","assistant_message_id":"asst-uuid","tokens_used":128,"duration_ms":920,"llm_provider":"ollama","llm_model":"gemma4:latest"}
```

---

## Graph API

Knowledge graph exploration and visualization endpoints.

### GET /api/v1/graph

Get the knowledge graph with optional traversal.

**Query Parameters**:

| Parameter    | Type    | Default | Description                     |
| ------------ | ------- | ------- | ------------------------------- |
| `start_node` | string  | null    | Entity ID to center traversal   |
| `depth`      | integer | 2       | Max traversal hops              |
| `max_nodes`  | integer | 100     | Max nodes to return (max: 1000) |

```bash
curl "http://localhost:8080/api/v1/graph?start_node=ENTITY_NAME&depth=2&max_nodes=50" \
  -H "X-Workspace-ID: workspace-uuid"
```

**Response**:

```json
{
  "nodes": [
    {
      "id": "ENTITY_NAME",
      "label": "Entity Name",
      "node_type": "PERSON",
      "description": "Description of the entity...",
      "degree": 5,
      "properties": {}
    }
  ],
  "edges": [
    {
      "source": "ENTITY_A",
      "target": "ENTITY_B",
      "edge_type": "WORKS_WITH",
      "weight": 1.0,
      "properties": {}
    }
  ],
  "total_nodes": 150,
  "total_edges": 200,
  "is_truncated": true
}
```

### GET /api/v1/graph/stats

Get graph statistics.

```bash
curl http://localhost:8080/api/v1/graph/stats \
  -H "X-Workspace-ID: workspace-uuid"
```

**Response**:

```json
{
  "total_nodes": 1500,
  "total_edges": 4200,
  "node_types": {
    "PERSON": 250,
    "ORGANIZATION": 180,
    "CONCEPT": 820,
    "LOCATION": 150,
    "EVENT": 100
  },
  "edge_types": {
    "RELATED_TO": 2100,
    "WORKS_WITH": 450,
    "LOCATED_IN": 320,
    "PART_OF": 580
  },
  "avg_degree": 2.8,
  "density": 0.0019
}
```

### GET /api/v1/graph/entities

List entities with pagination.

```bash
curl "http://localhost:8080/api/v1/graph/entities?limit=20&type=PERSON" \
  -H "X-Workspace-ID: workspace-uuid"
```

### GET /api/v1/graph/entities/:id

Get entity details by ID.

```bash
curl http://localhost:8080/api/v1/graph/entities/ENTITY_NAME \
  -H "X-Workspace-ID: workspace-uuid"
```

### GET /api/v1/graph/relationships

List relationships with pagination.

```bash
curl "http://localhost:8080/api/v1/graph/relationships?limit=20&type=WORKS_WITH" \
  -H "X-Workspace-ID: workspace-uuid"
```

### GET /api/v1/graph/stream

Stream graph updates via SSE (for real-time visualization).

```bash
curl http://localhost:8080/api/v1/graph/stream \
  -H "Accept: text/event-stream" \
  -H "X-Workspace-ID: workspace-uuid"
```

---

## Workspaces API

Manage workspaces for multi-tenant isolation.

### POST /api/v1/tenants/{tenant_id}/workspaces

Create a new workspace.

```bash
curl -X POST http://localhost:8080/api/v1/tenants/$TENANT_ID/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Research Project",
    "description": "Workspace for research documents",
    "embedding_model": "text-embedding-3-small",
    "embedding_dimension": 1536,
    "llm_model": "gpt-4.1-nano"
  }'
```

### GET /api/v1/tenants/{tenant_id}/workspaces

List workspaces for a tenant. `total` is `COUNT(*)`. Optional `?include_stats=true` attaches stats **from cache only**: a cold cache returns `stats: null` and does not run the AGE/SQL stats query inside the list deadline. `GET /api/v1/workspaces/{id}/stats` warms that cache.

**503** `read_path_busy` under the same interactive budget as the documents list ([above](#get-apiv1documents)). There is no unscoped `GET /api/v1/workspaces` list.

### GET /api/v1/workspaces/:id

Get workspace details.

### PUT /api/v1/workspaces/:id

Update workspace settings.

### DELETE /api/v1/workspaces/:id

Delete a workspace and all its data.

---

## Conversations API

Manage chat conversations.

### GET /api/v1/conversations

List conversations.

### POST /api/v1/conversations

Create a new conversation.

### GET /api/v1/conversations/:id

Get conversation with messages.

### DELETE /api/v1/conversations/:id

Delete a conversation.

### GET /api/v1/conversations/:id/messages

Get messages in a conversation.

---

## Models & Settings

### GET /api/v1/models

List available LLM models.

```bash
curl http://localhost:8080/api/v1/models
```

**Response** (`ModelsListResponse`):

```json
{
  "providers": [
    {
      "name": "openai",
      "display_name": "OpenAI",
      "provider_type": "openai",
      "enabled": true,
      "priority": 10,
      "description": "OpenAI GPT models",
      "models": [
        {
          "name": "gpt-4.1-mini",
          "display_name": "GPT-4.1 Mini",
          "model_type": "llm",
          "deprecated": false,
          "capabilities": {
            "context_length": 1047576,
            "max_output_tokens": 32768,
            "supports_vision": true,
            "supports_streaming": true,
            "embedding_dimension": 0
          }
        }
      ],
      "auth_kind": "api_key"
    },
    {
      "name": "ollama",
      "display_name": "Ollama",
      "provider_type": "ollama",
      "enabled": true,
      "models": [
        {
          "name": "gemma4:latest",
          "display_name": "Gemma 4 Latest",
          "model_type": "llm"
        }
      ],
      "auth_kind": "local"
    }
  ],
  "default_llm_provider": "openai",
  "default_llm_model": "gpt-4.1-mini",
  "default_embedding_provider": "openai",
  "default_embedding_model": "text-embedding-3-small"
}
```

> Runtime `default_llm_*` fields reflect the active provider from env/server config, not only `models.toml` static defaults.

### GET /api/v1/models/{provider}

Get models for a specific provider (e.g. `/api/v1/models/openai`).

### GET /api/v1/settings/llm-defaults

Get server-level LLM/embedding defaults (Settings UI).

### GET /api/v1/settings/providers

List available providers with credential requirements and default models.

### GET /api/v1/settings/attribution

Returns the effective application attribution context and a **provider header catalog** describing what EdgeQuake sends upstream to each LLM provider (OpenRouter referer, OpenAI client ID, Anthropic application ID, etc.).

**Auth:** Bearer token or API key (same as other `/api/v1/settings/*` routes).

```bash
curl http://localhost:8080/api/v1/settings/attribution \
  -H "Authorization: Bearer $TOKEN"
```

**Response**:

```json
{
  "effective_context": {
    "app_id": "edgequake",
    "app_name": "EdgeQuake",
    "app_url": "http://localhost:3000",
    "tenant_id": null,
    "request_id": null,
    "end_user_id": null,
    "active": true,
    "sources": ["env:EDGEQUAKE_APP_ID", "env:EDGEQUAKE_APP_NAME"]
  },
  "providers": [
    {
      "id": "openai",
      "display_name": "OpenAI",
      "attribution_support": "full",
      "headers": ["X-Client-Request-Id"],
      "body_fields": ["user"]
    },
    {
      "id": "anthropic",
      "display_name": "Anthropic",
      "attribution_support": "full",
      "headers": ["x-application-id", "x-request-id"],
      "body_fields": []
    },
    {
      "id": "openrouter",
      "display_name": "OpenRouter",
      "attribution_support": "full",
      "headers": ["HTTP-Referer", "X-OpenRouter-Title", "X-Title"],
      "body_fields": []
    }
  ],
  "ingress_headers": [
    "x-edgequake-app-id",
    "x-edgequake-app-name",
    "x-edgequake-app-url",
    "x-edgequake-tenant-id",
    "x-edgequake-request-id"
  ],
  "environment_variables": [
    "EDGEQUAKE_APP_ID",
    "EDGEQUAKE_APP_NAME",
    "EDGEQUAKE_APP_URL",
    "EDGEQUAKE_TENANT_ID"
  ]
}
```

| Field | Description |
| ----- | ----------- |
| `effective_context.active` | `true` when at least one of `app_id`, `app_name`, or `app_url` is set |
| `providers[].attribution_support` | `full`, `passthrough`, `observability_only`, or `none` (from provider catalog in edgequake-core) |
| `providers[].headers` | HTTP headers injected on upstream LLM requests for that provider |
| `providers[].body_fields` | JSON body fields set for attribution (e.g. OpenAI `user`) |
| `ingress_headers` | Request headers clients may send to override attribution per call |
| `environment_variables` | Env vars that populate `ApplicationContext` at process start |

### GET /api/v1/settings/app-attribution

Same response as `GET /settings/attribution`. Used by the Settings UI **Application Attribution** card.

### PATCH /api/v1/settings/app-attribution

Persist application attribution to PostgreSQL `server_config` (admin role required). Does **not** store API keys — only `app_id`, `app_name`, and `app_url`.

```bash
curl -X PATCH http://localhost:8080/api/v1/settings/app-attribution \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "app_id": "edgequake",
    "app_name": "EdgeQuake",
    "app_url": "http://localhost:3000"
  }'
```

**Response**:

```json
{
  "saved": true,
  "note": "Saved to server_config and applied immediately. Env vars (EDGEQUAKE_APP_*) still override on conflict."
}
```

> **Note:** Env vars (`EDGEQUAKE_APP_*`) override `server_config` values on conflict. PATCH applies immediately without restart.

### GET /api/v1/config/effective

Get the effective configuration resolution chain (env → server config → compiled defaults).

---

## Error Handling

EdgeQuake uses RFC 7807 Problem Details for error responses.

**Error Response Format**:

```json
{
  "type": "https://edgequake.dev/errors/not-found",
  "title": "Resource Not Found",
  "status": 404,
  "detail": "Document with ID 'doc-uuid' not found in workspace",
  "instance": "/api/v1/documents/doc-uuid"
}
```

**Common Error Codes**:

| Status | Type                  | Description                         |
| ------ | --------------------- | ----------------------------------- |
| 400    | `bad-request`         | Invalid request parameters          |
| 401    | `unauthorized`        | Missing or invalid authentication   |
| 403    | `forbidden`           | Access denied to resource           |
| 404    | `not-found`           | Resource not found                  |
| 409    | `conflict`            | Resource already exists (duplicate) |
| 413    | `payload-too-large`   | File exceeds size limit             |
| 422    | `validation-error`    | Request validation failed           |
| 429    | `rate-limited`        | Too many requests                   |
| 500    | `internal-error`      | Server error                        |
| 503    | `service-unavailable` | Dependency unavailable              |

---

## Rate Limiting

Rate limiting is applied per API key or IP address.

**Headers in Response**:

| Header                  | Description                         |
| ----------------------- | ----------------------------------- |
| `X-RateLimit-Limit`     | Max requests per window             |
| `X-RateLimit-Remaining` | Requests remaining                  |
| `X-RateLimit-Reset`     | Epoch timestamp when limit resets   |
| `Retry-After`           | Seconds to wait (when rate limited) |

**Default Limits**:

| Endpoint Category | Requests  | Window   |
| ----------------- | --------- | -------- |
| Document upload   | 10        | 1 minute |
| Query execution   | 60        | 1 minute |
| Graph traversal   | 100       | 1 minute |
| Health checks     | Unlimited | -        |

---

## Ollama Compatibility Layer

EdgeQuake provides Ollama-compatible endpoints for tool integration.

### POST /v1/embeddings

Generate embeddings (Ollama format).

```bash
curl -X POST http://localhost:8080/v1/embeddings \
  -H "Content-Type: application/json" \
  -d '{"model": "nomic-embed-text", "input": "Hello world"}'
```

### POST /v1/chat/completions

Chat completions (OpenAI format, Ollama compatible).

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemma4:latest",
    "messages": [
      {"role": "user", "content": "Hello!"}
    ],
    "stream": false
  }'
```

---

## Request Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    API REQUEST PROCESSING                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  Client Request                                                  │
│       ↓                                                          │
│  ┌─────────────────┐                                            │
│  │  Rate Limiter   │ ─ 429 if exceeded                          │
│  └────────┬────────┘                                            │
│           ↓                                                      │
│  ┌─────────────────┐                                            │
│  │ Authentication  │ ─ 401 if invalid                           │
│  └────────┬────────┘                                            │
│           ↓                                                      │
│  ┌─────────────────┐                                            │
│  │ Tenant Context  │ ─ Extract X-Tenant-ID, X-Workspace-ID      │
│  └────────┬────────┘                                            │
│           ↓                                                      │
│  ┌─────────────────┐                                            │
│  │ Request Handler │ ─ Business logic                           │
│  └────────┬────────┘                                            │
│           ↓                                                      │
│  ┌─────────────────┐                                            │
│  │  Response       │ ─ JSON or SSE stream                       │
│  └─────────────────┘                                            │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Knowledge Injection API

> **Added in v0.8.0** — Closes [#131](https://github.com/raphaelmansuy/edgequake/issues/131)

Knowledge injection lets you enrich a workspace's knowledge graph with acronym definitions, synonym mappings, and domain glossaries. Injection entries are processed through the standard entity-extraction pipeline but are **never listed as source citations** in query results — they silently improve retrieval quality.

### List Injections

```http
GET /api/v1/workspaces/{workspace_id}/injections
X-Workspace-ID: {workspace_id}
```

**Response 200**

```json
[
  {
    "injection_id": "a1b2c3d4-...",
    "name": "Domain Glossary",
    "status": "completed",
    "entity_count": 15,
    "content_length": 420,
    "source_type": "text",
    "created_at": "2026-04-03T10:00:00Z",
    "updated_at": "2026-04-03T10:01:30Z"
  }
]
```

### Create / Replace Injection (Text)

```http
PUT /api/v1/workspaces/{workspace_id}/injection
Content-Type: application/json
X-Workspace-ID: {workspace_id}

{
  "name": "Domain Glossary",
  "content": "OEE = Overall Equipment Effectiveness\nNLP = Natural Language Processing\n"
}
```

**Response 202**

```json
{
  "injection_id": "a1b2c3d4-...",
  "workspace_id": "default",
  "name": "Domain Glossary",
  "status": "processing"
}
```

### Upload Injection File

```http
PUT /api/v1/workspaces/{workspace_id}/injection/file
Content-Type: multipart/form-data
X-Workspace-ID: {workspace_id}

name=Domain Glossary
file=@glossary.txt
```

Accepted MIME types: `text/plain`, `text/markdown`, `application/octet-stream` (for `.md`/`.txt` files).

**Response 202** — same shape as PUT.

### Get Injection Detail

```http
GET /api/v1/workspaces/{workspace_id}/injections/{injection_id}
X-Workspace-ID: {workspace_id}
```

**Response 200**

```json
{
  "injection_id": "a1b2c3d4-...",
  "name": "Domain Glossary",
  "content": "OEE = Overall Equipment Effectiveness\n...",
  "status": "completed",
  "entity_count": 15,
  "source_type": "text",
  "created_at": "2026-04-03T10:00:00Z",
  "updated_at": "2026-04-03T10:01:30Z"
}
```

### Update Injection

```http
PATCH /api/v1/workspaces/{workspace_id}/injections/{injection_id}
Content-Type: application/json
X-Workspace-ID: {workspace_id}

{
  "name": "Updated Glossary",
  "content": "OEE = Overall Equipment Effectiveness\nKPI = Key Performance Indicator\n"
}
```

Updating `content` re-triggers the pipeline (old entities are deleted first). Updating only `name` is instant.

**Response 200** — updated `InjectionDetail`.

### Delete Injection

```http
DELETE /api/v1/workspaces/{workspace_id}/injections/{injection_id}
X-Workspace-ID: {workspace_id}
```

Cascades: removes all KV entries, vectors, graph nodes, and edges created by this injection.

**Response 204 No Content**

### Citation Exclusion

Injection entries enrich the knowledge graph and improve retrieval but are filtered out of `sources` arrays in all query and chat responses:

```json
{
  "answer": "OEE stands for Overall Equipment Effectiveness...",
  "sources": [
    { "id": "doc-123", "title": "Line 3 Report", "source_type": "chunk" }
    // injection entries never appear here
  ]
}
```

---

## See Also

- [OpenAPI snapshot](../../edgequake_webui/openapi/openapi.snapshot.json) — full endpoint catalog (v0.23.0)
- [Pipeline Progress](/docs/deep-dives/pipeline-progress/) — progress WS/REST/SSE
- [Ingestion cancel & fairness](/docs/ingestion-cancel-and-fairness.md) — cancel SSOT
- [Quick Start Guide](/docs/getting-started/quick-start/) - Get running in 5 minutes
- [Query Modes](/docs/deep-dives/lightrag-algorithm/#query-modes) - Detailed mode comparison
- [Architecture Overview](/docs/architecture/overview/) - System design

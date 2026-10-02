---
title: 'Docker Quickstart'
---

> **Product: v0.29.0** · Contract: OpenAPI · Spec ops: [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md)

# Docker Quickstart — Full Stack in One Command

> **Zero prerequisites beyond Docker.**  
> No Rust, no Node.js, no `cargo build`, no `npm install`.  
> Cold start: **~30 seconds** on a fast connection (image layers cache after first pull).

---

## The One-Liner (no git clone required)

Copy and paste this into any terminal that has Docker:

```bash
curl -fsSL https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/docker-compose.quickstart.yml \
  | docker compose -f - up -d
```

That's it. Three versioned images (API, Web UI, PostgreSQL) are pulled from GitHub Container Registry and started.

The API image is distroless (no shell). Compose healthchecks run `edgequake healthcheck` (`GET /live`). Use `docker compose logs` rather than `docker exec sh`.

**OCR in the API image:** Tesseract 5 + tessdata language packs (`eng`, `osd`, `fra`, `deu`, `spa`, `por`, `chi_sim`) are harvested into the distroless runtime for `pdf_parser_backend=edgeparse-ocr`. Vision LLM weights remain external (Ollama / OpenAI / …). pdfium stays compile-time embedded.

**Then open:** http://localhost:3000

> **Auth note:** Product default is auth **on**. Quickstart compose sets `EDGEQUAKE_DEV_MODE=true` (open API, no login) for frictionless demos — **do not use in production**. For production, set `EDGEQUAKE_DEV_MODE=false`, `EDGEQUAKE_AUTH_ENABLED=true`, and `EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD`.

---

## Option A — Pure `docker compose` (download file first)

```bash
# 1. Download the compose file
curl -fsSL https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/docker-compose.quickstart.yml \
  -o docker-compose.quickstart.yml

# 2. Start the full stack
docker compose -f docker-compose.quickstart.yml up -d

# 3. Check health
curl http://localhost:8080/health
```

---

## Option B — With git clone + make

```bash
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake
make stack
```

---

## Option C — Pinned version (production-stable)

```bash
# Download a specific version's compose file
EDGEQUAKE_VERSION=0.24.2
curl -fsSL "https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/docker-compose.quickstart.yml" \
  -o docker-compose.quickstart.yml

# Start with that version
EDGEQUAKE_VERSION=${EDGEQUAKE_VERSION} docker compose -f docker-compose.quickstart.yml up -d
```

---

## Access Points

| Service      | URL                              | Description                               |
| ------------ | -------------------------------- | ----------------------------------------- |
| 🌐 Web UI     | http://localhost:3000            | Upload documents, explore knowledge graph |
| 🔗 REST API   | http://localhost:8080            | Programmatic access                       |
| 📚 Swagger UI | http://localhost:8080/swagger-ui | Interactive API explorer                  |
| 🏥 Health     | http://localhost:8080/health     | JSON health status                        |

---

## LLM Providers

### Default: Ollama (free, local, no API key)

Requires [Ollama](https://ollama.ai) running on your machine:

```bash
# Install and run Ollama (once)
curl -fsSL https://ollama.ai/install.sh | sh
ollama serve &
ollama pull gemma4:latest

# Start EdgeQuake
docker compose -f docker-compose.quickstart.yml up -d
```

### OpenAI

```bash
EDGEQUAKE_LLM_PROVIDER=openai \
OPENAI_API_KEY=sk-... \
  docker compose -f docker-compose.quickstart.yml up -d
```

### Any OpenAI-compatible endpoint (LM Studio, vLLM, Azure, etc.)

```bash
EDGEQUAKE_LLM_PROVIDER=openai \
OPENAI_API_KEY=your-key \
OPENAI_BASE_URL=http://localhost:1234/v1 \
  docker compose -f docker-compose.quickstart.yml up -d
```

### Provider reference

| Variable                       | Default                             | Description                            |
| ------------------------------ | ----------------------------------- | -------------------------------------- |
| `EDGEQUAKE_DEFAULT_LLM_PROVIDER` | `ollama` (no API key) / `openai` (with key) | Primary LLM provider (`make dev` sets this) |
| `EDGEQUAKE_DEFAULT_LLM_MODEL`    | `gemma4:latest` / `gpt-5-nano`¹             | Primary LLM model                         |
| `EDGEQUAKE_EMBEDDING_PROVIDER`   | same as LLM                                 | Override embedding provider               |
| `EDGEQUAKE_EMBEDDING_MODEL`      | provider-specific default                   | Embedding model override                  |
| `OPENAI_API_KEY`               | _(empty)_                           | Required when provider is `openai`     |
| `OPENAI_BASE_URL`              | _(empty)_                           | Override OpenAI base URL               |
| `OLLAMA_HOST`                  | `http://host.docker.internal:11434` | Ollama server address                  |
| `EDGEQUAKE_VERSION`            | `latest`                            | Pin to a specific release tag          |
| `EDGEQUAKE_POSTGRES_TAG`       | same as `EDGEQUAKE_VERSION`         | PG image tag (`0.24.2-pg16\|pg17\|pg18`) |
| `EDGEQUAKE_DEV_MODE`           | `true` (quickstart)                 | Open API without login — not for prod  |
| `EDGEQUAKE_PORT`               | `8080`                              | API port                               |
| `FRONTEND_PORT`                | `3000`                              | Web UI port                            |
| `POSTGRES_PASSWORD`            | `edgequake_secret`                  | PostgreSQL password                    |

¹ With `OPENAI_API_KEY`, Makefile sets `gpt-5-nano`; bundled catalog default is `gpt-4.1-mini`.

Supported providers include `ollama`, `openai`, `lmstudio`, `anthropic`, `mistral`, `gemini`, `vertexai`, `openrouter`, `mock`, and others in `models.toml`.

### Migration aliases

If you are migrating from a LightRAG-style environment file, EdgeQuake also accepts:

| Alias                 | Canonical variable              |
| --------------------- | ------------------------------- |
| `MODEL_PROVIDER`      | `EDGEQUAKE_LLM_PROVIDER`        |
| `CHAT_MODEL`          | `EDGEQUAKE_LLM_MODEL`           |
| `EMBEDDING_PROVIDER`  | `EDGEQUAKE_EMBEDDING_PROVIDER`  |
| `EMBEDDING_MODEL`     | `EDGEQUAKE_EMBEDDING_MODEL`     |
| `EMBEDDING_DIMENSION` | `EDGEQUAKE_EMBEDDING_DIMENSION` |

Canonical `EDGEQUAKE_*` variables always win when both are set.

---

## Managing the Stack

```bash
# Stop and remove containers (data persists in the edgequake-pg-data volume)
docker compose -f docker-compose.quickstart.yml down

# Stop and remove containers + data
docker compose -f docker-compose.quickstart.yml down -v

# Tail all logs
docker compose -f docker-compose.quickstart.yml logs -f

# Check container status
docker compose -f docker-compose.quickstart.yml ps

# Pull latest images (then restart)
docker compose -f docker-compose.quickstart.yml pull
docker compose -f docker-compose.quickstart.yml up -d

# Restart a single service
docker compose -f docker-compose.quickstart.yml restart api
```

---

## Images

All images are multi-arch (`linux/amd64`, `linux/arm64`) and published to GitHub Container Registry on every tagged release.

| Image                                      | Tag                                      | Description                        |
| ------------------------------------------ | ---------------------------------------- | ---------------------------------- |
| `ghcr.io/raphaelmansuy/edgequake`          | `latest` / `0.24.2`                      | Rust API server                    |
| `ghcr.io/raphaelmansuy/edgequake-frontend` | `latest` / `0.24.2`                      | Next.js Web UI                     |
| `ghcr.io/raphaelmansuy/edgequake-postgres` | `latest` / `0.24.2` / `0.24.2-pg16\|pg17\|pg18` | PostgreSQL + pgvector + Apache AGE |

**PostgreSQL major tags** (extension pins: pgvector **0.8.3**, AGE **1.6.0** on PG16 / **1.7.0** on PG17+):

| Tag | PostgreSQL |
| --- | ---------- |
| `0.24.2` / `latest` / `0.24.2-pg18` / `latest-pg18` | PG18 (default) |
| `0.24.2-pg17` / `latest-pg17` | PG17 |
| `0.24.2-pg16` / `latest-pg16` | PG16 |

Pin PostgreSQL major when starting:

```bash
EDGEQUAKE_VERSION=0.24.2 EDGEQUAKE_POSTGRES_TAG=0.24.2-pg16 \
  docker compose -f docker-compose.quickstart.yml up -d
```

Pull an image manually:
```bash
docker pull ghcr.io/raphaelmansuy/edgequake:0.24.2
docker pull ghcr.io/raphaelmansuy/edgequake-frontend:0.24.2
docker pull ghcr.io/raphaelmansuy/edgequake-postgres:0.24.2
```

---

## Troubleshooting

### API never becomes healthy

```bash
# Check logs
docker compose -f docker-compose.quickstart.yml logs api

# Common cause: Ollama not running
curl http://localhost:11434/api/tags   # should return model list
ollama serve &                          # start if not running
```

### Port already in use

```bash
# Change ports
EDGEQUAKE_PORT=8081 FRONTEND_PORT=3001 \
  docker compose -f docker-compose.quickstart.yml up -d
```

### Reset all data

```bash
docker compose -f docker-compose.quickstart.yml down -v
docker compose -f docker-compose.quickstart.yml up -d
```

### Apple Silicon (M-series) / ARM64

Images include native `linux/arm64` layers — no QEMU emulation needed. Docker Desktop on macOS auto-selects the right architecture.

---

## Next Steps

- [REST API Reference](/docs/api-reference/) — upload documents, query, manage the graph
- [Configuration Guide](/docs/operations/configuration/) — tune LLM models, embedding dimensions, timeouts
- [Production Deployment](/docs/operations/deployment/) — TLS, secrets management, scaling

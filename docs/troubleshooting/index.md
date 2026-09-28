---
title: Troubleshooting
description: Diagnose and resolve common issues with EdgeQuake pipelines and deployments.
---

> **Product: v0.23.0** · Ingestion SSOT: [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md)

Guides for diagnosing and resolving common EdgeQuake issues.

## Guides

- **[Common Issues](/docs/troubleshooting/common-issues/)** — Solutions to frequently encountered errors: upload format mistakes, server startup, PDF extraction, query failures, and database issues.

## Pipeline reliability (v0.23)

Ingestion cancel, lease, and multi-replica behavior are documented in depth in the ops runbook. The troubleshooting guide covers the operator-facing symptoms:

| Topic | Where |
| ----- | ----- |
| **Cancel SSOT** (cancel ≠ Failed, `display_status`, Stopping…) | [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md) |
| **Interrupted → Reprocess** after restart or lease expiry | [Common Issues §3.1](/docs/troubleshooting/common-issues/#31-interrupted--reprocess-v019) |
| **Lease stuck in Processing** | [Common Issues §3.2](/docs/troubleshooting/common-issues/#32-lease-stuck-in-processing) |
| **`EDGEQUAKE_REPLICAS>1` boot fail** | [Common Issues §3.4](/docs/troubleshooting/common-issues/#34-multi-replica-boot-failure-edgequake_replicas1) |
| Queue pressure & compensation quarantine | [Observability — queue metrics](../OBSERVABILITY.md#queue-pressure--store-contention-v019) |
| **Documents "Read path busy"** (503, header Busy) | [Common Issues §10](/docs/troubleshooting/common-issues/#10-documents-page-read-path-busy) |

## Related

- [Configuration](/docs/operations/configuration/) — env vars including lease, replicas, and queue thresholds
- [Monitoring](/docs/operations/monitoring/) — Prometheus and health endpoints
- [SQLx offline mode](/docs/sqlx-offline-mode/) — build without DB + migration checksum adjacency

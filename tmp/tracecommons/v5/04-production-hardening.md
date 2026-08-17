# Production Hardening

**Date**: August 2026

TC runs on a single GCP VM (systemd + Caddy + Cloud SQL Auth Proxy). ~352 submissions, ~13/week. Security posture is strong: RLS with FORCE on every table, hash-only logging followed in practice, fail-closed defaults, 17 operational drills, audit chain with write-time drift detection, canary self-test before scoring ticks, timing-oracle mitigation on redeem-confirm. The gaps below are additive — observability, availability, and scale preparation as traffic grows from 13/week toward hundreds.

---

## Part 1: Observability

### 1.1 OTel for TC's Own Pipeline

TC positions itself as the system that ingests OTel-format traces. Emitting them from its own pipeline is both credibility and necessity. Right now there's no single trace showing a submission's path through redaction, chunking, NEAR AI Cloud scoring (the latency dominator), gate evaluation, and settlement queueing.

**Instrument in priority order:**

1. **Submission pipeline.** Root span per submission, child spans for redaction, chunking, embedding, perplexity scoring, novelty scoring, gate evaluation, settlement queueing.
2. **Settlement batch.** Span per batch: credit computation, NEAR receipt outbox write, on-chain confirmation.
3. **Revocation propagation.** Span per revocation: content deletion, credit ledger adjustment, object ref cleanup. PR #246 changes semantics — verify end-to-end.
4. **Background daemon** (PR #244). Periodic tasks should emit spans for duty cycle visibility.

Setup: `opentelemetry` + `opentelemetry-otlp` + `tracing-opentelemetry`, add `OpenTelemetryLayer` to the existing `tracing_subscriber` registry. ~200 LOC across ~10 call sites. Export to Grafana Cloud free OTLP endpoint (50GB traces/month, 14-day retention).

### 1.2 Prometheus Metrics Endpoint

`tower_http` is already a dependency (CORS) but there's no metrics export. Add `metrics` crate + `metrics-exporter-prometheus`, install recorder at startup, expose `/metrics`. ~50-80 LOC.

**Metrics to emit:**

| Metric | Type | Labels |
|---|---|---|
| `tc_http_requests_total` | counter | method, route_family, status |
| `tc_http_request_duration_seconds` | histogram | method, route_family |
| `tc_submission_accepted_total` | counter | — |
| `tc_gate_evaluation_duration_seconds` | histogram | gate_name |
| `tc_gate_evaluation_result` | counter | gate_name, result |
| `tc_settlement_batch_duration_seconds` | histogram | — |
| `tc_near_scorer_duration_seconds` | histogram | — |
| `tc_near_scorer_errors_total` | counter | error_class |
| `tc_redaction_applied_total` | counter | redaction_type |
| `tc_revocation_propagation_duration_seconds` | histogram | — |
| `tc_db_pool_connections_active` | gauge | pool_name |
| `tc_db_pool_connections_idle` | gauge | pool_name |
| `tc_dedup_index_size` | gauge | — |
| `tc_audit_chain_length` | gauge | — |

Route families should be coarse (submission, review, admin, account, settlement, worker) to avoid cardinality explosion. The stable error class names already in use (`AuditChainDriftRejected`, etc.) work as metric labels.

Scraping: Grafana Cloud free remote-write agent or `grafana-agent` / `vector` to scrape and push. No local Prometheus needed.

### 1.3 tower-http TraceLayer

One line: `app.layer(TraceLayer::new_for_http())`. Gives structured request/response logs with latency, status, method, path. Wire `on_response` to record the `tc_http_request_duration_seconds` histogram from 1.2.

### 1.4 SLO Targets

| Operation | Target (p99) | Rationale |
|---|---|---|
| Submission acceptance (pre-gate) | < 500ms | Redaction + chunking + dedup. No external calls. |
| Gate evaluation (full pipeline) | < 30s | NEAR AI Cloud round-trip dominates. |
| Settlement batch | < 5 min | Batch computation + NEAR outbox write. |
| Revocation propagation | < 60s | Content deletion + ledger adjustment. |
| Health check (/health) | < 100ms | TCP-alive. |
| Health/ready (/health/ready) | < 2s | DB + canary + scorer probe. |
| Drill execution | < 30s | Budget for self-test subsystem exercises. |
| Admin endpoints | < 1s | Config status, operational summary. |

No automated alerting at pilot scale. Value: grep structured logs or query Prometheus for violations, give drills a performance dimension, catch regressions before users notice.

### 1.5 Structured Error Types

Error handling already uses stable class names. The gap is at the handler boundary where `anyhow` loses structure. Wrap top-level handler responses in a struct carrying: error class (`&'static str`, safe to log and count), route family, client-vs-internal flag, status code, human-readable message (client errors only; internal errors get generic text). ~100 LOC for the type + `IntoResponse` impl. Preserves hash-only logging, makes Prometheus error counting automatic.

### 1.6 Slow Query Logging

TC has 5+ DB pools per privilege level, each serving different query patterns. Wrap pool execution with a threshold check (~500ms). Log the parameterized query template (never bound values — hash-only logging extends to queries). Record `tc_db_slow_query_seconds` histogram labeled by pool name. ~50 LOC.

---

## Part 2: Reliability

### 2.1 Graceful Shutdown for trace-commons-ingest

The upload-claim issuer has thorough graceful shutdown (`serve_both_with_graceful_shutdown` with SIGTERM/SIGINT, oneshot channels, configurable grace window). The main ingest binary doesn't.

At 13/week this is academic. At real traffic, deploying without connection draining kills in-flight gate evaluations (seconds-long NEAR AI Cloud calls), leaves partial settlement state, and drops in-progress revocation deletions.

Mirror the upload-claim issuer: `tokio::signal` for SIGTERM/SIGINT, `axum::serve(...).with_graceful_shutdown(shutdown_signal())`, grace period for in-flight gate evaluations. ~50 LOC modeled on existing code.

For the background daemon (PR #244), graceful shutdown also means flushing pending consolidation, checkpointing the dedup index, and writing a resume marker.

### 2.2 /health/ready

Current `/health` returns `{"status":"ok","schema_version":"..."}` — liveness, not readiness. A readiness probe answers "can this process serve traffic right now?"

Checks: (1) each privilege-scoped DB pool can acquire a connection and `SELECT 1`, (2) PII backstop canary passes (already wired), (3) scorer reachable via lightweight probe (if configured), (4) migration tracking table shows expected schema version. Return 200 when critical checks pass, 503 when any fails, per-check latency in the response body. ~100 LOC.

### 2.3 Migration Runner Extraction

`connect_from_config` calls `backend.run_migrations().await?` on every startup. The tracking table provides idempotency, but concurrent startups can interleave DDL.

Extract to a `--migrate` flag or separate binary. Add `pg_advisory_lock` to serialize concurrent migration runners. ~150 LOC. Matters for: blue/green deploys, container restarts, background daemon (PR #244) starting alongside the main binary.

### 2.4 Rate Limiting: In-Process to Redis

**Phase 1 (now):** Keep in-process `Mutex<HashMap>`. Add `tc_rate_limit_hits_total` counter (surface, action labels). ~20 LOC.

**Phase 2 (second instance):** Replace with Redis INCR + EXPIRE (or `governor` with Redis backend). Same interface, shared storage. ~200 LOC.

### 2.5 Redaction Fuzzing

PR #201 proposes extracting the redactor to a leaf `trace-commons-redaction` crate — prerequisite for fuzzing.

A standalone `fn redact(input: &Value) -> Value` is trivially fuzzable with `cargo-fuzz`: generate adversarial JSON with embedded secrets (AWS key prefixes, OpenAI key prefixes, GitHub PAT prefixes, Slack token prefixes, plus all `SENSITIVE_EXACT` / `SENSITIVE_PARTS` patterns), verify the output contains none. Complement with `proptest` for structured property testing.

Redaction defects are the highest-severity bug class. Contributors trust TC with raw traces that may contain credentials. ~100 LOC for fuzz + proptest targets, after PR #201 lands.

### 2.6 Cell Suppression Replacing DP

PR #238's analysis: at 352 submissions and 13/week, any meaningful epsilon destroys the aggregate signal. The DP mechanism is also structurally broken (noise derived from a hash including the true count — deterministic function of the thing it hides).

Replace with cell suppression: if a cell has fewer than k contributors (k=5 is standard), suppress the cell entirely. No epsilon accounting, no composition theorems. Update the analytics-release drill to verify: synthetic cohort below threshold gets suppressed, cohort at threshold is present. ~100 LOC.

---

## Part 3: Scale Readiness

### 3.1 Container Image

Current deploy: bare binary via Cloud Build + GCS + pull-and-install. Container adds reproducible environment, local parity, resource limits, portable restart policy.

Use `FROM gcr.io/distroless/cc-debian12:nonroot`. Distroless over Alpine: no shell = no shell-injection surface. `cc` variant includes `libgcc`/`libstdc++` for Rust binaries. Cloud SQL Auth Proxy runs as a sidecar container.

### 3.2 WebAuthn Ceremony State to Postgres

`CeremonyStore` is `Mutex<HashMap>` with TTL expiry, documented as single-process-only. Fix: short-TTL Postgres row keyed by ceremony ID with `consumed_at` to prevent replay, RLS with FORCE (consistent with every other table), periodic cleanup. ~100 LOC. Not needed until second instance.

### 3.3 Dedup Vector Index Persistence

`dedup_vector_index_id_map` is `Mutex<HashMap<u64, Uuid>>`, lost on restart. Persist to Postgres: table mapping usearch key (u64) to decision ID (UUID), load on startup, write-through on insert. One row per submission, fast even at scale. ~150 LOC.

### 3.4 Gate Evaluation as the Extraction Seam

Gate evaluation is the natural splitting point: synchronous external HTTP call (NEAR AI Cloud scorer), latency dominator, can scale independently from ingest.

**Phase 1 (now):** Instrument with OTel spans (1.1) and Prometheus metrics (1.2). Understand latency distribution.

**Phase 2 (~100/day):** Async evaluation via Postgres SKIP LOCKED queue. Submit → redact → chunk → embed → enqueue. Gate worker scores, writes result. Settlement picks up scored submissions. Background daemon (PR #244) is a natural home for the gate worker. Changes submission API from sync to async — contributors poll or receive SSE/webhook. ~300 LOC.

**Phase 3 (independent scaling):** Extract gate worker to its own binary. Smallest extraction: only needs scorer client + DB pool. Existing crate structure (`trace-commons-gate-api`, `trace-commons-gate-enclave`) already has the right boundaries.

### 3.5 Sleep-Time Pre-Computation

Pre-compute expensive results during idle windows (Lin et al., 2504.13171): embedding pre-computation for burst absorption, batch scoring economics via NEAR AI Cloud batch API during idle windows, dedup index compaction, cross-submission similarity consolidation. ~200 LOC.

### 3.6 When to Split

**Don't need a second instance when:** traffic < 100/day, concern is a slow query or scoring bottleneck (fix the query or go async per 3.4), want faster deploys (container + rolling restart).

**Do need a second instance when:** VM failure = complete downtime and you can't tolerate it, deploy without downtime (blue/green), regulatory/compliance geographic redundancy.

At that point, 2.1-2.3 and 3.2-3.3 should be done. Redis rate limiting (2.4) and gate extraction (3.4) can wait until the second instance serves real traffic.

---

## Priority Order

### Tier 1: This Week (~3-4 days)

1. Prometheus metrics endpoint (1.2) — ~80 LOC
2. tower-http TraceLayer (1.3) — 1 line
3. Graceful shutdown, ingest (2.1) — ~50 LOC
4. /health/ready (2.2) — ~100 LOC

### Tier 2: This Month (~5-7 days)

5. OTel instrumentation (1.1) — ~200 LOC
6. Cell suppression (2.6) — ~100 LOC
7. Migration runner extraction (2.3) — ~150 LOC
8. Slow query logging (1.6) — ~50 LOC

### Tier 3: This Quarter (~6-8 days)

9. Container image (3.1)
10. Redaction fuzzing (2.5) — after PR #201
11. Structured error types (1.5) — ~100 LOC
12. Dedup index persistence (3.3) — ~150 LOC
13. SLO formalization (1.4)

### Tier 4: When Traffic Demands It

14. Async gate evaluation (3.4) — ~100+/day
15. WebAuthn ceremony to Postgres (3.2)
16. Redis-backed rate limiting (2.4)
17. Sleep-time pre-computation (3.5)
18. Gate worker extraction (3.4)

### What's NOT on This List

- **Kubernetes.** Single VM + systemd + container runtime is simpler and sufficient.
- **Service mesh.** One service, one VM.
- **Self-hosted Jaeger/Tempo.** Grafana Cloud free tier handles pilot trace volume.
- **Blue/green deploys.** Container image is prereq (Tier 3); machinery is Tier 4.
- **Distributed tracing across services.** There's one service.

---

Open PRs that intersect: #244 (daemon), #201 (redaction), #238 (DP), #246 (revocation). Build graceful shutdown and OTel into the daemon from the start. Build fuzzing into the redaction leaf crate from the start.

Measure before you optimize, fail gracefully before you scale, automate deployment before you add instances.

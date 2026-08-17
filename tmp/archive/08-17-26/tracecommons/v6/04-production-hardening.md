# Production Hardening

**Date**: August 2026 (v6)

TraceCommons (TC) is an open-source Rust AI trace registry (~235K LOC, 6 crates, main binary at 70K LOC). Pilot runs on a single GCP VM (systemd + Caddy + Cloud SQL Auth Proxy). ~352 submissions to date, ~13/week. Traces are scored for quality and novelty inside TEEs (Trusted Execution Environments -- hardware-isolated encrypted compute on NEAR AI Cloud). Contributors earn NEAR blockchain credits for accepted traces. IronClaw (NEAR AI's agent runtime, 12.6K stars) is TC's primary integration partner (3 PRs merged, 20K+ lines).

Security posture: RLS with FORCE on every table, hash-only logging, fail-closed defaults, 17 operational drills, audit chain with write-time drift detection, canary self-test before scoring ticks, timing-oracle mitigation on redeem-confirm. The gaps below are additive -- observability, availability, and scale preparation as traffic grows from 13/week toward hundreds.

---

## What Shipped Since v5

| Item | PR | Status | Notes |
|---|---|---|---|
| Cell suppression replacing DP | #239 | **Merged** | k=5 threshold, replaces broken DP mechanism (noise derived from hash including true count -- deterministic function of the thing it hides) |
| CLI binary releases on tag | #240 | **Merged** | GitHub releases with binaries (pre-cargo-dist) |
| Background contributor daemon | #244 | **Merged** | Silent by default, weekly digest, configurable auto-submit |
| Gate API extraction | #212 | **Merged** | Gate evaluation as separate API surface |
| Third contributor (brapse) | #250 | **Merged** | First external contribution |

### What's Open

| Item | PR | Status | Notes |
|---|---|---|---|
| Logit capture design | #251 | **Open** | Design doc for capturing model logits in trace envelope |
| Private contributor insights via NEAR AI enclave | #241 | **Open** | TEE-hosted private analytics for contributors |
| Windows target support | #249 | **Open** | Cross-compilation for Windows |
| Linux GTK contributor shell | #248 | **Open** | Native Linux GUI for contributors |

---

## Part 1: Observability

### 1.1 OTel for TC's Own Pipeline

TC positions itself as the system that ingests OTel-format traces. Emitting them from its own pipeline is both credibility and necessity. Right now there's no single trace showing a submission's path through redaction, chunking, NEAR AI Cloud scoring (the latency dominator), gate evaluation, and settlement queueing.

**Instrument in priority order:**
1. **Submission pipeline.** Root span per submission, child spans for redaction, chunking, embedding, perplexity scoring, novelty scoring, gate evaluation, settlement queueing.
2. **Settlement batch.** Span per batch: credit computation, NEAR receipt outbox write, on-chain confirmation.
3. **Revocation propagation.** Span per revocation: content deletion, credit ledger adjustment, object ref cleanup. PR #246 changes semantics -- verify end-to-end.
4. **Background daemon** (PR #244, now merged). Periodic tasks emit spans for duty cycle visibility.

Setup: `opentelemetry` + `opentelemetry-otlp` + `tracing-opentelemetry`, add `OpenTelemetryLayer` to existing `tracing_subscriber` registry. ~200 LOC across ~10 call sites. Export to Grafana Cloud free OTLP endpoint (50GB traces/month, 14-day retention).

### 1.2 Prometheus Metrics Endpoint

`tower_http` already a dependency (CORS). Add `metrics` crate + `metrics-exporter-prometheus`, expose `/metrics`. ~50-80 LOC.

| Metric | Type | Labels |
|---|---|---|
| `tc_http_requests_total` | counter | method, route_family, status |
| `tc_http_request_duration_seconds` | histogram | method, route_family |
| `tc_submission_accepted_total` | counter | -- |
| `tc_gate_evaluation_duration_seconds` | histogram | gate_name |
| `tc_gate_evaluation_result` | counter | gate_name, result |
| `tc_settlement_batch_duration_seconds` | histogram | -- |
| `tc_near_scorer_duration_seconds` | histogram | -- |
| `tc_near_scorer_errors_total` | counter | error_class |
| `tc_redaction_applied_total` | counter | redaction_type |
| `tc_revocation_propagation_duration_seconds` | histogram | -- |
| `tc_db_pool_connections_active` | gauge | pool_name |
| `tc_db_pool_connections_idle` | gauge | pool_name |
| `tc_dedup_index_size` | gauge | -- |
| `tc_audit_chain_length` | gauge | -- |

Route families should be coarse (submission, review, admin, account, settlement, worker) to avoid cardinality explosion. The stable error class names already in use (`AuditChainDriftRejected`, etc.) work as metric labels.

Scraping: Grafana Cloud free remote-write agent or `grafana-agent` / `vector` to scrape and push. No local Prometheus needed.

### 1.3 tower-http TraceLayer

One line: `app.layer(TraceLayer::new_for_http())`. Wire `on_response` to record histogram from 1.2.

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

Wrap pool execution with threshold check (~500ms). Log parameterized query template (never bound values -- hash-only logging extends to queries). Record `tc_db_slow_query_seconds` histogram. ~50 LOC.

---

## Part 2: Reliability

### 2.1 Graceful Shutdown for trace-commons-ingest

At 13/week this is academic. At real traffic, deploying without connection draining kills in-flight gate evaluations (seconds-long NEAR AI Cloud calls), leaves partial settlement state, and drops in-progress revocation deletions.

Mirror upload-claim issuer: `tokio::signal` for SIGTERM/SIGINT, `axum::serve(...).with_graceful_shutdown(shutdown_signal())`, grace period for in-flight gate evaluations. ~50 LOC.

For background daemon (PR #244, now merged): graceful shutdown = flush pending consolidation, checkpoint dedup index, write resume marker.

### 2.2 /health/ready

Current `/health` returns `{"status":"ok","schema_version":"..."}` -- liveness, not readiness. A readiness probe answers "can this process serve traffic right now?"

Checks: (1) each privilege-scoped DB pool can acquire a connection and `SELECT 1`, (2) PII backstop canary passes (already wired), (3) scorer reachable via lightweight probe (if configured), (4) migration tracking table shows expected schema version. Return 200 when critical checks pass, 503 when any fails, per-check latency in the response body. ~100 LOC.

### 2.3 Migration Runner Extraction

`connect_from_config` calls `backend.run_migrations().await?` on every startup. The tracking table provides idempotency, but concurrent startups can interleave DDL.

Extract to a `--migrate` flag or separate binary. Add `pg_advisory_lock` to serialize concurrent migration runners -- without it, two runners race; with it, the second blocks until the first completes, then discovers everything is applied. ~150 LOC. Matters for: blue/green deploys, container restarts, daemon alongside main binary.

### 2.4 Rate Limiting

**Phase 1 (now):** In-process `Mutex<HashMap>`. Add `tc_rate_limit_hits_total` counter. ~20 LOC.
**Phase 2 (second instance):** Redis INCR + EXPIRE. ~200 LOC.

### 2.5 Redaction Fuzzing

PR #201 proposes `trace-commons-redaction` leaf crate -- prerequisite. Fuzz with `cargo-fuzz`: generate adversarial JSON with embedded secrets (AWS key prefixes, OpenAI key prefixes, GitHub PAT prefixes, Slack token prefixes, plus all `SENSITIVE_EXACT` / `SENSITIVE_PARTS` patterns), verify the output contains none. Complement with `proptest` for structured property testing. Redaction defects are the highest-severity bug class. ~100 LOC after PR #201.

### 2.6 Cell Suppression -- DONE (PR #239)

**Merged.** k=5 threshold. Replaced broken DP mechanism (noise derived from hash including true count -- deterministic function of what it hides).

---

## Part 3: Scale Readiness

### 3.1 Container Image

Current deploy: bare binary via Cloud Build + GCS + pull-and-install. A container image adds reproducible environment, local parity, resource limits, portable restart policy.

`FROM gcr.io/distroless/cc-debian12:nonroot`. Distroless over Alpine: no shell means no shell-injection surface. `cc` variant includes `libgcc`/`libstdc++` for Rust binaries. Cloud SQL Auth Proxy runs as a sidecar container.

### 3.2 WebAuthn Ceremony State to Postgres

`CeremonyStore` is `Mutex<HashMap>` with TTL. Fix: short-TTL Postgres row with `consumed_at`, RLS with FORCE. ~100 LOC. Not needed until second instance.

### 3.3 Dedup Vector Index Persistence

`dedup_vector_index_id_map` is `Mutex<HashMap<u64, Uuid>>`, lost on restart. The simhash signal (DB-persisted) degrades gracefully, but the gap between simhash-only and full vector dedup widens with corpus growth.

Persist the ID map to Postgres: table mapping usearch key (u64) to decision ID (UUID), load on startup, write-through on insert. One row per submission, fast even at scale. ~150 LOC.

### 3.4 Gate Evaluation as the Extraction Seam

**Phase 1 (now):** Instrument with OTel + Prometheus. Understand latency.
**Phase 2 (~100/day):** Async via Postgres SKIP LOCKED queue. Background daemon (PR #244) as natural home. ~300 LOC.
**Phase 3 (independent scaling):** Extract gate worker to own binary.

### 3.5 Sleep-Time Pre-Computation

Pre-compute during idle: embedding pre-computation, batch scoring via NEAR AI Cloud batch API, dedup index compaction, cross-submission similarity consolidation. ~200 LOC.

### 3.6 When to Split

**Don't need a second instance when:** traffic < 100/day, concern is a slow query or scoring bottleneck (fix the query or go async per 3.4), want faster deploys (container + rolling restart).

**Do need a second instance when:** VM failure = complete downtime and you can't tolerate it, deploy without downtime (blue/green), regulatory/compliance geographic redundancy.

At that point, 2.1-2.3 (graceful shutdown, health/ready, migration extraction) and 3.2-3.3 (ceremony state, dedup persistence) should be done. Redis rate limiting (2.4) and gate extraction (3.4) can wait until the second instance serves real traffic.

---

## Priority Order

Ranked by impact/effort for a 2-person team at pilot scale.

### Tier 1: This Week (~3-4 days)

| # | Item | Ref | Why first |
|---|------|-----|-----------|
| 1 | Prometheus metrics endpoint | 1.2 | ~80 LOC. Need numbers before you can improve anything. Everything else builds on this. |
| 2 | tower-http TraceLayer | 1.3 | One line. Instant structured request logging with latency. |
| 3 | Graceful shutdown (ingest) | 2.1 | ~50 LOC, modeled on existing code. Prevents lost work on deploy. |
| 4 | /health/ready | 2.2 | ~100 LOC. Exercises DB + canary + scorer. Foundation for deploy automation. |

### Tier 2: This Month (~5-7 days)

| # | Item | Ref | Why next |
|---|------|-----|----------|
| 5 | OTel instrumentation | 1.1 | Once metrics exist, you want traces. Submission pipeline first. Eat your own dog food. |
| 6 | ~~Cell suppression (2.6)~~ | -- | **DONE** (PR #239). |
| 7 | Migration runner extraction | 2.3 | ~150 LOC. Eliminates a class of deploy failure. Required before containerization. |
| 8 | Slow query logging | 1.6 | ~50 LOC. Surfaces DB issues before users notice. Valuable with 5+ pools. |

### Tier 3: This Quarter (~6-8 days)

| # | Item | Ref | Why now |
|---|------|-----|---------|
| 9 | Container image | 3.1 | Prereq for modern deploy. After migration extraction. |
| 10 | Redaction fuzzing | 2.5 | After PR #201. Highest-severity bug class deserves fuzzing. |
| 11 | Structured error types | 1.5 | ~100 LOC core. Makes Prometheus error counting automatic. |
| 12 | Dedup index persistence | 3.3 | ~150 LOC. No more dedup state loss on restart. |
| 13 | SLO formalization | 1.4 | Write them down. No code, just discipline atop the metrics from item 1. |

### Tier 4: When Traffic Demands It

| # | Item | Ref | Trigger |
|---|------|-----|---------|
| 14 | Async gate evaluation | 3.4 | ~100+ submissions/day |
| 15 | WebAuthn ceremony to Postgres | 3.2 | Second instance for availability |
| 16 | Redis-backed rate limiting | 2.4 | Second instance for availability |
| 17 | Sleep-time pre-computation | 3.5 | Batch costs or duty cycle becomes a concern |
| 18 | Gate worker extraction | 3.4 | Gate scaling independent of ingest |

### What's NOT on This List

- **Kubernetes.** Single VM + systemd + container runtime is sufficient.
- **Service mesh.** One service, one VM.
- **Self-hosted Jaeger/Tempo.** Grafana Cloud free tier handles pilot volume.
- **Blue/green deploys.** Container image is prereq (Tier 3); machinery is Tier 4.
- **Distributed tracing across services.** There's one service.

---

Open PRs that intersect: #201 (redaction), #246 (revocation), #251 (logit capture), #241 (private insights). Build graceful shutdown and OTel into the daemon. Build fuzzing into the redaction leaf crate.

Tiers 1-3 make the single-instance pilot robust and observable. Tier 4 handles multi-instance when traffic demands it.

Measure before you optimize, fail gracefully before you scale, automate deployment before you add instances.

# Production Hardening & Integrations

**Date**: August 2026

TC runs on a single GCP VM (systemd + Caddy + Cloud SQL Auth Proxy). ~352 submissions, ~13/week. Security posture is strong (RLS with FORCE, hash-only logging, fail-closed defaults, 17 operational drills). The gaps below are additive: observability, availability, and integration surface as traffic grows.

---

## Part 1: Production Hardening

### Tier 1: This Week (~3-4 days)

| # | Item | Effort | Why First |
|---|---|---|---|
| 1 | **Prometheus metrics endpoint** | ~80 LOC | Need numbers before you can improve anything |
| 2 | **tower-http TraceLayer** | 1 line | Instant structured request logging with latency |
| 3 | **Graceful shutdown (ingest)** | ~50 LOC | Prevents lost work on deploy |
| 4 | **/health/ready** | ~100 LOC | Exercises DB + canary + scorer. Foundation for deploy automation |

**Metrics to emit**: `tc_http_requests_total`, `tc_http_request_duration_seconds`, `tc_submission_accepted_total`, `tc_gate_evaluation_duration_seconds`, `tc_gate_evaluation_result`, `tc_settlement_batch_duration_seconds`, `tc_near_scorer_duration_seconds`, `tc_redaction_applied_total`, `tc_db_pool_connections_active/idle`.

### Tier 2: This Month (~5-7 days)

| # | Item | Effort | Why Next |
|---|---|---|---|
| 5 | **OTel instrumentation** (TC's own pipeline) | ~200 LOC | Root span per submission through redaction→chunking→scoring→settlement |
| 6 | **Cell suppression replacing DP** | ~100 LOC | DP is structurally broken at 352 submissions (PR #238 analysis). Cell suppression with k=5 |
| 7 | **Migration runner extraction** | ~150 LOC | Eliminates deploy failure class. Required before containerization |
| 8 | **Slow query logging** | ~50 LOC | 5+ DB pools, parameterized templates only (hash-only extends to queries) |

### Tier 3: This Quarter (~6-8 days)

| # | Item | Effort | Why Now |
|---|---|---|---|
| 9 | **Container image** (distroless, nonroot) | Half-day | Prereq for modern deploy |
| 10 | **Redaction fuzzing** (after PR #201) | ~100 LOC | Highest-severity bug class deserves fuzzing |
| 11 | **Structured error types** | ~100 LOC | Makes Prometheus error counting automatic |
| 12 | **Dedup index persistence** | ~150 LOC | No more state loss on restart |
| 13 | **SLO formalization** | No code | Write them down. p99 targets: submission acceptance <500ms, gate eval <30s, settlement <5min |

### Tier 4: When Traffic Demands It

- Async gate evaluation via Postgres SKIP LOCKED queue (~100+/day)
- WebAuthn ceremony state to Postgres (second instance)
- Redis-backed rate limiting (second instance)
- Sleep-time pre-computation (batch costs or duty cycle concern)
- Gate worker extraction (independent scaling)

---

## Part 2: Integrations

### Priority 1: OTel-Native Ingest (Highest Leverage)

**What**: OTLP receiver (gRPC + HTTP/protobuf) accepting OTel GenAI and OpenInference spans, mapped to `TraceContributionEnvelope` via a version-pinned adapter. The `opentelemetry-proto` + `tonic` Rust crates handle the transport.

**Why**: Any team already using Langfuse, Datadog, Arize Phoenix, or MLflow pipes existing telemetry to TC with a config change. Integration cost drops from "learn our SDK" to "add an exporter endpoint."

**Key components**:
- Attribute mapping layer: `gen_ai.request.model` → `envelope.model`, `gen_ai.usage.*` → token counts, tool-call spans → `ToolCallEvent`
- Span-to-envelope assembly (walk span tree, identify agent roots, construct envelope)
- Redaction on ingest (OTel spans carry raw content; run existing redaction pipeline identically)
- Version-pin the mapping (conventions are pre-stable, v1.42.0)

**Effort**: 2-4 weeks.

### Priority 2: Error Hub / Failure Commons

**What**: Searchable collection of scrubbed failure-diagnosis-repair bundles. Failure-attribution stage identifies root cause, diagnosis path, and repair diff.

**What to build**:
- Failure-attribution gate extension (failure type, root-cause span, diagnosis steps, repair diff)
- Bundle schema extending envelope with failure metadata
- Search interface: `tc search-failures "cargo build failed"` + API endpoint
- Novelty scoring extension for failure dimension

**Effort**: 6-8 weeks.

### Priority 3: Agent Skills Publishing

**What**: Mine corpus for recurring high-quality patterns → publish as SKILL.md files (~40 compatible products).

**What to build**:
- Manual curation via `tc skill publish` (initial version)
- Security scanner (injection detection, code execution analysis, data exfiltration checks)
- Attribution tracker (map skills to contributing traces, flow credit back)

**Effort**: 1-2 weeks (manual), 12-16 weeks (automated extraction).

### Priority 4: Protocol-Level Events

**What**: First-class MCP tool-call events, A2A delegation events, W3C trace context.

- MCP tool calls ship alongside OTel ingest (most common, best-standardized)
- A2A delegation when multi-agent traces appear in meaningful volume
- W3C trace context for cross-organizational trace stitching (needs bilateral opt-in)

**Effort**: MCP 1-2 weeks, A2A 4-6 weeks, W3C 2-3 weeks.

### Priority 5: Trajectory Replay

**What**: Cross-harness replay interface rendering TC traces as navigable step-by-step trajectories.

- Start with terminal-based viewer (`tc replay <trace-id>`)
- SSE replay stream at configurable speed
- Anonymization layer for network replay (heavy redaction by default)

**Effort**: 8-10 weeks.

---

## Part 3: IronClaw Integration Status

### Shipped (Working End-to-End)

```
agent turn → capture → policy check → envelope → redact →
credential resolve → JWT → ingest → gate → credit
```

3 PRs merged on IronClaw (~20K lines), 1 on TC server. 6 modules in `ironclaw_trace_commons` crate consumed by 5 workspace crates.

### Critical Fixes (Before New Features)

| Issue | What | Fix |
|---|---|---|
| **No TLS enforcement** | HTTP client builders don't validate HTTPS. Bearer tokens exposed on plaintext. | Scheme check at construction time (~10 lines) |
| **Quarantine check: stringly-typed** | Substring match on `"quarantined"`. If TC changes wording, check silently stops matching. | `TraceStatus` enum or exhaustive match with quarantine-by-default |
| **Empty-bytes redaction_hash** | Serialization failure swallowed → hash of empty bytes → audit trail broken. | Propagate error; reject trace rather than submit with meaningless hash |
| **No behavioral tests** (ContributionHttpSink) | HTTP layer fully mocked. No test has made a real request to a TC-compatible endpoint. | `wiremock`-based integration test |

### High-Impact IronClaw Opportunities

1. **Immediate scoring feedback**: TC ingest returns quality score in response → IronClaw surfaces inline ("Quality: 92/100, top 15% this week, 3.2 TC credits")
2. **WASM fuel as quality signal**: Per-tool-call fuel consumption as manipulation-resistant process efficiency metric
3. **Cross-provider comparison**: Same quality metrics across 26 providers on real-world tasks (track separately at first, don't publish provider rankings initially)
4. **Onboarding-time opt-in**: Surface TC contribution during agent setup, not buried in settings

---

## Roadmap

```
Month 1:  Tier 1 production hardening + prebuilt binaries + self-service registration
Month 2:  OTel ingest + MCP tool-call events + IronClaw critical fixes
Month 3:  Error Hub MVP + `tc scan` with insights
Month 4:  Skill publishing (manual curation) + Tier 2 production hardening
Month 5:  Trajectory replay prototype
Month 6:  A2A delegation events + auto skill extraction exploration
```

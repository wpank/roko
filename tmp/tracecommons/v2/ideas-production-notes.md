# TraceCommons Production Notes

Notes from reading the `trace-commons-server` codebase and open PRs.
Not an audit, not prescriptions -- just observations and ideas from
spending time in the code.

Date: 2026-08-10

---

## 1. What's Already Strong

I want to start here because it's easy to skip straight to gaps and miss
the fact that this codebase has an unusually mature security posture for
a pilot-stage project.

**RLS everywhere, forced.** Every Trace Commons table has PostgreSQL
Row-Level Security with `FORCE` semantics. The V6 migration
(`V6__trace_force_rls.sql`) applies `ALTER TABLE ... FORCE ROW LEVEL
SECURITY` and the V18 migration centralizes the tenant predicate through
`trace_current_tenant_id()`. There are separate DB pools for different
privilege levels -- contributor reads, reviewer reads, replay-export
reads, audit reads, tenant-policy reads -- and the runtime refuses to
start if RLS isn't verified ready. This is the kind of thing that's
usually promised in a design doc and never actually done. Here it's done,
and there's a drill (`/v1/admin/postgres-rls-drill`) that verifies it
stays done.

**Hash-only logging.** The CLAUDE.md is emphatic: "Never include raw
URLs, bearer tokens, ARNs, account references, transaction hashes,
contributor identity, trace bodies, or any operator-secret material in
stored rows or log strings." Reading through the handlers, this is
actually followed. Audit rows carry hashes. Error messages use
safe-label classes like `AuditChainDriftRejected` rather than
dumping the offending data. The admin operational surfaces
(`/v1/admin/config-status`, `/v1/admin/operational-summary`) expose
only boolean, label, and hash fields.

**Fail-closed defaults.** When WebAuthn config is partial, passkeys go
to 503 not to a fallback. When the KEK isn't production-grade, the
process refuses to start (`TRACE_COMMONS_KEK_REQUIRE_PRODUCTION_TRUST_BOUNDARY`).
When NEAR sign-in config is incomplete, the surface fails closed. When
the attestation signing key is partially configured, startup fails
rather than silently disabling. The pattern is consistent:
`Option<Arc<...>>` where `None` means "503, not silently skip."

**17 operational drills.** The `/v1/admin/*-drill` endpoints are
self-test exercises that verify critical subsystems: rollback, key
rotation, audit chain, DB reconciliation, RLS, retention, vector index,
analytics release, benchmark readiness, revocation propagation,
revocation effects, canary reads, object primary reads, object store
migration, credit settlement, and ranking readiness. These produce
hash-only evidence and feed the rollout-smoke checks. Having this
many operational tests baked into the binary is unusual and genuinely
useful for operator confidence.

**Audit chain with drift detection at write time.** The
`audit_event_matches_writeback` function in `audit_chain.rs` verifies
at every append that the mirrored DB row's `event_hash` and
`previous_event_hash` match the canonical file-format event. This
catches column drift at write time rather than waiting for a periodic
drill. The design -- canonical truth in the append-only file, DB mirror
verified on every write -- is simple and correct.

**Canary self-test before batch processing.** The PII backstop runs a
synthetic canary through the privacy filter before every scoring tick.
If the canary reports unhealthy, the tick aborts entirely. This is
defense-in-depth against a regression in the redaction pipeline silently
letting data through.

**Rate limiting with timing-oracle mitigation.** The account surface
imposes a `REDEEM_MIN_LATENCY` (250ms) floor on every redeem-confirm
response -- success and every deny alike -- removing the
found-vs-not-found timing oracle. The rate limiter has per-IP,
per-code, per-credential, and global caps, all with explicit comments
explaining their purpose.

---

## 2. Observations: Single-Process Limitations

The pilot runs on a single GCP VM with systemd units and a Cloud SQL
Auth Proxy sidecar. Everything is correctly scoped for single-process
deployment, and the code is honest about it. But it's worth enumerating
the in-process state that would need to move if the deployment ever
needed a second instance.

**WebAuthn CeremonyStore.** The `account_passkey.rs` module header
says it outright: "CeremonyStore keeps ceremony state in process memory
(Mutex + HashMap). It is therefore correct ONLY for a single-process
deployment: a challenge issued by one process cannot be completed by
another, and state is lost on restart." The store has TTL-based
expiry and is threaded through `AppState` (not a global), so it's
already injectable. A short-TTL row in Postgres keyed by ceremony ID
would be the natural replacement -- the module docs even suggest it.

**Rate limiting state.** The account-surface rate limiter is explicitly
documented as in-process: "this pilot runs on ONE host, so the rate
limiter below is in-process (a Mutex<HashMap> of fixed-window counters).
It is NOT distributed." The `InstanceRateLimiter` in
`instance_enroll_guard.rs` uses the same pattern. Both have comments
noting that multi-instance deployment would multiply the effective limit
by the instance count.

**Dedup vector index ID map.** The `dedup_vector_index_id_map` on
`AppState` is a `Mutex<HashMap<u64, Uuid>>` that maps usearch keys
back to decision IDs. It's process-local, grows unbounded for the
process lifetime, and is lost on restart. The doc comment is
thorough: "after a restart a query misses any vector inserted before
the restart (its key is unknown and is SKIPPED -- never fabricated)."
The simhash signal, being DB-persisted, is unaffected.

**Login link redemption.** Login links are DB-backed (the
`trace_login_links` table with `consumed_at` timestamps), so they
survive restarts and work across instances already. This one is actually
fine for multi-instance.

One thing worth considering: for the pilot's current trajectory, these
single-process constraints are probably fine for quite a while. The
corpus is at ~352 submissions and growing at ~13/week. At that pace,
vertical scaling (bigger VM) is simpler than introducing Redis or
DragonflyDB for shared state. The crossover point is probably
"when you need to deploy behind a load balancer for availability, not
throughput." And at that point, a short-TTL Postgres row for ceremonies
and a Redis-backed rate limiter are both well-understood patterns.

---

## 3. Observations: Metrics and Observability

I noticed there's no Prometheus or OpenTelemetry export anywhere in the
codebase. The `tower_http` dependency is used for CORS
(`tower_http::cors::CorsLayer`) but not for the trace layer. Structured
logging is in place via `tracing` and `tracing_subscriber`, but there's
no metrics export path.

The roadmap mentions an `operational-metrics` Prometheus exporter in
passing, but it's not implemented. For a pilot with 13 submissions/week,
this is fine -- the operational drills provide a heartbeat, and the
admin endpoints give snapshot visibility. But once the pilot starts
accepting real contributor traffic, a few specific metrics would be
worth having:

- **Request latency histograms** by route family (submission, review,
  settlement, admin). The 250ms timing floor on redeem-confirm is a
  good example of latency-aware design -- but there's no way to see
  whether real requests are hitting that floor or not.
- **Gate evaluation latency.** The NEAR AI Cloud TEE scoring path is
  an HTTP call to an external service. Knowing its p50/p95/p99 would
  inform whether the gate is a bottleneck.
- **Error rates by class.** The error types use stable class names
  (e.g., `AuditChainDriftRejected`). Counting them per window would
  surface degradation without exposing sensitive data.
- **Settlement batch latency.** The credit pipeline is end-to-end
  complete with a NEAR receipt outbox. How long from submission
  acceptance to on-chain settlement?
- **Queue depths.** The revocation-propagation scheduler and the
  ranking worker both maintain queues. Depth over time shows whether
  work is draining or accumulating.

The implementation path is straightforward: `metrics` crate (the Rust
ecosystem standard) with `metrics-exporter-prometheus` gives a
`/metrics` endpoint. The `axum-prometheus` crate wraps this into a
tower layer that auto-instruments all routes. Total integration is
probably ~50 lines of code.

Alternatively, if the goal is to avoid running a Prometheus scraper on
the pilot host, pushing to Grafana Cloud's OTLP endpoint via
`opentelemetry-otlp` works without local infrastructure. But that's
more plumbing than the `metrics` crate approach.

---

## 4. The Redaction Re-Scan Problem (PR #201)

I found the redaction code in `crates/trace-commons-protocol/src/redaction.rs`.
The current redactor uses a two-tier approach: exact key matches
(`SENSITIVE_EXACT`) and compound key detection with context-aware
part matching (`SENSITIVE_PARTS`, `TOKEN_PARTS`, `KEY_PARTS`,
`CONTEXT_PARTS`, and camelCase splitting). It's deterministic and
recursive over JSON structures.

The bug that PR #201 addresses is a tokenization defect: `api_key=SECRET`
(equals-delimited) was passing through while `api_key: SECRET`
(colon-delimited) was correctly redacted. This is the kind of edge case
that's easy to miss in a key-value redactor -- the redactor thinks in
JSON keys, but trace data can contain URL query parameters, environment
variable dumps, config file fragments, and other formats where the
key-value delimiter isn't a colon.

What I noticed about the proposed fix is that it's forward-only by
design. The interesting constraint is the re-scan: the corpus already
has data at rest that may contain the leaked patterns, and the at-rest
security boundary means you can't just run a decryptor on AppState.
The proposed approach -- extract the deterministic redactor into a
leaf `trace-commons-redaction` crate, then do a two-phase operator
action (audit pass to identify affected rows, then rewrite pass with
explicit operator approval) -- is the right shape. It keeps the
decryption authority narrow and the operation auditable.

This is exactly the kind of privacy-first thinking that makes TC
credible as a trace steward. The instinct to treat a redaction miss
as a "retroactive fix needed" rather than "oh well, it's already
stored" is the correct instinct for a system where contributors are
trusting you with their agent traces.

One thing worth considering: the leaf crate extraction also creates a
natural seam for fuzz testing the redactor. A standalone crate with
a `fn redact(input: &Value) -> Value` signature is trivially
fuzzable with `cargo fuzz` or `proptest`. Given that redaction defects
are among the highest-severity bugs this system can have, investing
in a property test that generates adversarial JSON with embedded
secrets in various delimited formats would pay for itself quickly.

---

## 5. The Honest DP Decision (PR #238)

PR #238's analysis is one of the most refreshingly honest pieces of
technical writing I've encountered in this kind of project. The
conclusion: at 352 submissions and 13/week, any epsilon value that
provides meaningful privacy completely destroys the aggregate signal.
The recommendation is to drop the differential privacy claim rather
than maintain a mechanism that doesn't actually protect anything.

I looked at the existing implementation. The `noisy_analytics_count`
function and the `TraceAnalyticsNoiseConfig` with its
`epsilon_micros_per_release` and `max_epsilon_micros` fields are wired
throughout the analytics surface -- `apply_broad_release_noise` touches
every aggregate field (submissions_total, evaluated_traces,
duplicate_groups, etc.). The PR notes that the noise derivation is
broken beyond repair: it's derived from a hash that includes the true
count, which means the "noise" is a deterministic function of the thing
it's supposed to hide. This isn't fixable by tuning parameters -- the
mechanism itself is structurally unsound.

The alternative proposed -- cell suppression with cohort-size floors --
is a real protection mechanism. If a cell has fewer than k contributors,
suppress it entirely. This is what the Census Bureau does, and it's
what actually works at small corpus sizes where DP noise would dominate.
It's also vastly simpler to implement correctly: no epsilon accounting,
no composition theorems, just "is this cell big enough to release?"

One thing worth considering: the analytics-release drill
(`/v1/admin/analytics-release-drill`) should probably be updated to
verify that cell suppression is working correctly when the DP code is
removed. The drill is already wired and produces hash-only evidence --
extending it to verify suppression thresholds is straightforward.

---

## 6. The Revocation Trust Issue (PR #246)

I noticed that the revocation system has extensive infrastructure -- a
propagation worker, per-item retry caps (`revocation_propagation_max_attempts`),
a scheduler, effects drills, credential-scoped bearer tokens, and
worker-queue invalidation (V22 migration). The revocation
propagation handles object refs, credit ledger reversals, and
service-owned artifact deletion.

PR #246 identifies a fundamental trust issue: revocation was (1)
clawing back credit from the contributor AND (2) NOT actually deleting
their content. From a contributor's perspective, this is the worst
possible outcome -- you asked for your data back, you lost the credit
you'd earned, and your data is still sitting on someone else's server.

The fix -- credit stays, content actually gets deleted -- realigns the
incentives correctly. A contributor who revokes should lose access to
future credit from that submission (because the data is no longer in
the corpus), but shouldn't be punished for exercising a right that
the system promises them.

This is foundational for user acquisition. Contributors evaluating
whether to opt in will look at the revocation path first. "What
happens if I change my mind?" is the trust question. The answer needs
to be "your data is actually deleted and you keep what you earned while
it was in the corpus." The alternative -- any form of punitive
revocation -- is a one-way trust destroyer.

One thing worth considering: the revocation-effects drill
(`/v1/admin/revocation-effects-drill`) should be updated to verify
that content is actually gone, not just that the revocation record
exists. The distinction is exactly what PR #246 is fixing -- the
drill was presumably passing before the fix because it was checking
for the revocation record, not for the absence of the content.

---

## 7. Ideas: Deployment Modernization

The current deployment is bare binary on a GCP VM with systemd units,
Caddy as a reverse proxy with ACME, and Cloud SQL Auth Proxy as a
sidecar. The `cloudbuild.yaml` builds on ubuntu:24.04 and pushes
binaries to GCS; `pull-and-install.sh` on the host pulls and installs.
SHA256 checksums are generated alongside each binary.

This is actually a reasonable setup for a pilot. A few thoughts on
what would help as the deployment matures:

**Container image.** Not strictly required -- the bare binary approach
works and avoids container complexity -- but a container image gives
you reproducible builds, hermetic dependencies, and the ability to
run the exact same artifact locally that runs in production. The
`cloudbuild.yaml` already does a from-scratch build on ubuntu:24.04;
wrapping the output in a `FROM scratch` or `FROM gcr.io/distroless/cc`
image is minimal additional work. The systemd units could then use
`podman` or Docker instead of a bare binary, gaining restartability
and resource limits for free.

**Migration runner extracted from startup.** Currently,
`connect_from_config` calls `backend.run_migrations().await?` during
connection setup -- meaning every process startup runs the migration
check. For a single instance this is fine, but if you ever have two
instances starting simultaneously (deploy, restart, etc.), you get a
migration race. The migration logic in `PgBackend::run_migrations`
does use a `_trace_commons_migrations` tracking table, which provides
idempotency, but two concurrent migration runners can still interleave
DDL statements in surprising ways. Extracting migration to a separate
binary or a `--migrate` flag that runs before the server starts would
eliminate this class of problem.

**Graceful shutdown.** The upload-claim issuer has a thorough graceful
shutdown implementation (`serve_both_with_graceful_shutdown` in
`trace_upload_claim_issuer.rs`) with SIGTERM/SIGINT handling, oneshot
channels for each axum::serve instance, and a configurable grace
window. I did not find equivalent graceful-shutdown wiring in the main
`trace-commons-ingest` binary -- it listens and serves but doesn't
appear to handle SIGTERM with connection draining. For a pilot with
13 submissions/week this is academic, but once the server is taking
real traffic, a deployment that kills in-flight requests (especially
mid-gate-evaluation, which involves the NEAR AI Cloud HTTP call) would
lose work.

**Health check depth.** The `/health` endpoint returns
`{"status":"ok","schema_version":"..."}` without exercising any
subsystem. This is fine as a TCP-alive check, but the 17 operational
drills show that the team understands the value of deep health checks.
One thing worth considering: a `/health/ready` endpoint that verifies
the DB pool can acquire a connection, the canary self-test passes, and
(if configured) the NEAR AI scorer is reachable. This separates
"process is up" from "process can serve traffic" -- useful for
load-balancer health checks if the deployment ever goes multi-instance.

---

## 8. Ideas: Observability Quick Wins

A few small things that would improve operational visibility without
requiring a full observability stack:

**tower-http TraceLayer.** The `tower_http` dependency is already in
`Cargo.toml` (used for CORS). Adding `TraceLayer::new_for_http()` to
the middleware stack gives structured request/response logging with
latency, status code, and route for free. This is a one-line addition
to the router builder and immediately makes the structured logs
useful for latency analysis.

**Structured error types with request context.** The error handling
uses `anyhow` extensively, which is fine for error propagation but
loses structure at the logging boundary. Wrapping the top-level
handler errors in a struct that carries (at minimum) the error class,
the route family, and whether the error is client-facing or internal
would make log analysis much easier without exposing sensitive data.

**Slow query logging.** The Postgres pool configuration
(`deadpool-postgres` or equivalent) likely supports query-duration
logging. Setting a threshold (say, 500ms) and logging slow queries
with their parameterized form (not their bound values) would surface
DB performance issues before they become user-visible. The separate
pools for different privilege levels make this especially valuable --
a slow query on the contributor-reads pool has different implications
than one on the audit-reads pool.

**Simple SLO targets.** Even informal ones help. For example:
- Submission acceptance: p99 < 500ms (excluding gate evaluation)
- Gate evaluation: p99 < 30s (NEAR AI Cloud round-trip)
- Credit settlement batch: < 5 minutes from trigger to NEAR receipt
- Health check: < 100ms

These don't need automated alerting to be useful. Just having them
written down gives the drills a performance dimension and makes it
possible to notice degradation during the pilot.

---

## 9. The Monolith Question

The main binary is 70,372 lines. The test file is another 80,809 lines.
The full codebase is ~235K lines of Rust across 6 crates. The CLAUDE.md
says "do not unilaterally split files unless a spec asks for it."

I think this is the right call at this stage, and I want to explain why
rather than reflexively suggesting microservices.

The monolith has real advantages for this project:

- **Single deployment artifact.** The cloudbuild produces one binary
  per service. There's no service mesh, no inter-service auth, no
  distributed tracing, no saga coordination. A request enters the
  process and either completes or fails within the same address space.
- **Shared AppState.** The `AppState` struct holds the DB pools, the
  ceremony store, the rate limiters, the dedup index, and the
  configuration. Every handler has access to everything it needs
  through a single `Arc<AppState>`. Splitting this across services
  would require either shared state infrastructure (Redis) or
  duplicated state (multiple DB pools, multiple ceremony stores).
- **Atomic migrations.** The migration runner applies all 41
  migrations in sequence on startup. With multiple services owning
  different tables, migration ordering becomes a coordination problem.
- **Test cohesion.** The 80K-line test file exercises end-to-end
  flows that cross handler boundaries (submit -> gate -> settle ->
  revoke). Splitting the binary would require integration tests with
  real HTTP between services.

The CLAUDE.md instruction is practical: "Files have grown large. Do
not unilaterally split files unless a spec asks for it. Add new
modules beside existing code when possible." This is the voice of
experience -- someone tried splitting and it made things worse.

The natural splitting point, if it ever comes, is gate evaluation.
The NEAR AI Cloud scoring path is the one handler that involves a
synchronous external HTTP call to an LLM. If gate evaluation latency
or throughput ever becomes the bottleneck, extracting it into a
separate service that communicates via a queue (submit -> enqueue ->
worker evaluates -> writes result -> settlement picks up) would let
ingest and gate scale independently. But that's a "when the numbers
demand it" decision, not a "because microservices are better"
decision.

The existing crate structure (`trace-commons-protocol`,
`trace-commons-gate-api`, `trace-commons-gate-enclave`,
`trace-commons-contributor`, `trace-commons-operator-client`,
`trace-commons-server`) already provides good module boundaries.
The gate API is behind traits (`PerplexityScorer`,
`VectorIndex`), the protocol DTOs are shared, and the server crate
contains the binaries. If a split is ever needed, the seams are
already there.

---

## 10. One More Thing: The 70K LOC Binary

I want to call out something specific about the `trace-commons-ingest.rs`
file. 70K lines in a single file is large by any standard. But reading
through it, the code is well-organized within the file: handlers are
grouped by surface (submission, review, admin, account, settlement,
worker), each with consistent auth patterns. The worker-route
credentials are scoped per surface (utility, review, retention,
vector, benchmark, process-evaluation, revocation, export,
revocation-propagation). The dead-code allowances are documented with
comments explaining which future task will wire them.

The test extraction to
`trace_commons_ingest_internal/tests.rs` via `#[path = ...]` was a
good call -- keeping 80K lines of tests in the same file would make
the file genuinely unworkable. The pattern works: production code in
one file, tests in a sibling, connected by a path attribute.

The risk with this approach isn't readability (the code is well-commented)
or maintainability (the patterns are consistent) -- it's compile times.
A 70K-line file is a single compilation unit, and incremental compilation
can only help if the file didn't change. Any edit to any handler
recompiles the entire binary. For a team working on different features
in parallel, this means every PR touches the same file and every build
recompiles the whole thing.

The CLAUDE.md instruction against splitting is correct for the current
team size and velocity. But if the team grows, the compile-time tax
may become the forcing function for extracting handler groups into
modules -- not for architectural reasons, but for build-time reasons.

---

## References

- `crates/trace-commons-server/src/bin/trace-commons-ingest.rs` --
  main server binary (70K LOC)
- `crates/trace-commons-server/src/account_passkey.rs` --
  CeremonyStore with single-instance docs
- `crates/trace-commons-server/src/instance_enroll_guard.rs` --
  in-process rate limiter
- `crates/trace-commons-server/src/audit_chain.rs` --
  hash-chain drift detection
- `crates/trace-commons-protocol/src/redaction.rs` --
  deterministic redactor
- `crates/trace-commons-server/src/trace_upload_claim_issuer.rs` --
  graceful shutdown implementation
- `deploy/pilot-gcp/` -- systemd units, Caddy config, deploy scripts
- `cloudbuild.yaml` -- Cloud Build pipeline
- `migrations/` -- 41 PostgreSQL migrations (V1 through V41)
- PR #201 -- corpus redaction re-scan
- PR #238 -- DP mechanism analysis
- PR #246 -- revocation fix
- PR #215 -- rate limiting trace submissions
- PR #226 -- atomic settlement batch
- PR #227 -- column-scoped gate-driver grants

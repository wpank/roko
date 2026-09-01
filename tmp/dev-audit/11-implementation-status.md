# Full development-speed implementation status

Date: 2026-08-31

Initial integration snapshot: `52d5f4df411c3faa7b2e97208b11f76960e5c35d`

Post-integration source reconciliation: lifecycle/TUI seams `43a48ee26`, bounded plan/event work
`6da6ae504`, cache lifecycle `97f897200` + `8c82c5b1b`, offline index repair `85c052fc9`, and
benchmark automation `d1b94b139`

Branch at reconciliation: `feat/dev-audit-complete`

This is the authoritative reconciliation for the expanded dev-audit implementation. Documents
00–10 preserve the original measurements, design, and P0 history; their status annotations point
here when the later integration supersedes an earlier “deferred” statement.

Two states are deliberately separate:

- **Implemented** means the code is present in the integration snapshot and passed static review.
- **Verified** means the named command or real runtime fixture has actually completed against the
  final integrated tree.

The user requested that compilation, tests, clippy, and integration runs happen once at the end.
That batch has now produced the concrete checkpoint below. A checked implementation box must not
be read as a claim that the workspace all-target release lane or representative paid benchmark
matrix is globally green; those intentionally remain separate and open.

## Implemented in the integration snapshot

### Lean build and hot runtime path — `88c724744`

- [x] Default CLI/serve development builds avoid the Alloy/full-provider graph; full release,
  Docker, chain, ACP, and embedded-frontend surfaces select their features explicitly.
- [x] Chain-disabled HTTP surfaces return a typed `501` diagnostic instead of disappearing or
  panicking.
- [x] Ordinary Rust checks do not install frontend packages; tracked fallback/prebuilt assets are
  used and release jobs own the explicit frontend build.
- [x] Rust is pinned, a `dev-fast` profile exists, and release automation selects the full feature
  contract explicitly.
- [x] Prompt source scans run off Tokio workers, model configuration is shared through `Arc`, the
  cascade router reuses memory on its global path, and non-Git work skips a Git subprocess.
- [x] Connected TUI redraw/watcher work is bounded and terminal usage labels distinguish current
  state from cumulative counters.

### Complete bounded evidence harness — `bba2f8858` + `6af235c0f`

- [x] `run-evidence`, `evidence-validate`, `feedback`, and `score` are supported through `dev.sh`.
- [x] Bundles are private, redacted, size-bounded, process-group bounded, and strict about one
  terminal result, JSON/JSONL validity, run ownership, secret patterns, and exit semantics.
- [x] Fresh status/log slicing, resource admission, Git evidence, process inventory, metrics,
  deterministic score/debrief output, safe GET/OpenAPI collection, and opt-in CLI/text/PNG hooks
  are implemented.
- [x] Optional/unavailable runtime surfaces are recorded as skipped rather than reported green.

### Impact-selected verification — `d43dd45cd`

- [x] Gate mode is explicit: `none`, `structural`, `focused`, or `full`; normal mode remains full
  and FAST selects focused.
- [x] Actual Git diffs plus Cargo metadata select lib/bin/test/example/bench targets and required
  features, widen shared modules, and add bounded transitive reverse dependents for likely public
  contracts.
- [x] Compiler ownership is serialized and observable; runner Cargo uses `dev-fast` and skips
  frontend work.
- [x] Focused pre-existing failures require the same stable structured evidence before they can be
  classified as baseline.
- [x] Nextest fast/slow/live profiles are present.

This analysis is intentionally conservative. It is not a complete semantic `roko-index` call-site
oracle for macro-generated APIs or non-Rust consumers; ambiguous/high-cap work widens or escalates.

### Bounded plan and prompt policy — `d7541a437`

- [x] FAST plans use exact source-PRD/task artifacts, bounded files/ranges/symbol anchors, explicit
  budgets, cohesive tasks, and a small verification count.
- [x] Generated plans are clamped to FAST's four-task execution ceiling, and the generator prompt
  states the same limit so it cannot plan work that execution will silently reject.
- [x] Broad workspace/history/source-map expansion is omitted in FAST; duplicate verification and
  same-file microtask fragmentation are rejected.
- [x] Generated, direct, and regenerated plans pass the same policy validation.
- [x] Native provider limits are configured where supported; a strict request for unsupported
  Codex operation-level enforcement fails closed rather than claiming an allowlist exists.

Codex still lacks a native binding operation-level broker for all built-in actions. Building such
a broker remains an explicit residual, not a hidden prompt-only policy.

### Run-scoped observability API — `5f689d66e`

- [x] New runner and canonical runtime records are indexed into safe hashed per-run JSONL paths
  without a second durable fsync on every output delta.
- [x] Loopback/authenticated GET routes expose run detail, cursor-paginated/filterable events,
  run-filtered SSE, task attempts, gates, scrubbed logs, metrics, artifact/screenshot inventories,
  bundle metadata, and dashboard run discovery.
- [x] IDs and paths are grammar/size checked; traversal, symlinks, arbitrary artifact content,
  unsafe remote unauthenticated reads, and oversized records fail safe.
- [x] OpenAPI and the HTTP reference describe the run surface.

Historical global records created before the index are never rebuilt during an HTTP request or
server startup. Commit `85c052fc9` adds an explicit `roko run-index repair` command: it is dry-run
by default, scans recognized live and immutable rotated logs under aggregate byte/record/deadline
budgets, rejects malformed or cross-run records, and atomically replaces per-run indexes only
after a complete scan. `--apply` fails closed around symlinks, path escapes, and active
workspace/writer/cache/repair leases.

### FAST deadline, scheduling, salvage, and convergence — `52d5f4df4`

- [x] Capacity release, agent/gate completion, retry readiness, and deadlines wake scheduling
  directly instead of paying the former 100 ms preparation poll.
- [x] The non-resetting FAST deadline interposes awaited worktree, route, prompt, hook, CLI, and
  in-process bridge startup, with a separate bounded startup cap.
- [x] Cancellation and deadline cleanup use bounded settlement and preserve unconfirmed
  process/ownership evidence rather than manufacturing a clean terminal.
- [x] Conductor `Restart` and `Fail` pre-cancel paths are also bounded; exhausted settlement fails
  closed with degraded ownership evidence instead of looping outside the deadline.
- [x] A non-empty timeout diff must be structurally mutable and safe, is fingerprinted from its
  immutable base plus exact content/modes, and re-enters the ordinary post-dispatch safety and
  gate lifecycle under no-provider ownership.
- [x] Restart recovery revalidates the content fingerprint before gate launch and suppresses a
  duplicate provider or gate producer.
- [x] Terminal event, dashboard/status, task totals, elapsed/ETA, ledger, and PID projections
  converge while retaining degraded cleanup truth.

### Safe cache lifecycle — `97f897200` + `8c82c5b1b`

- [x] `roko cache status` reports target/evidence/context/log pressure without mutation.
- [x] `roko cache prune` is dry-run by default and requires `--apply` before deleting an eligible
  entry.
- [x] Size-, age-, and revision-aware selection protects live logs, active/nonterminal evidence,
  current Git-authoritative revisions, recent evidence, workspaces, Cargo users, and active leases;
  unsafe links/path escapes fail closed.
- [x] Target pressure prefers stale incremental partitions and preserves compiled dependencies;
  warm targets remain available unless the operator explicitly enters the cleanup lane.
- [x] Cache cleanup stays outside plan dispatch and reports projected/reclaimed bytes plus cold-build
  risk.

### Deterministic benchmark automation — `d1b94b139`

- [x] `scripts/dev_benchmark.py` runs stock/FAST/manual lanes from fixed-SHA detached worktrees and
  delegates capture to the evidence harness.
- [x] Cold samples use uniquely owned targets, warm samples use bounded lane-local seeded targets,
  shared caches are never cleaned, and paid/network execution requires explicit admission and a
  cost ceiling.
- [x] Raw rows, bundles, admission decisions, p50/p95 scorecards, failures, timeouts, and missing
  measurements are retained instead of being silently discarded.
- [x] `./dev.sh benchmark history` adds bounded deterministic JSON/Markdown history, prefers raw
  measured scorecard rows with a bounded `runs.jsonl` fallback, and emits explicit
  newest-versus-previous or fixed-baseline regression alerts.
- [x] History scanning caps root entries, sessions, per-session rows/groups, artifact and aggregate
  bytes, and elapsed time. Over-limit enumeration fails closed instead of publishing a biased
  partial view.
- [x] p50/p95 latency, non-success, timeout, and validated-rate thresholds are configurable; a
  breach exits 1 by default, while missing and undersampled comparisons remain visibly
  inconclusive and can be made fatal separately.
- [ ] Execute the representative matrix and import real manual Claude/Codex samples; tooling
  presence is not benchmark evidence.

## Final verification checkpoint

The following results were produced from the integrated branch before the final rebase/merge. A
small delta after the first strict clippy checkpoint adds a backward-compatible default for the
legacy `[agent].command` field, replaces empty test-model placeholders, adds explicit-only evidence
endpoint selection, and makes the listener-authorization boundary immutable across config reloads.
That delta subsequently passed the production-path check and targeted strict lint listed below.

- [x] Shell/Python/JSON syntax and command-help checks completed for the development wrappers,
  evidence tooling, benchmark tooling, and checked-in manifests.
- [x] The default `roko-cli` dependency tree excludes `alloy-provider`, `alloy-network`, and
  `alloy-rpc-client`.
- [x] `cargo check -p roko-serve --no-default-features --locked -j1` passed.
- [x] `cargo check -p roko-cli --features alloy-backend,acp --locked -j1` passed.
- [x] `cargo build -p roko-cli --bin roko --locked -j1` produced the current CLI binary. The
  checked-in configuration also starts parsing without requiring the retired legacy agent command.
- [x] The latest `roko-cli` library harness completed with 2,301 passed, zero failed, and one
  ignored test.
- [x] `cargo clippy --locked --workspace --no-deps -j1 -- -D warnings` passed before the small
  post-checkpoint delta above; only the repository's two existing allowed unknown-lint warnings
  were emitted.
- [x] `cargo check -p roko-cli --bin roko --locked -j1` passed after the final config and listener
  security changes. With the check-profile cache cold, it took 7m34s; this was intentionally the
  only final production-path compile.
- [x] Nightly format plus `git diff --check` passed, followed by
  `cargo clippy -p roko-cli -p roko-serve -p roko-runtime --no-deps --locked -j1 -- -D warnings`
  in 48s. It emitted only the same two pre-existing unknown-lint warnings.
- [x] `roko layer-check` passed after replacing four empty test-only model placeholders with an
  explicit `test-model` value.
- [x] A disposable cache fixture planned four stale incremental deletions, reclaimed the same four
  entries/16 KiB under `--apply`, retained the newest eight partitions, and preserved compiled
  dependencies.
- [x] A bounded run-index dry-run/apply fixture indexed three valid records into two hashed run
  directories and rejected malformed, invalid-ID, and cross-run records. Truncation and active-lock
  refusal remain part of the release fixture matrix.
- [x] Benchmark inspection reported four lanes; its no-execution dry-run planned 140 measured runs
  and correctly required explicit network/cost admission. Synthetic two-session history produced
  the expected nonzero regression result and two alerts for a 100 ms to 200 ms p50 change.
- [x] The loopback serve fixture returned `200` for health, readiness, status, run detail, events,
  tasks, gates, and metrics. Event cursor pages advanced `0` to `40` to `123`, and bounded
  run-filtered SSE replay completed.
- [x] A strict evidence smoke used the explicit-only endpoint policy added in `25aaca597`: all
  eight selected GET endpoints and the CLI hook passed, validation reported no errors, warnings,
  or secret hits, and both deterministic feedback and score output were green.
- [ ] `cargo test --workspace` is not a recorded pass. It ran many earlier suites successfully but
  was intentionally stopped while compiling the large remaining integration-test binary matrix;
  the complete all-target/full-CI lane stays open rather than blocking the interactive loop.

## Final evidence still required

- [x] Refresh and reconcile against `origin/main` at `c28b2d618a142332022e1670764d8e5073053177`;
  it is an ancestor of the integrated branch (32 commits ahead, zero behind), so no conflictful
  replay was required and no active-agent work was dropped.
- [x] Run the final incremental formatting/diff check and targeted strict clippy over the small
  post-checkpoint delta.
- [x] Publish the exact final-batch commands, results, and allowed residuals in this document and
  the operator-facing FAST/evidence guides.
- [ ] Run real representative benchmark repetitions: at least five cold and five warm samples per
  selected fixture/lane, retain failures/timeouts, and publish p50/p95 plus bundle links.
- [x] Exercise `roko run-index repair` in a bounded disposable dry-run/apply fixture with valid,
  malformed, invalid-ID, and cross-run records.
- [ ] Add explicit truncation and active-lock-refusal cases to the run-index release fixture.
- [x] Exercise cache status/prune against disposable stale incremental data, confirming the
  dry-run/apply plan and reclaimed-byte accounting agree while newest entries/dependencies remain.
- [ ] Add an active-workspace/Cargo-owner refusal fixture to the cache release lane.
- [ ] Measure escaped regressions and full-CI baseline before promoting FAST or auto-merge.

## Explicit residuals

- [ ] A Roko-owned operation-level broker for restrictive Codex tool/read/network policies.
- [ ] Semantic equivalence deduplication hidden behind arbitrary shell wrappers.
- [ ] Complete symbol-level, macro-aware, non-Rust consumer analysis beyond conservative
  syntax/Cargo-graph impact selection.
- [ ] Policy decisions to make FAST the default or enable automatic merges.

These residuals are kept open on purpose. None is silently redefined as “done” by the faster local
lane.

## Status Update (2026-09-01)

Cross-referenced against findings from three parallel audits conducted 2026-08-31 and the
consolidated dogfood audit of 2026-08-13 through 2026-08-29:

- CLI audit: `tmp/cli-audit/SUMMARY.md` (30 agent reports, 47 commands, ~170 leaf paths)
- Engine audit: `tmp/engine-audit/SUMMARY.md` (20 agent reports, engine convergence roadmap)
- Dogfood audit: `tmp/dogfood-audit/01-findings-register.md` (10 sessions, 106 raw logs)

Note: the user's original request referenced a `tmp/ux-audit/SUMMARY.md` but no such directory
exists. The `tmp/cybernetic-audit/` directory is empty. This update covers the three audits that
produced substantive findings.

### Dev-audit items confirmed or strengthened by the new audits

**Lean build graph (implemented).** The CLI audit (report 19-feature-flags.md) confirmed that
the default `roko-cli` dependency tree now excludes `alloy-provider`/`alloy-network`/
`alloy-rpc-client`, matching the dev-audit verification checkpoint. The dogfood audit
independently created backlog #230 for the same Alloy default-build concern, confirming both
audits converge. The dev-audit's lean-build work is verified.

**Impact-selected verification (implemented).** The dogfood audit's P1 finding on cross-crate
change-impact scoping (backlog #231) directly validates the dev-audit's conservative design
choice: the current impact analysis is syntax/Cargo-graph-based and intentionally widens for
ambiguous cases. The dogfood sessions recorded a real `bool` to `Option<bool>` public type
change that missed consumers across three crates, exactly the scenario this dev-audit's explicit
residual (“complete symbol-level, macro-aware, non-Rust consumer analysis”) acknowledges as
unsolved. No change to that residual's status.

**Evidence bundles (implemented).** The dogfood audit's P1 finding (backlog #228) and the
dev-audit's evidence harness (`bba2f8858` + `6af235c0f`) address the same gap from different
angles. The dev-audit's `run-evidence`, `evidence-validate`, and strict bundle validation are
implemented. The dogfood audit confirms that operational sessions still assembled evidence
manually in practice, meaning the tooling exists but operator adoption/workflow integration
remains incomplete.

**Pre-existing gate failures (open residual overlap).** The dev-audit's impact-selected
verification includes focused baseline classification for pre-existing failures. The dogfood
audit's P1 finding on baseline gate rejection (backlogs #166, #170) and the CLI audit's
confirmation that `BenchmarkRegressionGate` always passes both reinforce that this area needs
further work. The dev-audit's “focused pre-existing failures require stable structured evidence”
item is implemented in code but not yet proven against the real recurrence pattern the dogfood
audit documented.

**FAST deadline and scheduling (implemented).** The dogfood audit's strongest repeated finding
was agent lifecycle and runner termination failures: agents made correct edits but the runner
failed to observe completion or hung during cleanup. The dev-audit's FAST deadline interposition,
bounded settlement, and convergent terminal projections address the deadline/timeout side. The
engine audit noted that `AgentSilence`/`LostEffect` typed deadlines now exist but recommended
end-to-end proof (backlog #138). The dev-audit's scheduling and salvage implementation is not
invalidated, but the dogfood evidence shows the broader runner lifecycle problem extends beyond
FAST mode.

### Dev-audit items with new context from the audits

**Blanket `#![allow(dead_code)]` suppression.** The CLI audit elevated this as critical finding
#3 and the engine audit reported 242 compiler warnings hidden by the blanket allow on roko-cli.
The dev-audit did not directly address this suppression (it focused on FAST-mode compilation
rather than lint hygiene). The engine audit's convergence roadmap schedules removal under
backlog #43 in Wave 11, after Runner-v2 retirement. This is new context: the dev-audit's
compilation and cache work coexists with a large hidden dead-code surface that distorts
incremental compile times and masks stale code.

**`event_loop.rs` size (23,673 lines).** The CLI audit flagged this as the largest file in the
codebase. The engine audit's convergence plan proposes decomposing it into Cells and ultimately
deleting it (Waves 2-4, 11). The dev-audit's FAST mode operates within this file for runner-
owned verification; neither audit contradicts the dev-audit's approach, but the engine audit's
planned decomposition will eventually restructure the surface the dev-audit's FAST integration
touches.

**Duplicate code paths.** The engine audit found 7 agent dispatch paths, 11 model resolution
functions, 9 config loading functions, and 4 doctor implementations. The dev-audit's bounded
plan context and impact-selected verification intentionally narrowed scope rather than
consolidating these paths. The engine audit's `RuntimeServices` unification (backlog #243,
Wave 2) will eventually reduce duplication that currently makes FAST impact analysis harder
than necessary.

**Run-scoped observability API (implemented).** The CLI audit confirmed ~378 HTTP endpoints
(versus the documented ~317), including the run-scoped routes added by the dev-audit. The
dogfood audit's observability reference (`04-observability-reference.md`) documents the
actual endpoint surface. The dev-audit's loopback fixture verified 8 selected GET endpoints;
the broader endpoint surface is operational but the full ~378 routes have not been individually
exercised against the dev-audit's evidence policy.

**Cache lifecycle (implemented).** The CLI audit did not surface cache-related issues. The
dogfood audit's performance section documented cold release builds at 10-14 minutes and fresh
worktree compilation at 7+ minutes before shared `CARGO_TARGET_DIR`. The dev-audit's cache
status/prune tooling and warm-target preservation are implemented, but the open benchmark
residual (representative cold/warm scorecard) remains the gap between “tooling exists” and
“measured improvement.”

**Provider/Codex broker residual (open).** The CLI audit documented that Codex streaming/cost
parsing was fixed in dogfood sessions, and that a first-class Codex provider kind is still
tracked under backlog #158. The dev-audit's explicit residual for a Roko-owned operation-level
Codex broker remains open and is independently supported by the engine audit's observation that
provider operation-level enforcement fails closed for unsupported providers.

### Dev-audit items unchanged by the new audits

The following items were neither confirmed nor contradicted:

- **Benchmark automation tooling** (`d1b94b139`): no audit exercised the benchmark fixtures.
  The open residual (execute representative matrix and import real samples) remains.
- **`cargo test --workspace` all-target/full-CI lane**: still not a recorded pass. The CLI
  audit counted 11,948 tests across the workspace; none of the audits ran the full suite as a
  verification pass.
- **Run-index release fixtures** (truncation, active-lock-refusal): still open.
- **Cache release fixtures** (active-workspace/Cargo-owner refusal): still open.
- **FAST promotion or auto-merge policy**: still an explicit deferred decision.

### New risks surfaced by the audits that affect dev-audit scope

**HDC feature flag pipeline severed (CLI audit critical finding #4).** `roko-cli` enables
`roko-neuro/hdc` but does not propagate `hdc` to `roko-compose`, `roko-fs`, or `roko-serve`.
This does not directly affect dev-audit FAST mode but means the HDC fingerprint-per-episode
path (used by the evidence harness for run identity) may be incomplete in prompt composition
and file substrate persistence.

**`roko inject` is a complete stub (CLI audit critical finding #1).** The dev-audit's evidence
harness and signal injection are unrelated, but a lying stub in the CLI raises a general
correctness concern about which advertised surfaces actually function.

**Graph engine convergence plan (engine audit).** The engine audit produced a 13-wave, 50+
backlog-item roadmap to make `GraphEngine` the single execution kernel. This is a large
structural change that will eventually obsolete Runner-v2, which is where the dev-audit's FAST
mode, scheduling, deadline, and gate integration live. The dev-audit's implementation will need
migration when the engine convergence reaches Waves 7-9 (workflow routing, shadow parity,
cutover). This is not an immediate concern but should be noted in planning.

**State and index drift (dogfood audit).** The dogfood audit found 11 completed plans still
marked ready and backlog items advertised as open after being implemented. Backlog #229 covers
reconciliation. This does not directly affect dev-audit implementation but means the backlog
references in this document (and its tracked reconciliation set) may have stale cross-
references.

**Marketplace stubs return 200/201 (CLI audit critical finding #6).** The dev-audit's evidence
harness records optional/unavailable surfaces as skipped rather than green. If marketplace
stubs are exercised, they would incorrectly report success. This is a correctness issue in the
broader HTTP surface, not specific to the dev-audit, but the dev-audit's evidence policy should
treat stub 200 responses the same as genuine 501 responses when scoring.

### Summary disposition

| Dev-audit area | Original status | Post-audit status | Notes |
|---|---|---|---|
| Lean build graph | Implemented, verified | Confirmed by CLI audit + dogfood #230 | No change |
| Evidence harness | Implemented, verified | Confirmed exists; operator adoption gap noted by dogfood audit | No change to implementation status |
| Impact-selected verification | Implemented, conservative | Confirmed conservative; dogfood #231 validates the explicit residual | No change |
| FAST deadline/scheduling | Implemented, verified | Confirmed; broader runner lifecycle issues noted | No change to FAST scope |
| Cache lifecycle | Implemented, verified | No contradicting evidence | Benchmark residual still open |
| Run-scoped observability | Implemented, verified | Endpoint count updated (~378 vs ~317) | No implementation change |
| Benchmark automation | Implemented, not exercised | Unchanged | Real samples still needed |
| Full workspace test pass | Open | Open | 11,948 tests exist; no full pass recorded |
| Codex operation broker | Open residual | Open; independently confirmed by CLI/engine audits | No change |
| Symbol-level impact analysis | Open residual | Open; directly validated by dogfood #231 evidence | No change |
| FAST promotion policy | Deferred | Deferred | No change |
| Blanket dead_code allow | Not in dev-audit scope | New context from CLI/engine audits | Affects compile-time accuracy |
| Runner-v2 decomposition | Not in dev-audit scope | Engine audit produced 13-wave plan | Future migration needed for FAST |

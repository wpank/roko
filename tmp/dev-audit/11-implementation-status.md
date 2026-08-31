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
Accordingly, the implementation boxes below are checked while the final-batch evidence and
benchmark boxes remain open until those commands produce artifacts. A checked implementation box
must not be read as a claim that the final tree is globally green.

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
- [ ] Execute the representative matrix and import real manual Claude/Codex samples; tooling
  presence is not benchmark evidence.

## Final evidence still required

- [ ] Rebase the complete integration on the latest `origin/main` and resolve other active-agent
  changes without dropping work.
- [ ] Run the single final formatting, syntax, lean/full compile, focused test, clippy, API smoke,
  evidence-validator, and regression batch selected by the coordinator.
- [ ] Publish the exact final-batch commands, results, and any allowed residual failures.
- [ ] Run real representative benchmark repetitions: at least five cold and five warm samples per
  selected fixture/lane, retain failures/timeouts, and publish p50/p95 plus bundle links.
- [ ] Exercise `roko run-index repair` in bounded dry-run/apply fixtures, including truncation,
  malformed/cross-run records, and active-lock refusal.
- [ ] Exercise cache status/prune against protected active data and disposable stale fixtures,
  confirming the dry-run/apply plans and reclaimed-byte accounting agree.
- [ ] Measure escaped regressions and full-CI baseline before promoting FAST or auto-merge.

## Explicit residuals

- [ ] A Roko-owned operation-level broker for restrictive Codex tool/read/network policies.
- [ ] Semantic equivalence deduplication hidden behind arbitrary shell wrappers.
- [ ] Complete symbol-level, macro-aware, non-Rust consumer analysis beyond conservative
  syntax/Cargo-graph impact selection.
- [ ] Policy decisions to make FAST the default or enable automatic merges.

These residuals are kept open on purpose. None is silently redefined as “done” by the faster local
lane.

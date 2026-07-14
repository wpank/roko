# CTRL-02 config-dispatch precursor implementation evidence (corrected candidate r2)

Assignment:
- Plan: `tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md`, Wave 0 `CTRL-02`; bounded reconstruction of the config-dispatch subset of historical precursor `3041d095d4daebed2c9e05c63eacb18e668e37e3`
- Base SHA: `b59e497e71143212dfca0f50dbbfac3ce4a47844`
- Branch/worktree: `agent/CTRL-02-config-dispatch` at `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/workers/CTRL-02-config-dispatch`
- Integration branch: `status-quo/integration-status-quo-20260714T073140Z`
- Reserved write scope: `crates/roko-core/src/config/loader.rs`; `crates/roko-core/src/config/mod.rs`; `crates/roko-core/src/config/timeouts.rs`; `crates/roko-serve/src/lib.rs`; `crates/roko-serve/src/routes/config.rs`; this evidence file

Requirement:
- Original defect or missing behavior: the July 14 precursor added deterministic model-reference normalization and ambiguity/unresolved errors, but the inherited function is invoked only by the runner-v2 event loop. The embedded serve builder and its config update/reload routes can therefore construct or store dispatch state with duplicate slugs or unresolved default/fallback/tier/role references. The inherited tests exercise only duplicate slugs, one default alias, and one unresolved default; they do not prove key precedence, all modeled reference fields, empty-registry compatibility, serve startup, route rejection, or no-write-on-invalid-update behavior.
- Acceptance requirements: prove the exact inherited bytes and later drift; preserve canonical-key precedence and unique-slug alias normalization; deterministically reject duplicate slugs and unresolved default/fallback/tier/role references; preserve the legacy empty-model-registry path; validate embedded serve startup before side effects; make config update/reload reject invalid model state with HTTP 400 semantics; retain secret masking behavior; prove timeout accessor defaults; run focused `roko-core` config and `roko-serve` config/startup tests plus affected all-target checks, formatting, history/scope, and artifact cleanup.
- Explicit non-goals: completing or changing SH05-T01 status; normalizing routing fields beyond this inherited contract; implementing E42 migrations/invariants/provenance; changing E04/E18 config redaction; changing P17 warning levels; accepting P20 same-provider duplicate slugs or synthesizing zero-config providers/models; repairing the legacy CLI `load_resolved_config` dual-config drop; changing the unrelated StateHub snapshot hunk in `roko-serve/src/lib.rs`; changing manifests, indexes, master status, lockfiles, or runner code.
- Dependencies and their integration commits: historical precursor parent `1649c18b2c3d2b3602bfe17398b0e1454a19c5ef` and precursor `3041d095d4daebed2c9e05c63eacb18e668e37e3` are ancestors of the assigned base; CTRL-01 recovery and six earlier CTRL-02 clusters are integrated in the assigned base; CTRL-15 is integrated. This lane is file-disjoint from the active StateHub/SSE and CTRL-16 work.

Reproduction:
- Pre-fix command: `rg -n "normalize_and_validate_dispatch_models" crates --glob '*.rs'` at the assigned base.
- Exit/result: exit 0; outside its own unit tests, the function had exactly one production invocation, in `crates/roko-cli/src/runner/event_loop.rs`. `ServerBuilder::start_background`, `update_config`, and `reload_config_from_disk` did not invoke it. Direct `ServerBuilder` embedders and both mutable HTTP config boundaries therefore bypassed the inherited dispatch-model contract.
- Expected: every production boundary that constructs live CLI runner or serve dispatch state rejects ambiguous/unresolved configured models before spawning work or persisting the state.
- Actual: runner-v2 validates, but serve startup/update/reload accept the invalid state; inherited tests do not cover those boundaries.

Implementation:
- Design and invariants: both `ServerBuilder::start_background` and public `run_server_with_state` call one startup normalization/validation helper before restore, watcher/subscriber/bridge/saver/job/archive task creation, router/listener work, or any other fallible startup operation. All PUT, explicit reload, and unchanged watcher reload calls cross one process-wide ownership gate and monotonic generation counter. Async HTTP callers execute the synchronous transaction on Tokio's blocking pool; the watcher retains its public synchronous API; no blocking mutex is held across an `.await`. The owned transaction includes snapshot selection, merge/load, normalization and validation, serialization, atomic `roko.toml` replacement, then the infallible live ArcSwap commit. Ephemeral copies are propagated only after the authoritative commit, outside the gate, and their failures are warned without rollback. Resolution remains inherited: trimmed exact keys win, unique aliases become canonical keys, and duplicate/unresolved references are fatal with an empty-registry compatibility escape. Tier and role keys are now sorted before validation, making the first nested error deterministic.
- Files/symbols changed: `crates/roko-core/src/config/loader.rs` additionally sorts nested dispatch maps and repeats multi-invalid insertion-order regressions. `crates/roko-serve/src/lib.rs` factors shared startup validation and proves an invalid caller-supplied `AppState` leaves the workdir, cancellation state, process supervisor, and probe port untouched. `crates/roko-serve/src/routes/config.rs` owns the serialized transaction, atomic persistence/generation ordering, blocking-pool adapters, and deterministic concurrent/rollback/reload regressions. Reserved `config/mod.rs` and `config/timeouts.rs` remain byte-unchanged because their inherited exports/errors and timeout-default assertions were correct.
- Compatibility/migration: no schema, dependency, manifest, lockfile, response shape, or public API change. PUT and reload now atomically persist the validated canonical effective TOML generation, so unique slug aliases and merged effective values no longer leave disk behind live state. Existing deployments with no configured models retain their legacy string references unchanged.
- Failure/recovery/security behavior: invalid PUT requests return the existing mapped HTTP 400 before creating or replacing `roko.toml`, publishing an update, or changing live state. Invalid reloads return HTTP 400 before changing file/live truth. Atomic replacement failure leaves the previous live generation untouched. Concurrent disjoint PUTs merge over the preceding committed generation instead of losing work; a reload waiting behind PUT loads and publishes the winning file rather than stale pre-transaction work. Invalid embedded startup returns a contextual error before any server side effect. Existing GET/PUT secret masking and response shapes remain green; errors contain model identifiers and field paths but no credentials.

Rejection correction:
- Rejected candidate: `91992857f160097b4e850708dfe7b606f262ee0a` (parent `b59e497e71143212dfca0f50dbbfac3ce4a47844`). Independent review record: integration commit `acac82c6e`, `tmp/status-quo/execution-evidence/CTRL-02-CONFIG-DISPATCH-REVIEW.md`.
- Disposition: all three findings were accepted and corrected. Public `run_server_with_state` now shares startup validation; PUT/explicit reload/watcher reload now share transaction ownership, atomic persistence, and generation ordering; tier/role validation order is sorted and repeatedly regression-tested.
- Correction source commit: `af66e1446b9f318bb1fc0ea5792b81bc73f03511` with exact parent `91992857f160097b4e850708dfe7b606f262ee0a`.

Verification:
- Historical command: `git diff --find-renames 1649c18b2c3d2b3602bfe17398b0e1454a19c5ef..3041d095d4daebed2c9e05c63eacb18e668e37e3 -- <five reserved production files>`
- Exit/result: exit 0; 205 insertions and 11 deletions. The config portion adds `normalize_and_validate_dispatch_models`, `AmbiguousModelSlug`, `UnresolvedModel`, three loader tests, five timeout accessor assertions, and serve error mapping. The same five-file diff also contains one unrelated StateHub snapshot update, which this lane does not claim or modify.
- Drift command: `git diff --stat 3041d095d4daebed2c9e05c63eacb18e668e37e3..b59e497e71143212dfca0f50dbbfac3ce4a47844 -- <five reserved production files>`
- Exit/result: exit 0 with no output; all five inherited production files are byte-unchanged from the precursor at the assigned base.
- Call-site command: `rg -n "normalize_and_validate_dispatch_models" crates --glob '*.rs'`
- Exit/result after implementation: exit 0; the production call sites are runner-v2 event-loop dispatch, serve startup, PUT update, and reload. All remaining hits are the implementation or its unit tests.
- Focused core command: `cargo test -p roko-core config::loader -- --nocapture`
- Exit/result after r2: exit 0; 23 passed, 0 failed, including the repeated deterministic multi-invalid tier/role regression.
- Timeout command: `cargo test -p roko-core config::timeouts -- --nocapture`
- Exit/result after r2: exit 0; 7 passed, 0 failed. The inherited timeout accessor defaults remain correct without source changes.
- Route command: `cargo test -p roko-serve routes::config::tests -- --nocapture`
- Exit/result after r2: exit 0; 11 passed, 0 failed. New proof covers a barrier-released pair of disjoint PUTs, atomic-write failure rollback, and PUT-versus-reload ordering/file-live agreement in addition to masking and invalid-input coverage.
- Startup command: `cargo test -p roko-serve server_builder_rejects_ambiguous_models_before_startup -- --nocapture`
- Exit/result after r2: exit 0; 1 passed, 0 failed. The original candidate's corrected chained-error assertion remains green.
- Caller-state startup command: `cargo test -p roko-serve run_server_with_state_rejects_invalid_config_before_side_effects -- --nocapture`
- Exit/result after r2: exit 0; 1 passed, 0 failed; workdir bytes, cancellation, supervised-process count, and probe-port availability remain unchanged.
- Serve config sweep: `cargo test -p roko-serve config -- --nocapture`
- Exit/result after r2: exit 0; 26 passed, 0 failed.
- Affected all-target command: `cargo check -p roko-core -p roko-serve --all-targets`
- Exit/result: exit 0. Only two pre-existing missing-documentation warnings in unchanged `crates/roko-core/benches/engram_bench.rs`; no errors.
- Broader diagnostic command: `cargo test -p roko-core config -- --nocapture`
- Exit/result after r2: expected non-green baseline reproduction, now 193 passed and the same 4 failed because r2 adds one passing loader test. The failures remain the three unchanged repository-root-`roko.toml` schema tests and unchanged `config::cache::tests::watched_cache_sees_file_change`. The original candidate recorded 192 passed/4 failed; no baseline failure was removed, masked, or added.
- Formatting/scope commands: `cargo fmt --all -- --check`; `git diff --check`; `git status --short`; `git diff --name-only`; `git diff --stat`; reserved-path allow-list comparison.
- Exit/result: formatting and diff checks exit 0; the only candidate paths are the three source files named above plus this evidence file, all inside the six-path reservation. No debug prints, TODOs, credentials, manifests, lockfiles, indexes, master records, runner files, or unrelated StateHub bytes are changed.
- Build-artifact cleanup: the earlier isolated target and the correction worktree's ignored default `target/` are removed after the evidence commit; cleanup is reported with the immutable final HEAD.

Review readiness:
- Implementation correction source commit: `af66e1446b9f318bb1fc0ea5792b81bc73f03511`; the final evidence-bearing HEAD and its exact parent are reported to the coordinator after this evidence commit. Fresh independent review must bind to that final immutable HEAD.
- Diff scope reviewed: self-review covered both startup entrypoints; unchanged watcher call-site reachability; absence of await under the ownership gate; snapshot/merge/load/validate/atomic-persist/live-swap ordering; failed persistence rollback; post-commit ephemeral warnings; response masking; deterministic nested errors; historical drift; and the reserved-path allow-list. Independent review remains mandatory.
- Known limitations: this bounded precursor reconstruction does not make the broader SH05/E42/E04/E18/P17/P20 outcomes terminal and does not repair CLI dual-config convergence outside the reservation.
- Required reviewer focus: production-boundary ordering (validation before bind/spawn/write/store), deterministic key/slug resolution, all inherited reference fields, empty registry behavior, HTTP error status and state/file preservation, secret masking regression, timeout accessor defaults, and bounded nonclaims.

Integration:
- Review evidence: pending independent review.
- Integration commit: pending coordinator action.
- Post-merge commands/results: pending coordinator action.
- Final status: implementation candidate verified locally; not `DONE` until independent acceptance, integration, and coordinator-owned post-merge proof.

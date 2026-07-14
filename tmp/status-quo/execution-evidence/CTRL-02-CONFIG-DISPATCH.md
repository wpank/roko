# CTRL-02 config-dispatch precursor implementation evidence

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
- Design and invariants: `ServerBuilder::start_background` now validates its owned config as its first operation, before `PORT` processing, app-state construction, socket bind, or task spawn. PUT `/config` validates and canonicalizes the merged typed config before TOML serialization, write, event propagation, or state replacement. POST `/config/reload` validates immediately after load and before diff/application. Resolution remains inherited and deterministic: trimmed exact map keys win over slug aliases, unique aliases become canonical keys, duplicate slug matches are sorted in the error, and every unresolved populated default/fallback/tier/role field is fatal when the model registry is non-empty. An empty registry still returns unchanged for backward compatibility.
- Files/symbols changed: `crates/roko-core/src/config/loader.rs` adds adversarial coverage for key precedence, normalization of every inherited dispatch reference, exact unresolved fallback/tier/role paths, and empty-registry compatibility. `crates/roko-serve/src/lib.rs` adds the startup boundary and an ambiguity regression. `crates/roko-serve/src/routes/config.rs` adds update/reload boundaries plus HTTP, state, file, response, and canonical persistence regressions. Reserved `config/mod.rs` and `config/timeouts.rs` remain byte-unchanged because their inherited exports/errors and timeout-default assertions were correct.
- Compatibility/migration: no schema, serialization format, dependency, manifest, lockfile, or public API change. Valid configs are unchanged except slug aliases written through PUT are now persisted and returned as canonical model keys. Existing deployments with no configured models retain their legacy string references unchanged.
- Failure/recovery/security behavior: invalid PUT requests return the existing mapped HTTP 400 before creating or replacing `roko.toml`, publishing an update, or changing live state. Invalid reloads return HTTP 400 before changing live state. Invalid embedded startup returns a contextual error before any server side effect. Existing GET/PUT secret-masking regressions remain green; errors contain model identifiers and field paths but no credentials.

Verification:
- Historical command: `git diff --find-renames 1649c18b2c3d2b3602bfe17398b0e1454a19c5ef..3041d095d4daebed2c9e05c63eacb18e668e37e3 -- <five reserved production files>`
- Exit/result: exit 0; 205 insertions and 11 deletions. The config portion adds `normalize_and_validate_dispatch_models`, `AmbiguousModelSlug`, `UnresolvedModel`, three loader tests, five timeout accessor assertions, and serve error mapping. The same five-file diff also contains one unrelated StateHub snapshot update, which this lane does not claim or modify.
- Drift command: `git diff --stat 3041d095d4daebed2c9e05c63eacb18e668e37e3..b59e497e71143212dfca0f50dbbfac3ce4a47844 -- <five reserved production files>`
- Exit/result: exit 0 with no output; all five inherited production files are byte-unchanged from the precursor at the assigned base.
- Call-site command: `rg -n "normalize_and_validate_dispatch_models" crates --glob '*.rs'`
- Exit/result after implementation: exit 0; the production call sites are runner-v2 event-loop dispatch, serve startup, PUT update, and reload. All remaining hits are the implementation or its unit tests.
- Focused core command: `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<isolated-target> cargo test -p roko-core config::loader -- --nocapture`
- Exit/result: exit 0; 22 passed, 0 failed (including duplicate slug, key-over-slug precedence, all reference normalization, unresolved default/fallback/tier/role, and empty registry).
- Timeout command: `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<isolated-target> cargo test -p roko-core config::timeouts -- --nocapture`
- Exit/result: exit 0; 7 passed, 0 failed. The inherited timeout accessor defaults remain correct without source changes.
- Route command: `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<isolated-target> cargo test -p roko-serve routes::config::tests -- --nocapture`
- Exit/result: exit 0; 8 passed, 0 failed, including pre-existing masking coverage and the new rejection/no-mutation/canonical-persistence cases. Repeated after the startup-test correction with the same result.
- Startup command: `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<isolated-target> cargo test -p roko-serve server_builder_rejects_ambiguous_models_before_startup -- --nocapture`
- Exit/result: final exit 0; 1 passed, 0 failed. The first run correctly returned the contextual chained product error, but the test incorrectly searched only `anyhow::Error::to_string()` for the inner cause. The assertion was corrected to inspect `format!("{error:#}")`; no production behavior changed, and the rerun passed.
- Serve config sweep: `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<isolated-target> cargo test -p roko-serve config`
- Exit/result: exit 0; 22 passed, 0 failed.
- Affected all-target command: `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<isolated-target> cargo check -p roko-core -p roko-serve --all-targets`
- Exit/result: exit 0. Only two pre-existing missing-documentation warnings in unchanged `crates/roko-core/benches/engram_bench.rs`; no errors.
- Broader diagnostic command: `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=<isolated-target> cargo test -p roko-core config`
- Exit/result: non-green baseline diagnostic: 192 passed and 4 failed. Three unchanged schema tests require repository-root `roko.toml`, which is absent both from this worktree and from base (`test -f roko.toml` exits 1; `git ls-tree b59e497e -- roko.toml` has no output). The fourth is unchanged `config::cache::tests::watched_cache_sees_file_change`; an isolated `--test-threads=1` retry failed identically after five seconds. `git diff b59e497e -- crates/roko-core/src/config/schema.rs crates/roko-core/src/config/cache.rs` has no output. These failures are reproducible, outside the reservation, and do not invalidate the focused loader/timeout proof.
- Formatting/scope commands: `cargo fmt --all -- --check`; `git diff --check`; `git status --short`; `git diff --name-only`; `git diff --stat`; reserved-path allow-list comparison.
- Exit/result: formatting and diff checks exit 0; the only candidate paths are the three source files named above plus this evidence file, all inside the six-path reservation. No debug prints, TODOs, credentials, manifests, lockfiles, indexes, master records, runner files, or unrelated StateHub bytes are changed.
- Isolated target: `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/targets/CTRL-02-config-dispatch`; it is removed with `cargo clean --target-dir ...` after the candidate commit, and cleanup is reported with the immutable SHA.

Review readiness:
- Implementation commit: exact immutable candidate SHA is supplied to the coordinator/reviewer immediately after this evidence is committed; the independent reviewer record must bind to that SHA.
- Diff scope reviewed: self-review covered startup ordering, write/state ordering, error mapping, canonical response/persistence, existing masking paths, adversarial field coverage, historical drift, and the reserved-path allow-list. Independent review remains mandatory.
- Known limitations: this bounded precursor reconstruction does not make the broader SH05/E42/E04/E18/P17/P20 outcomes terminal and does not repair CLI dual-config convergence outside the reservation.
- Required reviewer focus: production-boundary ordering (validation before bind/spawn/write/store), deterministic key/slug resolution, all inherited reference fields, empty registry behavior, HTTP error status and state/file preservation, secret masking regression, timeout accessor defaults, and bounded nonclaims.

Integration:
- Review evidence: pending independent review.
- Integration commit: pending coordinator action.
- Post-merge commands/results: pending coordinator action.
- Final status: implementation candidate verified locally; not `DONE` until independent acceptance, integration, and coordinator-owned post-merge proof.

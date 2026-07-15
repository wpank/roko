# CTRL-02 config-dispatch independent review r3

## Assignment and identity

- Task: `CTRL-02-CONFIG-DISPATCH-r3`, independent precursor review.
- Original assigned base: `b59e497e71143212dfca0f50dbbfac3ce4a47844`.
- R1 candidate/rejection: `91992857f160097b4e850708dfe7b606f262ee0a`; integrated rejection record `acac82c6e`.
- R2 candidate/rejection: `83bec51978422093885a5304b4144a24b1949697`; integrated rejection record `fafe21aff`.
- R3 correction: `2a75473a8019324b5e848de3d6d9f51c7a5fdad8`.
- Exact cumulative candidate reviewed: `f23986f3ceb657fd061c733f56ae6e7c421f8ab0`.
- Exact chain: `f23986f3` -> `2a75473a` -> `83bec519` -> `af66e144` -> `91992857`.
- Verdict: **REJECTED**.

I read the complete master, full SH05 manifest, issues 23 and 55, relevant July 14 audit mappings, both prior rejection records, candidate evidence, exact history/diffs, current loader and schema behavior, both public serve startup paths, watcher/reload paths, PUT transaction, workspace replication, production CLI startup callers, and all changed tests.

## Prior-finding disposition

R3 materially fixes the two direct r2 findings: reload no longer serializes its unified effective object directly into project source, and PUT now keeps ownership through replica propagation and live publication so caller cancellation or a newer PUT cannot reorder an older accepted propagation. The r1 startup entrypoint, deterministic nested-reference selection, and shared mutation-ownership corrections also remain intact.

Those changes do not preserve the source/effective boundary across the next PUT.

## Release-blocking finding

### High: any PUT after unified startup or reload persists runtime-derived state into project source

`load_config_unified` applies global merge, named and hierarchical environment overrides, interpolation, and file-secret resolution (`crates/roko-core/src/config/loader.rs:257-274`). Production CLI serve entrypoints pass that effective object into `ServerBuildConfig` (`crates/roko-cli/src/commands/server.rs:34-40`, `crates/roko-cli/src/unified.rs:214-225`). Reload likewise publishes the unified effective object into `AppState` while correctly leaving project bytes untouched (`crates/roko-serve/src/routes/config.rs:260-300`).

The next ordinary PUT selects `state.load_roko_config()` as its merge base, converts that effective object to JSON, applies the patch, serializes the entire result to TOML, and atomically replaces project `roko.toml` (`routes/config.rs:131-149`). It propagates the same payload to replicas (`routes/config.rs:151-176`) before storing it live. Thus R3 only delays the r2 secret/source promotion until the next PUT. It also affects the first PUT after normal unified server startup; no explicit reload is required.

The candidate source-preservation test stops immediately after reload, while its PUT tests start from hand-constructed configs without runtime overlays. No test crosses the effective-publication-to-PUT boundary.

### Exact-tip dynamic reproduction

I exported exact tip `f23986f3` to a disposable directory and changed only its existing isolated synthetic source-preservation test. No candidate file was edited and no real credential was used or recorded.

Variant 1 used the existing private HOME/project fixture, published its unified effective configuration through production reload, then performed an ordinary production PUT. The parent asserted that project source now contained the synthetic resolved file value, global configuration, environment/interpolation results, and no longer retained the source file-reference form.

```text
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/ctrl02-config-r3-target \
  cargo test -p roko-serve \
  routes::config::tests::reload_preserves_runtime_overlay_sources \
  -- --nocapture

exit 0; 2 passed, 0 failed
```

Variant 2 replaced reload with direct `load_config_unified` construction of `AppState`, matching production startup, then ran the same PUT and source assertions. With the warmed exact-tip target it also exited 0 with 2 passed and 0 failed. Both commands passed because the reviewer assertions described the vulnerability.

Expected behavior is that project source retains its source-only representation across every later mutation: file references remain references and global/environment/interpolated/resolved values remain runtime-only. Actual behavior writes those effective values to project source on the next accepted PUT.

This violates the required r2 correction, the r3 evidence's source/effective security claim, and the assignment requirement that reload/startup-derived values never be promoted into project source. Atomic ordering does not make an unsafe payload acceptable.

Required correction: retain or reload a distinct source-only project representation for PUT. Merge the patch into that representation, preserve source references, derive and validate the effective runtime object separately, then atomically commit the source payload and publish the effective payload under the existing ownership boundary. Add deterministic regressions for both unified startup -> PUT and reload -> PUT proving project bytes exclude every runtime-only layer while main/live/replica ordering and documented replica-failure behavior remain correct.

## Independent verification

- Pristine candidate route suite after `cargo clean -p roko-serve --target-dir /tmp/ctrl02-config-r3-target` forced candidate-path recompilation:
  - `cargo test -p roko-serve routes::config::tests -- --nocapture`
  - exit 0; 16 passed, 0 failed, including cancellation, concurrent PUT, PUT/reload exclusion, replica ordering/failure, direct source preservation, masking, and invalid rollback.
- `cargo test -p roko-serve server_builder_rejects_ambiguous_models_before_startup -- --nocapture`: exit 0; 1 passed.
- `cargo test -p roko-serve run_server_with_state_rejects_invalid_config_before_side_effects -- --nocapture`: exit 0; 1 passed.
- `cargo fmt --all -- --check`: exit 0.
- `cargo metadata --locked --no-deps --format-version 1`: exit 0.
- `git diff --check b59e497e..HEAD`: exit 0.
- No candidate diff in workspace/core/serve manifests or `Cargo.lock`.
- Both `1649c18b2c3d2b3602bfe17398b0e1454a19c5ef` and `3041d095d4daebed2c9e05c63eacb18e668e37e3` are candidate ancestors.
- Cumulative scope from `b59e497e`: only `crates/roko-core/src/config/loader.rs`, `crates/roko-serve/src/lib.rs`, `crates/roko-serve/src/routes/config.rs`, and the implementation evidence file before this review record.
- A chained core/all-target command was stopped during duplicate core test compilation after the blocker was independently reproduced; the author's recorded core/all-target results are not disputed and cannot exercise this cross-boundary persistence path.

The accepted parts remain bounded precursor work: this review does not claim full SH05, E42, E04, E18, P17, P20, CLI dual-config convergence, or general secret-redaction completion.

## Verdict

**REJECTED** for exact candidate `f23986f3ceb657fd061c733f56ae6e7c421f8ab0`.

Confidence: high. Direct reload preservation and ordered cancellation-safe propagation are real improvements, but the durable source/effective separation is still absent. Do not integrate the candidate until both startup/reload-to-PUT adversarial cases preserve project source and receive fresh independent review.

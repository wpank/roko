# CTRL-02 config dispatch — independent review r5

- Task: `CTRL-02-CONFIG-DISPATCH-r5`
- Reviewer role: independent review; no implementation, manifest, status, or master edits
- Prior r4 candidate/rejection: `960082d2518a8bfc851a77afb37f4699f05469cb`; integrated record `ef2dbcc74828582db63d3c3f05830f9198d9984a`
- r5 implementation: `a4c3ca731653211571554d14ddd22fc0803965e6`
- Exact evidence-bearing candidate reviewed: `788aedfd42d9133b736179e9fa5339ddf923e800`
- Verdict: **ACCEPTED**

## Review conclusion

R5 corrects the r4 release blocker without reopening the r1-r3 findings. PUT now derives the prospective effective object before dispatch canonicalization, constructs one index from the complete effective model registry, strictly validates and normalizes the effective references, and only then canonicalizes source references against the same exact-key-first namespace. Source providers, models, environment overlays, interpolated values, and resolved file-secret contents are never copied into the source projection.

The important ordering and behavior are sound:

- exact model keys are tested before aliases in the shared index, so a global/runtime exact key cannot be pre-empted by a project-only slug;
- duplicate effective slugs are rejected deterministically before either projection is serialized;
- effective default, fallback, tier, and role references all use the same strict routine, with sorted tier/role traversal;
- source references use the same complete namespace, canonicalize trimmed exact keys and unique aliases, and retain an unresolved raw value when a runtime field override masks it;
- an unresolved effective reference remains fatal, including fallback/tier/role fields, before main-file or replica mutation;
- PUT persists and replicates only normalized source while publishing only normalized effective state;
- reload remains publication-only and does not rewrite source;
- the existing mutation gate still spans source selection, merge, both validations, fallible serialization, atomic main persistence, replica propagation, live publication, and generation increment;
- caller cancellation cannot stop an accepted blocking transaction, and older/newer PUT plus PUT/reload ordering remains serialized;
- replica failure remains isolated after the authoritative main commit and before the infallible live swap, without rolling back main truth.

I specifically challenged startup -> PUT and reload -> PUT with global exact-key/source-alias collisions, named and hierarchical environment overrides, unresolved masked source references, file references, interpolation, global registries, rejection atomicity, and replica bytes. No counterexample survived. The fallback, tier, and role variants have no separate production branch: each passes through `normalize_dispatch_references` with the same effective index and rejection flag. Public builder and caller-state startup validation remain before startup side effects.

## Independent verification

All commands ran from the clean exact-tip review worktree with build output in `/private/tmp/ctrl02-config-r4-review-target`.

```text
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/private/tmp/ctrl02-config-r4-review-target \
  cargo test -p roko-serve routes::config::tests -- --nocapture
```

Exit 0; 19 passed, 0 failed, 377 unit tests filtered out. This includes the isolated startup/reload exact-key collision children, masked-source and runtime-layer preservation children, invalid rejection, concurrent disjoint PUT, PUT/reload exclusion, older/newer propagation, cancelled request completion, replica failure, ancestor-source seeding, response masking, and atomic-main failure.

```text
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/private/tmp/ctrl02-config-r4-review-target \
  cargo test -p roko-core config::loader -- --nocapture
```

Exit 0; 27 passed, 0 failed, 1067 unit tests filtered out. The suite covers effective exact-key precedence, source unique-alias canonicalization, unresolved masked-source retention, strict nested reference errors, deterministic tier/role error ordering, empty-registry compatibility, runtime resolver/file parity, global/env precedence, provider validation, and config discovery.

The two public startup boundaries were run independently:

- `cargo test -p roko-serve server_builder_rejects_ambiguous_models_before_startup -- --nocapture` — exit 0; 1 passed.
- `cargo test -p roko-serve run_server_with_state_rejects_invalid_config_before_side_effects -- --nocapture` — exit 0; 1 passed.

`CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/private/tmp/ctrl02-config-r4-review-target cargo check -p roko-core -p roko-serve --all-targets` exited 0 in 5m12s. Its only diagnostics were the two pre-existing missing-documentation warnings in unchanged `crates/roko-core/benches/engram_bench.rs`.

`cargo fmt --all -- --check`, locked no-dependency metadata, `git diff --check`, dependency-file identity, implementation ancestry, and reserved-scope checks all exited 0.

## Scope, history, and bounded acceptance

- `a4c3ca731` is directly based on rejected r4 tip `960082d2`; `788aedfd` is its evidence-only child.
- Direct r5 scope is only `crates/roko-core/src/config/loader.rs`, `crates/roko-serve/src/routes/config.rs`, and the existing implementation evidence file: 310 insertions and 51 deletions.
- Loader blob `e659c60d1e27c6f4a62f4f22ad48b30bdd41988c` and route blob `f5d8fbf429510e956be99bc21cd8129202ff9704` match the exact candidate.
- Root/core/serve manifests and `Cargo.lock` are byte-identical to r4. There is no schema, response-shape, startup API, runner, master, index, status, credential, deployment, or unrelated StateHub change.
- The new public loader helper is narrowly scoped to paired source/effective dispatch normalization and shares the same index and field traversal as the established strict single-config helper.

This accepts only the reconstructed CTRL-02 config-dispatch precursor. It does not mark SH05-T01 or broader SH05/E42/E04/E18/P17/P20, CLI dual-config convergence, general config provenance, or general secret-redaction work complete.

Final status: **ACCEPTED for integration, subject to coordinator-owned merge and post-merge verification.**

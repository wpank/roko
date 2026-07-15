# CTRL-02 config dispatch — independent review r4

- Task: `CTRL-02-CONFIG-DISPATCH-r4`
- Reviewer role: independent review; no candidate implementation, manifest, status, or master edits
- r3 candidate/rejection: `f23986f3ceb657fd061c733f56ae6e7c421f8ab0`; integrated record `52bfbb9f9`
- r4 implementation: `8ecda943b5b2ba11c3ccfae02bce6e16cee89b97`
- Exact evidence-bearing candidate reviewed: `960082d2518a8bfc851a77afb37f4699f05469cb`
- Earlier integrated rejections: `acac82c6e`, `fafe21aff`, `52bfbb9f9`
- Verdict: **REJECTED**

## Release-blocking finding

### Raw-source normalization can override effective exact-key precedence

r4 correctly stops using effective `AppState` as the persistence base and separately derives source and effective objects. Its ordering is nevertheless unsafe when runtime layers enlarge the model namespace. PUT normalizes the raw source first (`routes/config.rs:164-166`), then merges global/environment/runtime layers from that already-mutated source (`routes/config.rs:168-176`), and finally normalizes the effective object (`routes/config.rs:177-178`). The normalization contract gives an exact model key priority over a slug alias (`loader.rs:426-441`), but the first pass cannot see exact keys contributed by the global layer.

A project can therefore be valid at production startup and reload yet change dispatch identity during an unrelated accepted PUT. For example:

- project source defines model key `local`, whose slug is `shared`, and sets `agent.default_model = "shared"`;
- global config defines the exact model key `shared`;
- startup/reload effective validation sees both keys and correctly retains exact key `shared`;
- PUT's raw-only pass sees only project model `local` and irreversibly rewrites the reference to `local`;
- global merge then cannot restore the original exact-key choice, so source, replicas, response, and live effective state all select `local`.

This violates canonical-key precedence and the required startup/reload-to-PUT source/effective parity. It is not a presentation-only difference: an unrelated PUT silently changes the provider/model used for dispatch and persists that change.

## Exact-tip reproduction

I temporarily added an isolated reviewer-only parent/child unit test at exact tip `960082d2`. It used a private `HOME`, synthetic project/global providers, a real production unified startup load plus dispatch validation, one ephemeral replica, and an unrelated production PUT changing only `server.port`. The child first proved startup selected exact key `shared`; after PUT it proved project source, replica source, and live state all selected `local`.

```text
CARGO_INCREMENTAL=0 \
CARGO_TARGET_DIR=/private/tmp/ctrl02-config-r4-review-target \
  cargo test -p roko-serve \
  routes::config::tests::reviewer_repro_source_alias_overrides_effective_exact_key \
  -- --exact --nocapture
```

Result: exit 0; 1 passed, 0 failed, 395 unit tests filtered out; the test ran in 0.07 s after a clean 10m39s build. It passed because the assertions described the defect. The isolated process exited normally and its temporary directory removed project, replica, and private-global fixtures.

The reviewer-only tests were removed with named patches. The restored route blob is `6a705fdd0daa0a7ec2b4db606b4731e4ad5764d7`, exactly equal to `960082d2:crates/roko-serve/src/routes/config.rs`; `git diff --exit-code 960082d2 -- crates/roko-serve/src/routes/config.rs` passed.

## Required correction

Resolve alias identity against the complete model namespace that governs effective dispatch before mutating source references. A source field must not be rewritten using a project-only alias map when a higher-precedence exact key exists after global merge. Preserve runtime-only field overlays and secret references in raw source, while validating both projections before persistence. Add isolated startup -> PUT and reload -> PUT regressions where a project slug collides with an exact global model key; require source/replica bytes to retain safe source values and require live state to retain effective exact-key precedence. Re-run the existing ambiguity, cancellation, older/newer, replica-failure, ancestor, `ROKO_CONFIG`, and rejection-atomicity coverage.

## Scope and proportional verification

- Direct r4 correction scope `f23986f3..960082d2`: only `crates/roko-core/src/config/loader.rs`, `crates/roko-serve/src/routes/config.rs`, and the existing evidence file (396 insertions, 121 deletions). `crates/roko-serve/src/lib.rs` is unchanged by r4.
- Cumulative scope from original assigned base `b59e497e..960082d2`: the expected loader, serve startup, serve config route, and evidence paths only.
- No manifest, `Cargo.lock`, runner, master, status, index, credential, or deployment change is present. The implementation adds only the narrow public `resolve_config_source` loader bridge; schemas and response shapes are unchanged.
- `cargo fmt --all -- --check`, `cargo metadata --locked --no-deps --format-version 1`, cumulative `git diff --check`, dependency-file identity, ancestry, and reserved-scope checks exited 0.
- After restoring candidate bytes, a pristine route sweep began recompiling under a different Cargo incremental fingerprint. It was stopped without a result when the coordinator requested immediate evidence and no additional breadth. The author's recorded 17/17 route, startup, loader, integration, all-target, formatting, and metadata passes are not disputed; none exercises cross-layer exact-key-versus-alias precedence.
- Static review found the r3 source-promotion case materially corrected for non-colliding identities, source bytes used for replicas, effective state used for live publication, and mutation ownership retained across main persistence, propagation, publication, and generation ordering. These positives do not resolve the reproduced dispatch change.

Final status: **REJECTED; do not integrate `8ecda943b` / `960082d2` as the config-dispatch correction.**

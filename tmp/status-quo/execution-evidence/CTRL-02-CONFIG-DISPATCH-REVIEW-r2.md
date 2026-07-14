# CTRL-02 config-dispatch independent review r2

## Assignment and candidate identity

- Task: `CTRL-02-CONFIG-DISPATCH-r2`, independent precursor review.
- Assigned base: `b59e497e71143212dfca0f50dbbfac3ce4a47844`.
- Rejected candidate parent: `91992857f160097b4e850708dfe7b606f262ee0a`.
- Correction source commit: `af66e1446b9f318bb1fc0ea5792b81bc73f03511`.
- Exact evidence-bearing candidate reviewed:
  `83bec51978422093885a5304b4144a24b1949697`.
- Review branch/worktree: `review/CTRL-02-config-dispatch-r2-83bec519` /
  `reviews/CTRL-02-config-dispatch-r2-83bec519`.

I read the complete master; the complete r1 rejection; the full SH01, SH05,
E04, E18, E42, P17, and P20 manifests; issues 23 and 55; the July 14 audit and
self-heal audit; the candidate evidence and full cumulative/correction diffs;
and current loader, schema, startup, route, watcher, daemon, state, hot-reload,
runner, and test call sites. The correction changes exactly three production
paths, while the evidence-bearing candidate changes the same four cumulative
paths as r1:

```text
crates/roko-core/src/config/loader.rs
crates/roko-serve/src/lib.rs
crates/roko-serve/src/routes/config.rs
tmp/status-quo/execution-evidence/CTRL-02-CONFIG-DISPATCH.md
```

No manifest, master, index, lockfile, runner, status, or unrelated StateHub
hunk changed.

## R1 finding disposition

All three r1 findings are materially addressed in the submitted correction.
Both public serve startup profiles now use validation before side effects;
tier and role keys are sorted before validation and repeatedly tested; and
PUT, explicit reload, and watcher reload share one mutation gate covering
snapshot/load, validation, atomic main-file persistence, and live-state swap.
The barrier tests establish authoritative main-file/live-state ordering for
concurrent PUTs and PUT-versus-reload. These improvements do not close the new
release blocker below.

## Findings

### High: reload writes resolved secrets and runtime-only overlays into project config

`reload_config_from_disk_with_hook` in
`crates/roko-serve/src/routes/config.rs:248-277` loads with
`load_config_unified(&state.workdir)`, serializes the resulting `RokoConfig`,
and atomically replaces the project `roko.toml` whenever the bytes differ.
That loader is explicitly an *effective runtime* loader. Its default options
merge global config and apply named and hierarchical environment overrides
(`crates/roko-core/src/config/loader.rs:67-85`), after which it interpolates
environment strings and calls `resolve_file_secrets`
(`loader.rs:261-274`). The latter replaces every provider
`extra_headers.*_file` entry with the literal trimmed file contents under the
base key (`schema.rs:770-786`).

Consequently, an ordinary explicit, watcher, or daemon reload rewrites source
configuration with values that did not originate in that project file. Most
critically, this converts a safe source reference such as
`authorization_file = "/path/to/secret"` into
`authorization = "<literal secret bytes>"` in `roko.toml`. It also makes
temporary `ROKO_*`/`ROKO__*` values and user-global providers/models permanent
project configuration. This collapses source/effective provenance and directly
crosses the candidate's E04/E18 redaction and E42 provenance non-goals.

I reproduced the secret case against an exact archive of candidate
`83bec519...` with one reviewer-only test (never added to the candidate): write
a provider `authorization_file`, call the production
`reload_config_from_disk`, and reread `roko.toml`. The test passed only when it
asserted that the literal marker `review-only-file-secret` was present and the
`authorization_file` reference was gone. The focused production test output
was:

```text
test routes::config::tests::adversarial_reload_persists_resolved_file_secret ... ok
test result: ok. 1 passed; 0 failed
```

This is candidate-introduced behavior: r1 reload did not persist the unified
effective object. Existing GET/PUT response masking cannot protect a secret
written to the project file.

Required correction: keep source configuration distinct from effective
runtime configuration. Reload may validate and publish an effective object,
but it must not serialize merged global values, environment overlays,
interpolated values, or resolved file-secret contents into the project source.
If alias canonicalization must be persisted, perform it on a source-loaded
representation whose loader disables all runtime overlays and preserves secret
references, or structurally patch only the dispatch references in the raw
document. Add explicit regressions proving file-secret references remain
references and literal contents never reach `roko.toml`, hierarchical/named
environment overrides remain ephemeral, and global config is not copied into
the project file.

### Medium: post-commit ephemeral propagation is not generation ordered

The main-file/live-state transaction ends before `update_config` reads and
writes ephemeral workspaces (`routes/config.rs:90-117`). The returned
generation is used only in a warning log. Two accepted requests can therefore
commit generations 1 then 2 under the gate, while their asynchronous
ephemeral writes finish in the opposite order; generation 1 can overwrite
generation 2 in an ephemeral `roko.toml`. Request cancellation after the
authoritative commit can also skip some or all propagation. The new
concurrency tests do not create ephemeral workspaces and therefore prove only
main-file/live-state ordering.

Required correction: serialize or generation-guard propagation so an older
accepted generation can never overwrite a newer one. Ensure propagation that
belongs to an already committed request has explicit completion/cancellation
semantics, keep per-workspace failures separate from authoritative rollback,
and add a deterministic overlapping-propagation regression.

## Verified behavior and bounded ownership

Apart from the findings, the corrected startup validation, deterministic
nested-reference selection, atomic main-file commit, invalid-input rollback,
serial masking, and authoritative PUT/reload ordering behave as described by
the implementation evidence. The normalization semantics remain sound: exact
keys take precedence, unique aliases canonicalize, duplicate owners and nested
field errors are stable, and an empty model registry preserves compatibility.

The process-global gate is broader than the r1 recommendation of state-owned
ownership and its PUT poison handling differs from reload recovery. I did not
promote that design concern to a separate rejection because the reviewed
production paths contain no demonstrated panic under the gate. The correction
should nevertheless prefer path/state-scoped ownership and consistent poison
recovery when addressing the findings above.

## Independent commands and results

```text
candidate identity: 83bec51978422093885a5304b4144a24b1949697
candidate parent: af66e1446b9f318bb1fc0ea5792b81bc73f03511
correction range: 91992857f..af66e1446
cumulative candidate scope: exact four paths listed above
git diff --check: pass
cargo fmt --all -- --check: pass
cargo check -p roko-core -p roko-serve --all-targets
  exit 0; only the two recorded missing-documentation warnings in unchanged
  crates/roko-core/benches/engram_bench.rs

cargo test -p roko-core config::loader -- --nocapture
  exit 0; 23 passed, 0 failed
cargo test -p roko-core config::timeouts -- --nocapture
  exit 0; 7 passed, 0 failed

reviewer-only exact-candidate adversarial test
  exit 0; 1 passed, 0 failed
  proved reload persisted literal resolved file-secret bytes

cargo test -p roko-serve routes::config::tests -- --nocapture
  exit 0; 11 passed, 0 failed
cargo test -p roko-serve server_builder_rejects_ambiguous_models_before_startup -- --nocapture
  exit 0; 1 passed, 0 failed
cargo test -p roko-serve run_server_with_state_rejects_invalid_config_before_side_effects -- --nocapture
  exit 0; 1 passed, 0 failed
cargo test -p roko-serve config -- --nocapture
  exit 0; 26 passed, 0 failed

cargo test -p roko-core config -- --test-threads=1
  exit 101; 193 passed, 4 failed
  failures: config::cache::tests::watched_cache_sees_file_change and the three
  repository-root-roko.toml schema tests recorded by both r1 and the author
```

The four broad failures reproduce the immutable candidate evidence exactly.
The changed loader adds one passing test relative to r1; none of the four
unchanged baseline failures was removed, masked, or added.

The disposable adversarial source copy and all reviewer-created build artifacts
are outside the review worktree and are removed before handoff. The candidate
worktree was clean before this record was added; no production or candidate
evidence file was edited.

## Verdict

**REJECTED** for exact candidate
`83bec51978422093885a5304b4144a24b1949697`.

Confidence: high. The correction resolves every prior rejection, but its new
reload persistence path writes resolved secrets and runtime-only overlays into
the project source, and its post-commit ephemeral copies can regress to an
older accepted generation. Correct both, update the implementation evidence's
effective-persistence/security claims, complete the focused gates, and submit
a new immutable candidate for independent review.

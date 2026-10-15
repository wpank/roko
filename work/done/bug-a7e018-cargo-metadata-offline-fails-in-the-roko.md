+++
id = "bug-a7e018"
kind = "bug"
title = "cargo metadata --offline fails in the roko repo, so plan footprints and gate impact analysis treat every .rs edit as touching every package"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
last_verified_rev = "19f112373"
source = "tmp/hermes-roko-v5/findings/roko-demos.md (roko-demos, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: demo plans run from Hermes in the roko repo"
anchors = ["crates/roko-cli/src/runner/impact_analysis.rs::cargo_metadata", "crates/roko-cli/src/graph_execution/plan_set.rs::CargoWorkspace"]
links = { depends_on = [], blocks = [], related = ["bug-0c7f25"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn cargo_metadata_reads_this_workspace_offline' crates/roko-cli/ && cargo test -p roko-cli cargo_metadata_reads_this_workspace_offline"

[closed]
at = 2026-10-15
at_ts = "2026-10-15T18:12:46Z"
commit = "19f112373"
size = "S"
claimed_at = "2026-10-15T15:51:48Z"
forced = false
evidence = "cargo_metadata (runner/impact_analysis.rs) falls back to cargo metadata --no-deps when the offline resolve fails (here: android_system_properties missing from the registry cache), and reverse_dependents follows members' path dependencies without a resolve graph. cargo test -p roko-cli --lib: cargo_metadata_reads_this_workspace_offline, reverse_dependents_follow_path_dependencies_without_a_resolve_graph and this_workspace_scopes_the_default_check_to_the_written_crate pass; full lib suite 3631 passed, 0 failed (gist test skipped); clippy -p roko-cli -D warnings clean. plan run --dry-run now prints each plan's whole-plan check."
+++

## Problem

`cargo metadata --format-version=1 --locked --offline` fails in the roko repo on a machine whose cargo cache lacks
the lockfile's crates for other platforms: "failed to download `android_system_properties v0.1.5` … attempting to make
an HTTP request, but --offline was specified". Every plan run in the roko workspace logs
`WARN … plan_set: cargo metadata failed; plans that touch Rust code will not run at the same time`. Seen 2026-10-15 on
Will's machine (`~/.cargo` as is) with roko-main 7b6a5af44 and with Will's serve binary.

Without the package graph, `AreaResolver::for_path` counts every `.rs` path as `Area::AllPackages`, so:

- the default whole-plan check (`plan_verify.rs::default_plan_verify`) is `cargo fmt --all -- --check` plus
  `cargo clippy --workspace --no-deps -- -D warnings`, and runs no tests, for every plan that writes Rust. A one-file
  demo (`demo-hello-world`: its task passed in 13 s for $0.004) then sat in a cold build of the whole workspace in
  the delivery's regression checkout;
- plans that touch Rust never run side by side;
- gate impact analysis (`impact_analysis::analyze`, focused mode) falls back to full verification.

`cargo metadata --offline --locked --filter-platform aarch64-apple-darwin` and `cargo metadata --offline --locked
--no-deps` both succeed in the same checkout.

## Why it matters

Self-hosting runs in the roko repo get whole-workspace lint checks and lose their per-package tests in the
whole-plan check; Hermes users of the roko repo wait many minutes for any plan that writes a `.rs` file.

## Where

- `crates/roko-cli/src/runner/impact_analysis.rs::cargo_metadata`: the one invocation, shared by
- `crates/roko-cli/src/graph_execution/plan_set.rs::CargoWorkspace::load` (footprints, whole-plan defaults) and
  `impact_analysis::analyze` (focused gates).

## Plan

1. Pass `--filter-platform <host triple>` (the triple roko was built for, or `rustc -vV`'s `host:`): cargo then
   resolves only what the host builds, which is in the cache whenever the workspace has built.
2. If that still fails, retry with `--no-deps`: workspace members and their path dependencies are all the
   footprints and `dependents` need.
3. Test against this workspace: `cargo_metadata(repo root)` returns Ok offline.

## Done when

`cargo_metadata_reads_this_workspace_offline` passes (the `[[verify]]` command), and a plan run in the roko repo no
longer logs "cargo metadata failed".

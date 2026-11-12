+++
id = "bug-517d78"
kind = "bug"
title = "An installed roko serves the portal from its build tree's apps/portal/out on disk before its embedded copy, so re-exporting or cleaning that folder changes an installed binary's portal without a rebuild (Will's ~/.cargo/bin/roko reads roko-v7-build/apps/portal/out at run time); serve from disk first only in debug builds or with ROKO_SPA_DIR"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/embedded"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "0658ddfe3"
source = "round v8 portal-v8 audit (tmp/hermes-roko-v8/design/portal/before, main bd93521ee), the lead's approved proposal (tmp/hermes-roko-v8/design/portal/README.md, 2026-11-12)"
discovered_from = "agent:portal-v8 (round v8, bd93521ee)"
anchors = ["crates/roko-serve/src/embedded.rs::disk_portal_dir", "crates/roko-serve/src/embedded.rs::disk_demo_dir"]
links = { depends_on = [], blocks = [], related = ["gap-195374"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_release_build_serves_its_embedded_portal' crates/roko-serve/src && cargo test -p roko-serve --lib a_release_build_serves_its_embedded_portal"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T19:10:00Z"
commit = "0658ddfe3"
by = "portal-v8 (round v8)"
executor = "claude-agent"
via = "manual"
model = "claude-opus-5-5"
size = "S"
claimed_at = "2026-11-12T18:20:00Z"
forced = false
evidence = "crates/roko-serve/src/embedded.rs: choose_disk_dir picks a UI's folder for both the portal and the demo app: an explicit override (ROKO_SPA_DIR) in any build, the build tree's folder only when SERVE_BUILD_TREE (cfg!(debug_assertions)), else the embedded copy. In roko-work-gap-6d2dd2 (heavy.sh cargo, CARGO_INCREMENTAL=0): cargo test -p roko-serve --lib embedded:: 19 passed (a_release_build_serves_its_embedded_portal, a_debug_build_serves_its_build_trees_portal and the 17 serving tests), clippy -p roko-serve clean, nightly fmt. The release path is checked through the choice function; no release build was made."
+++

## Problem

An installed roko serves the portal from its build tree's `apps/portal/out` on disk, before its embedded copy.

`crates/roko-serve/src/embedded.rs::disk_portal_dir` looks for `$CARGO_MANIFEST_DIR/../../apps/portal/out/index.html`
at run time, in every build, and serves that folder when it exists. `CARGO_MANIFEST_DIR` is the build tree's path,
baked in at compile time. So:

- Will's `~/.cargo/bin/roko` (installed 2026-10-28, built in `roko-v7-build`) serves
  `/Users/will/dev/nunchi/roko/roko-v7-build/apps/portal/out` (its strings name the path), not the portal it embedded.
- Re-exporting, cleaning or checking out another commit's export in that folder changes an installed binary's
  portal without a rebuild; deleting the folder switches it back to the embedded copy.
- A binary built in the main checkout serves the main checkout's `apps/portal/out`, which is from 2026-09-29.

The demo app has the same rule (`disk_demo_dir`).

## Why it matters

Goal visibility, and the portal overhaul's own delivery: once the new portal merges, an installed roko should show the
portal it was built with, and the lead's rebuild of Will's binary should be what Will sees.

## Where

- `crates/roko-serve/src/embedded.rs`: `disk_portal_dir`, `disk_demo_dir`, and the serving functions that consult
  them before the `rust-embed` assets.
- `crates/roko-serve/build.rs` (what is embedded) is not the problem and stays.

## Plan

1. Serve from the build tree's folder first only in a debug build (`cfg!(debug_assertions)`), where it lets a
   developer re-export the portal and refresh without recompiling (the module's documented dev workflow).
2. `ROKO_SPA_DIR` keeps working in every build: it is an explicit choice.
3. A release build serves its embedded portal (and demo app) only.
4. Tests: `a_release_build_serves_its_embedded_portal` (the choice function with dev = false ignores an existing
   build-tree folder), and a debug-build case that still prefers it.

## Done when

`cargo test -p roko-serve --lib a_release_build_serves_its_embedded_portal` passes, and `embedded.rs`' other tests.

## Notes

- Found in round v8 by portal-v8 while auditing the portal; the lead asked for it fixed this round.

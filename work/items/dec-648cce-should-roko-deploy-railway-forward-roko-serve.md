+++
id = "dec-648cce"
kind = "decision"
title = "Should roko deploy railway forward ROKO__SERVE__AUTH__API_KEY to the services it deploys?"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-cli/deployment"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report, checked on work/gap-e9660f at 6820f1c2d)"
anchors = ["crates/roko-cli/src/deployment.rs", "docker/RAILWAY.md"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = [], blocks = [], related = ["bug-524a3b", "gap-ed511d", "gap-e9660f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'ROKO__SERVE__AUTH__API_KEY' crates/roko-cli/src/deployment.rs"
+++

## Problem

Once secrets move out of `roko.toml` (gap-e9660f, bug-524a3b), a Railway deployment needs serve's API key in its environment. At f48cc207d, `crates/roko-cli/src/deployment.rs` forwards no `ROKO__*` or API-key variables. Workers inherit the control plane's environment, so forwarding the key to the control plane also hands it to every worker.

## Decision

The options:

- **(a) Forward it to the control plane only.** Workers get a separate, narrower credential, or none.
- **(b) Forward it to every service.** This is simple, but every worker can then act as an admin client of serve.
- **(c) Don't forward it.** The operator sets it in Railway's dashboard, and the docs say so (gap-ed511d).

**Recommended default: (a)**, because workers shouldn't hold the control plane's admin key.

## Done when

- [ ] Will decides. Under (a) or (b), `deployment.rs` forwards the variable as decided, and the `[[verify]]` command passes. Under (c), close this item as decided, with the docs change as evidence.

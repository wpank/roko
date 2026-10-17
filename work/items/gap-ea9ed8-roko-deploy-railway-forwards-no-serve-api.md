+++
id = "gap-ea9ed8"
kind = "gap"
title = "roko deploy railway forwards no serve API key: dec-648cce's decision, the control plane only, is not implemented"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-cli/deployment"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "bug-ec244b worker, 2026-10-16"
discovered_from = "bug-ec244b: dec-648cce's verify greps deployment.rs, which never held the Railway env code"
anchors = ["crates/roko-cli/src/commands/server.rs::collect_railway_env_vars", "crates/roko-cli/src/commands/server.rs::railway_worker_env"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = [], blocks = [], related = ["dec-648cce", "gap-ed511d", "bug-ec244b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn railway_env_forwards_the_serve_key_to_the_control_plane_only' crates/roko-cli/src/ && cargo test -p roko-cli --bin roko railway_env_forwards_the_serve_key_to_the_control_plane_only"
+++

## Problem

Will's answer to dec-648cce (2026-10-15, `tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md`, decision 3: accept
default (a)) is that `roko deploy railway` forwards `ROKO__SERVE__AUTH__API_KEY` to the control plane only, and
workers get a narrower credential or none. The deploy code does neither:

- `collect_railway_env_vars` (`crates/roko-cli/src/commands/server.rs`) forwards `GITHUB_TOKEN`, `GH_TOKEN`,
  `SLACK_TOKEN`, `SLACK_BOT_TOKEN`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY` and `ROKO_SERVER_AUTH_TOKEN` (the event
  ingest token), never the serve key.
- `cmd_deploy_railway` gives the control plane those variables plus `ROKO_WORKER_CALLBACK_TOKEN`, and then starts
  mirage's environment and each worker's (`railway_worker_env`) from a clone of them. Adding the key to that set
  as it stands would hand the control plane's admin key to every worker, which is what option (a) rules out.

So the deployed control plane gets the key only if the operator sets it in Railway's dashboard by hand. The
posture check's hint in the same file says "the deployed server needs the variable too".

dec-648cce was closed as decided with `forced = true`. Its verify grepped `crates/roko-cli/src/deployment.rs`,
which holds only the Sigstore verifier and never had the Railway environment code (it was already in
`commands/server.rs` at f48cc207d, when dec-648cce was filed), so it could never pass. bug-ec244b retired that
verify in favour of this item.

## Why it matters

Goal `release` (spec-ae5f94, release blockers, which still lists dec-648cce unchecked). Since secrets moved out of
`roko.toml` (gap-e9660f, bug-524a3b), a Railway deployment needs serve's API key in the control plane's environment,
and the decision says only the control plane may hold it.

## Where

`crates/roko-cli/src/commands/server.rs`: `collect_railway_env_vars`, `railway_worker_env`, and
`cmd_deploy_railway`, which builds the control plane's, mirage's and the workers' `RailwayDeploySpec.env_vars`. The
tests sit in the same file's `tests` module (`railway_worker_env_reuses_control_plane_callback_token_and_sets_callback_id`).

## Current state

Nothing forwards the serve key. `docker/RAILWAY.md` and the docs/v3 auth pages describe the key's variable
(gap-ed511d).

## Plan

1. Read the key the way serve resolves it (`serve.auth.api_key`, which `ROKO__SERVE__AUTH__API_KEY` sets, also from
   `.roko/.env`) and add `ROKO__SERVE__AUTH__API_KEY` to the control plane's variables only.
2. Keep it out of mirage's and every worker's environment: build those from a set without the key. Workers keep
   `ROKO_SERVER_AUTH_TOKEN` and `ROKO_WORKER_CALLBACK_TOKEN`, their narrower credentials.
3. Add the test `railway_env_forwards_the_serve_key_to_the_control_plane_only`: the control plane's variables carry
   the key, and the worker and mirage variables do not.
4. Update `docker/RAILWAY.md` if it tells the operator to set the key by hand.

## Done when

The `[[verify]]` command passes: the named test exists and passes.

## Notes

- Never print the key: not in dry-run output, logs or errors.
- Found on 2026-10-16 by the bug-ec244b worker while repointing dec-648cce's verify.
- 2026-10-17: the `[[verify]]` command originally said `cargo test -p roko-cli --lib …`. `roko-cli`
  has no `[lib]` target for `commands::server` (`lib.rs`'s inline `commands` module re-exports only
  `diagnose` and `learn_loops`; `server` exists only in the `main.rs`-rooted tree compiled into the
  `roko` binary). `--lib` builds clean and matches 0 tests, so it would have passed before this fix
  too (confirmed empirically). Repointed to `--bin roko`, which actually runs the test.

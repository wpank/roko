+++
id = "bug-557dd1"
kind = "bug"
title = "roko doctor fails every fresh roko init workspace: serve_auth flags the default auth without a key, which roko serve covers with a launch token"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/doctor"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "8e68930c1"
source = "roko-init round v5, 2026-10-15 (tmp/hermes-roko-v5/findings/roko-init.md)"
anchors = ["crates/roko-cli/src/doctor.rs::check_serve_auth"]
links = { depends_on = [], blocks = [], related = ["bug-4e336b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn fresh_init_workspace_passes_serve_auth' crates/roko-cli/ && cargo test -p roko-cli --lib fresh_init_workspace_passes_serve_auth"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T01:16:11Z"
commit = "8e68930c1"
by = "roko-init"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-16T00:17:51Z"
forced = false
evidence = "8e68930c1 (on bug-d32a50, on main 1630fa1bf). check_serve_auth follows serve's launch-token rule: auth on with no api_key and no named api_keys is ok on a loopback [server] bind (roko serve mints a one-time launch token there), and still fails, naming the bind, on a bind beyond the machine. It reuses the lib's is_loopback_host. Checks at 8e68930c1: fresh_init_workspace_passes_serve_auth (the [[verify]] command) passes, asserting a fresh init workspace's doctor report is healthy and 0.0.0.0 without a key fails; all roko-cli lib/bin and roko-core tests pass; clippy -D warnings and fmt clean. Live, Will's real HOME: roko doctor in a fresh roko init workspace prints [ok] serve_auth and 'doctor: ok', 0 failed, exit 0. Before, it printed [fail] serve_auth and exited 1. Evidence: tmp/hermes-roko-v5/findings/roko-init.md"
+++

## Problem

`roko doctor` fails in every workspace that a fresh `roko init` wrote (exit 1, `doctor: fail`). Its one failed
check is `[fail] serve_auth: serve auth is enabled but api_key is empty`. The init template serializes the
default config, which includes `[serve.auth] enabled = true` and `api_key = ""`. That default is a valid local
setup: `roko serve` on a loopback bind mints a one-time launch token when no key is set (`commands/server.rs`,
the `launch_token` block). The doctor's `[fail]` contradicts it.

Seen 2026-10-15 (roko-init, round v5) in four fresh workspaces (Python, Node, empty, Rust) and in a workspace from
`main`'s `roko init` (`tmp/hermes-roko-v5/roko-init/evidence/*doctor*.out`).

## Why it matters

The README and `roko init`'s own "Next:" hint send a newcomer to `roko doctor` right after `roko init`. The first
thing they see is a failure, with a fix (`roko config set serve.auth.api_key …`) they don't need for local use.
The real problems in the report get lost next to it.

## Where

- `crates/roko-cli/src/doctor.rs::check_serve_auth`: fails on `enabled && api_key` empty whenever `[serve]` is
  explicit.
- `crates/roko-cli/src/commands/server.rs`: the launch-token rule (auth enabled, loopback bind, no key).
- `crates/roko-cli/src/commands/init.rs::render_init`: writes the default `[serve.auth]`.

## Current state

Open at the bug-4e336b branch (2026-10-15).

## Plan

1. Make the check follow serve's rule: a loopback `serve.bind` with auth enabled and no key is ok ("roko serve
   mints a launch token"). Keep `fail` for a non-loopback bind with no key, since every request would get 401
   there.
2. Test: a fresh `roko init` workspace's doctor report has no `[fail]` from `serve_auth`; a `0.0.0.0` bind with
   no key still fails.

## Done when

`fresh_init_workspace_passes_serve_auth` passes (the `[[verify]]` command).

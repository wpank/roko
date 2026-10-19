+++
id = "bug-10d745"
kind = "bug"
title = "roko serve restart moves a serve started by an older roko to a new port: serve.json had no port field, and restart does not read it back from the url"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/serve"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
last_verified_rev = "a07b3ac2f"
source = "round-v7 hermes-pilot dogfood report to roko-autostart, 2026-10-19"
anchors = ["crates/roko-cli/src/serve_lifecycle.rs::restart_options"]
parent = "spec-13c7ba"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn restart_keeps_a_legacy_serves_port_from_its_url' crates/roko-cli/src/ && cargo test -p roko-cli --lib restart_keeps_a_legacy_serves_port_from_its_url"

[closed]
at = 2026-10-19
at_ts = "2026-10-19T15:47:24Z"
commit = "a07b3ac2f"
by = "commit trailer"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-19T15:06:53Z"
forced = false
evidence = "a07b3ac2f: serve_lifecycle::served_port reads the port from serve.json's url when an older roko wrote no port field; restart waits up to 2 s for that port to come free after the stop, falls back to the configured or an OS port if it is taken at bind (DetachOptions::port_fallback; --port never moves), and reports moved_from in its JSON entry and a moved line in its human output. Proof: cargo test -p roko-cli --lib restart_keeps_a_legacy_serves_port_from_its_url passes (the item verify), the 14 serve_lifecycle unit tests and serve_detach_lifecycle_end_to_end pass, cargo clippy -p roko-cli --no-deps -- -D warnings and nightly fmt --check are clean. By hand: a roko 3b34aed92 serve on --port 6843 restarted by this build came back on 6843; with 6843 taken the moment it came free, restart moved it to the configured 6846 and printed the move."
+++

## Problem

`roko serve restart` moves a serve that an older roko started to a new port.

The round-v7 session reported this on 2026-10-19: a serve started by roko `3b34aed92` with `--port 6681` came back on
port 65254 after `roko serve restart` (roko-main-f33fd217c). The old 0.6-based plugin had that URL pinned, so Hermes
lost roko until a manual stop and `--detach --port 6681`.

Cause, read from the code:
- `restart_options` (`crates/roko-cli/src/serve_lifecycle.rs`) reuses `served.port`.
- `ServeEndpoint.port` is `#[serde(default)]`, so a serve.json written before gap-fb1594 reads as port 0.
- With port 0, restart passes no port. `--detach` then takes the configured port if it is free, else one the OS picks.
- The old port is still in serve.json's `url`, but nothing reads it back.

## Why it matters

- Restart (and `./dev.sh install`'s `restart --all`) should keep a serve where clients find it. That covers bookmarks,
  pinned URLs, and older plugins.
- Will's own :6677 serve is such a legacy serve, started by `roko-main-3b34aed92`. His configured port is also 6677,
  so he would not hit this, but a project started with an explicit `--port` would.

Parent: spec-13c7ba.

## Where

`crates/roko-cli/src/serve_lifecycle.rs::restart_options`, plus `port_is_free`.

## Plan

1. When `served.port == 0`, take the port from `served.url`.
2. After the stop, wait briefly (up to about 2 s) for the old port to be free before falling back.
3. If restart still has to move the serve, say so in its report: a `moved_from` port, and a line in the human output.

## Done when

- A unit test passes: a legacy endpoint (port 0, url `…:6681`) restarts with `Some(6681)` while 6681 is free.
- The integration test still passes.
- Verify: the named test.

## Notes

Reported by the round-v7 session (`tmp/hermes-roko-v7/findings/hermes-pilot.md`).

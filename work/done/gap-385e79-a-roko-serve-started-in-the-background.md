+++
id = "gap-385e79"
kind = "gap"
title = "A roko serve started in the background never shows its portal sign-in link, and no command prints it (--detach writes it only to serve.log, status hides the token): add roko serve link, the running serve's portal link with its token for its owner, so the portal's sign-in page can name one command"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/serve"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "6057dc5dc"
source = "round v8 portal-v8 audit (tmp/hermes-roko-v8/design/portal/before, main bd93521ee), the lead's approved proposal (tmp/hermes-roko-v8/design/portal/README.md, 2026-11-12)"
discovered_from = "agent:portal-v8 (round v8, bd93521ee)"
anchors = ["crates/roko-cli/src/commands/server.rs", "crates/roko-cli/src/serve_lifecycle.rs::hide_launch_token"]
links = { depends_on = [], blocks = [], related = ["gap-195374"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn serve_link_prints_the_portal_sign_in_link' crates/roko-cli/src && cargo test -p roko-cli --lib serve_link_prints_the_portal_sign_in_link"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T20:05:00Z"
commit = "6057dc5dc"
by = "portal-v8 (round v8)"
executor = "claude-agent"
via = "manual"
model = "claude-opus-5-5"
size = "S"
claimed_at = "2026-11-12T19:20:00Z"
forced = false
evidence = "roko serve link [--workdir W] [--open] (commands/server.rs) prints serve_lifecycle::sign_in_link: the running serve's URL with the launch token from .roko/runtime/serve.token; exit 1 with one sentence when no serve runs or its token is gone. In roko-work-gap-6d2dd2 (heavy.sh cargo, CARGO_INCREMENTAL=0): cargo test -p roko-cli --lib serve_lifecycle:: 15 passed incl. serve_link_prints_the_portal_sign_in_link; clippy -p roko-cli clean; nightly fmt. Portal: tsc clean, vitest 725/725, orphans none, build:export OK. End to end on a $0 round serve with this build (bin/roko-g385): serve link printed http://127.0.0.1:6684/#token=<hidden>, opening it signed the portal in and dropped the token from the address bar, and with no serve it exits 1: 'no roko serve runs in /private/tmp; start one with `roko serve --detach`'."
+++

## Problem

A roko serve started in the background never shows its portal sign-in link, and no command prints it.

- `roko serve` prints `portal: http://127.0.0.1:<port>/#token=<token>` when it starts in a terminal
  (`crates/roko-cli/src/commands/server.rs`).
- `roko serve --detach` (what Hermes uses) writes that line only to `.roko/logs/serve.log` (mode 600), and its own
  report hides the token (`serve_lifecycle.rs::hide_launch_token`). `roko serve status` prints the URL without it.
- The portal, opened without the token or after a restart, can only say "open the link roko serve printed".

## Why it matters

Goal visibility. The portal's sign-in page (gap-195374) should name one command that fixes it, as
the design system's offline rule asks (§4.13): "Open roko with its sign-in link … run `roko serve link`".

## Where

- `crates/roko-cli/src/commands/server.rs` (`ServeCmd`, `Status`), `crates/roko-cli/src/serve_lifecycle.rs`
  (the serve registry and `serve.json`), `.roko/runtime/serve.token` (written by roko serve, mode 600).

## Plan

1. `roko serve link [--workdir W] [--open]`: for the project's running serve, read its endpoint (`serve.json`) and
   its launch token (`.roko/runtime/serve.token`), and print `http://127.0.0.1:<port>/#token=<token>`; `--open` also
   opens it in the browser. Exit 1 with one sentence when no serve runs there.
2. It prints only to the token's owner: it reads a 600 file the same user wrote.
3. A test: `serve_link_prints_the_portal_sign_in_link`.

## Done when

`cargo test -p roko-cli --lib serve_link_prints_the_portal_sign_in_link` passes, and `roko serve link` prints a link
that signs the portal in on a round serve.

## Notes

- The portal's sign-in page names this command once it lands; until then it says to open the link roko serve
  printed, or to restart roko serve in a terminal.

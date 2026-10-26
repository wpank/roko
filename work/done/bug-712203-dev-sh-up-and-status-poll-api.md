+++
id = "bug-712203"
kind = "bug"
title = "dev.sh up and status poll /api/health, which answers 401 under roko's default auth, so up reports a timeout and status a stopped server"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["tooling/dev-sh"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
last_verified_rev = "8849a5383"
source = "roko-autostart round research, 2026-10-19 (roko 3b34aed92)"
anchors = ["dev.sh"]
parent = "spec-13c7ba"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qE '^serve_alive\\(\\)' dev.sh && grep -A2 '^serve_alive()' dev.sh | grep -q 'SERVE_PORT/health\"' && grep -q 'runtime/serve.token' dev.sh && ! grep -nE 'curl [^|]*SERVE_PORT/api/' dev.sh"

[closed]
at = 2026-10-19
at_ts = "2026-10-19T11:56:32Z"
commit = "8849a5383"
by = "commit trailer"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-19T10:13:25Z"
forced = false
evidence = "8849a5383: dev.sh serve_alive polls the public /health (up's wait loop, status, dump) and serve_api_get sends ROKO_API_KEY or .roko/runtime/serve.token (header on stdin) to the /api routes it prints. Confirmed first: under default auth /health answered 200 and /api/health 401. Scripted run against a real roko serve with default auth (scratch project, port 6841): the old dev.sh status printed 'serve not reachable', the new one the /api/health JSON. The static verify fails on 6ffd8a37f's dev.sh and passes now."
+++

## Problem

`./dev.sh up` waits for roko serve by polling `curl -sf http://localhost:$SERVE_PORT/api/health` (dev.sh ~513). `cmd_status` (~618) and the health printout (~1041) call the same route. Under roko's default auth, `/api/health` answers 401, so `curl -sf` fails:

- `up` reports a timeout although the server came up;
- `status` says the server is down.

The public liveness route is `/health`.

## Why it matters

These are tooling defects found while planning the roko-autostart round (2026-10-19). `./dev.sh install` (same round) sits next to these functions.

## Where

`dev.sh` `cmd_up` (~351), `cmd_status` (~591) and the health printout (~1041).

## Current state

This is a claim from the round's research and has not been run yet. Confirm it first: with `[serve.auth] enabled = true` and no key, `/api/health` should answer 401.

Confirmed on 2026-10-19 (roko-autostart round): against a real `roko serve` in a scratch project with the
default auth (a launch token, no API key), `curl /health` answered 200 and `curl /api/health` answered 401.

## Plan

- Poll `/health` for liveness.
- Where the richer `/api/health` body is shown, send the workspace's `serve.token` (or `ROKO_API_KEY`) as a Bearer token, the way `serve_client` does.

## Done when

A verify you write fails before the change and passes after it. For example, a static check that `up` polls `/health`, plus a scripted run against a serve with auth on, if that is cheap.

## Notes

Keep to `dev.sh`.

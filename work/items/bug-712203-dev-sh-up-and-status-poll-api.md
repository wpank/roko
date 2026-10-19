+++
id = "bug-712203"
kind = "bug"
title = "dev.sh up and status poll /api/health, which answers 401 under roko's default auth, so up reports a timeout and status a stopped server"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["tooling/dev-sh"]
created = 2026-10-19
updated = 2026-10-19
source = "roko-autostart round research, 2026-10-19 (roko 3b34aed92)"
anchors = ["dev.sh"]
parent = "spec-13c7ba"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
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

## Plan

- Poll `/health` for liveness.
- Where the richer `/api/health` body is shown, send the workspace's `serve.token` (or `ROKO_API_KEY`) as a Bearer token, the way `serve_client` does.

## Done when

A verify you write fails before the change and passes after it. For example, a static check that `up` polls `/health`, plus a scripted run against a serve with auth on, if that is cheap.

## Notes

Keep to `dev.sh`.

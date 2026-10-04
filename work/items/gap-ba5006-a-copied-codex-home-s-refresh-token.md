+++
id = "gap-ba5006"
kind = "gap"
title = "A copied Codex home's refresh-token rotation can stale the operator's real login with no warning"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-8 follow-up reports 2026-10-03 (PK30 gap-2ca903)"
discovered_from = "gap-2ca903"
anchors = ["benchmarks/viabilitybench/driver/run_codex.py"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_rotated_refresh_token_prints_a_login_warning' benchmarks/viabilitybench/driver/test_run_codex.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_codex.py -k test_rotated_refresh_token_prints_a_login_warning -q"
+++

## Problem

`benchmarks/viabilitybench/driver/run_codex.py`'s own module docstring (lines 11-16) already documents the risk
precisely: each session gets "a fresh `CODEX_HOME=$HOME/.codex` under the task's own HOME... holding only the
login: a private copy of the operator's `auth.json`... Codex refreshes an expired login inside the copy; a
refresh that rotates the refresh token can leave the operator's own login stale, which `codex login` then
renews." So running the `fd_codex` arm can, as an unintended side effect of a copied-home refresh, invalidate the
*operator's own, real* Codex CLI session outside the benchmark entirely — the operator then has to notice and
re-run `codex login` themselves, with no signal from the benchmark that this happened.

## Why it matters

Goal: proof (ViabilityBench operational hygiene). This is a real side effect on the operator's own account state,
not just inside the benchmark's sandbox — the comment already names the mechanism but nothing in the driver
detects or surfaces it when it actually occurs, so the operator's first sign of trouble is a *later*, unrelated
`codex exec` failing with a stale-login error, with no connection drawn back to a benchmark run.

## Where

- `benchmarks/viabilitybench/driver/run_codex.py` (module docstring lines 11-16; `_seed_codex_home`, ~line 243,
  and the canonical-home resolver, `~line 267`).

## Current state

Documented in a comment only; no detection or warning exists.

## Plan

1. Before seeding a session's private `CODEX_HOME` copy, record a fingerprint of the canonical `~/.codex`
   auth's refresh token (or whatever identifies it without storing the secret itself — a hash is enough, per the
   existing pattern at `run_codex.py:364`, "sha256 over a Codex home's paths and file contents; the login counts
   by name only, since it rotates").
2. After the session ends, compare: if the canonical home's login fingerprint changed in a way consistent with
   rotation, print a clear, actionable warning (not buried in a comment) telling the operator their own Codex
   login may now need `codex login` again.

## Done when

- A benchmark run that rotates the operator's refresh token prints a clear warning naming the cause, instead of
  leaving the operator to discover it from an unrelated later failure.
- The `[[verify]]` command passes.

## Notes

- Discovered during PK30's work (gap-2ca903, done). Distinct from the same docstring's "a same-uid agent can
  read that copy" risk (the keychain/credential-read class of finding, e.g. gap-3cfe4f for Claude Code's
  analogous wrapper) — this item is only about the rotation side effect on the operator's real account state.
- Out of scope per the report: planemit's missing `[learning] frozen = true` is already tracked as a precondition
  in `gap-394f28`'s own `hold` field ("frozen learning in planemit") — not re-filed here.

## Progress

- Confirmed the Plan's own step 1 ("fingerprint the canonical `~/.codex` auth... after the session ends, compare
  [it]") would never actually detect anything: `denied(ctx)` already sandboxes the operator's real Codex home
  out of the session's reach, and nothing else in `run_codex.py` ever writes to it, so its on-disk fingerprint
  cannot change because of anything the benchmark does. Implemented the mechanism that actually observes the
  rotation instead, in the same spirit: fingerprint the *session's own, sandboxed copy* right after
  `_seed_codex_home` seeds it, and again right after the session ends. A rotation happens inside that copy (the
  module docstring already said so), so a changed fingerprint there is the direct, positive evidence that Codex
  refreshed and the operator's real refresh token (the one that was copied from) is now stale server-side.
- Added `login_fingerprint(codex_home)`: a sha256 of the home's `auth.json`'s own `tokens` dict (not the whole
  file, so an unrelated field like `last_refresh` touching it does not create a false positive), `None` when the
  file is missing, unreadable or has no `tokens` dict -- deliberately never the token values themselves, per the
  item's own "never read or print the token itself."
- Wired it into `_run_confined`: fingerprint right after `build_invocation` returns (the copy was just seeded),
  fingerprint again right after `run_cli.run_session` returns, and `_warn_if_login_rotated` prints a
  `run_codex: ...` warning (the module's established `print(..., file=sys.stderr)` convention, matching
  `run_cli.py`/`vb.py`) naming the cause and telling the operator to re-run `codex login`, only when both
  fingerprints are known and differ -- silent when either is unavailable, since that is "unknown," not "proof it
  did not rotate," and silent when they match.
- Added `test_rotated_refresh_token_prints_a_login_warning`: seeds a real Codex home via `_seed_codex_home`
  (the `login` fixture's fake subscription auth), confirms no warning when unchanged, writes a rotated `tokens`
  dict (a new `refresh_token`) directly to simulate what Codex itself would do mid-session, confirms the warning
  fires and names neither the old nor the new token value nor the access token, then confirms deleting the file
  afterward is silent too (not a false "it rotated"). Verified the test is load-bearing by stubbing
  `_warn_if_login_rotated` to a no-op and re-running: fails on the missing warning, then passes restored.
- Checked the blast radius: nothing outside `driver/run_codex.py`/`driver/test_run_codex.py` imports either
  file, so that suite (7 passed, including the full fake-`codex` `vb run` integration test, unaffected) is the
  whole one.
- Verify: named `[[verify]]` command -> 1 passed. Full `driver/test_run_codex.py`: 7 passed.

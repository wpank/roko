+++
id = "gap-6e1381"
kind = "gap"
title = "ViabilityBench docs are stale: README status lines, the gap-a8a160 pointer, records' s01_run_dir and S08 decision 8"
status = "done"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "4f3a7aca9"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (20:01, the reports of wk-bench-report, wk-bench-rokoarm and wk-bench-secret on gap-b24517, gap-b7ab99 and gap-a8a160)"
anchors = ["benchmarks/viabilitybench/README.md", "benchmarks/viabilitybench/driver/records.py:117", "tmp/cybernetic-harness/specs/S08-benchmark-suite.md"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-b24517", "gap-b7ab99", "gap-a8a160", "gap-308373"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'Not built yet: the families F1 and F4' benchmarks/viabilitybench/README.md && ! grep -q 'Proving that it never reaches an agent is gap-a8a160' benchmarks/viabilitybench/README.md && ! grep -q '\"s01_run_dir\": None' benchmarks/viabilitybench/driver/records.py"

[closed]
at = 2026-09-29
commit = "4f3a7aca9"
by = "wk-bench-fix3"
evidence = "README: status block lists what exists and points at epic spec-567e52; layout shows the actual tree (analysis, ledger, proxy, the four arms, secret, verifier CI, families/plan_slice/); the secret bullet says what the driver-only file guarantees and what it does not (gap-308373); the Not-yet-built list is gone. records.py sets provenance.s01_run_dir from TaskOutcome.s01_run_dir (s01/<key> for the Roko arm, null for direct and CLI arms; tests in test_run_roko.py and test_run_cli.py). census.py CACHE_PARTS and F1 _bytecode_cache removed (astcheck handles caches, bug-993e7e). S08 §9 decision 8 edited in place to the driver-only secret file (gap-a8a160). Verify passes; full bench suite 305 passed, 4 skipped."
+++

## Problem

At BASE (4315add32), several ViabilityBench docs describe an older state:

- **`README.md:11-15`, "Status (2026-09-29)".** It says F1 and F4, verifier CI, analysis and reports, and the other arms are not built. Four of these have since merged: F1 (gap-4723ff), F4 (gap-9e7079), analysis and `vb report` (gap-b24517) and the Roko arm (gap-b7ab99). Verifier CI (gap-7ee7c2) and the Claude Code arm (gap-c4f364) are still open.
- **`README.md:84-85`.** "Proving that it never reaches an agent is gap-a8a160." gap-a8a160 is done (fce93aced). The line should say what the secret file now guarantees and what it doesn't (gap-308373).
- **`README.md:88-90`, "Not yet built".** It lists the Roko arm and `vb census`/`vb report`, which are built. Re-check the other entries (gap-33d54b's budget-line caps, gap-e003ec's proxy, the BLAKE3 `config_hash`) when fixing.
- **`driver/records.py:117`.** It writes `provenance.s01_run_dir = None` for every arm, including Roko arms, whose runner reads Roko's S01 run directory (`runs/*/attempts.jsonl`, run_roko.py:246-251). The field should point there for Roko arms, and stay null only for arms without S01 records.
- **S08 §9 decision 8** (`tmp/cybernetic-harness/specs/S08-benchmark-suite.md:483`). It still says `VB_SECRET` lives in `~/.roko/.env`. `secret.py` (:3-6) explains why that was dropped: roko loads that file into its environment, and agents inherit it.

## Why it matters

Pilot benchmark (epic spec-567e52): the README is the entry point for whoever runs the pilot. A reader following it would think the families and the report are missing, and would put the secret where agents can read it.

## Where

The anchors. S08 is untracked (`tmp/`), so its edit is checked by hand.

## Current state

The code is ahead of the docs. No behaviour depends on these lines, apart from `s01_run_dir`, which is data.

## Plan

1. Rewrite the README's status block and "Not yet built" list from `work/` (the epic's children), and fix the secret paragraph.
2. Set `s01_run_dir` from the runner's result (None for direct and CLI arms), and add a test for the Roko arm.
3. Update S08 §9 decision 8 to the driver-only secret file, citing gap-a8a160.

## Done when

- [ ] The README and S08 match BASE, and Roko records point at their S01 run directory.
- [ ] The `[[verify]]` command passes.

## Notes

- Status claims belong in `work/`. The README should say only what exists and point at the epic for the rest.
- 2026-09-29 (wk-bench-ci, wk-bench-proxy): also stale at ad391f99a. `README.md:13-14` still lists the verifier CI (gap-7ee7c2, done) as not built, and `README.md:88` lists the metering proxy (gap-e003ec, done) under "Not yet built". `driver/census.py`'s `CACHE_PARTS` (:61, :123) and F1's `_bytecode_cache` (`families/f1_pyconv/gaming.py:48`, :70) now repeat astcheck's own cache filtering (bug-993e7e) and can go.

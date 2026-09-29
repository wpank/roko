+++
id = "gap-b7ab99"
kind = "gap"
title = "ViabilityBench Roko arm with the model pinned and checked on every attempt (S08.T11)"
status = "open"
triage = "verified"
last_verified = 2026-09-29
severity = "p1"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W10-benchmarks-proof.md (recs 5-6); specs/S08-benchmark-suite.md (§4.9, §6 T11; checklist S08.T11)"
anchors = ["benchmarks/viabilitybench/driver/planemit.py", "benchmarks/viabilitybench/driver/run_roko.py", "benchmarks/viabilitybench/driver/test_run_roko.py", "benchmarks/viabilitybench/arms/roko_fixed.toml"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-28ebea", "bug-7d7200"], blocks = [], related = ["bug-35379d", "bug-1410e8", "gap-644040", "gap-e003ec"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_model_mismatch_marks_attempt_infra_error' benchmarks/viabilitybench/driver/test_run_roko.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko.py -k test_model_mismatch_marks_attempt_infra_error -q"

[[verify]]
command = "grep -qw 'def test_emitted_plan_has_explicit_rungs_and_no_hidden_checks' benchmarks/viabilitybench/driver/test_run_roko.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko.py -k test_emitted_plan_has_explicit_rungs_and_no_hidden_checks -q"
+++

## Problem

The arm this pilot exists for, gpt-oss-120b inside Roko (`roko_fixed`), does not exist. Run naively, it would be
distorted by known defects:

- provider failover silently runs a different model (bug-35379d);
- agents and verify commands inherit roko's whole environment, provider keys included (bug-7d7200);
- outside Rust projects, `roko run` falls back to `cargo check` gates (bug-1410e8);
- Roko prices gpt-oss-120b from `roko.toml`'s wrong rates (S08 D9);
- learned state carries over from one seed to the next (gap-644040).

## Why it matters

This is the arm that tests the thesis. One substituted model, or one hidden check leaking into the prompt,
invalidates the whole cell.

## Where

All new, and the paths follow D4: `benchmarks/viabilitybench/driver/planemit.py`, `run_roko.py`,
`test_run_roko.py` and `arms/roko_fixed.toml`. The arm drives the built binary:
`roko --repo <ws> --model <m> plan validate --strict --dag`, then `plan run`.

## Current state

Checked at `41c7ffbd6`: nothing of the arm exists.

- bug-7d7200's fix, `dc99a9e81`, is on the portal session's branch `fix/hermetic-child-env`, not merged.
- Pinning `--model` turns failover into an error (bug-35379d).
- Episodes record the model that ran and its tokens (`crates/roko-cli/src/runtime_feedback/episodes.rs`).
- S01's `attempt_key` records don't exist yet (epic E4), so this is S08's degraded mode.

## Plan

1. **Emit a one-task plan per (task, seed)** (`planemit.py`, B §2.3): `meta.total = 1`; an implementer with
   `files`, one visible `[[task.verify]]` and `max_retries = 2`; `max_parallel = 1`; an explicit
   `skip_enrichment`. No hidden check goes into `tasks.toml` or `roko.toml`, because agents see verify commands.
2. **Write the workspace `roko.toml`:** the Cerebras provider, the pinned model, and explicit `[[gates.rungs]]`
   that run only the visible check.
3. **Run it** (`run_roko.py`) with `plan run`, never `roko run`. Give every run a fresh `HOME` and a fresh
   workspace `.roko/`, so no `~/.roko/.env` loads and no learned state carries over, and pass only
   `CEREBRAS_API_KEY` and a git identity. Check that roko needs nothing else from `~/.roko`.
4. **Check every attempt** in `.roko/episodes.jsonl`. A model other than the requested one makes the run
   `infra_error` (excluded and counted). Re-price tokens from the snapshot, never from Roko's USD. If the proxy
   (gap-e003ec) runs, route through it and flag attempts that sent no traffic.
5. **Record** `harness_sha`, `dirty` and `config_hash` for every run.

## Done when

- [ ] The emitted plan has explicit rungs and no hidden checks, and passes `roko plan validate --strict --dag` once
      by hand.
- [ ] A fixture episode naming a different model marks the attempt `infra_error`.
- [ ] Both `[[verify]]` commands pass.

## Notes

- **Waits for** bug-7d7200 to merge. Treat the first 20 runs as a shakedown (W10).
- **Later:** joining records on `attempt_key`, after E4.
- **No Rust changes** and no hot files.

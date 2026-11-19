+++
id = "gap-d0c5d9"
kind = "gap"
title = "roko's crash recovery has no repo test that kills a real run: the kill matrix that found bug-b7f6a8 lives in tmp/, so no CI run checks a resume after a power cut"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "M"
subsystem = ["tools", "roko-cli/graph_checkpoint", "roko-serve/plans"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 roko-recovery kill matrix (tmp/hermes-roko-v7/roko-recovery), bug-b7f6a8's close, the lead's ask"
discovered_from = "agent:roko-recovery (round v7)"
anchors = ["crates/roko-cli/src/graph_checkpoint.rs::prepare_graph_checkpoint_for_run", "crates/roko-serve/src/routes/plans/run_control.rs::start_plan_run_with"]
links = { depends_on = [], blocks = [], related = ["bug-b7f6a8", "bug-4026d8", "bug-02e0d3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -x tools/crash_matrix/run_fast.sh && cargo build -p roko-cli --bin roko && tools/crash_matrix/run_fast.sh target/debug/roko"
+++

## Problem

roko's crash recovery has no regression test that kills a real run. The kill matrix that found bug-b7f6a8 lives in
`tmp/hermes-roko-v7/roko-recovery/` (about 1.5k lines of Python), and `tmp/` is local, so nothing in the repo or
CI runs it.
- **What the matrix does, at $0:**
  - it runs a two-task plan against a fake OpenAI-compatible model;
  - it kills roko and everything under it at 11 points: the model stream, a bash tool call, the hand-off check, the
    gate's verify, between tries and tasks, after a merge or commit, and delivery;
  - it does this on the CLI path and the serve path;
  - it snapshots the state left (an APFS clone), and replays each way back on a fresh copy: `--resume-plan`,
    `roko resume`, a plain re-run, `--force-resume`, `--fresh`, and MCP or REST through a new serve.
- `harness/check.py` turns the results into pass/fail expectations E1–E6:
  - E1: resume works;
  - E2: passed tasks are kept;
  - E3: an honest serve resume;
  - E4: a dead run reads as dead;
  - E5: the dead run's spend is carried;
  - E6: no silent discard.
- bug-b7f6a8 put E1, E2, E3 and E5 in its evidence rather than its `[[verify]]`, because `tmp/` is local.

## Why it matters

- **Goal core: "plan runs work reliably".** A machine restart or a crash is the normal way a long run dies (the
  2026-10-26 reboot).
- The failure modes the matrix found are each only one change away from coming back:
  - a resume that fails closed;
  - a serve resume that starts over and says it resumed;
  - spend that escapes the cap.
- The unit tests pin each piece. Only a kill of a real process checks the whole path.

## Where

- Now: `tmp/hermes-roko-v7/roko-recovery/`:
  - `harness/rr.py` (fences, fake model, serve, power cut);
  - `harness/matrix.py`, `harness/scenarios.py`, `harness/state.py`, `harness/check.py`;
  - `run_matrix.sh` and `README.md`.
- What it exercises:
  - `crates/roko-cli/src/graph_checkpoint.rs::prepare_graph_checkpoint_for_run`;
  - `crates/roko-serve/src/routes/plans/run_control.rs::start_plan_run_with`.

## Plan

1. Move the harness to `tools/crash_matrix/`.
   - Keep its fences: `env -i` with an explicit PATH, HOME and USER; a scratch HOME; fake providers only; and ports
     taken from a range that is checked free first.
   - Drop the round-specific paths: the binary comes from an argument or `ROKO_BIN`.
2. Add a fast subset that CI could run in a few minutes:
   - kill points a-stream-t2, b-tool, c-verify and d-after-commit;
   - CLI and serve;
   - the `cli-resume` and `serve-mcp-resume` ways back;
   - `check.py --only E1,E2,E3,E5`.
   - The APFS clone falls back to a plain copy off macOS.
3. Point bug-b7f6a8's evidence command and this item's `[[verify]]` at it.

## Done when

`tools/crash_matrix/run_fast.sh target/debug/roko` runs the subset at $0 and exits 0 on a fixed binary. On
4d22b94e8, the binary that found bug-b7f6a8, it exits 1 and names the failing kill points.

## Notes

- The full matrix (`run_matrix.sh`) takes about 30 minutes. Its v2 results on 4d22b94e8 are in
  `tmp/hermes-roko-v7/roko-recovery/runs/roko-main-4d22b94e8` (`summary.md`, `expectations.md`).
- Related:
  - bug-b7f6a8: the fix the subset guards;
  - bug-4026d8: fsync, which a kill alone does not exercise;
  - bug-02e0d3: orphan children, the `--orphans` variant.

+++
id = "gap-eb1aa3"
kind = "gap"
title = "PK23 ViabilityBench proof: Family F6 ts-result: the TypeScript transfer probe (+2 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
rank = 23
size = "L"
subsystem = ["benchmarks/viabilitybench/families/f6_tsresult"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK23"
anchors = [".github/workflows/viabilitybench-ci.yml"]
lane = "bench"
parent = "spec-fef7c5"
links = { depends_on = ["gap-46fd19"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/families/f6_tsresult/hidden.py && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/ci/verify_verifiers.py --families f6 --levels 1-5 --seeds 2"

[[verify]]
command = "python3 -c \"import re,sys; t=open('.github/workflows/viabilitybench-ci.yml').read(); s=set(','.join(re.findall(r'--families ([a-z0-9,]+)', t)).split(',')); sys.exit(0 if {'f1','f2','f3','f4','f5','f6','f7','f8'} <= s else 1)\""

[[verify]]
command = "grep -qw 'def test_p1_streams_are_reproducible_from_their_seeds' benchmarks/viabilitybench/streams/test_streams.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/streams/test_streams.py -k test_p1_streams_are_reproducible_from_their_seeds -q"
+++

## Problem

This package delivers 3 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK23, slice 33xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3326 | M | p3 | Family F6 ts-result: the TypeScript transfer probe | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3326-family-f6-tsresult.md` |
| 2 | 3327 | S | p2 | Verifier CI runs all eight families | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3327-verifier-ci-runs-all-eight-families.md` |
| 3 | 3328 | M | p2 | The P1 streams p1_core (120) and p1_h3 (48), reproducible from their seeds | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3328-p1-streams-p1-core-and-p1-h3.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `.github/workflows/viabilitybench-ci.yml`, `benchmarks/viabilitybench/families/f6_tsresult/`, `benchmarks/viabilitybench/streams/compile.py`, `benchmarks/viabilitybench/streams/p1_core.toml`, `benchmarks/viabilitybench/streams/p1_h3.toml`, `benchmarks/viabilitybench/streams/test_streams.py`.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK22 (gap-46fd19).
- Suggested model: sonnet.
- 2026-10-02 (filer-grpD, backlog wave reports, PK96 gap-a6dab7): task 3328's P1 stream work should name the 30-task
  pass^5 subset's stream id exactly `p1_pass5` — `benchmarks/viabilitybench/analysis/figlib.py:102-103` already
  hardcodes `H3_STREAM = "p1_h3"  # task 3328's stream ids` and
  `PASS5_STREAM = "p1_pass5"  # the 30-task pass^5 subset's stream`, and `fig_f4_passk.py` reads
  `stream.id == "p1_pass5"` (and `CORE_STREAM`/`"p1_core"`). Confirmed at HEAD: this task's own file
  (`tmp/backlog/2026-10-02-complete-and-wire/3328-p1-streams-p1-core-and-p1-h3.md`) mentions a "pass^5 subset"
  inside `p1_core` but never commits to the literal id `p1_pass5` — whoever implements it should use that exact
  string so the already-written figure/table scripts resolve without edits.

## Progress

- 3326: implemented at `4b3aa137a`. New family `f6_tsresult` (gen.py, hidden.py, gaming.py,
  reference/solutions.py, ladder.toml, test_f6.py): `reserveUnits` throws on an over-reservation instead of
  returning `Result`'s `err(...)`; the truth suite lints every `.ts` under `src/domain/` for `throw`, replays
  `reserveUnits` and `explain()` over HMAC-seeded hidden cases through a Node probe, and runs `tsc --strict` when
  present. Latent v2 moves `Result` to a shipped `@lib/result` node_modules package (plain JS + `.d.ts`: Node
  refuses to type-strip `.ts` under `node_modules`). Verified locally (Node 22.22.2, both with and without `tsc`
  on PATH): `ci/verify_verifiers.py --families f6 --levels 1-5 --seeds 10` reports 100/100 cells green (reference
  VS=1, stub fails visibly, gaming passes visibly with VS=0, two runs identical, 0 leaks), both latents;
  `pytest families/f6_tsresult/test_f6.py` passes (6/6).
- 3327: implemented at `824ee8525`. `.github/workflows/viabilitybench-ci.yml` split into three jobs: the existing
  Python job now also runs f2, f3, f5 and f8 (f1, f4, pl unchanged); a new `verifier-ci-rust` job gives F7 its own
  `dtolnay/rust-toolchain@1.96.1`, a dedicated `CARGO_TARGET_DIR` cached and keyed on
  `families/f7_rustiter/gen.py`'s content, and `concurrency: group: viabilitybench-f7-${{ github.ref }}` so two
  runs on one ref never race the cache; a new `verifier-ci-ts` job gives F6 a pinned Node 22
  (`actions/setup-node@v4`) and TypeScript 5.7.3. Verified: the item's `[[verify]]` command (the regex union over
  every `--families` flag in the file) passes, and `python3 -m yaml` parses the file cleanly.
- 3328: implemented at `97fc585c6`. New `streams/compile.py` draws `p1_core.toml` (120: F1-F5, F7 x levels 1-5 x 4
  instances), `p1_h3.toml` (48: 2 of the 4 per family at levels 1-4) and `p1_pass5.toml` (30: 1 per family and
  level, stream id `p1_pass5` per this item's note above) from one recorded `--seed` (default 1, recorded in each
  file's header) via `common.hmac_seed`'s surface stream; H3 and pass^5 are drawn FROM `p1_core`'s own instance
  list, so both are true subsets with equal spec hashes. Verified: the item's `[[verify]]` command passes;
  `pytest streams/test_streams.py` passes (9/9), including loading all three through the real `vb.load_stream`,
  `Stream.order` determinism, and one `vb materialize`/`materialize.materialize` round trip giving the same
  pristine commit and tree twice.

All three tasks' own verify commands passed locally (Python/Node only; no cargo run, per BUILD-RULES.md). Cargo
verification of the touched Rust-adjacent surface (none directly; F7's job only adds CI config) is deferred to
the coordinator's batched gate.

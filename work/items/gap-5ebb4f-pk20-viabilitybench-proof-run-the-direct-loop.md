+++
id = "gap-5ebb4f"
kind = "gap"
title = "PK20 ViabilityBench proof: Run the direct loop's shell commands in a loopback-only network sandbox on macOS (+7 more)"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
rank = 20
size = "L"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK20"
anchors = ["benchmarks/viabilitybench/driver/agent_env.py", "benchmarks/viabilitybench/driver/mini_loop.py", "benchmarks/viabilitybench/driver/planemit.py", "benchmarks/viabilitybench/driver/records.py", "benchmarks/viabilitybench/driver/run_cli.py", "benchmarks/viabilitybench/driver/run_roko.py", "benchmarks/viabilitybench/driver/test_run_cli.py", "benchmarks/viabilitybench/driver/vb.py", "benchmarks/viabilitybench/families/common/sandbox.py"]
lane = "bench"
parent = "spec-fef7c5"
links = { depends_on = [], blocks = [], related = ["gap-0bd49a", "gap-327242", "gap-c33709"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_direct_loop_shell_cannot_reach_the_network' benchmarks/viabilitybench/driver/test_sandbox_net.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_sandbox_net.py -k test_direct_loop_shell_cannot_reach_the_network -q"

[[verify]]
command = "grep -qw 'def test_roko_arm_process_cannot_reach_the_network' benchmarks/viabilitybench/driver/test_run_roko_sandbox.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko_sandbox.py -k test_roko_arm_process_cannot_reach_the_network -q"

[[verify]]
command = "grep -qw 'def test_cli_arm_shell_cannot_reach_the_network' benchmarks/viabilitybench/driver/test_run_cli.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_cli.py -k test_cli_arm_shell_cannot_reach_the_network -q"

[[verify]]
command = "grep -qw 'def test_campaign_refuses_a_block_over_its_line_cap' benchmarks/viabilitybench/driver/test_campaign.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_campaign.py -k test_campaign_refuses_a_block_over_its_line_cap -q"

[[verify]]
command = "grep -qw 'def test_pilot_a_manifest_rehearses_offline' benchmarks/viabilitybench/experiments/test_pilot_a.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/experiments/test_pilot_a.py -k test_pilot_a_manifest_rehearses_offline -q"

[[verify]]
command = "grep -qw 'def test_pilot_b_manifest_rehearses_offline' benchmarks/viabilitybench/experiments/test_pilot_b.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/experiments/test_pilot_b.py -k test_pilot_b_manifest_rehearses_offline -q"

[[verify]]
command = "grep -qw 'def test_g0_page_reports_every_check_with_its_value' benchmarks/viabilitybench/analysis/test_gates.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_gates.py -k test_g0_page_reports_every_check_with_its_value -q"

[[verify]]
command = "grep -qw 'def test_ladder_mode_emits_rungs_without_fallbacks' benchmarks/viabilitybench/driver/test_planemit.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_planemit.py -k test_ladder_mode_emits_rungs_without_fallbacks -q"
+++

## Problem

This package delivers 8 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK20, slice 33xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3303 | M | p1 | Run the direct loop's shell commands in a loopback-only network sandbox on macOS | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3303-sandbox-the-direct-loop-shell-on-macos.md` |
| 2 | 3304 | S | p1 | Run the Roko arm's roko process inside the loopback-only network sandbox | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3304-sandbox-the-roko-arm-process.md` |
| 3 | 3305 | M | p1 | Keep Claude Code's shell commands off the network while the CLI itself reaches Anthropic | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3305-keep-claude-code-shell-commands-off-the-network.md` |
| 4 | 3306 | M | p1 | vb campaign: validate an experiment manifest, estimate it and run its blocks in order | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3306-vb-campaign-experiment-manifests.md` |
| 5 | 3307 | S | p1 | Pilot A's manifest and an offline rehearsal of it | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3307-pilot-a-manifest-and-offline-rehearsal.md` |
| 6 | 3308 | S | p1 | Pilot B's manifest, seeds 2-3 included, and an offline rehearsal of it | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3308-pilot-b-manifest-and-offline-rehearsal.md` |
| 7 | 3309 | M | p1 | The G0 go/no-go page: every check with its value and its run ids | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3309-g0-go-no-go-page.md` |
| 8 | 3310 | M | p1 | Planemit ladder mode: emit the cheap-pool ladder instead of one pinned model | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3310-planemit-ladder-mode.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/analysis/gates.py`, `benchmarks/viabilitybench/analysis/test_gates.py`, `benchmarks/viabilitybench/driver/agent_env.py`, `benchmarks/viabilitybench/driver/campaign.py`, `benchmarks/viabilitybench/driver/egress.py`, `benchmarks/viabilitybench/driver/mini_loop.py`, `benchmarks/viabilitybench/driver/planemit.py`, `benchmarks/viabilitybench/driver/records.py`, `benchmarks/viabilitybench/driver/run_cli.py`, `benchmarks/viabilitybench/driver/run_roko.py`, `benchmarks/viabilitybench/driver/test_campaign.py`, `benchmarks/viabilitybench/driver/test_planemit.py`, `benchmarks/viabilitybench/driver/test_run_cli.py`, `benchmarks/viabilitybench/driver/test_run_roko_sandbox.py`, `benchmarks/viabilitybench/driver/test_sandbox_net.py`, `benchmarks/viabilitybench/driver/vb.py`, `benchmarks/viabilitybench/experiments/pilot_a.toml`, `benchmarks/viabilitybench/experiments/pilot_b.toml`, `benchmarks/viabilitybench/experiments/provider_fault.toml`, `benchmarks/viabilitybench/experiments/test_pilot_a.py`, `benchmarks/viabilitybench/experiments/test_pilot_b.py`, `benchmarks/viabilitybench/families/common/sandbox.py`, `benchmarks/viabilitybench/schema/experiment.schema.json`, `benchmarks/viabilitybench/streams/pilot_fd_api.toml`.

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

- Waits on: nothing.
- Existing work items this package covers or touches: gap-0bd49a, gap-327242, gap-c33709. When its tasks are done, close those whose verify then passes.
- Suggested model: sonnet.

## Progress

Worker claude-agent on `work/gap-5ebb4f`, 2026-10-02. All Python, no cargo; every `[[verify]]` above passes in the
worktree, and the whole benchmark suite passes (409, with `VB_TEST_ROKO_BIN` set to the prebuilt roko a43288b5f).

- 3303: implemented at a38dfcdc0 (`sandbox.command(network=...)`; the direct loop runs with no network and is denied
  the secret file, the key file and the run's private directories; `provenance.network_policy`)
- 3304: implemented at 6eca7b814 (every roko process under `loopback:<endpoint port>` plus Unix sockets in the
  workspace; real Roko's `bash` tool checked against the sandbox)
- 3305: implemented at 433d508fb (option (1): `driver/egress.py` CONNECT allowlist per session, `api.anthropic.com:443`
  by default via `[cli] egress_allow`; refused requests in the record)
- 3306: implemented at 47b1cb29a (`vb campaign`, `vb.experiment/1`); its budget check refined at a4e28d7b0
- 3307: implemented at 3c979eaf6 (`experiments/pilot_a.toml`, `streams/pilot_fd_api.toml`, 65-run offline rehearsal)
- 3308: implemented at a4e28d7b0 (`experiments/pilot_b.toml`, `experiments/provider_fault.toml`; seeds 2-3 refused under
  BL0 $10 after Pilot A's spend and fitting under $14)
- 3309: implemented at 3d0b45d35 (`analysis/gates.py G0`, `go-no-go.md` and `g0.json`)
- 3310: implemented at 5f1d3bce6 (planemit ladder mode; pinned output byte-identical)

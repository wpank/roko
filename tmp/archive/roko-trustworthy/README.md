# Roko Trustworthy Runner

This directory is an overnight-style Codex runner for making Roko a trustworthy self-hosting executor before using Roko agents to implement the rest of the architecture backlog.

The runner is intentionally self-contained. Each batch prompt assumes the Codex agent has no prior chat context. The script composes:

- `context-pack/*.md`
- prior attempt failure context, when present
- the per-batch prompt from `prompts/*.prompt.md`
- the runner contract and verification commands

Default model settings:

- model: `gpt-5.5`
- reasoning: `high`
- timeout: `7200` seconds per batch
- retries: `2` attempts per batch

## Quick Start

```bash
bash tmp/roko-trustworthy/run-roko-trustworthy.sh --list
bash tmp/roko-trustworthy/run-roko-trustworthy.sh --dry-run --only RT00
bash tmp/roko-trustworthy/run-roko-trustworthy.sh --only RT00
bash tmp/roko-trustworthy/run-roko-trustworthy.sh --group kernel
bash tmp/roko-trustworthy/run-roko-trustworthy.sh --continue last
```

The runner creates isolated git worktrees under `.roko/worktrees/` and logs under `tmp/roko-trustworthy/logs/run-*`.

To run the entire queue:

```bash
bash tmp/roko-trustworthy/run-roko-trustworthy.sh
```

For a long unattended run with aggressive Rust artifact cleanup:

```bash
bash tmp/roko-trustworthy/run-roko-trustworthy.sh \
  --cleanup-every 1 \
  --cleanup-stale-days 1
```

The runner sets `CARGO_TARGET_DIR` to per-batch temp directories under `${TMPDIR:-/tmp}/roko-trustworthy-targets`, removes each batch's cargo targets after success/final failure, removes failed-attempt cargo targets before retry by default, removes `target/` and `.cargo-target/` from the runner worktree periodically, and prunes stale runner target directories older than `--cleanup-stale-days`.

Use `--keep-failed-targets` only when you need to inspect failed build artifacts.

## Roadmap Shape

The ordering follows one bootstrap objective:

Make Roko a trustworthy self-hosting executor first.

That means dashboard and product surfaces are deliberately late. The first batches establish acceptance gates, structured review output, compile-failure recovery, context scoping, configurable roles/prompts, adaptive telemetry, and replanning. Only after those are in place should Roko agents be trusted to drive docs parity, dashboard surfaces, chain/economy work, or broad architecture implementation.

## Groups

- `gate`: done gate and parity ledger contract
- `kernel`: self-hosting execution reliability
- `policy`: roles, prompts, context, and learning policy
- `runtime`: durable runtime and control plane
- `selfhost`: end-to-end trustworthy self-hosting loop
- `core`: handoff queue for the rest of the architecture implementation
- `parity`: docs parity enforcement after gates exist
- `dashboard`: dashboard/product surfaces after backend projections stabilize
- `advanced`: chain, economy, and advanced surfaces

## Run Results

For each batch the runner records:

- prompt snapshot: `logs/<run>/prompts/<batch>.prompt.md`
- Codex transcript/log: `logs/<run>/<batch>.log`
- final Codex message: `logs/<run>/<batch>.last.txt`
- failure summary: `logs/<run>/<batch>.failure.txt`
- batch status: `logs/<run>/<batch>.result`
- status timeline: `logs/<run>/status.tsv`

Successful batches are committed in the worktree branch as:

```text
roko-trustworthy(<batch>): <title>
```

## Operating Notes

- Run `RT00` first. It defines the acceptance contract used by the rest of the runner.
- Use `--max-batches N` for shorter supervised runs.
- Use `--group kernel` before `--group policy`; the policy layer needs trustworthy observations.
- Use `--continue last` after interruption. The runner preserves the currently dirty batch worktree when resuming an interrupted batch.
- Use `--verify-only --continue last --only RTxx` to re-run a gate against an existing run worktree.
- Use `--cleanup-every 1` for the lowest disk pressure during long Rust-heavy runs.

# Dogfood: Plan Runner Gaps — 2026-04-26

Issues found running `roko plan run .roko/plans/unified-migration-phase0`.

## Enrichment Phase Issues

### 1. Enrichment too aggressive for pre-authored plans
The runner auto-enriches with 13 steps (prd, briefs, tasks, decompose, research, dependencies,
fixtures, integration, verify, reviews, tests, invariants, scribe) even when tasks.toml is
already fully authored. This wastes ~$0.30 and 6+ minutes on enrichment that mostly fails or
produces empty artifacts.

**Fix needed**: Skip enrichment when tasks.toml already has complete task definitions, or add
a `skip_enrichment = true` flag to the plan metadata.

### 2. LLM output wrapped in markdown fences
The `verify` enrichment step failed because the LLM returned TOML wrapped in ````toml` code
fences. The TOML parser can't handle this.

**Fix needed**: Strip markdown fences from LLM output before TOML parsing (already a known
pattern — check if strip_code_fences exists).

### 3. Enrichment timeouts (120s too short)
Multiple enrichment steps timed out at 120s. The agents are reading many files (16+ Read calls)
and the model takes time to synthesize. 120s is too short for enrichment of a 10-task plan.

**Fix needed**: Increase enrichment timeout or make it configurable via `[executor]` config.

### 4. Exit signal crashes
Two enrichment steps failed with "exit signal: claude failed" — likely the spawned claude
process was killed (OOM? signal?). No diagnostic info captured.

**Fix needed**: Capture stderr/signal info from failed agent spawns.

## State Persistence Issues

### 5. No executor.json written during run
The executor snapshot (`.roko/state/executor.json`) is never created during this run. State
is only in-memory. If the process crashes, all progress is lost.

**Fix needed**: Write executor snapshot after each phase transition (enrich → implement → gate).

### 6. Episodes not written during enrichment
Enrichment spawns 6+ claude agents but writes 0 episodes. Token usage is lost.

**Fix needed**: Record episodes for enrichment agent calls, not just implementation agents.

### 7. Efficiency events not emitted
The efficiency JSONL file doesn't exist. No per-turn cost tracking during enrichment.

## TUI Integration Issues

### 8. Plan runner agents invisible to TUI
The TUI shows "no parallel agents" even though the plan runner is actively spawning agents.
The plan runner and TUI/serve run in separate processes with no shared state beyond files.

**Fix needed**: Plan runner should publish progress to StateHub (via HTTP POST to serve, or
shared file). The `state_hub.publish()` calls only work within the serve process.

### 9. No worktree created for plan
The plan runner uses the main working directory, not a worktree. If the enrichment agent
writes files (it does — Write tool was called), those changes go to the main repo.

**Fix needed**: `--use-worktrees` flag or `use_worktrees = true` in executor config.

## Config Issues

### 10. Config v1 warnings spam
Every agent spawn logs "roko.toml uses config version 1 (no [providers] section)". This
clutters the output. Should be logged once, not per-spawn.

**Fix needed**: Log the warning once at startup, not on every config load.

### 11. No codex backend support
The plan runner only uses `claude_cli` backend. There's no way to route tasks to codex
(which the user wants for gpt-5.4 with high reasoning).

**Fix needed**: Add codex provider to the provider adapter, or allow specifying backend
per-task in tasks.toml.

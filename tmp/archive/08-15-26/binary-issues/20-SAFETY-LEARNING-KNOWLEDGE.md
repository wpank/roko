# 20 — Safety, Learning, and Knowledge Subsystem Audit

**Status**: open (medium)
**Scope**: `crates/roko-agent/src/safety/`, `crates/roko-learn/`, `crates/roko-gate/`, `crates/roko-neuro/`, `crates/roko-dreams/`

## What This Document Covers

The subsystems that make roko smarter and safer over time: safety contracts, cascade router
learning, knowledge store, episode logging, adaptive gate thresholds, playbooks, and cost
tracking. These are the "built but how well do they actually work?" systems.

---

## 1. Safety Contracts

### What works well
- **Fail-closed by default**: Missing contract YAML falls back to `restricted()` (deny-all),
  not `permissive()`. Verified at `contract.rs:157`.
- **Eight role-specific contracts**: implementer, reviewer, researcher, architect, auditor,
  scribe, auto-fixer, strategist — each with appropriate tool restrictions.
- **BashPolicy**: Blocks `rm -rf /`, `sudo`, `curl|sh`, etc. (`safety/bash.rs:92`)
- **GitPolicy**: Blocks force-push to main (`safety/git.rs:73`)
- **PathPolicy**: Prevents path escapes outside worktree

### Issues

**SAF1. Post-dispatch safety violations are warnings, not blocks** (`safety/mod.rs:696,714,745`)

Post-dispatch checks for secret leaks (`ViolationType::SecretLeak`) and forbidden file
writes return `ViolationSeverity::Warn`. The agent output is scrubbed, but **changes to
the worktree are committed and retained**. A reviewer role that manages to modify files
(e.g., via bash) only gets a warning — the damage is done.

**SAF2. Network escape via Python/Node subprocess** (implementer contract)

The `implementer` contract forbids `["network", "fetch"]` tools but does NOT have
`NoNetworkAccess` invariant. The bash policy checks for `curl`, `wget`, `http://`,
`https://` — but an implementer could use `python3 -c "import urllib..."` to reach the
network. Pattern matching on shell commands is inherently incomplete.

**SAF3. `MaxCostPerTurn` not enforced** (`contract.rs:432-444`)

The governance rule exists but has a TODO: "enforce cumulative per-turn spend once
tool-cost accounting is threaded into ToolContext." Currently checks estimated cost
from tool call arguments, not actual cumulative spend. The `estimated_cost_usd` field
must be set by the caller, which rarely happens.

**SAF4. `permissive()` constructor still exists** (`contract.rs:78`)

Retained "for tests and adapter shims" but its mere existence is a footgun. Any code
path that calls `AgentContract::permissive()` bypasses all safety.

**SAF5. CLAUDE.md is outdated** — still says "falls back to permissive default when YAML
missing." The code was fixed to use restricted fallback. The docs lie.

---

## 2. Cascade Router Learning

### How it works (`cascade_router.rs`)

Three-stage cascade:
| Stage | Observations | Strategy |
|---|---|---|
| Static | < 50 | Hardcoded role-to-model table |
| Confidence | 50-200 | Empirical pass rates + confidence intervals |
| UCB1 | > 200 | LinUCB contextual bandit |

`observe()` feeds rewards. `FeedbackService` calls `router.observe()` on every `ModelCall`
event. Routing decisions are persisted to `.roko/learn/cascade-router.json`.

### Issues

**LR1. LinUCB bandit weights are NOT persisted** (`cascade_router.rs:1551`)

`save()` writes observation counts and confidence stats. The actual bandit weights
(the learned W matrix) are NOT serialized. When reloading, total_observations is
restored (so the stage might be UCB1), but the learned weights start fresh. Stage 3
learning is lost on every restart.

**LR2. Manual overrides don't feed proper context** (`cascade_router.rs:134-143`)

`record_override_outcome()` uses `RoutingContext::default()` which doesn't capture
actual task context. Manual overrides contribute generic context to the bandit,
reducing learning quality.

**LR3. Stage transitions not visible to user** (`learn.rs:124-181`)

`roko learn route` shows per-model observation counts and mean reward, but does NOT
show the current stage (Static/Confidence/UCB1) or when transitions happened. The user
cannot see whether the router has "graduated" to real learning.

---

## 3. Knowledge Store

### What works
- Disk-backed under `.roko/neuro/`
- Queried during plan runs for anti-patterns and playbook matching
- Full management CLI: `roko knowledge query/stats/gc/backup/restore/sync`
- Dream consolidation writes to durable store

### Issues

**KS1. Not queried during chat** (only plan runs)

The knowledge store enriches plan execution but is completely absent from the chat REPL.
If a user asks "what did we learn about gate failures?" in chat, the model has no access
to the knowledge store.

**KS2. Dream triggers write to JSONL but no background worker consumes them**
(`runtime_feedback/dreams.rs:20`)

The comment says "a separate worker consumes those." There is no standalone cron/daemon
that reads `dream_triggers.jsonl`. Dreams only run when:
- `auto_dream` is enabled in config AND the orchestrator event loop processes them
- `roko knowledge dream run` is called manually

The "cold substrate archival" (CLAUDE.md item #14) is also uninstantiated at runtime.

---

## 4. Episode Logging

### What's logged to `.roko/episodes.jsonl`

Per-episode: agent_id, task_id, model, success, failure_reason, gate_verdicts (per-gate),
usage (tokens, cost, wall_ms), HDC fingerprint, emotional tags, headline flag.

### Issues

**EP1. Compaction is never automatically triggered** (`episode_logger.rs:1086`)

`compact()` exists with retention policy (200 episodes, 90 days). But it must be called
explicitly — no evidence of automatic invocation in the CLI or orchestrator. The JSONL
file grows until the user manually intervenes. No CLI command exposes compaction either.

**EP2. Episode logger failure continues silently** (`run.rs:1063`)

```rust
eprintln!("[run] episode logger failed: {err}");
```

The run continues. The user loses episode data without understanding the consequences.

---

## 5. Adaptive Gate Thresholds

### How they work (`adaptive_threshold.rs`)

Per-rung EMA with `alpha = 0.1`. Each observation updates pass rate, consecutive passes,
CUSUM change detection, and SPC detector ensemble (CUSUM + EWMA + BOCPD). Additional:
Hotelling T² joint anomaly detection, neuro hints, temperament adjustments.

### Issues

**GT1. `roko learn tune gates` output is raw JSON** (`learn.rs:46-56`)

Prints `serde_json::to_string_pretty` of the entire `AdaptiveThresholds` struct —
including `cusum_high`, `cusum_low`, `spc_detectors`. Implementation details meaningless
to users. No human-readable summary like "compile pass rate: 92%, suggested retries: 1."

**GT2. `roko learn tune gates` never applies changes** (`learn.rs:84`)

Has a `--dry-run` flag that prints "(dry-run: no changes applied)" — but the non-dry-run
path ALSO does nothing. The command is read-only regardless of the flag. Misleading.

**GT3. No threshold reset CLI**

To reset corrupted thresholds, the user must manually delete
`.roko/learn/gate-thresholds.json`. No `roko learn tune gates --reset` command.

**GT4. No outlier rejection**

A single flaky test causing gate failure directly affects the EMA. No outlier filtering.
After a reset to 0.0, recovery takes ~30+ passing observations (alpha=0.1).

---

## 6. Playbook Store

### What works
- Generated from successful runs (`orchestrate.rs:10465-10484`)
- Queried during dispatch — relevant playbooks injected into system prompt
- Stored as individual JSON files in `.roko/learn/playbooks/`
- Rules in `.roko/learn/playbook-rules.toml`

### Issues

**PB1. No CLI for playbook management**

No `roko playbook list/show/delete`. Users must manually browse the directory. Cannot
inspect or edit playbook rules from the CLI.

---

## 7. Cost Tracking

### Issues

**CT1. Claude API agent sets `cost_usd: 0.0`** (`claude_agent.rs:445`)

The actual cost is NOT computed at the agent level. It depends on the `CostTable` being
applied downstream. If the CostTable is missing the model or the task runner path isn't
used, cost is zero.

**CT2. `cost_usd_without_cache` is always identical to `cost_usd`**
(`orchestrate.rs:12614-12615`)

```rust
cost_usd_without_cache: f64::from(result.usage.cost_usd)
```

The "without cache" variant is never separately computed. It's always the same value.

**CT3. CostTable defaults missing many models** (`cost_table.rs:99-122`)

Only 8 models in defaults. Missing: Gemini (despite codebase support), Perplexity (despite
`roko research` using them), Ollama/local, older Claude variants. Unrecognized models get
`$0.00` cost from `calculate()` returning 0.0 at line 37.

**CT4. f32 precision loss for cost_usd** (`chat_types.rs:111`)

`cost_usd: f32` in core `Usage` but `f64` in `episode_logger::Usage`. Conversion at
`orchestrate.rs:15217` (`as f32`) loses precision. For sub-cent costs, f32 gives ~7
decimal digits — marginal.

**CT5. No tool-use token tracking**

No separate tracking of tool-use tokens. Claude's API reports tool-use within
`input_tokens`/`output_tokens`. Users can't see how much budget goes to tool calls vs
reasoning.

---

## Anti-Patterns

1. **Write-only learning**: The cascade router learns but loses bandit weights on restart.
   The adaptive thresholds adapt but the tune command can't apply changes. The playbook
   store generates playbooks but has no management CLI. Systems that write data but
   provide no way to inspect, control, or maintain it.

2. **Phantom features**: `MaxCostPerTurn` exists but doesn't enforce. `compact()` exists
   but is never called. `tune gates` has `--dry-run` but never writes regardless. Dream
   triggers write to a file that nothing reads. Features that appear complete but are
   non-functional.

3. **Permissive as escape hatch**: Post-dispatch violations are warnings. `permissive()`
   constructor exists. `dangerously_skip_permissions` is default. Safety is always the
   thing that gets relaxed for convenience.

4. **Single-path cost tracking**: Cost is accurate only when the specific right combination
   of dispatch path + CostTable + model slug match aligns. Any deviation silently produces
   $0.00.

---

## Root Cause Fix

1. **Persist bandit weights** — serialize the LinUCB W matrix alongside observation
   counts. Learning should survive restarts.

2. **Automatic compaction** — trigger episode compaction on session start or after N new
   episodes. Don't rely on manual intervention.

3. **Dream daemon** — implement the background worker that consumes `dream_triggers.jsonl`.
   Or consolidate dreams inline at plan completion (simpler).

4. **Playbook CLI** — `roko playbook list/show/delete/rules` for managing the store.

5. **Cost at the agent level** — every agent backend should compute cost from tokens ×
   rates before returning. Don't rely on downstream CostTable lookup.

6. **Post-dispatch violations should block or rollback** — secret leaks and forbidden
   writes should not be warnings. Either prevent the action or rollback the changes.

---

## Checklist

### Safety
- [ ] Promote post-dispatch secret leak / forbidden write to Block severity
- [ ] Add `NoNetworkAccess` invariant to implementer contract
- [ ] Remove or `#[cfg(test)]`-gate `AgentContract::permissive()`
- [ ] Wire `MaxCostPerTurn` to actual cumulative spend
- [ ] Fix CLAUDE.md claim about permissive fallback

### Learning
- [ ] Persist LinUCB bandit weights (W matrix)
- [ ] Feed proper routing context from manual overrides
- [ ] Show stage transitions in `roko learn route` output
- [ ] Add `roko playbook list/show/delete/rules` CLI

### Knowledge
- [ ] Query knowledge store in chat path (not just plan runs)
- [ ] Implement dream trigger consumer (daemon or inline)

### Episodes
- [ ] Trigger compaction automatically (on session start or threshold)
- [ ] Add `roko episodes compact` CLI command

### Gates
- [ ] Human-readable output for `roko learn tune gates`
- [ ] Fix `tune gates` to actually apply changes (or remove `--dry-run` flag)
- [ ] Add `--reset` flag for threshold reset
- [ ] Add outlier rejection to adaptive thresholds

### Cost
- [ ] Compute cost at agent level, not downstream
- [ ] Separately compute `cost_usd_without_cache`
- [ ] Add missing models to CostTable defaults
- [ ] Upgrade `cost_usd` to f64 in core Usage
- [ ] Track tool-use tokens separately

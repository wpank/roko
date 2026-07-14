# 11 — Execution Playbook

> **What this doc is:** the step-by-step operations manual for executing the 48-epic
> backlog. Written for a human operator or a roko agent running from the CLI.
> Every command is copy-pasteable. Every decision has a default.
>
> - Repo: `/Users/will/dev/nunchi/roko/roko`
> - Backlog index: [`00-INDEX.md`](00-INDEX.md)
> - Execution readiness: [`04-EXECUTION-READINESS.md`](04-EXECUTION-READINESS.md)
> - Work breakdown: [`03-WORK-BREAKDOWN-EPICS.md`](03-WORK-BREAKDOWN-EPICS.md)

---

## 1. Prerequisites

### 1.1 Rust toolchain

```bash
# Need 1.91+ for alloy deps
rustup update stable
rustup default stable
rustc --version   # must be >= 1.91.0

# Nightly formatter (matches CI)
rustup install nightly
```

### 1.2 Required tools

| Tool | Install | Why |
|---|---|---|
| `cargo` | (with rustup) | Build, test, clippy |
| `claude` | `brew install claude` or [claude.ai](https://claude.ai/code) | Default agent backend (`agent.command = "claude"`) |
| `git` | system | Branch/worktree isolation, PR workflow |
| `gh` | `brew install gh && gh auth login` | PR creation/merge from CLI |
| `jq` | `brew install jq` | Parse JSONL state files |

### 1.3 Environment variables

The default provider is `claude_cli` (spawns `claude` subprocess), which uses your
Claude login -- no API key needed. For non-Claude providers, set keys in your shell:

```bash
# Required for non-Claude routing
export ANTHROPIC_API_KEY="sk-ant-..."     # Claude API (if using claude_api provider)
export OPENAI_API_KEY="sk-..."            # OpenAI models
export PERPLEXITY_API_KEY="pplx-..."      # Research agent (sonar)
export GEMINI_API_KEY="..."               # Gemini models

# Optional
export MOONSHOT_API_KEY="..."             # Kimi models
export ZAI_API_KEY="..."                  # Zhipu GLM models
```

Verify provider health:

```bash
cd /Users/will/dev/nunchi/roko/roko
cargo run -p roko-cli --bin roko -- config providers health
```

### 1.4 Build from source

```bash
cd /Users/will/dev/nunchi/roko/roko
cargo build --workspace 2>&1 | tail -5
# Should end with "Finished" — no errors
```

### 1.5 Initialize workspace state

```bash
# Create .roko/ if it doesn't exist
cargo run -p roko-cli --bin roko -- init

# Verify workspace health
cargo run -p roko-cli --bin roko -- doctor
```

---

## 2. Step-by-step M0 bootstrap

**M0 is the gate on everything.** Until E01 lands, `roko plan run` defaults to
the Graph engine (a dry-run stub that does nothing). See
[`04-EXECUTION-READINESS.md`](04-EXECUTION-READINESS.md) for full details.

### 2.1 Execute E01-T01: flip the engine default

This is THE self-hosting unblock. One literal change.

```bash
# Create a branch
git checkout -b feat/E01-engine-default main

# Validate the plan first
cargo run -p roko-cli --bin roko -- plan validate \
  tmp/status-quo/backlog/plans/E01-execution-engine

# Execute E01 (use --engine runner-v2 since the default is still broken)
cargo run -p roko-cli --bin roko -- plan run \
  tmp/status-quo/backlog/plans/E01-execution-engine \
  --engine runner-v2

# Or do it manually: change one line in main.rs
#   default_value = "graph"  →  default_value = "runner-v2"
#   at crates/roko-cli/src/main.rs:1361
```

### 2.2 Verify the fix

```bash
# Build
cargo build -p roko-cli

# Smoke test: bare plan run (no --engine flag) must do real work
cargo run -p roko-cli --bin roko -- plan run \
  tmp/status-quo/backlog/plans/E01-execution-engine --fresh

# Check: did real files change?
git status --porcelain

# Check: did episodes get written?
test -s .roko/episodes.jsonl && echo "OK: episodes written" || echo "FAIL: no episodes"

# Check: gate verdicts recorded?
grep -c '"kind":"GateVerdict"' .roko/signals.jsonl 2>/dev/null || echo "0 verdicts"
```

### 2.3 Land the remaining M0 items

After E01-T01, execute in order:

```bash
# E01-T02: fix resume routing
# E05-T02 + E05-T03: honest gates (stubs → Skipped, not pass)
# E04 safety subset: deny-list plumbing (P16)

# Each follows the same pattern:
cargo run -p roko-cli --bin roko -- plan validate \
  tmp/status-quo/backlog/plans/<epic-dir>

cargo run -p roko-cli --bin roko -- plan run \
  tmp/status-quo/backlog/plans/<epic-dir> --engine runner-v2
```

### 2.4 M0 exit smoke test

```bash
cd /Users/will/dev/nunchi/roko/roko
cargo build -p roko-cli
cargo run -p roko-cli --bin roko -- plan validate tmp/status-quo/backlog/plans/E01-execution-engine
cargo run -p roko-cli --bin roko -- plan run tmp/status-quo/backlog/plans/E01-execution-engine --fresh
git status --porcelain                                             # real edits present?
grep -c '"kind":"GateVerdict"' .roko/signals.jsonl                 # verdicts recorded?
tail -5 .roko/state/run-ledger.jsonl                               # honest task pass/fail?
```

If `git status` is empty after a successful run, M0.1 is not done.

---

## 3. Parallel execution strategy

### 3.1 Worktree-based isolation

After M0 lands, run independent tracks in parallel using git worktrees.
Each worktree is an isolated copy of the repo with its own branch.

```bash
# Create worktrees for parallel tracks
REPO=/Users/will/dev/nunchi/roko/roko

# Track A: Security (E04)
git worktree add "$REPO/.claude/worktrees/track-a-security" -b track-a/E04-security main

# Track B: Providers + MCP (E14, E15)
git worktree add "$REPO/.claude/worktrees/track-b-providers" -b track-b/E14-E15-providers main

# Track C: Types + Storage (E03, E02)
git worktree add "$REPO/.claude/worktrees/track-c-types" -b track-c/E03-E02-types main
```

### 3.2 Run a plan inside a worktree

```bash
cd "$REPO/.claude/worktrees/track-a-security"
cargo run -p roko-cli --bin roko -- plan run \
  tmp/status-quo/backlog/plans/E04-security-perimeter \
  --engine runner-v2
```

### 3.3 File exclusivity rules

Tracks MUST be file-disjoint. The recommended parallel groups from
[`03-WORK-BREAKDOWN-EPICS.md`](03-WORK-BREAKDOWN-EPICS.md):

| Track | Epics | Primary files | Safe to run with |
|---|---|---|---|
| A | E04 | `roko-serve` middleware, `roko-agent/safety`, `roko-acp` | B, D |
| B | E14, E15 | `roko-std/tool/*`, `roko-agent/provider`, `roko-mcp-code` | A, D |
| C | E03, E02, E05, E06 | `roko-core`, `roko-fs`, `runner/gate_dispatch.rs` | D only (E02 touches serve) |
| D | E09, E16 | `roko-cli/src/commands/`, `.roko/metrics/` | A, B, C |

**Caution:** E02, E03, and E04 all touch `roko-serve` routes. Run E03 before E02
(E03 changes type signatures that E02 consumers depend on). Never run E02 and E04
as siblings in the same parallel wave.

### 3.4 Merge completed tracks

```bash
# After a track finishes, merge back to main
cd "$REPO"
git checkout main
git merge --no-ff track-a/E04-security
# or: create a PR
gh pr create --base main --head track-a/E04-security \
  --title "E04: Security perimeter" \
  --body "Closes E04-T01 through E04-T19"
```

### 3.5 Maximum concurrency

Runner v2 supports up to `max_concurrent_plans = 4` (hardcoded in
`roko-core/src/defaults.rs:313`). Within a single plan, execution is currently
serial (one agent at a time). Intra-plan parallelism is tracked as E01-T04.

---

## 4. Resource management

### 4.1 Disk space

| Item | Typical size | Location |
|---|---|---|
| Build artifacts | 5-15 GB | `target/` |
| Git worktrees (each) | 200-500 MB | `.claude/worktrees/` |
| `.roko/` state | 10-200 MB | `.roko/` |
| Episode log | Grows ~1 KB/task | `.roko/episodes.jsonl` |
| Signal log | Grows ~2 KB/task | `.roko/signals.jsonl` |
| Events log | Grows fast (heartbeats) | `.roko/events.jsonl` |

**Minimum free space:** 20 GB for one worktree + build. 50 GB for 4 parallel tracks.

### 4.2 Cleanup commands

```bash
# Clear build cache (recoverable -- just rebuilds)
cargo clean

# Remove old worktrees
git worktree list
git worktree remove .claude/worktrees/<name>

# GC the knowledge store
cargo run -p roko-cli --bin roko -- knowledge gc

# Rotate large logs (E02-T07 will automate this; until then, manual)
ROKO_DIR=/Users/will/dev/nunchi/roko/roko/.roko

# Rotate events.jsonl (main offender for disk usage)
if [ -f "$ROKO_DIR/events.jsonl" ] && [ "$(wc -c < "$ROKO_DIR/events.jsonl")" -gt 104857600 ]; then
  mv "$ROKO_DIR/events.jsonl" "$ROKO_DIR/events.jsonl.bak.$(date +%s)"
  echo "Rotated events.jsonl (was > 100 MB)"
fi

# Prune signals.jsonl similarly
if [ -f "$ROKO_DIR/signals.jsonl" ] && [ "$(wc -c < "$ROKO_DIR/signals.jsonl")" -gt 52428800 ]; then
  mv "$ROKO_DIR/signals.jsonl" "$ROKO_DIR/signals.jsonl.bak.$(date +%s)"
  echo "Rotated signals.jsonl (was > 50 MB)"
fi
```

### 4.3 State files to preserve

Never delete these without backing up:

| File | Why |
|---|---|
| `.roko/state/state-snapshot.json` | Runner v2 resume state |
| `.roko/state/executor.json` | Legacy executor resume state |
| `.roko/episodes.jsonl` | Agent turn history (learning input) |
| `.roko/learn/cascade-router.json` | Model routing state (trained weights) |
| `.roko/learn/gate-thresholds.json` | Gate EMA thresholds |

---

## 5. Rate limit awareness

### 5.1 Provider rate limits

| Provider | Default tier limits | Mitigation |
|---|---|---|
| Claude CLI | Bounded by your plan (Pro/Team/Enterprise) | Use `--engine runner-v2` which processes tasks serially |
| Claude API | 50 RPM / 40k TPM (free) to 4000 RPM (scale) | Set `max_concurrent_plans = 1` for free tier |
| OpenAI | Varies by tier | Set `rate_limit_retry_attempts` in config |
| Perplexity | 50 RPM (Pro) | Research calls are infrequent; rarely rate-limited |

### 5.2 Budget configuration

Task-level budget controls in `tasks.toml`:

```toml
[[task]]
id = "E01-T04"
tier = "architectural"           # Higher tier = more expensive model
model_hint = "claude-sonnet-4-6" # Override default model
timeout_secs = 600               # 10 min max per task
max_retries = 3                  # Retry on failure
```

Global budget in `roko.toml`:

```toml
[agent]
context_limit_k = 200       # Max context window (tokens, thousands)
default_effort = "medium"   # low/medium/high → affects token usage

[learning_config]
enable_cascade_routing = true  # Routes cheap tasks to cheaper models
```

### 5.3 Tier-to-model mapping

The cascade router selects models based on task tier:

| Tier | Default model | Approx. cost/task |
|---|---|---|
| `mechanical` | `claude-haiku-4-5` | ~$0.01-0.05 |
| `focused` | `claude-sonnet-4-6` | ~$0.05-0.50 |
| `integrative` | `claude-sonnet-4-6` | ~$0.10-1.00 |
| `architectural` | `claude-opus-4-6` | ~$0.50-5.00 |

### 5.4 Handling rate limit errors

If you see `429 Too Many Requests`:

1. Runner v2 retries automatically (`DEFAULT_RATE_LIMIT_RETRY_ATTEMPTS = 5`,
   base delay 2s, max backoff 60s).
2. If retries exhaust, reduce concurrency:
   ```bash
   # Run one plan at a time
   cargo run -p roko-cli --bin roko -- plan run \
     tmp/status-quo/backlog/plans/E01-execution-engine \
     --engine runner-v2
   # (default max_concurrent_plans = 4; serial = no concurrent pressure)
   ```
3. Switch provider or model: add `model_hint = "claude-haiku-4-5"` to the task.

---

## 6. GitHub workflow

### 6.1 Branch naming convention

```
feat/E<NN>-<short-name>        # Epic implementation
fix/E<NN>-T<NN>-<what>         # Single task fix
track-<letter>/<scope>         # Parallel track worktrees
chore/<scope>                  # Non-functional (docs, cleanup)
```

### 6.2 Agent branching flow

For each epic or track:

```bash
# 1. Create branch from main
git checkout -b feat/E03-type-consolidation main

# 2. Run the plan
cargo run -p roko-cli --bin roko -- plan run \
  tmp/status-quo/backlog/plans/E03-type-consolidation \
  --engine runner-v2

# 3. Pre-commit checks (MANDATORY)
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace

# 4. Commit
git add -A
git commit -m "feat(E03): canonicalize type families

Collapse 7 duplicate type definitions to single canonical homes.
Add From adapters for cross-crate conversions.

Tasks: E03-T01 through E03-T07"

# 5. Push and PR
git push -u origin feat/E03-type-consolidation
gh pr create --base main \
  --title "E03: Type consolidation" \
  --body "$(cat <<'EOF'
## Summary
- Canonicalize GateVerdict, DashboardSnapshot, RetentionPolicy
- Delete orphan state_hub.rs and chain Engram stub
- Add From adapters for cross-crate conversions

## Checklist
- [ ] E03-T01 through E03-T07 acceptance criteria met
- [ ] cargo test --workspace passes
- [ ] cargo clippy clean

## Test plan
- Verify `roko plan run` still works after type changes
- Verify `roko dashboard` renders correctly
EOF
)"
```

### 6.3 PR review + merge

```bash
# Check PR status
gh pr status

# View CI checks
gh pr checks <number>

# Merge when CI passes (use squash for single-epic PRs)
gh pr merge <number> --squash --delete-branch

# Update local
git checkout main && git pull
```

### 6.4 Never push directly to main

All changes go through PRs. This is enforced by convention and CLAUDE.md rules.

---

## 7. Monitoring

### 7.1 Interactive TUI

```bash
# Launch the ratatui dashboard (F1-F7 tabs)
cargo run -p roko-cli --bin roko -- dashboard
```

| Tab | Shows |
|---|---|
| F1 | Overview: plans, agents, recent activity |
| F2 | Plans: task status, progress bars |
| F3 | Agents: active agents, resource usage |
| F4 | Gates: verdict history, pass rates |
| F5 | Learning: routing decisions, experiments |
| F6 | Knowledge: neuro store stats |
| F7 | System: logs, events, health |

### 7.2 HTTP control plane

```bash
# Start the API server (background)
cargo run -p roko-cli --bin roko -- serve &

# Query plan status
curl -s http://localhost:6677/api/v1/plans | jq '.[]'

# Query agent status
curl -s http://localhost:6677/api/v1/agents | jq '.[]'

# Watch events via SSE
curl -N http://localhost:6677/api/v1/events/stream

# Check workspace status
curl -s http://localhost:6677/api/v1/workspace/status | jq '.'
```

### 7.3 CLI status commands

```bash
# Quick status
cargo run -p roko-cli --bin roko -- status

# Learning state
cargo run -p roko-cli --bin roko -- learn all

# Plan list
cargo run -p roko-cli --bin roko -- plan list

# Episode count
wc -l .roko/episodes.jsonl

# Recent gate verdicts
tail -20 .roko/signals.jsonl | jq -r 'select(.kind=="GateVerdict") | "\(.task_id): \(.passed)"'
```

### 7.4 Tail logs during a run

```bash
# In a separate terminal, watch the run-ledger
tail -f .roko/state/run-ledger.jsonl | jq -r '"\(.timestamp) \(.task_id) \(.status)"'

# Watch episodes as they're written
tail -f .roko/episodes.jsonl | jq -r '"\(.timestamp) \(.agent_role) \(.task_id)"'

# Watch events
tail -f .roko/events.jsonl | jq -r '"\(.ts) \(.kind)"' 2>/dev/null
```

---

## 8. Failure recovery

### 8.1 Task failure during plan run

When a task fails (gate verdict = fail, or agent timeout), Runner v2:

1. Records the failure in `.roko/state/run-ledger.jsonl`
2. Appends replan context to the next prompt (prompt-only replan; true
   tasks.toml rewrite is E01-T06)
3. Retries up to `max_retries` (default 3)
4. Marks the task `failed` and moves to the next independent task

**To investigate a failure:**

```bash
# Check run ledger for failed tasks
grep '"failed"' .roko/state/run-ledger.jsonl | jq '.'

# Check the latest episode for error context
tail -50 .roko/episodes.jsonl | jq 'select(.task_id=="E01-T04")'

# Check gate verdicts
grep 'E01-T04' .roko/signals.jsonl | jq '.'
```

### 8.2 Resume after interruption

If a run is interrupted (Ctrl-C, crash, OOM):

```bash
# Resume from last snapshot (Runner v2 auto-detects completed tasks)
cargo run -p roko-cli --bin roko -- plan run \
  tmp/status-quo/backlog/plans/E01-execution-engine \
  --engine runner-v2 \
  --resume .roko/state/executor.json

# Or start fresh (re-runs everything)
cargo run -p roko-cli --bin roko -- plan run \
  tmp/status-quo/backlog/plans/E01-execution-engine \
  --engine runner-v2 \
  --fresh
```

### 8.3 Common failure modes

| Symptom | Cause | Fix |
|---|---|---|
| Run completes instantly, no files changed | Graph engine (dry-run stub) | Add `--engine runner-v2` (or land E01-T01) |
| All tasks "pass" but nothing is real | Stub-pass in rungs 3-6 | Land E05-T02/T03 (honest gates) |
| `claude: command not found` | Claude CLI not installed | `brew install claude` or `npm i -g @anthropic/claude-code` |
| `429 Too Many Requests` | Provider rate limit | Reduce concurrency, wait, or switch model |
| `cargo build` fails | Rustc too old | `rustup update stable` (need 1.91+) |
| `.roko/episodes.jsonl` empty after run | Agent never dispatched | Check `--engine runner-v2` flag; inspect run-ledger |
| Resume re-runs completed tasks | Resume routing to Graph | Land E01-T02 (route resume to RunnerV2) |

### 8.4 Manual replan

If a task repeatedly fails and needs scope adjustment:

```bash
# 1. Edit the task in the tasks.toml
vim tmp/status-quo/backlog/plans/E01-execution-engine/tasks.toml
# Change: tier, model_hint, description, max_retries, timeout_secs

# 2. Re-validate
cargo run -p roko-cli --bin roko -- plan validate \
  tmp/status-quo/backlog/plans/E01-execution-engine

# 3. Re-run (--fresh to ignore stale state)
cargo run -p roko-cli --bin roko -- plan run \
  tmp/status-quo/backlog/plans/E01-execution-engine \
  --engine runner-v2 --fresh
```

---

## 9. Plan validation

### 9.1 Pre-flight validation

Always validate before executing:

```bash
# Validate a single epic plan
cargo run -p roko-cli --bin roko -- plan validate \
  tmp/status-quo/backlog/plans/E01-execution-engine

# Validate all plans (batch)
for d in tmp/status-quo/backlog/plans/E*/; do
  echo "--- $d ---"
  cargo run -p roko-cli --bin roko -- plan validate "$d" 2>&1 | tail -3
done
```

### 9.2 What validation checks

The validator (`plan_validate.rs`) enforces:

| Check | Code | Severity |
|---|---|---|
| Required fields (id, title) | `PLAN_001` | Error |
| Valid role | `PLAN_008` | Warning |
| Valid tier | `PLAN_004` | Warning |
| File references exist on disk | `validate_file_references` | Warning |
| DAG is acyclic | `PLAN_002` | Error |
| Description present and < 500 words | `PLAN_005` | Warning |
| Verify steps present for implementer | `MissingVerify` | Warning |
| Context/read_files present | `MissingReadFiles` | Warning |
| Timeout > 0 | schema check | Error |
| gate_rung in 0..=6 | `PLAN_007` | Error |

### 9.3 Fixing validation errors

```bash
# Common fix: add missing verify step
# In tasks.toml, add under the [[task]] block:
[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli 2>&1"
fail_msg = "roko-cli must compile"
```

---

## 10. Cost estimation

### 10.1 Per-milestone estimates

Based on task counts and tier distribution from
[`05-MASTER-CHECKLIST.md`](05-MASTER-CHECKLIST.md):

| Milestone | Tasks | Tier mix | Est. cost (Claude CLI) | Est. time (serial) |
|---|---|---|---|---|
| **M0** | 56 | 15 mech, 20 focused, 15 integ, 6 arch | $15-50 | 4-8 hours |
| **M1** | 39 | 8 mech, 15 focused, 12 integ, 4 arch | $15-40 | 3-6 hours |
| **M2** | 51 | 10 mech, 20 focused, 15 integ, 6 arch | $20-60 | 4-8 hours |
| **M3+** | 3 | 1 focused, 1 integ, 1 arch | $2-10 | 30-60 min |
| **E19-E48** | 240 | Mixed | $80-250 | 20-40 hours |
| **Total** | 389 | | **$130-410** | **30-60 hours** |

> **Note:** These are rough estimates. Actual cost depends on retry rate,
> context length, and model selection. Claude CLI cost is included in your
> subscription; API costs are pay-per-token.

### 10.2 Cost per tier (API pricing, approximate)

| Tier | Model | Input $/1M | Output $/1M | Typical task cost |
|---|---|---|---|---|
| mechanical | haiku-4-5 | $0.80 | $4 | $0.01-0.05 |
| focused | sonnet-4-6 | $3 | $15 | $0.05-0.50 |
| integrative | sonnet-4-6 | $3 | $15 | $0.10-1.00 |
| architectural | opus-4-6 | $15 | $75 | $0.50-5.00 |

### 10.3 Reducing costs

- Use `model_hint = "claude-haiku-4-5"` for mechanical/focused tasks
- Set `max_retries = 1` for cheap tasks (failures are cheap to re-run)
- Enable cascade routing: `learning_config.enable_cascade_routing = true`
- Run `roko learn tune routing` periodically to optimize model selection

---

## 11. Example session: M0 through M1

This is a complete walkthrough of bootstrapping roko's self-hosting capability and
then executing the first correctness wave.

### Phase 1: M0 bootstrap (serial, ~4-8 hours)

```bash
cd /Users/will/dev/nunchi/roko/roko

# ── 0. Preflight ──────────────────────────────────────────────────
rustc --version                                    # >= 1.91.0
cargo build --workspace                            # clean build
cargo run -p roko-cli --bin roko -- doctor          # workspace health

# ── 1. Branch for M0 ─────────────────────────────────────────────
git checkout -b feat/M0-bootstrap main

# ── 2. Validate E01 plan ─────────────────────────────────────────
cargo run -p roko-cli --bin roko -- plan validate \
  tmp/status-quo/backlog/plans/E01-execution-engine
# Expect: 0 errors (warnings OK)

# ── 3. Execute E01 (the self-hosting unblock) ────────────────────
# MUST use --engine runner-v2 until E01-T01 flips the default
cargo run -p roko-cli --bin roko -- plan run \
  tmp/status-quo/backlog/plans/E01-execution-engine \
  --engine runner-v2

# ── 4. Verify E01 landed ─────────────────────────────────────────
# After E01-T01: bare plan run should work without --engine flag
cargo run -p roko-cli --bin roko -- plan run \
  tmp/status-quo/backlog/plans/E01-execution-engine --fresh
git status --porcelain                             # real file changes?
test -s .roko/episodes.jsonl && echo "OK"          # episodes written?

# ── 5. Pre-commit checks ─────────────────────────────────────────
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace

# ── 6. Commit E01 ────────────────────────────────────────────────
git add -A
git commit -m "feat(E01): flip engine default to runner-v2

Make bare 'roko plan run' dispatch real agents via Runner v2 instead
of the dry-run Graph stub. Fixes resume routing. Adds regression test.

Tasks: E01-T01 through E01-T10"

# ── 7. Execute E05 minimum (honest gates) ────────────────────────
cargo run -p roko-cli --bin roko -- plan validate \
  tmp/status-quo/backlog/plans/E05-gate-adaptivity-live
cargo run -p roko-cli --bin roko -- plan run \
  tmp/status-quo/backlog/plans/E05-gate-adaptivity-live

# ── 8. Execute E04 safety subset ─────────────────────────────────
cargo run -p roko-cli --bin roko -- plan validate \
  tmp/status-quo/backlog/plans/E04-security-perimeter
cargo run -p roko-cli --bin roko -- plan run \
  tmp/status-quo/backlog/plans/E04-security-perimeter

# ── 9. Pre-commit + commit remaining M0 ──────────────────────────
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
git add -A
git commit -m "feat(M0): honest gates + safety perimeter

E05-T02/T03: stubs report Skipped not pass; gate verdicts are honest.
E04 subset: deny-list plumbing for unattended agent safety.

Tasks: E05-T02, E05-T03, E04-T05, E04-T06, E04-T07"

# ── 10. M0 exit smoke ────────────────────────────────────────────
cargo build -p roko-cli
cargo run -p roko-cli --bin roko -- plan run \
  tmp/status-quo/backlog/plans/E01-execution-engine --fresh
git status --porcelain                             # non-empty?
grep -c '"kind":"GateVerdict"' .roko/signals.jsonl # verdicts counted?
tail -5 .roko/state/run-ledger.jsonl               # honest pass/fail?

# ── 11. Push + PR ────────────────────────────────────────────────
git push -u origin feat/M0-bootstrap
gh pr create --base main --title "M0: Self-hosting bootstrap" \
  --body "$(cat <<'EOF'
## Summary
- E01: Flip engine default to runner-v2 (bare plan run does real work)
- E05 minimum: Honest gates (no stub-pass)
- E04 subset: Safety deny-list for unattended agents

## Test plan
- [ ] Bare `roko plan run` dispatches real agents
- [ ] Stub rungs report Skipped, not Pass
- [ ] Resume works without --engine flag
- [ ] cargo test --workspace passes
EOF
)"
```

### Phase 2: M1 correctness (parallel tracks, ~3-6 hours)

After M0 merges:

```bash
git checkout main && git pull
REPO=/Users/will/dev/nunchi/roko/roko

# ── Create parallel worktrees ────────────────────────────────────

# Track B: Providers + MCP (file-disjoint from C)
git worktree add "$REPO/.claude/worktrees/track-b" -b track-b/E14-E15 main

# Track C: Types → Storage → Gates → Compose (serial within track)
git worktree add "$REPO/.claude/worktrees/track-c" -b track-c/E03-E02-E05-E06 main

# ── Track B (in one terminal) ────────────────────────────────────
cd "$REPO/.claude/worktrees/track-b"
cargo run -p roko-cli --bin roko -- plan run \
  tmp/status-quo/backlog/plans/E14-providers-tools
cargo run -p roko-cli --bin roko -- plan run \
  tmp/status-quo/backlog/plans/E15-mcp-config
cargo +nightly fmt --all && cargo clippy --workspace --no-deps -- -D warnings && cargo test --workspace
git add -A && git commit -m "feat(M1-B): providers + MCP correctness"
git push -u origin track-b/E14-E15

# ── Track C (in another terminal) ────────────────────────────────
cd "$REPO/.claude/worktrees/track-c"
# E03 MUST precede E02 (type signatures before consumers)
cargo run -p roko-cli --bin roko -- plan run \
  tmp/status-quo/backlog/plans/E03-type-consolidation
cargo run -p roko-cli --bin roko -- plan run \
  tmp/status-quo/backlog/plans/E02-STORAGE-CONVERGENCE
cargo run -p roko-cli --bin roko -- plan run \
  tmp/status-quo/backlog/plans/E05-gate-adaptivity-live
cargo run -p roko-cli --bin roko -- plan run \
  tmp/status-quo/backlog/plans/E06-COMPOSE-UNIFY
cargo +nightly fmt --all && cargo clippy --workspace --no-deps -- -D warnings && cargo test --workspace
git add -A && git commit -m "feat(M1-C): types + storage + gates + compose"
git push -u origin track-c/E03-E02-E05-E06

# ── E16 (after E14 merges) ──────────────────────────────────────
git checkout -b feat/E16-prd main
# wait for track-b PR to merge first
git merge origin/track-b/E14-E15  # or after PR merge: git pull
cargo run -p roko-cli --bin roko -- plan run \
  tmp/status-quo/backlog/plans/E16-prd-self-hosting-gaps
cargo +nightly fmt --all && cargo clippy --workspace --no-deps -- -D warnings && cargo test --workspace
git add -A && git commit -m "feat(E16): PRD self-hosting pipeline"
git push -u origin feat/E16-prd

# ── Create PRs ───────────────────────────────────────────────────
gh pr create --base main --head track-b/E14-E15 --title "M1-B: Providers + MCP"
gh pr create --base main --head track-c/E03-E02-E05-E06 --title "M1-C: Types + Storage + Compose"
gh pr create --base main --head feat/E16-prd --title "E16: PRD self-hosting"

# ── After all PRs merge: M1 exit validation ──────────────────────
git checkout main && git pull
cargo build --workspace
cargo test --workspace
cargo run -p roko-cli --bin roko -- status
cargo run -p roko-cli --bin roko -- plan list
```

---

## 12. Quick reference card

```bash
# ── Validate a plan ──────────────────────────────────────────────
cargo run -p roko-cli --bin roko -- plan validate tmp/status-quo/backlog/plans/<EPIC>

# ── Execute a plan ───────────────────────────────────────────────
cargo run -p roko-cli --bin roko -- plan run tmp/status-quo/backlog/plans/<EPIC> --engine runner-v2

# ── Resume an interrupted run ────────────────────────────────────
cargo run -p roko-cli --bin roko -- plan run tmp/status-quo/backlog/plans/<EPIC> --resume .roko/state/executor.json

# ── Fresh re-run (ignore prior state) ────────────────────────────
cargo run -p roko-cli --bin roko -- plan run tmp/status-quo/backlog/plans/<EPIC> --fresh

# ── Pre-commit checks ───────────────────────────────────────────
cargo +nightly fmt --all && cargo clippy --workspace --no-deps -- -D warnings && cargo test --workspace

# ── Check what changed ───────────────────────────────────────────
git status --porcelain
tail -5 .roko/state/run-ledger.jsonl | jq '.'
wc -l .roko/episodes.jsonl

# ── Watch a run ──────────────────────────────────────────────────
cargo run -p roko-cli --bin roko -- dashboard

# ── Provider health ──────────────────────────────────────────────
cargo run -p roko-cli --bin roko -- config providers health
```

---

_Back to backlog index: [`00-INDEX.md`](00-INDEX.md)._

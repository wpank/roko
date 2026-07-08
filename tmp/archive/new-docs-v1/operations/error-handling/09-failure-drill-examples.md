# Failure Drill Examples

> Step-by-step walkthroughs of real failure scenarios an operator will encounter running
> Roko in production. Each drill covers: symptoms → diagnosis → recovery → prevention.

**Status**: Built (recovery mechanics) / Shipping (CLI commands shown)
**Crate**: cross-crate
**Depends on**: [Error Taxonomy](01-error-taxonomy.md), [Recovery Strategies](02-recovery-strategies.md),
[Partial Failure](05-partial-failure.md), [Cascade Failure](06-cascade-failure.md),
[Forensic Replay](07-forensic-replay.md), [Observability](08-observability.md)
**Last reviewed**: 2026-04-19

---

## Drill Index

| # | Scenario | Error class | Complexity |
|---|---|---|---|
| 1 | [Gate test rung fails repeatedly](#drill-1-gate-test-rung-fails-repeatedly) | Gate | Basic |
| 2 | [LLM rate limit causes cascade](#drill-2-llm-rate-limit-causes-cascade) | LLM + Cascade | Moderate |
| 3 | [Process crash mid-task](#drill-3-process-crash-mid-task) | Infrastructure | Basic |
| 4 | [Substrate runs out of disk space](#drill-4-substrate-runs-out-of-disk-space) | Infrastructure + Cascade | Moderate |
| 5 | [Agent hits max_turns without producing a diff](#drill-5-agent-hits-max_turns-without-producing-a-diff) | User + LLM | Basic |
| 6 | [Safety gate triggers on generated code](#drill-6-safety-gate-triggers-on-generated-code) | Safety | Moderate |
| 7 | [Parallel subtask fails with partial output](#drill-7-parallel-subtask-fails-with-partial-output) | Infrastructure | Moderate |
| 8 | [Config migration error after upgrade](#drill-8-config-migration-error-after-upgrade) | User | Basic |

---

## Drill 1: Gate Test Rung Fails Repeatedly

### Symptoms

```
[14:03:09] rung/test  FAIL (attempt 1/3)  ROKO-G-002
[14:03:11] rung/test  FAIL (attempt 2/3)  ROKO-G-002
[14:03:17] rung/test  FAIL (attempt 3/3)  ROKO-G-002
[14:03:17] gate       ESCALATE → returning failure to agent
```

The task is still running — agent received the test output as context and is attempting
a fix — but the same test keeps failing across multiple turns.

### Diagnosis

```bash
# Show the test failure output
roko events dump <task-id> | jq 'select(.event == "GateFailed") | .data.stderr_tail'

# Show which agent diffs were produced between test failures
roko events show <task-id> --type AgentProducedDiff,GateFailed
```

Check: is the agent producing different diffs each time, or the same diff? Same diff =
agent is stuck in a loop. Different diffs = agent is trying but the root cause is subtle.

### Recovery

```bash
# If agent is looping: reset the task, adjust the system_prompt, retry
roko run --reset-running <task-id>

# Edit system_prompt to give the agent more specific guidance about the failing test
$EDITOR roko.toml

# Retry with a stricter turn cap so loops exit quickly
roko run --task "fix the bug" --max-turns 5
```

### Prevention

```toml
[agent]
max_turns = 10          # tighter cap; expose loops faster
[gate]
max_retries = 2         # fewer retries before escalation
```

Add a test-failure summary step in `system_prompt`:
> "When tests fail, first identify the failing assertion, then produce a minimal diff
> targeting only that assertion's root cause."

---

## Drill 2: LLM Rate Limit Causes Cascade

### Symptoms

```
[14:01:30] llm   ROKO-L-001: 429 Too Many Requests (model=gpt-4o)
[14:01:30] llm   RETRY 1/3 (back-off 2s)
[14:01:32] llm   RETRY 2/3 (back-off 4s)
[14:01:36] llm   RETRY 3/3 (back-off 8s)
[14:01:44] llm   CIRCUIT OPEN: llm/openai
[14:01:44] agent ROKO-L-001: no model available
[14:01:44] gate  ROKO-G-004: circuit open — gate not executed
[14:01:44] task  FAILED cascade_depth=2
```

### Diagnosis

```bash
# Confirm circuit state
roko status --circuits
# llm/openai  OPEN  (opened 14:01:44, resets 14:02:44)

# Check rate limit event count
roko events dump <task-id> | jq 'select(.event == "LLMError") | .data.http_status'

# Check token consumption trend
roko metrics show --json | jq '.llm.tokens_in_total'
```

### Recovery

```bash
# Wait for auto-reset (60s) — check if circuit closed
roko status --circuits

# If provider is confirmed healthy, force-close
roko circuit reset llm/openai

# Resume the failed task
roko run --resume <task-id>
```

### Prevention

```toml
[agent]
# Add a fallback model — if primary hits rate limit, T1 router uses fallback
model = "gpt-4o"
# (CascadeRouter T1 already falls back to a cheaper model automatically)

[gate]
circuit_reset_timeout = "30s"   # faster recovery in low-volume environments
```

Distribute API keys across multiple providers (see
[../configuration/13-security-considerations.md](../configuration/13-security-considerations.md)):

```toml
[agent]
model   = "claude-opus-4-5"
gateway = "https://api.anthropic.com"

# Secondary key in .env:
# ROKO_AGENT_MODEL=gpt-4o
# ANTHROPIC_API_KEY=<primary>
# OPENAI_API_KEY=<fallback>
```

---

## Drill 3: Process Crash Mid-Task

### Symptoms

```bash
$ roko run --task "refactor auth module"
[14:05:33] task-7f3a  RUNNING ...
[14:06:12] (process exits; no output)

$ echo $?
137    # SIGKILL (OOM or manual kill)
```

The terminal shows an incomplete run. `.roko/state/executor.json` exists.

### Diagnosis

```bash
# Check executor state
cat .roko/state/executor.json | jq '{task_id, subtasks: [.subtasks[] | {id, status}]}'

# Verify event log up to crash point
roko events verify --task task-7f3a

# Find the last successful event
roko events show task-7f3a | tail -20
```

Expected: some subtasks `Succeeded`, some `Running` (in-flight at crash), none with
a clean `Failed` state.

### Recovery

```bash
# Mark in-flight subtasks as failed and prepare for resume
roko run --reset-running task-7f3a

# Inspect what was completed
roko run --resume task-7f3a --dry-run

# Resume — completed subtasks are skipped
roko run --resume task-7f3a
```

### If the event log chain is broken

```
Chain: BROKEN at seq=103 (hash mismatch)
```

Do not resume from a corrupt log. Start fresh:

```bash
roko run --reset-running task-7f3a   # mark as abandoned
roko run --task "refactor auth module"  # new task, new event log
```

### Prevention

```bash
# Use graceful shutdown (SIGTERM) instead of SIGKILL when possible
kill -SIGTERM <roko-pid>
# Roko will drain in-flight events and checkpoint state before exiting
```

For OOM kills: reduce `--concurrency` or set `ROKO_AGENT_MAX_TURNS` lower to limit
agent subprocess memory.

---

## Drill 4: Substrate Runs Out of Disk Space

### Symptoms

```
[14:10:01] substrate  ROKO-I-003: write failed — ENOSPC
[14:10:01] learn      episode dropped (substrate unavailable)
[14:10:01] learn      episodes_dropped_total: 1
```

Tasks may continue succeeding (substrate failure does not block task completion), but
the learning subsystem silently stops recording episodes.

### Diagnosis

```bash
# Check disk usage
df -h $(roko config get substrate.data_dir)
# /dev/sda1   95G  95G  0 4G  100% /Users/will

# Check substrate health
roko substrate health
# status: ERROR — write test failed (ENOSPC)

# Check how many episodes were dropped
roko metrics show --json | jq '.learn.episodes_dropped_total'
```

### Recovery

**Step 1: Free disk space**

```bash
# Check what's large in the substrate directory
du -sh $(roko config get substrate.data_dir)/*

# If substrate itself is bloated, run GC
roko substrate gc
# Removes expired/unreachable engrams; logs bytes reclaimed

# Check max_size_gb setting
roko config get substrate.max_size_gb
```

**Step 2: Increase disk cap if warranted**

```toml
[substrate]
max_size_gb = 20    # increase from default 10
```

**Step 3: Verify substrate health**

```bash
roko substrate health
# status: OK — write test passed
```

**Step 4: Re-process dropped episodes**

```bash
roko learn --process --retry-failed
roko learn status   # confirm episodes_queued draining
```

### Prevention

```toml
[substrate]
max_size_gb = 10    # set a hard cap to prevent unbounded growth
gc_interval = "24h" # run GC daily
gc_min_age  = "30d" # keep everything < 30 days
```

Alert when `substrate.size_bytes > max_size_gb × 0.9 × 1e9` (see
[08-observability.md](08-observability.md)).

---

## Drill 5: Agent Hits max_turns Without Producing a Diff

### Symptoms

```
[14:15:00] agent  turn 1/20 ...
[14:15:08] agent  turn 2/20 ...
...
[14:21:40] agent  turn 20/20  ROKO-U-001: max_turns exceeded
[14:21:40] task   FAILED — no diff produced
```

No gate ran. No code was changed. The task consumed 20 turns and produced nothing.

### Diagnosis

```bash
# Show all agent turns
roko events show <task-id> --type AgentTurn,AgentProducedDiff

# If no AgentProducedDiff events exist, the agent never committed a diff.
# Show what the agent was doing:
roko events dump <task-id> | jq 'select(.event == "AgentTurn") | {turn: .data.turn, tokens_out: .data.tokens_out}'
```

Common causes:
- **Ambiguous task description** — agent is asking clarifying questions to a void
- **system_prompt conflict** — instructions contradict each other
- **Wrong model for the task** — cheap T1 model used for a complex refactor
- **MCP tool failure** — agent's tool calls are all failing, blocking progress

Check tool failures:

```bash
roko events dump <task-id> | jq 'select(.event == "MCPToolError")'
```

### Recovery

```bash
# Inspect the last few agent turns' outputs (if prompt_logging = true)
roko prompt show $(roko events dump <task-id> | jq -r 'select(.event == "LLMResponse") | .data.prompt_hash' | tail -3)

# Rewrite the task description and retry
roko run --task "Refactor the auth module: extract JwtValidator into its own struct in src/auth/jwt.rs"

# If MCP tool failures were the cause, check tool health
roko tools health
```

### Prevention

```toml
[agent]
max_turns = 10    # expose stuck agents faster
```

Write specific, actionable task descriptions. Avoid:
- "clean up the code"
- "fix things"
- "make it better"

Prefer:
- "Extract `validate_token` from `src/auth.rs` into a new `src/auth/validator.rs` module"

---

## Drill 6: Safety Gate Triggers on Generated Code

### Symptoms

```
[14:30:05] rung/security  FAIL  ROKO-S-001: policy violation detected
[14:30:05] gate           ESCALATE → agent receives violation details
[14:30:05] agent          turn 8/20 — attempting fix ...
[14:30:15] rung/security  FAIL  ROKO-S-001: policy violation detected (again)
```

The agent is generating code that repeatedly triggers the security gate.

### Diagnosis

```bash
# Show security gate failure details
roko events dump <task-id> | jq 'select(.event == "GateFailed" and .data.rung == "security") | .data'

# What policy was violated?
# data.violation_type examples: "hardcoded_secret", "shell_injection", "path_traversal"
```

### Recovery

If the violation is a **false positive** (generated code is safe):

```bash
# Add an exemption for the specific rule (if your policy allows)
roko config set gate.security_exemptions '["rule-id-123"]'

# Or adjust the security gate sensitivity
roko config set gate.security_level "standard"  # from "strict"
```

If the violation is **real** (agent is generating unsafe code):

```bash
# Reset the task
roko run --reset-running <task-id>

# Add explicit security constraints to system_prompt
# Example addition to roko.toml:
[agent]
system_prompt = """
... existing prompt ...
SECURITY CONSTRAINTS:
- Never hardcode secrets, API keys, or credentials.
- Never use os.system() or subprocess with user-controlled input.
- Always use parameterised queries for database access.
"""
```

### Prevention

Run the security gate locally before submitting tasks:

```bash
roko gate run security --diff $(git diff HEAD)
```

---

## Drill 7: Parallel Subtask Fails With Partial Output

### Symptoms

```
$ roko run --task "analyse all 5 modules"

[14:40:00] subtask sub-0  STARTED
[14:40:00] subtask sub-1  STARTED
[14:40:00] subtask sub-2  STARTED
[14:40:00] subtask sub-3  STARTED
[14:40:00] subtask sub-4  STARTED
[14:42:15] subtask sub-0  SUCCEEDED
[14:42:30] subtask sub-1  SUCCEEDED
[14:43:00] subtask sub-2  FAILED  ROKO-L-001: rate_limit
[14:43:05] subtask sub-3  SUCCEEDED
[14:44:00] subtask sub-4  SUCCEEDED

Task partially complete: 4/5 subtasks succeeded.
```

### Diagnosis

```bash
# Show failed subtask details
cat .roko/state/executor.json | jq '.subtasks[] | select(.status == "Failed")'

# Show the event log for the failed subtask
roko events show <task-id> --subtask sub-2

# Confirm the 4 successful subtasks' outputs are valid
roko events verify --task <task-id>
```

### Recovery

```bash
# Resume — only sub-2 will be re-run
roko run --resume <task-id>

# If sub-2 hit rate limits, wait or switch model
roko config set agent.model "claude-opus-4-5"   # if openai was rate-limited
roko run --resume <task-id>
```

### Prevention

```toml
[agent]
max_turns = 10

[orchestrator]
fail_fast = false   # already the default; ensure it's not set to true
```

Spread subtasks across models to avoid single-provider rate limits.

---

## Drill 8: Config Migration Error After Upgrade

### Symptoms

```bash
$ roko run --task "..."
Error: config parse failed — unknown key `bardo.model` at line 3 of roko.toml
```

After upgrading Roko, the old config format is rejected.

### Diagnosis

```bash
# Show full validation errors
roko config show
# Error: unknown key `bardo.model`
# Error: unknown key `mori.gate_timeout`
# Hint: run `roko config migrate` to update roko.toml
```

### Recovery

```bash
# Auto-migrate old keys to new format
roko config migrate

# Verify the migrated config is valid
roko config show

# Diff the migration to understand what changed
roko config migrate --dry-run
```

If auto-migration fails for a key:

```bash
# Show the mapping table
roko config migrate --show-mappings

# Example mappings:
# bardo.model         → agent.model
# bardo.max_turns     → agent.max_turns
# mori.gate_timeout   → gate.timeout
# mori.compile_gate   → gate.pipeline (value mapping: true → ["compile","test","clippy"])
```

Manually edit `roko.toml` using the mapping table in
[../configuration/11-config-migration.md](../configuration/11-config-migration.md).

### Prevention

Before upgrading, run:

```bash
roko config validate   # validate against the installed schema
roko config migrate --dry-run  # preview any needed changes
```

Pin your Roko version in CI until config migration is verified:

```bash
cargo install roko --version =0.x.y  # exact version pin
```

---

## Appendix: Decision Tree for Unknown Failures

```
Task failed with ROKO-?-??? error code
│
├─ error_code starts with ROKO-G → Gate failure
│   ├─ rung = compile → Check for syntax/compile errors in generated code
│   ├─ rung = test    → Check test failures; agent may need better context
│   └─ rung = security → Check violation type; may be false positive
│
├─ error_code starts with ROKO-I → Infrastructure failure
│   ├─ ROKO-I-001 → Filesystem; check df, permissions
│   ├─ ROKO-I-002 → Network; check connectivity, DNS
│   ├─ ROKO-I-003 → Substrate; check disk space, run roko substrate health
│   └─ ROKO-I-004 → Process; check executor.json, consider --resume
│
├─ error_code starts with ROKO-L → LLM failure
│   ├─ ROKO-L-001 → Rate limit; wait or switch model
│   ├─ ROKO-L-002 → Auth; check API key
│   └─ ROKO-L-003 → Timeout; check latency, reduce task scope
│
├─ error_code starts with ROKO-U → User/config error
│   ├─ ROKO-U-001 → max_turns; rewrite task description, increase limit
│   └─ ROKO-U-002 → Invalid config; run roko config validate
│
└─ error_code starts with ROKO-S → Safety violation
    └─ Check violation type; adjust policy or system_prompt
```

---

## See also

- [01-error-taxonomy.md](01-error-taxonomy.md) — full error class × recovery matrix
- [02-recovery-strategies.md](02-recovery-strategies.md) — retry, circuit-break, escalate, fail
- [07-forensic-replay.md](07-forensic-replay.md) — deep event log analysis
- [08-observability.md](08-observability.md) — metrics and alerts reference

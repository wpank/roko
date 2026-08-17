# Dogfood Debrief: 2026-08-17

> End-to-end retest of roko's self-hosting workflow, verifying fixes from the 2026-08-13 dogfood session.

## Executive Summary

**The plan-execute-gate-persist loop works end-to-end when the model is forced.**
The scheduler deadlock from 2026-08-13 is fixed. Config slug conflicts from `roko init`
are no longer hit (existing config reused). `core.fsmonitor` and stale-snapshot blockers
are confirmed resolved. However, **two new issues prevent a fully hands-free run**:

1. The cascade router selects models that aren't configured with a usable provider
   (selects `claude-opus` via `anthropic_api` when only `claude_cli` is configured)
2. Plan generation (`prd plan`) produces malformed TOML and the retry/escalation path
   fails to instantiate the fallback model

**With `--force-model claude-sonnet`, the pipeline completes successfully**: agent
dispatches, writes code, gates pass (compile + structural verify), knowledge is
distilled, and the plan is marked complete.

## Test Scenario

**Approach:** Reuse existing workspace (no `roko init`) to test the realistic workflow
where a developer already has a configured project.

**Chosen task:** "Add a `roko doctor network` subcommand that probes configured LLM
provider endpoints for connectivity and latency."

**Fallback task:** `demo-hello` (1-task smoke test) used to test `plan run` after
`prd plan` failed.

## Timeline

| Step | Command | Time | Outcome |
|---|---|---|---|
| 0 | `cargo build --release` | 10m 40s | ✓ Binary built |
| 1 | `roko prd idea "..."` | <1s | ✓ Idea captured |
| 2 | `roko prd draft new "doctor-network-probe"` | 3m 14s | ✓ Draft generated (1 warning: missing Repository Grounding) |
| 3 | `roko prd list` | <1s | ✓ Draft visible |
| 4 | `roko prd plan doctor-network-probe` | 5m 1s | ✗ **Agent exit code 1 (25 bytes output)** |
| 4a | (retry with RUST_LOG debug) | 2m 38s | ✗ **TOML extraction failed; escalation to opus failed** |
| 5 | `roko plan run plans/demo-hello --fresh` | 9s | ✗ **Cascade router selected claude-opus, no API key** |
| 5a | `roko plan run ... --force-model claude-sonnet` | 254s | ✓ **Plan complete: 1/1 tasks, $0.47** |
| 6 | `roko status` | <1s | ✓ Health: ready, run passed |

**Total wall time:** ~25 minutes of commands + troubleshooting
**Furthest progress:** Full plan completion with --force-model

## Issues Found

### 1. CRITICAL: Cascade router selects unconfigured model/provider combinations

**Symptom:** `failed to create agent for 'claude-opus': Missing API key: env var
ANTHROPIC_API_KEY not set`

**Root cause:** The cascade router's learned state (`cascade-router.json`) contains 14
model slugs learned from prior sessions, including `claude-opus`. For the `implementer`
role, it selects `claude-opus` which would need the `anthropic_api` provider. But the
workspace only has `claude-sonnet` configured with `claude_cli` provider.

**Why the 2026-08-14 fix is insufficient:** The fix made ACP adaptive selection opt-in
via `ROKO_ACP_CASCADE_SELECT=1`, but the runner-v2 cascade router is a *separate code
path* that still routes freely across all learned models regardless of which providers
are actually configured and have valid credentials.

**Impact:** Without `--force-model`, no plan can run in this workspace.

**Fix needed:** The cascade router must filter its candidate set to only models that
have a configured, credential-ready provider. If a selected model can't be instantiated,
it should fall back to the configured default rather than failing.

**Files:** `crates/roko-cli/src/dispatch/model_routing.rs`, cascade router candidate
selection, agent factory `create_agent` error path.

### 2. HIGH: Plan generation produces unparseable TOML

**Symptom:** `prd plan` agent returns raw Rust code mixed into TOML output. The
extracted ````toml` block contains `pub struct NetworkProbeCheck {` — not valid TOML.

**Root cause:** The Claude CLI agent (sonnet via claude_cli) wrote both TOML task
definitions and Rust code examples *inside* the toml fence block. The TOML extraction
logic found the block but its content is invalid.

**Escalation failure:** When TOML parsing fails, the system tries to retry with
`claude-opus-4-6`. But it fails instantly with `create agent for model claude-opus-4-6`
— the same provider-configuration issue as #1. The retry never actually runs.

**Impact:** Cannot generate plans from PRDs in this workspace.

**Fix needed:**
1. TOML extraction should strip non-TOML content (Rust code blocks) before parsing
2. Escalation should fall back to the configured provider, not assume `anthropic_api`
3. The retry prompt should more explicitly instruct "only TOML, no code examples"

**Files:** `crates/roko-cli/src/prd.rs:1330-1340` (extraction), agent prompt template

### 3. MEDIUM: Unknown config key warnings persist

**Symptom:**
```
WARN roko_cli::config: unknown config field 'gate' was ignored
WARN roko_cli::config: unknown config field 'isfr' was ignored
```

**Root cause:** The `roko.toml` has `[gate]` and `[isfr]` sections that the config
loader doesn't recognize. These are different from the `[github]`/`[resources]` sections
that were fixed in 2026-08-13.

**Impact:** Noisy warnings on every command. Not blocking.

**Fix:** Either add these to the config schema or remove them from `roko.toml`.

### 4. LOW: Dream consolidation timeout

**Symptom:** `dream consolidation timed out — skipping timeout_secs=120`

**Root cause:** Post-plan dream consolidation takes >120s and hits the timeout. This
runs after a successful plan completion.

**Impact:** Non-blocking — the plan still succeeds. But dream-based learning from the
run is lost.

**Fix:** Either increase the timeout or make dream consolidation async/background.

### 5. LOW: First `prd plan` attempt exited with code 1 (25 bytes)

**Symptom:** First `prd plan` attempt produced only 25 bytes and exit code 1. Second
attempt (identical command) produced 20KB and exit code 0.

**Root cause:** Unclear — possibly a transient Claude CLI error, rate limit, or network
issue. The 25 bytes were likely an error message from the CLI itself.

**Impact:** Non-deterministic failure on first attempt. The retry succeeded.

## Verified Fixes from 2026-08-13

| Issue | Status | Verified |
|---|---|---|
| Config/model slug ambiguity on init | **Fixed** | Not re-tested (used existing config) |
| Scheduler deadlock after AgentCompleted | **Fixed** | ✓ Agent→Gate→Complete transition works |
| Stale snapshot blocking fresh runs | **Fixed** | ✓ `--fresh` works, auto-detect not tested |
| core.fsmonitor breaks worktrees | **Fixed** | ✓ No fsmonitor errors in any run |
| Cascade router ignoring configured model | **Partially fixed** | ✗ ACP path fixed, runner-v2 path still broken |
| Unknown config key warnings | **Partially fixed** | ✗ `github`/`resources` fixed, `gate`/`isfr` new |
| --bare equivalent | **Fixed** | Not directly tested |

## Performance Observations

| Operation | Time | Notes |
|---|---|---|
| Release binary build | 10m 40s | Warm-ish build (most deps cached) |
| `roko prd idea` | <1s | Fast |
| `roko prd draft` | 3m 14s | 194s agent time via claude_cli |
| `roko prd list` | <1s | Fast |
| `roko prd plan` (attempt 1) | 5m 1s | Failed — agent exit code 1 |
| `roko prd plan` (attempt 2) | 2m 38s | Succeeded agent call, failed TOML parse |
| Cargo cache warm | 22.6s | Incremental build pre-check |
| Agent dispatch (demo-hello) | 19.5s | claude-sonnet via claude_cli |
| Gate pipeline (compile + verify) | 224s | Dominated by cargo compile check |
| Plan verify | 0.6s | Structural check only |
| Dream consolidation | >120s | Timed out |
| Post-plan cleanup | 7s | Worktree removal |
| `roko status` | <1s | Fast |

**Total successful plan run (demo-hello):** 254s event loop + 138s reporting = 392s wall

## Comparison with 2026-08-13 Dogfood

| Factor | 2026-08-13 | 2026-08-17 | Change |
|---|---|---|---|
| Init conflicts | ✗ Slug ambiguity | Not hit (existing config) | **Improved** |
| PRD draft | ✓ Works | ✓ Works | Same |
| PRD plan | ✓ Generated plan | ✗ TOML extraction fails | **Regressed** |
| Plan run startup | ✗ 5 manual fixes | ✗ 1 fix (--force-model) | **Improved** |
| Agent dispatch | ✓ Works | ✓ Works | Same |
| Agent→Gate transition | ✗ Deadlocked | ✓ Works | **Fixed** |
| Gate execution | Never reached | ✓ Passes | **Fixed** |
| Plan completion | Never reached | ✓ Completes | **Fixed** |

## Recommendations (Priority Order)

### P0: Cascade router must respect configured providers
The runner-v2 cascade router selects models that have no configured, credential-ready
provider. It must filter candidates to models that can actually be instantiated.
Fallback should go to `default_model` in config, not to a hardcoded model.

### P1: Fix plan generation TOML extraction
Either strip non-TOML content from fenced blocks, or make the agent prompt more
explicit about output format. The escalation retry path must also use a configured
provider.

### P2: Clean up unknown config keys
Add `[gate]` and `[isfr]` to the schema or remove them from the default `roko.toml`.

### P3: Make dream consolidation non-blocking
Either increase the 120s timeout or run it asynchronously so it doesn't delay the
plan completion report.

## Raw Logs

All command outputs captured in `tmp/dogfood-2026-08-17/`:
- `00-prd-idea.log`: Idea capture (succeeded)
- `01-prd-draft.log`: PRD draft generation (succeeded, 3m 14s)
- `02-prd-list.log`: PRD listing (succeeded)
- `03-prd-plan.log`: Plan generation attempt 1 (failed, agent exit 1)
- `04-prd-plan-retry.log`: Plan generation attempt 2 (agent succeeded, TOML parse failed)
- `05-plan-run-demo.log`: Plan run without --force-model (failed, no API key for opus)
- `06-plan-run-force-model.log`: Plan run with --force-model (succeeded, full pipeline)
- `07-status.log`: Status check (succeeded)

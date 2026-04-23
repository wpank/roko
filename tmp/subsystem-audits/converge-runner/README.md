# Converge Runner — Runtime Convergence Batch Execution

## What This Is

The **converge runner** is an automated batch execution system that applies 87
targeted code transformations to the roko codebase. Each "batch" is a focused
code change executed by OpenAI's Codex (`gpt-5.5`), verified by `cargo check`,
and committed atomically. The purpose is to converge roko's three separate
runtime implementations (orchestrate.rs monolith, ACP pipeline, Runner v2) into
a single unified `WorkflowEngine` backed by foundation-style services.

This was run as a follow-up to the **arch runner** (see `tmp/runners/arch/`),
which created the foundation types and services. The converge runner wires those
services into live code paths, adds tests, and retires legacy code.

## Key Files & Locations

| What | Path |
|------|------|
| Runner script | `tmp/runners/converge/run-converge.sh` |
| Batch definitions | `tmp/runners/converge/BATCHES.md` |
| Per-batch prompts | `tmp/runners/converge/prompts/*.prompt.md` |
| Run logs | `tmp/runners/converge/logs/run-20260428-045041/` |
| Status TSV | `tmp/runners/converge/logs/run-20260428-045041/status.tsv` |
| Per-batch results | `tmp/runners/converge/logs/run-20260428-045041/*.result` |
| Converge branch | `codex/converge-run-20260428-045041` |
| Worktree | `.roko/worktrees/converge-run-20260428-045041/` |

## How the Runner Works

1. **Runner script** (`run-converge.sh`) reads `BATCHES.md` to get the ordered
   list of 87 batches with their titles, write scopes, and dependencies.

2. For each batch:
   - Checks if dependencies have `success` result files — if not, marks `blocked`
   - Feeds the batch prompt from `prompts/<BATCH>.prompt.md` to `codex exec`
     (OpenAI Codex with `gpt-5.5` model)
   - Codex writes code changes to the specified file scope
   - Runner verifies: structural checks (grep for expected patterns) + `cargo check`
   - On success: `git commit` with message `converge(<BATCH>): <title>`
   - On failure: retries up to 3 times, then marks `verify_failed`

3. **`--continue last`** flag: resumes from the last run, skipping batches that
   already have a `success` result file.

4. All events are logged to `status.tsv` (tab-separated: timestamp, batch,
   attempt, event, details).

## The 13-Track Taxonomy

The 87 batches are organized into 13 tracks by subsystem:

| Track | Count | Focus |
|-------|-------|-------|
| **F** (Foundation) | 6 | Fix crate cycle between roko-core and roko-runtime, unify duplicated traits |
| **S** (Services) | 13 | Make ModelCallService, PromptAssemblyService, FeedbackService, GateService production-ready |
| **E** (Engine) | 8 | PipelineStateV2 config loading, checkpoint/resume, EffectDriver real agent spawn/commit/save |
| **W** (Wiring) | 8 | Connect WorkflowEngine to `roko run`, `roko plan run`, ACP bridge, roko-serve |
| **O** (Observability) | 6 | JsonlLogger, RuntimeProjection, StateHub bridge, CLI progress printer |
| **R** (Retirement) | 5 | Feature-gate orchestrate.rs behind `legacy-orchestrate` cargo feature |
| **C** (CLI/Demo) | 12 | Clack-style CLI output, `--share` flag, agent list formatting, dashboard SPA |
| **T** (Tests) | 5 | Integration tests for WorkflowEngine, CLI flags, share endpoint |
| **D** (Daimon) | 4 | Extract AffectPolicy trait, implement DaimonPolicy, wire to EffectDriver |
| **G** (Gateway) | 9 | Unify 5 provider abstractions into ModelCallService, gateway event writer |
| **K** (Knowledge) | 5 | Knowledge-aware CascadeRouter routing, knowledge injection in prompts |
| **X** (Security) | 2 | Fail-closed contracts (X01), stream JSON parser consolidation (X02) |
| **L** (Layering) | 4 | Layer metadata in Cargo.toml, layer-check binary, cargo-deny, CI enforcement |

Full batch definitions with dependencies are in `tmp/runners/converge/BATCHES.md`.

## Run Results (2026-04-28)

**Run ID:** `run-20260428-045041`
**Duration:** ~6 hours (04:50 → ~15:35)
**Result:** 83 success, 1 success_noop, 4 failed (R-track)

### Failed Batches

| Batch | Result | Why |
|-------|--------|-----|
| **R02** | `verify_failed` | Feature-gating orchestrate.rs (21K LOC monolith) behind `legacy-orchestrate` — too complex for single Codex batch |
| **R03** | `blocked` | Depends on R02 |
| **R04** | `blocked` | Depends on R03 |
| **R05** | `blocked` | Depends on R04 |

The R-track failure is expected — feature-gating a 21K-line file with deep
cross-crate dependencies is beyond what a single Codex prompt can handle.
This will need to be done manually or split into smaller sub-batches.

### Success Summary by Track

| Track | Succeeded | Failed | Total |
|-------|-----------|--------|-------|
| F | 6 | 0 | 6 |
| S | 13 | 0 | 13 |
| E | 8 | 0 | 8 |
| W | 8 | 0 | 8 |
| O | 6 | 0 | 6 |
| R | 1 | 4 | 5 |
| C | 12 | 0 | 12 |
| T | 5 | 0 | 5 |
| D | 4 | 0 | 4 |
| G | 9 | 0 | 9 |
| K | 5 | 0 | 5 |
| X | 2 | 0 | 2 |
| L | 4 | 0 | 4 |
| **Total** | **83** | **4** | **87** |

## Post-Run Integration

### Pre-merge compile fixes (commit `db0df777`)

After the converge runner completed, `cargo check --workspace` failed across
the full workspace (the runner only verified individual crates per-batch). A
previous session fixed 23 files:

- **tokio::sync::Mutex vs std::sync::Mutex**: Several files used `std::sync::Mutex`
  guards across `.await` points, which is `!Send`. Fixed by switching to
  `tokio::sync::Mutex` where needed.
- **`#[path]` state_hub module**: `state_hub.rs` is included in both roko-cli and
  roko-serve via `#[path = "../roko-core/src/state_hub.rs"]`. This creates
  structurally identical but distinct types. Import references had to be updated
  to use the local `crate::state_hub::*` path.
- **Missing fields/imports**: Various compile errors from new fields added in one
  batch but not propagated to all usage sites.

### Merge into wp-arch2

The converge branch (`codex/converge-run-20260428-045041`) was merged into
`wp-arch2` via fast-forward (the merge base was wp-arch2's HEAD, so all converge
commits applied cleanly on top).

One conflict existed in `demo/demo-app/src/main.tsx` between stashed demo WIP
(AppShell layout, Landing page, dashboard routes) and converge dashboard
additions (CascadeRouter, KnowledgeEntries, ShareView pages). Resolved by
combining both: kept AppShell layout wrapper, added all dashboard sub-routes.

## What Happened Next

After merging, a full audit was conducted across all 13 tracks. The audit found
issues at three severity levels, and critical fixes were applied. See:

- **[AUDIT.md](AUDIT.md)** — Full audit findings (10 critical, 25 warning, 20 note)
- **[FIXES-APPLIED.md](FIXES-APPLIED.md)** — What was fixed and how
- **[OPEN-ISSUES.md](OPEN-ISSUES.md)** — Checklist of remaining issues

## Relationship to Previous Runners

| Runner | Branch | Batches | Purpose |
|--------|--------|---------|---------|
| **Arch runner** | `codex/arch-run-20260428-012508` | 16 (P0A-P4B) | Created foundation types: `RuntimeEvent`, `foundation.rs` traits, `ModelCallService`, `PromptAssemblyService`, `FeedbackService`, `GateService`, `PipelineStateV2`, `EffectDriver`, `WorkflowEngine`, CLI/serve adapters |
| **Converge runner** | `codex/converge-run-20260428-045041` | 87 (F/S/E/W/O/R/C/T/D/G/K/X/L) | Wired foundation services into live paths, added tests, CLI output, dashboard, security hardening, layer enforcement |

Both runners are documented in `tmp/subsystem-audits/INDEX.md` under
"Architecture Runner Status".

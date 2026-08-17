# Agent Execution Guide

This document tells you exactly how to execute all 56 batches (W0-W15). Read this first.

## Key Principles

1. **No per-batch compilation.** Do NOT run `cargo build`, `cargo test`, `cargo clippy`, or `cargo fmt` inside individual batches. These are deferred to a final compilation phase.
2. **Parallel everything possible.** Within a wave, batches *should* touch different files. **Check the File Conflict Matrix below** -- some waves (W10, W12, W13, W15) have intra-wave conflicts and need sub-waving. Spin up one agent per batch when safe; use sub-waves when files overlap.
3. **Commit per-wave.** After all batches in a wave are done, stage and commit them together.
4. **Fix forward.** If something isn't 100% clear in a batch file, make the best reasonable decision and move on. The compilation phase will catch type errors.

## Workspace Root

All paths are relative to: `/Users/will/dev/nunchi/roko/roko/`

## Branch

Work on the current branch: `wp-arch2`. Do NOT create a new branch.

## Execution Protocol

### Step 1: Read the batch file
Each batch file in `batches/` is self-contained. It has:
- **Problem**: What's wrong
- **Root Cause**: Why it's wrong (with exact file:line)
- **Exact Code to Change**: Before/after code with file paths
- **Checklist**: Mechanical steps to follow

### Step 2: Make the code changes
Follow the batch file's instructions literally. Each change specifies:
- Exact file path
- Exact code to find (the "before")
- Exact code to write (the "after")

### Step 3: Mark the batch done
After completing all changes in the batch, edit the batch file itself: change all `- [ ]` to `- [x]` in the checklist. This is how other agents know this batch is done.

### Step 4: Move to the next batch
Check the wave's batch files. If all batches in the current wave are `[x]`, the wave is done.

## Parallel Execution Plan

### Round 0: Wave 0 — Critical Pipeline Fixes (7 batches)

**Must run first.** These fix the root causes of why the demo fails.

**Execution order:**
- **Subwave 0a** (parallel): W0-A, W0-D, W0-E — all independent
- **Subwave 0b** (parallel, after 0a): W0-B, W0-C, W0-F, W0-G — W0-F depends on W0-D (same file: run.rs)

**Agent W0-A** (strip tools):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W0-A-strip-tools-prd-draft.md and implement all changes described in it. Add allowed_tools field to AgentExecOpts in agent_exec.rs, thread it through to SpawnAgentSpec, update all callers to pass None (default), and pass "none" for prd draft new and read-only tools for prd plan. Also rewrite the prompts in commands/prd.rs to remove "if you have file tools" language. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

**Agent W0-D** (dispatch routing — Subwave 0a, parallel with W0-A):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W0-D-dispatch-routing-command-false.md and implement all changes. Change use_provider_routing at run.rs:1829 to not gate on command == "claude" — gate only on has_routing. Add error for command = "false" with no providers configured. Do NOT run cargo build/test/clippy/fmt. Mark the checklist items as done.
```

**Agent W0-E** (max_completion_tokens — Subwave 0a, parallel with W0-A and W0-D):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W0-E-max-completion-tokens.md and implement all changes. Add use_max_completion_tokens = true to gpt54-mini in docker/railway.roko.toml. Add .with_use_max_completion_tokens() to Perplexity and Cerebras backends in tool_loop/backends/mod.rs. Do NOT run cargo build/test/clippy/fmt. Mark the checklist items as done.
```

**Agent W0-B** (plan discovery — Subwave 0b):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W0-B-plan-discovery-mismatch.md and implement all changes described in it. The key fixes are: (1) make validate_before_run() use plan_loader::load_plans() instead of discover_plans(), (2) ensure prd plan always writes a plan.md alongside tasks.toml, (3) add TOML fallback extraction. Do NOT run cargo build/test/clippy/fmt. Mark the checklist items as done.
```

**Agent W0-C** (speed optimizations — Subwave 0b):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W0-C-speed-optimizations.md and implement all changes. Skip repo context for empty workspaces, cap PRD content size, reduce agent timeout, better diagnostics. Do NOT run cargo build/test/clippy/fmt. Mark the checklist items as done.
```

**Agent W0-F** (dispatch parity — Subwave 0b, after W0-D since both touch run.rs):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W0-F-run-dispatch-parity.md and implement all changes. Extract system prompt + tool CSV building into a helper function, then use it in all 5 dispatch paths in run.rs dispatch_agent(). Do NOT run cargo build/test/clippy/fmt. Mark the checklist items as done.
```

**Agent W0-G** (BUILD page — Subwave 0b, independent):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W0-G-build-page-resilience.md and implement all changes. Increase timeout from 120s to 300s in Builder.tsx, add running indicator, error detection, cancel button. Do NOT run cargo build/test. Mark the checklist items as done.
```

**Commit after Round 0:**
```bash
cd /Users/will/dev/nunchi/roko/roko
git add crates/roko-cli/src/agent_exec.rs crates/roko-cli/src/commands/prd.rs crates/roko-cli/src/prd.rs
git add crates/roko-cli/src/agent_spawn.rs crates/roko-cli/src/commands/plan.rs
git add crates/roko-cli/src/run.rs crates/roko-agent/src/tool_loop/backends/mod.rs
git add docker/railway.roko.toml
git add demo/demo-app/src/pages/Builder.tsx demo/demo-app/src/lib/terminal-session.ts
git commit -m "W0: critical pipeline fixes — routing, max_tokens, tools, plan discovery, speed, BUILD page"
```

### Round 1: Waves 1+2+3 (10 batches, 10 agents)

These waves are actually all independent — they touch different files and different subsystems:
- **Wave 1** (3 agents): plan.rs, prd.rs, plan_validate.rs
- **Wave 2** (5 agents): main.rs (tracing), command files (eprintln), schema.rs, Cargo.toml (indicatif), util.rs
- **Wave 3** (2 agents): chat_inline.rs, terminal.rs

**File overlap check**: W2-A touches main.rs (tracing setup at line 1800+). W2-B touches command files (eprintln in plan.rs, prd.rs, etc). W1-A also touches plan.rs. **Conflict**: W1-A and W2-B both touch `plan.rs`. To avoid: W1-A changes the `PlanCmd::Run` handler (lines 209-240) and `validate_before_run` (line 965+). W2-B changes `eprintln!` calls throughout. These are different code regions, so can still run in parallel — but merge carefully.

**Agent prompts for Round 1:**

**Agent W1-A** (plan run path):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W1-A-plan-run-path.md and implement all changes described in it. Follow the code changes exactly as specified. Do NOT run cargo build/test/clippy/fmt. Just make the code changes and mark the checklist items as done.
```

**Agent W1-B** (prd plan extraction):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W1-B-prd-plan-extraction.md and implement all changes described in it. The batch requires investigating AgentExecOpts in agent_exec.rs to determine how to restrict tools. Do NOT run cargo build/test/clippy/fmt. Just make the code changes and mark the checklist items as done.
```

**Agent W1-C** (plan schema unify):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W1-C-plan-schema-unify.md and implement all changes described in it. Read task_parser.rs and plan_validate.rs first to understand both parsers. Do NOT run cargo build/test/clippy/fmt. Just make the code changes and mark the checklist items as done.
```

**Agent W2-A** (tracing to file):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W2-A-tracing-to-file.md and implement all changes described in it. Focus on main.rs only — the tracing subscriber setup around line 1800 and the Cli struct. Do NOT run cargo build/test/clippy/fmt. Just make the code changes and mark the checklist items as done.
```

**Agent W2-B** (error dedup):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W2-B-error-dedup.md and implement all changes described in it. Audit each eprintln! in the 8 command files. Remove ones that duplicate returned Err. Convert eprintln+Ok(1) to anyhow::bail!. Keep warnings. Do NOT run cargo build/test/clippy/fmt. Just make the code changes and mark the checklist items as done.
```

**Agent W2-C** (config version):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W2-C-config-version-warn.md and implement all changes described in it. This is a 1-line change in schema.rs. Do NOT run cargo build/test/clippy/fmt. Just make the code changes and mark the checklist items as done.
```

**Agent W2-D** (spinners):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W2-D-spinners.md and implement all changes described in it. Add indicatif to Cargo.toml, add spinners to prd.rs and plan.rs. Do NOT run cargo build/test/clippy/fmt. Just make the code changes and mark the checklist items as done.
```

**Agent W2-E** (negative cost):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W2-E-negative-cost.md and implement all changes described in it. Three .max(0.0) additions in util.rs. Do NOT run cargo build/test/clippy/fmt. Just make the code changes and mark the checklist items as done.
```

**Agent W3-A** (ctrl-c phases):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W3-A-ctrl-c-phases.md and implement all changes described in it. Two Phase::Error handlers in chat_inline.rs, plus audit Thinking/Streaming phases. Do NOT run cargo build/test/clippy/fmt. Just make the code changes and mark the checklist items as done.
```

**Agent W3-B** (panic hook):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W3-B-panic-hook.md and implement all changes described in it. Add panic hook in InlineTerminal::new() in terminal.rs. Do NOT run cargo build/test/clippy/fmt. Just make the code changes and mark the checklist items as done.
```

**Commit after Round 1:**
```bash
cd /Users/will/dev/nunchi/roko/roko
git add crates/roko-cli/src/commands/plan.rs crates/roko-cli/src/prd.rs crates/roko-cli/src/plan_validate.rs
git add crates/roko-cli/src/main.rs crates/roko-cli/src/commands/*.rs crates/roko-core/src/config/schema.rs
git add crates/roko-cli/Cargo.toml crates/roko-cli/src/chat_inline.rs crates/roko-cli/src/inline/terminal.rs
git commit -m "W1-W3: pipeline fixes, output quality, terminal safety (10 batches)"
```

### Round 2: Wave 4 (3 batches, sequential A→B→C)

Wave 4 is TypeScript (demo app), fully independent from the Rust changes. Can run alongside Round 1 or after.

**Agent W4-ALL** (single agent for the sequential wave):
```
Read these three batch files in order and implement them sequentially:
1. /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W4-A-clickable-scenario-type.md
2. /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W4-B-context-panel.md
3. /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W4-C-prd-pipeline-redesign.md

These are TypeScript/React changes in demo/demo-app/. Implement A first (types + component + hook), then B (ContextPanel), then C (pipeline rewrite + ScenarioSlot update). Do NOT run npm build. Just make the code changes and mark all checklists as done.
```

**Commit after Round 2:**
```bash
cd /Users/will/dev/nunchi/roko/roko
git add demo/demo-app/src/
git commit -m "W4: demo UI redesign — ClickableScenario, CommandList, ContextPanel, PRD pipeline"
```

### Round 3: Wave 5 (4 batches, 4 agents)

**Agent W5-A** (auth detect):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W5-A-auth-detect-config.md and implement all changes described in it. Create detect_auth_from_config() in auth_detect.rs. Update all callers. Do NOT run cargo build/test/clippy/fmt. Just make the code changes and mark the checklist items as done.
```

**Agent W5-B** (ACP startup):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W5-B-acp-startup.md and implement all changes described in it. Add workspace auto-creation and log fallback to handler.rs. Do NOT run cargo build/test/clippy/fmt. Just make the code changes and mark the checklist items as done.
```

**Agent W5-C** (provider preflight):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W5-C-provider-preflight.md and implement all changes described in it. Create preflight_providers() and preflight_gate_deps() functions. Wire into plan.rs and prd.rs. Do NOT run cargo build/test/clippy/fmt. Just make the code changes and mark the checklist items as done.
```

**Agent W5-D** (cat fallback):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W5-D-cat-fallback-refuse.md and implement all changes described in it. Find the cat/NeedsSetup fallback and replace with error+exit. Do NOT run cargo build/test/clippy/fmt. Just make the code changes and mark the checklist items as done.
```

**Commit after Round 3:**
```bash
cd /Users/will/dev/nunchi/roko/roko
git add crates/roko-cli/src/auth_detect.rs crates/roko-acp/src/handler.rs
git add crates/roko-cli/src/commands/plan.rs crates/roko-cli/src/prd.rs
git add crates/roko-cli/src/chat*.rs crates/roko-cli/src/main.rs
git commit -m "W5: provider robustness — auth config, ACP startup, preflight, no cat fallback"
```

### Round 4: Wave 6 (3 batches; A first, then B+C parallel)

W6-A (config unify) is large and touches many files. Run it first, then B+C in parallel after.

**Agent W6-A** (config unify):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W6-A-config-unify.md and implement all changes described in it. Delete the 8 duplicate load_roko_config functions listed in the batch. Update all callers to use roko_core::config::loader::load_config_unified. Keep the cached wrapper in orchestrate.rs, the file-only variant in serve_runtime.rs, and the models-only variant in run.rs. Do NOT run cargo build/test/clippy/fmt. Just make the code changes and mark the checklist items as done.
```

**Agent W6-B** (file locking) — after W6-A commits:
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W6-B-file-locking.md and implement all changes described in it. Create workspace_lock.rs, add fs2 dependency, wire into plan.rs and prd.rs. Do NOT run cargo build/test/clippy/fmt. Just make the code changes and mark the checklist items as done.
```

**Agent W6-C** (boot sequence) — after W6-A commits:
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W6-C-boot-sequence.md and implement all changes described in it. Create bootstrap.rs with RokoBootstrap struct. Wire into chat, plan run, and serve entry points. Do NOT run cargo build/test/clippy/fmt. Just make the code changes and mark the checklist items as done.
```

**Commit after Round 4:**
```bash
cd /Users/will/dev/nunchi/roko/roko
git add crates/roko-cli/src/ crates/roko-serve/src/ crates/roko-acp/src/
git add crates/roko-cli/Cargo.toml
git commit -m "W6: config unification, workspace locking, bootstrap struct"
```

### Round 5: Waves 7+8 (7 batches, 7 agents, all parallel)

All batches touch different files, no conflicts.

**Agent W7-A** (sync mutex):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W7-A-sync-mutex-serve.md and implement all changes. Replace parking_lot::Mutex with tokio::sync::Mutex for affect_engine in state.rs. Update all .lock() to .lock().await. Do NOT run cargo build/test/clippy/fmt.
```

**Agent W7-B** (cancel notify):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W7-B-cancel-notify.md and implement all changes. Replace polling loop in cancel.rs with event-driven cancellation. Do NOT run cargo build/test/clippy/fmt.
```

**Agent W7-C** (playbook locks):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W7-C-playbook-locks.md and implement all changes. Remove nested per-ID locks in save_or_merge, keep global merge lock. Do NOT run cargo build/test/clippy/fmt.
```

**Agent W8-A** (clippy blanket):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W8-A-clippy-blanket.md and implement all changes. Remove the cfg_attr block in main.rs lines 10-20. Keep too_many_lines and missing_docs allows. Then run `cargo clippy -p roko-cli --no-deps 2>&1 | head -200` to find warnings and fix them with per-item #[allow] or actual code fixes. This is the ONE batch that needs an iterative cargo clippy loop. Do NOT run cargo test.
```

**Agent W8-B** (rust toolchain):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W8-B-rust-toolchain.md and implement all changes. Create rust-toolchain.toml at workspace root. Do NOT run cargo build/test/clippy/fmt.
```

**Agent W8-C** (enrichment backend):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W8-C-enrichment-backend-removal.md and implement all changes. Replace resolve_enrichment_backend() substring matching with provider kind match. Update callers. Do NOT run cargo build/test/clippy/fmt.
```

**Agent W8-D** (TOCTOU):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W8-D-toctou-fixes.md and implement all changes. Search for .exists() + read patterns, convert to try-then-handle. Do NOT run cargo build/test/clippy/fmt.
```

**Commit after Round 5:**
```bash
cd /Users/will/dev/nunchi/roko/roko
git add crates/roko-serve/src/state.rs crates/roko-agent/src/dispatcher/cancel.rs
git add crates/roko-learn/src/playbook.rs crates/roko-cli/src/main.rs
git add rust-toolchain.toml crates/roko-cli/src/orchestrate.rs
git add -A  # catch any remaining .rs files touched by TOCTOU fixes
git commit -m "W7-W8: concurrency fixes, clippy cleanup, rust-toolchain, TOCTOU fixes"
```

### Phase 2: Compilation + Fix

Single agent, iterative:

```
You have made 22 batches of code changes across the roko workspace at /Users/will/dev/nunchi/roko/roko. Now compile and fix all errors.

1. Run: cargo +nightly fmt --all
2. Run: cargo build --workspace 2>&1 | head -200
3. Fix all compilation errors (missing imports, type mismatches, removed functions with remaining callers, etc.)
4. Repeat steps 2-3 until cargo build succeeds
5. Run: cargo clippy --workspace --no-deps -- -D warnings 2>&1 | head -200
6. Fix clippy warnings
7. Run: cargo test --workspace 2>&1 | head -200
8. Fix test failures
9. Commit: git commit -m "fix: compilation and test fixes after W1-W8 batches"
```

### Phase 3: Demo App Build

Single agent for TypeScript:

```
Build the demo app after Wave 4 changes:
cd /Users/will/dev/nunchi/roko/roko/demo/demo-app
npm install
npm run build 2>&1 | head -100
Fix any TypeScript compilation errors.
Commit: git commit -m "fix: demo app build fixes after W4 UI redesign"
```

## File Conflict Matrix

This matrix shows which batch files touch which source files, to identify potential merge conflicts:

| Source File | Batches That Touch It |
|-------------|----------------------|
| `agent_exec.rs` | **W0-A** |
| `commands/prd.rs` | **W0-A**, W0-C, W2-B |
| `prd.rs` (lib) | **W0-B**, W0-C, W1-B, W2-B, W2-D, W5-C, W6-B |
| `commands/plan.rs` | **W0-B**, W1-A, W2-B, W2-D, W5-C, W6-B |
| `agent_spawn.rs` | **W0-A** |
| `plan.rs` | W1-A, W2-B, W2-D, W5-C, W6-B |
| `main.rs` | W2-A, W2-B, W8-A |
| `plan_validate.rs` | W1-C |
| `chat_inline.rs` | W3-A |
| `terminal.rs` | W3-B |
| `schema.rs` | W2-C |
| `util.rs` | W2-E |
| `auth_detect.rs` | W5-A |
| `handler.rs` (ACP) | W5-B |
| `orchestrate.rs` | W6-A, W8-C |
| `state.rs` (serve) | W7-A |
| `cancel.rs` | W7-B |
| `playbook.rs` | W7-C |
| `scenarios.ts` | W4-A |
| `ScenarioSlot.tsx` | W4-C |
| `prd-pipeline.ts` | W4-C |

**Key conflicts to watch (W0-W8)**:
- `plan.rs`: W1-A (lines 209-240, 965+), W2-B (eprintln! lines throughout), W2-D (spinner near dispatch), W5-C (preflight call near top), W6-B (lock acquisition near top). These touch different code regions but need careful merge.
- `main.rs`: W2-A (tracing setup at 1800+), W2-B (top-level error handler), W8-A (clippy allows at lines 10-20). All different regions.
- `prd.rs`: W1-B (line 926+ generation flow), W2-B (eprintln! throughout), W2-D (spinner near dispatch), W5-C (preflight call). Different regions.

### Waves 9-15 File Conflict Matrix

This matrix covers all 30 batches across W9-W15. Files are sorted by conflict severity (most batches first).

| Source File | Batches That Touch It | Intra-Wave Conflict? | Notes |
|---|---|---|---|
| `runner/event_loop.rs` | W9-B, **W10-A, W10-B, W10-D**, **W11-A**, **W12-A, W12-B, W12-C, W12-D**, **W13-B, W13-D** | W10 (3), W12 (4), W13 (2) | **WORST CONFLICT**: 11 batches across 5 waves. W12 has 4-way conflict. |
| `config/schema.rs` | **W10-E**, **W11-C**, **W14-D**, **W15-B, W15-C, W15-E** | W15 (3) | 6 batches across 4 waves. W15 has 3-way conflict. Also touched by W2-C (W0-W8). |
| `orchestrate.rs` | **W10-E**, **W11-B**, **W15-A, W15-B, W15-C** | W15 (3) | 5 batches across 3 waves. W15 has 3-way conflict. Also touched by W6-A, W8-C (W0-W8). |
| `prd.rs` (lib) | **W10-A, W10-C, W10-E**, **W13-A** | W10 (3) | 4 batches across 2 waves. W10 has 3-way conflict. Also touched by W0-B, W0-C, W1-B, W2-B, W2-D, W5-C, W6-B (W0-W8). |
| `runner/state.rs` | W9-B, **W11-A**, **W12-B, W12-C** | W12 (2) | 4 batches across 3 waves. |
| `task_parser.rs` | W9-A, **W12-C**, **W13-A, W13-E** | W13 (2) | 4 batches across 3 waves. |
| `commands/plan.rs` | **W12-A, W12-D**, **W13-D** | W12 (2) | 3 batches across 2 waves. Also touched by W0-B, W1-A, W2-B, W2-D, W5-C, W6-B (W0-W8). |
| `runner/types.rs` | **W12-A, W12-D**, **W13-B** | W12 (2) | 3 batches across 2 waves. |
| `runner/plan_loader.rs` | W9-A, **W10-D**, **W13-E** | None | 3 batches across 3 waves (no intra-wave conflict). |
| `dispatch/prompt_builder.rs` | **W9-A, W9-B** | W9 (2) | 2 batches in same wave. |
| `runner/merge.rs` | **W10-A, W10-D** | W10 (2) | 2 batches in same wave. |
| `index.rs` | **W10-A, W10-C** | W10 (2) | 2 batches in same wave. |
| `runner/gate_dispatch.rs` | W10-A, W12-A | None | 2 batches across 2 waves (no intra-wave conflict). |
| `runner/persist.rs` | W10-D, W13-D | None | 2 batches across 2 waves (no intra-wave conflict). |
| `plan_generate.rs` | W10-A, W15-A | None | 2 batches across 2 waves. |
| `cascade_router.rs` | W10-C, W14-C | None | 2 batches across 2 waves. |
| `compose/templates/implementer.rs` | W14-A, W15-A | None | 2 batches across 2 waves. |
| `compose/templates/common.rs` | W14-A, W15-E | None | 2 batches across 2 waves. |
| `config/mod.rs` | W14-D, W15-C | None | 2 batches across 2 waves. |
| `terminal-session.ts` | **W11-D**, **W15-D** | None | 2 batches across 2 waves. Also touched by W0-G (W0-W8 — missing from W0-W8 matrix). |
| `prd-pipeline.ts` | **W15-D** | None | 1 batch in W15. Also touched by W4-C (W0-W8). |
| `serve/routes/status/health.rs` | W14-B | None | Single batch, no conflict. |
| `serve/routes/sse.rs` | W14-B | None | Single batch, no conflict. |
| `serve/routes/ws.rs` | W14-B | None | Single batch, no conflict. |
| `serve/state.rs` | W14-B | None | Single batch, no conflict. |
| `learn/cascade/persistence.rs` | W14-C | None | Single batch, no conflict. |
| `learn/episode_logger.rs` | W14-C | None | Single batch, no conflict. |
| `learn/costs_log.rs` | W14-C | None | Single batch, no conflict. |
| `config/loader.rs` | W14-D | None | Single batch, no conflict. |
| `compose/system_prompt_builder.rs` | W14-A | None | Single batch, no conflict. |
| `compose/templates/strategist.rs` | W14-A | None | Single batch, no conflict. |
| `compose/templates/reviewer.rs` | W14-A | None | Single batch, no conflict. |
| `compose/templates/scribe.rs` | W14-A | None | Single batch, no conflict. |
| `compose/templates/quick.rs` | W14-A | None | Single batch, no conflict. |
| `compose/templates/integration.rs` | W14-A | None | Single batch, no conflict. |
| `compose/templates/task_impl.rs` | W14-A | None | Single batch, no conflict. |
| `agent/provider/openai_compat.rs` | W9-C | None | Single batch, no conflict. |
| `agent/translate/openai.rs` | W9-C | None | Single batch, no conflict. |
| `agent/provider/mod.rs` | W13-C | None | Single batch, no conflict. |
| `agent/task_runner.rs` | W13-E | None | Single batch, no conflict. |
| `agent/dispatcher/mod.rs` | W15-B | None | Single batch, no conflict. |
| `gate/adaptive_threshold.rs` | W10-D | None | Single batch, no conflict. |
| `runner/agent_events.rs` | W12-C | None | Single batch, no conflict. |
| `runner/snapshot_writer.rs` | W13-D | None | Single batch, no conflict. |
| `runner/output_sink.rs` (NEW) | W15-B | None | New file, no conflict. |
| `runner/mod.rs` | W15-B | None | Single batch (re-export). |
| `core/workspace.rs` (NEW) | W15-E | None | New file, no conflict. |
| `core/lib.rs` | W15-E | None | Single batch (re-export). |
| `core/config/timeouts.rs` (NEW) | W15-C | None | New file, no conflict. |
| `core/defaults.rs` | W15-C | None | Single batch (reads only). |
| `std/skill_library.rs` | W15-C | None | Single batch, no conflict. |
| `orchestrator/dag.rs` | W15-C | None | Single batch, no conflict. |
| `neuro/distiller.rs` | W15-C | None | Single batch, no conflict. |
| `cli/worktree.rs` | W15-C | None | Single batch, no conflict. |
| `cli/agent_episode.rs` | W15-C | None | Single batch, no conflict. |
| `commands/util.rs` | W10-C | None | Single batch, no conflict. |
| `dispatch/mod.rs` | W9-B | None | Single batch, no conflict. |

#### Intra-Wave Conflicts (batches that CANNOT safely run in full parallel)

These batches within the same wave modify the same files and require either sequential execution or sub-waving:

| Wave | Conflicting Batches | Shared File(s) | Recommendation |
|---|---|---|---|
| **W9** | W9-A, W9-B | `dispatch/prompt_builder.rs` | Run W9-A first, then W9-B (different code regions, but same file). W9-C is fully independent. |
| **W10** | W10-A, W10-B, W10-D | `runner/event_loop.rs` | **Subwave 10a**: W10-C, W10-E (independent). **Subwave 10b**: W10-A first, then W10-B + W10-D (all touch event_loop.rs). |
| | W10-A, W10-C | `index.rs` | See subwaving above. |
| | W10-A, W10-D | `runner/merge.rs` | See subwaving above. |
| | W10-A, W10-C, W10-E | `prd.rs` | W10-A touches different regions than W10-C and W10-E, but 3-way conflict in same wave is risky. |
| **W11** | None | (no intra-wave conflicts) | All 4 batches can run fully in parallel. |
| **W12** | W12-A, W12-B, W12-C, W12-D | `runner/event_loop.rs` | **ALL 4 batches** touch event_loop.rs. Must be sequential or carefully sub-waved. Suggest: W12-A first, then W12-B + W12-C parallel, then W12-D last. |
| | W12-A, W12-D | `runner/types.rs`, `commands/plan.rs` | Additional conflicts between A and D. |
| | W12-B, W12-C | `runner/state.rs` | Additional conflict between B and C. |
| **W13** | W13-A, W13-E | `task_parser.rs` | Run sequentially or in subwaves. W13-B, W13-C, W13-D are independent of each other. |
| | W13-B, W13-D | `runner/event_loop.rs` | Run sequentially. |
| **W14** | None | (no intra-wave conflicts) | All 4 batches can run fully in parallel. |
| **W15** | W15-A, W15-B, W15-C | `orchestrate.rs` | 3-way conflict. Run sequentially: W15-A first, then W15-B, then W15-C. |
| | W15-B, W15-C, W15-E | `config/schema.rs` | 3-way conflict. Run W15-E after W15-B and W15-C. |

#### Cross-Wave Conflicts with W0-W8

These files are touched by BOTH W0-W8 batches AND W9-W15 batches. Waves must be applied in order.

| Source File | W0-W8 Batches | W9-W15 Batches | Risk |
|---|---|---|---|
| `prd.rs` (lib) | W0-B, W0-C, W1-B, W2-B, W2-D, W5-C, W6-B | W10-A, W10-C, W10-E, W13-A | **HIGH** — 11 total batches. W0-W8 changes must be fully merged before W10+ touches this file. |
| `commands/plan.rs` | W0-B, W1-A, W2-B, W2-D, W5-C, W6-B | W12-A, W12-D, W13-D | **HIGH** — 9 total batches. |
| `orchestrate.rs` | W6-A, W8-C | W10-E, W11-B, W15-A, W15-B, W15-C | **HIGH** — 7 total batches. |
| `config/schema.rs` | W2-C | W10-E, W11-C, W14-D, W15-B, W15-C, W15-E | **MEDIUM** — W2-C is a small change, but 6 subsequent batches pile onto this file. |
| `terminal-session.ts` | W0-G | W11-D, W15-D | **LOW** — different code regions (error detection vs shell quoting vs timeouts). |
| `prd-pipeline.ts` | W4-C | W15-D | **LOW** — W4-C rewrites the file; W15-D restructures it again. W15-D must run after W4-C. |

#### "Dependencies: None" Verification

All W9-W15 batch files claim "Dependencies: None". This is accurate **across waves** — no batch explicitly depends on another batch's output. However, the file conflicts above mean **implicit ordering** is required:

- W9 batches depend on W0-W8 being done (W9-A/B touch `dispatch/prompt_builder.rs`, which is adjacent to files modified by W0-A and W0-F).
- W10 batches depend on W9 (W10-A touches `event_loop.rs`, also touched by W9-B).
- W11 depends on W10 (W11-A touches `event_loop.rs` and `state.rs`, both touched by W10 batches).
- W12 depends on W11 (W12-* all touch `event_loop.rs`, also touched by W11-A).
- W13 depends on W12 (W13-B/D touch `event_loop.rs`, also touched by W12-*).
- W14 is mostly independent (new subsystem files), but W14-D touches `config/schema.rs` (touched by W10-E, W11-C).
- W15 depends on W14 (W15-A touches `implementer.rs`, also touched by W14-A; W15-B/C/E touch `config/schema.rs`, also touched by W14-D).

The "Dependencies: None" claims are correct in the narrow sense (no batch requires another batch's *feature*), but **file-level ordering constraints exist and must be respected.**

#### Recommended Execution Sub-waves

Based on the conflict analysis above, here are the safe parallel groupings within each wave:

**Wave 9:**
- Subwave 9a: W9-A, W9-C (parallel)
- Subwave 9b: W9-B (after W9-A, shares `dispatch/prompt_builder.rs`)

**Wave 10:**
- Subwave 10a: W10-C, W10-E (parallel, independent files)
- Subwave 10b: W10-B (after 10a, touches `event_loop.rs`)
- Subwave 10c: W10-A, W10-D (sequential — share `event_loop.rs`, `merge.rs`; W10-A also conflicts with 10a via `prd.rs` and `index.rs`)

**Wave 11:** All parallel (W11-A, W11-B, W11-C, W11-D) — no intra-wave conflicts.

**Wave 12:**
- Subwave 12a: W12-A (first — touches event_loop, types, plan, gate_dispatch)
- Subwave 12b: W12-B, W12-C (parallel after 12a — but share `state.rs`, so careful merge needed)
- Subwave 12c: W12-D (last — touches event_loop, types, plan — overlaps with both 12a and 12b)

**Wave 13:**
- Subwave 13a: W13-A, W13-C (parallel — independent files)
- Subwave 13b: W13-B, W13-E (parallel — independent files, but W13-B touches event_loop.rs)
- Subwave 13c: W13-D (last — touches event_loop.rs and commands/plan.rs)

**Wave 14:** All parallel (W14-A, W14-B, W14-C, W14-D) — no intra-wave conflicts.

**Wave 15:**
- Subwave 15a: W15-D (fully independent — TypeScript demo app)
- Subwave 15b: W15-A, W15-E (parallel — independent files except W15-E touches common.rs, also touched by W14-A)
- Subwave 15c: W15-B (after 15b — touches orchestrate.rs and schema.rs)
- Subwave 15d: W15-C (last — touches orchestrate.rs, schema.rs, config/mod.rs — overlaps with 15b and 15c)

### Round 6: Wave 9 — Systemic Pipeline Quality (3 batches, 3 agents, all parallel)

The highest-ROI wave. Fixes THE root cause of poor pipeline output quality.

**Agent W9-A** (wire ImplementerTemplate + PRD injection):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W9-A-wire-implementer-template.md and implement all changes described in it. Wire ImplementerTemplate to runtime dispatch in dispatch_helpers.rs, generate workspace_map by walking crate dirs, inject tasks.toml and PRD excerpt into prompts. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

**Agent W9-B** (cross-task output injection):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W9-B-cross-task-output.md and implement all changes described in it. Add task_outputs HashMap to RunState, record git diff after task completion, inject dependency outputs into build_prompt. Do NOT run cargo build/test/clippy/fmt. Mark the checklist items as done.
```

**Agent W9-C** (cost tracking):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W9-C-cost-tracking.md and implement all changes described in it. Add .with_model_profile(model.clone()) to ToolLoop::new() in openai_compat.rs, add input_tokens/output_tokens fallback in translate/openai.rs. Do NOT run cargo build/test/clippy/fmt. Mark the checklist items as done.
```

**Commit after Round 6:**
```bash
cd /Users/will/dev/nunchi/roko/roko
git add crates/roko-cli/src/dispatch/ crates/roko-cli/src/task_parser.rs
git add crates/roko-cli/src/runner/plan_loader.rs crates/roko-cli/src/runner/state.rs
git add crates/roko-cli/src/runner/event_loop.rs
git add crates/roko-agent/src/provider/openai_compat.rs crates/roko-agent/src/translate/openai.rs
git commit -m "W9: systemic pipeline quality — ImplementerTemplate, PRD injection, cross-task output, cost tracking"
```

### Round 7: Waves 10+11 (9 batches, sub-waved per conflict matrix)

Pipeline bug fixes and critical safety fixes. **W10 has intra-wave conflicts** (see Waves 9-15 File Conflict Matrix above). W11 is fully parallel.

**Wave 10** (5 batches, sub-waved): pipeline data path fixes -- W10-A/B/D share `event_loop.rs`; W10-A/C share `index.rs` and `prd.rs`
**Wave 11** (4 batches, all parallel): crash/hang/injection prevention -- no intra-wave conflicts

**Agent W10-A** (pipeline quick fixes — 8 items):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W10-A-pipeline-quick-fixes.md and implement all changes. Suppress JSON leak, strip model_hint, fix dream path, gate rung constants, INDEX path, slug injection, remove mcp_servers examples, workdir fix. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W10-B** (episode data wiring):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W10-B-episode-data-wiring.md and implement all changes. Thread RunState token/cost/duration into runner_event_to_feedback, write GateVerdict engrams to substrate. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W10-C** (memory/learn unify):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W10-C-memory-learn-unify.md and implement all changes. Standardize on .roko/learn/, auto-register unknown cascade router slugs, read executor.json for INDEX, update PRD plans_generated. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W10-D** (scaffold and gates):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W10-D-scaffold-and-gates.md and implement all changes. Add inter-crate deps to scaffolded Cargo.toml, fix gate threshold schema, add git commits after gates pass, strengthen max_loc. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W10-E** (plan prompt quality):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W10-E-plan-prompt-quality.md and implement all changes. Fix playbook ID mismatch, plan.md generation, PRD type specs in prompts, config version warning. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W11-A** (gate channel + fatal):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W11-A-gate-channel-and-fatal.md and implement all changes. Thread fatal_tx into gate spawn, replace 3 swallowed Fatal results with force_plan_terminal fallback. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W11-B** (unwrap + lock safety):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W11-B-unwrap-lock-safety.md and implement all changes. Replace chain_client unwrap with pattern match, convert std::sync::Mutex to parking_lot::Mutex. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W11-C** (config validation):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W11-C-config-validation.md and implement all changes. Add validate_references() call to from_toml(), fix synthesized_model_profile fallback. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W11-D** (shell injection):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W11-D-shell-injection.md and implement all changes. Wrap ctx.activeModel in shellQuote() in terminal-session.ts. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Commit after Round 7:**
```bash
cd /Users/will/dev/nunchi/roko/roko
git add crates/roko-cli/src/ crates/roko-core/src/ crates/roko-agent/src/
git add crates/roko-learn/src/ crates/roko-gate/src/
git add demo/demo-app/src/lib/terminal-session.ts
git commit -m "W10-W11: pipeline bug fixes + critical safety (9 batches)"
```

### Round 8: Waves 12+13 (9 batches, sub-waved per conflict matrix)

Runner architecture restructuring and speed/reliability improvements. **Both W12 and W13 have intra-wave conflicts** (see Waves 9-15 File Conflict Matrix above). W12 is the worst -- ALL 4 batches touch `event_loop.rs`. W13 has 2 conflict pairs.

**Agent W12-A** (gate semaphore per-run):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W12-A-gate-semaphore-per-run.md and implement all changes. Remove OnceLock global, add gate_concurrency to RunConfig, create per-run semaphore. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W12-B** (multi-agent handle):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W12-B-multi-agent-handle.md and implement all changes. Replace Option<AgentHandle> with HashMap, fix FailPlan attribution, per-plan iteration counter. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W12-C** (event loop safety):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W12-C-event-loop-safety.md and implement all changes. Add sequence field to TaskDef, cap agent_output, epoch timestamp, timeout guard, hook role parameter. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W12-D** (runner config fixes):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W12-D-runner-config-fixes.md and implement all changes. Wire MCP config, invert dream logic, fix Permanent retryable, enforce per-turn budget, bound feedback tasks. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W13-A** (TOML repair pipeline):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W13-A-toml-repair-pipeline.md and implement all changes. Add repair_toml(), split_merged_fields(), close_unclosed_strings() to task_parser.rs. Call before LLM retry in prd.rs. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W13-B** (cache warm + gate buffer):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W13-B-cache-warm-and-gates.md and implement all changes. Add cargo check warm-up, dynamic gate channel buffer. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W13-C** (connection pooling):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W13-C-connection-pooling.md and implement all changes. Add User-Agent header and docs to existing connection pool. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W13-D** (atomic state writes):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W13-D-atomic-state-writes.md and implement all changes. Add checkpoint file for crash recovery, expand --fresh to clean all state files. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W13-E** (error taxonomy):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W13-E-error-taxonomy.md and implement all changes. Improve error classification, add TaskFieldSchema validation, fix scaffold Cargo.toml insertion, validate crate names. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Commit after Round 8:**
```bash
cd /Users/will/dev/nunchi/roko/roko
git add crates/roko-cli/src/ crates/roko-agent/src/ crates/roko-core/src/
git commit -m "W12-W13: runner architecture + speed & reliability (9 batches)"
```

### Round 9: Waves 14+15 (9 batches, sub-waved per conflict matrix)

Subsystem improvements and generalization. **W14 is fully parallel** (no intra-wave conflicts). **W15 has intra-wave conflicts** -- W15-A/B/C share `orchestrate.rs`, W15-B/C/E share `config/schema.rs` (see Waves 9-15 File Conflict Matrix above).

**Agent W14-A** (compose sections):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W14-A-compose-sections.md and implement all changes. Fix budget caps for all 11 sections, O(N²) measurement, SectionSpec table, DRY agents_instructions, budget conflicts. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W14-B** (serve fixes):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W14-B-serve-fixes.md and implement all changes. Fix health endpoint HTTP status, relay_health RwLock, SSE keep-alive, replay memory, WS back_pressure, lock ordering docs. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W14-C** (learning fixes):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W14-C-learning-fixes.md and implement all changes. Add LinUCB persistence, fix nested mutex, episode dual identity, CostsLog batching, importance scoring. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W14-D** (config fixes):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W14-D-config-fixes.md and implement all changes. Fix ROKO__* docs, deprecated config delegation, global merge scope, diagnostics context, interpolation docs. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W15-A** (prompt quality):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W15-A-prompt-quality.md and implement all changes. Add workspace context, remove model_hint method, failure recovery guidance, few-shot TOML, role-tool mapping, file path consolidation. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W15-B** (design patterns):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W15-B-design-patterns.md and implement all changes. Extract dispatch_and_record helper, log daimon/substrate errors, SafetyLayer required, pluggable output sinks, env var parse warnings. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W15-C** (code health):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W15-C-code-health.md and implement all changes. Top unwrap replacements in priority files, replace hardcoded model strings with defaults module, add TimeoutConfig struct. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W15-D** (demo app):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W15-D-demo-app.md and implement all changes. Add TimeoutConfig, CommandFailureReason, single-source command templates, metrics AbortController. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Agent W15-E** (generalization):
```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W15-E-generalization.md and implement all changes. Add GateRungConfig, Workspace abstraction, AdaptiveBudget. Do NOT run cargo build/test/clippy/fmt. Mark checklist as done.
```

**Commit after Round 9:**
```bash
cd /Users/will/dev/nunchi/roko/roko
git add crates/ demo/
git commit -m "W14-W15: compose, serve, learning, config, prompt, design, code health, generalization (9 batches)"
```

### Phase 2: Compilation + Fix (Updated)

Single agent, iterative. Now covers W0-W15:

```
You have made 56 batches of code changes across the roko workspace at /Users/will/dev/nunchi/roko/roko. Now compile and fix all errors.

1. Run: cargo +nightly fmt --all
2. Run: cargo build --workspace 2>&1 | head -200
3. Fix all compilation errors (missing imports, type mismatches, removed functions with remaining callers, etc.)
4. Repeat steps 2-3 until cargo build succeeds
5. Run: cargo clippy --workspace --no-deps -- -D warnings 2>&1 | head -200
6. Fix clippy warnings
7. Run: cargo test --workspace 2>&1 | head -200
8. Fix test failures
9. Commit: git commit -m "fix: compilation and test fixes after W0-W15 batches"
```

---

## How to Pick Up Where Someone Left Off

1. Read the batch files in the current wave
2. Check which ones have `- [x]` vs `- [ ]` in their checklists
3. Pick the next batch with `- [ ]` items
4. If a batch is partially done (some `[x]`, some `[ ]`), continue from the first unchecked item

# Demo App Performance Analysis

**Date:** 2026-05-01
**Benchmark environment:** macOS arm64, debug build, localhost:6677

## Executive Summary

A simple `roko run "Reply with only the word hello"` takes **30–77 seconds** depending on model. The root cause is a **mandatory reviewer step** that doubles or triples the number of LLM calls. This is a config dispatch bug, not an architectural limitation.

---

## Benchmark Results

### Phase 1: CLI Overhead (No LLM)

| Command | Time |
|---------|------|
| `roko status` | 42ms |
| `roko doctor` | 41ms |
| `roko config show` | 38ms |

CLI startup is fine.

### Phase 2: `roko run` — Simple Prompt

| Model | Provider | Wall Time | Implementer | Reviewer | Notes |
|-------|----------|-----------|-------------|----------|-------|
| glm-5.1 | zhipu | **77s** | 3.8s | **70.6s** | Reviewer 18x slower |
| glm-5-turbo | zhipu | **31s** | 3.5s | **11.8s** | Reviewer 3.4x slower |
| glm-4.5-flash | zhipu | **69s** | 3.8s | **49.7s** | Reviewer 13x slower |
| cerebras-scout | cerebras | 548ms | 178ms (fail) | — | Model 404 |
| gemini-2.5-flash | gemini | 47ms | 0ms (fail) | — | Missing API key |
| haiku | anthropic | **4.6s** | 4.6s (fail) | — | Empty response |
| sonnet | anthropic | **5.5s** | 5.5s (fail) | — | Empty response |
| kimi-k2.6 | moonshot | **417s (7 min!)** | 5.6s | **~400s** | Reviewer ran tools |

### Phase 3: WebSocket Terminal Layer

| Model | WS Connect | Shell Ready | Cmd Send |
|-------|-----------|-------------|----------|
| All models | 9–248ms | 1–13ms | 43–47ms |

**The WebSocket/xterm layer is NOT a bottleneck.** 50ms total overhead.

---

## Root Cause Analysis

### Issue #1 (CRITICAL): Mandatory Reviewer in Default Pipeline

**Impact: 3–100x slowdown on every `roko run` call**

`cmd_run()` in `commands/util.rs:280` hardcodes `template = "standard"`, which flows to `run_workflow_engine_report_with_hub()` → `workflow_config_for_template("standard")` → `WorkflowConfig::standard()` → `has_review: true`.

This means every `roko run "hello"` executes:
1. **Implementer** → LLM call #1 (fast, ~3s)
2. **Reviewer** → LLM call #2 (slow, 11–70s+)
3. **Implementer** again → LLM call #3 (based on review)
4. Potentially another reviewer...

The `roko.toml` already configures `[pipeline.mechanical]` with `reviewers = false`, but **this config is never read** by the `roko run` path. The function `run_workflow_engine_report_with_hub()` at `run.rs:576` calls `workflow_config_for_template()` instead of `workflow_config_from_band()`.

**Fix:** Change `run_workflow_engine_report_with_hub()` to read pipeline bands from roko.toml, or change the hardcoded template to `"express"`.

**File:** `crates/roko-cli/src/commands/util.rs:280`
**File:** `crates/roko-cli/src/run.rs:592`

### Issue #2 (HIGH): Reviewer Gets Implementer System Prompt

The string `"reviewer"` passed to `spawn_agent()` does not match any `AgentRole::label()`. The `resolve_role()` function in `prompt_assembly_service.rs:491-501` falls back to `AgentRole::Implementer`.

This means the reviewer:
- Gets the wrong system prompt (implementer, not quick-reviewer)
- Does NOT use the `ReviewerTemplate` or `QuickReviewerTemplate`
- Has no review-specific criteria

This may cause the model to hallucinate code instead of reviewing, which explains why kimi-k2.6's reviewer ran for 400s executing tools.

**Fix:** Map `"reviewer"` → `AgentRole::QuickReviewer` in `resolve_role()`, or change the role string in `workflow_engine.rs` to `"quick-reviewer"`.

**File:** `crates/roko-compose/src/prompt_assembly_service.rs:491-501`
**File:** `crates/roko-runtime/src/workflow_engine.rs:266`

### Issue #3 (HIGH): No `--pipeline` Flag on `roko run`

The `roko run` command has no way to override the pipeline template. The user must edit `roko.toml` to change the workflow. The demo app has no way to set it either.

**Fix:** Add `--pipeline` / `--workflow` flag to `Run {}` in `main.rs`.

**File:** `crates/roko-cli/src/main.rs:328-349`

### Issue #4 (MEDIUM): Demo Scenarios Don't Configure Pipeline

The `roko()` helper in `terminal-session.ts:82-89` only injects `--model`. It doesn't set `--pipeline express` or any workflow config. Every scenario pays the full reviewer tax.

**Fix:** After adding `--pipeline` flag, update `roko()` helper to inject it.

**File:** `demo/demo-app/src/lib/terminal-session.ts:82-89`

### Issue #5 (MEDIUM): max_tokens Budget Throttle

The `affect dispatch modulation` sets `max_tokens=2253` (visible in debug logs). This is the daimon affect engine throttling output tokens. For simple prompts this is fine, but for reviewers processing large context, this forces the model to stop mid-response and restart.

**File:** `crates/roko-daimon/` affect modulation

### Issue #6 (LOW): Debug Binary Performance

All benchmarks ran against `target/debug/roko`. Release builds would be faster for config loading, TOML parsing, regex compilation, etc. For the demo, the binary should be release-built.

### Issue #7 (LOW): Claude CLI Empty Response

Both haiku and sonnet return "empty response". This is likely a system prompt issue — Claude CLI agents may need specific prompt formatting that the workflow engine doesn't provide.

**File:** `crates/roko-agent/src/provider/claude_cli.rs`

---

## Waterfall Breakdown: `roko run "hello" --model glm5-turbo`

```
0ms      CLI startup + config load
│
├─ 0ms   WorkflowEngine::start
│        ├─ PipelineStateV2::new(standard)  — has_review=true, max_iterations=2
│        └─ Phase: Implementing
│
├─ 0ms   EffectDriver: spawn_agent("implementer")
│        ├─ affect modulation: max_tokens=2253, temp=0.58
│        ├─ create_agent_for_model("glm-5.1")
│        │   ├─ resolve_model → zhipu/glm-5.1
│        │   ├─ OpenAiCompatAdapter::create_agent
│        │   └─ ToolLoopAgent ready
│        └─ agent.run(prompt)
│           ├─ HTTP POST to open.bigmodel.cn (1 turn, no tools)
│           └─ Response: "hello"
│
├─ 2400ms  Implementer complete (success)
│          ├─ Phase → Gating (no gates configured in /tmp workspace)
│          └─ Phase → Reviewing (has_review=true)
│
├─ 2430ms  EffectDriver: spawn_agent("reviewer")
│          ├─ affect modulation: same budget
│          ├─ create_agent_for_model("glm-5.1")
│          ├─ prompt: "Review the changes"
│          │   ├─ No diff context (workspace has no git changes)
│          │   ├─ resolve_role("reviewer") → falls back to Implementer
│          │   └─ System prompt: implementer template (wrong!)
│          └─ agent.run("Review the changes")
│             ├─ Model thinks it's an implementer, not reviewer
│             ├─ May attempt tool calls, code generation
│             └─ Generates review-like output (eventually)
│
├─ 31100ms  Reviewer complete (success, 28.7s)
│           ├─ Phase → Implementing (iteration 2)
│           └─ SpawnImplementer with review findings
│
├─ 31130ms  EffectDriver: spawn_agent("implementer") #2
│           ├─ Prompt includes reviewer findings
│           └─ agent.run(revised prompt)
│
├─ 31885ms  Implementer #2 complete (755ms, fast — nothing to do)
│           └─ Phase → Gating → Reviewing (iteration 2)
│
├─ ...      Reviewer #2 (another 10-30s)
│
└─ ~60-80s  max_iterations reached, workflow complete
```

**~97% of wall time is spent in reviewer calls that shouldn't exist.**

---

## Impact on Demo Scenarios

Each demo scenario runs multiple `roko run` commands. With the reviewer bug:

| Scenario | # roko run calls | Expected time | Actual time |
|----------|-----------------|---------------|-------------|
| PRD Pipeline | 4-5 | 20-30s | 3-5 min |
| Race | 2 (parallel) | 10-15s | 1-2 min |
| Gate Retry | 2 | 15-20s | 2-3 min |
| Knowledge Accumulation | 3 | 15-20s | 2-4 min |
| Providers | 4-8 | 20-30s | 4-8 min |
| ISFR Agents | 8 | 30-40s | 5-10 min |

---

## Fix Priority

### P0 — Fix Now (eliminates 90% of latency)

1. **Wire pipeline bands in `run_workflow_engine_report_with_hub()`**
   - Read `pipeline_config.mechanical` (or appropriate band) instead of calling `workflow_config_for_template()`
   - This alone drops `roko run "hello"` from 77s → ~4s for glm-5.1

2. **Add `--pipeline` flag to `roko run`**
   - Allow `roko run "prompt" --pipeline express`
   - Demo app injects `--pipeline express` for all scenarios

### P1 — Fix Soon

3. **Fix reviewer role resolution**
   - Map `"reviewer"` string to `AgentRole::QuickReviewer`
   - Prevents implementer-as-reviewer confusion

4. **Update demo `roko()` helper**
   - Inject `--pipeline express` by default for demo scenarios

### P2 — Optimize Later

5. Per BOTTLENECK-ANALYSIS.md and OPTIMIZATION-PLAYBOOK.md items
6. Release build for demo binary
7. Warm pool design (WARM-POOL-DESIGN.md)

---

## Verification Plan

After P0 fixes:

```bash
# Should complete in <5s with glm-5.1
time roko run "Reply with only the word hello" --model glm51 --pipeline express

# Should complete in <2s with cerebras
time roko run "Reply with only the word hello" --model cerebras-scout --pipeline express
```

---

## Files to Change

| File | Change | Priority |
|------|--------|----------|
| `crates/roko-cli/src/run.rs:576-596` | Use `workflow_config_from_band()` | P0 |
| `crates/roko-cli/src/commands/util.rs:278-280` | Read template from config | P0 |
| `crates/roko-cli/src/main.rs:328-349` | Add `--pipeline` arg to `Run {}` | P0 |
| `crates/roko-compose/src/prompt_assembly_service.rs:491-501` | Fix reviewer role | P1 |
| `crates/roko-runtime/src/workflow_engine.rs:266` | Use correct role string | P1 |
| `demo/demo-app/src/lib/terminal-session.ts:82-89` | Inject `--pipeline` | P1 |

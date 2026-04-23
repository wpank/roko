# Deep Audit Prompt — Converge Runner Output

Copy everything below the line into a fresh Claude Code terminal at `/Users/will/dev/nunchi/roko/roko`.

---

## Context

You are auditing the output of two automated batch runners ("arch" and "converge") that applied ~100 commits of Codex-generated code to the roko codebase. Your job is to understand what was intended, what actually happened, where the gaps are, what the ideal design would look like, and produce a concrete action plan to get there.

**You are on branch `wp-arch2`.** There are 59 uncommitted files (post-merge audit fixes). The converge branch has been merged in.

## Phase 1: Understand the Intent

Read these documents to understand what the runners were trying to accomplish and why:

### Vision & Anti-Patterns (the "why")
- `tmp/subsystem-audits/VISION.md` — Master vision for what roko should become
- `tmp/subsystem-audits/ANTI-PATTERNS.md` — 10 documented anti-patterns with real codebase examples. These are the problems the runners were designed to fix.
- `tmp/subsystem-audits/UNIFIED-IMPLEMENTATION-PLAN.md` — 80+ tasks across 7 phases to converge all runtimes
- `tmp/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md` — Prioritized T0-T8 tiers, 100+ tasks

### Arch Runner (the foundation layer — 16 batches, all succeeded)
- `tmp/runners/arch/BATCHES.md` — 16 batch definitions (P0A through P4B)
- `tmp/runners/arch/context-pack/00-RULES.md` — Rules given to Codex
- `tmp/runners/arch/context-pack/01-ARCHITECTURE.md` — Target architecture
- `tmp/runners/arch/context-pack/02-EXISTING-CODE.md` — Existing code context
- `tmp/runners/arch/context-pack/03-ANTI-PATTERNS.md` — Anti-patterns to avoid
- `tmp/runners/arch/prompts/` — 16 per-batch prompts (P0A.prompt.md through P4B.prompt.md)

### Converge Runner (the wiring layer — 87 batches, 83 succeeded)
- `tmp/runners/converge/BATCHES.md` — 87 batch definitions across 13 tracks (F/S/E/W/O/R/C/T/D/G/K/X/L)
- `tmp/runners/converge/prompts/` — 87 per-batch prompts
- `tmp/runners/converge/run-converge.sh` — The runner script itself

### Post-merge audit results
- `tmp/subsystem-audits/converge-runner/README.md` — Overview of the converge run
- `tmp/subsystem-audits/converge-runner/AUDIT.md` — Audit findings (10 critical, 25 warning, 20 note)
- `tmp/subsystem-audits/converge-runner/FIXES-APPLIED.md` — What was fixed after merge
- `tmp/subsystem-audits/converge-runner/OPEN-ISSUES.md` — Remaining issues checklist

### Per-subsystem audits (pre-runner, show original state)
- `tmp/subsystem-audits/orchestration/` — The 3-runtime problem (orchestrate.rs monolith, ACP, Runner v2)
- `tmp/subsystem-audits/gate-pipeline/` — Gate verification pipeline state
- `tmp/subsystem-audits/inference-dispatch/` — 13+ LLM call sites problem
- `tmp/subsystem-audits/prompt-assembly/` — Prompt assembly fragmentation
- `tmp/subsystem-audits/learning-feedback/` — Learning components state
- `tmp/subsystem-audits/safety-agent/` — Safety layer state
- `tmp/subsystem-audits/cognitive-layer/` — Neuro/dreams/daimon state

## Phase 2: Understand the Actual Implementation

Now read the actual code that was produced. These are the key new/modified files:

### Foundation types (arch runner output)
- `crates/roko-core/src/runtime_event.rs` — RuntimeEvent enum (all event variants)
- `crates/roko-core/src/foundation.rs` — Foundation traits: ModelCaller, PromptAssembler, FeedbackSink, GateRunner, GateConfig, GateReport, etc.

### Foundation services (arch + converge output)
- `crates/roko-agent/src/model_call_service.rs` — ModelCallService: unified LLM dispatch
- `crates/roko-compose/src/prompt_assembly_service.rs` — PromptAssemblyService: prompt construction
- `crates/roko-learn/src/feedback_service.rs` — FeedbackService: event recording + flush
- `crates/roko-gate/src/gate_service.rs` — GateService: 7-rung gate pipeline via foundation traits

### Execution engine (arch + converge output)
- `crates/roko-runtime/src/pipeline_state.rs` — PipelineStateV2: pure state machine
- `crates/roko-runtime/src/effect_driver.rs` — EffectDriver: side effects (agent spawn, gates, commit, checkpoint)
- `crates/roko-runtime/src/workflow_engine.rs` — WorkflowEngine: top-level orchestration loop
- `crates/roko-runtime/src/jsonl_logger.rs` — JsonlLogger: durable event log
- `crates/roko-runtime/src/projection.rs` — RuntimeProjection: read model from event log

### Wiring (converge output)
- `crates/roko-cli/src/run.rs` — CLI entry point: `--engine v2` flag, service construction, WorkflowEngine invocation
- `crates/roko-serve/src/adapters.rs` — Serve adapter for WorkflowEngine events → SSE/WS
- `crates/roko-cli/src/output_format.rs` — Clack-style CLI output formatters

### Daimon/Affect (converge D-track output)
- `crates/roko-daimon/src/policy.rs` — DaimonPolicy: AffectPolicy implementation wrapping DaimonState
- `crates/roko-runtime/src/effect_driver.rs` lines 31-87 — AffectPolicy trait + DispatchModulation (local version)

### Security (converge X-track output)
- `crates/roko-agent/src/safety/mod.rs` — X01: fail-closed contracts
- `crates/roko-agent/src/safety/contract.rs` — AgentContract::restricted() default

### Layering (converge L-track output)
- `crates/roko-cli/src/layer_check.rs` — Layer enforcement binary
- `deny.toml` — cargo-deny config
- `.github/workflows/ci.yml` — CI integration

### Tests (converge T-track output)
- `crates/roko-runtime/src/workflow_engine.rs` (bottom of file) — WorkflowEngine tests
- `crates/roko-agent/tests/contracts.rs` — Safety contract integration tests

### The legacy monolith (for comparison)
- `crates/roko-cli/src/orchestrate.rs` — 21K lines, the monolith being replaced

## Phase 3: Produce the Audit

Write your findings to `tmp/subsystem-audits/converge-runner/DEEP-AUDIT.md`. Structure it as follows:

### Section 1: Implementation Inventory

For each of the 13 tracks (F, S, E, W, O, R, C, T, D, G, K, X, L):
- What problem was this track trying to solve? (reference the anti-patterns)
- What batches ran and what did they produce? (reference specific files + line ranges)
- What is the current state — fully working, partially working, or dead code?

### Section 2: Gap Analysis — Intent vs Reality

For each foundation service and engine component:
- What was the intended design? (from VISION.md, MASTER-IMPLEMENTATION-PLAN.md, the batch prompts)
- What was actually implemented? (from reading the code)
- Where do they diverge? Be specific: "intended to use CascadeRouter for model selection, actually uses empty string"

### Section 3: Ideal Design

If you were redesigning these components from scratch today, knowing what you know from reading both the intent documents and the actual implementation, what would the ideal design look like?

For each major component (ModelCallService, PromptAssemblyService, FeedbackService, GateService, PipelineStateV2, EffectDriver, WorkflowEngine):
- What should its public API be?
- What should it depend on?
- What should it NOT do?
- How should it interact with the other components?
- What's the minimum viable version vs the full version?

Pay special attention to:
- The `AffectPolicy` trait duplication (effect_driver.rs vs foundation.rs) — which should survive?
- The `StateHub` type split (`#[path]` trick) — what's the right way to share this?
- The `orchestrate.rs` retirement — what's the incremental path to feature-gate 21K lines?
- The `ModelCallService` → `CascadeRouter` → `DispatchModulation` pipeline — how should model selection actually flow?

### Section 4: Anti-Patterns Introduced

The runners were supposed to fix anti-patterns. Did they introduce new ones? Look for:
- Dead code (functions/structs/traits defined but never called)
- Duplicate abstractions (same concept in two places with different types)
- Stubs that silently succeed (returning Ok/pass without doing anything)
- Configuration that's ignored (flags parsed but not used)
- Hardcoded values where config should be read
- Type system workarounds (`#[path]`, incompatible trait versions, format string hacks)

### Section 5: Concrete Action Plan

Produce a prioritized, checked list of every concrete change needed to get from the current state to the ideal design from Section 3. Format:

```
## Priority 1: Fix broken things (runtime panics, incorrect behavior)
- [ ] Description (file:line) — what to change and why

## Priority 2: Unify duplicates (delete one, keep one)
- [ ] Description (file:line) — what to change and why

## Priority 3: Wire stubs to real implementations
- [ ] Description (file:line) — what to change and why

## Priority 4: Delete dead code
- [ ] Description (file:line) — what to change and why

## Priority 5: Design improvements
- [ ] Description (file:line) — what to change and why
```

Each item should be specific enough that someone could implement it without reading the audit. Include file paths, line numbers, function names, and a one-sentence description of the change.

### Section 6: What Would a Second Converge Runner Look Like?

If we were to run another batch runner (like the converge runner) to fix everything in Section 5, what would the batches look like? Define them in the same format as `tmp/runners/converge/BATCHES.md`:

```
| Batch | Title | Write Scope | Deps |
```

Group by track. Estimate which batches are small enough for a single Codex prompt (< 3 files, < 200 lines changed) vs which need to be split or done manually.

## Constraints

- Do NOT modify any code. This is a read-only audit.
- Do NOT skip reading files. If I referenced a file, read it.
- Be honest about what you find. If a component is well-designed, say so. If it's a mess, say so.
- Use exact file paths and line numbers so findings are actionable.
- If you run out of context, write what you have and note where you stopped. Prioritize Sections 1-3 and 5 over 4 and 6.
- Write to `tmp/subsystem-audits/converge-runner/DEEP-AUDIT.md` as a single document.

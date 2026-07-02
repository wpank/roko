# Design: Next-Generation Task Generation & Execution System

> **The thesis**: The harness delivers 6x performance improvement — the scaffold
> matters more than the model [LEE-2026]. Right context, not more context.
> Right verification, not more verification. Right granularity, not more tasks.

## What went wrong with mori

### Failure mode 1: Tasks too coarse
**Evidence**: Plans like "scaffold golem-eval crate" produced 22 tasks but each task
was still 200-400 lines of code. Agents reimplemented existing code because the task
didn't say "import X from Y", it said "implement X".

**Fix**: Tasks should be ≤50 LOC changes. If a task requires more, it needs decomposition.

### Failure mode 2: Context overload OR starvation
**Evidence**: Enrichment pipeline injected full PRD text (20K+ chars) + decomposition +
brief + invariants. Total context per task: ~30K tokens of "helpful" context, but the
agent only needed 3 specific functions from 2 specific files.

**Fix**: Context should be *precisely targeted* — the exact functions, types, and
interfaces the task touches, nothing else.

### Failure mode 3: Acceptance criteria not executable
**Evidence**: Criteria like "enum ThesisHypothesis defined in hypothesis.rs" — yes,
but is it *imported and used*? Criteria that check file existence don't catch
"built but never wired."

**Fix**: Every acceptance criterion must be a runnable command that exits 0/1.
Criteria must test *integration*, not just existence.

### Failure mode 4: No feedback loop
**Evidence**: When Task T3 failed because T1's output was wrong, T3 just failed.
No information flowed back to explain *why* or *how to fix it*.

**Fix**: Gate failures must produce structured diagnostics that feed back into
the next attempt's context.

### Failure mode 5: Parallel agents can't coordinate
**Evidence**: 24 parallel subagents each built their assigned crate perfectly
but none wired crates together.

**Fix**: "Wiring" tasks that connect modules must be sequential, explicitly
scoped, and have integration tests as acceptance criteria.

---

## The new architecture

### 1. Task granularity tiers (model-adaptive)

```
Tier 0: Mechanical (Haiku / small models, ≤20 LOC)
  - Add an import statement
  - Add a field to a struct
  - Rename a function
  - Add a #[test] that calls an existing function
  → Acceptance: grep + cargo check

Tier 1: Focused (Sonnet, ≤50 LOC)
  - Implement a single function body
  - Wire module A into module B (add import + 1 call site)
  - Write a test that exercises one code path
  → Acceptance: cargo test -p crate -- test_name

Tier 2: Integrative (Sonnet/Opus, ≤150 LOC)
  - Connect 2-3 modules into a working pipeline
  - Implement a trait for an existing type
  - Refactor a function while preserving behavior
  → Acceptance: cargo test + integration test

Tier 3: Architectural (Opus only, ≤300 LOC)
  - Design a new module API from requirements
  - Decompose a complex feature into Tier 0-2 tasks
  → Acceptance: cargo check + API review
```

### 2. Context assembly: "Surgical context injection" [MEMORY-POINTERS-2025]

Instead of dumping full files, generate a *context pack* per task:

```toml
[[task]]
id = "T3"
title = "Wire SystemPromptBuilder into dispatch_agent"
tier = 1
model_hint = "sonnet"  # minimum model for this tier

# SURGICAL CONTEXT: only what the agent needs to see
[task.context]
# Files to read (with line ranges)
read = [
    { path = "crates/roko-cli/src/orchestrate.rs", lines = "540-570", why = "dispatch_agent function to modify" },
    { path = "crates/roko-compose/src/system_prompt_builder.rs", lines = "43-80", why = "SystemPromptBuilder API" },
    { path = "crates/roko-compose/src/lib.rs", lines = "1-40", why = "public exports to import" },
]

# Types/functions the agent must know about (extracted by code intelligence)
symbols = [
    { name = "SystemPromptBuilder::new", signature = "pub fn new(role_identity: impl Into<String>) -> Self" },
    { name = "SystemPromptBuilder::build", signature = "pub fn build(self) -> String" },
    { name = "AgentRole", signature = "pub enum AgentRole { Implementer, Reviewer, Architect, ... }" },
]

# What NOT to do (anti-context from prior failures)
anti_patterns = [
    "Do NOT create a new file. Modify orchestrate.rs only.",
    "Do NOT rewrite SystemPromptBuilder. Import and call it.",
]

# Prior failure context (if retrying)
prior_failures = []
```

### 3. Acceptance criteria: executable verification pipeline

Every task gets a verification pipeline, not just a checklist:

```toml
[task.verify]
# Phase 1: Structural (instant, no compilation)
[[task.verify.structural]]
check = "grep"
pattern = "SystemPromptBuilder"
file = "crates/roko-cli/src/orchestrate.rs"
expect = "present"
fail_msg = "SystemPromptBuilder not imported in orchestrate.rs"

[[task.verify.structural]]
check = "grep"
pattern = "ExecAgent::new"
file = "crates/roko-cli/src/orchestrate.rs"
expect = "absent"
fail_msg = "ExecAgent still used — should be replaced with ClaudeCliAgent"

# Phase 2: Compilation
[[task.verify.compile]]
command = "cargo check -p roko-cli"
timeout_ms = 30000

# Phase 3: Unit tests
[[task.verify.test]]
command = "cargo test -p roko-cli -- test_system_prompt_composed"
timeout_ms = 60000

# Phase 4: Integration (tests that the wiring actually works end-to-end)
[[task.verify.integration]]
command = "cargo test -p roko-cli -- test_dispatch_uses_system_prompt"
timeout_ms = 120000
```

### 4. Self-learning feedback loop

```
Task attempt → Gate result → Structured diagnosis → Context update → Retry

On FAILURE:
  1. Extract compiler error / test failure message
  2. Map error to the specific line/function that caused it
  3. Generate a "fix hint" that goes into the retry's context
  4. If 3 retries fail at same error → escalate to higher-tier model
  5. Log the failure pattern for future tasks (bandit update)

On SUCCESS:
  1. Record: model used, tokens consumed, wall time, attempt count
  2. Update bandit arm weights for this task type + model combo
  3. Extract any new patterns (e.g., "wiring tasks succeed 90% with Opus, 30% with Sonnet")
  4. Store successful context pack as a "skill" for similar future tasks
```

### 5. Task decomposition algorithm

Given a feature description or PRD requirement:

```
1. SCOPE: Identify all files that need to change
   → Use code intelligence (roko-index) to find callers, dependencies
   → Output: file list + line ranges

2. CHUNK: Split changes into ≤50 LOC edits
   → Each chunk touches 1-2 files max
   → Each chunk has a clear before/after

3. ORDER: Build dependency graph
   → Type definitions before implementations
   → Implementations before wiring
   → Wiring before tests
   → Tests before integration tests

4. CLASSIFY: Assign tier + model hint
   → Tier 0 (mechanical): imports, renames, field additions
   → Tier 1 (focused): single function, single test
   → Tier 2 (integrative): multi-module connection
   → Tier 3 (architectural): design decisions

5. CONTEXT: For each task, generate surgical context
   → Read exactly the lines/symbols needed
   → Include anti-patterns from prior failures
   → Include relevant skill snippets from success library

6. VERIFY: For each task, generate executable checks
   → Structural (grep), Compile, Test, Integration
   → Each check is a command that exits 0 or 1
```

### 6. Language-agnostic task format

The task format works across languages by abstracting the verification commands:

```toml
[task.verify.compile]
# Rust
command = "cargo check -p {crate}"
# TypeScript
command = "npx tsc --noEmit --project {project}"
# Go
command = "go build ./{package}/..."
# Python
command = "python -m py_compile {file}"

[task.verify.test]
# Rust
command = "cargo test -p {crate} -- {test_name}"
# TypeScript
command = "npx jest --testPathPattern {test_file}"
# Go
command = "go test ./{package}/... -run {test_name}"
# Python
command = "python -m pytest {test_file}::{test_name}"

[task.verify.lint]
# Rust
command = "cargo clippy -p {crate} --no-deps -- -D warnings"
# TypeScript
command = "npx eslint {file}"
# Go
command = "golangci-lint run ./{package}/..."
# Python
command = "ruff check {file}"
```

The task generator detects language from the project structure and fills in
the right commands.

---

## Implementation: what to build in roko

### Phase 1: Task generator CLI

```bash
# From a PRD document
roko plan generate --from .roko/prd/published/02-agents.md

# From arbitrary text
roko plan generate --prompt "Add rate limiting to the API endpoint"

# From a file (any format)
roko plan generate --from /path/to/requirements.md

# From a checklist section
roko plan generate --from-checklist MORI-PARITY-CHECKLIST.md --section "7.1"
```

### Phase 2: Enrichment pipeline (replaces mori's bardo-enrich.sh)

```bash
roko plan enrich plans/W01-wire-system-prompts/
# Generates:
#   tasks.toml        (with surgical context per task)
#   verify-tasks.toml (executable verification pipeline)
#   invariants.md     (type/state/security/performance invariants)
#   context-packs/    (per-task context files)
```

### Phase 3: Execution with feedback

```bash
roko plan run plans/W01-wire-system-prompts/
# For each task:
#   1. Assemble context from context-packs/
#   2. Select model from tier hint + bandit weights
#   3. Execute task
#   4. Run verification pipeline
#   5. On failure: diagnose, update context, retry
#   6. On success: update bandit, store skill
#   7. Advance to next task
```

### Phase 4: Self-learning

```bash
roko learn                    # analyze all episodes, update bandits
roko learn --report           # show what's working, what isn't
roko learn --optimize         # suggest task decomposition improvements
```

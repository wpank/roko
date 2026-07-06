# Task Format Guide: Writing Tasks for Fresh Agents

## The Constraint

Each task is executed by a freshly spawned agent with **zero prior context**. The agent only knows:
1. Its system prompt (role identity, conventions, anti-patterns)
2. The task definition from `tasks.toml`
3. What it discovers via tools (grep, read_file) — but only if it has tools

The task definition must be **completely self-contained**.

## tasks.toml Schema

```toml
[meta]
plan = "plan-slug"
iteration = 1
total = 4
done = 0
status = "ready"
max_parallel = 2
estimated_total_minutes = 30

[[task]]
id = "T1"
title = "Imperative verb phrase describing outcome"
description = "1-2 sentence elaboration if title isn't enough"
role = "Implementer"                    # Implementer, Reviewer, Researcher, Scribe, Strategist
status = "ready"                        # ready, done, failed
tier = "mechanical"                     # mechanical, focused, integrative, architectural
model_hint = "claude-haiku-4-5"         # cheapest model for this tier
max_loc = 20                            # maximum lines of change
files = ["crates/roko-cli/Cargo.toml"]  # files this modifies
depends_on = []                         # task IDs within this plan
depends_on_plan = []                    # plan IDs this blocks on
timeout_secs = 3600
max_retries = 3

[task.context]
read_files = [
    { path = "crates/roko-cli/Cargo.toml", lines = "1-30", why = "see current dependencies" },
]
symbols = [
    "RoleSystemPromptSpec::new — creates role-scoped system prompt",
]
anti_patterns = [
    "Do NOT modify other crates' Cargo.toml files",
    "Do NOT add features that aren't in the PRD",
]

[[task.verify]]
phase = "structural"
command = "grep -q 'roko-compose' crates/roko-cli/Cargo.toml"
fail_msg = "roko-compose not found in dependencies"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli"
fail_msg = "cargo check failed"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-cli --lib"
fail_msg = "tests failed"
```

## Task Tiers

| Tier | Max LOC | Model | Use For |
|------|---------|-------|---------|
| `mechanical` | ≤20 | `claude-haiku-4-5` | Imports, renames, config changes |
| `focused` | ≤50 | `claude-sonnet-4-6` | Single function, single test |
| `integrative` | ≤150 | `claude-sonnet-4-6` | Multi-module wiring |
| `architectural` | ≤300 | `claude-opus-4-6` | API design, trait definitions |

## Golden Rules

### 1. Smaller is better
A 20 LOC task with clear context will succeed on the first try.
A 200 LOC task with vague context will fail and waste money.

### 2. One semantic change per task
"Add dependency AND wire the import AND update the function" = 3 tasks, not 1.

### 3. Verification must be executable
```toml
# GOOD: machine-checkable
[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli"

# BAD: subjective
acceptance = ["code is clean and well-structured"]
```

### 4. Context must be surgical
```toml
# GOOD: exact location
[task.context]
read_files = [
    { path = "crates/roko-core/src/signal.rs", lines = "35-80", why = "Signal struct definition" }
]

# BAD: entire crate
read_files = [
    { path = "crates/roko-core/src/", why = "understand the crate" }
]
```

### 5. Anti-patterns prevent the most common failures
Based on the MISTAKES-LEARNED.md catalog:
```toml
anti_patterns = [
    "Do NOT reimplement what already exists — grep first",
    "Do NOT use git checkout/switch/branch",
    "Do NOT modify files outside the listed `files` array",
    "Do NOT add error handling for impossible scenarios",
]
```

## Template: Mechanical Rename Task

```toml
[[task]]
id = "T1"
title = "Replace bardo_runtime imports with roko_runtime in roko-cli"
tier = "mechanical"
model_hint = "claude-haiku-4-5"
max_loc = 15
files = ["crates/roko-cli/src/main.rs", "crates/roko-cli/src/orchestrate.rs"]
depends_on = ["T0-rename-crate-dir"]

[task.context]
read_files = [
    { path = "crates/roko-cli/Cargo.toml", lines = "1-40", why = "verify dependency name changed" }
]
symbols = []
anti_patterns = [
    "Do NOT change any logic — rename imports only",
    "Do NOT modify Cargo.toml — that's a separate task",
]

[[task.verify]]
phase = "structural"
command = "! grep -q 'bardo_runtime' crates/roko-cli/src/main.rs"
fail_msg = "old import still present"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli"
```

## Template: Wiring Task (connecting existing code)

```toml
[[task]]
id = "T3"
title = "Call NeuroStore::query during context assembly in orchestrate.rs"
tier = "integrative"
model_hint = "claude-sonnet-4-6"
max_loc = 40
files = ["crates/roko-cli/src/orchestrate.rs"]
depends_on = ["T1-add-neuro-dep", "T2-import-neuro"]

[task.context]
read_files = [
    { path = "crates/roko-neuro/src/lib.rs", lines = "1-50", why = "NeuroStore trait API" },
    { path = "crates/roko-cli/src/orchestrate.rs", lines = "8199-8250", why = "dispatch_agent_with where context is assembled" },
]
symbols = [
    "NeuroStore::query — async fn query(&self, topic: &str) -> Vec<KnowledgeEntry>",
    "dispatch_agent_with — where system prompt and context are assembled",
]
anti_patterns = [
    "Do NOT rewrite the context assembly — ADD neuro query results to the existing flow",
    "Do NOT create a new NeuroStore — use the one already initialized in PlanRunner",
    "Do NOT change the function signature of dispatch_agent_with",
]

[[task.verify]]
phase = "structural"
command = "grep -q 'neuro.*query\\|NeuroStore' crates/roko-cli/src/orchestrate.rs"
fail_msg = "neuro query not found in orchestrate.rs"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-cli --lib -- --ignored 2>/dev/null; cargo test -p roko-cli --lib"
```

## Template: Multi-Crate Breaking Change (use multi-plan)

See [03-REFACTOR-SEQUENCE.md](03-REFACTOR-SEQUENCE.md) Step 6 for the Signal → Engram example.

Key principles:
- **Plan P1**: Change the source of truth (roko-core), add compat alias
- **Plan P2**: Update all consumers (one task per crate, parallel)
- **Plan P3**: Remove compat alias, final verification
- Use `depends_on_plan` between plans, `depends_on` within plans

## Common Failure Modes

| Failure | Cause | Prevention |
|---------|-------|-----------|
| Agent reimplements existing code | Didn't know it existed | Add read_files pointing to existing impl |
| Agent modifies wrong files | Task scope unclear | Explicit `files` array + anti-pattern |
| Agent adds unnecessary error handling | Over-engineering tendency | Anti-pattern: "Do NOT add error handling for impossible scenarios" |
| Agent uses old names | Naming context missing | Include naming glossary in system prompt |
| Agent breaks other crates | Didn't check dependents | Verification: `cargo check --workspace` |
| Task too large to complete | >50 LOC or >2 files | Split into smaller tasks |

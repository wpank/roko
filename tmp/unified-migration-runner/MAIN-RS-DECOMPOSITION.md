# main.rs Decomposition — Implementation Prompt

> **What this is**: Self-contained implementation guide. Each task (M-D001–M-D012)
> can be executed by an independent agent.
>
> **Goal**: Split `crates/roko-cli/src/main.rs` (12,690 lines, 135 functions) into
> focused subcommand modules. Zero behavior change — pure extraction.

## Context

`main.rs` is a monolithic command dispatcher. Every CLI subcommand's logic lives
inline. This makes it hard to find anything, hard to test, and hard to modify.

The file already has some partial extraction (e.g., `plan.rs`, `prd.rs`, `research.rs`
exist as modules) but most logic remains in `main.rs`.

### Files to read first
```
crates/roko-cli/src/main.rs          — the 12,690-line file to decompose
crates/roko-cli/src/lib.rs           — module declarations
crates/roko-cli/src/plan.rs          — existing partial extraction (plan helpers)
crates/roko-cli/src/prd.rs           — existing partial extraction (PRD commands)
```

### Build command
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
```

### Approach
1. Identify each `cmd_*` or `fn cmd_*` function group in main.rs
2. Move each group to a dedicated module file
3. Leave main.rs as a thin dispatcher that calls into modules
4. Preserve all behavior exactly

---

## Tasks

### M-D001 — Map all command handlers in main.rs

**Objective**: Catalog every command handler function, its line range, and subcommand group.

**Steps**:
1. `grep -n 'fn cmd_\|async fn cmd_' crates/roko-cli/src/main.rs` to find all handlers
2. Group by subcommand: plan, prd, agent, config, research, knowledge, learn, job, deploy, serve, daemon, etc.
3. For each group, note: function names, line ranges, approximate LOC
4. Write the catalog to `tmp/refactoring/main-rs-function-map.md`
5. This is the extraction plan for subsequent tasks

**Verification**: catalog file exists, line counts sum to ~12,000

---

### M-D002 — Extract plan subcommands

**Objective**: Move `cmd_plan_*` functions to `crates/roko-cli/src/commands/plan.rs`.

**Steps**:
1. Create `crates/roko-cli/src/commands/` directory and `mod.rs`
2. Create `crates/roko-cli/src/commands/plan.rs`
3. Move all `cmd_plan_*` functions (plan list, plan show, plan create, plan run, plan validate, plan generate, plan regenerate) from main.rs
4. Make functions `pub(crate)` with same signatures
5. In main.rs, replace inline logic with `commands::plan::cmd_plan_list(...)` calls
6. Add `pub mod commands;` to lib.rs

**Verification**:
```bash
cargo check -p roko-cli
cargo run -p roko-cli -- plan list  # smoke test
```

---

### M-D003 — Extract PRD subcommands

**Objective**: Move `cmd_prd_*` functions to `crates/roko-cli/src/commands/prd.rs`.

**Steps**: Same pattern as M-D002 but for PRD commands (idea, draft, plan, list, status, consolidate).

---

### M-D004 — Extract agent subcommands

**Objective**: Move `cmd_agent_*` functions to `crates/roko-cli/src/commands/agent.rs`.

**Steps**: Same pattern. Agent commands: create, start, stop, list, status, serve, chat.

---

### M-D005 — Extract config subcommands

**Objective**: Move `cmd_config_*` functions to `crates/roko-cli/src/commands/config.rs`.

**Steps**: Config commands: init, show, path, edit, set, validate, migrate, set-secret, check-secrets, providers, models, subscriptions, events, experiments, plugins, secrets.

---

### M-D006 — Extract research subcommands

**Objective**: Move `cmd_research_*` functions to `crates/roko-cli/src/commands/research.rs`.

**Steps**: Research commands: topic, search, enhance-prd, plan, tasks, analyze.

---

### M-D007 — Extract knowledge subcommands

**Objective**: Move `cmd_knowledge_*` to `crates/roko-cli/src/commands/knowledge.rs`.

**Steps**: Knowledge commands: query, stats, gc, backup, restore, sync, dream.

---

### M-D008 — Extract learn subcommands

**Objective**: Move `cmd_learn_*` to `crates/roko-cli/src/commands/learn.rs`.

**Steps**: Learn commands: all, router, experiments, efficiency, episodes, tune.

---

### M-D009 — Extract job subcommands

**Objective**: Move `cmd_job_*` to `crates/roko-cli/src/commands/job.rs`.

**Steps**: Job commands: list, create, show, execute, cancel.

---

### M-D010 — Extract deploy/serve/daemon subcommands

**Objective**: Move server-related commands to `crates/roko-cli/src/commands/server.rs`.

**Steps**: Commands: serve, daemon start/stop/status/logs/install, deploy railway/fly/docker, worker.

---

### M-D011 — Extract utility subcommands

**Objective**: Move remaining commands to `crates/roko-cli/src/commands/util.rs`.

**Steps**: Commands: run, status, doctor, init, dashboard, replay, inject, index, new, explain, completions, chat.

---

### M-D012 — Final cleanup of main.rs

**Objective**: main.rs should be ~500 lines: arg parsing + match dispatch to command modules.

**Steps**:
1. Verify main.rs only contains: clap arg definitions + match arms calling `commands::*`
2. Move any remaining helper functions to appropriate modules
3. Run full test suite
4. Verify all CLI commands still work (spot-check 5-10)

**Verification**:
```bash
wc -l crates/roko-cli/src/main.rs  # should be <1000 lines
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings
```

---

## Expected Result

```
crates/roko-cli/src/
  main.rs           — ~500 lines (arg parsing + dispatch)
  commands/
    mod.rs          — pub mod declarations
    plan.rs         — plan subcommands (~800 lines)
    prd.rs          — PRD subcommands (~600 lines)
    agent.rs        — agent subcommands (~500 lines)
    config.rs       — config subcommands (~1200 lines)
    research.rs     — research subcommands (~400 lines)
    knowledge.rs    — knowledge subcommands (~500 lines)
    learn.rs        — learn subcommands (~400 lines)
    job.rs          — job subcommands (~300 lines)
    server.rs       — serve/daemon/deploy (~800 lines)
    util.rs         — remaining commands (~600 lines)
```

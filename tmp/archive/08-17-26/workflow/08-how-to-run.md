# How to Run a Multi-Agent Workflow in Roko Today

## Option 1: Plan-Based Workflow (`roko plan run`)

This is the mori-equivalent flow: define a plan with tasks, execute them with agents, gate, review, commit.

### Step 1: Create a plan directory

```bash
mkdir -p plans/01-my-feature/
```

### Step 2: Write plan.md with frontmatter

```markdown
---
plan: "01-my-feature"
depends_on: []
crates_touched: ["crates/roko-core/"]
estimated_tasks: 3
estimated_minutes: 30
priority: 10
parallel_safe: true
---

# My Feature

Implement X, Y, Z...
```

### Step 3: Write tasks.toml

```toml
[meta]
plan = "01-my-feature"
iteration = 1
total = 3
done = 0
status = "pending"

[[task]]
id = "T1"
title = "Add the FooBar struct"
role = "implementer"
status = "pending"
tier = "focused"
files = ["crates/roko-core/src/foo.rs"]
depends_on = []
timeout_secs = 300
max_retries = 1

[[task.verify]]
command = "cargo check -p roko-core"
expect_exit = 0

[[task]]
id = "T2"
title = "Wire FooBar into the runtime"
role = "implementer"
status = "pending"
tier = "integrative"
files = ["crates/roko-cli/src/run.rs"]
depends_on = ["T1"]
timeout_secs = 300
max_retries = 1

[[task.verify]]
command = "cargo test -p roko-cli"
expect_exit = 0

[[task]]
id = "T3"
title = "Review the implementation"
role = "auditor"
status = "pending"
tier = "focused"
depends_on = ["T1", "T2"]
timeout_secs = 300
max_retries = 0
```

### Step 4: Run the plan

```bash
cargo run -p roko-cli -- plan run plans/
```

### Step 5: Resume if interrupted

```bash
cargo run -p roko-cli -- plan run plans/ --resume .roko/state/executor.json
```

### Step 6: Monitor

```bash
# TUI dashboard
cargo run -p roko-cli -- dashboard

# HTTP API (if roko serve is running)
curl http://localhost:6677/api/status
```

### What happens under the hood

1. Plans discovered from `plans/` directory
2. tasks.toml parsed, DAG built from `depends_on`
3. Tasks dispatched in wave order (T1 first, then T2, then T3)
4. Each task: build 9-layer system prompt -> select model via CascadeRouter -> create agent -> invoke -> log episode
5. After implementation tasks: run gate pipeline (compile, lint, test)
6. On gate failure: AutoFixer agent dispatched with error context
7. On review rejection: back to implementing with review feedback
8. State checkpointed to `.roko/state/executor.json`

### Available task roles

Any of the 28 `AgentRole` values as kebab-case strings:
```
implementer, strategist, architect, auditor, quick-reviewer, scribe,
critic, auto-fixer, refactorer, researcher, conductor, pre-planner,
doc-verifier, integration-tester, merge-resolver, ...
```

### Available task tiers

```
mechanical    -> simple, no strategist, no review, 1 iteration
focused       -> moderate, no strategist, no review, 2 iterations
integrative   -> strategist + quick review, 2 iterations
architectural -> strategist + full review panel, 3 iterations
```

---

## Option 2: Single Prompt (`roko run`)

For one-off tasks without a plan:

```bash
cargo run -p roko-cli -- run "Add a health check endpoint to roko-serve"
```

This runs the universal loop: compose prompt -> dispatch agent -> run gates -> log episode.

No multi-agent pipeline (no strategist, no reviewer). Just: implement -> gate.

---

## Option 3: PRD-Driven (Full Self-Hosting Loop)

The recommended flow for feature work:

```bash
# 1. Capture idea
cargo run -p roko-cli -- prd idea "Multi-agent workflow improvements"

# 2. Draft PRD
cargo run -p roko-cli -- prd draft new "multi-agent-workflow"

# 3. Research for context
cargo run -p roko-cli -- research enhance-prd multi-agent-workflow

# 4. Generate plan + tasks from PRD
cargo run -p roko-cli -- prd plan multi-agent-workflow

# 5. Execute
cargo run -p roko-cli -- plan run plans/

# 6. Watch
cargo run -p roko-cli -- dashboard
```

---

## Option 4: ACP Pipeline (Editor Integration)

For per-prompt workflows from editors:

```bash
# Start ACP server (used by editors over stdio)
cargo run -p roko-cli -- acp
```

The editor connects via ACP protocol and sends `session/prompt`. The workflow config (express/standard/full/auto) determines which agents participate:

- **Express**: Implement -> Gate -> Commit
- **Standard**: Implement -> Gate -> Review -> Commit
- **Full**: Strategy -> Implement -> Gate -> Review -> Commit

Session config options (set from editor UI):
- `workflow`: none/express/standard/full/auto
- `review_strictness`: none/quick/standard/thorough
- `max_iterations`: 1-3
- `clippy_enabled`: true/false
- `tests_enabled`: true/false

---

## What's Not Yet Working / Caveats

1. **Parallel plans disabled by default** (`parallel_enabled = false` in roko.toml). Set to `true` to enable.
2. **Max parallel plans = 1** in config. Increase to enable concurrent plan execution.
3. **ACP pipeline uses raw `claude` CLI** -- doesn't go through roko's agent provider system. No model routing, no provider config.
4. **Conductor meta-agent** is configured but not fully wired for active intervention.
5. **Warm agent pool** config exists but pre-spawning isn't implemented.
6. **No milestone/queue system** like mori's `queue.toml`. Plans execute in priority/numeric order.
7. **No per-plan worktree isolation** (unlike mori). All agents work in the same checkout.
8. **No separate review-tasks.toml / scribe-tasks.toml** -- all tasks in one file with `role` field.

---

## Comparison: Mori Equivalent Commands

| Mori | Roko |
|---|---|
| `mori run --parallel plans/` | `cargo run -p roko-cli -- plan run plans/` |
| `mori run --sequential plans/` | Same (set `parallel_enabled = false`) |
| `mori dashboard` | `cargo run -p roko-cli -- dashboard` |
| `mori status` | `cargo run -p roko-cli -- status` |
| (no equivalent) | `cargo run -p roko-cli -- run "<prompt>"` |
| (no equivalent) | `cargo run -p roko-cli -- acp` |
| (no equivalent) | `cargo run -p roko-cli -- prd idea/draft/plan` |

# Runner 2: `execution-contract` — Granular Batch Specification

Date: 2026-04-28

Parent: [FULL-WORK-PLAN.md](./FULL-WORK-PLAN.md) Runner 2 section.

This document specifies every batch at the same detail level as
[RUNNER-PLAN.md](./RUNNER-PLAN.md) (Runner 1: demo-truth).

---

## Runner Goal (one sentence)

Make CLI execution contracts (init, model selection, gates, failure semantics, state, learn
paths) truthful enough that demo scenarios and agent sessions can rely on them.

## Context Pack Files

```text
tmp/runners/execution-contract/
  README.md
  batches.toml
  context/
    00-RULES.md                    — universal + runner-specific anti-patterns
    ARCHITECTURE-CONTRACT.md       — single-owner map for this runner
    ANTI-PATTERNS.md               — forbidden patterns with repo examples
    ACCEPTANCE.md                  — proof commands including negative proofs
    FILE-OWNERSHIP.md              — batch → write path map
    ISSUE-MAP.md                   — batch → issue id map
    CONFIG-CONTRACT.md             — current init/config/migrate data flow (Group A output)
    GATE-DATA-FLOW.md              — current gate config → execution flow (Group C output)
    MODEL-SELECTION-CONTRACT.md    — EffectiveModelSelection spec (Group B output)
```

---

## Anti-Pattern Rules (00-RULES.md)

Include the universal rules from FULL-WORK-PLAN.md plus:

```markdown
# Execution-Contract Anti-Patterns

EC-1. **One model selection path.** There is exactly ONE function that resolves effective
      model+provider. Every command calls it. If you are tempted to resolve model/provider
      locally in a command handler, STOP. Call the shared resolver instead.

      EXISTING ANTI-PATTERN (do not repeat):
      - `crates/roko-cli/src/commands/prd.rs` reads `cli.model` but `PrdCmd::Plan` calls
        `generate_plan_from_prd()` which ignores it and reads `resolved.config.agent.model`.
      - `crates/roko-cli/src/commands/plan.rs` `PlanCmd::Regenerate` reads `model_from_config()`
        instead of `cli.model`.
      - `crates/roko-cli/src/commands/config_cmd.rs` `cmd_provider_test` calls
        `select_provider_test_model()` and ignores `--model`.

EC-2. **CLI --model is a hard override.** If user passes --model X and provider for X is
      unavailable, the command FAILS with a clear error naming the missing provider/key.
      It does NOT silently fall back to another model.

      EXISTING ANTI-PATTERN (do not repeat):
      - `roko --model gpt-4o "say ok"` silently uses glm-5.1 instead.
      - `roko run --model claude-haiku-4-5 "prompt"` uses anthropic_api sonnet instead.

EC-3. **Gate verdicts are typed.** Pass means the gate ran and succeeded. Fail means it ran
      and failed. Skipped/NotWired means it did not run. These are distinct in the type system.

      EXISTING ANTI-PATTERN (do not repeat):
      - Stub gates return `GateVerdict { passed: true }` with a string saying "stub gate; not wired."
      - Shell gate falls through to `_ => None` in `gate_for_name()` which creates a fail verdict
        even though it's a config/wiring issue, not a code issue.

EC-4. **Workflow halt = nonzero exit.** If `roko run` prints "workflow halted", the process
      MUST exit nonzero. Scripts and CI depend on exit codes.

      EXISTING ANTI-PATTERN (do not repeat):
      - `roko run` halted on missing ANTHROPIC_API_KEY but exited 0.
      - `roko explain "cascade routing"` printed "unknown topic" and exited 0.

EC-5. **Config schema v2 is the only output for new workspaces.** `roko init` writes v2.
      There is no "upgrade later" path for new workspaces. Only existing workspaces need migrate.

EC-6. **State views agree.** There is ONE canonical state source. `status`, `plan list`,
      `resume`, and `plan run` all read the same file or projection.

      EXISTING ANTI-PATTERN (do not repeat):
      - `status` reads executor.json, `plan list` reads plan directories, `run-state.json`
        has different counts. Three views disagree.

EC-7. **Learn paths match write paths.** `learn all` reads from the exact paths that
      execution writes to. If you change where events are written, update readers.

      EXISTING ANTI-PATTERN (do not repeat):
      - `.roko/learn/efficiency.jsonl` has 22 entries, `roko learn all` says "empty."
```

---

## Group A: Config and Init Contract

### A01 — Document current init template and config data flow

**Type:** Context-only (no code changes)

**Goal:** Map the data flow from `roko init` → roko.toml → config loading → provider resolution,
so later batches know exactly what to change.

**Write scope:**
- `tmp/runners/execution-contract/context/CONFIG-CONTRACT.md`

**Read (context only):**
- `crates/roko-cli/src/commands/init.rs`
- `crates/roko-core/src/config.rs` (and any config/ submodules)
- `crates/roko-cli/src/commands/config_cmd.rs` (migrate logic)
- `crates/roko-cli/src/run.rs` (how WorkflowEngine reads config)

**Required output:**
- Document: what fields does `roko init` currently write to `roko.toml`?
- Document: what does schema v1 look like vs schema v2?
- Document: what does `config migrate` add?
- Document: where does WorkflowEngine read provider/model info?
- Document: where does the one-shot path read provider/model info?
- Identify the specific template string or function that generates the initial TOML.

**DO NOT:**
- Change any source code in this batch
- Guess at implementation — read the actual code

**Verify:** N/A (context-only)

---

### A02 — Make `roko init` emit schema v2 with providers

**Goal:** A fresh `roko init` workspace is immediately usable without `config migrate`.

**Write scope:**
- `crates/roko-cli/src/commands/init.rs` (or wherever the config template lives)

**Read:**
- `tmp/runners/execution-contract/context/CONFIG-CONTRACT.md` (from A01)
- `crates/roko-core/src/config.rs`

**Required behavior:**
- Generated `roko.toml` has `schema_version = 2`
- Generated `roko.toml` has `[providers.claude_cli]` with `kind = "claude_cli"` and
  `command = "claude"` (or equivalent)
- Generated `roko.toml` has `[models.claude-sonnet-4-6]` mapping to `claude_cli` provider
- If `claude` is NOT on PATH, still write the provider section but add a comment:
  `# Install Claude CLI: https://docs.anthropic.com/...`
- The generated config does NOT have a bare `[agent] command = "claude"` v1-style section

**DO NOT:**
- Change config parsing/reading code — only change what `init` writes
- Add new config fields not already in schema v2
- Make init interactive (non-interactive by default)
- Touch the `config migrate` command — that stays for existing workspaces

**Verify:** `cargo check -p roko-cli`

**Evidence this is needed:** E2E-DOGFOOD-AUDIT Path 1, E2E-TEST-RESULTS S1, COMPREHENSIVE-ISSUES 1.1, 8.1

---

### A03 — Non-interactive `config migrate --yes`

**Goal:** Scripts and runners can migrate configs without TTY prompts.

**Write scope:**
- `crates/roko-cli/src/commands/config_cmd.rs` (migrate subcommand handler)

**Required behavior:**
- Accept `--yes` or `-y` flag to skip `Apply changes? [y/N]` prompt
- When `-y` passed: apply and print what changed
- When not passed: keep existing interactive behavior
- Dry-run (`--dry-run`) still works and does not apply

**DO NOT:**
- Change what the migration actually does (field additions/removals)
- Make `-y` the default
- Change other config subcommands

**Verify:** `cargo check -p roko-cli`

**Evidence:** E2E-DOGFOOD-AUDIT Path 7b ("prompted Apply changes? and exited cancelled")

---

### A04 — Config preflight: warn on v1

**Goal:** Commands that need provider/model config detect stale config early.

**Write scope:**
- `crates/roko-cli/src/main.rs` (or shared config loading path)

**Read:**
- `crates/roko-core/src/config.rs` — how schema_version is read

**Required behavior:**
- Before commands `run`, `prd draft`, `prd plan`, `plan run`, `plan generate`:
  check `schema_version` in loaded config
- If v1 or missing `[providers]`: print to stderr:
  `warning: config uses schema v1. Run 'roko config migrate --yes' to update.`
- Warning fires at most once per invocation
- Do NOT block execution (warn only for now)

**DO NOT:**
- Auto-migrate without user consent
- Make this an error that blocks execution
- Add the check to commands that don't need providers (e.g., `status`, `doctor`)

**Verify:** `cargo check -p roko-cli`

---

### A05 — Default gates use real programs, not `true`

**Goal:** `roko init --profile rust` generates gates that do real validation.

**Write scope:**
- Same files as A02 (init config template)

**Required behavior:**
- `--profile rust` gates:
  ```toml
  [[gate]]
  kind = "shell"
  name = "compile"
  program = "cargo"
  args = ["check"]

  [[gate]]
  kind = "shell"
  name = "test"
  program = "cargo"
  args = ["test"]

  [[gate]]
  kind = "shell"
  name = "lint"
  program = "cargo"
  args = ["clippy", "--", "-D", "warnings"]
  ```
- `--profile typescript` gates: `npx tsc --noEmit`, `npm test`
- No profile / unknown: empty `[[gate]]` array with a comment explaining how to add gates
- NEVER generate `program = "true"` — that is the anti-pattern this fixes

**DO NOT:**
- Change GateConfig type definitions
- Change gate execution logic (that's Group C)
- Generate gate rung configurations (just the shell commands)

**Verify:** `cargo check -p roko-cli`

**Evidence:** E2E-DOGFOOD-AUDIT Path 1, Path 8 ("default gate is `program = true`"), COMPREHENSIVE-ISSUES 2.1

---

### A06 — `config validate` counts schema warnings honestly

**Goal:** Validation output doesn't contradict itself.

**Write scope:**
- `crates/roko-cli/src/commands/config_cmd.rs` (validate handler)

**Required behavior:**
- Schema version warnings increment the reported warning count
- Output: `Result: 1 warning, 0 errors` (not `0 warnings, 0 errors` after logging a warning)
- Categorize: `[migration] schema v1 detected — run 'roko config migrate'`

**DO NOT:**
- Make schema v1 an error
- Change what constitutes a warning vs error for other fields
- Touch the validate logic for non-schema checks

**Verify:** `cargo check -p roko-cli`

**Evidence:** DEMO-APP-WORKFLOW-AUDIT Explore section, E2E-DOGFOOD-AUDIT Path 1

---

## Group B: Effective Model and Provider Selection

### B01 — Document current model resolution paths

**Type:** Context-only (no code changes)

**Goal:** Map exactly where and how each command currently resolves model/provider.

**Write scope:**
- `tmp/runners/execution-contract/context/MODEL-SELECTION-CONTRACT.md`

**Read:**
- `crates/roko-cli/src/run.rs` (v2 run model resolution, ~lines 200-300)
- `crates/roko-cli/src/commands/prd.rs` (prd draft/plan model usage)
- `crates/roko-cli/src/commands/plan.rs` (plan run/regenerate model)
- `crates/roko-cli/src/commands/config_cmd.rs` (provider test model)
- `crates/roko-cli/src/main.rs` (positional prompt model)
- `crates/roko-cli/src/orchestrate.rs` (plan runner model per-task)
- `crates/roko-learn/src/cascade_router.rs` (router API)

**Required output:**
- For each of these 8 paths: where is model resolved? What value is actually used?
- Identify the EXACT lines where --model CLI arg is dropped or overridden
- Document current cascade router API: `recommend()` or equivalent
- Propose the `EffectiveModelSelection` struct and `resolve_effective_model()` function signature
- Identify where in the source tree this new module should live

**DO NOT:** Change any source code.

---

### B02 — Implement `EffectiveModelSelection` module with tests

**Goal:** One module, one function, one resolution path.

**Write scope:**
- `crates/roko-cli/src/model_selection.rs` (NEW FILE)
- `crates/roko-cli/src/lib.rs` (add module declaration)

**Read:**
- `tmp/runners/execution-contract/context/MODEL-SELECTION-CONTRACT.md` (from B01)
- `crates/roko-core/src/config.rs` (Config struct, provider/model tables)
- `crates/roko-learn/src/cascade_router.rs` (CascadeRouter API)

**Required behavior:**
```rust
pub struct EffectiveModelSelection {
    pub requested_model: Option<String>,
    pub effective_model_key: String,
    pub provider_key: String,
    pub provider_kind: String,     // "claude_cli", "anthropic_api", "openai_compat", "ollama"
    pub backend_slug: String,      // actual slug sent to provider
    pub source: SelectionSource,
    pub reason: String,            // human-readable explanation
}

pub enum SelectionSource {
    CliOverride,
    TaskModel,
    RoleConfig,
    CascadeRouter,
    ProjectDefault,
    BuiltInDefault,
}

pub fn resolve_effective_model(
    cli_model: Option<&str>,
    task_hint: Option<&str>,
    role_config: Option<&str>,
    cascade: &CascadeRouter,
    config: &Config,
) -> Result<EffectiveModelSelection> {
    // Precedence:
    // 1. cli_model → HARD OVERRIDE. If provider unavailable, FAIL.
    // 2. task_hint → only when no CLI override
    // 3. role_config → only when no task hint
    // 4. cascade.recommend() → only when no explicit config
    // 5. config.default_model() → last fallback
    // 6. BuiltInDefault ("claude-sonnet-4-6" via claude_cli) → absolute last resort
}
```
- Include unit tests for each precedence level
- Include test: CLI override with unavailable provider → error
- Include test: unknown model slug → error

**DO NOT:**
- Move CascadeRouter — just call its existing API
- Change the Config struct
- Add new config fields
- Create a new crate — one file in roko-cli

**Verify:** `cargo test -p roko-cli -- model_selection`

---

### B03 — Wire selection into `roko run` (v2 path)

**Goal:** `roko run --model X` actually uses model X.

**Write scope:**
- `crates/roko-cli/src/run.rs` (v2 run path)

**Read:**
- `crates/roko-cli/src/model_selection.rs` (from B02)

**Required behavior:**
- Replace existing ad-hoc model resolution with `resolve_effective_model(cli.model, ...)`
- If selected provider is unavailable: return error with specific message
  `"Provider 'anthropic_api' for model 'claude-sonnet-4-6' requires ANTHROPIC_API_KEY"`
- Print effective selection: `model: claude-sonnet-4-6 via claude_cli (source: CLI override)`
- Pass the resolved model key to the agent/workflow dispatch

**DO NOT:**
- Change the WorkflowEngine internals — just change what model info is passed to it
- Add fallback chains — if CLI said `--model X`, either use X or fail
- Change the legacy run path (that's B05)

**Verify:** `cargo check -p roko-cli`

**Evidence:** E2E-DOGFOOD-AUDIT Path 7 ("ignores --model, uses anthropic_api sonnet")

---

### B04 — Wire selection into `prd draft`, `prd plan`, `plan generate`, `plan regenerate`

**Goal:** These commands honor --model instead of using config-only resolution.

**Write scope:**
- `crates/roko-cli/src/commands/prd.rs`
- `crates/roko-cli/src/commands/plan.rs`
- `crates/roko-cli/src/prd.rs` (function signatures may need model param)

**Required behavior:**
- `prd draft new --model X`: uses X
- `prd plan --model X`: uses X (currently ignores and uses Opus)
- `plan regenerate --model X`: uses X (currently reads `model_from_config()`)
- All call `resolve_effective_model(cli.model, ...)`
- All print effective selection

**DO NOT:**
- Add per-command resolution logic — all call the SAME function from B02
- Change model resolution for `plan run` (that's B05 — needs per-task handling)
- Change the prompt content — just change what model is used

**Verify:** `cargo check -p roko-cli`

**Evidence:** E2E-DOGFOOD-AUDIT Path 3 ("CLI requested claude-haiku-4-5, actually started claude-opus-4-6")

---

### B05 — Wire selection into `plan run` (per-task) and legacy run

**Goal:** Plan runner respects CLI --model as override over task model_hint.

**Write scope:**
- `crates/roko-cli/src/orchestrate.rs` (task dispatch section)
- `crates/roko-cli/src/run.rs` (legacy path if separate)

**Required behavior:**
- CLI `--model` overrides task `model_hint` in each task dispatch
- If no CLI override: use task `model_hint` → role config → cascade → default
- Legacy `run --engine legacy --model X`: uses X

**DO NOT:**
- Remove task model_hint support — it's useful when no CLI override
- Change the per-task dispatch broadly — just swap in `resolve_effective_model()`

**Verify:** `cargo check -p roko-cli`

---

### B06 — Wire selection into `config providers test` and `config models route`

**Goal:** Provider tests test the specified model. Route shows real resolution.

**Write scope:**
- `crates/roko-cli/src/commands/config_cmd.rs` (provider test + model route handlers)

**Required behavior:**
- `config providers test ollama --model llama32`: tests `llama32` (currently ignores and tests gemma4)
- `config providers test --json`: outputs JSON (currently ignored)
- `config models route gpt-4o`: either resolves exactly to configured model, or clearly says
  "gpt-4o is not configured; cascade router recommends: claude-sonnet-4-6"
- Provider test with empty response content: mark as `"content_empty": true`, not full success

**Root cause to fix:**
- `cmd_provider_test` calls `select_provider_test_model(&config, provider_name)` — replace with
  using `cli.model` when present
- `cmd_model_route` treats requested model as `previous_model` for cascade recommendation —
  should either resolve exactly or document that it's a recommendation tool

**DO NOT:**
- Change provider test to do real expensive work
- Remove the human-readable output — --json is opt-in

**Verify:** `cargo check -p roko-cli`

**Evidence:** E2E-DOGFOOD-AUDIT Path 10 ("Ollama was requested as llama32 but tested gemma4:26b")

---

### B07 — Print and persist effective selection

**Goal:** Users can always see what model/provider was used and why.

**Write scope:**
- Wherever B03-B06 print selection (consolidate formatting)

**Required behavior:**
- Every agent-starting command prints to stderr (with `--timing` or always):
  `model: <key> via <provider> (source: <source>)`
- If `--json` or structured logging is active: include in event/episode data
- Existing efficiency/episode events should include the resolved model key

**DO NOT:**
- Print to stdout (interferes with piped output)
- Add new logging frameworks
- Change event schemas broadly

**Verify:** `cargo check -p roko-cli`

---

## Group C: Gate Truth

### C01 — Document current gate data flow

**Type:** Context-only (no code changes)

**Goal:** Map how gate configs flow from roko.toml through to execution.

**Write scope:**
- `tmp/runners/execution-contract/context/GATE-DATA-FLOW.md`

**Read:**
- `crates/roko-core/src/foundation.rs` (GateConfig enum)
- `crates/roko-gate/src/gate_service.rs` (gate_for_name, ShellGate if exists)
- `crates/roko-cli/src/orchestrate.rs` (~line 7522 where GateConfig::Shell → "shell" string)
- `crates/roko-gate/src/` (look for existing gate implementations)

**Required output:**
- How does `GateConfig::Shell { program, args }` turn into a gate execution?
- Where is it converted to a string?
- What does `gate_for_name()` handle?
- Does `ShellGate` exist? Where? What's its API?
- What does the return path look like (GateVerdict fields)?
- What is the current `GateVerdict` struct definition?

**DO NOT:** Change any source code.

---

### C02 — Wire shell gate in `gate_for_name()`

**Goal:** `gate_for_name("shell")` instantiates and runs `ShellGate`.

**Write scope:**
- `crates/roko-gate/src/gate_service.rs`

**Read:**
- `tmp/runners/execution-contract/context/GATE-DATA-FLOW.md` (from C01)
- `crates/roko-core/src/foundation.rs` (GateConfig)

**Required behavior:**
- `gate_for_name("shell")` with program + args: creates a `ShellGate` and runs it
- `ShellGate` spawns the program with args in the workspace directory
- Exit 0 → `GateVerdict { passed: true, ... }`
- Exit nonzero → `GateVerdict { passed: false, evidence: stderr excerpt }`
- If `ShellGate` already exists somewhere, import and wire it (search first!)

**DO NOT:**
- Create a new gate trait — use existing
- Make ShellGate always pass
- Add timeout/retry (separate concern)
- Change the `gate_for_name` signature if it's called from multiple places

**Verify:** `cargo check -p roko-gate`

**Evidence:** COMPREHENSIVE-ISSUES 2.1, E2E-TEST-RESULTS S2, E2E-DOGFOOD-AUDIT Path 8

---

### C03 — Pass program/args from config through to gate dispatch

**Goal:** The shell gate receives its configured program and args, not hardcoded values.

**Write scope:**
- `crates/roko-cli/src/orchestrate.rs` (gate dispatch section, ~line 7522)

**Required behavior:**
- When dispatching a shell gate, pass the `program` and `args` from `GateConfig::Shell`
- The gate dispatch does NOT hardcode `cargo check` — it reads from config
- The workspace directory is passed as the gate's working directory

**DO NOT:**
- Change `GateConfig` type — just read its existing fields
- Hardcode any programs
- Change non-shell gate dispatching

**Verify:** `cargo check -p roko-cli`

---

### C04 — Add `skipped/not_wired` to gate verdicts

**Goal:** Stub gates are distinguishable from real passes.

**Write scope:**
- `crates/roko-core/src/foundation.rs` (GateVerdict struct)
- `crates/roko-gate/src/gate_service.rs` (stub gate return values)

**Required behavior:**
- Add to GateVerdict: `pub skipped: bool` with `#[serde(default)]`
- Add to GateVerdict: `pub skip_reason: Option<String>` with `#[serde(default)]`
- All stub gates set `skipped = true, skip_reason = Some("not wired: <name>")`
- Existing callers compile without changes (new fields have defaults)

**DO NOT:**
- Create a separate SkippedVerdict type
- Remove existing fields
- Change real gate behavior

**Verify:** `cargo check --workspace` (touches core type)

---

### C05 — Learning/dashboard: don't count skipped as pass

**Goal:** Gate pass rate excludes skipped gates from the denominator.

**Write scope:**
- `crates/roko-cli/src/orchestrate.rs` (gate summary)
- `crates/roko-learn/src/runtime_feedback.rs` (threshold updates if applicable)

**Required behavior:**
- Pass rate = `passed / (passed + failed)` — skipped excluded
- Adaptive thresholds only update from real pass/fail
- Gate summary output: `X passed, Y failed, Z skipped`
- Efficiency events include `gates_skipped` count

**DO NOT:**
- Change gate execution order
- Remove skipped gates from reports (they should still appear)
- Count NotWired differently from Skipped for pass rate purposes

**Verify:** `cargo check -p roko-cli -p roko-learn`

---

### C06 — Gate truth regression test

**Goal:** Prove gates work correctly under all conditions.

**Write scope:**
- `crates/roko-gate/tests/` or `crates/roko-gate/src/` (test module)

**Required behavior — tests:**
- `shell:true` → passes
- `shell:false` → fails with evidence
- `shell:cargo check` in valid Rust project → passes
- Unknown gate name → clear error
- Stub gate → skipped = true, not counted as pass

**DO NOT:**
- Make tests depend on network
- Make tests modify global state

**Verify:** `cargo test -p roko-gate`

---

## Group D: CLI Failure Semantics and Run State

### D01 — Workflow halt → nonzero exit

**Goal:** Failed workflows cannot be mistaken for success by scripts.

**Write scope:**
- `crates/roko-cli/src/run.rs` (exit path)
- `crates/roko-cli/src/main.rs` (if exit code is set there)

**Required behavior:**
- Workflow halted (missing key, provider error, gate failure without replan): exit 1
- Success (all gates pass, agent completed): exit 0
- Partial success (some tasks done, some failed): exit 1
- Print halt reason to stderr before exiting

**DO NOT:**
- Change exit codes for commands that already work (like `--provider` arg error = 2)
- Add complex exit code schemes
- Change one-shot path here (that's D02)

**Verify:** `cargo check -p roko-cli`

**Evidence:** E2E-DOGFOOD-AUDIT Path 7 ("halted on missing API key but exited 0"), DEMO-APP-WORKFLOW-AUDIT D06

---

### D02 — `roko explain` nonzero for unknown topics

**Write scope:**
- `crates/roko-cli/src/explain.rs`

**Required behavior:**
- Unknown topic → exit 1, print to stderr: `Unknown topic: "X". Known topics: <list>`
- Known topic → exit 0, print explanation

**Evidence:** DEMO-APP-WORKFLOW-AUDIT Explore section

---

### D03 — `plan validate` mandatory before `plan run`

**Goal:** Invalid plans cannot accidentally execute.

**Write scope:**
- `crates/roko-cli/src/orchestrate.rs` (plan run entry point)

**Required behavior:**
- Before executing any task: run same validation as `plan validate`
- If validation fails: print diagnostics, exit 1, start NO agents
- `--dry-run` also runs validation
- No `--skip-validate` flag (too dangerous)

**Evidence:** E2E-DOGFOOD-AUDIT Path 5 ("dry-run accepted invalid plan")

---

### D04 — `plan run --fresh` for state reset

**Write scope:**
- `crates/roko-cli/src/orchestrate.rs` (plan run entry)
- `crates/roko-cli/src/commands/plan.rs` (CLI flag)

**Required behavior:**
- `plan run --fresh`: archives old state to `.roko/state/archive-<timestamp>/`, starts clean
- Without flag: keep existing resume behavior
- When state drift detected: print "Run 'plan run --fresh' to start over"

**Evidence:** E2E-DOGFOOD-AUDIT Path 6 ("resume validation failed: 3 tasks drifted")

---

### D05 — `roko resume` uses canonical `.roko/plans`

**Write scope:**
- `crates/roko-cli/src/main.rs` (resume handler)

**Required behavior:**
- Look in `.roko/plans/` first (PRD plan output)
- If not found: check `./plans/` as fallback with a note
- Error message names all checked paths
- `roko resume <plan-id>` looks in `.roko/plans/<plan-id>/`

**Evidence:** E2E-DOGFOOD-AUDIT Path 11 ("error: cannot read directory ./plans")

---

### D06 — `status`, `plan list`, run-state agree

**Write scope:**
- `crates/roko-cli/src/commands/status.rs`
- `crates/roko-cli/src/commands/plan.rs` (plan list)

**Required behavior:**
- Identify which state file is canonical and use it consistently
- If run-state says 1/3 completed: status and plan list both say 1/3
- If no state file: "no run state found" (not "0/3 tasks")

**Evidence:** E2E-DOGFOOD-AUDIT Path 11 ("status shows 0 plans while run-state has 1 completed")

---

## Group E: Learn Path Visibility

### E01 — Map learn write/read paths

**Type:** Context-only

**Write scope:** runner context

**Read:**
- `crates/roko-cli/src/orchestrate.rs` (where efficiency/episodes are written)
- `crates/roko-cli/src/commands/learn.rs` (where learn all reads)
- `.roko/learn/` directory (actual files)

**Output:** Document exact write path vs read path for efficiency, episodes, costs, cascade-router.

---

### E02 — Align `learn all` with actual write paths

**Write scope:**
- `crates/roko-cli/src/commands/learn.rs`

**Required behavior:**
- Read from the exact paths that orchestrate.rs writes to
- If files don't exist: say "No data at <path>" (not just "empty")
- Print data summary if files exist and have entries

**Evidence:** COMPREHENSIVE-ISSUES 4.1, E2E-TEST-RESULTS S4

---

### E03 — Fixture test for learn paths

**Write scope:**
- `crates/roko-cli/tests/` (integration test)

**Required behavior:**
- Write one fake efficiency event to the correct path
- Write one fake episode to the correct path
- Run `learn all` (or its underlying function)
- Assert it finds and reports the data

---

## Group F: Suppressed Noise

### F01 — Raw stream JSON not dumped to terminal

**Goal:** Normal users don't see raw JSON protocol lines.

**Write scope:**
- `crates/roko-agent/src/claude_cli_agent.rs` or `crates/roko-agent/src/provider/claude_cli.rs`

**Required behavior:**
- Raw stream-json lines go to a debug channel (not stderr/stdout in normal mode)
- With `--verbose` or `ROKO_DEBUG=1`: lines go to stderr
- Final agent text response IS printed
- Tool call summaries ("Used: Read src/lib.rs") are printed

**Evidence:** E2E-DOGFOOD-AUDIT Path 3, Path 4 ("dumped raw Claude stream JSON to the terminal")

---

### F02 — Server unmatched `/api/*` returns JSON 404

**Write scope:**
- `crates/roko-serve/src/lib.rs` or route configuration

**Required behavior:**
- Any request to `/api/*` not matching a route → `404 {"error": "not found"}` with JSON content-type
- SPA catch-all only for non-`/api/` paths
- `/ws/*` also excluded from SPA catch-all

**Evidence:** DEMO-APP-WORKFLOW-AUDIT D12, COMPREHENSIVE-ISSUES 11.x

---

## Batch Summary

| Group | Batches | Main scope |
|---|---:|---|
| A: Config/Init | 6 | init template, config validation, preflight |
| B: Model Selection | 7 | EffectiveModelSelection, wire into all commands |
| C: Gate Truth | 6 | ShellGate wire, skipped verdicts, tests |
| D: CLI Failure | 6 | exit codes, validate-before-run, resume, state |
| E: Learn Paths | 3 | align read/write, fixture test |
| F: Noise/API | 2 | suppress raw JSON, JSON 404 |
| **Total** | **30** | |

## Suggested Execution Waves

Wave 1: A01, B01, C01, E01 (context-only, parallel)
Wave 2: A02, A03 (init and migrate)
Wave 3: A04, A05, A06 (config preflight, gates template, validate)
Wave 4: B02 (EffectiveModelSelection module + tests — foundation for all B0x)
Wave 5: B03, B04 (wire into run, prd, plan)
Wave 6: B05, B06, B07 (wire into plan run, provider test, print)
Wave 7: C02, C03 (shell gate wire)
Wave 8: C04, C05, C06 (skipped verdicts, learning, tests)
Wave 9: D01, D02, D03 (exit codes, validate-before-run)
Wave 10: D04, D05, D06 (fresh flag, resume path, state agree)
Wave 11: E02, E03, F01, F02 (learn paths, suppress noise, JSON 404)

## Acceptance Criteria

This runner is done when:

**Positive proofs:**
- Fresh `roko init --profile rust` → v2 config with real gates and providers
- `roko --model claude-haiku-4-5 prd plan foo` → uses claude-haiku-4-5 (not Opus)
- `roko plan validate .roko/plans/foo` → passes or gives actionable errors
- `roko plan run .roko/plans/foo` with valid plan → runs with real shell gates
- `roko resume` → finds plans in `.roko/plans/`
- `roko learn all` → shows data after a run
- All commands print effective model selection with `--timing`

**Negative proofs:**
- `roko --model nonexistent-model run "x"` → fails with "model not configured" (not silent fallback)
- `roko run` with missing API key → exits nonzero (not 0 after "workflow halted")
- `roko plan run` with invalid plan → refuses to start (prints validation errors)
- Shell gate with `program = "false"` → gate fails
- Stub gates → skipped, not counted as pass
- `roko explain "nonexistent"` → exits nonzero
- `/api/nonexistent` → JSON 404 (not SPA HTML)

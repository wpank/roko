# E2E Roko Dogfood Audit

Date: 2026-04-28

This is the end-to-end run requested on 2026-04-28. I ran Roko in fresh temporary
Rust repos, tried the PRD/plan workflow, the plan runner, top-level `run`, legacy
`run`, positional one-shot prompts, provider tests, route explanations, status,
plan list/show, and resume.

The short version: Roko can complete a small code change only through the legacy
runner path or through a manually repaired generated plan. The default paths are
not yet trustworthy because model routing, generated task contracts, real gates,
resume, and telemetry disagree with each other.

## Workspaces Tested

- Primary PRD/plan runner workspace: `/private/tmp/roko-e2e-rust-yW9yhe`
- Top-level `run` workspace: `/private/tmp/roko-e2e-run-1777399243`
- Roko binary: `/Users/will/dev/nunchi/roko/roko/target/debug/roko`

Tooling present:

- `claude --version`: `2.1.63 (Claude Code)`
- `codex --version`: `codex-cli 0.125.0`
- `cargo`, `jq`, `rg` available

Provider config observed through Roko:

- OpenAI: configured and working for provider test
- ZAI: configured and working for provider test, but content was empty
- Moonshot: configured and working for provider test, but content was empty
- Ollama: configured and reachable, but provider test ignored the requested model
- Claude CLI: installed and usable, but provider list initially claimed command missing
- Anthropic API: no `ANTHROPIC_API_KEY`, so API route fails

## Executive Verdict

The previous two-rail M0 is still the right shape:

- M0-A: Mori-like interactive session parity
- M0-B: grounded PRD/plan artifact generation

But the dogfood run adds a prerequisite before either rail can be trusted:

**M0-0: Execution Contract Repair**

Fix model/provider selection, generated task schema, real gate status, resume pathing,
and telemetry truthfulness first. Without that, Roko can spend money, mark work as
successful, and show green UI signals while using the wrong model, writing invalid
plans, accepting stub gates, and double-counting or zeroing cost.

## Path 1: Fresh Init

Commands:

```sh
cargo init --lib /private/tmp/roko-e2e-rust-yW9yhe
/Users/will/dev/nunchi/roko/roko/target/debug/roko init --profile rust .
```

Outcome:

- `roko init` wrote `roko.toml` and detected Rust.
- Generated config still uses schema version 1 and no `[providers]` section.
- It wrote a no-op gate: `[[gate]] program = "true"`.
- `roko config validate` warned about schema v1 but reported `0 warnings, 0 errors`.
- `doctor` said `serve_auth` is OK because auth is disabled.

Issue:

The first generated config teaches the system to pass a no-op gate and treats missing
new config shape as a warning in logs but not in validation totals. For a Mori-parity
first mile, `init` should create real compile/test/clippy gates for Rust, and
validation should be honest about migration warnings. Either `init` should emit schema
version 2 directly, or first run should block on an explicit migration/preflight path.

## Path 2: PRD Draft

Commands:

```sh
roko --no-serve prd idea "Add a public greet(name: &str) -> String function to src/lib.rs and tests proving it returns Hello, <name>!. Use the existing Rust library."
roko --no-serve --timing --model claude-haiku-4-5 --effort low prd draft new greet-function
```

Outcome:

- Roko started `claude_cli:claude-haiku-4-5`.
- Claude used tools: `Read`, `Read`, `Glob`, `Read`, `Write`.
- Draft completed in 64.3s.
- Draft written to `.roko/prd/drafts/greet-function.md`.
- Agent log line said `result received (0 bytes text, 5 tool calls)`.

Artifact quality:

- The draft did identify `src/lib.rs` and the current crate.
- It was dramatically overbuilt for a one-function change: 15 citations, diagrams,
  academic framing, and a long formal spec.
- It did not include the proposed `Repository Grounding` section.

Telemetry:

The PRD draft episode recorded 0 input tokens, 0 output tokens, and $0 cost:

```text
prd:draft:new:greet-function claude claude-haiku-4-5 true 64.299 ... input=0 output=0 cost=0
```

Issue:

This path is capable but over-incentivized toward polished documents instead of short
repo-grounded implementation specs. Usage is still recorded as zero.

## Path 3: PRD Plan Generation

Command:

```sh
roko --no-serve --timing --model claude-haiku-4-5 --effort low prd plan greet-function
```

Outcome:

- CLI requested `claude-haiku-4-5`.
- Roko actually started `claude_cli:claude-opus-4-6`.
- Plan generation completed in 92.3s.
- It wrote `.roko/plans/greet-function/plan.md` and `tasks.toml`.
- It dumped raw Claude stream JSON to the terminal, including tool calls, tool results,
  assistant thinking metadata, and final result events.

Actual Claude result metadata visible in the raw stream:

- `total_cost_usd`: about `$0.47014`
- `cache_creation_input_tokens`: 35490
- `cache_read_input_tokens`: 290425
- `output_tokens`: 4121

Roko episode telemetry for the same run:

```text
prd:plan:greet-function claude_cli claude-opus-4-6 true 92.303 ... input=0 output=0 cost=0
```

Generated plan quality:

- `plan.md` was concise and reasonable.
- `tasks.toml` was semantically reasonable but invalid for Roko's own validator.
- It omitted required `role` fields.
- It used model aliases `sonnet` and `haiku`, which are not configured model keys.

Validation result:

```text
PLAN_003 task 'T1' is missing required field 'role'
PLAN_003 task 'T2' is missing required field 'role'
PLAN_003 task 'T3' is missing required field 'role'
PLAN_009 model 'sonnet' is not configured
PLAN_009 model 'haiku' is not configured
```

Root cause in source:

- `crates/roko-cli/src/commands/prd.rs` captures `cli.model`, but `PrdCmd::Plan`
  calls `roko_cli::prd::generate_plan_from_prd(&slug, &prd_path, dry_run)` and does
  not pass the CLI model or effort.
- `crates/roko-cli/src/prd.rs` then uses `resolved.config.agent.model.as_deref()`.
- The generator prompt asks for `model_hint`, but does not force full configured
  model slugs or required `role`.

Issue:

The PRD plan command ignores `--model`, spends Opus-class cost, writes an invalid
plan, records zero cost, and prints raw stream internals into user-facing logs.

## Path 4: Plan Regenerate

Commands:

```sh
roko --no-serve --timing --model claude-haiku-4-5 --effort low plan regenerate .roko/plans/greet-function --dry-run
roko --no-serve --timing --model claude-haiku-4-5 --effort low plan regenerate .roko/plans/greet-function
```

Dry-run outcome:

- Did not call the model.
- Printed only prompt length.
- Did not preview the changed plan or run validation.

Normal outcome:

- Ignored requested `claude-haiku-4-5`.
- Started `claude-opus-4-6`.
- Dumped raw Claude stream JSON again.
- Actual Claude result cost visible in stream: about `$0.18259`.
- Roko memory still recorded zero tokens and zero cost.
- Regenerated `tasks.toml` still missed `role` and still used `sonnet`/`haiku`.

Root cause in source:

- `crates/roko-cli/src/commands/plan.rs` handles `PlanCmd::Regenerate` by reading
  `model_from_config(&workdir)` instead of `cli.model`.

Issue:

Regeneration has the same schema problem as initial generation and cannot repair the
exact invalid output Roko just produced.

## Path 5: Running The Generated Plan

Command:

```sh
roko --no-serve --timing --model claude-haiku-4-5 --effort low plan run .roko/plans/greet-function
```

Outcome on generated plan:

```text
agent provider resolution failed plan_id=greet-function task=T1 error=model 'sonnet' resolved to unsupported provider 'openai_compat': model references missing provider 'openai_compat'
Plan complete: 0/3 tasks, $0.00, 0s
```

Issues:

- `plan run --dry-run` accepted and displayed the invalid plan.
- `plan run` did not enforce the same validation gate before execution.
- `--model claude-haiku-4-5` did not override task `model_hint`.
- Model alias resolution for `sonnet` produced a different result than `config models route sonnet`.

## Path 6: Manual Repair And Runner Execution

I repaired the temp plan only, to test the runner:

- Added `role = "implementer"` to T1/T2.
- Added `role = "auditor"` to T3.
- Replaced `sonnet`/`haiku` with `claude-sonnet-4-6`.

Validation after manual repair:

```text
0 diagnostics in 1 plan
```

First run after repair failed with strict resume drift because the prior failed run left
state files:

```text
resume validation failed; aborting run error=3 task(s) drifted since the last run
```

There is no obvious `plan run --fresh` or `--reset-state` flag. I moved temp state files
under `.roko/state/archive-e2e/` and reran.

Runner outcome:

- T1 completed.
- T2 made the requested code changes but failed rung 7.
- T2 auto-fix ran repeatedly and did not fix the root failures.
- T3 never ran.
- Final summary said `Plan complete: 1/3 tasks, $0.22, 170s`.
- The plan row still said `greet-function - 0/3 tasks`, contradicting run-state.

Actual code result:

- `src/lib.rs` had `greet(name: &str) -> String`.
- It had four focused tests.
- `cargo test --lib` passed 5 tests.
- Full `cargo test` failed due a doctest import error.
- `cargo clippy -D warnings` failed due the uppercase random suffix in the temp package name.

Gate failures:

- Doctest failed because the doc example used `greet("world")` without importing the
  crate function.
- Clippy failed because `cargo init` used a temp path with uppercase random chars, yielding
  crate name `roko_e2e_rust_yW9yhe`, which violates `non_snake_case` under `-D warnings`.

Stub gates marked as pass:

- `symbol`: `stub gate; no SymbolManifest wired into rung 3`
- `generated_test:cargo`: `stub gate; generated test artifacts not wired`
- `verify_chain`: `stub gate; no verify script wired into rung 4`
- `fact_check`: `stub gate; no fact-check content wired into rung 5`
- `llm_judge`: `stub gate; no judge payload wired into rung 6`
- `integration:build_test`: `stub gate; no integration scenario wired into rung 6`

Run state:

```json
{
  "tasks_total": 3,
  "tasks_completed": 1,
  "tasks_failed": 1,
  "total_tokens_in": 66,
  "total_tokens_out": 5170,
  "total_cost_usd": 0.21700025,
  "total_agent_calls": 5,
  "completed_tasks": {"greet-function": ["T1"]}
}
```

Cost logs:

- `.roko/state/run-state.json` total cost: `$0.21700025`
- `roko status` total cost: `$0.7518`
- `.roko/learn/costs.jsonl` double-logged each runner attempt once as success and once
  as failure, which explains the inflated status total.
- Model recorded as `unknown-model` in runner cost logs despite tasks using `claude-sonnet-4-6`.

Issue:

The runner can get real work done, but only after manual plan repair and state cleanup.
It records contradictory status, includes stub gates as passing evidence, and loops on
fixable validation failures.

## Path 7: Top-Level `roko run` V2

Clean workspace:

```sh
cargo init --lib --name roko_e2e_run /private/tmp/roko-e2e-run-1777399243
roko init --profile rust .
```

Command:

```sh
roko --no-serve --timing --model claude-haiku-4-5 --effort low run "Add a public multiply(...)"
```

Outcome:

```text
model claude-sonnet-4-20250514
creating agent via provider adapter model_key="claude-sonnet-4-6" provider=anthropic_api
workflow halted: Missing API key: env var ANTHROPIC_API_KEY not set
```

Same result with `--model glm-5-1`.

Issue:

The default top-level workflow ignores `--model`, chooses Anthropic API instead of the
usable Claude CLI, and fails immediately on machines without `ANTHROPIC_API_KEY`.
This is likely the first path many users will try.

## Path 7b: `config migrate` Then Top-Level `roko run`

Commands:

```sh
roko config migrate --dry-run
roko config migrate
roko --no-serve --timing --model claude-sonnet-4-6 --effort low run \
  "Say exactly roko migrated run ok and do not edit files."
```

Outcome:

- Dry run proposed adding `schema_version = 2`, `[providers.claude_cli]`, and
  `[models.claude-sonnet-4-6]`.
- Non-interactive `roko config migrate` prompted `Apply changes? [y/N]` and exited
  cancelled. I reran it in a tty and accepted with `y`.
- After migration, top-level `run` selected `provider=claude_cli` instead of the missing
  Anthropic API route.
- The migrated `roko.toml` still kept the default `[[gate]] kind = "shell"` with
  `program = "true"`.
- The no-edit prompt produced an empty implementer turn, then entered autofix and repeated
  the implementer/autofix loop. I killed the temp run after collecting that evidence.
- Source files in the temp repo were unchanged by this no-edit migrated run.

Issue:

`config migrate` fixes one important provider-routing failure for v2 `run`, but it does
not fix the first-run contract. Fresh `init` still creates old/no-op config, migration is
interactive without an obvious non-interactive `--yes` path, the default gate remains
meaningless, and the workflow can still enter autofix loops even for a no-edit prompt.

## Path 8: Top-Level `roko run --engine legacy`

Command:

```sh
roko --no-serve --timing --model claude-haiku-4-5 --effort low run --engine legacy "Add a public subtract(...)"
```

Outcome:

- Started `claude_cli:claude-haiku-4-5`.
- Used tools: `Read`, `Glob`, `Edit`, `Bash`, `Bash`, `Bash`, `Read`.
- Completed in 24s.
- Added `subtract(left: u64, right: u64) -> u64`.
- Added three focused tests.
- Manual `cargo test` passed.

But:

- The Claude Code Bash hooks blocked `cargo check` with `BLOCKED: branch rename forbidden in plan worktrees`.
- The Roko gate was still `[PASS] shell:true`, so Roko marked the run successful even though
  the agent's internal verification was blocked.
- `.roko/events.jsonl` persisted raw Claude stream JSON, including assistant thinking and
  protocol signatures.
- Legacy OpenAI path failed with `you must provide a model parameter`, but Roko still printed
  `gates: [PASS] shell:true`.

Issue:

Legacy is the closest path to "works" for a small local change, but it is not safe to treat
as done because success is backed by a no-op gate, and internal verification can be blocked
without changing the run verdict.

## Path 9: Positional One-Shot Prompt

Commands:

```sh
roko --no-serve --timing --model claude-haiku-4-5 "Say exactly roko positional prompt ok..."
roko --no-serve --timing --model gpt-4o "Say exactly roko gpt flag ok..."
roko --no-serve --timing --model glm-5-1 "Say exactly roko glm flag ok..."
roko --no-serve --timing --model llama32 "Say exactly roko llama flag ok..."
```

Outcome:

- All completed successfully.
- All reported `roko - auth: glm-5.1 (OpenAI-compat)`.
- All final usage lines reported `glm-5.1`, regardless of requested model.
- Each also logged an unrelated `creating agent via provider adapter model_key="claude-sonnet-4-6" provider=anthropic_api` line.

Issue:

The positional prompt path ignores the model flag and has contradictory provider logs.
This path may feel healthy to a user while silently using a different provider/model.

## Path 10: Provider Tests And Model Routing

Provider test commands:

```sh
roko config providers test claude_cli --workdir /private/tmp/roko-e2e-rust-yW9yhe --model claude-haiku-4-5 --json
roko config providers test openai --workdir /private/tmp/roko-e2e-rust-yW9yhe --model gpt-4o --json
roko config providers test zai --workdir /private/tmp/roko-e2e-rust-yW9yhe --model glm-5-1 --json
roko config providers test moonshot --workdir /private/tmp/roko-e2e-rust-yW9yhe --model moonshot-kimi-k2-0905 --json
roko config providers test ollama --workdir /private/tmp/roko-e2e-rust-yW9yhe --model llama32 --json
```

Outcomes:

- `--json` was ignored; output was human-readable text.
- Claude CLI passed and used the local `claude` command.
- OpenAI `gpt-4o` returned non-empty content and worked.
- ZAI returned 200 OK, token usage, empty content, and was marked working.
- Moonshot returned 200 OK, token usage, empty content, and was marked working.
- Ollama was requested as `llama32` but tested `gemma4:26b-a4b-it-q8_0`.

Root cause in source:

- `crates/roko-cli/src/commands/config_cmd.rs::cmd_provider_test` ignores the CLI
  `--model` value and calls `select_provider_test_model(&config, provider_name)`.
- Provider tests do not require non-empty content before declaring success.

Model route commands:

```sh
roko config models route sonnet --explain
roko config models route haiku --explain
roko config models route claude-sonnet-4-6 --explain
roko config models route gpt-4o --explain
roko config models route glm-5-1 --explain
roko config models route llama32 --explain
```

Outcome:

- Every route selected `claude-sonnet-4-6`.
- The final provider printed as `anthropic`, not `claude_cli`.

Root cause in source:

- `cmd_model_route` treats the requested model as `previous_model`, then asks the cascade
  router to choose among all candidates. It is a recommendation command, not an exact
  model resolver, but the CLI shape reads like exact model routing.

Issue:

The provider/model surface cannot currently be used to prove that a model flag will be
honored by actual execution.

## Path 11: Resume, Status, Plan List, Plan Show

Resume:

```sh
roko --no-serve --timing resume
roko --no-serve --timing resume greet-function
```

Both failed:

```text
error: cannot read directory ./plans: No such file or directory
```

Root cause in source:

- `crates/roko-cli/src/main.rs` implements `roko resume` as sugar for plan run, but
  hardcodes `let plan_dir = workdir.join("plans")`.
- PRD-generated plans live under `.roko/plans`.

Status after the failed runner:

- Reported cost totals and gate thresholds.
- Reported `workspace: 0 plan(s) in executor snapshot`.
- Reported most recent episode `(none)`.
- Did not line up with `.roko/state/run-state.json`, which had one completed task.

Plan list/show:

- `roko plan list .roko/plans` failed because `plan list` does not accept a path.
- `roko plan show .roko/plans/greet-function` failed because it expects an ID.
- `roko plan list --repo /private/tmp/roko-e2e-rust-yW9yhe` found the plan but showed
  `pending 0/3`.
- `roko plan show greet-function --repo ...` found the plan and paths.

Issue:

State, resume, plan list, and plan run disagree about the canonical plan root and current
task status.

## Highest Priority Findings

### P0. Model Selection Is Not A Contract

Observed across commands:

- `prd draft new --model claude-haiku-4-5`: honored.
- `prd plan --model claude-haiku-4-5`: ignored, used Opus.
- `plan regenerate --model claude-haiku-4-5`: ignored, used Opus.
- `plan run --model claude-haiku-4-5`: ignored in favor of task `model_hint`.
- `run --model claude-haiku-4-5`: ignored, chose Anthropic API Sonnet.
- positional prompt `--model gpt-4o`, `--model llama32`: ignored, used GLM.
- `config providers test --model llama32`: ignored, tested provider default Gemma.
- `config models route gpt-4o`: selected Sonnet.

Fix:

Create one execution selection contract used everywhere. Suggested precedence:

1. Explicit CLI `--model` is a hard override for the current command.
2. Task `model` or `model_hint` applies only when there is no CLI override.
3. Role config model applies only when neither of the above exists.
4. Cascade router recommendation applies only when no explicit model is present.
5. Project default is last fallback.

The returned `EffectiveModelSelection` should include:

- requested model
- resolved model key
- provider kind
- provider config source
- slug sent to backend
- reason/preference source
- whether this was hard override, task hint, role config, router, or default
- whether the provider is executable in the current environment

Every command should print and persist this exact structure.

### P0. Generated Plans Are Not Self-Executable

Roko generated a plan that failed its own validator, and regenerate failed to repair it.

Fix:

- Update PRD/plan prompts to require `role` and configured full model keys.
- Add deterministic post-processing for aliases like `sonnet` and `haiku`, or reject
  them before writing.
- Run `plan validate` automatically after `prd plan`, `plan generate`, and `plan regenerate`.
- If validation fails, mark artifact generation failed even when the agent process exited 0.
- Do not allow `plan run` or `plan run --dry-run` to proceed on invalid plans.

### P0. The Default Runner Path Fails Before Doing Work

`roko run` v2 chooses Anthropic API Sonnet despite a usable Claude CLI and an explicit
model flag.

Fix:

- Route `run` v2 through the same selection contract as every other command.
- If the chosen provider is unavailable but a compatible local CLI provider exists, either
  use the local provider based on config or fail with a targeted message explaining how to
  choose one.
- Add an E2E test where `ANTHROPIC_API_KEY` is absent, `claude` is installed, and
  `--model claude-haiku-4-5` must use Claude CLI.

### P0. Gates Are Not Truthful

Stub gates pass, no-op `shell:true` passes, and legacy run can be marked successful despite
the agent's internal `cargo check` being blocked.

Fix:

- Stub gates must be `skipped/not_wired`, not passed.
- `roko init --profile rust` should default to real `cargo check`, `cargo test`, and
  `cargo clippy` gates.
- `shell:true` should be reserved for explicit smoke/test fixtures, not generated projects.
- Gate summaries should distinguish code failures from environment/setup failures.
- Auto-fix should receive a compact failure object with concrete root causes, not a giant
  mixed gate dump.

### P1. Telemetry Is Not Truthful

Observed:

- PRD/plan and regenerate logged zero cost despite Claude result cost metadata.
- Runner costs were double-counted in `.roko/learn/costs.jsonl`.
- Runner cost model was `unknown-model`.
- Tool-call counts were sometimes zero despite visible tools.
- Raw stream JSON, assistant thinking metadata, and protocol signatures were dumped to
  terminal/events.

Fix:

- Parse Claude CLI `result` and assistant `usage` events into the shared usage model.
- Store unknown usage as null/unknown, not numeric zero.
- Emit one cost event per agent attempt, then a separate gate outcome event without cost.
- Preserve raw streams only as scrubbed sidecars, not as default terminal output or giant
  inline event payloads.
- Store tool names/counts separately from text output.

### P1. Resume And Status Views Disagree

Resume defaults to `./plans`; PRD plans live under `.roko/plans`. `status`, `plan list`,
and `run-state.json` report different task completion status.

Fix:

- Normalize canonical plan root through `workspace_paths::plans_dir`.
- Make `roko resume` use the same plan root as PRD-generated plans.
- Add `plan run --fresh` or `plan run --reset-state` for deliberate state replacement.
- Update `status` and `plan list` to read the same run-state/executor projection.

## Shortest Path Fix Order

### M0-0: Execution Contract Repair

This should be inserted before or at the very start of the current M0 batch plan.

1. Implement one model/provider selection contract and use it in:
   - positional prompt
   - `run`
   - `run --engine legacy`
   - `prd draft`
   - `prd plan`
   - `plan generate`
   - `plan regenerate`
   - `plan run`
   - provider tests
2. Thread CLI `--model`, `--effort`, `--repo`, `--workdir`, and `--resume` through every
   command that starts an agent.
3. Make `config models route` either resolve the requested model exactly or rename it to
   `recommend`.
4. Make provider tests honor `--model`, honor `--json`, and fail on empty content unless the
   user asks only for transport connectivity.
5. Fix PRD/plan generation so generated `tasks.toml` is valid before it is accepted.
6. Remove pass status from stub gates and generated no-op gates.
7. Make `roko init` emit schema v2 provider/model tables, or make first run block on an
   explicit migration/preflight with non-interactive confirmation support.
8. Deduplicate cost/usage events and parse Claude CLI cost from result events.
9. Fix `resume` to use `.roko/plans` and add a fresh-run escape hatch.

### Then M0-A: Interactive Session Parity

Proceed with the existing `ChatAgentSession` plan after model/provider selection is reliable.
Otherwise the session will still silently use the wrong model or provider.

### Then M0-B: Grounded Artifact Generation

Proceed with repo context packs and artifact validators after generated plans are valid and
model selection is deterministic. Otherwise the validators will be fighting unstable command
behavior.

## Acceptance Proof Suite

Before calling Roko Mori-parity M0, these should pass from clean temp repos:

1. `roko init --profile rust .` creates schema v2 provider/model config and real Rust
   gates, not only `true`.
2. `roko --model claude-haiku-4-5 prd draft new foo` uses `claude-haiku-4-5`.
3. `roko --model claude-haiku-4-5 prd plan foo` uses `claude-haiku-4-5` or loudly explains
   why a planner override is required.
4. `roko prd plan foo` writes a plan that passes `roko plan validate`.
5. `roko plan run --dry-run .roko/plans/foo` refuses invalid plans and shows validation
   diagnostics.
6. `roko plan run .roko/plans/foo` can run a one-function Rust plan to all green gates.
7. `roko run --model claude-haiku-4-5 "small change"` uses Claude CLI when that is the
   configured/available provider and Anthropic API lacks a key.
8. `roko run --engine legacy` cannot pass on `shell:true` unless that gate was explicitly
   configured by the user.
9. `roko --model gpt-4o "say ok"` either uses OpenAI `gpt-4o` or fails before spending any
   non-requested provider call.
10. `roko config providers test ollama --model llama32 --json` tests `llama32` and emits JSON.
11. Empty provider-test content is not marked as a full success.
12. `roko resume` finds `.roko/plans` generated by `prd plan`.
13. `status`, `plan list`, and `run-state.json` agree on completed/failed task counts.
14. Cost totals are not zero when Claude result metadata has costs.
15. Cost totals are not double-counted between agent attempts and gate records.

## Bottom Line

The best possible solution is still to wire existing pieces, not build new architecture.
But the next pass should not start by adding more PRD polish or a bigger runtime. It should
first make Roko's execution contract true:

- the model requested is the model used
- the provider selected is available
- a generated plan is runnable by Roko
- a passed gate means a real gate passed
- a successful run has truthful cost, model, provider, tool, and state records

Once those are true, the earlier two-rail M0 becomes credible.

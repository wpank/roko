# E06 — Compose / Prompt Unification

> Epic work-breakdown · native task schema (`crates/roko-cli/src/task_parser.rs::TaskDef`)
> Source doc: `tmp/status-quo/34-COMPOSE-PROMPTS.md` (verified 2026-07-08 @ HEAD 5852c93c05)
> Depends on: **E01** (kernel/type + config plumbing prerequisites)
> Status: GAP — no existing plan under `plans/` targets prompt-assembly unification.

## Problem

`roko plan run` defaults to **Runner v2** (`PlanEngine::RunnerV2` is `#[default]`, main.rs:1300-1303).
Runner v2 builds each task's prompt with a **CLI-side `PromptAssembler`** (`dispatch/prompt_builder.rs:717`,
constructed at `factory.rs:87`, invoked at `dispatch/mod.rs:158`) that hand-rolls markdown and
**never** touches the canonical roko-compose stack. So on the default path the entire 12-slot
`SystemPromptBuilder` / `RoleSystemPromptSpec` / `PromptComposer` machinery — U-shape placement,
affect modulation, pheromones, section-effectiveness bumps, cache markers, VCG/greedy auction —
**does not run**. It only fires on two non-default paths (orchestrate inline + `RoleSystemPromptSpec`).

Compounding this: **four parallel assembly surfaces** exist, the **VCG auction is unreachable**
(empty `learning_bidders`, zero `update_bidders` callers, warmup threshold 10 never crossed), and
one surface (`templates/assembly.rs`) is fully **dormant** (zero callers).

## Assembly-surface census (the 4 parallel surfaces)

| # | Surface | File:line | Delegates to canonical builder? | Live callers | Status |
|---|---|---|---|---|---|
| A | `SystemPromptBuilder` (12-slot canonical builder) | `roko-compose/src/system_prompt_builder.rs:62` | — (is the canonical builder) | via C + orchestrate inline | ✅ wired (non-default only) |
| C | `RoleSystemPromptSpec::build*` → builder + `PromptComposer` | `roko-compose/src/role_prompts.rs:264,451,467` | **yes** (wraps A + `PromptComposer`) | `prompting.rs:71` (`build_role_system_prompt`), `run.rs:1517`, `dispatch_helpers.rs:133`, `prompt_helpers.rs:94`, `orchestrate.rs:20545` | ✅ wired |
| D | **`PromptAssembler` (CLI runner-v2)** | `roko-cli/src/dispatch/prompt_builder.rs:717` | **NO — self-contained markdown shortcut** | **Runner v2 `Dispatcher` — the DEFAULT `plan run`** (`dispatch/mod.rs:158`, `factory.rs:87`) | 🟡 live but bypasses canonical stack |
| E | `PromptAssembler` (compose templates) | `roko-compose/src/templates/assembly.rs:42,96` | no (own knapsack over `RolePromptTemplate::sections`) | **zero** (grep in roko-cli → 0 hits) | 🔌 dormant / dead export |

Two degenerate one-offs (out of scope for the 4-surface unify, tracked as follow-ups): roko-serve
`dispatch.rs:1999` (`SystemPromptBuilder::new(prompt).build()`, 1 layer) and roko-graph `ComposeCell`
(`{{var}}` string substitution) + `system-prompt-builder` `PassthroughCell` stub.

## VCG warmup — why it can never fire

`CompositionStrategy::Auto` flips to VCG only when every active bidder has ≥ `DEFAULT_VCG_WARMUP_OBSERVATIONS = 10`
observations (`strategy.rs:10,79`). But `PromptComposer::new()` is built with an **empty**
`learning_bidders` map (`prompt.rs:634`), and the only `update_with_cost`/`update_bidders` callers
are inside `prompt.rs` itself (dormant foraging pre-pass) + unit tests (`prompt.rs:2126`, `auction.rs:531`).
`with_learning_bidders` (`prompt.rs:680`) and `with_strategy` (`prompt.rs:657`) have **zero non-test callers**.
Result: `min().unwrap_or(0) = 0 < 10` → `DensityGreedy` 100% of the time. A VCG payment summary is
still computed for diagnostics, so signals *look* auction-aware while allocation is pure greedy.

## Decision (recommended canonical surface)

**Canonical = C (`RoleSystemPromptSpec` → A + `PromptComposer`), reached via the existing CLI wrapper
`roko_cli::prompting::build_role_system_prompt` (prompting.rs:71).** It already delegates to the 12-slot
builder + composer, is `#[must_use]`, takes `(AgentRole, TaskContext, tools_csv, PromptBuildOptions)`,
and is the shape every non-default path already uses. Route Runner v2's `Dispatcher` through it,
retire surface D's hand-rolled markdown, and delete dormant surface E. This collapses 4 surfaces → 1
canonical entrypoint (A+C) without inventing new abstractions. The v2 `ComposeProtocol` (surface-F /
graph-cell) work is explicitly deferred to a later epic.

## Task DAG

```
E01 ─┬─> E06-T01 (decide + ADR) ─┬─> E06-T03 (route runner→canonical) ─┬─> E06-T04 (retire surface D)
     │                           │                                     └─> E06-T05 (dedupe section-effectiveness)
     ├─> E06-T02 (delete surface E, dormant) [independent]
     └─────────────────────────> E06-T06 (persist LearningBidders store)
                                       └─> E06-T07 (call update_bidders post-gate — closes VCG warmup)
                                       └─> E06-T08 (config knob composition_strategy/warmup)
E06-T03 + E06-T04 + E06-T02 ─────> E06-T09 (verify single entrypoint remains)
```

## Tasks

### E06-T01 — Decide canonical surface + write ADR
- **tier**: planning · **role**: architect · **files**: `tmp/status-quo/backlog/decisions/E06-canonical-surface.md`
- **depends_on**: `["E01"]`
- Record the decision above as an ADR: canonical = C via `build_role_system_prompt`; surface D routed then retired; surface E deleted; VCG either warmed (T06/T07) or, if the ADR rejects VCG, downgraded to diagnostics-only (drop the `Auto`→`Vcg` flip, keep the payment summary). The ADR fixes the branch for T03/T07.
- **acceptance**: ADR file exists naming the canonical surface, the routing plan for D, and an explicit VCG verdict (warm vs downgrade).
- **verify**: `test -f tmp/status-quo/backlog/decisions/E06-canonical-surface.md && rg -q 'canonical' tmp/status-quo/backlog/decisions/E06-canonical-surface.md`

### E06-T02 — Delete dormant surface E (`templates/assembly.rs` `PromptAssembler`)
- **tier**: mechanical · **role**: refactorer · **files**: `crates/roko-compose/src/templates/assembly.rs`, `crates/roko-compose/src/templates/mod.rs`, `crates/roko-compose/src/lib.rs`
- **depends_on**: `["E01"]` (independent of T01)
- Confirm zero non-test callers, then remove the module + re-exports. Keep `common.rs` (`PromptBudget`, `adaptive_budget_for`) which is shared.
- **acceptance**: `templates/assembly.rs::PromptAssembler` gone; workspace compiles; no `assemble_from`/`TemplateAssembly` refs remain.
- **verify**: `! rg -n 'templates::assembly|assembly::PromptAssembler|assemble_from' crates/ --glob '!target/**'` then `cargo check -p roko-compose`

### E06-T03 — Route Runner v2 `Dispatcher` prompt through the canonical surface
- **tier**: complex · **role**: implementer · **files**: `crates/roko-cli/src/dispatch/prompt_builder.rs`, `crates/roko-cli/src/dispatch/mod.rs`
- **depends_on**: `["E06-T01"]`
- Replace the hand-rolled `PromptAssembler::assemble` body (prompt_builder.rs:774) with a call to `roko_cli::prompting::build_role_system_prompt(role, task_context, tools_csv, options)` — mapping `PromptContext`/`TaskDef` into `TaskContext` + `PromptBuildOptions`. The default `plan run` must now emit a `composition_manifest` and exercise the 12-slot builder.
- **acceptance**: `rg 'build_role_system_prompt|RoleSystemPromptSpec|SystemPromptBuilder' crates/roko-cli/src/dispatch/prompt_builder.rs` > 0; `roko plan run` produces a prompt with U-shape/affect/pheromone sections.
- **verify**: `rg -q 'build_role_system_prompt|RoleSystemPromptSpec' crates/roko-cli/src/dispatch/prompt_builder.rs` + `cargo test -p roko-cli`

### E06-T04 — Retire surface D authorship (fold or delete the markdown path)
- **tier**: focused · **role**: refactorer · depends_on `["E06-T03"]`
- Once T03 proves the canonical path, delete the now-dead markdown authorship in `prompt_builder.rs` (role/task/files/acceptance section builders) and the parallel `apply_section_effectiveness` (prompt_builder.rs:927). `PromptContext`/`AssembledPrompt` plumbing may stay as adapter types.
- **acceptance**: no hand-rolled `# Role`/`# Task` string authorship remains; single builder reachable from the runner.
- **verify**: `! rg -n 'fn assemble\b' crates/roko-cli/src/dispatch/prompt_builder.rs || rg -q 'build_role_system_prompt' crates/roko-cli/src/dispatch/prompt_builder.rs`

### E06-T05 — De-duplicate section-effectiveness
- **tier**: focused · **role**: refactorer · depends_on `["E06-T03"]`
- Collapse the CLI copy (`prompt_builder.rs:927 apply_section_effectiveness`) onto the compose `effective_priority` (`system_prompt_builder.rs:745`) so a single implementation runs on the canonical path.
- **acceptance**: one section-effectiveness implementation reachable from the runner.
- **verify**: `rg -c 'apply_section_effectiveness' crates/roko-cli/src` returns 0

### E06-T06 — Persist LearningBidders store
- **tier**: focused · **role**: implementer · **files**: `crates/roko-cli/src/orchestrate.rs` (or runner post-processing), new `.roko/learn/attention-bidders.json`
- **depends_on**: `["E01"]`
- Add load/save of a `HashMap<AttentionBidder, LearningBidder>` under `.roko/learn/attention-bidders.json`; construct the composer via `PromptComposer::with_learning_bidders` (prompt.rs:680) on the canonical path.
- **acceptance**: composer built `with_learning_bidders`; store file created on run.
- **verify**: `rg -q 'with_learning_bidders' crates/roko-cli/src` + `test -f .roko/learn/attention-bidders.json` after a run

### E06-T07 — Close VCG warmup: call `update_bidders` post-gate (or downgrade per T01)
- **tier**: complex · **role**: implementer · depends_on `["E06-T06"]`
- After each gate verdict, call `composer.update_bidders(included_sections, gate_passed)` (prompt.rs:692) and persist. If the T01 ADR chose *downgrade*, instead delete the `Auto`→`Vcg` flip in `strategy.rs:79` and keep VCG as diagnostics only.
- **acceptance (warm branch)**: after ~10 tasks, `.roko/signals.jsonl` shows `selected_strategy":"vcg"`. **(downgrade branch)**: `Auto` never returns `Vcg`; diagnostics summary retained.
- **verify (warm)**: `rg -c '"selected_strategy":"vcg"' .roko/signals.jsonl` > 0 · **(downgrade)**: `! rg -n 'Vcg' crates/roko-compose/src/strategy.rs`

### E06-T08 — Expose `[prompt] composition_strategy` + `vcg_warmup_observations` config
- **tier**: focused · **role**: implementer · **files**: `crates/roko-core/src/config.rs`, canonical construction site
- **depends_on**: `["E06-T06"]`
- Add config knobs feeding `PromptComposer::with_strategy` (prompt.rs:657); default preserves current greedy behaviour.
- **acceptance**: `roko config show | grep composition_strategy` prints a value.
- **verify**: `rg -q 'composition_strategy' crates/roko-core/src/config.rs`

### E06-T09 — Verify single canonical entrypoint remains
- **tier**: mechanical · **role**: auditor · depends_on `["E06-T02","E06-T03","E06-T04"]`
- Structural sweep confirming exactly one live assembly entrypoint (A via C) and no dormant/duplicate surfaces.
- **acceptance**: surface E gone; surface D delegates to C; `SystemPromptBuilder` reachable from the runner.
- **verify**: `! rg -n 'templates::assembly' crates/` and `rg -q 'build_role_system_prompt|RoleSystemPromptSpec' crates/roko-cli/src/dispatch/prompt_builder.rs`

---

## First 3 tasks — native TOML

```toml
[meta]
plan = "E06-COMPOSE-UNIFY"
total = 9
done = 0
status = "ready"
max_parallel = 2

# ─────────────────────────────────────────────────────────────────────────────
# E06-T01: Decide the canonical prompt-assembly surface and record the ADR
# ─────────────────────────────────────────────────────────────────────────────
[[task]]
id = "E06-T01"
title = "Decide canonical prompt-assembly surface and write ADR"
status = "ready"
tier = "planning"
model_hint = "claude-sonnet-4-5"
max_loc = 80
files = ["tmp/status-quo/backlog/decisions/E06-canonical-surface.md"]
role = "architect"
depends_on = ["E01"]

[task.context]
read_files = [
    { path = "tmp/status-quo/34-COMPOSE-PROMPTS.md", lines = "118-207", why = "Live-path trace, 6-surface census, VCG path, unify checklist" },
    { path = "crates/roko-cli/src/prompting.rs", lines = "69-96", why = "build_role_system_prompt — the proposed canonical CLI entrypoint (wraps surface C)" },
    { path = "crates/roko-compose/src/role_prompts.rs", lines = "261-468", why = "RoleSystemPromptSpec::new/build/build_sections — surface C" },
]
symbols = [
    "build_role_system_prompt — pub fn(AgentRole, TaskContext, impl Into<String>, PromptBuildOptions) -> String",
    "RoleSystemPromptSpec::new — pub fn new(role: AgentRole, task_context: TaskContext, tool_csv: impl Into<String>) -> Self",
]
anti_patterns = [
    "Do NOT invent a new v2 ComposeProtocol/ComposeBid trait — that is a later epic; pick among existing surfaces A/C/D/E only.",
    "Do NOT write any Rust code in this task — the ADR is a decision doc only.",
    "Do NOT leave the VCG verdict ambiguous — the ADR MUST choose either 'warm the bidders' or 'downgrade to diagnostics'.",
]
acceptance = "ADR names the canonical surface (recommended: C via build_role_system_prompt), the plan to route/retire surface D, deletion of surface E, and an explicit VCG verdict (warm vs downgrade)."

[[task.verify]]
phase = "structural"
command = "test -f tmp/status-quo/backlog/decisions/E06-canonical-surface.md && rg -q 'canonical' tmp/status-quo/backlog/decisions/E06-canonical-surface.md"
fail_msg = "ADR file must exist and name the canonical surface"

# ─────────────────────────────────────────────────────────────────────────────
# E06-T02: Delete dormant surface E (templates/assembly.rs PromptAssembler)
# ─────────────────────────────────────────────────────────────────────────────
[[task]]
id = "E06-T02"
title = "Delete dormant templates/assembly.rs PromptAssembler (zero callers)"
status = "ready"
tier = "mechanical"
model_hint = "claude-haiku-4-5"
max_loc = 40
files = [
    "crates/roko-compose/src/templates/assembly.rs",
    "crates/roko-compose/src/templates/mod.rs",
    "crates/roko-compose/src/lib.rs",
]
role = "refactorer"
depends_on = ["E01"]

[task.context]
read_files = [
    { path = "crates/roko-compose/src/templates/assembly.rs", lines = "1-60", why = "The dormant PromptAssembler struct + impl to remove (keep nothing here)" },
    { path = "crates/roko-compose/src/templates/mod.rs", lines = "1-40", why = "Module declaration + re-export of assembly::PromptAssembler to remove" },
    { path = "crates/roko-compose/src/lib.rs", lines = "60-100", why = "Any crate-root re-export of the templates PromptAssembler to remove" },
]
symbols = [
    "PromptAssembler (templates/assembly.rs) — dormant knapsack+U-shape assembler over RolePromptTemplate::sections",
]
anti_patterns = [
    "Do NOT delete templates/common.rs (PromptBudget, adaptive_budget_for) — it is shared by live templates.",
    "Do NOT touch the CLI-side PromptAssembler in roko-cli/src/dispatch/prompt_builder.rs — that is surface D, handled by E06-T03/T04.",
    "Do NOT remove the RolePromptTemplate trait or any role template struct.",
]
acceptance = "templates/assembly.rs removed along with its module decl and re-exports; roko-compose compiles; no assemble_from / TemplateAssembly / templates::assembly references remain in the workspace."

[[task.verify]]
phase = "structural"
command = "! rg -n 'templates::assembly|assembly::PromptAssembler|assemble_from|TemplateAssembly' crates/ --glob '!target/**'"
fail_msg = "No references to the dormant templates assembly surface may remain"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-compose 2>&1"
fail_msg = "roko-compose must compile after removing the dormant surface"

# ─────────────────────────────────────────────────────────────────────────────
# E06-T03: Route Runner v2 Dispatcher prompt through the canonical surface
# ─────────────────────────────────────────────────────────────────────────────
[[task]]
id = "E06-T03"
title = "Route Runner v2 Dispatcher prompt through build_role_system_prompt (canonical)"
status = "ready"
tier = "complex"
model_hint = "claude-opus-4-1"
max_loc = 180
files = [
    "crates/roko-cli/src/dispatch/prompt_builder.rs",
    "crates/roko-cli/src/dispatch/mod.rs",
]
role = "implementer"
depends_on = ["E06-T01"]

[task.context]
read_files = [
    { path = "crates/roko-cli/src/dispatch/prompt_builder.rs", lines = "708-1010", why = "Surface D: PromptAssembler struct + assemble() hand-rolled markdown to replace with a canonical call" },
    { path = "crates/roko-cli/src/dispatch/mod.rs", lines = "108-160", why = "Dispatcher holds prompt_assembler and invokes assemble() at :158 — the live runner-v2 call site" },
    { path = "crates/roko-cli/src/prompting.rs", lines = "1-96", why = "build_role_system_prompt + build_spec + PromptBuildOptions — the canonical entrypoint to delegate to" },
    { path = "crates/roko-compose/src/role_prompts.rs", lines = "130-160", why = "TaskContext::new — needed to map PromptContext/TaskDef into the canonical inputs" },
]
symbols = [
    "PromptAssembler::assemble — pub fn assemble(&self, task: &TaskDef, ctx: &PromptContext) -> Result<AssembledPrompt, DispatchError>",
    "build_role_system_prompt — pub fn(AgentRole, TaskContext, impl Into<String>, PromptBuildOptions) -> String",
    "Dispatcher (dispatch/mod.rs:112) — holds prompt_assembler: PromptAssembler",
]
anti_patterns = [
    "Do NOT reintroduce a second parallel assembler — assemble() MUST end up calling build_role_system_prompt (or RoleSystemPromptSpec directly).",
    "Do NOT change the Dispatcher::assemble call site signature at dispatch/mod.rs:158 unless required; keep AssembledPrompt as the adapter return type.",
    "Do NOT drop context the runner already passes (files_in_scope, acceptance, prior outputs) — map them into TaskContext/PromptBuildOptions.",
    "Do NOT delete the hand-rolled markdown helpers yet — that removal is E06-T04, after this routing is proven.",
]
acceptance = "PromptAssembler::assemble delegates to the canonical builder; grep for build_role_system_prompt / RoleSystemPromptSpec / SystemPromptBuilder in prompt_builder.rs returns >0; roko-cli tests pass; default `roko plan run` now exercises the 12-slot builder."

[[task.verify]]
phase = "structural"
command = "rg -q 'build_role_system_prompt|RoleSystemPromptSpec|SystemPromptBuilder' crates/roko-cli/src/dispatch/prompt_builder.rs"
fail_msg = "Runner-v2 assembler must reference the canonical builder surface"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli 2>&1"
fail_msg = "roko-cli must compile after routing the prompt through the canonical surface"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-cli --lib dispatch 2>&1"
fail_msg = "Dispatch prompt tests must pass after routing"
```

## Follow-ups deferred to later epics (not in E06)
- Surface F: roko-graph `ComposeCell` + `system-prompt-builder` `PassthroughCell` → real cell delegating to `RoleSystemPromptSpec::build_sections` (so `--engine graph` also uses the canonical stack).
- roko-serve `dispatch.rs:1999` 1-layer builder normalisation.
- v2 `ComposeProtocol` / `ComposeBid` / `ComposeResult` protocolisation (the bid-shaped internals already exist).
- Triplicate `GateFeedback` unification (compose / gate / runner).
- `layer_count()` fix (returns ≤11, misses `tool_hints`; should be 12).

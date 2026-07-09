# C — Role Templates (Doc 03)

Parity analysis of `docs/03-composition/03-role-templates.md` vs actual codebase.

---

## C.01 — `RoleSystemPromptSpec` struct (Doc 03 §Cross-References)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Doc 03 does not describe `RoleSystemPromptSpec` directly beyond a cross-reference (line 484: "`crates/roko-compose/src/role_prompts.rs` — Role prompt spec"). Doc 13 calls it out as "RoleSystemPromptSpec (12 roles), role_prompts.rs, 462 lines, Wired into orchestrate.rs" (line 52). It is the typed entry point that carries task context, tool allowlist, role, optional skills/pheromones/affect, and cache-marker toggle into the underlying `SystemPromptBuilder`.

### What exists
`RoleSystemPromptSpec` is defined at `crates/roko-compose/src/role_prompts.rs:154` (not line 462 — that was file size in doc 13, now stale; the struct body runs `154–175`). Ten fields:

| Field | Type | Line | Purpose |
|---|---|---|---|
| `role` | `AgentRole` | 156 | Role that will run the task |
| `task_context` | `TaskContext` | 158 | Typed task + domain context (see C.02) |
| `tool_allowlist_csv` | `String` | 160 | Hosted-backend tool allowlist CSV |
| `model_hint` | `Option<String>` | 162 | Reserved for model-specific formatting |
| `extra_conventions` | `Option<String>` | 164 | Extra conventions appended after defaults |
| `extra_anti_patterns` | `Vec<String>` | 166 | Extra anti-patterns appended after defaults |
| `relevant_skills` | `Vec<Skill>` | 168 | Learned skills to inject |
| `pheromones` | `Vec<ContextChunk>` | 170 | Active-signal chunks to inject |
| `affect_state` | `Option<PadState>` | 172 | PAD state for tone/focus guidance |
| `cache_markers` | `bool` | 174 | Emit cache markers between tiers |

Builder methods (`new`, `with_extra_conventions`, `with_model_hint`, `add_anti_pattern`, `with_relevant_skills`, `with_pheromones`, `with_affect_state`, `with_cache_markers`) at `role_prompts.rs:180–242`.

Exit points into the prompt pipeline:
- `build()` at `role_prompts.rs:308` — simple rendered string
- `build_with_section_effectiveness()` at `role_prompts.rs:314` — applies learned section weights
- `build_sections()` at `role_prompts.rs:324` — structured `Vec<PromptSection>`
- `compose_with_budget()` at `role_prompts.rs:343` — budget-aware string
- `compose_with_budget_and_scorer()` at `role_prompts.rs:351` — budget-aware with explicit scorer
- `build_with_context_window()` at `role_prompts.rs:372` — enforces 30%/50% soft/hard limits of the model context window, recomposes with tighter budget when the soft limit is exceeded and returns `RokoError::BudgetExceeded` when the hard limit is exceeded

The file is 676 LOC total (not 462 — doc 03 §Status and doc 13 §2.1 both cite the stale 462 number).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.01.1 | Doc 03 front-matter says "462 lines" and doc 13 table also lists "462 lines" for `role_prompts.rs`; the file is 676 lines. Documentation metadata is stale after the `build_with_context_window`, pheromone, and section-effectiveness additions. | `docs/03-composition/03-role-templates.md:4`, `docs/03-composition/13-current-status-and-gaps.md:26,52`, `role_prompts.rs:676` | LOW |

### Verify
```bash
wc -l /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/role_prompts.rs
grep -n 'pub struct RoleSystemPromptSpec' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/role_prompts.rs
```

---

## C.02 — `TaskContext` struct (Doc 03 implicit — no explicit section)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Doc 03 does not document a `TaskContext` struct directly. Doc 13 references it implicitly under "RoleSystemPromptSpec (12 roles)". The struct is the canonical input shape to `RoleSystemPromptSpec` and drives the domain/task/context layers passed to `SystemPromptBuilder`.

### What exists
`TaskContext` at `crates/roko-compose/src/role_prompts.rs:42` with six fields (lines 42–55):

| Field | Type | Purpose |
|---|---|---|
| `task` | `String` | Current task description |
| `plan_id` | `Option<String>` | Plan identifier |
| `goal` | `Option<String>` | Active-inference goal for scoring |
| `workspace` | `Option<String>` | Workspace label or path |
| `context_layer` | `Option<String>` | Pre-assembled relevant context |
| `domain_notes` | `Option<String>` | Extra domain notes |

Builder methods (`new`, `with_plan_id`, `with_goal`, `with_workspace`, `with_context`, `with_domain_notes`) at `role_prompts.rs:58–100`. Internal helpers produce three derived strings: `task_layer()` at line 106, `domain_layer()` at line 125, `context_layer()` at line 145, which feed the builder's `with_task`, `with_domain`, and `with_context` layers (role_prompts.rs:289–299).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.02.1 | `TaskContext` is not named or described in doc 03; its fields (including `goal` used by `ActiveInferenceScorer`) are implicit. | docs/03-composition/03-role-templates.md | LOW |

### Verify
```bash
grep -n 'pub struct TaskContext\|pub fn with_plan_id\|pub fn with_goal' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/role_prompts.rs
```

---

## C.03 — `RolePromptTemplate` trait (Doc 03 §4)

- **Status**: PARTIAL
- **Priority**: P3
- **Estimated LOC**: ~10 (doc update)
- **Dependencies**: None
- **Files to modify**: `docs/03-composition/03-role-templates.md`

### What the doc says
Doc 03 §4 declares the trait at `crates/roko-compose/src/templates/mod.rs`:

```rust
pub trait RolePromptTemplate {
    fn sections(&self, context: &TemplateContext) -> Vec<PromptSection>;
    fn role_identity(&self) -> &str;
}
```

The doc also says `PlanSlice` has five fields (`plan_content`, `task_toml`, `workspace_map`, `prd_extract`, `cross_plan_context`) and `TaskEnhancements` has six (`brief`, `reviews`, `iteration_memory`, `research`, `playbook_rules`, `file_context`).

### What exists
The trait exists at `crates/roko-compose/src/templates/mod.rs:76` with a different signature than doc 03 specifies:

```rust
pub trait RolePromptTemplate {
    type Input;
    fn sections(&self, input: &Self::Input) -> Vec<PromptSection>;
    fn role_identity(&self) -> &'static str;
}
```

Differences from doc 03 §4:

| Aspect | Doc 03 | Code | Match |
|---|---|---|---|
| Associated type | None | `type Input` | NO |
| `sections` arg | `context: &TemplateContext` | `input: &Self::Input` | NO |
| `role_identity` return | `&str` | `&'static str` | PARTIAL |

`PlanSlice` at `templates/mod.rs:34` has four fields (`num`, `base`, `title`, `content`) — not the five doc 03 lists. The doc's fields `task_toml`, `workspace_map`, `prd_extract`, `cross_plan_context` are carried on per-template `*Input` structs (e.g. `ImplementerInput.tasks`, `ImplementerInput.workspace_map`), not on `PlanSlice`.

`TaskEnhancements` at `templates/mod.rs:47` has five fields (`types_to_define`, `formulas`, `imports`, `example_pattern`, `test_invariants`) that do not match doc 03's six field set (`brief`, `reviews`, `iteration_memory`, `research`, `playbook_rules`, `file_context`). The doc's `brief`/`reviews`/`research`/`file_context` are also per-template `*Input` fields (e.g. `ImplementerInput.brief`, `ImplementerInput.prev_reviews`).

Helpers exported from the module (`templates/mod.rs:94–187`): `truncate`, `truncate_tail`, `format_enhancements`, `format_files_changed`.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.03.1 | Doc 03 §4 signature omits the `type Input` associated type. | docs/03-composition/03-role-templates.md:292 | LOW |
| C.03.2 | Doc 03 §6 `PlanSlice` field list (`plan_content`, `task_toml`, `workspace_map`, `prd_extract`, `cross_plan_context`) does not match code (`num`, `base`, `title`, `content`). | docs/03-composition/03-role-templates.md:374 vs templates/mod.rs:34 | LOW |
| C.03.3 | Doc 03 §6 `TaskEnhancements` field list does not match code. Documented fields live on per-template `*Input` structs. | docs/03-composition/03-role-templates.md:387 vs templates/mod.rs:47 | LOW |

### Verify
```bash
grep -n 'pub trait RolePromptTemplate\|pub struct PlanSlice\|pub struct TaskEnhancements' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/mod.rs
```

---

## C.04 — `PromptBudget` struct (Doc 03 §2)

- **Status**: PARTIAL
- **Priority**: P1
- **Estimated LOC**: ~60 (wire budget into template hard caps, or delete unused type)
- **Dependencies**: C.05
- **Files to modify**: `crates/roko-compose/src/templates/implementer.rs`, `integration.rs`, `quick.rs`, `reviewer.rs`, `scribe.rs`, `strategist.rs`, `task_impl.rs`

### What the doc says
Doc 03 §2 declares `PromptBudget` at `crates/roko-compose/src/templates/common.rs` as the per-role budget struct with nine fields: `plan`, `workspace_map`, `prd2`, `context`, `brief`, `reviews`, `instructions`, `file_context`, `skills`.

### What exists
`PromptBudget` exists at `crates/roko-compose/src/templates/common.rs:17` with exactly the 9 documented fields (lines 17–36). The struct is `Clone, Copy, Debug, PartialEq, Eq`.

Where `PromptBudget` is actually referenced (full list):
- `common.rs:17` (definition), `common.rs:44` (return type of `budget_for`)
- `budget.rs:15,40,128` (wrapped by `AdjustedBudget`)
- `lib.rs:58` (re-export)
- `templates/mod.rs:21` (re-export)

`PromptBudget` is **not** read by any of the concrete template `sections()` implementations. Each template hardcodes its caps inline, e.g. `implementer.rs:76` hardcodes `50_000`, `implementer.rs` uses 19 `hard_cap` calls with literal integers, `reviewer.rs:130` hardcodes `50_000`, `quick.rs:83` and `:92` use `50_000` and `6_000`. The `hard_cap` counts per template (rg):

| Template | `hard_cap` call count |
|---|---|
| `assembly.rs` | 3 (in tests) |
| `integration.rs` | 10 |
| `reviewer.rs` | 14 |
| `strategist.rs` | 25 |
| `task_impl.rs` | 21 |
| `implementer.rs` | 19 |
| `quick.rs` | 13 |
| `scribe.rs` | 6 |

None of these templates read `budget_for(role)` or `PromptBudget` before calling `truncate` or `with_hard_cap`.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.04.1 | `PromptBudget` and `budget_for` form a budget table that is not wired into the templates they are meant to govern. Template caps are hardcoded literals inside each `sections()` impl. | templates/implementer.rs:76-278, templates/strategist.rs:83-247, templates/task_impl.rs:104-305, templates/scribe.rs:122-178, templates/reviewer.rs:126-254, templates/integration.rs:66-115, templates/quick.rs:79-113 | HIGH |
| C.04.2 | The doc's §2.1 allocation table claims Scribe's `prd2=16k` and Implementer's `file_context=8k`; only the Scribe hardcode at `scribe.rs:135` (`hard_cap(16_000)`) and Implementer hardcode at `implementer.rs` match by coincidence. Changing `budget_for` will not change runtime behavior. | common.rs vs template files | HIGH |

### Verify
```bash
# Templates never read PromptBudget or budget_for():
grep -rn 'budget_for\|PromptBudget' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/implementer.rs /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/task_impl.rs /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/strategist.rs /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/reviewer.rs /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/scribe.rs /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/quick.rs /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/integration.rs
# Count hardcoded hard_caps per template:
grep -c 'hard_cap\|truncate(.*,[[:space:]]*[0-9][0-9_]*)' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/*.rs
```

---

## C.05 — `budget_for(role)` helper (Doc 03 §2.1)

- **Status**: PARTIAL
- **Priority**: P1
- **Estimated LOC**: ~50 (wire into templates or callers)
- **Dependencies**: None
- **Files to modify**: `crates/roko-compose/src/templates/*.rs`

### What the doc says
Doc 03 §2.1 shows `budget_for(role)` with per-role match arms for `Implementer`, `Strategist`, `Architect | Auditor`, `Scribe`, and `_ =>` default. Doc 03's §2.1 table only covers 4 explicit arms (Implementer, Strategist, Architect|Auditor, Scribe) + default.

### What exists
`budget_for` at `crates/roko-compose/src/templates/common.rs:44` is a `pub const fn` with seven match arms, richer than doc 03:

| Role arm | Lines | Key caps |
|---|---|---|
| `Implementer` | 46–56 | plan=50k, workspace=20k, prd2=12k, brief=8k, file_context=8k, skills=8k |
| `Strategist` | 57–67 | plan=50k, workspace=20k, prd2=12k, brief=6k, file_context=0, skills=4k |
| `Architect \| Auditor` | 68–78 | plan=50k, workspace=6k, prd2=6k, file_context=6k |
| `Scribe \| Critic` | 79–89 | plan=50k, workspace=6k, prd2=16k, file_context=6k |
| `QuickReviewer` | 90–100 | plan=50k, workspace=6k, prd2=0, file_context=0, skills=0 |
| `AutoFixer` | 101–111 | plan=0, workspace=0, prd2=0, brief=0, file_context=0, skills=0, instructions=2k |
| `_` (default) | 112–123 | plan=50k, workspace=8k, prd2=6k, brief=4k, file_context=6k, skills=4k |

Call sites (verified via `grep -rn 'budget_for(' crates`):
- `budget.rs:67` (inside `adjusted_budget_for` — not called from live prompt path)
- `common.rs:244,254,266,276,282` (unit tests only)
- `budget.rs:172,180,188,218,242,243,252,265` (tests)

There are **no non-test callers of `budget_for` outside `adjusted_budget_for`**, and `adjusted_budget_for` itself has **no non-test callers** (see C.08).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.05.1 | Doc 03 §2.1 omits the `QuickReviewer` and `AutoFixer` arms. The code has 5 explicit role arms + default; the doc has only 3 + default. | docs/03-composition/03-role-templates.md:157-183 vs common.rs:44-124 | MEDIUM |
| C.05.2 | Doc 03 §2.1 puts `Scribe` alone; code groups `Scribe \| Critic`. Parity holds because Critic inherits Scribe's budget, but the doc does not mention this. | common.rs:79 vs doc §1.7 "Critic" budget emphasis | LOW |
| C.05.3 | `budget_for` has no non-test callers. The helper is dead code relative to the runtime prompt path. | budget.rs:67 (only live caller is `adjusted_budget_for`, which is also uncalled) | HIGH |

### Verify
```bash
grep -rn 'budget_for(' /Users/will/dev/nunchi/roko/roko/crates --include='*.rs' | grep -v tests | grep -v 'crates/roko-compose/src/budget.rs\|crates/roko-compose/src/templates/common.rs'
```

---

## C.06 — Role-to-template mapping (Doc 03 §1.1–§1.12)

- **Status**: PARTIAL
- **Priority**: P2
- **Estimated LOC**: ~200 (add Researcher + Conductor templates, or update doc)
- **Dependencies**: None
- **Files to modify**: new `templates/researcher.rs`, new `templates/conductor.rs`; or `docs/03-composition/03-role-templates.md`

### What the doc says
Doc 03 §1 names 12 roles: Strategist, Implementer, Architect, Auditor, QuickReviewer, Scribe, Critic, AutoFixer, IntegrationTester, Refactorer, Researcher, Conductor. Doc 13 §2.1 says "RoleSystemPromptSpec (12 roles)" is wired.

### What exists
Actual concrete `RolePromptTemplate` implementations in `crates/roko-compose/src/templates/` — 8 impls across 7 template files (via `grep 'impl RolePromptTemplate for'`):

| File | `impl RolePromptTemplate for` | Line |
|---|---|---|
| `implementer.rs` | `ImplementerTemplate` | 60 |
| `strategist.rs` | `StrategistTemplate` | 67 |
| `reviewer.rs` | `ReviewerTemplate` (variants: Architect / Auditor / Combined via enum) | 110 |
| `scribe.rs` | `ScribeTemplate` (variants: Initial / Revision / Critic via enum) | 107 |
| `quick.rs` | `QuickReviewerTemplate` | 63 |
| `quick.rs` | `QuickFixTemplate` | 169 |
| `task_impl.rs` | `TaskImplTemplate` | 87 |
| `integration.rs` | `IntegrationTemplate` | 51 |

`assembly.rs` and `prompts.rs` and `common.rs` are not role templates (they are the `PromptAssembler` layer and the enrichment step prompts and shared stanzas, respectively).

`role_identity_for(role)` at `role_prompts.rs:485` is the single source that maps `AgentRole -> identity string`. The mapping (lines 486–517):

| Doc 03 role | Code `AgentRole` | Template implementation | Source line | Match |
|---|---|---|---|---|
| Strategist | `Strategist` | `StrategistTemplate` | role_prompts.rs:487 | MATCH |
| Implementer | `Implementer` | `ImplementerTemplate` | role_prompts.rs:488 | MATCH |
| Architect | `Architect` | `ReviewerTemplate::new(Reviewer::Architect)` | role_prompts.rs:489 | MATCH |
| Auditor | `Auditor` | `ReviewerTemplate::new(Reviewer::Auditor)` | role_prompts.rs:492 | MATCH |
| QuickReviewer | `QuickReviewer` | `QuickReviewerTemplate` | role_prompts.rs:495 | MATCH |
| Scribe | `Scribe` | `ScribeTemplate::role_identity_for_variant(Initial)` | role_prompts.rs:496 | MATCH |
| Critic | `Critic` | `ScribeTemplate::role_identity_for_variant(Critic)` | role_prompts.rs:499 | MATCH (variant re-use) |
| AutoFixer | `AutoFixer` | `QuickFixTemplate` | role_prompts.rs:502 | MATCH |
| IntegrationTester | `IntegrationTester` | `IntegrationTemplate` | role_prompts.rs:503 | MATCH |
| Refactorer | `Refactorer` | `TaskImplTemplate` | role_prompts.rs:504 | PARTIAL — reuses per-task implementer prompt, not a Refactorer-specific prompt |
| Researcher | `Researcher` | *inline fallback string (no template)* | role_prompts.rs:505-508 | MISSING TEMPLATE |
| Conductor | `Conductor` | *inline fallback string (no template)* | role_prompts.rs:509-512 | MISSING TEMPLATE |

Additional role identities only reachable through the `other =>` fallback at `role_prompts.rs:513` (produces "You are an AI agent in the {label} role.") for every other `AgentRole` variant (`PrePlanner`, `DocVerifier`, `MergeResolver`, `TerminalValidator`, `GolemLifecycleTester`, `SpecDriftDetector`, `RegressionDetector`, `PerformanceSentinel`, etc. — see `crates/roko-core/src/agent.rs:582` for the full 23-variant enum).

The Reviewer file also defines a `Reviewer::Combined` variant (`reviewer.rs:19`) that is never selected through `role_identity_for` — doc 03 does not name a "Combined Reviewer" role.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.06.1 | Researcher has no `RolePromptTemplate` implementation. `role_identity_for` returns a 99-char inline fallback ("You are a researcher. Gather context..."). Doc 03 §1.11 promises "Academic rigor. Finds and cites primary sources." | role_prompts.rs:505-508 vs docs/03-composition/03-role-templates.md:102-109 | MEDIUM |
| C.06.2 | Conductor has no `RolePromptTemplate` implementation. `role_identity_for` returns a 99-char inline fallback ("You are the Conductor. Coordinate the work..."). Doc 03 §1.12 describes it as the "meta-role" for coordination. | role_prompts.rs:509-512 vs docs/03-composition/03-role-templates.md:110-116 | MEDIUM |
| C.06.3 | Refactorer role reuses `TaskImplTemplate` (per-task implementer prompt). Doc 03 §1.10 describes Refactorer as a role that "preserves behavior, improves structure, reduces duplication, respects public API" — this is not reflected in the Implementer-oriented identity text or section set. | role_prompts.rs:504 vs docs/03-composition/03-role-templates.md:94-100 | MEDIUM |
| C.06.4 | Doc 13 claims "12 role templates: Implemented". In practice, 9 concrete templates cover 10 of the 12 roles (Researcher, Conductor lack concrete templates). The claim overstates coverage. | docs/03-composition/13-current-status-and-gaps.md:467 | MEDIUM |
| C.06.5 | The 13 `AgentRole` variants beyond the 12 named roles (`PrePlanner`, `DocVerifier`, `MergeResolver`, `TerminalValidator`, `GolemLifecycleTester`, `SpecDriftDetector`, `RegressionDetector`, `PerformanceSentinel`) fall through to the generic "You are an AI agent in the {label} role" fallback. No doc coverage of how these roles are supposed to be prompted. | role_prompts.rs:513 vs roko-core/src/agent.rs:582 | LOW |
| C.06.6 | `ReviewerTemplate::Combined` variant exists in code but has no matching doc 03 role and is never selected through `role_identity_for`. | reviewer.rs:19, role_prompts.rs:485-518 | LOW |

### Verify
```bash
grep -n 'impl RolePromptTemplate for' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/*.rs
grep -n 'AgentRole::\(Researcher\|Conductor\|Refactorer\)' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/role_prompts.rs
```

---

## C.07 — `role_identity_for()` + `tool_allowlist_instructions()` (Doc 03 §4 implicit)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Doc 03 does not name `role_identity_for` or `tool_allowlist_instructions`. Doc 13 §2.2 implies their role via the "Wired" data flow ("orchestrate.rs -> RoleSystemPromptSpec::for_role(task.role) -> SystemPromptBuilder"). They are the glue between `AgentRole` and the per-template `role_identity()` trait methods, plus the tool allowlist block emitted into every prompt.

### What exists
`role_identity_for(role: AgentRole) -> String` at `crates/roko-compose/src/role_prompts.rs:485`. Resolves a role to its identity string (see mapping table in C.06). Called from:
- `crates/roko-compose/src/role_prompts.rs:273` — inside `builder_with_section_effectiveness`, the single canonical call site
- `crates/roko-cli/src/orchestrate.rs:14000` — additionally used for identity resolution outside the full builder path
- `crates/roko-compose/src/role_prompts.rs:542` — test

`tool_allowlist_instructions(tools_csv: &str) -> String` at `crates/roko-compose/src/role_prompts.rs:473`. Produces a short block — either:
- `"No hosted-backend tool allowlist was supplied. Use only the minimum tools required for the role."` when CSV is empty (line 476)
- `"Claude tool allowlist: {csv}\n\nUse only the tools granted to your role."` otherwise (line 479)

Called from `role_prompts.rs:275` (only non-test call site, routed through `SystemPromptBuilder::with_tools`). Re-exported from `lib.rs:53`. Format text hardcodes "Claude tool allowlist" — Claude-specific phrasing.

Default conventions block (`DEFAULT_CONVENTIONS_SUFFIX` at `role_prompts.rs:30`) and anti-patterns (`DEFAULT_ANTI_PATTERNS` at `role_prompts.rs:34–38`) are also applied uniformly to every prompt via `conventions_text()` (line 244) and `anti_patterns()` (line 256).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.07.1 | "Claude tool allowlist:" phrasing is hardcoded; non-Claude backends receive a Claude-specific block. `model_hint` exists (role_prompts.rs:162) but is not consulted when producing the allowlist block. | role_prompts.rs:479 | LOW |
| C.07.2 | Doc 03 does not mention `role_identity_for`, `tool_allowlist_instructions`, `DEFAULT_CONVENTIONS_SUFFIX`, or `DEFAULT_ANTI_PATTERNS`. The glue API is undocumented. | docs/03-composition/03-role-templates.md | LOW |

### Verify
```bash
grep -n 'pub fn role_identity_for\|pub fn tool_allowlist_instructions\|DEFAULT_CONVENTIONS_SUFFIX\|DEFAULT_ANTI_PATTERNS' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/role_prompts.rs
grep -rn 'role_identity_for\|tool_allowlist_instructions' /Users/will/dev/nunchi/roko/roko/crates --include='*.rs'
```

---

## C.08 — Complexity-adaptive budgets + shared stanzas + truncation helpers (Doc 03 §3, §4, §5)

- **Status**: PARTIAL
- **Priority**: P1
- **Estimated LOC**: ~80 (wire `adjusted_budget_for` into templates or remove it)
- **Dependencies**: C.04, C.05
- **Files to modify**: `crates/roko-compose/src/role_prompts.rs`, `crates/roko-compose/src/templates/*.rs`, `crates/roko-cli/src/orchestrate.rs`

### What the doc says
Doc 03 §3 specifies `adjusted_budget_for(role, complexity)` at `crates/roko-compose/src/budget.rs` with three complexity bands (`Trivial`, `Standard`, `Complex`), section-dropping on `Trivial` (prd2/context/skills zeroed; workspace_map and brief halved), and section-inflation on `Complex` (workspace_map x1.5, context x2, file_context x1.5). Doc 03 §4 lists shared stanzas `CONTEXT_LAYOUT_STANZA`, `MCP_TOOLS_STANZA`, `NITS_FORMAT`. Doc 03 §5 specifies `truncate` (head-preserving) and `truncate_tail` (tail-preserving) helpers.

### What exists
`Complexity` enum at `crates/roko-compose/src/budget.rs:23` (variants `Trivial`, `Standard` (default), `Complex`).

`AdjustedBudget` at `crates/roko-compose/src/budget.rs:38` (fields `budget`, `dropped_sections`, `cache_breaks`, `complexity`, `role`).

`adjusted_budget_for(role, complexity)` at `crates/roko-compose/src/budget.rs:66`. The implementation matches the doc's §3 spec:
- Trivial (lines 71–88): zeros `prd2`, `context`, `skills`; halves `workspace_map`, `brief`
- Standard (lines 89–91): passthrough
- Complex (lines 92–98): x1.5 `workspace_map`, x2 `context`, x1.5 `file_context`

Cache break hints at `budget.rs:108–112`: `["conventions", "workspace_map", "file_context"]` — a single hardcoded list regardless of complexity. Doc 03 §3.3 describes differentiated cache breaks per complexity (Trivial should skip `workspace_map` break); the code does not differentiate.

Call sites of `adjusted_budget_for` (all at `budget.rs:161,173,181,189,199,208,242,243,252,258,266`) are **all in `#[cfg(test)]` blocks inside budget.rs itself**. Zero non-test callers.

`Complexity` is used in one non-test location: `lib.rs:37` re-exports it (`pub use budget::{AdjustedBudget, Complexity, adjusted_budget_for};`). Nothing in `role_prompts.rs`, `orchestrate.rs`, `prompt.rs`, or `system_prompt_builder.rs` reads `Complexity` or invokes `adjusted_budget_for`. Templates do not consult it either.

Per-role application: templates use hardcoded caps that happen to approximate the base budget for the most common role for that template (e.g. the scribe template's `hard_cap(16_000)` for prd2 matches Scribe's `budget_for.prd2=16_000`). But this is not computed — it is baked into the source. `adjusted_budget_for` has no effect on any runtime output.

`cache_marker(layer_name: &str) -> String` at `budget.rs:151` returns `"<!-- cache:{layer_name} -->"`. Never called outside tests (`grep 'cache_marker(' crates` returns only `budget.rs:236` test).

Shared stanzas (doc §4):
- `CONTEXT_LAYOUT_STANZA` at `templates/common.rs:132` — emitted into every `RoleSystemPromptSpec` via `conventions_text()` at `role_prompts.rs:245`, so it is live.
- `MCP_TOOLS_STANZA` at `templates/common.rs:154` — imported only by `templates/assembly.rs:17`; not routed through `RoleSystemPromptSpec`. Live only when a caller uses `PromptAssembler` directly.
- `NITS_FORMAT` at `templates/common.rs:164` — used by `format_verdict_instructions` at `common.rs:194`, which is consumed by `QuickReviewerTemplate` at `quick.rs:119` and `ReviewerTemplate` (same pattern). Live in review paths only.

Truncation helpers (doc §5):
- `truncate(s, max_chars)` at `templates/mod.rs:94` — cuts at the last newline boundary before the limit and appends `"…[truncated N chars]"`. Doc 03 says it appends `"\n...(truncated)"` without chars count — mismatch.
- `truncate_tail(s, max_chars)` at `templates/mod.rs:114` — keeps the last `max_chars`, walks forward to a newline boundary, prepends `"…[truncated N chars]\n"`. Doc 03 says it prepends `"(truncated)...\n"` — mismatch.
- Usage counts (`grep -c 'truncate(' crates/roko-compose/src/templates/*.rs`): implementer 9, integration 3, quick 6, reviewer 9, scribe 3, strategist 11, task_impl 13. `truncate_tail` is imported nowhere outside `templates/mod.rs` (`grep -rn 'truncate_tail' crates` returns only the definition and tests).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.08.1 | `adjusted_budget_for` has zero non-test callers. Doc 13 §2.1 row "Complexity-adaptive budgets: Wired via adjusted_budget_for()" is factually false. | docs/03-composition/13-current-status-and-gaps.md:54 vs budget.rs (no external callers) | HIGH |
| C.08.2 | Cache breaks at `budget.rs:108-112` are a single hardcoded list; doc 03 §3.3 describes per-complexity cache break rules (Trivial should drop the workspace_map break). Not implemented. | budget.rs:108 vs docs/03-composition/03-role-templates.md:272-281 | LOW |
| C.08.3 | `cache_marker` helper is never called outside tests; cache markers emitted into the prompt come from `SystemPromptBuilder::with_cache_markers`, not from this helper. | budget.rs:151 | LOW |
| C.08.4 | `truncate` / `truncate_tail` return strings differ from doc 03 §5 format (`"…[truncated N chars]"` not `"\n...(truncated)"`). The doc's stated format does not match the live implementation. | templates/mod.rs:94,114 vs docs/03-composition/03-role-templates.md:345-356 | LOW |
| C.08.5 | `truncate_tail` has no callers outside the definition file — doc 03 §5 describes its use (for `gate_errors`), but nothing in the template tree calls it. | templates/mod.rs:114 | LOW |
| C.08.6 | `MCP_TOOLS_STANZA` bypasses the main `RoleSystemPromptSpec` path — only `PromptAssembler::assemble` injects it. Doc 03 §4 implies it is part of every role prompt, but when templates are used via `RoleSystemPromptSpec::build`, it is absent. | templates/assembly.rs:17 vs role_prompts.rs:269-304 | LOW |

### Verify
```bash
# adjusted_budget_for: zero non-test callers
grep -rn 'adjusted_budget_for' /Users/will/dev/nunchi/roko/roko/crates --include='*.rs' | grep -v 'crates/roko-compose/src/budget.rs'
# cache_marker: zero non-test callers
grep -rn 'cache_marker(' /Users/will/dev/nunchi/roko/roko/crates --include='*.rs' | grep -v '#\[cfg(test)\]\|crates/roko-compose/src/budget.rs'
# truncate_tail: zero uses outside mod.rs
grep -rn 'truncate_tail' /Users/will/dev/nunchi/roko/roko/crates --include='*.rs' | grep -v 'crates/roko-compose/src/templates/mod.rs'
# MCP_TOOLS_STANZA: only assembly.rs
grep -rn 'MCP_TOOLS_STANZA' /Users/will/dev/nunchi/roko/roko/crates --include='*.rs'
```

---

## Agent Execution Notes

### C.04 / C.05 — Static Role Budgets

This should be one of the first executable batches in `03`.

Recommended slice:

1. make templates derive caps from `budget_for(role)`,
2. remove duplicated literals where possible,
3. add tests for the key roles that matter in production.

Acceptance criteria:

- one obvious source of truth for static role budgets,
- reduced template drift,
- no broad rewrite of the template system just to remove literals.

### C.06 / C.07 — Role Coverage

Good outcome:

- Researcher and Conductor have real template-backed identities,
- Refactorer has role-specific wording,
- generic tool glue stops hardcoding Claude-specific language.

Do not widen this into full coverage for every tertiary `AgentRole` unless a runtime need forces it.

### C.08 — Complexity Budgets + Shared Stanzas

Treat this as runtime activation work.

Recommended slice:

1. route one live path through `adjusted_budget_for`,
2. decide what to do about `MCP_TOOLS_STANZA` on the main prompt path,
3. stop before building predictive budget systems.

# 01 — Task Execution Schema (canonical)

**Purpose.** This is the canonical reference for how a work item becomes a
roko-executable task. Every file in `tmp/status-quo/backlog/` MUST conform to
the schema documented here. The schema is not aspirational — it is exactly what
`crates/roko-cli/src/task_parser.rs` deserializes and what
`crates/roko-cli/src/plan_validate.rs` enforces. When this doc and the code
disagree, the code wins; fix this doc.

A plan is a directory under `plans/<plan-id>/` containing a single
`tasks.toml`. The runner (`plan run <dir>`) discovers every `tasks.toml`
recursively, parses each into a `TasksFile`, sorts tasks into a DAG, dispatches
agents, and runs the verify pipeline + gates per task.

---

## 1. File shape

```toml
[meta]            # exactly one, required
# ... meta fields

[[task]]          # one or more
# ... task fields
[task.context]    # optional, one per task
[[task.verify]]   # zero or more per task
[task.acceptance_contract]   # optional, one per task
```

Source of truth: `TasksFile { meta: TaskMeta, tasks: Vec<TaskDef> }`
(`task_parser.rs:697-702`; note `#[serde(rename = "task")]` — the array key is
`task`, rendered as `[[task]]`). Parsing entry points: `TasksFile::parse`
(reads a path, `:706`), `parse_str` (`:713`, stamps `sequence` in author
order), `parse_agent_output` (`:723`, strips ```` ``` ```` fences).

---

## 2. `[meta]` — `TaskMeta` (`task_parser.rs:21-43`)

| Field | Type | Req? | Default | Meaning |
|---|---|---|---|---|
| `plan` | String | **required** | — | Plan id. Used as the plan key, PRD lookup key, and cross-plan dependency target. |
| `iteration` | u32 | optional | `0` | Replan/iteration counter. |
| `total` | u32 | optional | `0` | Declared task count (informational; not enforced). |
| `done` | u32 | optional | `0` | Completed count (informational). |
| `status` | String | optional | `""` | Lifecycle: see §9. `ready` / `superseded` / `archived` / `done`. |
| `superseded_by` | Option\<String\> | optional | none | Plan id that replaces this one (set when `status = "superseded"`). |
| `max_parallel` | u32 | optional | `1` | Max tasks dispatched concurrently (`default_max_parallel`, `:45`). |
| `estimated_total_minutes` | u32 | optional | `0` | Budget hint. |
| `skip_enrichment` | bool | optional | `false` | When `true`, skip the enrichment pipeline and go straight to implementing. Set this for hand-authored plans that are already complete. |

Extra meta keys are read by the validator but not by the runtime struct:
`queue_kind` / `queue_schema` / `kind = "architecture_implementation"`
trigger stricter "architecture queue" validation (`plan_validate.rs:683-689`,
§8), and `source_prd` is used to link a plan back to its PRD.

---

## 3. `[[task]]` — `TaskDef` (`task_parser.rs:50-105`, defaults in `TaskDefSerde` `:115-163`)

Full field list, in schema order:

| Field | Type | Req? | Default | Meaning |
|---|---|---|---|---|
| `id` | String | **required** | — | Stable task id, unique within the plan. Referenced by `depends_on`. |
| `title` | String | **required** | — | One-line human summary. Rendered as the prompt header. |
| `description` | Option\<String\> | optional* | none | Longer prose. *`validate_structure` treats empty description as a missing required field (`:972`). >500 words → quality warning (`:823`). |
| `role` | Option\<String\> | optional | none→`implementer` | Agent role (see valid set §5). Drives system-prompt template + default denied tools. Validator errors if missing (`PLAN_003`) or unknown (`PLAN_008`, warning). |
| `status` | String | optional | `"ready"` (`default_status` `:333`) | Per-task state: `pending`/`ready`/`active`/`done`/`blocked`/`skipped`. Only `ready` is dispatchable (`is_ready`, `:455`). |
| `tier` | String | optional | `"focused"` (`default_tier` `:337`) | Complexity tier: `mechanical`/`focused`/`integrative`/`architectural`. See §4. |
| `frequency` | Option\<OperatingFrequency\> | optional | inferred | `gamma`/`theta`/`delta` override. If absent, inferred from description keywords (`:377-399`). |
| `model_hint` | Option\<String\> | optional | tier default | Preferred model. Short aliases `haiku`/`sonnet`/`opus` normalize (`normalize_model_alias`, `:633`). |
| `replan_strategy` | Option\<ReplanStrategy\> | optional | none | Per-task replan override (e.g. `decompose`). |
| `max_loc` | Option\<u32\> | optional | none | Max lines of change. `0` is normalized to `None` = unlimited (`normalize_max_loc`, `:351`). Injected into the prompt. |
| `files` | Vec\<String\> | optional | `[]` | Files this task modifies. Alias: `write_files` (`:135`). Required for `implementer` role (`PLAN`/schema). Checked against workspace by `validate_file_references`. |
| `allowed_tools` | Option\<Vec\<String\>\> | optional | none | Whitelist of tool names. |
| `denied_tools` | Option\<Vec\<String\>\> | optional | role default | Blocklist. If unset, filled from `denied_tools_for_role(role)` (`apply_role_tool_defaults`, `:612`). |
| `mcp_servers` | Option\<Vec\<String\>\> | optional | none | MCP server names this task needs. |
| `depends_on` | Vec\<String\> | optional | `[]` | Task ids (same plan) that must finish first. DAG edges. |
| `depends_on_plan` | Vec\<String\> | optional | `[]` | Plan ids that must complete before this task dispatches. |
| `split_into` | Option\<Vec\<String\>\> | optional | none | Subtask ids created when this task is decomposed at runtime. |
| `context` | Option\<TaskContext\> | optional | none | Surgical context block (§6). Absence → quality warning (`MissingReadFiles`). |
| `verify` | Vec\<VerifyStep\> | optional | `[]` | Machine-checkable verification pipeline (§7). Absence → quality warning (`MissingVerify`); required for `implementer`. |
| `timeout_secs` | u64 | optional | `600` (`default_timeout_secs` `:341`) | Per-task agent timeout. Must be > 0 (schema check `:936`). |
| `max_retries` | u32 | optional | `3` (`default_max_retries` `:345`) | Retry attempts on failure. |
| `acceptance` | Vec\<String\> | optional | `[]` | Free-form human-readable done criteria (legacy). Rendered in prompt only when `verify` is empty. |
| `acceptance_contract` | Option\<AcceptanceContract\> | optional | none | Typed done-gate (§8). Validated shape at `plan validate` time. |
| `domain` | Option\<TaskDomain\> | optional | config default | Work domain → gate set + git policy (§5.2). |
| `sequence` | usize | (internal) | stamped | Author-order index, stamped by `parse_str`; used to break DAG ties so tasks run in written order, not alphabetical. Not authored by hand. |

Also accepted by the validator (not on the runtime struct): `gate_rung`
(integer `0..=6`; `PLAN_007` if out of range, `PLAN_011` warns if `0` with no
verify), `prompt` (scanned for greenfield phrasing), and `deferral.*` metadata
(required for architecture-queue tasks).

---

## 4. Tier taxonomy

Valid tiers: `mechanical`, `focused`, `integrative`, `architectural`
(`VALID_TIERS`, `plan_validate`/`task_parser.rs:866`). Default `focused`.

Tier drives three things:

**(a) Model selection** — `effective_model` (`:430-452`). Precedence:
`model_hint` > config `tier_models[tier]` > built-in default > fallback.
Built-in defaults:

| Tier | Built-in model | Complexity band (`:2986`) | External-context enrichment (`:408`) |
|---|---|---|---|
| `mechanical` | `claude-haiku-4-5` | `Fast` | no |
| `focused` | `claude-sonnet-4-6` | `Standard` | no |
| `integrative` | `claude-sonnet-4-6` | `Standard` | **yes** (Perplexity search) |
| `architectural` | `claude-opus-4-6` | `Complex` | **yes** |

**(b) Complexity band** → feeds the CascadeRouter routing context
(`orchestrate.rs:2986-2990`): `mechanical→Fast`, `architectural→Complex`, else
`Standard`.

**(c) Enrichment / gate strictness** — `needs_external_context()` returns true
for `integrative`+`architectural`, so those tiers get pre-dispatch search
context. Gate strictness itself is governed by the `gate_rung` / domain / the
adaptive threshold store rather than tier directly, but higher tiers are
conventionally paired with higher rungs and an `acceptance_contract`.

**Meaning of each tier (authoring guidance):**
- **mechanical** — one obvious edit, no design decisions (rename, flag flip,
  string change). ≤~15 LOC, haiku, structural verify usually suffices.
- **focused** — a single well-scoped change in one area, some judgment.
  ≤~60 LOC, sonnet, compile+test verify.
- **integrative** — touches multiple files/crates that must agree; wiring an
  existing capability into a call site. ≤~200 LOC, sonnet + external context.
- **architectural** — new subsystem, new abstraction, or cross-cutting
  contract. opus, external context, acceptance_contract expected.

---

## 5. Roles, domains, gates

### 5.1 Roles
Valid roles (`plan_validate.rs` VALID_ROLES / `parse_task_role` `:912-953`):
`implementer` (default), `researcher`, `strategist`, `architect`,
`reviewer`/`auditor`, `quick-reviewer`, `scribe`, plus many specialized
validator roles. `implementer` requires both `verify` and `files`.

### 5.2 `TaskDomain` (`roko-core/src/task.rs:247-303`)
Controls gate selection and git policy. Serialized as a lowercase label.

| Variant | Label(s) | Default gates (`default_gates` `:294`) |
|---|---|---|
| `Code` | `code`, `coding` | `compile`, `test`, `clippy`, `diff` |
| `Chain` | `chain` | `compile`, `test`, `clippy`, `diff`, `invariant-check` |
| `Research` | `research` | `citation-check`, `quality` |
| `Docs` | `docs`, `documentation` | `lint`, `spell`, `link-check` |
| `Custom(String)` | any other non-empty label | `compile`, `test` |

`from_label` (`:277`) maps unknown non-empty labels to `Custom`, so teams can
attach their own gate profiles without touching the enum. Empty label → parse
error. Effective domain: explicit task `domain` > config default > `None`
(`effective_domain`, `task_parser.rs:110`).

---

## 6. `[task.context]` — `TaskContext` (`task_parser.rs:643-671`)

Surgical context inlined into the agent prompt before it makes changes.

| Field | Type | Default | Meaning |
|---|---|---|---|
| `read_files` | Vec\<ReadFile\> | `[]` | Files to read first. Content is inlined into the prompt (`build_prompt` `:519-540`). |
| `symbols` | Vec\<String\> | `[]` | Key types/functions the agent should know (rendered as "Key symbols"). |
| `anti_patterns` | Vec\<String\> | `[]` | "Do NOT" list, rendered with a ⛔ header. |
| `prior_failures` | Vec\<String\> | `[]` | Context injected from previous failed attempts. |

**`ReadFile`** (`:660-671`): `path` (String, required); `lines`
(Option\<String\>, e.g. `"40-80"` or `"10-"`, parsed by `extract_line_range`
`:1101`); `why` (String, default `"context"` — a one-line justification shown
to the agent).

---

## 7. `[[task.verify]]` — `VerifyStep` (`task_parser.rs:674-694`)

The machine-checkable pipeline. Each step is a shell command; **exit code 0 =
pass**, non-zero = fail.

| Field | Type | Default | Meaning |
|---|---|---|---|
| `phase` | String | `""` | Category label: `structural` / `compile` / `test` / `integration`. Free-form but conventionally one of these. Used to match retry hints back to the failing step (`:11022`). |
| `command` | String | **required** | Shell command run via `sh -c` in the plan exec dir. |
| `fail_msg` | Option\<String\> | none | Message shown on failure (defaults to "verification failed"). |
| `timeout_ms` | u64 | `gate_test()*1000` (`default_verify_timeout` `:689`) | Per-step timeout in ms. |

**How the runner executes verify** (`run_verify_steps`,
`orchestrate.rs:17544-17616`):
1. Steps run **sequentially in author order**; the first failure short-circuits
   and returns `(task_id, phase, command, stderr)`.
2. Each command is run as `sh -c "<command>"` with `current_dir(exec_dir)`.
3. Before spawning, the command passes through the safety layer
   (`check_exec_command`, `:17562`); a blocked command fails the step.
4. `o.status.success()` (exit 0) = pass; otherwise stderr is secret-scrubbed
   (`scrub_text`) and returned as the failure reason.
5. Failures feed the retry / auto-fix loop (`build_fix_prompt`, `:582`) and
   gate-failure replan.

The four phases map onto the runner's progression: `structural` (grep/existence
checks — did the change land?), `compile` (`cargo check`), `test`
(`cargo test`), `integration` (cross-crate / end-to-end). Verify steps are
distinct from — and run before — the rung-based gate pipeline
(`run_gate_pipeline`, `:17620`), which additionally applies compile/test/clippy/
diff gates and adaptive thresholds.

**Constraint for the backlog:** because verify is executed literally as shell
and judged only by exit code, every backlog task's verify commands MUST be
real, runnable shell that exits 0 exactly when the work is done. No prose, no
placeholders. Prefer `grep -q`, `test -f`, `cargo check -p <crate>`,
`cargo test -p <crate> <filter>`. Negative checks use `! grep -q ...`.

---

## 8. `[task.acceptance_contract]` — typed done-gate (`roko-gate/src/acceptance_contract.rs`)

An `AcceptanceContract` (`:36-57`) is a stronger, fail-closed done-gate used for
self-hosting / architecture-queue tasks. Where `verify` answers "do these
commands pass?", the contract answers "is there *structured evidence* the task
is genuinely done?" — and missing/malformed evidence is a blocking failure, not
a pass.

Top-level fields:

| Field | Type | Meaning |
|---|---|---|
| `version` | u32 | Must be `1` (else `ACCEPT_001`). |
| `gates` | Vec\<GateRequirement\> | Required commands producing gate evidence. Each: `id`, `kind` (`compile`/`test`/`lint`/`review`/`custom`), `command` (required if the gate is), `required` (default true). |
| `no_stub` | Option | Assert named `production_paths` are not satisfied by stubs/noops. |
| `agent_output` | Option | Require structured agent output validated against a named `schema`. |
| `review_verdict` | Option | Require a structured reviewer verdict: `reviewer_role_id`, `min_confidence ∈ [0,1]`. |
| `recovery` | Option | Require retry/reflection/replan signals after failures. |
| `parity_ledger` | Option | Require doc-parity rows: each row has `requirement_id`, `source_ref`, and implementation evidence (`evidence_ref` or `implementation_refs`) + `test_evidence_refs`. |

Validation happens in two stages: `validate_contract` (shape, at `plan
validate` time, `plan_validate.rs:410-443`) emitting `ACCEPT_001..012`, and
`validate_evidence` (runtime, `ACCEPT_020..032`). Notably, a passing
`review_verdict` must have `required_next_action = None`, and parity rows must
be `status = Verified` with both implementation and test evidence, else the
outcome degrades to `NeedsWork`. Use an `acceptance_contract` for
**architectural / integrative** tasks that produce a durable contract or must
prove non-stub implementation + doc parity; plain `verify` is enough for
mechanical/focused tasks.

---

## 9. Meta status lifecycle

`meta.status` (and per-task `status`) drive readiness:

- **`ready`** — plan is active; its `ready` tasks with satisfied deps are
  dispatchable. This is the default (`default_status`).
- **`superseded`** — replaced by another plan; set `superseded_by = "<plan>"`.
  Should not be executed; kept for provenance.
- **`archived`** — retired / out of scope; retained but not run.
- **`done`** — all tasks complete.

Per-task statuses (`VALID_STATUSES`, `plan_validate.rs:867`):
`pending`/`ready`/`active`/`done`/`blocked`/`skipped`. Only `ready` +
deps-complete is dispatched (`is_ready` / `is_ready_with_plan_deps`,
`:455-470`).

---

## 10. What `roko plan validate` checks (`plan_validate.rs`)

`plan validate <dir>` walks every `tasks.toml` and emits `Diagnostic`s
(Error/Warning). Exit code is 1 if any errors (or, with `--strict`, any
warnings) — `ValidationReport::exit_code` (`:65`). A plan is **"ready"** when it
validates with **zero errors**. Key rules:

| Rule | Severity | Meaning |
|---|---|---|
| `PLAN_001` | error | TOML failed to parse. |
| `PLAN_002` | error | `task` array missing/empty. |
| `PLAN_003` | error | Task missing required `id` / `title` / `role`. |
| `PLAN_004` | error | Duplicate task id within the plan. |
| `PLAN_005` | error | `depends_on` points at unknown task. |
| `PLAN_006` | error | Dependency cycle (`detect_cycle_nodes`). |
| `PLAN_007` | error | `gate_rung` outside `0..=6`. |
| `PLAN_008` | warning | Role has no compose template. |
| `PLAN_009` | warning | Model not configured in `roko.toml`. |
| `PLAN_010` | warning | Task unreachable from any root. |
| `PLAN_011` | warning | `gate_rung = 0` but no verify steps. |
| `PLAN_012` | error/warn | Malformed `acceptance_contract` / model alias. |
| `PLAN_030/031` | warning | Declared `files` reference missing crate/package/path (needs `--workdir`). |
| `PLAN_032/033` | error | Greenfield claim / "create crate X" where X already exists. |
| `PLAN_034` | error | Runtime parser (`TasksFile::parse_str`) would fail — `plan run` and `plan validate` agree. |
| `PLAN_035` | error | `validate_against_schema` issue (bad role/tier/status, missing implementer `verify`/`files`, `timeout_secs == 0`). |
| `PLAN_020..026` | error | Architecture-queue task missing `depends_on`/`read_files`/`files`/`verify`/`acceptance_contract`/parity rows / deferral metadata. |

The runtime `TasksFile::validate` (`task_parser.rs:784`) additionally requires,
per task: a known tier, at least one `verify` step, and at least one
`context.read_files` entry — the practical bar for a "complete" task.

---

## 11. How to author a task

### 11.1 Fully-annotated template

```toml
[meta]
plan = "my-plan-id"        # REQUIRED. Matches plans/<my-plan-id>/ and the PRD slug.
status = "ready"           # ready | superseded | archived | done
max_parallel = 1           # bump only if tasks are truly independent
# superseded_by = "..."    # set when status = "superseded"
# skip_enrichment = true   # set for complete, hand-authored plans

[[task]]
id = "T1"                            # REQUIRED, unique in plan, referenced by depends_on
title = "One-line imperative summary"# REQUIRED
description = "What & why in prose."  # keep < 500 words; empty = validation error
role = "implementer"                 # implementer requires files + verify
status = "ready"                     # only "ready" (+ deps done) dispatches
tier = "focused"                     # mechanical | focused | integrative | architectural
model_hint = "claude-sonnet-4-6"     # optional; omit to use tier default
max_loc = 60                         # soft budget; 0 => unlimited
files = ["crates/roko-x/src/y.rs"]   # files this task edits (alias: write_files)
depends_on = []                      # task ids in THIS plan
# depends_on_plan = ["other-plan"]   # cross-plan gating
# domain = "code"                    # code|chain|research|docs|<custom> => gate set
# timeout_secs = 600  max_retries = 3

[task.context]                       # STRONGLY RECOMMENDED (empty => quality warning)
read_files = [
  { path = "crates/roko-x/src/y.rs", lines = "40-80", why = "the fn to change" },
]
symbols = ["Foo::bar — fn bar(&self) -> Result<()>"]
anti_patterns = ["Do NOT add new deps", "Do NOT change the signature"]

[[task.verify]]                      # machine-checkable; exit 0 = pass
phase = "structural"
command = "grep -q 'fn bar' crates/roko-x/src/y.rs"
fail_msg = "bar must exist"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-x 2>&1"
fail_msg = "must compile"

# Optional: free-form human criteria (shown only if no verify steps)
# acceptance = ["bar returns Ok for the happy path"]

# Optional: typed done-gate for integrative/architectural work
# [task.acceptance_contract]
# version = 1
# [[task.acceptance_contract.gates]]
# id = "compile"  kind = "compile"  command = "cargo check -p roko-x"  required = true
```

### 11.2 Acceptance criteria vs verify commands

- **`acceptance`** = human-readable done conditions ("the status output shows
  linked/unlinked plan counts"). They document intent and appear in the prompt
  *only when there are no verify steps*. They are never executed.
- **`verify`** = machine-checkable shell. **Exit 0 = pass.** These are the real
  gate. Write one structural check per observable effect of the change, then a
  `compile` check, then `test` if behavior changed. Negative assertions use
  `! grep -q ...`. Every backlog task MUST carry verify steps — a task whose
  "doneness" cannot be expressed as `exit 0` is not yet ready to author; split
  or sharpen it until it can.
- **`acceptance_contract`** = use when a plain exit code is insufficient: you
  need proof of non-stub implementation (`no_stub`), a reviewer verdict, doc
  parity rows, or recovery evidence. Reserve for integrative/architectural
  tasks; it is fail-closed and validated at `plan validate` time.

### 11.3 tier → model_hint → max_loc heuristics

| Tier | model_hint | max_loc | Typical verify |
|---|---|---|---|
| mechanical | `claude-haiku-4-5` | ≤ 15 | structural + compile |
| focused | `claude-sonnet-4-6` | ≤ 60 | structural + compile + test |
| integrative | `claude-sonnet-4-6` | ≤ 200 | compile + test (+ integration); consider acceptance_contract |
| architectural | `claude-opus-4-6` | ≥ 200 (or unset) | acceptance_contract with gates + parity_ledger |

Omit `model_hint` to inherit the tier default; only set it to deviate.

### 11.4 Splitting a large finding into tasks

A finding rarely maps to one task. Decompose by tier and dependency:

1. **mechanical** leaves first — the trivial edits (flag flips, renames,
   deleting dead branches) that unblock everything else. No deps.
2. **focused** — each self-contained behavior change in one area. Depends on the
   mechanical prereqs.
3. **integrative** — the wiring step that makes the focused pieces agree across
   files/crates. `depends_on` the focused tasks.
4. **architectural** — only if the finding needs a new contract/abstraction;
   give it an `acceptance_contract` and make downstream tasks `depends_on` it.

Keep each task's verify independently checkable, keep `max_loc` within the tier
band, and wire `depends_on` so the DAG expresses the real ordering (author order
breaks ties). If a task exceeds its band or its verify can't be a single exit
code, split it again.

---

## 12. Annotated real examples

### 12.1 A mechanical task — `plans/P23-prd-pipeline-fix/tasks.toml`, T1

```toml
[[task]]
id = "T1"
title = "Give prd draft-new agent read-only codebase tools"
status = "ready"
tier = "mechanical"              # one obvious edit → haiku, tiny budget
model_hint = "claude-haiku-4-5"
max_loc = 5                      # a 5-line change
files = ["crates/roko-cli/src/commands/prd.rs"]
role = "implementer"
depends_on = []                  # a root task

[task.context]
read_files = [
  { path = "crates/roko-cli/src/commands/prd.rs", lines = "459-470",
    why = "The AgentExecOpts block with allowed_tools: Some(\"none\") that must change" },
  { path = "crates/roko-cli/src/prd.rs", lines = "1194-1204",
    why = "Reference: plan-gen agent already uses Some(\"Read,Grep,Glob\")" },
]
symbols = ["AgentExecOpts.allowed_tools — pub allowed_tools: Option<&'a str>"]
anti_patterns = [
  "Do NOT set allowed_tools to None — use Some(\"Read,Grep,Glob\") for read-only.",
]

[[task.verify]]                  # positive structural check: the new value landed
phase = "structural"
command = "grep -q 'allowed_tools: Some(\"Read,Grep,Glob\")' crates/roko-cli/src/commands/prd.rs"
fail_msg = "allowed_tools must be changed to Some(\"Read,Grep,Glob\")"

[[task.verify]]                  # negative check: the old value is gone (! grep -q)
phase = "structural"
command = "! grep -q 'allowed_tools: Some(\"none\")' crates/roko-cli/src/commands/prd.rs"

[[task.verify]]                  # compile gate always closes the set
phase = "compile"
command = "cargo check -p roko-cli 2>&1"
```

Why it's exemplary: tiny surgical scope, exact line ranges + a reference
implementation in `read_files`, an anti-pattern that pre-empts the obvious
wrong fix, and three verify steps that pin down "done" as pure exit codes
(landed value + removed old value + still compiles).

### 12.2 An integrative task with a typed contract — `plans/architecture-defi-critical-path/tasks.toml`, D01

```toml
[meta]
plan = "architecture-defi-critical-path"
queue_kind = "architecture_implementation"   # triggers PLAN_020..026 strict checks

[[task]]
id = "D01-chain-registry-indexer-foundation"
title = "Implement chain registry client and event indexer foundation"
description = "Create the chain-side primitives ... and compile-time wiring."
role = "implementer"
tier = "integrative"             # multiple files must agree → sonnet, larger budget
model_hint = "claude-sonnet-4-6"
max_loc = 500
files = [
  "crates/roko-chain/src/knowledge_registry.rs",
  "crates/roko-chain/src/indexer.rs",
  "crates/roko-chain/src/lib.rs",
]
depends_on = []

[task.context]
read_files = [
  { path = "crates/roko-chain/src/agent_registry.rs", why = "existing registry conventions" },
  # ... two more sibling registries for pattern-matching
]
symbols = ["AgentRegistry", "ReputationRegistry", "ValidationRegistry"]
anti_patterns = ["Do not require chain config for local-only runs."]

[[task.verify]]                  # verify = fast shell gate
phase = "compile"
command = "cargo check -p roko-chain 2>&1 | tail -10"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-chain --lib --no-run 2>&1 | tail -10"

[task.acceptance_contract]       # typed done-gate = durable, fail-closed evidence
version = 1

[[task.acceptance_contract.gates]]
id = "compile"  kind = "compile"  command = "cargo check -p roko-chain"  required = true

[[task.acceptance_contract.gates]]
id = "test-build"  kind = "test"  command = "cargo test -p roko-chain --lib --no-run"  required = true

[task.acceptance_contract.parity_ledger]
required = true

[[task.acceptance_contract.parity_ledger.rows]]
requirement_id = "DEFI.D01.chain-registry-indexer"
source_ref = "plans/architecture-core-queue/tasks.toml#Q14-chain-registries-defi-foundation"
evidence_ref = "crates/roko-chain/src/knowledge_registry.rs"
```

Why it's exemplary: an integrative task that both (a) has quick `verify` shell
gates for the retry loop and (b) an `acceptance_contract` that ties the work to
a source requirement (`parity_ledger` row) with implementation evidence — the
pattern every architecture-queue task must follow.

---

## 13. Constraints this schema imposes on the backlog

1. **Verify must be shell-checkable.** Every task's `verify.command` is run as
   `sh -c` and judged solely by exit 0. Backlog tasks must express "done" as
   runnable commands, not prose.
2. **A task needs id + title + role, a tier, ≥1 verify step, and
   `context.read_files`** to be considered complete by the runtime validator.
   `implementer` additionally requires `files`.
3. **Tier bounds scope.** Pick the tier that matches the change size; it fixes
   the default model and the expected `max_loc` band. Oversized tasks must be
   split.
4. **DAG must be acyclic and rooted.** At least one task with empty
   `depends_on`; all `depends_on` ids must exist; no cycles (`PLAN_005/006`,
   NoStartNode).
5. **No greenfield fictions.** Don't propose creating crates that exist or
   describe an existing workspace as empty (`PLAN_032/033`).
6. **Architecture-queue tasks** (`meta.queue_kind =
   "architecture_implementation"`) must additionally carry `acceptance_contract`
   + parity rows + deferral metadata.
7. **A plan is "ready" only at zero validation errors** (`plan validate`,
   exit 0). Author to that bar.

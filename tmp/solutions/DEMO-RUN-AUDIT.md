# Demo Run Audit: PRD/Plan Generation Failure

Date: 2026-04-28

Scope:

- User-provided demo workspace: `/tmp/roko-demo-1777396797076`
- Additional recent demo runs found under `/private/tmp`:
  - `/private/tmp/roko-demo-1777296817271`
  - `/private/tmp/roko-demo-1777314002260`
  - `/private/tmp/roko-demo-1777396797076`
- Relevant source paths:
  - `crates/roko-cli/src/commands/prd.rs`
  - `crates/roko-cli/src/prd.rs`
  - `crates/roko-cli/src/prd_prompt.rs`
  - `crates/roko-cli/src/plan_generate.rs`
  - `crates/roko-cli/src/agent_exec.rs`
  - `crates/roko-agent/src/claude_cli_agent.rs`
  - `crates/roko-learn/src/runtime_feedback.rs`

## Executive Summary

The demo failure is real and systemic. It is not just noisy logs.

Across three recent demo runs, Roko generated PRDs and plans for
`system-prompt-wiring` that repeatedly described a greenfield workspace and proposed new crates
such as `roko-prompt`, `roko-orchestrate`, and sometimes `roko-config`.

There is an important nuance: the demo workspace itself was effectively a temp workspace with
`.roko` artifacts, not the Roko source tree. That means the immediate failure is a context-root
contract failure. If the request was intended to plan a Roko-internal feature, Roko ran against the
wrong repository context. If the request was intended as a blank demo project, Roko should have
made that explicit and avoided producing a confident Roko-internal architecture plan.

Either way, the product behavior is wrong: Roko accepted a detailed implementation plan without
proving that it was grounded in the intended codebase.

The root problem is that PRD generation and plan generation are not grounded strongly enough in
the actual repository before artifacts are accepted. The agent is asked to "search the codebase",
but Roko does not require or validate that the produced artifact cites existing files, avoids
duplicate crates, or maps requirements onto current architecture.

The logging issue is a second, compounding problem: Roko marks these episodes successful and writes
learning/cost/efficiency records, but token/cost/tool telemetry is mostly zero or missing. The
result is a UI full of process noise and learning records that look confident while carrying very
little useful evidence.

## What The Latest Demo Produced

Latest run inspected: `/private/tmp/roko-demo-1777396797076`.

The workspace contained Roko metadata and generated artifacts, but no normal Rust workspace source
tree. The important product issue is that this absence was not surfaced as a blocker or ambiguity.
The run proceeded and recorded a successful plan.

Generated artifacts:

- `.roko/prd/drafts/system-prompt-wiring.md`: 30,471 bytes
- `.roko/plans/system-prompt-wiring/plan.md`: 3,478 bytes
- `.roko/plans/system-prompt-wiring/tasks.toml`: 34,061 bytes

The plan says:

- crates: `roko-prompt`, `roko-orchestrate`
- "greenfield implementation"
- "no Rust crates or source files exist yet"
- create stub `Grimoire`, `Daimon`, `StrategyVector`, `PlanTask`, and `TaskResult`

That directly conflicts with the actual Roko source tree if this task was meant to modify Roko,
where prompt assembly and agent/runtime pieces already exist in these areas:

- `crates/roko-compose/src/system_prompt_builder.rs`
- `crates/roko-compose/src/prompt_assembly_service.rs`
- `crates/roko-agent/src/claude_cli_agent.rs`
- `crates/roko-agent/src/model_call_service.rs`
- `crates/roko-runtime`
- `crates/roko-cli/src/runner`
- `crates/roko-cli/src/orchestrate.rs` as a legacy/donor path

## Repeated Run Pattern

This happened across all three recent runs:

| Run | Draft bytes | Plan bytes | Tasks bytes | Bad plan signal |
| --- | ---: | ---: | ---: | --- |
| `1777296817271` | 29,318 | 4,016 | 33,751 | creates `roko-config`, `roko-prompt`, `roko-orchestrate`; says greenfield |
| `1777314002260` | 38,718 | 3,033 | 34,025 | creates `roko-config`, `roko-prompt`, `roko-orchestrate` |
| `1777396797076` | 30,471 | 3,478 | 34,061 | creates `roko-prompt`, `roko-orchestrate`; says greenfield |

Episode telemetry also repeats the same pattern:

| Run | Episode | Duration | Prompt chars | Output chars | Tokens/cost |
| --- | --- | ---: | ---: | ---: | --- |
| `1777296817271` | draft | 229.998s | 365 | 160,827 | all zero |
| `1777296817271` | plan | 322.207s | 29,848 | 239,019 | all zero |
| `1777314002260` | draft | 390.790s | 365 | 438,311 | all zero |
| `1777314002260` | plan | 309.241s | 39,157 | 237,608 | all zero |
| `1777396797076` | draft | 247.697s | 365 | 293,317 | all zero |
| `1777396797076` | plan | 273.476s | 31,014 | 237,352 | all zero |

The first PRD draft prompt is only about 365 characters. The system prompt is large, but the
actual user task gives almost no concrete repository context. That makes it easy for the model to
write a plausible architecture essay instead of a repo-grounded product document.

## Artifact Findings

### F0: The workflow lacks a context-root contract

The demo run appears to have been executed from a temp workspace that did not contain the Roko
source tree. For a generic blank project this may be valid; for a Roko-internal feature it is not.

Roko should detect this before generation:

- What repository is this PRD/plan intended to modify?
- Is the current working directory that repository?
- If the feature mentions Roko internals but the repo has no Roko crates, should generation stop?
- If the repo is intentionally blank, should the plan be labeled "new project" and avoid claiming
  knowledge of existing Roko internals?

The current flow allows a context mismatch to become a successful plan.

### F1: The PRD prompt rewards impressive documents over correct documents

`crates/roko-cli/src/prd_prompt.rs` asks for:

- first-reader orientation
- 10-30 academic citations
- 2-5 styled Mermaid diagrams
- dense architecture prose
- Rust interface sketches

Those standards can produce polished documents, but they are badly weighted for a repo-local
implementation planning workflow. They reward breadth, citations, and invented architecture more
than "what exists in this repo and what is the shortest valid patch path."

The prompt says to study existing PRDs, but it does not require a repository inventory, exact file
citations, or a "do not invent crates" proof section.

### F2: Draft generation is not forced to inspect the repo

`crates/roko-cli/src/commands/prd.rs` builds a draft task prompt that says "search the codebase",
but it accepts either:

- the agent directly modifying the file, or
- the agent returning a full markdown document

The acceptance condition is only that markdown content exists. There is no validation that the PRD:

- names actual existing crates
- references existing files
- avoids greenfield crate scaffolding when a workspace already exists
- identifies prior implementations
- includes a shortest-path implementation map

This is why the generated PRD can be very large and still wrong.

### F3: Plan generation asks for code search but does not verify code search happened

`crates/roko-cli/src/plan_generate.rs` says the plan generator must search and read files before
generating tasks. But this is only a prompt instruction. There is no hard gate that checks the
generated `tasks.toml` against the file system.

The latest run created tasks for non-existent target crates. A validator should have rejected that
before the plan was written as a successful artifact.

### F4: The plan path lacks artifact-level rejection rules

`crates/roko-cli/src/prd.rs` validates whether `tasks.toml` has modern fields, then emits a
`prd:plan:generated` signal. It does not validate whether the plan is grounded.

Minimum missing checks:

- reject new crate creation unless explicitly allowed
- reject "greenfield" and "no source files exist" in an existing workspace
- reject task files that do not exist unless the task is explicitly a new-file task
- reject creation of crates that are not in the root `Cargo.toml` workspace unless allowed
- reject plans that do not mention any existing source file
- reject plans where `plan.md` and `tasks.toml` disagree on total task count

The latest `engrams.jsonl` recorded `task_count = 21`, while the generated metadata reported 20
tasks. That mismatch is another sign the artifact path needs a consistency gate.

### F5: Tool logs are too shallow to be useful

`crates/roko-agent/src/claude_cli_agent.rs` parses Claude stream JSON enough to print progress:

- `tool: <name>`
- `generating text...`
- `result received (<bytes> bytes text, <N> tool calls)`

It does not preserve the structured stdout stream as an artifact, and it does not turn tool calls
into durable per-tool telemetry. The UI can therefore show "tool" events without useful inputs,
outputs, duration, or whether the tool advanced the task.

The latest demo's persisted efficiency summaries say:

- `tools_available = 0`
- `tools_used = 0`
- `tool_calls = 0`
- `prompt_section_count = 0`
- `total_prompt_tokens = 0`
- `time_to_first_token_ms = 0`

So the system is both printing tool-ish process noise and persisting "no tools" in learning data.

### F6: Token and cost telemetry is wrong, not unknown

The demo episodes record:

- input tokens: 0
- output tokens: 0
- cache tokens: 0
- cost: 0

This is misleading. The run clearly used Claude for several minutes and produced hundreds of
thousands of output characters. If usage cannot be parsed from Claude CLI stream JSON, Roko should
record `unknown`, not numeric zero. Zero means "free and tokenless", which pollutes cost routing,
efficiency summaries, dashboards, and future model selection.

### F7: Success means process success, not artifact success

The PRD/plan episodes are marked success because the subprocess exited successfully and wrote
files. But the artifacts are semantically bad.

For PRD and plan workflows, success must mean:

- process succeeded
- artifact exists
- artifact passes schema validation
- artifact passes grounding validation
- artifact passes consistency validation
- artifact is small enough and specific enough to execute

Without those checks, the learning system records bad plans as positive examples.

### F8: Knowledge seeds are low-value and potentially harmful

The latest `knowledge-seeds.jsonl` emits generic insights like successful task X used role Y,
provider Z, model M, with no file scope and no gate evidence. For this workflow, that is not useful
knowledge. It can reinforce the wrong behavior because the run was marked successful even though
the artifact quality was poor.

Knowledge seeds should be withheld for artifact-generation episodes unless the artifact quality
gate passes.

### F9: Raw transcripts are missing

The `.roko/memory/episodes.jsonl` entries include fingerprints and metadata, but not enough raw
model transcript or structured event data to debug what happened later. The final PRD/plan files
exist, but the path from prompt to artifact is mostly lost.

For this class of bug, the durable audit record should include:

- the exact system prompt hash and a bounded prompt excerpt
- the exact user prompt
- raw Claude stream JSON, either compressed or sidecar-filed
- extracted text output
- file diff summary
- tool call summary with arguments redacted as needed
- artifact validation report

Fingerprints are useful for retrieval, but they are not a substitute for an audit trail.

## Source-Level Root Cause Chain

1. The command runs in a temp workspace with little or no real source tree.
2. `prd draft new` creates a scaffold and asks the agent to fill it.
3. The task prompt is short and broad.
4. The PRD system prompt strongly rewards complete, academic, architecture-heavy documents.
5. The output is accepted if it is substantive markdown.
6. `prd plan` feeds the huge ungrounded PRD into the plan generator.
7. The plan generator prompt asks for repo search, but Roko does not enforce search evidence or
   file existence.
8. The generated plan is accepted if `tasks.toml` parses and modern fields exist.
9. Learning runtime marks the episode successful and derives cost/efficiency/knowledge records
   from incomplete metadata.
10. The UI shows process logs but lacks meaningful artifact-quality and tool-use detail.

## Why This Matters For Mori Parity

The earlier shortest-path recommendation focused on no-args interactive chat. That remains the
right product-first priority. But the demo shows a second first-mile failure: Roko's PRD/plan UI
can confidently produce wrong implementation work.

If the goal is "works as good as Mori", then the first milestone should include both:

- interactive session parity: prompt, tools, MCP, resume, streaming
- artifact generation parity: grounded PRDs/plans with validators that reject hallucinated work

Otherwise Roko may get a better chat loop while still generating bad plans that lead agents into
the wrong files.

## Recommended Fix Direction

Do not solve this by adding a huge new architecture.

Do solve it by adding a small "ground then generate, validate before learning" layer around the
existing PRD/plan paths.

### 1. Build a repository context pack before PRD and plan generation

Create a bounded context pack that includes:

- the intended repository root and whether it matches the current working directory
- root `Cargo.toml` workspace members
- crate list with one-line purpose when available
- relevant file candidates from `rg`
- existing symbols matching the feature keywords
- existing PRDs/plans touching the same area
- explicit "do not create these duplicate crates" warnings when equivalent crates exist

For `system-prompt-wiring`, the pack should have pointed to:

- `roko-compose` for prompt assembly
- `roko-agent` for Claude/provider adapters
- `roko-runtime` and `roko-cli/src/runner` for active workflow execution
- `orchestrate.rs` as legacy/donor, not the preferred place for new runtime behavior

If those files are not present because the command is running in a temp workspace, Roko should say
so and either ask for the target repo/context or generate only a clearly labeled blank-project
plan.

### 2. Require a repo-grounding section in PRDs

Every generated PRD should include a short, machine-checkable section:

```markdown
## Repository Grounding

- Existing crates: ...
- Existing source files to modify: ...
- Existing code that already solves part of this: ...
- New crates needed: none
- Explicit non-goals: do not create ...
```

If this section is missing or cites no existing source files in a non-empty repo, reject the PRD
as a draft that needs regeneration.

### 3. Add plan artifact validators

Before accepting `plan.md` and `tasks.toml`, validate:

- every referenced existing file path exists
- every new file path has a parent directory that exists or is explicitly created by an earlier
  task
- workspace crate names match `Cargo.toml`
- no task creates a new crate unless PRD frontmatter has something like
  `allow_new_crates: true`
- no banned phrases appear in normal repo mode:
  - "greenfield"
  - "no Rust crates or source files exist yet"
  - "stub Grimoire"
  - "stub Daimon"
- all verification commands are executable and relevant
- `meta.total` equals the task count
- generated engram task count equals parsed task count

### 4. Change success and learning semantics

For artifact generation, persist two outcomes:

- process outcome: subprocess exited 0 or not
- artifact outcome: validation passed or failed

Only feed positive learning, knowledge seeds, cascade-router rewards, and "successful strategy"
signals from artifact episodes when the artifact outcome passes.

### 5. Persist usable run transcripts

Add a sidecar layout under each run or artifact:

```text
.roko/runs/<run-id>/
  prompt.md
  system-prompt.md
  stream.jsonl
  output.md
  tool-calls.jsonl
  artifact-validation.json
  files-changed.json
```

The UI can then show useful details instead of generic tool lines.

### 6. Fix usage extraction or mark it unknown

Parse token/cost/session metadata from Claude stream JSON result events when present. If a provider
does not expose usage, store an explicit unknown/null representation. Do not write zero values as
if the run cost nothing.

## Concrete Shortest Path

Add one new first-mile batch before broad runtime convergence:

1. Repository context pack for PRD/plan generation.
2. Grounding section requirement in PRD output.
3. Plan validation gate for file/crate/greenfield errors.
4. Artifact outcome status separate from process outcome.
5. Transcript sidecars and useful UI detail.
6. Token/cost unknown-vs-zero fix.

This is smaller than a new gateway and more urgent than full `orchestrate.rs` retirement because
it prevents Roko from generating bad work instructions in the default planning workflow.

## Clarifying Questions

1. Should the first "works like Mori" milestone include PRD/plan generation from the demo UI, or
   should it be scoped strictly to no-args interactive chat?
2. Should Roko ever create new crates from a PRD/plan automatically, or should that require an
   explicit `allow_new_crates` flag?
3. Should artifact-generation episodes be excluded from positive learning until validation passes?
4. Do you want raw Claude stream JSON stored by default for every PRD/plan run, or only behind a
   debug/audit setting?

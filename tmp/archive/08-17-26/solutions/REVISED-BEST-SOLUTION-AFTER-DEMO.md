# Revised Best Solution After Demo Audit

Date: 2026-04-28

This supersedes the earlier recommendation only in one way: the first milestone should not be
chat-only if the demo UI's PRD/plan flow is part of the product surface you want to trust.

Additional update after E2E dogfood: the first milestone now needs a small prerequisite rail,
M0-0 Execution Contract Repair. The clean Rust run showed that Roko cannot yet be trusted to
honor `--model`, accept only runnable generated plans, report real gates, record truthful costs,
or resume from the plan root it generates. See `E2E-ROKO-DOGFOOD-AUDIT.md`.

Additional update after the full demo-app workflow audit: the demo UI itself needs a truthfulness
rail. It currently mixes fallback/live data, uses stale CLI commands, runs benchmark mutations
against wrong routes, starts terminals in ambiguous workdirs, and can show connected/running states
after failures. See `DEMO-APP-WORKFLOW-AUDIT.md`.

The main architectural recommendation is unchanged:

- do not build a large new gateway first
- do not grow `dispatch_direct.rs` into another orchestrator
- do not create greenfield crates for functionality already present in Roko
- do use the existing agent/provider/prompt/runtime pieces and wire them into a coherent first
  mile

## Updated Verdict

The best possible shortest path is now a staged M0:

0. **M0-0: Execution Contract Repair**
   Make model/provider selection, generated plan validation, gate status, telemetry, and resume
   pathing truthful across the CLI.

1. **M0-A: Interactive Agent Session Parity**
   Make no-args `roko` behave like a real Mori/Claude-style local coding session: system prompt,
   tools, MCP, resume, streaming, cancellation, and sane server posture.

2. **M0-B: Grounded Artifact Generation**
   Make `prd draft new` and `prd plan` produce repo-grounded artifacts or fail loudly. The demo
   must not accept greenfield plans for an existing workspace and then record them as successful
   learning.

3. **M0-C: Truthful Demo Proof Surface**
   Make `demo/demo-app` prove real workflows instead of hiding broken live paths behind fallback
   data and terminal scraping.

These rails are connected. A good interactive agent loop without grounded plans still creates bad
work queues. A good plan generator without Mori-like session execution still cannot execute well.
And none of the demo proof is reliable if the CLI silently uses the wrong provider/model, accepts
invalid tasks, or the UI fabricates success after endpoint failures. For the shortest path to
"Roko works", M0-0 has to happen first, then the demo app needs to become a real proof harness.

## What The E2E Dogfood Changes

The fresh-repo test found failures that sit underneath both earlier rails:

- `prd plan` and `plan regenerate` ignored `--model claude-haiku-4-5` and used Opus.
- `roko run` ignored model flags and chose Anthropic API Sonnet, then failed because
  `ANTHROPIC_API_KEY` was absent.
- `config migrate` moved top-level `run` onto Claude CLI, but remained interactive, preserved
  `shell:true`, and did not prevent an implementer/autofix loop for a no-edit prompt.
- Positional prompts ignored `--model gpt-4o`, `--model glm-5-1`, and `--model llama32`,
  reporting GLM for all of them.
- `config models route <model>` selected Sonnet for every tested model.
- `prd plan` generated `tasks.toml` without required `role` fields and with unconfigured
  `sonnet`/`haiku` aliases.
- `plan regenerate` did not repair those invalid fields.
- The runner made useful code changes only after manual plan repair and state cleanup.
- Stub gates were recorded as passes.
- PRD/plan cost was recorded as zero; runner costs were double-counted.
- `roko resume` looked in `./plans` while PRD plans live in `.roko/plans`.

This makes execution truth the first product requirement.

## Why The Demo Changes The Priority

The latest demo generated a PRD and plan for `system-prompt-wiring` that proposed `roko-prompt`
and `roko-orchestrate` as new crates. Two nearby runs did the same, sometimes adding
`roko-config`.

The demo workspace looked like a temp workspace rather than the actual Roko source tree, so this is
also a context-root failure. If the requested feature was meant for Roko itself, Roko needed the
Roko repo as context. If the requested feature was meant for a blank demo project, the artifact
should have been labeled as a new-project plan instead of a confident Roko-internal design.

This matters because the real Roko repo already has:

- prompt assembly in `roko-compose`
- Claude/provider adapters in `roko-agent`
- active runner paths in `roko-cli/src/runner`
- runtime workflow pieces in `roko-runtime`
- legacy/donor orchestration in `roko-cli/src/orchestrate.rs`

This means implementation agents following the generated plan would spend time creating duplicate
architecture instead of wiring existing pieces. That directly fights the user's goal: shortest path
to full Mori functionality.

## The Refined First Milestone

### M0-A: Interactive Agent Session Parity

Keep the earlier plan:

- introduce a small session owner for interactive chat
- route Claude CLI through `ClaudeCliAgent` or the same command-builder behavior
- pass system prompt, tools, MCP config, model, effort, resume id, timeout, and safety settings
- stream Claude events to the inline UI
- make `/system`, `/model`, `/effort`, and `/reset` mutate actual dispatch state
- disable or secure background serve before calling M0 complete

This is still the shortest path to Mori-like feel.

### M0-B: Grounded PRD/Plan Artifacts

Add a narrow artifact quality layer around `prd draft new`, `prd draft edit`, `prd plan`, and
possibly `plan generate`.

Required behavior:

- generate a bounded repo context pack before the agent runs
- verify the intended repository root before the agent runs
- force PRDs to include a repository-grounding section
- force plans to cite existing files and crates
- validate generated plans against the file system and `Cargo.toml`
- reject greenfield duplicate-crate plans unless explicitly allowed
- record artifact validation status separately from subprocess exit status
- withhold positive learning and knowledge seeds when artifact validation fails
- persist a useful run transcript sidecar

This is not a broad runtime rewrite. It is a product-quality gate on the existing PRD/plan command
paths.

### M0-C: Truthful Demo App

Add this narrow UI/API rail before using the demo as proof:

- make fallback/mock mode explicit and remove fallback from mutations
- align bench routes and response shapes
- align share route names and returned URLs
- make unmatched `/api/*` return JSON errors, not SPA HTML
- make terminal/workflow workdirs explicit
- block scenario play until terminal handles are ready
- replace terminal scraping as product state with typed server events
- hide or label placeholder workflows until real backend paths exist
- add Playwright proof for every route and scenario

## What To Fix First In M0-B

### 1. Replace the PRD prompt's incentive structure

Current PRD prompt standards over-index on citations, diagrams, and architecture prose. Keep those
as optional quality polish, but make the highest-priority rules:

- identify existing implementation surfaces first
- prefer modifying existing crates
- do not invent crates without explicit permission
- produce machine-verifiable acceptance criteria tied to current commands
- document shortest path, not idealized architecture

### 2. Add a repository context pack

Before the model writes a PRD or plan, Roko should collect a small context pack:

- intended repository root and whether it contains source code
- workspace members from root `Cargo.toml`
- top-level crate names and descriptions
- matching symbols from `rg`
- related PRDs/plans
- existing prompt/runtime/provider files
- current execution-path notes, such as runner vs legacy `orchestrate.rs`

This should be included in the system or task prompt as non-optional context.

If the context pack cannot find the intended repo shape, generation should stop or explicitly enter
new-project mode. It should not silently turn a missing Roko repo into a greenfield implementation
plan for duplicate Roko concepts.

### 3. Add artifact validators

A generated plan should fail validation if it:

- says the workspace is greenfield when `Cargo.toml` exists
- claims repo-local certainty when the intended repository root is missing
- creates a crate not in the workspace without explicit permission
- references a path that does not exist without a task creating it first
- lacks any existing source-file references
- creates stubs for concepts already implemented elsewhere
- has mismatched task counts

This catches the observed demo failure directly.

### 4. Fix telemetry truthfulness

The demo logs recorded zero tokens, zero cost, zero prompt sections, and zero tools despite several
minutes of Claude execution and hundreds of thousands of output characters. That makes dashboards
and learning untrustworthy.

Minimum fix:

- parse Claude stream result metadata where available
- if usage is unavailable, store null/unknown instead of zero
- persist tool call counts and names from stream JSON
- record raw stream sidecars for audit
- make UI logs point to artifacts, transcript, validation report, and changed files

### 5. Gate learning on artifact quality

Successful subprocess exit is not enough for:

- knowledge seeds
- cascade-router positive rewards
- provider/model pass outcomes
- "successful strategy" summaries

For PRD/plan generation, positive learning should require artifact validation success.

## What Not To Do

- Do not implement the generated `roko-prompt` / `roko-orchestrate` plan.
- Do not make `orchestrate.rs` the target for new first-class behavior just because the generated
  PRD said so.
- Do not build `roko-gateway` or a cell graph before M0-A and M0-B.
- Do not treat all learning JSONL files as evidence of useful learning; the demo shows they can
  be false confidence.
- Do not keep accepting PRDs/plans merely because markdown/TOML parses.

## Recommended Batch Order

0. **Execution contract repair**
   One model/provider selection contract, generated plan validation before execution, truthful
   gates/telemetry, and consistent resume/state roots.

1. **Safety posture for no-args `roko`**
   Disable or secure background serve before interactive parity is called done.

2. **Chat session object**
   Add the narrow live session state and wire slash commands into it.

3. **Claude CLI session parity**
   Use existing `ClaudeCliAgent` behavior for prompt/tools/MCP/resume/timeout.

4. **PRD/plan context pack**
   Build and inject repo inventory before PRD/plan generation.

5. **PRD/plan validators**
   Reject the exact failure class seen in `/tmp/roko-demo-1777396797076`.

6. **Transcript and telemetry cleanup**
   Store stream sidecars, tool call details, and honest usage.

7. **Demo truth repair**
   Fix fallback/mutation behavior, bench/share API drift, terminal workdirs, scenario readiness,
   and `/api/*` error handling.

8. **Streaming UI**
   Feed Claude stream events into the inline/demo UI with useful text/tool/artifact progress.

9. **API provider parity**
   Route API providers through existing adapters or `ModelCallService` instead of hand-rolled
   request JSON.

10. **Product proof suite**
   Add E2E proof for chat and PRD/plan generation before widening runtime convergence.

## Definition Of "Works As Good As Mori"

For the first milestone, I would require these proof runs:

- no-args `roko` can answer repo questions using tools
- a follow-up turn uses prior session context
- `/system`, `/model`, `/effort`, and `/reset` visibly affect behavior
- a PRD generated for an existing repo names existing crates and files
- a plan generated from that PRD modifies existing files unless new files are explicitly justified
- a bad greenfield plan is rejected with a clear validation error
- the demo UI shows transcript/artifact/validation details rather than generic tool noise
- learning records do not claim success when artifact validation fails

That is the shortest credible path to a product that feels like Mori and does not lead itself into
wrong implementation work.

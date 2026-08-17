# Full Work Plan: Controlled Mori Parity Without New Fracture

Date: 2026-04-28

Purpose: turn the audit material in `tmp/solutions/` into a runner sequence that gets Roko to
core Mori parity while preventing agents from adding more duplicate paths, silent fallbacks, fake
success states, and half-wired abstractions.

This supersedes the earlier version of this file. The earlier plan was directionally useful, but
it was too optimistic in three ways:

1. It said "everything needed" while still leaving major Mori features outside the sequence.
2. It put some architectural work into batches that were too vague, especially dispatch, gates,
   telemetry, and model selection.
3. It did not give runner agents enough negative instructions, ownership boundaries, and
   acceptance contracts to stop them from creating more anti-patterns.

## Direct Answer

Does this plan get Roko to "work like Mori"?

Yes for core Mori parity, if all primary runners land and pass their proof harnesses. Core Mori
parity means:

- `roko` interactive mode is a real local agent session.
- The agent receives workspace context, system prompt, tool policy, MCP config, model/effort,
  and resume state.
- Long responses stream.
- Tool output is visible.
- Ctrl-C and timeouts clean up child processes.
- `roko "prompt"` and `roko run` use the same execution contracts instead of separate ad hoc
  dispatch paths.
- PRD and plan generation are grounded in the intended repository and invalid artifacts are
  rejected before execution or learning.
- Demo UI shows real pass/fail/skip states instead of hiding broken workflows.

Does this plan cover every issue mentioned across `tmp/solutions/`?

It accounts for every issue class in the solution docs, but not every item is implemented in the
first five product runners. Each item has one of four dispositions:

- Primary runner: must be fixed for core Mori parity.
- Policy runner: needs a product/security decision first.
- Deferred platform runner: real, but not needed for core Mori parity.
- Explicitly rejected: previous over-engineered solution direction that should not be built now.

Does this plan elegantly architect the work?

That is the goal of this rewrite. The key architectural rule is: add contracts around existing
working subsystems; do not grow old thin paths into new orchestrators. In practice:

- Do not grow `dispatch_direct.rs`.
- Do not hand-roll provider tool loops in CLI code.
- Do not add another model router.
- Do not add another prompt builder.
- Do not make demo terminal scraping the source of truth.
- Do not record unknown telemetry as zero.
- Do not accept generated artifacts just because files were written.

## Core Architecture Target

The final shape should have these owners:

| Concern | Owner | Forbidden duplicate |
|---|---|---|
| Effective model/provider selection | one `EffectiveModelSelection` module | path-specific fallback chains |
| Claude CLI execution | `ClaudeCliAgent` or shared command builder | raw `claude` subprocess code in chat/one-shot |
| API provider tool loops | existing provider adapters / ModelCallService | handwritten JSON loops in CLI dispatch |
| Interactive session state | `ChatAgentSession` | scattered fields in `chat_inline.rs` plus `dispatch_direct.rs` |
| Prompt assembly | existing compose/prompt services | new prompt builder for chat only |
| Tool policy | existing safety/tool contracts | per-command hardcoded tool strings |
| Gate execution | gate service plus typed gate config | string-only gates losing program/args |
| PRD/plan grounding | repo context pack and artifact validators | prompt-only "please inspect repo" instructions |
| Telemetry truth | normalized usage/attempt events | zero as "unknown" |
| Demo workflow truth | typed workflow/API events | terminal regex scraping as product state |

## Runner Encapsulation Rules

Every runner must include a `context/` directory with these files before implementation batches
start:

```text
tmp/runners/<runner-name>/
  README.md
  batches.toml
  context/
    ARCHITECTURE-CONTRACT.md
    ANTI-PATTERNS.md
    ACCEPTANCE.md
    FILE-OWNERSHIP.md
    ISSUE-MAP.md
```

Every batch prompt must include:

- the runner goal in one sentence;
- exact write scope;
- files it may read;
- files it must not edit;
- the architectural contract it must preserve;
- verification command;
- rollback risk;
- a "do not" list.

Every batch final response must report:

- files changed;
- whether it changed a public contract;
- verification results;
- any known remaining gap;
- any surprising source discovery.

No runner is allowed to merge without a final proof batch that runs the runner acceptance checks.

## Agent Containment Protocol

The biggest risk is not that a batch misses a small bug. The biggest risk is that parallel agents
make the system look better by adding more local fallbacks, more duplicate pathways, and more
terminal-shaped glue. Each runner must therefore be treated as a controlled change envelope.

### Runner Setup Checklist

Before any implementation batch starts, the runner owner must create these context files:

1. `ARCHITECTURE-CONTRACT.md`
   - Defines the single owner for each behavior changed by the runner.
   - Names the existing modules the runner must reuse.
   - Names the old paths that may be deprecated but not grown.
2. `ANTI-PATTERNS.md`
   - Contains the runner-specific forbidden list.
   - Contains examples from the current repo so agents recognize the pattern.
   - Contains review vetoes.
3. `ACCEPTANCE.md`
   - Lists proof commands.
   - Lists expected failure behavior, not only expected success behavior.
   - Lists required UI/API/CLI observable outputs.
4. `FILE-OWNERSHIP.md`
   - Maps each batch to exact write paths.
   - Marks shared files as serialized batches only.
   - Marks read-only reference files.
5. `ISSUE-MAP.md`
   - Maps every batch to issue ids from this work plan and source docs.
   - Marks "not in scope" issues explicitly.

If a batch needs to write outside its declared scope, it must stop and update `FILE-OWNERSHIP.md`
first. This is deliberate friction: surprising write expansion is how runners create fracture.

### Batch Size Rules

A batch is too broad if it does any of these:

- touches more than one architectural concern;
- changes a public contract and a UI consumer in the same patch;
- introduces a new type and wires every caller in the same patch without tests;
- changes provider/model selection in more than one command before the selector has unit tests;
- changes telemetry schema and dashboard rendering in the same patch;
- changes `orchestrate.rs` broadly while also changing behavior;
- fixes a demo symptom without proving the API or CLI state underneath.

When a batch is too broad, split it into:

1. contract or type definition;
2. one caller or producer;
3. one consumer;
4. regression test or proof.

### Shared File Rules

Some files are fracture hotspots. Only one batch at a time should edit them, and only for a narrow
contract reason:

| Hotspot | Allowed reason to edit | Forbidden reason to edit |
|---|---|---|
| `roko-cli/src/orchestrate.rs` | route to a new contract, fix one status semantic, add proof hook | broad cleanup, opportunistic refactor, hidden telemetry changes |
| `roko-cli/src/dispatch_direct.rs` | deprecate or route away from it | adding system prompt, MCP, tools, provider loops, session state |
| chat inline/TUI files | render session state, pass user commands to session owner | own model/provider/session state directly |
| provider config/model routing files | central selector and tests | per-command fallback patches |
| gate service/config files | typed gate contract and verdicts | string-only `"shell"` special case |
| demo hooks/pages | truthful state rendering and typed API consumption | inline hardcoded live-looking fallback values |
| server route modules | explicit `/api/*` JSON contracts | relying on SPA catch-all for API paths |
| telemetry/learning files | normalized observations and outcome linkage | storing unknowns as zero or positive learning from failed artifacts |

### Review Vetoes

A batch should be rejected if any of these are true:

- It says "fallback" but does not distinguish fallback-to-demo from fallback-to-error.
- It marks a workflow successful because a child process exited zero while the artifact is invalid.
- It adds a model alias mapping locally instead of using the central selection contract.
- It adds a prompt string outside the prompt assembly owner.
- It adds another session/history struct.
- It treats missing usage as `$0.00`.
- It makes a UI page prettier without fixing truthfulness.
- It adds a broad abstraction whose first use is only the batch's own code.
- It changes generated sample/demo data to hide a live failure.
- It claims parity without an end-to-end proof command.

### Required Proof Shape

Every runner proof should include at least one negative proof. Examples:

- invalid config fails with a specific message;
- unknown model fails or normalizes with a specific source;
- shell gate `false` fails;
- stub/not-wired gate is not counted as pass;
- missing API route returns typed JSON error, not SPA HTML;
- failed bench start does not create a fake run;
- invalid PRD/plan artifact is rejected despite process success;
- missing usage displays as unknown, not zero.

This matters because most current failures are false-success failures. A proof suite that only
checks happy paths will miss the core defect.

## Runner Sequence

```text
Runner 1: demo-truth             - make the demo UI truthful
Runner 2: execution-contract     - make CLI/server execution contracts coherent
Runner 3: agent-session-parity   - make chat and one-shot use real agent sessions
Runner 4: plan-grounding         - make PRD/plans grounded, valid, and rejectable
Runner 5: telemetry-learning     - make cost/usage/learning/router feedback truthful
Runner 6: security-posture       - secure serve, terminal, CORS, and sharing defaults
Runner 7: mori-polish            - finish remaining Mori UX/slash/demo polish
```

Runners 1 and 2 are the critical path. Runner 3 is the highest-value user-facing runner.
Runners 4 and 5 close the self-hosting loop. Runner 6 is required before anything public or
network-exposed is called production-safe. Runner 7 catches non-core Mori polish.

## Runner 1: `demo-truth`

Status: planned in detail in [RUNNER-PLAN.md](./RUNNER-PLAN.md).

Goal: make false success impossible in the demo app.

What it fixes:

- silent fallback for live API failures;
- mutation fallback that fabricates success;
- bench route and response shape drift;
- share route drift;
- dashboard/knowledge/fleet mixed live/demo state;
- Explorer event envelope handling;
- terminal false-connected states;
- demo scenario undefined-handle crashes;
- Builder workspace/setup/failure display;
- unsupported demo commands pretending to run;
- safe proof scripts.

What it must not do:

- no chat architecture;
- no full model routing;
- no broad telemetry migration;
- no security policy changes;
- no new demo fake data to make pages look healthy.

Acceptance:

- `npm run build` passes in `demo/demo-app`.
- API smoke proves `/api/*` does not return SPA HTML.
- Failed bench POST does not create a fake active run.
- Live bench start/cancel proof works only when explicitly enabled.
- Dashboard has no `NaN%` or mixed fallback/live headline stats.
- `/demo` no longer throws undefined-handle errors.
- Unsupported scenarios are visibly skipped or failed.

## Runner 2: `execution-contract`

Goal: make `roko init`, `roko run`, `roko plan run`, provider resolution, gates, state, and CLI
failure semantics coherent enough that demo and agent sessions can rely on them.

Name: `tmp/runners/execution-contract/`

Estimated size: 28-34 batches. This should be split internally into groups, but kept as one runner
because the same execution contracts must be changed together.

### Architectural Contract

This runner creates the execution contract that all agent-starting commands must use.

Required contract objects:

```rust
struct EffectiveModelSelection {
    requested_model: Option<String>,
    effective_model_key: String,
    provider_key: String,
    provider_kind: String,
    backend_slug: String,
    source: SelectionSource,
    reason: String,
}

enum SelectionSource {
    CliOverride,
    TaskModel,
    RoleConfig,
    CascadeRouter,
    ProjectDefault,
    BuiltInDefault,
}
```

Required behavior:

- explicit CLI `--model` is a hard override unless the command prints a documented stronger policy;
- every command that starts an agent prints/persists effective model/provider/source;
- provider tests honor `--model`;
- unknown model aliases are rejected or normalized before execution;
- generated plans are validated before execution;
- workflow failures exit nonzero or return a structured failed result;
- gate verdicts distinguish passed, failed, skipped/not-wired;
- state and resume paths use the same canonical plan directory.

Forbidden:

- no command-specific fallback chain;
- no hardcoded "try Anthropic, then ZAI" in a leaf command;
- no accepting invalid `tasks.toml` because a model wrote it;
- no new runner state path;
- no more `shell:true` as proof of a coding task.

### Group A: Config and Init Contract

Purpose: a fresh workspace must be runnable without manual migration.

Batches:

1. Locate and document the current init template and config migration path.
   - Write scope: runner context only.
   - Output: `context/CONFIG-CONTRACT.md`.
2. Make `roko init` emit current provider/model schema.
   - Include `[providers.claude_cli]` if Claude CLI is the intended local happy path.
   - Include configured model keys that map to provider/backend slugs.
3. Add a non-interactive `config migrate --yes` or equivalent.
   - Required for scripts and runners.
4. Add config preflight used by `run`, `prd`, `plan`, one-shot, and provider tests.
   - If config is old, fail early with exact command to fix or auto-migrate only when requested.
5. Replace default no-op gates for code profiles.
   - For a Rust repo, default gate should be `cargo check` at minimum.
   - For unknown repos, gates may be explicitly `skipped/not_configured`, not pass.

Acceptance:

- fresh `roko init` followed by a preflight command does not warn about schema v1;
- fresh `roko run --dry-run` or equivalent prints usable provider/model selection;
- default gates are real or explicitly not configured.

### Group B: Effective Model and Provider Selection

Purpose: remove the largest source of divergent behavior.

Batches:

1. Add the `EffectiveModelSelection` module and tests.
2. Wire selection into `roko run`.
3. Wire selection into positional one-shot, but do not implement agent session behavior here.
4. Wire selection into `prd draft`, `prd plan`, `plan generate`, `plan regenerate`.
5. Wire selection into `plan run` task execution.
6. Wire selection into `config providers test` and `config models route`.
7. Print effective selection in human output.
8. Persist effective selection in structured JSON/events where those already exist.

Acceptance:

- the same `--model claude-haiku-4-5` resolves identically in one-shot, `run`, `prd plan`,
  `plan regenerate`, and provider test;
- unsupported models fail with a specific error;
- `config models route <model>` either resolves exactly or is renamed/documented as recommendation.

### Group C: Gate Truth

Purpose: make gates truthful without losing configured shell command details.

Important source finding: `GateService` currently receives only foundation `GateConfig` names in
some paths. A batch must not pretend `"shell"` is a one-line fix unless it can pass program/args
through the full gate contract.

Batches:

1. Map current gate data flow from CLI config to gate service.
   - Write scope: runner context only.
2. Introduce or reuse a typed gate execution config that carries `kind`, `program`, `args`,
   `timeout`, and name.
3. Make shell gates instantiate `ShellGate` with configured program/args.
4. Make unknown gates fail with a clear "unknown gate" message.
5. Add `skipped/not_wired` representation in the least invasive place.
   - Prefer an existing summary/status enum if present.
   - If adding a field to core types is required, do it intentionally and update all consumers.
6. Update learning/dashboard consumers not to count skipped/not-wired gates as pass.
7. Add tests for shell true, shell false, unknown gate, and stub/not-wired gate.

Acceptance:

- configured `program = "true"` passes;
- configured `program = "false"` fails;
- stubs are not counted as validation passes;
- `roko run` does not report success on no-op proof.

### Group D: CLI Failure Semantics and Run State

Purpose: a failed workflow must be impossible to mistake for success.

Batches:

1. Make `roko run` return failed status/nonzero when the workflow halts before success.
2. Make one-shot unsupported/empty-model results failed, not successful empty responses.
3. Make `roko explain <unknown>` nonzero or structured not-found.
4. Make `plan validate` mandatory before plan execution unless explicitly bypassed.
5. Add `plan run --fresh` or documented state reset for reruns.
6. Make status, plan list, and executor state agree on task completion.
7. Make top-level `roko resume` use canonical `plans_dir`.

Acceptance:

- missing API key is a failed command;
- unknown explain topic is not silent success;
- invalid generated plan cannot be run by accident;
- resume finds PRD-generated plans under `.roko/plans`.

### Group E: Learn Path Visibility

Purpose: `learn all` must read the data that execution writes.

Batches:

1. Map learn write/read paths.
2. Align `learn all` with efficiency and episode write paths.
3. Add empty-state messages that say which paths were checked.
4. Add fixture test with one efficiency event and one episode.

Acceptance:

- after a real or fixture run, `roko learn all` reports non-empty data;
- if empty, it names checked paths.

## Runner 3: `agent-session-parity`

Goal: make interactive `roko` and one-shot prompts use a real Mori-style agent session without
growing `dispatch_direct.rs`.

Name: `tmp/runners/agent-session-parity/`

Estimated size: 20-26 batches.

### Architectural Contract

Add one session owner:

```rust
struct ChatAgentSession {
    workdir: PathBuf,
    auth: AuthMethod,
    model_selection: EffectiveModelSelection,
    effort: String,
    system_prompt: String,
    allowed_tools_csv: String,
    mcp_config: Option<PathBuf>,
    claude_session_id: Option<String>,
    api_history: Vec<ChatMessage>,
    http_client: reqwest::Client,
}
```

This type owns session state. UI code may render it; dispatch code may consume it; no other module
should own parallel chat session state.

Required:

- Claude CLI turns delegate to `ClaudeCliAgent` or its shared command builder.
- API turns delegate to existing provider adapters or `ModelCallService`.
- `/system`, `/model`, `/effort`, `/reset` mutate this session.
- returned Claude session id is stored and reused.
- streaming events are forwarded incrementally.

Forbidden:

- no adding prompt/tools/MCP logic directly to `dispatch_direct.rs`;
- no new provider-specific HTTP loop in CLI;
- no second session struct for one-shot;
- no non-streaming-only implementation declared complete.

### Group A: Session Core

Batches:

1. Add session contract docs and tests for state mutation.
2. Add `ChatAgentSession` with constructor from config/workdir/auth.
3. Resolve system prompt from existing prompt services.
4. Resolve tool policy from existing safety/tool contracts.
5. Resolve MCP config from existing discovery/config paths.
6. Store and expose effective model/effort.
7. Implement slash command mutation tests without changing UI yet.

Acceptance:

- unit tests prove `/system`, `/model`, `/effort`, `/reset` change session state;
- no dispatch call only accepts `(&AuthMethod, &str)` for the new path.

### Group B: Claude CLI Turn Execution

Batches:

1. Add a thin `run_claude_turn` method using `ClaudeCliAgent`.
2. Pass workdir, model, effort, system prompt, tools, MCP, resume, timeout.
3. Capture session id from result.
4. Capture visible tool output.
5. Add cancellation/timeout cleanup through existing process management.
6. Add tests with mocked command builder or fake agent result.

Acceptance:

- command builder includes system prompt, tools, MCP, model, effort, resume when available;
- follow-up turn uses prior session id;
- Ctrl-C/timeout does not leave a child process in the test/proof.

### Group C: Streaming

Batches:

1. Parse Claude stream-json as lines arrive.
2. Convert text deltas into inline streaming state.
3. Convert tool events into visible terminal/tool output.
4. Capture final result metadata.
5. Add integration proof for incremental output.

Acceptance:

- long answer appears incrementally;
- final metadata is not lost.

### Group D: One-Shot Uses Session Path

Batches:

1. Route `roko "prompt"` through the same session machinery in non-interactive mode.
2. Ensure one-shot gets system prompt, tools, MCP, model selection, and workdir.
3. Ensure one-shot can be used by scripts without TUI-only assumptions.
4. Deprecate old direct path to fallback only, with logging.

Acceptance:

- `roko "What files are here?"` can use tools and workspace context;
- one-shot and chat report the same effective model for the same inputs.

## Runner 4: `plan-grounding`

Goal: make PRD and plan generation grounded in the intended repository and reject bad artifacts
before execution or learning.

Name: `tmp/runners/plan-grounding/`

Estimated size: 18-24 batches.

### Architectural Contract

Introduce two concepts:

```rust
struct RepoContextPack {
    root: PathBuf,
    project_kind: ProjectKind,
    workspace_members: Vec<String>,
    key_files: Vec<PathBuf>,
    matching_symbols: Vec<SymbolHit>,
    related_prds: Vec<PathBuf>,
    related_plans: Vec<PathBuf>,
}

struct ArtifactValidationReport {
    process_success: bool,
    schema_valid: bool,
    grounded: bool,
    executable: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
}
```

Required:

- PRD/plan prompts receive a bounded repo context pack.
- Existing repo features are identified before proposing new crates.
- Generated artifacts are validated before being accepted.
- Process success is separate from artifact success.

Forbidden:

- no prompt-only "please inspect repo" as the only grounding mechanism;
- no accepting plans with missing required fields;
- no accepting greenfield duplicate crates in an existing workspace;
- no positive learning record for failed artifact validation.

### Group A: Repo Context Pack

Batches:

1. Define the context pack contract and max size.
2. Collect root `Cargo.toml` workspace members.
3. Collect project kind and key source files.
4. Add bounded `rg` symbol/path matches from the requested feature.
5. Include related PRDs/plans.
6. Add temp-workspace/intended-repo detection.

Acceptance:

- a Roko-internal request from a non-Roko temp workspace is flagged ambiguous;
- a Roko repo request names existing crates such as `roko-compose`, `roko-agent`, and
  `roko-cli/src/runner` where relevant.

### Group B: PRD Grounding

Batches:

1. Inject context pack into `prd draft new`.
2. Require `Repository Grounding` section.
3. Validate PRD references existing surfaces for existing-repo mode.
4. Persist context pack and validation report sidecars.

Acceptance:

- PRD for `system-prompt-wiring` references existing prompt/agent/runtime files;
- no accepted PRD claims greenfield if the repo is existing.

### Group C: Plan Validation and Repair

Batches:

1. Require `role` field in generated tasks.
2. Use configured model names or normalize aliases before execution.
3. Validate referenced files/crates against repo context.
4. Reject duplicate new crates unless explicitly allowed.
5. Feed validation errors into `plan regenerate`.
6. Store artifact validation separately from process status.

Acceptance:

- generated `tasks.toml` passes `plan validate` before run;
- invalid plans fail with actionable errors;
- regeneration prompt includes exact validation failures.

## Runner 5: `telemetry-learning`

Goal: make cost, usage, episodes, learning, and cascade router feedback truthful enough for
dashboards and self-improvement.

Name: `tmp/runners/telemetry-learning/`

Estimated size: 20-28 batches.

### Architectural Contract

Do not start with a broad `cost_usd: Option` migration unless it is scoped and tested. Prefer a
normalized event layer that can express unknowns without breaking every downstream type.

Recommended event shape:

```rust
struct UsageObservation {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_tokens: Option<u64>,
    cost_usd: Option<f64>,
    source: UsageSource,
}

enum UsageSource {
    ProviderReported,
    Estimated,
    Unknown,
}
```

Required:

- unknown is not zero;
- costs are not double-counted;
- model/provider are known before logging;
- router receives actual outcomes;
- learning reads what execution writes.

Forbidden:

- no string `"unknown-model"` as a model id;
- no cost event duplicated for gate failure;
- no positive learning from artifact-validation failure;
- no treating skipped gates as pass.

### Group A: Usage Extraction

Batches:

1. Parse Claude CLI stream-json result usage where available.
2. Parse total cost where available.
3. Add `UsageObservation` conversion without breaking existing serialized formats.
4. Persist unknown usage as unknown/null in new sidecars and projections.
5. Backfill display logic to show "unknown" instead of `$0.00` when appropriate.

Acceptance:

- a Claude CLI result with `total_cost_usd` produces nonzero usage;
- absent usage displays as unknown, not free.

### Group B: Cost Event Semantics

Batches:

1. Identify all cost event emitters.
2. Deduplicate agent attempt and gate failure cost logging.
3. Attach gate outcome to the attempt record.
4. Add regression test for one attempt plus failed gate equals one cost event.

Acceptance:

- no double-count for the same agent attempt.

### Group C: Learning and Router Feedback

Batches:

1. Feed successful/failed dispatch outcomes into cascade router.
2. Prune or flag unavailable model slugs.
3. Ensure `learn all` and `/api/learn/efficiency` read aligned data.
4. Exclude failed artifact validation from positive learning.
5. Update dashboard projections to use truthful usage/learning data.

Acceptance:

- cascade router observations increase after real runs;
- dashboards can distinguish zero cost from unknown cost.

## Runner 6: `security-posture`

Goal: make server, terminal, CORS, and share behavior safe enough for real use.

This runner requires policy decisions before implementation. It should not be smuggled into chat
or demo runners.

Decisions needed:

- Should no-args `roko` start `serve` at all?
- Should terminal routes be disabled by default?
- What auth mechanism should local browser UI use?
- What bind addresses are allowed without explicit `--unsafe-public` or equivalent?
- Should share links be local-only, signed, expiring, or scrubbed?

Minimum likely batches:

1. Bind local-only by default.
2. Require explicit flag for public bind.
3. Auth gate terminal routes.
4. Restrict CORS to local UI origins by default.
5. Add share output scrubbing and expiration.
6. Add security smoke tests.

Acceptance:

- public bind without auth fails;
- terminal route requires auth or explicit local-only trust;
- CORS is not wildcard for public bind.

## Runner 7: `mori-polish`

Goal: complete remaining Mori-like UX and command polish after the core contracts are correct.

Examples:

- full slash command set: `/tools`, `/mcp`, `/context`, `/history`;
- richer tool transcript rendering;
- local session history browser;
- MCP server mesh polish;
- better demo seeding once data contracts are honest;
- deploy workflow polish after security posture lands.

This runner should not be started until Runners 1-6 have proof artifacts.

## Issue Coverage Matrix

This maps `COMPREHENSIVE-ISSUES.md` plus the newer demo audit issues.

| Issue class | Disposition |
|---|---|
| 1.1 Fresh workspace defaults to broken provider routing | Runner 2 |
| 1.2 `--model` ignored | Runner 2 |
| 1.3 `config models route` always Sonnet | Runner 2 |
| 1.4 Provider health 0/0 | Runner 2 for configured enumeration, Runner 6/7 for probing policy |
| 1.5 model slug explosion | Runner 5 |
| 2.1 shell gate broken | Runner 2 |
| 2.2 stub gates pass | Runner 2 and Runner 5 consumers |
| 3.1 cost always zero | Runner 5 |
| 3.2 double-count cost | Runner 5 |
| 3.3 negative zero | Runner 1 and Runner 2 display fixes |
| 3.4 unknown-model | Runner 5 |
| 4.1 `learn all` empty | Runner 2 path visibility, Runner 5 full semantics |
| 4.2 cascade router 0 observations | Runner 5 |
| 5.1-5.5 interactive chat | Runner 3 |
| 6.1-6.2 one-shot dispatch | Runner 3, with selection from Runner 2 |
| 7.1 missing role field | Runner 4 |
| 7.2 bad model aliases | Runner 2 rejects/normalizes for execution, Runner 4 fixes generation |
| 7.3 greenfield duplicate plans | Runner 4 |
| 7.4 regenerate ignores validation | Runner 4 |
| 8.1 init schema v1 | Runner 2 |
| 8.2 resume wrong path | Runner 1 small fix and Runner 2 contract |
| 8.3 no config preflight | Runner 2 |
| 9.x API shape mismatches | Runner 1 |
| 10.1 share URL 404 | Runner 1 |
| 10.2 knowledge empty | Runner 5/7 depending on desired seed/source behavior |
| 10.3 bench detail 404 | Runner 1 |
| 10.4 SSE empty on timeout | no bug if no events; Runner 1 filters and reports state honestly |
| 11.x route path mismatches | Runner 1 |
| 12.1 selfhost fails fresh | Runner 2 plus Runner 4 |
| 12.2 builder broken | Runner 1 plus Runner 2 |
| 12.3 race compares broken | Runner 2 plus Runner 1 truthful skip/fail |
| 12.4 providers unconfigured | Runner 1 skip/preflight, Runner 2 provider selection |
| 12.5 explore poor data | Runner 5/7 |
| 12.6 chat fails | Runner 3 |
| 12.7 Mirage placeholder | Runner 1 labels/skips; real Mirage is deferred feature |
| 13.x dashboard data quality | Runner 1 for truth, Runner 5 for real data |
| 14.x UI logic | Runner 1 |
| 15.x security | Runner 6 |
| 16.1 two dispatch paths | Runner 3 removes happy-path dependence on direct dispatch |
| 16.2 duplicate state | Runner 2 state contract |
| 16.3 throwaway HTTP clients | Runner 3 session-owned client for chat/API paths; broader cleanup deferred |
| 16.4 `orchestrate.rs` size | deferred refactor; do not touch until behavior is stable |
| 16.5 demo data drift | Runner 1 |
| 16.6 mutex/unwrap | opportunistic only when touched; not a parity blocker |
| 16.7 no demo seeding | Runner 7 after truth contracts |

## Solution File Coverage

| Source doc | How this plan uses it |
|---|---|
| `00-CONTEXT.md` | Preserves the high-level diagnosis, Mori reference behavior, and "what not to do" constraints. |
| `CLARIFYING-QUESTIONS.md` | Converts unresolved choices into Runner 6 policy decisions and assumptions for Runners 1-5. |
| `E2E-TEST-RESULTS.md` | Feeds Runner 2 and Runner 4 proof cases: provider routing, shell gates, learn path, plan schema, and model flag behavior. |
| `FINAL-SOLUTION.md` | Primary architecture: execution contract, demo truth, ChatAgentSession through existing adapters. |
| `REVISED-BEST-SOLUTION-AFTER-DEMO.md` | Plan grounding and artifact validation become Runner 4. |
| `DEMO-APP-WORKFLOW-AUDIT.md` | Demo truth becomes Runner 1. |
| `E2E-ROKO-DOGFOOD-AUDIT.md` | Execution-contract findings become Runner 2. |
| `DEMO-RUN-AUDIT.md` | Greenfield duplicate plan failures become Runner 4. |
| `COMPREHENSIVE-ISSUES.md` | Coverage matrix above. |
| `MORI-PARITY-BATCH-PLAN.md` | Used for chat/session/gating order, but split to avoid mixing scopes. |
| `MY-TAKE-SHORTEST-PATH.md` | Enforces "use existing adapters, do not grow dispatch_direct." |
| `solution-ACTUAL.md` | Product insight retained; literal implementation replaced by safer session/adapters plan. |
| `effort-estimates.md` | Used only for sizing intuition; the runner order is driven by dependency and false-success risk. |
| `solution-A-surgical.md` | Rejects surgical-only as insufficient; keeps small safety/reliability ideas for Runner 6/7. |
| `solution-B-architectural.md` | Rejects first-mile InferenceGateway rewrite; keeps the central-owner insight as architecture target. |
| `solution-C-phased-migration.md` | Rejects growing `dispatch_direct.rs`; keeps phased proof discipline. |
| `solution-1-service-triad.md` | Defers service-triad architecture; keeps "single owner per concern" principle. |
| `solution-2-cell-graph-engine.md` | Defers cell/graph platform as future phase, not core Mori parity. |
| `solution-3-hybrid-engine-first.md` | Rejects engine-first for the shortest path; platform concepts may return after parity. |
| `streamlined/RUNNER-PLAN.md` | Runner 1 concrete batch plan. |
| `streamlined/FULL-WORK-PLAN.md` | This controlling plan; should be treated as the top-level implementation contract. |

## Anti-Pattern Checklist For Every Runner

A runner should fail review if it introduces any of these:

- A second provider resolution chain.
- A second prompt assembly path for the same mode.
- A second chat/session state owner.
- Raw provider HTTP in CLI code when an adapter exists.
- Terminal transcript scraping as final workflow state.
- Demo data shown as live data.
- Mutation fallback.
- Unknown usage recorded as zero.
- Stub gate counted as pass.
- Process success treated as artifact success.
- A new top-level crate for behavior that already exists in a current crate.
- A broad `orchestrate.rs` refactor mixed with behavior changes.

## Batch Prompt Template

Every batch prompt should follow this exact structure:

```text
You are working in runner <name>, batch <id>.

Goal:
<one sentence>

Architecture contract:
<specific contract from this runner>

Allowed write scope:
- <files>

Read-only context:
- <files/docs>

Do not edit:
- <files/modules>

Required behavior:
- <bullets>

Forbidden behavior:
- <bullets>

Verification:
- <commands>

Final response must include:
- changed files
- verification result
- whether any public contract changed
- any follow-up needed
```

## Final Definition Of Core Mori Parity

After Runners 1-5 land, core Mori parity means these proof runs pass:

1. Fresh repo: `roko init` creates runnable config.
2. Fresh repo: `roko run "build a tiny Rust CLI"` selects the requested model/provider, runs real
   gates, and fails/succeeds truthfully.
3. One-shot: `roko "What files are here?"` uses workspace context and tools.
4. Interactive: `roko` starts a session; follow-up turn uses prior context.
5. Interactive: `/system`, `/model`, `/effort`, `/reset` affect the next turn.
6. Interactive: output streams and Ctrl-C cleans up.
7. PRD: generated PRD includes repository grounding.
8. Plan: generated `tasks.toml` validates before execution.
9. Plan: invalid greenfield duplicate plans are rejected.
10. Learning: costs/tokens are known or explicitly unknown, never fake zero.
11. Learning: cascade router receives dispatch outcomes.
12. Demo UI: all routes show live/demo/failure truthfully.

After Runner 6 lands, this can be called safe for non-local exposure subject to the chosen auth
policy.

After Runner 7 lands, it can be called polished Mori-like UX.

## What Remains Outside Core Mori Parity

These are real but not blockers for the first Mori-like product:

| Feature | Disposition |
|---|---|
| Chain witness / blockchain anchoring | separate phase 2 feature |
| Dream consolidation runtime | platform self-improvement runner |
| Cold substrate archival | platform lifecycle runner |
| A2A agent protocol | future multi-agent protocol runner |
| Full gateway/cell graph architecture | long-term architecture, not first-mile parity |
| `orchestrate.rs` decomposition | refactor only after behavior stabilizes |
| Complete deployment product | after security posture |

## Bottom Line

The previous full plan did not give enough protection against fractured implementation. This
version does. It does not claim every future Mori/platform feature is solved by the first five
runners. It does account for every known issue and gives each issue a disposition. The most
important control is architectural ownership: fix the contracts once, route all user-facing paths
through those contracts, and make every runner prove it did not add another parallel path.

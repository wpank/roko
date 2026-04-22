# Roko Anti-Patterns v2 — Lessons from 661 Batches

Distilled from the 05-01 audit (22 docs, 136 issues), 89 additional subsystem audits
across 16 directories, the converge-runner deep audit, and the MASTER/UNIFIED
implementation plans. These are the recurring habits that produced the current state.
Each pattern appears across multiple subsystems, proving it is systemic, not accidental.

**Previous version:** `ANTI-PATTERNS.md` (2026-04-28) marked most patterns as "Resolved."
The 05-01 audit found they all recurred. This version drops the "Resolved" claims and
focuses on the structural habits that cause recurrence.

**Scope:** 12 categories (A–L), 40 named patterns, 10 reusable agent rules, decision table.

---

## Category A: Duplication Instead of Delegation

### A1. Hand-rolling provider HTTP in surface code

**Pattern:** A surface crate (ACP, CLI chat, serve) needs to call an LLM. Instead of
using the existing `ProviderAdapter` / `ModelCallService`, it writes its own
`reqwest::Client::new()`, constructs the request body, parses SSE events, and extracts
usage — all inline.

**Where it happened:**
- `bridge_events.rs:1459` — ACP's Anthropic HTTP client
- `bridge_events.rs:1666` — ACP's OpenAI-compat HTTP client
- `chat_session.rs:551` — CLI chat's Anthropic HTTP client
- `chat_session.rs:583` — CLI chat's OpenAI-compat HTTP client

**Why it keeps happening:** The shared provider layer (`ProviderAdapter`) only has
`create_agent()` → `Agent::call()`. It has no streaming method. When ACP needed
streaming, the fastest path was to write a raw HTTP client. When chat needed API
dispatch, same thing. The abstraction gap in the shared layer pushes callers to bypass it.

**Root fix:** Add `stream()` to the provider interface. Until that exists, every
surface that needs streaming will re-implement provider HTTP.

**Detection rule:**
```bash
rg 'reqwest::Client::new\(\)' crates/ --type rust | grep -v test | grep -v provider/
# Must return 0 results
```

---

### A2. Reimplementing SSE parsing per caller

**Pattern:** Each surface writes its own `while let Some(chunk) = response.chunk()` loop,
its own `data:` prefix stripping, its own JSON parsing for `content_block_delta` /
`message_delta` / etc.

**Where it happened:**
- `bridge_events.rs:1512-1590` — ACP Anthropic SSE
- `bridge_events.rs:1727-1820` — ACP OpenAI SSE
- `chat_session.rs` — non-streaming (different bug: no streaming at all)
- `roko-agent/src/provider/` — the proper implementation that nobody calls

**Why it keeps happening:** Same as A1. The shared layer doesn't expose streaming,
so callers rewrite the streaming parser.

**Each copy is slightly different:**
- ACP handles `content_block_delta` + `message_delta` + `message_start`
- ACP misses: `content_block_start`, `error`, `ping`, `input_json_delta`
- The proper adapter handles all of these

---

### A3. Multiple session/state owners for the same concept

**Pattern:** The same concept (selected model, provider config, conversation history,
system prompt) is stored in multiple structs. Changes to one don't propagate to the
others.

**Where it happened:**
- `ChatAgentSession.model` vs `ChatAgentSession.model_selection.effective_model_key`
- `AcpSession.config_state.model` vs resolved model in bridge_events dispatch
- `dispatch_direct.rs` auth method model vs `chat_session.rs` model
- CLI `Config` vs core `RokoConfig` overlapping schemas

**Why it keeps happening:** Each runner batch or feature added "one more field" to
the nearest struct. Nobody removed the old field or made it derived. Over time, two
or three fields represent the same concept, and code paths read from different ones.

**Detection rule:** For any concept X, there should be exactly one canonical field.
All other fields should be `fn x(&self) -> &X` accessors that read from the canonical
source.

---

## Category B: Permissive Defaults and Silent Failures

### B1. Safety defaults are permissive for developer convenience

**Pattern:** A safety feature defaults to "off" or "allow everything" so that tests
pass and development is frictionless. Nobody flips it to secure before shipping.

**Where it happened:**
- `dangerously_skip_permissions = true` in roko.toml AND schema default
- `AgentContract::permissive()` as fallback when YAML missing
- `HallucinationDetector::permissive()` in all tests (empty tool list)
- `SupervisionStrategy::default()` with `max_restarts: 0`
- 8 hardcoded `dangerously_skip_permissions: true` sites (PE_01 inventory)

**Why it keeps happening:** The codebase was built iteratively. Each feature needed
agents to actually run during development. Restrictive defaults would break every
test. PE_02 ("flip to secure") was planned but never scheduled.

**Root fix:** Secure by default. Opt-in to bypass via explicit flag + logged warning.
Test fixtures can use `#[cfg(test)]` permissive defaults.

---

### B2. Errors swallowed with `let _ =`

**Pattern:** An operation that can fail (daimon appraise, conductor decide, feedback
record, file write, session persist) has its Result silently discarded.

**Where it happened:**
- `orchestrate.rs` — 15+ `let _ = self.daimon.appraise(...)` calls
- `orchestrate.rs` — `let _ = self.conductor.decide(...)` calls
- `deployments.rs` — `let _ = tokio::fs::rename(...)` (persistence)
- `plans.rs` — `let _ = tokio::fs::write(...)` (snapshot)
- `bridge_events.rs` — `let _ = event_sender.send(...)` (streaming)

**Why it keeps happening:** The caller doesn't want to abort its main operation for
a secondary concern (feedback, telemetry, persistence). `let _ =` is the fastest way
to suppress the warning.

**Root fix:** Categorize operations:
- **Critical path:** propagate errors with `?`
- **Best-effort telemetry:** log at `warn!` level, don't abort
- **Must-succeed persistence:** propagate errors, fail the operation

Never use bare `let _ =` without at minimum a `warn!` log.

---

### B3. `Ok(None)` / `unwrap_or_default()` hiding configuration errors

**Pattern:** When a provider, model, or config lookup fails, the code returns
`Ok(None)` or uses a default value instead of surfacing the error. The caller
interprets "no result" as "nothing to do" and silently skips the operation.

**Where it happened:**
- `orchestrate.rs:7285` — missing provider returns `Ok(None)`, task is skipped
- `model_selection.rs:304` — unknown provider picks alphabetically first model
- `schema.rs:311` — `synthesized_model_profile()` guesses provider from slug prefix
- `chat_session.rs:2874` — failed model resolution updates `model` string but not provider

**Root fix:** Config errors should be errors, not fallbacks. Return typed errors that
name what's missing and what the user should do.

---

## Category C: Write-Only Feedback

### C1. Data recorded but never consumed for decisions

**Pattern:** A runner batch implements the "record data" half of a feedback loop.
The "read data and make a decision" half is never implemented.

**Where it happened:**
- Efficiency events → `.roko/learn/efficiency.jsonl` (recorded with empty model/provider)
- Dream triggers → `.roko/learn/dream_triggers.jsonl` (no consumer exists)
- Routing observations → confidence stats only (not contextual bandit updates)
- Playbook patterns → queried but not injected into system prompts
- Episode JSONL → grows without bound, no rotation, no cleanup

**Why it keeps happening:** Runner batch prompts say "record X." Nobody's prompt says
"read X back and use it to inform decision Y." Each batch optimizes for its local
deliverable (recording works, test passes) without closing the loop.

**Root fix:** Never implement recording without implementing consumption in the same
batch. If consumption requires a different subsystem, the batch should at minimum
create a stub consumer with a TODO that blocks the "wired" claim.

---

### C2. Unknown values collapsed to zero

**Pattern:** When a provider doesn't report token usage, cost, or duration, the code
uses `unwrap_or(0)` or `Usage::default()`. The learning system can't distinguish
"zero tokens" from "usage unknown."

**Where it happened:**
- `bridge_events.rs:1571` — `unwrap_or(0)` for output tokens
- `translate/openai.rs:267` — `Usage::default()` when no usage block
- `runtime_feedback.rs:884` — zeros for absent metadata
- `efficiency.rs:220` — default efficiency event with concrete zeros

**Root fix:** Optional usage must stay optional through the entire pipeline:
provider → response → feedback → runtime event → learning. Convert to display
defaults only at UI boundaries, never at collection boundaries.

---

## Category D: Scope-Limited Fixes

### D1. Fixing the symptom in one surface instead of the root in the shared layer

**Pattern:** A surface crate (ACP, CLI, serve) encounters a gap in the shared layer
(no streaming, no model validation, no config field). Instead of adding the capability
to the shared layer, the surface works around it locally.

**Where it happened:**
- ACP needed streaming → wrote its own SSE parser (should have added streaming to ProviderAdapter)
- ACP needed ClaudeCli support → mapped it to AnthropicApi (should have used ClaudeCliAdapter)
- Chat needed model switch → updated string field only (should have made switch atomic)
- Demo needed gate results → scraped terminal text (should have queried HTTP API)
- Terminal needed prompt detection → regex on output (should have used shell markers)

**Why it keeps happening:** Runner batches had narrow write scopes (≤5 files, ≤300 LOC).
An agent touching ACP couldn't reasonably add streaming to the provider layer and update
all consumers. So it patched locally.

**Root fix:** When a surface needs a shared capability, the fix goes to the shared layer
first, even if that means the batch is bigger. A narrow-scope batch that creates a local
workaround makes the codebase worse.

---

### D2. Marking things "wired" when they compile and unit-test

**Pattern:** A runner batch implements a feature, writes a unit test, passes compile,
and marks the task as "done." CLAUDE.md is updated to say "Wired." But the feature
doesn't work end-to-end because:
- The upstream data is never populated
- The downstream consumer ignores the output
- The test uses permissive mocks that hide real failures

**Where it happened:**
- CascadeRouter: "Wired" but returns default slug (RoutingContext never populated)
- Safety contracts: "Wired" but falls back to permissive on every role
- LLM judge gate: "Wired" but `StubJudgeGate` always fails, skipped at runtime
- Dream consolidation: "Wired" but triggers written, no consumer exists
- Playbook store: "Wired" but results not fed into system prompts

**Root fix:** "Wired" means an integration test proves the end-to-end path works.
Not "code exists" or "unit test passes with mocks."

---

## Category E: Accumulation Without Extraction

### E1. God-file growth by accretion

**Pattern:** Each batch adds 50-200 lines to an existing function. Nobody extracts.
Over 661 batches, `dispatch_agent_with` grew to 2,059 lines and `orchestrate.rs`
grew to 22,635 lines.

**Where it happened:**
- `orchestrate.rs` — 138 functions, 14 exceeding 300 lines
- `bridge_events.rs` — 3,300 lines, 3 functions over 300 lines
- `runner/types.rs` — 1,500 lines, 4 functions over 300 lines

**Why it keeps happening:** Adding to an existing function is easier than extracting.
Each batch's diff is small and reviewable. The cumulative effect is invisible to
individual batch reviewers.

**Root fix:** CI fitness functions that reject functions over a threshold (e.g., 200
lines). Force extraction before the function can grow further.

---

### E2. Parameter explosion instead of context structs

**Pattern:** Each feature adds "one more parameter" to an existing function signature.
Over time, functions accumulate 8-10+ parameters with 3-4 `Option` overrides.

**Where it happened:**
- `dispatch_agent_with` — 8 params including 3 Option overrides
- `run_gate_pipeline` — complex gate config params
- `spawn_agent_with_layer` — SpawnAgentSpec with 15+ fields

**Root fix:** When a function exceeds 5 parameters, introduce a context struct.
`struct DispatchContext { model_override, exec_dir, system_prompt, ... }`.

---

### E3. String-typed dispatch where enums would be safe

**Pattern:** A value that has a fixed set of options is stored as a `String` and
matched with `.as_str()`. Typos compile successfully and fail at runtime.

**Where it happened:**
- `if cfg.command == "claude"` — should be `Command::Claude`
- `match task_def.tier.as_str()` — should be `Tier` enum
- `match verdict.gate.as_deref()` — should be `GatePhase` enum
- `match kind.as_str()` — should be `SourceKind` enum
- `model_selection.provider_kind == ProviderKind::ClaudeCli.label()` — comparing strings not enums

---

## Category F: Enforcement Gap

### F1. Rules written in markdown, not in tooling

**Pattern:** Runner context packs defined comprehensive rules (43 named anti-patterns,
10 review vetoes, 7 performance contracts). None were enforced by automated checks.
The rules existed as instructions to codex agents, not as executable gates.

**Where it happened:**
- Rule: "no raw provider HTTP" → ACP and chat both have raw provider HTTP
- Rule: "dangerously_skip_permissions defaults false" → config defaults true
- Rule: "one reqwest::Client" → ACP creates one per request
- Rule: "unknown is not zero" → zero used throughout

**Root fix:** Convert every rule into a grep/lint/test that runs in CI. A rule
that isn't automated will be violated by the next batch.

**Minimum fitness checks:**
```bash
# No raw provider HTTP outside roko-agent/src/provider/
rg 'reqwest::Client::new\(\)' crates/ --type rust | grep -v test | grep -v provider/

# No permission bypass outside test
rg 'dangerously_skip_permissions.*true' crates/ roko.toml --type-add 'cfg:*.toml' | grep -v test

# No env var reads in library crates
rg 'std::env::var.*API_KEY' crates/roko-{compose,learn,gate,neuro,dreams}/ --type rust | grep -v test

# Functions under 200 lines (proxy check)
# python script or custom lint
```

---

### F2. Cherry-pick merging bypasses review

**Pattern:** Batches are developed on separate branches and cherry-picked onto the
working branch. The cherry-pick result is never reviewed as a whole. Conflicts
resolved during cherry-pick can introduce violations that weren't in either original
branch.

**Root fix:** After cherry-pick waves, run the full fitness suite on the merged
result. Treat the merge as a new artifact that needs verification, not a sum of
verified parts.

---

## Category G: Contract and Type Evasion

### G1. Debug strings as data contracts

**Pattern:** A typed event, verdict, or classification is converted to a string and
later parsed, searched, or matched as if that string were a stable schema.

**Where it happened:**
- `converge-runner/DEEP-AUDIT.md` calls out runtime JSONL/projection code parsing debug strings
- `runner/gate_dispatch.rs` serializes a classification to JSON, then checks whether the string contains `external_environment`
- terminal/demo flows scrape prompt/output text instead of consuming command/result events
- older dispatch helpers use broad "format guesser" functions to recover text from many possible response shapes

**Why it keeps happening:** Strings are easy to log and easy to inspect. Agents use
the visible representation as the integration surface instead of adding a typed
field to the underlying event or result.

**Root fix:** Every cross-module contract gets a typed struct/enum with versioned
serialization. Debug/display strings are for humans only. No production code should
branch on rendered JSON, formatted status text, stderr snippets, or prompt output.

**Detection rule:**
```bash
rg 'contains\\(|to_string\\(|format!\\(' crates/roko-{cli,runtime,gate,serve}/ --type rust
# Manually review hits that classify behavior from rendered text.
```

---

### G2. Success variants carrying errors, noops, or unsupported states

**Pattern:** A success enum variant is reused to carry a non-success outcome because
adding a new outcome type seems too invasive.

**Where it happened:**
- `CommitDone { hash: "noop" }` means no commit was created
- ACP provider failures send a normal completion event after an error chunk
- stub or missing gates have historically returned pass-like results
- shared run creation built placeholder transcripts that looked like successful artifacts

**Why it keeps happening:** The existing state machine only has "done" and "failed",
so agents encode a third state inside a string field to keep the code compiling.

**Root fix:** Add the missing state. Use enums like `CommitOutcome::{Created,
NoChanges, Failed}`, `StreamOutcome::{Completed, Failed}`, and
`ArtifactOutcome::{Valid, Invalid, NotProduced}`. Never require callers to special-case
string sentinels like `"noop"`.

---

### G3. Boolean pairs instead of a status enum

**Pattern:** Two or three booleans are used to represent a mutually exclusive state.
Examples: `passed` plus `skipped`, `success` plus `cancelled`, `process_success` plus
`artifact_valid`.

**Where it happened:**
- `GateVerdict { passed, skipped, skip_reason }`
- `GenerationOutcome { process_success, artifact_valid }`
- workflow reports with `success: bool` plus loosely inferred output/error text
- tool call summaries with `success: bool` but no typed failure domain

**Why it keeps happening:** Booleans are fast to add without touching many callers.
The cost appears later, when impossible combinations need to be interpreted.

**Root fix:** Use `Status` enums. A gate is `Passed`, `Failed`, `Skipped`,
`NotWired`, or `InvalidConfig`. An artifact is `Valid`, `Invalid(report)`, or
`NotProduced`. Illegal combinations should be unrepresentable.

---

### G4. Empty string placeholders instead of optional or validated fields

**Pattern:** Required identifiers are filled with `""` to satisfy a struct while the
real value is unavailable.

**Where it happened:**
- converge audit found empty `agent_id`, `model`, and checkpoint path fields
- `resume()` emitted `StateCheckpointed` with an empty path
- workflow reports fall back to `"unconfigured"` or `"success"` output text
- config/model/provider strings are accepted empty in several compatibility paths

**Why it keeps happening:** Empty strings avoid changing constructor signatures and
are easy to serialize.

**Root fix:** Use `Option<NonEmptyString>` for genuinely optional data and typed
construction errors for required data. Invalid state should fail at creation time.

---

### G5. Hand-rolled parsers for structured config and protocol data

**Pattern:** Code parses TOML, JSONL, provider events, terminal output, or gate
requirements by string manipulation even though serde or a protocol parser should
own the format.

**Where it happened:**
- hand-rolled TOML parsing in workflow pipeline state
- multiple SSE parsers in ACP/chat/provider code
- gate requirement matching with string contains
- terminal demo prompt detection by regex
- `projection.rs` ignoring invalid JSONL lines

**Root fix:** Use serde or one canonical parser per protocol. If the format is not
strict enough to parse structurally, the design should add a typed marker/event
instead of improving the regex.

---

## Category H: Partial Wiring and Shadow Systems

### H1. Built but not wired to live paths

**Pattern:** A subsystem is implemented and tested, but no production entry point
calls it.

**Where it happened:**
- `learning-feedback/ISSUES.md`: full learning loop only reachable from dead `orchestrate.rs`
- `gate-pipeline/ISSUES.md`: SPC alerts, Hotelling T-squared, domain profiles, PRM signals implemented but unused
- `code-intelligence/AUDIT.md`: HDC, SQLite backend, PageRank, structural search built but not used by prompt injection
- `converge-runner/OPEN-ISSUES.md`: GatewayEventWriter, knowledge injection, adaptive thresholds, distillation, SSE completeness all listed as built but not wired

**Why it keeps happening:** Agents optimize for "component exists and tests pass."
They do not prove that the product path invokes the component.

**Root fix:** Every feature doc needs an entry-point coverage table: CLI one-shot,
chat, ACP, serve, run/workflow, background jobs. A feature is not "wired" until at
least one live product path exercises it end to end.

---

### H2. Shadow runtime remains reachable after replacement

**Pattern:** A new runtime/service is introduced as the intended path, but the old
runtime remains compiled and reachable through flags, fallbacks, or alternate entry
points.

**Where it happened:**
- `legacy-orchestrate` keeps old dispatch/direct helpers reachable
- `roko run`, ACP, serve, and chat still have divergent behavior after WorkflowEngine landed
- `dispatch_direct` remains a fallback after ChatAgentSession init failure
- serve background tasks can bypass WorkflowEngine

**Why it keeps happening:** Deleting the old path is risky while parity is incomplete,
so agents keep it "just in case." Over time, new work lands in both paths or only one
path, and divergence returns.

**Root fix:** Replacement work must include a retirement gate: either prove parity
and delete the old path, or make the old path explicit legacy with no silent fallback.
Fallback from the new path to the old path should be treated as a product regression.

---

### H3. Optional production services

**Pattern:** A service that is core to the product principle is modeled as optional,
so production can run without it and silently lose behavior.

**Where it happened:**
- feedback sinks can be absent even though learning is a core feature
- safety layer is optional in tool dispatch paths
- GatewayEventWriter is optional and never instantiated
- MCP config absence silently removes tool access
- budget guardrails are implemented but not mandatory in live paths

**Why it keeps happening:** Optional dependencies make tests and local setup easier.
Agents use `Option` to avoid threading construction through all callers.

**Root fix:** Production constructors require core services. Tests can pass explicit
`NoopFeedbackSink`, `NoopBudgetGuard`, or `RestrictedSafetyLayer`, but production
should not infer no-op behavior from absence.

---

### H4. Path-based sharing and local duplicate traits

**Pattern:** Instead of moving a type to a proper shared crate, a module is included
by path or a local trait is duplicated with a similar shape.

**Where it happened:**
- `StateHub` imported by path from CLI and serve
- `AffectPolicy` duplicated between runtime and foundation
- gate/rung mapping duplicated in runtime, gate, and CLI runner
- code context helper duplicated in prompt and dispatch helpers

**Why it keeps happening:** Local duplication avoids crate dependency work and
compile-cycle concerns.

**Root fix:** If two crates need the same type, promote it to the owning crate and
make both depend on it. Do not use `#[path]` sharing or shape-compatible duplicate
traits in production code.

---

## Category I: Config and Provenance Drift

### I1. Config as a dumping ground for runner convenience

**Pattern:** A batch adds a config field, alias, model entry, or safety bypass to
make its local task work, without validating how it affects the full product.

**Where it happened:**
- root `roko.toml` sets `dangerously_skip_permissions = true`
- `[providers.anthropic]` is named like an API provider but configured as `claude_cli`
- config accepts both `agent.model` and `agent.default_model`
- model/provider aliases are added while old names stay accepted indefinitely

**Why it keeps happening:** Config changes are low-friction and make demos pass
quickly. The blast radius is hidden because many commands read different config
types or synthesize defaults later.

**Root fix:** Config changes need migration, validation, provenance, and ambiguity
diagnostics. Dangerous local settings belong in local override files, not shared
root config.

---

### I2. Runtime synthesis of providers/models from env or command names

**Pattern:** Execution code invents providers or model profiles based on environment
variables, model prefixes, or executable names.

**Where it happened:**
- `RokoConfig::effective_providers()` synthesizes providers from `agent.command` and `ANTHROPIC_API_KEY`
- `ModelCallService::config_for_model()` inserts providers/models at request time
- provider factory maps executable names like `claude`, `codex`, and `cursor-agent` to provider kinds

**Why it keeps happening:** Synthesis makes first-run demos smoother. It also hides
which provider is actually configured and why.

**Root fix:** Synthesis belongs in config resolution, not execution. The resolved
config should include provenance for every provider/model: file, migration, default,
env inference, or command inference.

---

### I3. Stale "resolved" docs and status claims

**Pattern:** Documentation or task trackers mark an anti-pattern as resolved because
a new abstraction exists, even though live paths still bypass it.

**Where it happened:**
- `ANTI-PATTERNS.md` marks provider dispatch, prompt assembly, feedback, runtime convergence, and god-file issues as resolved
- later audits show ACP/chat/serve/run still bypass those abstractions
- `INDEX.md` cross-cutting table also labels several items addressed when the 05-01 audit found regressions

**Why it keeps happening:** The status describes architectural intent, not runtime
coverage.

**Root fix:** Status fields must use `Built`, `WiredInOnePath`, `LiveInAllProductPaths`,
`RetiredOldPath`, or `ProvenByE2E`. Avoid binary "resolved" unless old paths are gone.

---

### I4. Config validation asymmetry between surfaces

**Pattern:** CLI, HTTP, ACP, and runtime each validate or load config differently.
The same project config can be accepted by one surface and rejected or silently
reinterpreted by another.

**Where it happened:**
- HTTP uses validator crate while CLI uses custom logic
- CLI `Config` and core `RokoConfig` overlap and require manual copying
- one-shot chat manually maps fields from CLI config to `RokoConfig`
- model selection, provider factory, and model call service each apply their own defaulting

**Root fix:** One validated domain config should feed every surface. Surface-specific
input structs are acceptable only if they resolve into the same `ResolvedConfig`.

---

## Category J: Runner and Agent Failure Habits

### J1. Local optimization under narrow write scope

**Pattern:** An agent is assigned a small subsystem and fixes the symptom there,
even when the correct fix belongs in a shared lower layer.

**Where it happened:**
- ACP streaming was fixed by adding provider HTTP in ACP instead of adding streaming to the provider layer
- demo status was fixed by scraping terminal output instead of adding explicit command/result events
- chat fallback was kept local instead of converging session dispatch
- gate status mapping was patched in runtime while gate registry ownership remained split

**Root fix:** Prompt agents with the ownership question first: "Which layer should
own this behavior?" If the answer is outside the assigned surface, expand scope or
split the work. Do not accept local copies of shared responsibilities.

---

### J2. Unit-test proof mistaken for product proof

**Pattern:** Agents add unit tests for a helper or mocked service and mark the feature
done without a product-flow test.

**Where it happened:**
- learning components have tests but live paths do not call them
- gate intelligence components have tests but runtime does not drain/observe them
- safety tests use permissive fixtures while production defaults also stay permissive
- model selection tests prove deterministic choices but not dispatch capability/auth/streaming

**Root fix:** Every "wired" claim needs one integration or smoke test that starts
from a real entry point and observes the intended side effect.

---

### J3. Grep-only rules that miss semantic violations

**Pattern:** Agents add negative checks for narrow syntax but the underlying design
escape remains possible.

**Where it happened:**
- checks caught trait duplication and dead imports but missed function bloat, parameter explosion, raw provider logic inside non-provider modules, and path-based module sharing
- no rule enforced "raw provider HTTP only in provider adapters"
- no rule enforced "dangerous permissions cannot appear in shared config"
- no rule enforced "unknown usage cannot become zero"

**Root fix:** Keep grep checks for cheap wins, but pair them with semantic tests or
architectural fitness functions. A rule should fail on the behavior, not only the
exact old spelling.

---

### J4. Success-noop counted as batch success

**Pattern:** A batch reports success because the requested file already existed,
the code compiled, or a placeholder path returned OK.

**Where it happened:**
- converge runner had `success_noop` noted for C01
- generated docs/files existed before the batch attempted to write them
- commit `noop` is treated as workflow success
- custom gates can be no-op or skipped while the surrounding run proceeds

**Root fix:** Batch runners should distinguish `Changed`, `NoopAlreadySatisfied`,
`NoopSkipped`, and `Failed`. Noop may be acceptable, but it is not implementation
evidence.

---

### J5. Instruction compliance treated as trust instead of control

**Pattern:** Prompts instruct agents not to build, not to touch certain files, not
to add stubs, or not to use dangerous permissions, but the environment does not
enforce the instruction.

**Where it happened:**
- orchestration docs note agents ignored no-build instructions
- runner rules forbade raw provider HTTP and dangerous permission defaults, but both returned
- review vetoes lived in markdown and were not CI gates

**Root fix:** Treat agent instructions as hints, not controls. Critical constraints
need sandboxing, restricted PATH, CI checks, file ownership checks, or failing tests.

---

### J6. Overclaiming from architectural vocabulary

**Pattern:** Code adopts the right names (`GatewayEventWriter`, `FeedbackService`,
`PromptAssemblyService`, `WorkflowEngine`, `GateService`) but does not enforce the
semantic contract those names imply.

**Where it happened:**
- `ModelCallService` exists but raw dispatch still bypasses it
- `FeedbackService` exists but live paths can run without feedback
- `GateService` exists but gate/rung mappings are duplicated elsewhere
- `PromptAssemblyService` exists but chat/ACP still use inline or bare prompts

**Root fix:** New abstractions should include deprecation/removal work for the old
paths and fitness checks that prevent bypass.

---

## Category K: Observability, Persistence, and Replay Pitfalls

### K1. Reconstructing truth from lossy events

**Pattern:** Runtime reports or projections infer final state by replaying events
that were emitted as side effects, not by reading the typed state that actually
executed.

**Where it happened:**
- workflow report uses global event-bus replay and fills provider as `None`
- runtime projections skip invalid JSONL lines
- event logger ignores write errors
- dashboard events and learning events use different shapes

**Root fix:** The execution engine should maintain a typed run ledger. Events,
logs, dashboards, and reports should be projections of that ledger, not the source
of truth.

---

### K2. Multiple persistence locations for the same data

**Pattern:** The same conceptual data is written under two paths or stores, and
readers choose whichever path they know about.

**Where it happened:**
- episodes in `.roko/episodes.jsonl` and `.roko/learn/episodes.jsonl`
- cascade-router state appears under more than one `.roko` subtree
- `.roko/learn` versus `.roko/memory` duplication
- terminal sessions exist only in memory while other state is persistent

**Root fix:** One canonical storage location per data type. Legacy paths should be
read-only migration inputs, not active writers.

---

### K3. Non-transactional multi-file updates

**Pattern:** Related state files are updated separately. A crash between writes
leaves the system in a valid-looking but inconsistent state.

**Where it happened:**
- CascadeRouter and gate thresholds written separately
- runtime projection and event log can drift
- config/model/provider changes are not validated as one resolved unit

**Root fix:** Group related state updates under a transaction, manifest, or
content-addressed checkpoint. If that is too heavy, record a version/epoch in every
file and reject mixed epochs on load.

---

### K4. Silent context or history loss

**Pattern:** The system trims, drops, or ignores context to keep running, but the
user and downstream components are not told.

**Where it happened:**
- conversation history is trimmed silently
- `_history_context` suppresses the warning that history may no longer be injected
- context windowing/budget features exist but many entry points bypass them
- terminal sessions disappear after server restart

**Root fix:** Context loss should emit a typed event and show up in reports. If
history was dropped, the model call should carry `context_truncated = true` with
counts and reason.

---

## Category L: Demo and UX False Confidence

### L1. Demo truth from terminal scraping

**Pattern:** A UI or demo decides whether something worked by scraping terminal
text, prompt markers, or shell output.

**Where it happened:**
- terminal demo workflow relies on prompt/output scraping
- base64 fallback redirects stderr to `/dev/null`
- terminal spawn errors close indirectly instead of emitting typed WebSocket errors

**Root fix:** Demos should consume the same typed APIs as product UI: workflow
events, gate events, command status events, and artifact outcomes.

---

### L2. Placeholder demos that do not exercise production invariants

**Pattern:** A demo or scaffold shows the intended experience but avoids the hard
parts: provider auth, real gates, real artifacts, persistence, or failure states.

**Where it happened:**
- demo app routes around real workflow truth
- custom gates and judge gates can be skipped/not wired
- serve has large route surface but some routes are stubs or incomplete
- batch inference route stubs exist without provider integration

**Root fix:** Every demo scenario needs an invariant list: real provider path, real
gate path, real persistence, real failure rendering, real artifact validation. If a
scenario uses fakes, label them explicitly in the UI and docs.

---

## Category M: Async and Concurrency Misuse

### M1. block_on inside async runtime

**Pattern:** Synchronous blocking calls (`block_on`, `block_in_place`,
`.blocking_lock()`) are used inside an async context, deadlocking the runtime or
starving the executor.

**Where it happened:**
- `orchestrate.rs` uses `tokio::runtime::Handle::current().block_on()` inside async fn
- `converge-runner` audit identified `block_on` inside tokio runtime as CRIT-01
- sync gate dispatch blocks the async executor while waiting for shell commands
- JSONL file writes use blocking I/O inside async task contexts

**Why it keeps happening:** Agents need to call sync code from async code (or vice
versa). `block_on` "works" in testing when the runtime has spare threads, but deadlocks
under load.

**Root fix:** Never call `block_on` inside an active tokio runtime. Use
`tokio::task::spawn_blocking()` for sync-in-async. Use channels or `.await` for
async-in-async. Add a clippy/lint rule that rejects `block_on` outside of `main()`.

**Detection rule:**
```bash
rg 'block_on\(|block_in_place\(' crates/ --type rust | grep -v main.rs | grep -v test
# Must return 0 results
```

---

### M2. Shared mutable state without clear ownership

**Pattern:** Multiple async tasks read/write the same data structure through
`Arc<Mutex<_>>` or `Arc<RwLock<_>>` without documenting which task owns writes
and which reads.

**Where it happened:**
- `AcpSession` busy flag race condition (session.rs)
- `ChatAgentSession` model/provider fields updated from multiple paths
- `StateHub` shared between TUI, SSE, and HTTP routes
- terminal session map shared without typed locking protocol

**Root fix:** Document lock ordering and ownership for every shared state struct.
Prefer message-passing (channels) over shared locks for cross-task communication.

---

## Category N: Speculative Infrastructure

### N1. Built for future requirements that never came

**Pattern:** A subsystem is fully implemented for a hypothetical use case, but the
actual product never needed that use case. The dead infrastructure adds compile time,
complexity, and maintenance burden.

**Where it happened:**
- `roko-chain`: 16K LOC of blockchain witness primitives, zero live callers
- `roko-dreams`: Full hypnagogia/imagination/cycle engine, no runtime trigger or cron
- `roko-index`: HDC vector indexing, SQLite backend, PageRank — disabled in prompt assembly
- VCG auction: `vcg_allocate` built and exported but greedy path dominates
- SPC alerts, Hotelling T-squared, domain profiles, PRM signals: gate intelligence implemented but unused
- `roko-daimon`: affect engine loaded per-task but results discarded by `let _ =`

**Why it keeps happening:** Agents build what the design doc describes, not what the
product needs right now. There is no mechanism to distinguish "build this now" from
"this is in the roadmap."

**Root fix:** Before building a subsystem, identify the first live caller. If no live
caller exists yet, the work is a design spike, not an implementation task. Label it
accordingly and do not count it toward "wired" status.

**Detection rule:**
```bash
# Find crates with zero external callers
for crate in crates/roko-*/; do
  name=$(basename "$crate" | tr '-' '_')
  count=$(rg "use ${name}::" crates/ --type rust | grep -v "$crate" | wc -l)
  [ "$count" -eq 0 ] && echo "ZERO external callers: $crate"
done
```

---

### N2. Computed but never applied

**Pattern:** A value is computed (scores, budgets, recommendations, priorities) but
the result is never used by the decision that follows.

**Where it happened:**
- CascadeRouter computes routing scores but all callers pass `RoutingContext::None`
- daimon affect engine computes appraisals but results are discarded
- budget guardrails compute limits but are not mandatory in live paths
- conductor.decide() returns recommendations but callers ignore them
- gate intelligence computes adaptive thresholds but many gate dispatchers don't query them

**Root fix:** If you compute a value, wire it into the next decision in the same PR.
If the decision-maker doesn't exist yet, do not build the computation.

---

## Category O: Convergence Failures

### O1. Multiple implementations of the same behavior

**Pattern:** The same logical operation has N independent implementations, each with
slightly different semantics, error handling, and edge cases.

**Where it happened:**
- **4 dispatch implementations:** `dispatch_agent_with`, `dispatch_direct`,
  `ChatAgentSession::call_provider`, `AcpSession::handle_completion`
- **4 SSE/stream-JSON parsers:** ACP Anthropic, ACP OpenAI, chat Anthropic, provider adapter
- **3 gate dispatch paths:** `gate_service`, `gate_runner`, `gate_dispatch`
- **3 model resolution paths:** `model_selection`, `ModelCallService::config_for_model`,
  `RokoConfig::effective_providers`
- **2 config schemas:** CLI `Config` and core `RokoConfig`
- **2 feedback recording paths:** runtime `EffectDriver` and CLI `runtime_feedback`

**Why it keeps happening:** Each surface was built by a different batch. The batch
makes the surface work. Nobody is prompted to converge across surfaces.

**Root fix:** Before adding a new implementation, search for existing ones. If one
exists, extend it. If extending is infeasible, document why and create a convergence
task. Never silently add a second implementation.

**Detection rule:**
```bash
# SSE parsers outside provider/
rg 'data:\s' crates/ --type rust | grep -v provider/ | grep -v test
# Config schemas
rg 'struct.*Config' crates/roko-{cli,core}/ --type rust | grep 'pub struct'
```

---

### O2. Format guesser instead of typed contract

**Pattern:** A function accepts multiple possible input shapes and uses heuristics
(string length, prefix checking, JSON probing) to guess which format it received.

**Where it happened:**
- 130-line format guesser in dispatch helpers trying to recover text from many response shapes
- provider factory maps executable names to provider kinds by substring
- model selection guesses provider from model slug prefix
- config migration warns but still accepts old format

**Root fix:** Define the contract at the boundary. If you accept JSON, parse it into a
typed struct. If you accept text, validate the format. If you need to support old
formats, migrate them into the canonical format once at load time, not repeatedly at
use time.

---

## Category P: State Machine and Lifecycle Gaps

### P1. Incomplete state machine transitions

**Pattern:** A state machine defines all states but only implements transitions for the
happy path. Error, cancellation, timeout, and retry transitions are missing or lead
to undefined behavior.

**Where it happened:**
- workflow engine: cancel/timeout transitions not fully wired
- agent lifecycle: start/stop/crash states defined but restart logic has max_restarts: 0
- terminal sessions: spawn/running states exist but no clean shutdown on error
- ACP sessions: busy flag set but never cleared on error paths
- plan runner: tasks can get stuck in running state if agent crashes

**Root fix:** For every state machine, draw the full transition table including error
and cancellation edges before implementing. Every state must have a defined exit path.
Untested transitions should be marked as `todo!()` or `unimplemented!()`, not silently
ignored.

---

### P2. Resume/restart emitting events with stale or empty data

**Pattern:** When a process resumes or restarts, it emits the same events as a fresh
start but with empty or stale data, confusing downstream consumers.

**Where it happened:**
- `resume()` emits `StateCheckpointed` with an empty path
- efficiency events after resume carry empty model/provider fields
- feedback records after model switch carry the old model name
- workflow restart replays events from the beginning with current timestamps

**Root fix:** Resume events should carry `ResumeContext { previous_checkpoint, resumed_at,
reason }`. Do not reuse creation events for resume. Downstream consumers must be able to
distinguish "new" from "resumed."

---

## Reusable Agent Prompt Rules (Short Form)

Use these as short instructions when assigning future agents:

1. Name the owning layer before editing. If the correct owner is outside your file,
   change the owner or stop.
2. Do not add a second implementation of provider dispatch, gate dispatch, prompt
   assembly, feedback recording, config resolution, or event replay.
3. Do not encode a new state in a string, boolean pair, sentinel value, or debug
   rendering. Add a typed enum/struct.
4. Do not call something wired unless a live entry point exercises it end to end.
5. Do not make production services optional. Use explicit no-op test services only
   in tests.
6. Do not collapse unknown telemetry to zero.
7. Do not silently fallback from a safer/newer path to a weaker/legacy path.
8. Do not mark no-op as success. Distinguish no change from implemented change.
9. Do not trust markdown rules for critical behavior. Add a check, test, or sandbox.
10. Do not leave stale "resolved" claims after finding a bypass. Update the status
    vocabulary to reflect runtime coverage.
11. Do not call `block_on` inside an async runtime. Use `spawn_blocking` or channels.
12. Do not build a subsystem before identifying its first live caller.
13. Do not compute a value without wiring it into the next decision.
14. Do not add a format guesser. Define the contract at the boundary.
15. Do not implement only happy-path transitions. Draw the full state table first.

---

## Quick Reference: Before Writing Code

| Question | If yes → | If no → |
|----------|----------|---------|
| Does this provider call exist in roko-agent? | Use it | Add it to roko-agent, then use it |
| Does this concept already have a field? | Use the existing field | Add one field in one struct |
| Does this fail safely if config is wrong? | Good | Return a typed error, not a fallback |
| Does this record data? | Also implement the consumer | Don't record without consuming |
| Is my function > 150 lines? | Extract before adding | Proceed |
| Is my parameter count > 5? | Use a context struct | Proceed |
| Am I using `.as_str()` to match? | Use an enum instead | Proceed |
| Am I using `let _ =`? | At minimum add `warn!` | Proceed |
| Am I adding a capability to one surface? | Add it to the shared layer first | Proceed |
| Can I prove this works end-to-end? | Mark as "wired" | Mark as "built, not wired" |
| Am I adding a status with two booleans or a sentinel string? | Add an enum | Proceed |
| Am I relying on old docs that say "resolved"? | Verify live paths first | Proceed |
| Am I adding an optional core service? | Use explicit test no-op only | Proceed |
| Am I parsing terminal/debug/log text? | Add a typed event/result | Proceed |
| Does a second implementation of this already exist? | Converge, don't duplicate | Proceed |
| Am I using `block_on` inside async? | Use spawn_blocking | Proceed |
| Is there a live caller for what I'm building? | Good | Build the caller first |
| Am I guessing the input format? | Define the contract | Proceed |
| Have I drawn error/cancel transitions? | Good | Draw them before coding | Proceed |

---

## Curated Agent Prompts

These are copy-paste-ready prompt blocks for roko agents. Each one targets a specific
failure mode observed across 661 batches. Include the relevant blocks in system prompts
or task instructions.

### Prompt: Ownership Check (prevents A1, A2, D1, J1, O1)

```
BEFORE EDITING: Identify which crate/module owns the behavior you need.

Search for existing implementations:
  rg 'fn function_name\|struct TypeName\|trait TraitName' crates/ --type rust

If the behavior exists in a shared crate (roko-agent, roko-core, roko-gate,
roko-compose, roko-runtime), use it. Do not reimplement it in a surface crate
(roko-cli, roko-acp, roko-serve).

If the shared crate is missing a capability you need (e.g., streaming, validation),
add the capability to the shared crate first, then use it from the surface.

NEVER create a second implementation of:
- Provider HTTP/SSE dispatch (owned by roko-agent/src/provider/)
- Gate dispatch (owned by roko-gate/src/gate_service.rs)
- Prompt assembly (owned by roko-compose/src/system_prompt_builder.rs)
- Config resolution (owned by roko-core/src/config/)
- Model selection (owned by roko-cli/src/model_selection.rs)
- Feedback recording (owned by roko-runtime/src/effect_driver.rs)
```

### Prompt: Type Safety (prevents E3, G1, G2, G3, G4, G5)

```
TYPE SAFETY RULES — do not violate:

1. ENUMS NOT STRINGS: If a value has a fixed set of options, use an enum.
   Bad:  if kind.as_str() == "claude_cli"
   Good: match kind { ProviderKind::ClaudeCli => ... }

2. ENUMS NOT BOOLEAN PAIRS: If states are mutually exclusive, use an enum.
   Bad:  struct Verdict { passed: bool, skipped: bool, skip_reason: Option<String> }
   Good: enum GateStatus { Passed, Failed, Skipped(String), NotWired, InvalidConfig }

3. OPTION NOT EMPTY STRING: If a value can be absent, use Option<T>.
   Bad:  agent_id: String  (set to "" when unknown)
   Good: agent_id: Option<AgentId>

4. TYPED OUTCOMES NOT SENTINELS: If an operation can produce different results,
   use an enum.
   Bad:  CommitDone { hash: "noop" }
   Good: enum CommitOutcome { Created(Hash), NoChanges, Failed(Error) }

5. STRUCT NOT DEBUG STRING: Never branch on rendered JSON, formatted text,
   or debug output. Add a typed field.
   Bad:  if json_string.contains("external_environment")
   Good: match classification.failure_domain { FailureDomain::External => ... }
```

### Prompt: Feedback Loop Integrity (prevents C1, C2, K1, N2)

```
FEEDBACK LOOP RULES:

1. NEVER record data without implementing the consumer in the same PR.
   If the consumer requires a different subsystem, create a stub consumer
   with a TODO and do not mark the feature as "wired."

2. NEVER collapse unknown to zero.
   Bad:  tokens: usage.input_tokens.unwrap_or(0)
   Good: tokens: usage.input_tokens  // Option<u64>, stays None if unknown
   Convert to display defaults at UI boundaries only.

3. NEVER reconstruct truth from side-effect events.
   Bad:  replay event log to build workflow report
   Good: read the typed run ledger; events are projections of it

4. NEVER compute a value and discard the result.
   Bad:  let _ = self.daimon.appraise(...)
   Good: let appraisal = self.daimon.appraise(...)?;
         self.adjust_dispatch(appraisal);
   If you truly don't need the result, explain why in a comment and log at warn!.
```

### Prompt: Safety and Permissions (prevents B1, B2, B3, I1, J5)

```
SAFETY RULES — violations will be rejected:

1. SECURE BY DEFAULT. Safety features default to restrictive.
   Bad:  dangerously_skip_permissions = true
   Good: dangerously_skip_permissions = false  // override in local .roko.local.toml

2. NEVER use bare `let _ =` on a Result.
   Bad:  let _ = tokio::fs::write(path, data);
   Good: if let Err(e) = tokio::fs::write(path, data).await {
             warn!("Failed to persist {}: {e}", path.display());
         }

3. NEVER use Ok(None) or unwrap_or_default() to hide config errors.
   Bad:  provider.ok_or(None)?  →  caller silently skips
   Good: provider.ok_or_else(|| ConfigError::MissingProvider { name })?

4. NEVER add dangerous permission flags to shared config (roko.toml).
   Dangerous local overrides belong in .roko.local.toml with an explicit
   reason and environment guard.

5. NEVER trust agent instructions as enforcement. Critical constraints
   need CI checks, sandboxing, or failing tests.
```

### Prompt: Wiring Verification (prevents D2, H1, H2, H3, J2, J4, J6, I3)

```
WIRING VERIFICATION — required before marking anything "done":

1. "WIRED" means a live product path exercises the feature end to end.
   Not "code exists." Not "unit test passes with mocks."

   Verify by tracing from entry point to effect:
   - CLI one-shot: cargo run -p roko-cli -- <command>
   - Chat session: roko chat → exercise the feature
   - ACP/Zed: connect Zed and trigger the feature
   - Serve: curl the relevant HTTP route
   - Plan run: run a plan that exercises the feature

2. STATUS VOCABULARY:
   - Built: code exists, compiles, has unit tests
   - WiredInOnePath: one live entry point exercises it
   - LiveInAllPaths: all surfaces (CLI, chat, ACP, serve, run) exercise it
   - ProvenByE2E: integration test proves the path
   - RetiredOldPath: old implementation deleted

   Never use "Resolved" or "Done" without specifying which level.

3. NO-OP IS NOT SUCCESS.
   If your batch found the work already done, report NoopAlreadySatisfied.
   If you created a placeholder, report Built, not Wired.

4. SHADOW RUNTIMES: If you introduce a new path, delete or gate the old path.
   A fallback from new → old is a regression, not resilience.

5. OPTIONAL CORE SERVICES: Production constructors must require core services
   (feedback, safety, budget). Only test constructors may use no-op variants.
```

### Prompt: Code Size and Structure (prevents E1, E2, M1, M2)

```
CODE STRUCTURE RULES:

1. MAX FUNCTION LENGTH: 200 lines. If your edit would push a function over
   200 lines, extract a helper first, then add your code.

2. MAX PARAMETERS: 5. If a function has more than 5 parameters, introduce
   a context struct:
   Bad:  fn dispatch(model, provider, prompt, override_a, override_b, exec_dir, ...)
   Good: fn dispatch(ctx: &DispatchContext)

3. NO block_on INSIDE ASYNC: Never call block_on inside a tokio runtime.
   Use tokio::task::spawn_blocking() for sync-in-async.

4. SHARED STATE: Document lock ordering for every Arc<Mutex<_>>.
   Prefer channels over shared locks for cross-task communication.

5. ONE CANONICAL LOCATION per data type. If you need data that's already
   stored somewhere, read it from that location. Do not create a second
   copy in a different path.
```

### Prompt: Build Only What's Needed (prevents N1, N2, L2)

```
BUILD DISCIPLINE:

1. IDENTIFY THE FIRST LIVE CALLER before building a subsystem.
   If no live caller exists, this is a design spike, not implementation.

2. DO NOT BUILD:
   - Subsystems for future requirements that have no current caller
   - Computed values with no downstream consumer
   - Format guessers when you can define a typed contract
   - Placeholder demos that bypass production invariants

3. IF YOU COMPUTE, CONSUME: Every computed value must be wired into
   a decision in the same PR. If the decision-maker doesn't exist,
   don't build the computation.

4. STATE MACHINES: Draw the full transition table (including error,
   cancel, timeout, and retry) before implementing. Every state must
   have a defined exit. Untested transitions use todo!(), not silence.
```

---

## CI Fitness Checks (Enforce the Rules)

Implement these as CI gates. A rule that isn't automated will be violated by the next batch.

```bash
#!/bin/bash
# roko-fitness-checks.sh — run in CI before merge

set -e

echo "=== F1: No raw provider HTTP outside roko-agent ==="
if rg 'reqwest::Client::new\(\)' crates/ --type rust | grep -v test | grep -v provider/; then
  echo "FAIL: Raw reqwest::Client found outside provider layer"
  exit 1
fi

echo "=== F2: No dangerous permission bypass in shared config ==="
if rg 'dangerously_skip_permissions.*=.*true' roko.toml crates/ --type-add 'cfg:*.toml' | grep -v test; then
  echo "FAIL: Dangerous permission bypass in shared config"
  exit 1
fi

echo "=== F3: No env var reads in library crates ==="
if rg 'std::env::var.*API_KEY' crates/roko-{compose,learn,gate,neuro,dreams}/ --type rust | grep -v test; then
  echo "FAIL: Env var access in library crate"
  exit 1
fi

echo "=== F4: No block_on inside async runtime ==="
if rg 'block_on\(' crates/ --type rust | grep -v main.rs | grep -v test | grep -v 'fn main'; then
  echo "FAIL: block_on found outside main()"
  exit 1
fi

echo "=== F5: No unwrap_or(0) on usage/token/cost fields ==="
if rg 'unwrap_or\(0\)' crates/ --type rust | grep -i 'token\|usage\|cost'; then
  echo "FAIL: Unknown usage collapsed to zero"
  exit 1
fi

echo "=== F6: No duplicate SSE parsers ==="
sse_count=$(rg 'data:.*\[DONE\]|event:\s*content_block' crates/ --type rust -l | grep -v provider/ | grep -v test | wc -l)
if [ "$sse_count" -gt 0 ]; then
  echo "FAIL: SSE parsing found outside provider layer ($sse_count files)"
  exit 1
fi

echo "All fitness checks passed."
```

---

## Pattern Frequency Table

How often each pattern appeared across the 89 audit docs and 22 05-01 audit docs:

| Pattern | Occurrences | Subsystems Affected | Severity |
|---------|-------------|---------------------|----------|
| A1 Hand-rolled provider HTTP | 4 sites | ACP, CLI, chat, serve | CRITICAL |
| A2 Duplicate SSE parser | 4 copies | ACP(2), chat, provider | CRITICAL |
| A3 Multiple state owners | 8+ sites | ACP, CLI, chat, config | HIGH |
| B1 Permissive safety defaults | 8+ sites | config, runner, safety, gate | CRITICAL |
| B2 Errors swallowed | 15+ sites | orchestrate, deploy, bridge | HIGH |
| B3 Ok(None) hiding errors | 6+ sites | model, provider, config | HIGH |
| C1 Write-only feedback | 5 loops | efficiency, dreams, routing, playbook, episodes | HIGH |
| C2 Unknown = zero | 4+ adapters | usage, openai, feedback, efficiency | HIGH |
| D1 Surface-local fix | 5+ surfaces | ACP, CLI, chat, demo, terminal | HIGH |
| D2 False "wired" claims | 5+ subsystems | cascade, safety, judge, dreams, playbook | HIGH |
| E1 God-file growth | 3 files | orchestrate, bridge, runner | HIGH |
| E2 Parameter explosion | 3+ functions | dispatch, gate, spawn | MEDIUM |
| E3 String-typed dispatch | 5+ sites | command, tier, gate, source, provider | MEDIUM |
| F1 Markdown-only rules | 43 rules | all subsystems | HIGH |
| F2 Cherry-pick bypass | systemic | all batches | MEDIUM |
| G1 Debug strings as contracts | 4+ sites | runner, terminal, demo, dispatch | HIGH |
| G2 Success carrying errors | 4+ sites | commit, ACP, gates, runs | HIGH |
| G3 Boolean pairs | 4+ sites | gate, workflow, tool, generation | MEDIUM |
| G4 Empty string placeholders | 4+ sites | agent, checkpoint, workflow, config | MEDIUM |
| G5 Hand-rolled parsers | 5+ sites | TOML, SSE, gate, terminal, JSONL | MEDIUM |
| H1 Built not wired | 25+ components | learning, gate, code-intel, chain, dreams | CRITICAL |
| H2 Shadow runtime | 4+ paths | orchestrate, dispatch, workflow, serve | HIGH |
| H3 Optional production services | 5+ services | feedback, safety, budget, gateway, MCP | HIGH |
| H4 Path-based sharing | 4+ sites | StateHub, AffectPolicy, gate/rung, context | MEDIUM |
| I1 Config dumping ground | 6+ fields | roko.toml, permissions, providers, aliases | HIGH |
| I2 Runtime synthesis | 3+ layers | config, model-call, provider | MEDIUM |
| I3 Stale resolved docs | 5+ claims | ANTI-PATTERNS.md, INDEX.md, component specs | MEDIUM |
| I4 Config validation asymmetry | 4 surfaces | CLI, HTTP, ACP, runtime | MEDIUM |
| J1 Local optimization | 4+ surfaces | ACP, demo, chat, gate | HIGH |
| J2 Unit-test as proof | 5+ components | learning, gate, safety, model, dispatch | HIGH |
| J3 Grep-only rules | systemic | all runner contexts | MEDIUM |
| J4 Success-noop | 3+ sites | converge, commit, gates | MEDIUM |
| J5 Instructions as trust | systemic | all runner batches | HIGH |
| J6 Overclaiming from names | 5+ services | ModelCall, Feedback, Gate, Prompt, Workflow | HIGH |
| K1 Lossy event replay | 3+ sites | workflow, projection, logger | HIGH |
| K2 Multiple persistence | 3+ data types | episodes, cascade, memory | MEDIUM |
| K3 Non-transactional updates | 3+ pairs | router+thresholds, projection+log, config | MEDIUM |
| K4 Silent context loss | 4+ sites | history, context, terminal, budget | MEDIUM |
| L1 Demo terminal scraping | 3+ sites | terminal, demo, base64 | MEDIUM |
| L2 Placeholder demos | 4+ sites | demo, gates, serve, inference | MEDIUM |
| M1 block_on in async | 3+ sites | orchestrate, gate, JSONL | CRITICAL |
| M2 Shared mutable state | 4+ sites | ACP, chat, StateHub, terminal | HIGH |
| N1 Built for future | 6+ subsystems | chain, dreams, index, VCG, SPC, daimon | HIGH |
| N2 Computed not applied | 5+ values | cascade, daimon, budget, conductor, thresholds | HIGH |
| O1 Multiple implementations | 6+ behaviors | dispatch, SSE, gate, model, config, feedback | CRITICAL |
| O2 Format guesser | 4+ sites | dispatch, provider, model, config | MEDIUM |
| P1 Incomplete state machine | 5+ machines | workflow, agent, terminal, ACP, plan | HIGH |
| P2 Stale resume events | 4+ sites | checkpoint, efficiency, feedback, workflow | MEDIUM |

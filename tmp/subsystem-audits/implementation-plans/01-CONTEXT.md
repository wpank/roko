# 01 — Codebase Context (read once before any plan)

This file gives a fresh agent everything they need to know about the roko
codebase to start implementing tasks from this folder.

---

## Repo Layout

Workspace root: `/Users/will/dev/nunchi/roko/roko/`

Top-level directories:

```
crates/        Rust workspace, ~18 crates (the engine)
demo/          React + TypeScript demo app (Vite)
contracts/     Foundry smart-contract crate (Solidity)
scripts/       Developer scripts and the fitness inventory checks
tmp/           Audits, scratch space, research notes (this folder lives here)
```

---

## Cargo Workspace Layout

The 18 production crates, grouped by responsibility:

| Crate | Purpose |
|---|---|
| `roko-core` | Domain types, config schema, validation, foundation traits |
| `roko-runtime` | WorkflowEngine, RunLedger, EffectDriver, gate/effect orchestration |
| `roko-agent` | ModelCallService, dispatch_resolver, provider adapters, safety contracts |
| `roko-cli` | CLI entrypoints, chat REPL, plan/run commands, runner v2, dashboard, dispatch_direct (legacy), runtime_feedback (sinks) |
| `roko-acp` | Agent Client Protocol (JSON-RPC) bridge for editors (Zed, Cursor, etc.) |
| `roko-serve` | HTTP/WebSocket API. ~85 routes. Embeds React UI |
| `roko-gate` | 7-rung gate pipeline + adaptive thresholds + SPC/CUSUM/EWMA detectors |
| `roko-learn` | Cascade router, contextual bandit, episode/efficiency learners, ~14 dead modules |
| `roko-neuro` | Episodic memory store, knowledge ingestion, dreams/admission |
| `roko-chain` | EVM-bound primitives (mostly dormant) |
| `roko-deploy` | Railway / Docker / daemon deploy |
| `roko-codeintel` | Tree-sitter symbol graph, HDC fingerprints |
| `roko-prompt` | SystemPromptBuilder (9-layer assembly) |
| `roko-config` | Live-load helpers, hot-reload, env binding |
| `roko-events` | Cross-crate event types |
| `roko-orch` | (Old) orchestration types — being absorbed |
| `roko-mcp` | MCP server adapters |
| `roko-tooling` | Internal CLI plumbing (codegen, layer-check) |

The big files you should know about:

- `crates/roko-cli/src/orchestrate.rs` — **22,756 lines**. The "god file."
  Contains `dispatch_agent_with` (~2,059 lines, line 14575+), gate pipeline
  glue, model selection, prompt assembly, telemetry recording, and dozens of
  one-off helpers. Plan 20 deconstructs this file; T5-35 is the first slice.
- `crates/roko-cli/src/chat_inline.rs` — ~4,100 lines. The interactive chat
  loop with two near-identical state machines (one for inline, one for
  fullscreen). Has the slash-command palette and `/model` switching.
- `crates/roko-cli/src/runner/event_loop.rs` — runner v2 event loop. Source
  of feedback events. T1-8/T1-9 already shipped here.
- `crates/roko-acp/src/bridge_events.rs` — ACP wire/protocol layer. Recent
  audit fixes for ContentBlock and `session/update` shape live here.

---

## Build & Test Commands

The workspace uses Rust **stable** for `cargo check`/`test`, **nightly** for
`cargo fmt` (the project's `rustfmt.toml` requires unstable features).

Pre-commit gate (run before every commit; see `02-ANTI-PATTERNS.md`):

```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

Faster smoke during iteration:

```bash
cargo check --workspace
cargo test -p <crate>           # one crate
cargo test -p <crate> <pattern> # one test pattern in one crate
```

Frontend (demo app):

```bash
cd demo/demo-app
yarn install
yarn dev      # http://localhost:5173
yarn build    # produces static bundle that roko-serve embeds
```

**User rule**: always use `yarn`, not `npm`.

---

## Important Singletons & Patterns

### `ModelCallService` (in `roko-agent`)

The shared LLM dispatch surface. Everything that calls a model **should** go
through this. Today, four paths dispatch: `ModelCallService`,
`DispatchResolver`, `dispatch_direct.rs` (legacy), and route-local
`reqwest::Client` constructions in `roko-serve`. The migration plan is
documented in `15-tier5-architectural.md` (T5-36, T5-37) and
`22-dispatch-streaming-completion.md`.

API:

- `call(req: ModelCallRequest) -> ModelCallResponse` — non-streaming
- `stream(req: ModelCallRequest) -> impl Stream<Item = ModelStreamEvent>` —
  streaming with typed events including `Failed`

### `DispatchResolver` (in `roko-agent`)

Translates a high-level dispatch intent (model name, role, complexity hint)
into a concrete `DispatchPlan`. Today it returns `Unvalidated` diagnostics for
auth and capability — that's the open hole. See plan 22 (P2 in doc 35).

### `RunLedger` (in `roko-runtime`)

Append-only typed log of `RunLedgerEntry` for one workflow run. Replaces the
old "infer report from replayed events" path. Skeleton landed; gate/artifact/
event/resume migrations open. See plan 24.

### `GateRegistry` (in `roko-gate`)

Single source of truth for gate alias → rung → metadata mapping. Skeleton
landed; one duplicate map removed. Two more duplicate maps to fold in.

### `CommandEvent` DTOs (in `roko-serve`)

Typed terminal lifecycle events: `Started`, `Output`, `Exited`, `SpawnFailed`,
`Cancelled`. Demo automation still uses regex prompt scraping; see plan 26.

### `UsageObservation` (in `roko-core`)

Canonical optional usage telemetry: `input_tokens: Option<u64>`,
`output_tokens: Option<u64>`, `total_tokens: Option<u64>`,
`cost_usd: Option<f64>`. Only some providers/parsers/consumers preserve `None`.
Migrating the rest is T4-31. See plan 25.

### `FeedbackFacade` + `FeedbackSink` (in `roko-cli/src/runtime_feedback`)

Composite sink: events fan out to `EpisodeSink`, `RoutingObservationSink`,
`KnowledgeIngestionSink`, `ConductorObservationSink`, `DreamTriggerSink`. The
last two are write-only and get deleted in T2-20.

### `SafetyLayer` (in `roko-agent/src/safety`)

Default constructor uses `AgentContract::restricted("default")` (T1-15
landed). Permissive contracts only appear in test code. Recovery actions are
defined but never invoked end-to-end; see plan 28.

### `SystemPromptBuilder` (in `roko-prompt`)

Nine-layer assembly: identity, capability, role, task, context (HDC),
playbooks, scratch, system, hint. Only one of six entry points uses the full
builder; HDC similarity step is disabled; playbook layer reads nothing from
the playbook store. See plan 30.

### `StateHub` + `DashboardEvent` (in `roko-runtime`)

`watch::Sender<DashboardSnapshot>` + broadcast channel for events. Read by the
TUI. CLI and serve don't surface it (yet). The progressive-formality and
push-progress ideas in doc 42 lean on this.

---

## Key File Paths You'll Touch Often

```
crates/roko-cli/src/orchestrate.rs         # The god file
crates/roko-cli/src/chat_inline.rs         # Chat REPL
crates/roko-cli/src/runner/event_loop.rs   # Runner v2 event loop
crates/roko-cli/src/runtime_feedback/      # Sinks
crates/roko-cli/src/commands/plan.rs       # Plan-run construction (sinks wired here)
crates/roko-cli/src/dispatch_direct.rs     # Legacy dispatch path
crates/roko-agent/src/model_call_service.rs
crates/roko-agent/src/dispatch_resolver.rs
crates/roko-agent/src/safety/mod.rs
crates/roko-acp/src/bridge_events.rs
crates/roko-acp/src/session.rs
crates/roko-serve/src/routes/mod.rs        # Router assembly + auth/CORS layers
crates/roko-serve/src/routes/middleware.rs # Auth + CORS + scrub
crates/roko-serve/src/routes/agents.rs     # Agent CRUD + manifest
crates/roko-serve/src/routes/config.rs     # Config get/put/reload
crates/roko-serve/src/lib.rs               # Bind, port, builder
crates/roko-core/src/config/mod.rs         # load_config (strict validator wired)
crates/roko-core/src/config/schema.rs      # RokoConfig + sub-configs
crates/roko-core/src/config/validation.rs  # Strict validator
crates/roko-core/src/config/provenance.rs  # Resolved/Validated wrappers
crates/roko-core/src/usage.rs              # UsageObservation
crates/roko-runtime/src/run_ledger.rs      # RunLedger
crates/roko-runtime/src/effect_driver.rs
crates/roko-runtime/src/workflow_engine.rs
crates/roko-gate/src/registry.rs           # GateRegistry
crates/roko-gate/src/adaptive_threshold.rs # observe / observe_pipeline / drain_spc_alerts
crates/roko-learn/src/cascade_router.rs    # CascadeRouter
crates/roko-learn/src/lib.rs               # Module list — dead modules to remove
crates/roko-prompt/src/                    # SystemPromptBuilder
roko.toml                                  # Root config; dangerous overrides removed
demo/demo-app/src/lib/scenario-runners/    # Demo automation (regex scraping)
scripts/roko-fitness-checks.sh             # Inventory script (will become CI gate)
scripts/docs-status-check.sh               # Doc status inventory
```

---

## Configuration Surface

Loading order:

1. `roko.toml` at workspace root — shared config; dangerous bypasses **removed**
2. `~/.config/roko/local.toml` — local-only overrides (dangerous bypass requires
   reason / scope / expiry / acknowledgement env)
3. Environment variables (e.g. `ROKO_SERVE_AUTH_API_KEY`, `ANTHROPIC_API_KEY`)
4. CLI flags (highest precedence)

The strict validator (`config::validation::validate_strict_config_toml`) runs
at every shared-config load and rejects:

- `runner.dangerously_skip_permissions = true` in shared file
- Unknown root-level sections (when strict)
- Provider entries with conflicting `api_key` and `api_key_env`

Local-only overrides have a typed scope; see plan 23.

---

## Anti-Pattern Quick Reference

These are the cross-cutting failure modes that show up over and over. The full
catalog with examples is in `02-ANTI-PATTERNS.md`. The condensed list:

1. **Build another runtime / shadow runtime.** Anything that introduces a
   parallel dispatch / state machine / event bus is wrong. Improve the existing
   one.
2. **Inline prompt strings.** Use `SystemPromptBuilder`.
3. **Shell out / raw provider HTTP.** Use `ModelCallService`.
4. **Feedback as afterthought.** New code paths must emit `FeedbackEvent`s
   from day one, not later.
5. **Parse output strings as contracts.** Use typed events.
6. **God file accumulation.** New code goes in a focused module, not appended
   to `orchestrate.rs`.
7. **Hardcoded role/model/provider behavior.** Configurable, not branched.
8. **Transient or lossy state.** Persist or surface; do not silently drop.
9. **Copy between runtimes.** Extract a service or adapter.
10. **Features in the wrong layer.** Decide which crate owns the concept and
    keep it there.

---

## Where to Look for Examples

| Question | Look at |
|---|---|
| How do I add a new feedback sink? | `runtime_feedback/episodes.rs` (cleanest), `routing.rs` (good test pattern) |
| How do I add a serve route? | `routes/config.rs` (read/write), `routes/jobs.rs` (path validation, multiple files) |
| How do I add a CLI command? | `commands/plan.rs`, `commands/config_cmd.rs` |
| How do I add a gate rung? | `roko-gate` crate; `Rung` enum + `GateRegistry` |
| How do I add a provider adapter? | `roko-agent/src/providers/openai_compat.rs` is the cleanest example |
| How do I emit DashboardEvent? | `roko-runtime/src/state_hub.rs` |
| How do I write an ACP test? | `roko-acp/tests/telemetry_integration.rs` |
| How does the strict config validator look? | `roko-core/src/config/validation.rs` |

---

## Glossary

- **Rung**: a stage in the 7-rung gate pipeline (Compile, Test, Clippy, Symbol,
  Diff, PropertyTest, Integration). Rungs 3, 5, 6 are skipped today; T1-11
  unblocks construction; plan 29 builds them.
- **Episode**: a single task attempt's record (model, provider, gate outcomes,
  cost, duration). Persisted to `.roko/learn/episodes.jsonl`.
- **Knowledge candidate**: a learning entry produced from an episode that the
  neuro `KnowledgeStore` may admit. Persisted to `.roko/learn/knowledge-candidates.jsonl`.
- **Dispatch metadata**: model, provider, selection reason, complexity hint,
  budget pressure — the things `RoutingContext` needs (T4-30).
- **Confidence-only outcome**: a routing observation that updates aggregate
  success rate without any context features. The current state until T4-30.
- **Local override**: a typed `DangerousPermissionOverride` in
  `~/.config/roko/local.toml` with reason / scope / expiry / source /
  acknowledgement env. The only legal way to bypass safety today.
- **DispatchPlan**: a typed, validated request describing one model call
  (auth, capability, parameters). The migration target for all dispatch.

---

## Common "Gotchas"

- **Editing `orchestrate.rs`**: this file shadows itself. There are 2-3
  near-identical helpers in many areas. **Do not** copy a helper to make a
  new branch — extract or reuse. See plan 20.
- **Adding a `pub mod foo;` to `lib.rs`**: every new module must have at
  least one external caller within 1 PR or be feature-gated, or it will be
  flagged dead in the next learn-module audit.
- **Provider streaming events**: typed `ModelStreamEvent::Failed` exists.
  Use it. Returning a normal `Completed` for a failed stream silently
  corrupts learning data.
- **`unwrap()` on env vars**: provider env-var access in `roko-cli` and
  `roko-serve` is a constant audit hit. Use `var().ok()` and surface a
  typed error.
- **Workspace path operations in serve routes**: every other route group
  uses `validate_path_segment`. Don't be the one that doesn't.

---

## Where the Audits Live

- `tmp/subsystem-audits/05-01/` — newest, most actionable. Start here.
- `tmp/subsystem-audits/<subsystem>/{AUDIT,PLAN,GOALS,ISSUES}.md` — earlier
  per-subsystem audits. Some claims are stale; trust this folder's plans
  over those.
- `tmp/subsystem-audits/INDEX.md` — top-level summary of all audits.
- `tmp/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md` — the 100+ task
  prioritized list from 04-28 (predates 05-01 audit; cross-reference).

---

That's enough context. Pick a plan from `00-INDEX.md` and start.

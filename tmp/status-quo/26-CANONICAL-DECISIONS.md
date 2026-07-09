# Canonical Decisions Needed

**Date**: 2026-07-08 · HEAD `5852c93c05` on `main`.

This file lists decisions that should be made explicitly before deeper implementation continues. Several below are now **recommended-and-ratifiable** because the audit removed the ambiguity: the reality is confirmed, only the sign-off is pending. Evidence: [95-ENGINE-DRIFT.md](95-ENGINE-DRIFT.md), [92-RUNNER-V2-MODULE-FAMILY.md](92-RUNNER-V2-MODULE-FAMILY.md), [60-STATE-PERSISTENCE-LEDGER.md](60-STATE-PERSISTENCE-LEDGER.md).

## D1: Production Plan Engine — **RATIFY**

**Decision needed**: Is Runner v2 the production plan engine while Graph matures, or is Graph the production engine once TaskExecutorCell is live?

**Current reality**: CLI default is Graph (a dry-run stub), but Runner v2 is the live executor reached by every other surface (`do`/`serve`/`prd`/`worker` + `--engine runner-v2`). See [95](95-ENGINE-DRIFT.md).

**Canonical**: **Runner v2 is the production engine.** Graph is a target-shape opt-in until parity proof gates pass. Make Runner v2 the honest default and keep Graph behind `--engine graph` with an explicit "not yet live" warning until `TaskExecutorCell` dispatches real work.

## D2: Execution Result Contract

**Decision needed**: Which crate owns the canonical dispatch/run/gate/commit/routing result vocabulary?

**Current reality**: Types exist in fragments:

- `DispatchPlan`: `roko-core`
- `RunnerDispatchPlan`: `roko-cli`
- `RunLedger` and `CommitOutcome`: `roko-runtime`
- `GateStatus`: `roko-gate` and CLI inline UI
- `RoutingContext`: `roko-learn`
- provider resolver: `roko-cli`

**Recommended**: Put cross-runtime contracts in `roko-core` or a tiny `roko-primitives`-style contract crate; keep UI-only enums separate by name.

## D3: Signal vs Engram — **RATIFY: Engram**

**Decision needed**: Is the public noun `Signal` or `Engram`?

**Current reality**: There is **no `struct Signal`** — `Engram` is the only concrete noun; `Signal` survives only as a compatibility re-export. A second, dead `Engram` also exists in `roko-chain`. v2 docs still say Signal.

**Canonical**: **`Engram` is the canonical noun.** Label `Signal` deprecated/compat, delete the dead `roko-chain` copy, and update v2 docs. Avoid bidirectional aliases.

## D4: Event Source Of Truth

**Decision needed**: Is StateHub the canonical event/state surface, or a projection over a lower runtime bus?

**Current reality**: StateHub is canonical for server/TUI/dashboard snapshots, while runtime `EventBus`, server `EventBus`, learn `EventBus`, PulseBus, and JSONL logs coexist.

**Recommended**: Treat runtime EventBus/PulseBus as transport, StateHub as projection/cache, JSONL as durability. Write bridge loss policy and tests.

## D5: Episode Source Of Truth

**Decision needed**: Which path is canonical for episodes?

**Current reality**: `.roko/episodes.jsonl`, `.roko/learn/episodes.jsonl`, and `.roko/memory/episodes.jsonl` all exist/read in different code.

**Recommended**: Canonical writes to `.roko/episodes.jsonl`; learning keeps derived indices under `.roko/learn/`; `.roko/memory` is read-only migration fallback.

## D6: Gate Rung Semantics

**Decision needed**: Are rungs canonical numeric stages or service-local names?

**Current reality**: canonical `Rung` enum and `GateService::rung_for_name` use different stage meanings.

**Recommended**: Use one canonical `Rung` enum for all verification metrics; if GateService keeps diff/fmt/shell names, namespace them outside canonical rung metrics.

## D7: Chain Authority

**Decision needed**: Which path owns identity, reputation, jobs, and settlement?

**Current reality**: in-memory registries, Solidity contracts, local `.roko/jobs`, chain watcher, ISFR, and tool definitions overlap.

**Recommended**: Document local-only mode vs chain-backed mode and make conversion explicit. Do not imply local job JSON is the v2 on-chain marketplace.

## D8: Plugin Boundary

**Decision needed**: Are plugins declarative manifests, extension hooks, event sources, tool providers, or all of the above?

**Current reality**: all shapes exist partially, but manifest content is not product-lifecycle canonical.

**Recommended**: Start with declarative local plugins for prompts/tools/event sources, then add hooks and sandbox tiers after permission model proof.

## D9: Workspace Path Authority

**Decision needed**: Which API owns every `.roko` path?

**Current reality**: `Workspace`, `RokoLayout`, CLI helpers, serve helpers, Docker/deploy scripts, and app-specific binaries all derive paths directly. Hot paths disagree on episodes, signals/engrams, agent registries, Daimon/PAD state, and knowledge candidates.

**Recommended**: Add one `WorkspacePaths` facade over `RokoLayout`; let app-specific code request named paths from it, not join strings directly. Keep legacy readers behind migration helpers only.

## D10: Public API Contract Authority

**Decision needed**: Is the API contract generated from `roko-serve` route assembly, handwritten OpenAPI, or frontend DataHub assumptions?

**Current reality**: `roko-serve` has hundreds of mounted routes, partial OpenAPI, proxy/synthetic/in-memory routes, and known frontend mismatches.

**Recommended**: Generate a route manifest from route assembly, annotate each route as canonical/compat/private/proxy/synthetic, and make frontend route extraction fail CI on unowned paths.

## D11: Release Proof Scope

**Decision needed**: What must pass before Roko advertises migrated v2 behavior?

**Current reality**: CI covers core Rust fmt/clippy/test, but release proof misses deny, frontend, Foundry, deterministic runtime smoke, Docker health, feature matrix, and some workflow consistency checks.

**Recommended**: Treat migrated-v2 claims as release claims. Tie them to proof gates in `25`, parity tests in `64`, and CI/release gaps in `71`.

## D12: Canonical Snapshot File — **RATIFY: `state-snapshot.json`**

**Decision needed**: Which `.roko/state/` file is the source of truth for run/resume state?

**Current reality**: Runner v2 writes four overlapping generations (`executor.json`, `orchestrator.json`, `run-state.json`, `state-snapshot.json`); only `state-snapshot.json` (checksummed) is current on the live workspace, yet `roko-serve` still tries to **read** `state/executor.json` → error. See [60](60-STATE-PERSISTENCE-LEDGER.md), [92](92-RUNNER-V2-MODULE-FAMILY.md).

**Canonical**: **`state-snapshot.json` is canonical.** Point serve and any reader at it; keep the other three as compat writes behind a documented migration, and GC the `.bak.*` accumulation.

## D13: Signals vs Engrams Log — **must converge**

**Decision needed**: Which append log carries gate verdicts and run engrams, and what reads it?

**Current reality**: Gate verdicts are written to `signals.jsonl`, but dashboards read `engrams.jsonl` → empty panels. `events.jsonl` is a 44 MB / 97% `feed_tick` firehose. See [60](60-STATE-PERSISTENCE-LEDGER.md).

**Canonical**: Given D3 (Engram is the noun), converge onto **`engrams.jsonl`**; migrate `signals.jsonl` writers; trim the `feed_tick` firehose or move it to a separate low-value stream. Dashboards and writers must read/write the same file.

## D14: Default Security Posture — **deny-by-default**

**Decision needed**: Is the serve perimeter fail-open or fail-closed for unclassified mutating routes and un-nested routers?

**Current reality**: The relay proxy is merged **outside** `/api` (unauthed), and unlisted mutating `/api/*` routes fall through to a permissive `read` scope. ACP tool permissions and safety post-checks are advisory. See [75](75-SECURITY-AUTH-SCOPE-MATRIX.md).

**Canonical**: **Deny-by-default.** Every mutating route requires an explicit scope (no `read` fallback); every router is inside the auth stack unless explicitly public; ACP mutating tools call `request_permission`; SecretLeak/PathEscape post-checks `Block`.

## D15: One Prompt-Assembly Surface — **converge**

**Decision needed**: Which module assembles the live agent prompt — the CLI-side `PromptAssembler` or the canonical `SystemPromptBuilder`/12-slot/`RoleSystemPromptSpec`/VCG stack?

**Current reality**: Runner-v2 (the live engine) builds prompts with the CLI-side `PromptAssembler` (`dispatch/prompt_builder.rs:717`), **not** the 9-layer `SystemPromptBuilder` + VCG attention auction. The canonical builder and VCG run only on non-default paths and are reachable-but-cold (greedy dominates; `AttentionBidder` is compose-path only). There are **two `PromptAssembler`/prompt-build surfaces** ([103](103-DUPLICATE-TYPES-CENSUS.md) row 12, [102](102-SPEC-DEBT-LEDGER.md)). So "the 9-layer builder is the live prompt path" is false.

**Canonical**: **One prompt-assembly surface on the live path.** Either route runner-v2 through `SystemPromptBuilder`/`RoleSystemPromptSpec` (and warm VCG bidders so the auction is reachable), or declare the CLI `PromptAssembler` canonical and retire `roko-compose` template assembly to a documented compat layer. Docs must name the live surface.

## D16: Safety Must Cover All Providers — **provider-agnostic funnel**

**Decision needed**: Does per-tool safety (`ToolDispatcher`→`SafetyLayer` 9-policy pre-check) apply to every provider, or only to the OpenAI-compat `ToolLoop`?

**Current reality**: The safety/tool-dispatch funnel runs **only** for the roko-driven `ToolLoop` (OpenAI-compat + `supports_tools`). The **default Claude-CLI provider and Codex drive their own subprocess tool loop and never touch roko's `SafetyLayer` per tool call** ([99](99-TRACE-AGENT-TURN.md) §7). Tool safety on the default self-host path is delegated to Claude's own permission system with `--dangerously-skip-permissions:true`. Only 16 of 37 tool defs even have executable handlers.

**Canonical**: **Safety must be provider-agnostic on any path used for self-hosting.** Either route Claude/Codex tool calls through a roko-side pre-check, or ratify the [BYPASS] as an explicit, tested boundary where `build_settings_json` encodes the equivalent policy and an integration test proves the same denial on CLI + `ToolLoop`. A silent per-provider gap is not acceptable as a default.

## D17: Gate Adaptivity Must Move To The Live Path — **port enrichment into Runner v2**

**Decision needed**: Do adaptive thresholds, gate oracles 4-6, SPC/ratchet, and `VerdictPublisher` become live, or are they abandoned?

**Current reality**: All of that apparatus lives **only** on the dead `orchestrate.rs` `PlanRunner`. The live runner gate path (`gate_dispatch::run_gate_once`) builds the pipeline with `RungExecutionInputs::default()` and **never calls `enrich_rung_config`** ([101](101-TRACE-GATE-PIPELINE.md)). Live rungs 3-6 stub-pass `Verdict::pass`, per-rung EMA only ever updates rung 2, and `GateThresholds::save` has zero callers. Stub passes inflate the single rung-2 EMA toward 1.0.

**Canonical**: **Adaptivity moves onto the live path or is deleted, not left dual.** Port `enrich_rung_config`/`RungExecutionInputs` construction into `run_gate_once` so Symbol/FactCheck/LlmJudge/Integration receive real signals+oracles; make stubs report `Skipped`/`NotWired` (excluded from the EMA); label verdicts with the real inner rung; persist per-rung thresholds to `.roko/learn/gate-thresholds.json`. Then delete the dead `PlanRunner` gate methods.

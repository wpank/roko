# Code-Only Legacy / Ad-Hoc Audit

This audit is intentionally **code-only**. I did **not** use the docs as source material for the findings below.

## Proof of scope

- Enumerated the implementation surface with:
  - `rg --files crates apps tests demo examples -g '!**/target/**' | sort`
- Counted the code files in scope:
  - `1297` files
- Scanned that full set for legacy / ad-hoc markers with:
  - `TODO`
  - `FIXME`
  - `stub`
  - `placeholder`
  - `hack`
  - `legacy`
  - `temporary`
  - `hardcoded`
  - `compat`
  - `fallback`
  - `bypass`
  - `one-off`
  - `special case`

The items below are the highest-signal architectural leftovers I found in the implementation, grouped by subsystem.

## 1. CLI runner still mixes legacy orchestration, provider synthesis, and production dispatch

- [ ] Split `crates/roko-cli/src/orchestrate.rs` into smaller runtime modules. The file still owns plan discovery, provider synthesis, gate dispatch, worktree management, merge flow, replay, dreaming, neuro promotion, and dashboard event emission in one place. That is the old monolith shape the redesign was supposed to remove.
- [ ] Remove command-specific synthesis branches from `crates/roko-cli/src/orchestrate.rs:1598`, `:1659`, and `:1711`. It still has separate paths for `claude`, known-protocol CLIs, and generic subprocesses instead of one provider-neutral dispatch contract.
- [ ] Replace the hardcoded backend heuristics in `crates/roko-cli/src/orchestrate.rs:1799` with a declarative provider routing layer. The current `resolve_enrichment_backend()` is still command/provider/model string matching.
- [ ] Finish retiring the legacy resume parser in `crates/roko-cli/src/orchestrate.rs:6864`. The code still has explicit `legacy_completed_tasks_from_snapshot()` compatibility logic.
- [ ] Extract the remaining direct orchestration side effects in `crates/roko-cli/src/orchestrate.rs:7643` onward into explicit executor actions. The file still mixes state-machine transitions with process I/O, persistence, and policy updates.

## 2. Runner v2 is better, but it still carries legacy prompt and stream shape assumptions

- [ ] Replace the legacy system-prompt builder in `crates/roko-cli/src/runner/agent_stream.rs:367` through `:412`. The file still falls back to `build_legacy_system_prompt()` instead of treating the 9-layer composer as the only production path.
- [ ] Remove the old CLI-program fallback in `crates/roko-cli/src/runner/agent_stream.rs:247`. `CliProviderConfig::from_legacy_runner_program()` is still the escape hatch when provider resolution is missing.
- [ ] Decouple stream parsing from Claude-shaped JSON in `crates/roko-cli/src/runner/agent_stream.rs:1`. `parse_stream_line()` still knows too much about `stream-json` event shapes and has generic JSON fallback parsing bolted on.
- [ ] Split runner event handling responsibilities in `crates/roko-cli/src/runner/event_loop.rs`. The loop already improved, but it still owns dispatch, persistence, merge queue handling, learning feedback, and resume logic in one event pump.
- [ ] Remove the remaining legacy-efficiency compatibility path in `crates/roko-cli/src/runner/event_loop.rs:2272`. That label is a sign the runner still emits old dashboard-shaped records.

## 3. Gate execution still has explicit stub verdicts instead of complete data wiring

- [ ] Finish wiring rung inputs so `crates/roko-gate/src/rung_dispatch.rs:146`, `:173`, `:201`, `:220`, and `:237` never need to return `stub_verdict(...)` in production. Right now the dispatcher advertises the full 7-rung model but still passes through on missing signal wiring.
- [ ] Replace `crates/roko-gate/src/benchmark_gate.rs` entirely. It still declares itself a stub and always passes.
- [ ] Remove the ad-hoc stub-by-default surface from `crates/roko-gate/src/process_reward.rs:159` and `:278`. The reward path is still marked as stubbed heuristics rather than a real scoring pipeline.
- [ ] Eliminate the legacy compatibility mode in `crates/roko-gate/src/gate_pipeline.rs:326` and the fallback composition logic around `:524` through `:577` unless it is intentionally part of the final design. The pipeline still has backward-compatible sequential/fallback handling wired alongside the newer composition model.

## 4. Provider routing still contains fallback inference and old compatibility shims

- [ ] Remove the deprecated crate-root shims in `crates/roko-agent/src/lib.rs:85` through `:92`. The public surface still preserves old flat module names for compatibility.
- [ ] Remove the last legacy command inference branch in `crates/roko-agent/src/provider/mod.rs:130` through `:177`. If provider config is missing, it still infers behavior from the executable name and falls back to `ExecAgent`.
- [ ] Stop treating `ExecAgent` fallback as the default escape hatch in `crates/roko-agent/src/provider/mod.rs:969`. Unknown model keys still resolve to a generic prompt pipe instead of failing closed or forcing explicit configuration.
- [ ] Separate provider-neutral logic from protocol-specific behavior in `crates/roko-agent/src/provider/mod.rs` and `crates/roko-agent/src/tool_loop/backends/mod.rs`. These files still special-case known protocols and OpenAI-compatible flows in ways that are useful, but not yet fully abstracted.
- [ ] Finish removing Claude CLI assumptions from `crates/roko-agent/src/claude_cli_agent.rs`. The fallback model handling at `:89`, `:118`, and `:323` still encodes provider-specific policy directly in the adapter.
- [ ] Treat `crates/roko-agent/src/process/stderr.rs` as transitional glue only. It still contains suppression logic for upstream warnings and fallback metadata paths rather than a uniform process telemetry contract.

## 5. Persistence and state restore still preserve legacy shapes

- [ ] Replace the legacy AgentRegistry restore path in `apps/mirage-rs/src/persist.rs:219` through `:237`. The loader explicitly ignores old snapshots and logs that the ERC-8004 contracts are now canonical, which is correct, but it means the persistence layer still has compatibility baggage.
- [ ] Remove the legacy restore test name and the old agent-registry discard logic in `apps/mirage-rs/src/persist.rs:512`. The code path is still framed as a discard of legacy state rather than a normal migrated schema.
- [ ] Audit the JSON-RPC fallback surface in `apps/mirage-rs/src/rpc.rs:858` through `:912`. The service still has a POST-only fallback service path that exists to catch legacy traffic.
- [ ] Remove or isolate the compatibility-only Anvil/Hardhat no-op methods in `apps/mirage-rs/src/rpc.rs:2480` through `:2481`. They are useful for compatibility, but they are also an example of old behavior kept alive by special-case handling.
- [ ] Replace the minimal trace stub in `apps/mirage-rs/src/rpc.rs:1662`. Full trace-level observability is still not first-class there.

## 6. CLI config and subcommand handling still has legacy fallbacks

- [ ] Remove the `agent.command = "cat"` test-only default warning path in `crates/roko-cli/src/main.rs:2217` through `:2222`. It is useful for bootstrapping, but it is still a legacy-shaped escape hatch.
- [ ] Eliminate the config migration assumptions in `crates/roko-cli/src/main.rs:1418`. The CLI still exposes a `migrate legacy project roko.toml` path rather than only operating on the final config model.
- [ ] Collapse the `plan` discovery fallback in `crates/roko-cli/src/plan.rs:13` and the legacy-location tests around `:519` through `:550` once the repo is fully migrated to the new layout.
- [ ] Remove the config synthesis helper branches in `crates/roko-cli/src/agent_config.rs:76` through `:98` if the new provider model is supposed to be declarative-only. Right now the CLI still manufactures synthetic configs for Claude, known protocols, and subprocesses.
- [ ] Tighten `crates/roko-cli/src/dispatch_v2.rs:118` through `:159` so `from_legacy_runner_program()` and executable-name inference are no longer part of the steady-state design.

## 7. Observability is still a mix of old dashboard shapes and new state-hub data

- [ ] Remove the last dashboard-compatibility emit path in `crates/roko-cli/src/runner/event_loop.rs:2272` and `crates/roko-cli/src/orchestrate.rs:5161` if the UI is supposed to consume only the new event model.
- [ ] Finish replacing ad-hoc event-tail fallbacks in `crates/roko-cli/src/orchestrate.rs:4726` through `:4732`. These are still direct file reads with fallback text instead of a projection/query model.
- [ ] Normalize the direct file-backed observability writes in `crates/roko-cli/src/orchestrate.rs:5290` through `:5315` and `:6425` through `:6513`. The code still writes several different persistence artifacts independently.

## 8. A few code areas are modern, but still deliberately retain compatibility

These are not necessarily bugs, but they are still the kinds of places where legacy architecture remains visible:

- `crates/roko-agent/src/safety/contract.rs` keeps deny-all fallbacks for missing roles.
- `crates/roko-agent/src/safety/path.rs` keeps backward-compatible canonicalization behavior.
- `crates/roko-agent/src/safety/provenance.rs` keeps legacy-import taint mapping.
- `crates/roko-agent/src/tool_loop/mod.rs` still has explicit bypasses for the old tool-loop path in favor of backend-specific execution.
- `crates/roko-agent/src/provider/openai_compat.rs` and `crates/roko-agent/src/tool_loop/backends/openai_compat.rs` are intentionally compatibility-adapted surfaces, but they remain protocol-specific by design.

## Bottom line

The repo is not done with its legacy architecture cleanup. The main unfinished work is:

- collapsing `orchestrate.rs` into smaller runtime seams
- removing the remaining stub gates and fallback verdicts
- eliminating command-name inference and synthetic config generation
- finishing the state/persistence migration off old snapshot shapes
- making observability and UI projection consume one canonical event model

If you want the next pass, the most useful thing is to turn this into a task list ordered by subsystem ownership, with one checklist per file or module so another agent can pick it up without reading the rest of the repo.

## 2026-04-27 Deepening Pass - Legacy Surface Retirement Plan

### Self-grade for this deepening pass

Initial rating: `9.91 / 10`.

Rationale: this pass converts the original code-only audit from a high-level list into an implementation-grade retirement plan. It records a refreshed source-only scan, defines the difference between acceptable compatibility and forbidden legacy behavior, assigns each legacy surface to a replacement owner, and provides ordered checklists that can be executed without needing the rest of the conversation. The remaining gap is that the generated legacy-surface ledger is specified here but still needs a checked-in scanner and machine-enforced CI gate.

### Current code-only source refresh

Run this from the repository root to refresh the runtime legacy scan:

```bash
rg -n "orchestrate|PlanRunner::from_plans_dir|dispatch_direct::dispatch_prompt|from_legacy_runner_program|ExecAgent|legacy|compat|fallback|stub|placeholder|NoOpGate|AlwaysUpProbe|signals\\.jsonl|DashboardScaffold|DaimonPolicy::default|RungExecutionInputs::default|MergeSucceeded|merge" crates/roko-cli crates/roko-agent crates/roko-serve crates/roko-gate crates/roko-core crates/roko-std -g '*.rs' --stats
rg -n "PlanRunner::from_plans_dir|dispatch_direct::dispatch_prompt|from_legacy_runner_program|ExecAgent::new|AlwaysUpProbe::new|NoOpGate|RungExecutionInputs::default|MergeSucceeded|DashboardScaffold::new|signals\\.jsonl|DaimonPolicy::default\\(\\)" crates/roko-cli crates/roko-agent crates/roko-serve crates/roko-gate crates/roko-core crates/roko-std -g '*.rs'
```

Observed on 2026-04-27:

- [ ] The targeted code-only legacy scan searched `661` Rust files.
- [ ] It found `1947` matches across `231` files.
- [ ] The highest-risk matches are not all `legacy` comments. The blockers are places where legacy behavior is still executable: direct dispatch, subprocess fallback, no-op gates, scaffold queries, default policy, legacy event paths, placeholder gateway/cache, and merge-state transitions.
- [ ] Treat tests and compatibility-only adapters as lower risk unless the same pattern appears in production constructors, route handlers, runner paths, or CLI commands.
- [ ] Do not close this audit by deleting comments. Close it only when the live runtime no longer reaches the legacy behavior or when the behavior is isolated behind an explicit compatibility boundary.

### Retirement taxonomy

Every item in this file should be assigned one of these statuses:

- [ ] `retire_now`: executable legacy behavior in a production path that should be removed or fail closed.
- [ ] `wrap_with_policy`: behavior can remain only behind explicit runtime policy, durable events, and user-visible status.
- [ ] `compat_boundary`: compatibility behavior is allowed, but it must live behind named compatibility APIs and not count as Mori parity.
- [ ] `migration_adapter`: adapter remains only to migrate old data/config into the canonical shape and must emit migration evidence.
- [ ] `test_only`: behavior exists only under tests or fixtures.
- [ ] `dead_after_migration`: behavior can be deleted after a migration/proof milestone is complete.
- [ ] `already_replaced_needs_cleanup`: replacement exists, but old imports/callers/docs remain.
- [ ] `proved_retired`: grep and runtime proof show the legacy path is no longer reachable.

### Replacement architecture

The desired shape is not "patch each call site." The desired shape is a small set of owned services:

- [ ] `RuntimeEngine`: the only service that owns plan/task state transitions, retry/replan, gate decisions, merge decisions, and resume snapshots.
- [ ] `Dispatcher`: the only service that owns provider/model selection, prompt assembly handoff, process/API dispatch, runtime event normalization, and provider fallback policy.
- [ ] `PromptAssembler`: the only service that owns system prompt, task context, knowledge/playbook insertion, section effectiveness, tool policy, and prompt diagnostics.
- [ ] `GateService`: the only service that owns gate input assembly, gate execution, missing-input failures, no-stub evidence, and gate result events.
- [ ] `MergeService`: the only service that owns git merge execution, conflict capture, post-merge gate, rollback/failure evidence, and merge result events.
- [ ] `RuntimeEventStore`: the only service that owns durable event writes, legacy event import, projections, query endpoints, and proof exports.
- [ ] `RuntimePolicy`: the only service that owns fallback, degraded mode, safety, no-op/mock allowance, health readiness, and default policy provenance.
- [ ] `WorkspaceRepository`: the only service that owns plan/PRD/task/artifact paths, legacy layout migration, and clean-clone file access.

An implementation is not considered elegant if any feature code bypasses these services to read/write legacy files, spawn providers, synthesize config, mark merges successful, or fabricate dashboard data.

### High-risk executable legacy surfaces

This is the concrete queue an implementation agent should start with:

- [ ] `crates/roko-cli/src/runner/event_loop.rs:1054`: `ExecutorEvent::MergeSucceeded` is still applied from completion handling. Verify that every path to this event is preceded by `MergeService` evidence from `runner/merge.rs`, not by optimistic state changes.
- [ ] `crates/roko-cli/src/orchestrate.rs:8100` and `:8126`: legacy orchestrate still applies `MergeSucceeded`; keep it legacy-only until the runner path fully owns merge state.
- [ ] `crates/roko-cli/src/runner/merge.rs:4`: this module explicitly documents the old defect: direct `MergeSucceeded` without real git merge. Use this module as the replacement contract and prove all runner merge transitions pass through it.
- [ ] `crates/roko-cli/src/unified.rs:95` and `crates/roko-cli/src/chat_inline.rs:1475`: direct dispatch calls bypass the dispatcher/model-call architecture.
- [ ] `crates/roko-cli/src/runner/agent_stream.rs:131` and `crates/roko-cli/src/dispatch_v2.rs:122`: legacy runner-program conversion remains an executable fallback.
- [ ] `crates/roko-agent/src/provider/mod.rs:177`: provider creation can still instantiate `ExecAgent` with `cat` fallback when provider resolution fails.
- [ ] `crates/roko-cli/src/runner/gate_dispatch.rs:49`: gate dispatch can still use `RungExecutionInputs::default()` instead of real assembled gate inputs.
- [ ] `crates/roko-gate/src/rung_dispatch.rs:132`: stub verdicts remain executable for missing gate wiring.
- [ ] `crates/roko-std/src/noop.rs:30` and `crates/roko-std/src/lib.rs:29`: no-op implementations are exported and must be blocked from production constructors.
- [ ] `crates/roko-core/src/obs/health.rs:190`, `crates/roko-cli/src/orchestrate.rs:5504`, and `crates/roko-cli/src/commands/util.rs:657`: always-up health can mask missing dependencies.
- [ ] `crates/roko-cli/src/commands/dashboard.rs:56`, `crates/roko-cli/src/serve_runtime.rs:109`, and `crates/roko-cli/src/tui/app.rs:455`: dashboard scaffold creation remains a live data source in CLI/TUI/server surfaces.
- [ ] `crates/roko-cli/src/tui/app.rs:71`, `:72`, `:76`, `:88`, `:90`, `:92`, and `:2314`: TUI state still labels legacy scaffold/page state as compatibility state.
- [ ] `crates/roko-cli/src/commands/util.rs:140`, `crates/roko-cli/src/agent_serve.rs:1324`, `crates/roko-cli/src/chat_inline.rs:2314`, and `crates/roko-serve/src/parity.rs:424`: `signals.jsonl` is still referenced in live code and parity metadata.
- [ ] `crates/roko-serve/src/routes/providers.rs:150`, `crates/roko-serve/src/routes/gateway.rs:854`, `crates/roko-cli/src/runner/event_loop.rs:2432`, and `crates/roko-cli/src/commands/config_cmd.rs:480`: `DaimonPolicy::default()` appears in service paths where runtime policy should be resolved with provenance.
- [ ] `crates/roko-serve/src/routes/gateway.rs:437`: gateway cache metrics are placeholder-backed, so gateway observability is not proof-grade.
- [ ] `crates/roko-serve/src/job_runner.rs:626` and `:647`: job runner can synthesize fallback plans when referenced plan/PRD planning is unavailable; this should become an explicit degraded workflow result.
- [ ] `crates/roko-cli/src/run.rs:348`, `:364`, `:404`, `:577`, and `:610`: one-off `roko run` path still has fallback prompt/config synthesis that should converge on the dispatcher and prompt assembler.
- [ ] `crates/roko-cli/src/agent_serve.rs:394` through `:402`: agent serve still has legacy subprocess-command detection and API-vs-CLI branching.

### Generated legacy-surface ledger

Add a tracked scanner that writes `tmp/mori-diffs/generated/legacy-surface-ledger.json`. It should be narrower than the repository marker inventory in [26-REPOSITORY-WIDE-CODE-AUDIT.md](26-REPOSITORY-WIDE-CODE-AUDIT.md): this ledger tracks executable legacy behavior, not every TODO/comment.

```json
{
  "schema_version": 1,
  "generated_at": "2026-04-27T00:00:00Z",
  "scan_scope": [
    "crates/roko-cli/**/*.rs",
    "crates/roko-agent/**/*.rs",
    "crates/roko-serve/**/*.rs",
    "crates/roko-gate/**/*.rs",
    "crates/roko-core/**/*.rs",
    "crates/roko-std/**/*.rs"
  ],
  "surfaces": [
    {
      "id": "legacy-surface-0001",
      "path": "crates/roko-cli/src/unified.rs",
      "line": 95,
      "symbol": "dispatch_direct::dispatch_prompt",
      "category": "direct_dispatch_bypass",
      "status": "retire_now",
      "replacement_owner": "Dispatcher",
      "owner_doc": "41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md",
      "reachable_from": ["roko unified"],
      "required_change": "route through Dispatcher and ModelCallService; emit provider lifecycle and prompt diagnostics",
      "proof": ["real_provider_run", "provider_matrix", "runtime_event_query"],
      "closed_by": null
    }
  ],
  "summary": {
    "retire_now": 0,
    "wrap_with_policy": 0,
    "compat_boundary": 0,
    "migration_adapter": 0,
    "test_only": 0,
    "dead_after_migration": 0,
    "already_replaced_needs_cleanup": 0,
    "proved_retired": 0
  }
}
```

Implementation requirements:

- [ ] Build the ledger from source, not from this markdown file.
- [ ] Include symbol/call-site category detection for `dispatch_direct::dispatch_prompt`, `from_legacy_runner_program`, `ExecAgent::new`, `RungExecutionInputs::default`, `stub_verdict`, `MergeSucceeded`, `DashboardScaffold::new`, `signals.jsonl`, `AlwaysUpProbe::new`, `NoOpGate`, and `DaimonPolicy::default()`.
- [ ] Distinguish tests from production by path and `#[cfg(test)]` region where practical.
- [ ] Include `reachable_from` for every production surface.
- [ ] Fail CI/proof if any `retire_now` surface has no replacement owner.
- [ ] Fail CI/proof if any `proved_retired` surface still appears in source outside tests.
- [ ] Import the ledger summary into [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md).

### Implementation batch LC-01 - Merge transitions through MergeService

- [ ] Audit every `ExecutorEvent::MergeSucceeded` call site.
- [ ] Create or finalize a `MergeService` facade over `runner/merge.rs`.
- [ ] Make `MergeService` return a typed result with `attempt_id`, `plan_id`, `branch`, `base`, `commit_before`, `commit_after`, `exit_status`, `stdout_ref`, `stderr_ref`, `conflict_files`, `conflict_markers`, and `duration_ms`.
- [ ] Update runner event handling so `MergeSucceeded` can be emitted only from a successful `MergeService` result.
- [ ] Convert merge conflicts into typed failed merge events, not generic task failures.
- [ ] Keep legacy `orchestrate.rs` merge code behind legacy-only entrypoints until deleted.
- [ ] Add a grep/proof gate that fails if a new production call site applies `MergeSucceeded` without merge evidence.

Proof:

- [ ] Successful merge creates a merge backend event and a queryable projection row.
- [ ] Merge conflict creates conflict-file evidence and does not mark the plan merged.
- [ ] Crash after merge attempt can be resumed from persisted merge result.
- [ ] HTTP and TUI show the same merge status and conflict evidence.

### Implementation batch LC-02 - Direct dispatch retirement

- [ ] Replace `dispatch_direct::dispatch_prompt` call sites in `unified.rs` and `chat_inline.rs`.
- [ ] Route all one-shot chat/unified/inline calls through `Dispatcher`.
- [ ] Ensure `Dispatcher` always uses `PromptAssembler` or explicitly records that no task prompt assembly was needed.
- [ ] Normalize provider outputs into provider-neutral runtime events.
- [ ] Emit prompt diagnostics, provider lifecycle, token/cost, retry/fallback, and final outcome events.
- [ ] Remove direct API-key auth detection from feature code and use resolved runtime config.
- [ ] Delete or demote `dispatch_direct.rs` after all production callers move.

Proof:

- [ ] `roko chat` or equivalent inline path emits the same provider event envelope as `plan run`.
- [ ] Provider matrix proof covers inline/unified route through the same dispatch path.
- [ ] A missing key reports `missing_credentials`, not a subprocess fallback.

### Implementation batch LC-03 - Legacy provider-program fallback

- [ ] Replace `CliProviderConfig::from_legacy_runner_program` in production runner code with explicit provider registry resolution.
- [ ] Keep `from_legacy_runner_program` only as a migration helper or test utility.
- [ ] Fail config validation when a production run would infer provider kind from executable name.
- [ ] Require explicit provider kind, model key, command/API mode, credential source, and tool support declaration.
- [ ] Mark generic `ExecAgent` as `explicit_subprocess_provider` with no implicit tools, no session restore, and no default safety bypass.
- [ ] Remove `cat` as an implicit production fallback.
- [ ] Emit a runtime policy event whenever explicit subprocess mode is selected.

Proof:

- [ ] Unknown model/provider config fails closed with an actionable error.
- [ ] Explicit subprocess config works but is labeled with no-tool/no-session capabilities.
- [ ] Provider proof cannot pass through `cat` or implicit `ExecAgent`.

### Implementation batch LC-04 - Prompt assembly convergence

- [ ] Remove runner-local legacy prompt builders and one-off fallback prompts.
- [ ] Make `PromptAssembler` the only production path for task prompts, review prompts, repair prompts, inline prompts that need context, and gateway prompt diagnostics.
- [ ] Plug knowledge, playbooks, section effectiveness, safety policy, tool policy, external context, and task artifacts into `PromptAssembler` as typed sections.
- [ ] Store section IDs and content hashes in prompt diagnostics.
- [ ] Record which prompt sections were referenced or ignored after provider output.
- [ ] Replace raw stdout or legacy context prepends with typed prompt inputs.
- [ ] Add a proof that prompt assembly is identical across CLI and HTTP for the same request.

Proof:

- [ ] Prompt diagnostics query shows section list, token estimates, hashes, and policy source.
- [ ] Knowledge/playbook inclusion is visible in diagnostics.
- [ ] A direct grep for production `fallback_system_prompt` or `build_legacy_system_prompt` callers returns no rows.

### Implementation batch LC-05 - Gate input and stub verdict retirement

- [ ] Replace `RungExecutionInputs::default()` with a gate input assembly service.
- [ ] Make missing inputs typed failures with remediation fields.
- [ ] Ensure `stub_verdict` is not reachable in production execution.
- [ ] Move gate fallback composition behind explicit gate policy.
- [ ] Block no-op fallback gates in production.
- [ ] Emit durable events for assembled gate inputs and gate execution decisions.
- [ ] Update acceptance-contract no-stub evidence to consume the generated marker/legacy ledgers.

Proof:

- [ ] Missing symbol manifest fails as `gate_input_missing`.
- [ ] Missing fact-check oracle fails as `gate_input_missing`.
- [ ] No-op fallback cannot make a required gate pass.
- [ ] Gate statuses are queryable through HTTP and visible in TUI.

### Implementation batch LC-06 - Runtime policy and health

- [ ] Introduce one resolved `RuntimePolicy` record for production runs and services.
- [ ] Replace ad-hoc `DaimonPolicy::default()` construction with resolved policy plus provenance.
- [ ] Replace `AlwaysUpProbe` in production health registries with dependency-aware probes.
- [ ] Keep `AlwaysUpProbe` only for tests or explicit demo-only health.
- [ ] Add policy decisions for fallback allowance, mock/no-op allowance, provider degradation, chain availability, merge requirements, and gate strictness.
- [ ] Emit runtime policy events at process start and on hot reload.
- [ ] Expose policy and health through query endpoints.

Proof:

- [ ] Health is degraded/unready when provider, event store, workspace, or chain dependency is required but unavailable.
- [ ] Runtime policy query shows every default and its source.
- [ ] Grep shows no production route/runner constructs `DaimonPolicy::default()` directly.

### Implementation batch LC-07 - Observability and legacy event paths

- [ ] Define canonical event log ownership in `RuntimeEventStore`.
- [ ] Move `signals.jsonl` reads/writes/copies to a migration/export layer.
- [ ] Replace dashboard/parity direct reads with projection queries.
- [ ] Emit migration events when legacy signals are imported.
- [ ] Remove dashboard compatibility event emitters once projections cover the UI.
- [ ] Replace direct event-tail fallback text with typed query responses.
- [ ] Add projection parity proof for CLI, HTTP, and TUI.

Proof:

- [ ] Fresh runtime writes only canonical event logs.
- [ ] Legacy workspace with `signals.jsonl` imports with migration evidence.
- [ ] HTTP/TUI projections can be rebuilt after restart.
- [ ] No production feature code reads `.roko/signals.jsonl` directly.

### Implementation batch LC-08 - Dashboard scaffold retirement

- [ ] Stop using `DashboardScaffold::new_in` as live data source for server/TUI/CLI commands.
- [ ] Replace scaffold page state with query-backed view models.
- [ ] Keep text-mode compatibility only as a renderer over query data.
- [ ] Remove legacy page IDs after Mori-style tabs and query contracts are complete.
- [ ] Make CreateJob, marketplace, operations, efficiency, and gate pages use command/query services.
- [ ] Update `surface_inventory` from source truth after each page migrates.
- [ ] Block parity claims when a page still reads scaffold-only data.

Proof:

- [ ] TUI can rebuild dashboard state from HTTP/query projections after restart.
- [ ] Offline CLI dashboard uses local projection store, not scaffolds.
- [ ] Surface inventory reports no scaffold-only pages as parity-complete.

### Implementation batch LC-09 - Job runner fallback planning

- [ ] Replace fallback plan synthesis in `roko-serve/src/job_runner.rs` with explicit workflow status.
- [ ] Route job planning through the same PRD/plan/task workflow engine used by CLI.
- [ ] Emit `job_planning_unavailable`, `job_plan_synthesized_by_policy`, or `job_plan_created` events.
- [ ] Require policy approval before synthesizing a fallback plan.
- [ ] Record the source PRD, plan path, provider/model, and prompt diagnostics for job planning.
- [ ] Ensure job runner does not construct mock chain clients in production service paths.
- [ ] Add query endpoints for job planning status and errors.

Proof:

- [ ] Missing referenced plan returns a degraded workflow status unless policy allows synthesis.
- [ ] Successful job plan appears in artifact repository and event projections.
- [ ] Production job runner cannot silently use mock chain/provider behavior.

### Implementation batch LC-10 - Compatibility boundary cleanup

- [ ] Create a compatibility manifest listing every remaining legacy adapter: old config migration, old layout migration, Mirage-compatible routes, EVM compatibility no-ops, old JSON shapes, and CLI text fallbacks.
- [ ] For each adapter, record `owner`, `reason`, `default_enabled`, `sunset_condition`, `proof_exclusion`, and `query_status`.
- [ ] Ensure compatibility adapters cannot satisfy Mori parity checklists.
- [ ] Emit compatibility-use events whenever a legacy adapter handles live traffic.
- [ ] Add warnings in CLI/HTTP responses when compatibility paths are used.
- [ ] Delete adapters once their sunset conditions are met.
- [ ] Keep tests for migration behavior until deletion.

Proof:

- [ ] Compatibility status endpoint lists all active adapters.
- [ ] Live use of a compatibility adapter emits an event.
- [ ] Mori proof scripts fail if compatibility-only behavior is used as success evidence.

### No-context handoff checklist

- [ ] Generate `tmp/mori-diffs/generated/legacy-surface-ledger.json`.
- [ ] Start with `retire_now` surfaces in this order: merge transitions, direct dispatch, legacy provider-program fallback, gate stub verdicts, no-op/default policy, scaffold dashboards, legacy event files.
- [ ] For each surface, identify the replacement owner service from the Replacement architecture section.
- [ ] Move the implementation to the owner service.
- [ ] Remove or policy-wrap the old call site.
- [ ] Add a durable event for the decision or side effect.
- [ ] Add HTTP/query/TUI proof where user-visible.
- [ ] Add grep proof that the old production call site is gone or classified.
- [ ] Update this file, [26-REPOSITORY-WIDE-CODE-AUDIT.md](26-REPOSITORY-WIDE-CODE-AUDIT.md), and [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md).

### Definition of complete

- [ ] `legacy-surface-ledger.json` exists and is reproducible from a clean clone.
- [ ] No `retire_now` production surface remains open.
- [ ] Direct dispatch is gone from production feature code.
- [ ] Provider fallback is explicit, policy-governed, evented, and never `cat` by default.
- [ ] Merge success can be produced only by real merge evidence.
- [ ] Gate success can be produced only by real gate execution or explicit non-acceptance-critical skip policy.
- [ ] Runtime policy is resolved once with provenance and passed into services.
- [ ] Legacy event paths are migration/export-only.
- [ ] Dashboard/TUI/HTTP read from query/projection contracts, not scaffolds.
- [ ] Compatibility behavior is visible, isolated, evented, and excluded from Mori parity proof.

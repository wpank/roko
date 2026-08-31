# Backlog — Master Implementation Index

> **What is this?** Self-contained backlog items for the roko workspace. Each doc
> specifies a problem, what already exists, what needs building, where to put it,
> and how to verify it.
>
> **How to use:** Pick an item, read the spec, implement it. Update `.roko/GAPS.md` when done.
>
> Last reviewed: 2026-08-31 (highest allocated ID: 286; filesystem location/status is authoritative)

---

## Start Here

**Single source of truth.** This index covers the general backlog. `.roko/GAPS.md` tracks
epic-level programme status and `plans/INDEX.md` tracks machine-executed plans. For engine
convergence specifically, the dependency order, readiness gates, and progress checklist in
[`tmp/engine-audit/IMPLEMENTATION-ROADMAP.md`](../engine-audit/IMPLEMENTATION-ROADMAP.md)
override the older generic waves below. Every engine worker must also follow the
[`MECHANICAL-EXECUTION-GUIDE.md`](../engine-audit/MECHANICAL-EXECUTION-GUIDE.md), and the
coordinator must record dispatch/leases/evidence in [`RUN-LEDGER.md`](../engine-audit/RUN-LEDGER.md).
Packets are frozen work orders: return `SPEC_DRIFT` instead of inventing a different contract.
Do not schedule `archive/` entries.

### Engine Convergence Program

The engine audit has been converted into self-contained, mechanical work packets optimized for parallel execution and one shared Cargo build lane.
The immediate ready queue is #47, #75, #208, #271, and #273; after those, follow the roadmap DAG.

| Program segment | Backlog work packets |
|---|---|
| Baseline and contracts | [#47](47-configlayer-elimination.md), [#75](75-graph-example-schema-drift.md), [#208](208-unified-event-schema.md), [#242](242-engine-convergence-contract.md), [#271](271-graph-engine-edge-validation.md), [#273](273-replan-mutation-contract.md) |
| Shared services and events | [#233](233-tui-runner-command-channel.md), [#243](243-runtime-services-profiles-builder.md)-[#248](248-graph-dashboard-tui-adapter.md) |
| Resource and lifecycle parity | [#138](138-crash-resume-proof-matrix.md), [#249](249-graph-worktree-attempt-lifecycle.md)-[#255](255-graph-approval-control-cancellation.md), [#274](274-graph-task-dispatch-host-integration.md), [#275](275-runner-shared-gate-adapter.md), [#282](282-lifecycle-checkpoint-completeness.md), [#284](284-topology-restore-killpoint-gate.md) |
| Production topology and cutover | [#256](256-production-plan-graph-topology.md)-[#261](261-runner-retirement-orchestrator-prune.md), [#276](276-workflow-engine-retirement.md)-[#281](281-deferred-graph-spec-ledger.md), [#283](283-direct-server-runtime-migration.md) |
| CLI/docs and additive surfaces | [#43](43-clippy-suppression-removal.md), [#262](262-cli-routing-flag-contract.md)-[#267](267-authored-graph-production-profile.md), [#285](285-precutover-graph-host-lint-gate.md) |
| Post-cutover Cell convergence | [#268](268-canonical-cell-contract-modernization.md)-[#270](270-production-cognitive-hot-graph.md) |
| Multi-run throughput | [#272](272-parallel-plan-queues.md) after graph resource/snapshot/control contracts |

**Highest-impact open items** (for engine work, follow the DAG above):

| # | Item | Size | Why |
|---|---|---|---|
| 232 | [TUI connected-mode data bridge](232-tui-connected-mode-data-bridge.md) | M | Token sparkline/efficiency/learning show zeros in plan run |
| 233 | [Executor-neutral TUI command channel](233-tui-runner-command-channel.md) | M | Recovery actions need one acknowledged control path for either executor |
| 75 | [Graph example schema regression](75-graph-example-schema-drift.md) | XS | The graph crate's integration baseline is currently red |
| 242 | [Engine convergence contract](242-engine-convergence-contract.md) | L | Freezes parity and side-effect ownership before parallel implementation |
| 178 | [Conductor supervisor loop](178-conductor-supervisor-loop.md) | M | Stalled agents burn tokens indefinitely without a safety net |
| 204 | [Review cap force-commit](204-review-cap-force-commit.md) | S | Prevents infinite review loops consuming unlimited tokens |
| 203 | [Error pattern sharing](203-error-pattern-sharing.md) | S | Parallel agents re-discover the same errors without this |
| 15 | [Post-Gate Reflection](15-post-gate-reflection.md) | M | Agents retry blind without failure analysis |
| 138 | [Executor-neutral crash/resume proof matrix](138-crash-resume-proof-matrix.md) | M | Default cutover requires proof that resume never duplicates side effects |
| 229 | [Backlog and plan state reconciliation](229-backlog-plan-state-reconciliation.md) | M | Prevents completed work from being silently re-dispatched |
| 90 | [UX34 Override Learning Isolation](90-ux34-override-learning-isolation.md) | M | Manual provider overrides corrupt the cascade router's learning signal |

**Implementation order:** Waves 0→7 below. Within each wave, do XS items first (quick wins),
then S, then M/L. Wave 3 is the current frontier — Waves 0-2 are mostly done.

---

## Implementation Order

Items are grouped into waves by dependency and impact. Complete earlier waves first; later
waves contain items that either depend on earlier ones or are lower risk to defer.

### Wave 0: Critical Fixes (do first — crashes, data loss, security)

These are correctness failures that silently produce wrong output or crash the system.

| # | Title | Size |
|---|---|---|
| 75 | [Graph Example Schema Drift Regression](75-graph-example-schema-drift.md) | XS | **Reopened** (2026-08-31; current integration test fails) |
| ~~78~~ | ~~[Efficiency gate_passed Bug](78-efficiency-gate-passed-bug.md)~~ | ~~S~~ | **Done** (2026-08-25) |
| ~~86~~ | ~~[Gate Compile Tool Bypass](archive/86-gate-compile-tool-bypass.md)~~ | ~~XS~~ | **Done** (2026-08-24) |
| ~~87~~ | ~~[Task Parser Duplicate IDs](archive/87-task-parser-duplicate-ids.md)~~ | ~~S~~ | **Done** (2026-08-24) |
| 17 | [ACP Stability Hardening](17-acp-stability-hardening.md) | L |

### Wave 1: Safety & Security Hardening

Security posture items that should be resolved before any public-facing deployment.

| # | Title | Size |
|---|---|---|
| ~~48~~ | ~~[Serve Auth Default Posture](archive/48-serve-auth-default.md)~~ | ~~S~~ | **Done** (2026-08-24) |
| ~~49~~ | ~~[Serve CORS Restrictive](49-serve-cors-restrictive.md)~~ | ~~S~~ | **Done** (2026-08-25) |
| ~~50~~ | ~~[Serve Rate and Body Limits](50-serve-rate-body-limits.md)~~ | ~~S~~ | **Done** (2026-08-25) |
| ~~51~~ | ~~[Serve Agent Name Validation](51-serve-path-traversal.md)~~ | ~~S~~ | **Done** (2026-08-25) |
| ~~96~~ | ~~[API Key Timing-Safe Comparison](archive/96-serve-api-key-timing-safe.md)~~ | ~~XS~~ | **Done** (2026-08-24) |
| 60 | [Safety Dispatch Hardening](60-safety-dispatch-hardening.md) | M |
| ~~33~~ | ~~[CLI Gist Scrubbing](33-cli-gist-scrubbing.md)~~ | ~~S~~ | **Done** (2026-08-25) |
| ~~133~~ | ~~[dangerously_skip_permissions opt-in](133-dangerously-skip-permissions-opt-in.md)~~ | ~~XS~~ | **Done** (2026-08-25) |
| ~~136~~ | ~~[Safety denial audit events](136-safety-denial-audit-events.md)~~ | ~~S~~ | **Done** (2026-08-28) |

### Wave 2: Core Runtime Correctness

P1 items fixing wrong behavior in the primary execution path.

| # | Title | Size |
|---|---|---|
| ~~88~~ | ~~[Handler Future Lifecycle Panic](archive/88-handler-future-lifecycle-panic.md)~~ | ~~XS~~ | **Done** (2026-08-24) |
| ~~89~~ | ~~[Rate Limiter Mutex Poisoning](archive/89-rate-limiter-mutex-poisoning.md)~~ | ~~XS~~ | **Done** (2026-08-24) |
| 90 | [UX34 Override Learning Isolation](90-ux34-override-learning-isolation.md) | M |
| ~~95~~ | ~~[Config Loader Robustness](95-config-loader-robustness.md)~~ | ~~S~~ | **Done** (2026-08-28) |
| ~~97~~ | ~~[Snapshot Backup & Staging Cleanup](97-snapshot-backup-and-staging-cleanup.md)~~ | ~~S~~ | **Done** (2026-08-26) |
| 101 | [Async Runtime Anti-Patterns](101-async-runtime-anti-patterns.md) | M |
| ~~131~~ | ~~[PRD/cloud-worker migration to runner-v2](131-prd-cloud-worker-runner-v2-migration.md)~~ | ~~S~~ | **Done** (2026-08-31) |
| ~~132~~ | ~~[orchestrate.rs freeze/retirement](132-orchestrate-rs-freeze-retirement.md)~~ | ~~M~~ | **Done** (confirmed 2026-08-29; file removed) |
| ~~134~~ | ~~[Runner-v2 gate-failure replan](archive/134-replan-on-gate-failure-runner-v2.md)~~ | ~~M~~ | **Superseded by #252** |
| ~~135~~ | ~~[Runner-v2 adaptive gate thresholds](archive/135-adaptive-gate-thresholds-runner-v2.md)~~ | ~~S~~ | **Superseded by #250/#253** |
| ~~137~~ | ~~[Router + threshold state in crash snapshot](archive/137-router-threshold-state-crash-snapshot.md)~~ | ~~XS~~ | **Superseded by #251** |
| ~~81~~ | ~~[Layer-Check False Positives](81-layer-check-false-positives.md)~~ | ~~S~~ | **Done** (2026-08-26) |
| 138 | [Executor-neutral crash/resume proof matrix](138-crash-resume-proof-matrix.md) | M |

### Wave 3: Execution UX Foundation

**TUI Parity:** Items 232-234 address the 3 root causes making the TUI feel static during plan runs. See `tmp/tui-parity/` for full audit.

Items that make self-hosting reliable and observable for the first time.

| # | Title | Size |
|---|---|---|
| ~~159~~ | ~~[Plan Run Report Phase Latency](archive/159-plan-run-report-phase-latency.md)~~ | ~~S~~ | **Done** (2026-08-24) |
| ~~160~~ | ~~[--fresh Worktree/Branch Cleanup](archive/160-fresh-worktree-branch-cleanup.md)~~ | ~~XS~~ | **Done** (2026-08-24) |
| ~~163~~ | ~~[Shared Build Cache for Worktrees](archive/163-worktree-shared-build-cache.md)~~ | ~~S~~ | **Done** (2026-08-22) |
| ~~164~~ | ~~[SchedulerNoProgress Timeout Too Short](archive/164-scheduler-progress-timeout-too-short.md)~~ | ~~XS~~ | **Done** (2026-08-22) |
| 166 | [Gate verify pre-existing failure filtering](166-gate-verify-preexisting-filter.md) | S |
| 170 | [Adaptive task-verify scoping](170-adaptive-verify-scoping.md) | S |
| ~~286~~ | ~~[FAST hard-deadline interposition](286-fast-hard-deadline-interposition.md)~~ | ~~M~~ | **Source complete; final kill-point matrix pending** (2026-08-31) |
| ~~107~~ | ~~[Plan Run UX Friction](107-plan-run-ux-friction.md)~~ | ~~M~~ | **Done** (2026-08-31) |
| ~~108~~ | ~~[TUI Live Feedback Gaps](108-tui-live-feedback-gaps.md)~~ | ~~L~~ | **Done** (2026-08-31; 5/6 sub-items wired) |
| ~~119~~ | ~~[TUI recovery keybindings](archive/119-tui-recovery-keybindings.md)~~ | ~~M~~ | **Superseded by #233/#255** |
| ~~120~~ | ~~[Plan run preflight checks](120-plan-run-preflight-checks.md)~~ | ~~S~~ | **Done** (2026-08-31) |
| 03 | [Context Injection Scoping](03-context-injection-scoping.md) | M |
| 04 | [Compile Auto-Fix Path](04-compile-autofix-path.md) | S |
| ~~77~~ | ~~[CLI UX Consistency](77-cli-ux-consistency.md)~~ | ~~S~~ | **Done** (2026-08-31) |
| ~~79~~ | ~~[Doctor/Onboarding Diagnostics](79-doctor-onboarding-diagnostics.md)~~ | ~~M~~ | **Done** (2026-08-31) |
| 93 | [Plugin Runtime Verification](93-plugin-runtime-verification.md) | S |
| 111 | [Screenshot command completion](111-screenshot-command-completion.md) | S |
| ~~113~~ | ~~[CLI JSON output mode](113-cli-json-output-mode.md)~~ | ~~M~~ | **Done** (2026-08-31) |
| ~~114~~ | ~~[Diagnose command enrichment](114-diagnose-command-enrich.md)~~ | ~~XS~~ | **Done** (2026-08-27) |
| ~~115~~ | ~~[Structured log wire verification](115-structured-log-wire-verification.md)~~ | ~~XS~~ | **Done** (2026-08-28) |
| ~~146~~ | ~~[Plan CLI control commands](146-plan-cli-control-commands.md)~~ | ~~M~~ | **Done** (2026-08-31) |
| 178 | [Conductor supervisor loop](178-conductor-supervisor-loop.md) | M |
| ~~182~~ | ~~[Lightweight status file](182-lightweight-status-file.md)~~ | ~~XS~~ | **Done** (2026-08-31) |
| ~~206~~ | ~~[CARGO_BUILD_JOBS limit](206-cargo-build-jobs-limit.md)~~ | ~~XS~~ | **Done** (2026-08-31) |
| ~~212~~ | ~~[run_id in snapshots and events](212-run-id-snapshot-events.md)~~ | ~~XS~~ | **Done** (2026-08-31) |
| 213 | [Hardcoded backend string cleanup](213-hardcoded-backend-strings.md) | XS |
| 214 | [Timeout policy object](214-timeout-policy-object.md) | S |
| 218 | [Gate failures JSONL](218-gate-failures-jsonl.md) | XS |
| 220 | [Disabled providers list](220-disabled-providers-list.md) | XS |
| ~~222~~ | ~~[Provider configuration UX](222-provider-config-ux.md)~~ | ~~L~~ | **Done** (2026-08-31) |
| 223 | [Interactive setup wizard (ratatui)](223-setup-wizard-tui.md) | XL |
| 224 | [Platforms & output transports](224-platforms-and-transports.md) | XL |
| 225 | [Telegram bot integration](225-telegram-bot-integration.md) | L |
| ~~226~~ | ~~[Plan generate lock scope reduction](226-plan-generate-lock-scope.md)~~ | ~~S~~ | **Done** (2026-08-31) |
| ~~227~~ | ~~[`--from-backlog` plan generation](227-from-backlog-plan-generation.md)~~ | ~~M~~ | **Done** (2026-08-31) |
| ~~228~~ | ~~[Dogfood session evidence bundle](228-dogfood-session-evidence-bundle.md)~~ | ~~M~~ | **Development harness source complete; final live fixtures pending** (2026-08-31) |
| 229 | [Backlog and plan state reconciliation](229-backlog-plan-state-reconciliation.md) | M |
| 232 | [TUI connected-mode data bridge](232-tui-connected-mode-data-bridge.md) | M |
| 233 | [Executor-neutral TUI command channel](233-tui-runner-command-channel.md) | M |
| 234 | [Gate output streaming to TUI](234-gate-output-streaming-tui.md) | M |

### Wave 4: TUI Quality

Improvements to the TUI that make monitoring and debugging sessions productive.

| # | Title | Size |
|---|---|---|
| ~~109~~ | ~~[TUI Real-Time Streaming Parity](archive/109-tui-realtime-streaming-parity.md)~~ | ~~XL~~ | **Superseded by #121/#232-#234/#246-#248/#266/#274** |
| 121 | [TUI data model unification](121-tui-data-model-unification.md) | L |
| 122 | [Remove legacy page system](122-remove-legacy-page-system.md) | S |
| ~~123~~ | ~~[ROSEDUST palette port](123-rosedust-palette.md)~~ | ~~S~~ | **Done** (2026-08-31) |
| ~~124~~ | ~~[Header bar Mori parity](124-header-bar-mori-parity.md)~~ | ~~M~~ | **Done** (2026-08-31) |
| ~~125~~ | ~~[Plan tree wave hierarchy widget](125-plan-tree-wave-hierarchy-widget.md)~~ | ~~M~~ | **Done** (2026-08-31) |
| ~~126~~ | ~~[Error digest widget](126-error-digest-widget.md)~~ | ~~S~~ | **Done** (2026-08-31) |
| ~~127~~ | ~~[F7 inspect view parity](127-inspect-view-f7-parity.md)~~ | ~~M~~ | **Done** (2026-08-31) |
| ~~10~~ | ~~[Daimon TUI View](10-daimon-tui-view.md)~~ | ~~S~~ | **Done** (2026-08-31) |
| ~~112~~ | ~~[Plan run continuous screenshots](112-plan-run-continuous-screenshots.md)~~ | ~~M~~ | **Done** (2026-08-31) |
| ~~141~~ | ~~[Per-turn efficiency events](141-per-turn-efficiency-events.md)~~ | ~~XS~~ | **Done** (2026-08-24) |
| ~~149~~ | ~~[Full prompt text logging](149-full-prompt-text-logging.md)~~ | ~~XS~~ | **Done** (2026-08-28) |
| 151 | [TUI PNG Snapshot Rendering](151-tui-png-snapshot-rendering.md) | M |
| 152 | [Screenshot Diff/Compare Engine](152-screenshot-diff-compare-engine.md) | M |
| ~~156~~ | ~~[Per-Model/Provider Cost Stats TUI](156-per-model-provider-cost-stats-tui.md)~~ | ~~S~~ | **Done** (2026-08-31) |
| ~~157~~ | ~~[Context-Sensitive Keybind Hints](157-context-sensitive-keybind-hints.md)~~ | ~~S~~ | **Done** (2026-08-31) |
| ~~189~~ | ~~[Agent status panel](189-agent-status-panel.md)~~ | ~~S~~ | **Done** (2026-08-31) |
| ~~201~~ | ~~[TUI notification toasts](201-tui-notification-toasts.md)~~ | ~~S~~ | **Done** (2026-08-31) |
| 217 | [TUI log search/filter](217-tui-log-search-filter.md) | S |
| 219 | [TUI plan filter](219-tui-plan-filter.md) | S |
| 235 | [TUI render-path disk I/O elimination](235-tui-render-path-disk-io-elimination.md) | S |
| 236 | [TUI empty state messages](236-tui-empty-state-messages.md) | S |
| 237 | [TUI keyboard model fixes](237-tui-keyboard-model-fixes.md) | M |
| 238 | [Plan detail enrichment](238-plan-detail-enrichment.md) | M |
| 239 | [Header bar enrichment](239-header-bar-enrichment.md) | S |
| 240 | [NET/DSK system metrics sampling](240-net-dsk-system-metrics-sampling.md) | XS |
| 241 | [TUI visual density improvements](241-tui-visual-density-improvements.md) | S |

### Wave 5: Learning & Knowledge Loop Closure

Items that close feedback loops so the system actually improves from runs.

| # | Title | Size |
|---|---|---|
| ~~116~~ | ~~[Queue manifest](116-queue-manifest.md)~~ | ~~L~~ | **Done** (2026-08-31) |
| ~~117~~ | ~~[Plan-level wave computation](117-plan-wave-computation.md)~~ | ~~M~~ | **Done** (2026-08-31) |
| 118 | [Express mode auto-fix](118-express-mode-autofix.md) | M |
| 05 | [Express Mode](05-express-mode.md) | M |
| 14 | [Plan Mutation Protocol](14-plan-mutation-protocol.md) | M |
| 15 | [Post-Gate Reflection](15-post-gate-reflection.md) | M |
| ~~34~~ | ~~[PRD Cascade Learning](34-prd-cascade-learning.md)~~ | ~~S~~ | **Done** (2026-08-28) |
| 80 | [Learning Subsystem Data Quality](80-learning-subsystem-data-quality.md) | M |
| 84 | [Cascade Router Task Category Awareness](84-cascade-router-task-category-awareness.md) | M |
| 90 | [UX34 Override Learning Isolation](90-ux34-override-learning-isolation.md) | M |
| ~~91~~ | ~~[Experiment Assignment Stability](91-experiment-assignment-stability.md)~~ | ~~S~~ | **Done** (2026-08-28) |
| 92 | [Hindsight Causal Inference](92-hindsight-causal-inference.md) | M |
| 139 | [Per-plan agent-handle map for concurrency](139-per-plan-agent-handle-map.md) | M |
| ~~140~~ | ~~[Merge success/conflict proof](140-merge-success-conflict-proof.md)~~ | ~~S~~ | **Done** (2026-08-31) |
| 142 | [Knowledge write-back proof](142-knowledge-write-back-proof.md) | S |
| ~~143~~ | ~~[Dream consolidation trigger](143-dream-consolidation-trigger.md)~~ | ~~S~~ | **Done** (2026-08-28) |
| ~~144~~ | ~~[Daimon affect persistence](144-daimon-affect-persistence-metadata.md)~~ | ~~S~~ | **Done** (verified already wired, 2026-08-28) |
| ~~83~~ | ~~[Dream Consolidation Deadlock](83-dream-consolidation-deadlock.md)~~ | ~~S~~ | **Done** (2026-08-28) |
| 145 | [Prompt section effectiveness proof](145-prompt-section-effectiveness-loop-proof.md) | S |
| 180 | [Per-plan config overrides](180-per-plan-config-overrides.md) | M |
| 181 | [Per-role context/effort](181-per-role-context-effort.md) | S |
| 187 | [Per-worktree MCP config](187-per-worktree-mcp-config.md) | S |
| 188 | [TUI config editing persistence](188-tui-config-editing.md) | S |
| 194 | [Post-merge regression](194-post-merge-regression.md) | M |
| 197 | [Structured reviewer JSON](197-structured-reviewer-json.md) | S |
| ~~200~~ | ~~[Plan validate --dag](200-plan-validate-dag.md)~~ | ~~S~~ | **Done** (2026-08-31) |
| 202 | [Directive classification/routing](202-directive-classification-routing.md) | M |
| 203 | [Error pattern sharing](203-error-pattern-sharing.md) | S |
| 204 | [Review cap force-commit](204-review-cap-force-commit.md) | S |
| 205 | [get_plan_context MCP tool](205-get-plan-context-mcp-tool.md) | S |
| 207 | [Per-task routing hints](207-per-task-routing-hints.md) | S |
| 209 | [Provider proof matrix](209-provider-proof-matrix.md) | M |
| 210 | [Code index prompt section](210-code-index-prompt-section.md) | S |
| 211 | [Prompt snapshot tests](211-prompt-snapshot-tests.md) | XS |
| ~~215~~ | ~~[HTTP run-scoped event query](215-http-run-scoped-event-query.md)~~ | ~~M~~ | **Done for new runs; offline historical repair remains** (2026-08-31) |
| 221 | [Per-role agent toggles](221-per-role-agent-toggles.md) | XS |
| 231 | [Cross-crate change-impact scoping](231-cross-crate-change-impact-scoping.md) | M | **Partial: conservative Cargo/syntax path implemented; semantic index open** |

### Wave 6: Architecture Cleanup

Decomposition, consolidation, and dead code removal. Safe to do in parallel with Wave 5.

| # | Title | Size |
|---|---|---|
| 43 | [Clippy Suppression Removal](43-clippy-suppression-removal.md) | M |
| 47 | [ConfigLayer Elimination](47-configlayer-elimination.md) | L |
| ~~61~~ | ~~[Agent Dispatch Consolidation](archive/61-dispatch-consolidation.md)~~ | ~~XL~~ | **Superseded by #243-#247/#253/#274** |
| 102 | [Cross-Crate Code Duplication](102-cross-crate-code-duplication.md) | M |
| 105 | [HTTP API Design Consistency](105-http-api-design-consistency.md) | M |
| 106 | [Memory Allocation Hot Paths](106-memory-allocation-hot-paths.md) | M |
| 110 | [Deprecate JSONL / StateHub-only](110-deprecate-jsonl-statehub-only.md) | XL |
| ~~147~~ | ~~[Backlog import command](147-backlog-import-command.md)~~ | ~~M~~ | **Done** (2026-08-31) |
| 148 | [TUI god objects decomposition](148-tui-god-objects-decomposition.md) | L |
| ~~150~~ | ~~[Cybernetic feature wire-or-remove audit](archive/150-cybernetic-feature-wire-or-remove-audit.md)~~ | ~~M~~ | **Superseded by engine audit/#242-#285** |
| ~~19~~ | ~~[Contextual Bandit Dead Code](archive/19-contextual-bandit-dead-code.md)~~ | ~~XS~~ | **Done** (2026-08-26) |
| ~~82~~ | ~~[Graph Stub Cell Warnings](82-graph-stub-cell-warnings.md)~~ | ~~S~~ | **Done** (2026-08-28) |
| ~~155~~ | ~~[HDC Math Dead Code Archival](155-hdc-math-dead-code-archival.md)~~ | ~~XS~~ | **Done** (2026-08-26) |
| ~~42~~ | ~~[Duplicate Type Consolidation](42-duplicate-type-consolidation.md)~~ | ~~S~~ | **Done** (2026-08-28) |
| 190 | [CorticalState wiring](190-corticalstate-wiring.md) | M |
| ~~191~~ | ~~[EnrichedCell dispatch](archive/191-enriched-cell-dispatch.md)~~ | ~~M~~ | **Superseded by #253/#265/#268-#270** |
| ~~193~~ | ~~[Gateway runner-v2 wiring](archive/193-gateway-runner-v2-wiring.md)~~ | ~~L~~ | **Superseded by #243-#247/#274** |
| 208 | [Canonical layer-safe runtime event schema](208-unified-event-schema.md) | M |
| 230 | [Feature-gate chain/Alloy from the default CLI build](230-chain-alloy-default-build-feature-gating.md) | M | **Source complete; final compile/benchmark evidence open** |

### Wave 7: Nice-to-Have / Future

P3 items and Phase 2+ features. Do last or defer indefinitely.

| # | Title | Size |
|---|---|---|
| 158 | [Codex CLI Provider Kind](158-codex-cli-provider-kind.md) | M |
| ~~161~~ | ~~[Codex Cost Reporting](161-codex-cost-reporting.md)~~ | ~~XS~~ | **Done** (2026-08-24) |
| 162 | [PRD/Research Provider Routing](162-prd-provider-routing.md) | S |
| 172 | [Deep Hermes / Nous Research Integration](172-deep-hermes-nous-integration.md) | XL |
| 173 | [Nous Research Ecosystem Reference](173-nous-ecosystem-reference.md) | — (ref doc) |
| 174 | [Novel Nous Integration Architectures](174-nous-novel-integrations.md) | XL |
| 175 | [Standalone Nous Research Projects](175-standalone-nous-projects.md) | — (catalog) |
| 176 | [Nous Research Competitive & Research Landscape](176-nous-research-landscape.md) | — (ref doc) |
| 177 | [Hermes Agent Patterns to Adopt](177-hermes-agent-patterns.md) | M |
| 09 | [Recursive Safety Patterns](09-recursive-safety-patterns.md) | L |
| ~~11~~ | ~~[Justfile](11-justfile.md)~~ | ~~XS~~ | **Done** (2026-08-22) |
| 16 | [Warm Agent Spawning](16-warm-agent-spawning.md) | M |
| ~~41~~ | ~~[TUI Push-Mode Panel Data](41-tui-push-mode-panel-data.md)~~ | ~~S~~ | **Done** (2026-08-31) |
| 59 | [HuggingFace Provider](59-huggingface-provider.md) | S |
| 63 | [Zero-Config Onboarding Wizard](63-zero-config-onboarding.md) | M |
| 64 | [Model Discovery UX](64-model-discovery-ux.md) | S |
| ~~65~~ | ~~[CLI Verb Consolidation](65-cli-verb-consolidation.md)~~ | ~~L~~ | **Done** (2026-08-31) |
| 66 | [Context Sources & Editor Integration](66-context-editor-integration.md) | L |
| ~~71~~ | ~~[TUI Design System Alignment](71-tui-design-alignment.md)~~ | ~~S~~ | **Done** (2026-08-31) |
| 73 | [UX Backlog Rollup](73-ux-backlog-rollup.md) | M |
| 74 | [Unified Evaluation Framework](74-evaluation-framework.md) | XL |
| 99 | [Examples & Documentation Gaps](99-examples-documentation-gaps.md) | M |
| ~~128~~ | ~~[Adaptive frame rate](128-adaptive-frame-rate.md)~~ | ~~XS~~ | **Done** (2026-08-27) |
| ~~129~~ | ~~[Metric exponential smoothing](129-metric-exponential-smoothing.md)~~ | ~~XS~~ | **Done** (2026-08-27) |
| 130 | [Tab content badges](130-tab-content-badges.md) | XS |
| 153 | [Automated Visual Assessment Loop](153-automated-visual-assessment-loop.md) | L |
| 154 | [HTTP Monitoring Workflow Documentation](154-http-monitoring-workflow-documentation.md) | XS |
| 179 | [Batch controller](179-batch-controller.md) | XS |
| 183 | [Crash report file](183-crash-report-file.md) | S |
| 184 | [Artifact freshness checking](184-artifact-freshness-checking.md) | XS |
| 185 | [MCP result caching](185-mcp-result-caching.md) | XS |
| 186 | [MCP token savings tracking](186-mcp-token-savings-tracking.md) | XS |
| 192 | [Native agent telemetry](192-native-agent-telemetry.md) | L |
| ~~195~~ | ~~[File overlap analysis](195-file-overlap-analysis.md)~~ | ~~XS~~ | **Done** (2026-08-31) |
| 196 | [Critical path ETA](196-critical-path-eta.md) | XS |
| 198 | [Supervisor auto-recovery](198-supervisor-auto-recovery.md) | S |
| 199 | [TUI resizable panes](199-tui-resizable-panes.md) | S |
| 216 | [TUI queue overview modal](216-tui-queue-overview-modal.md) | S |

---

## Full Priority Index

### P0 — Critical

Crashes, silent data loss, or correctness failures that corrupt the learning pipeline.

| # | Title | Size | Wave |
|---|---|---|---|
| 17 | [ACP Stability Hardening](17-acp-stability-hardening.md) | L | 0 |
| 75 | [Graph Example Schema Drift Regression](75-graph-example-schema-drift.md) | XS | 0 | **Reopened** (2026-08-31) |
| ~~78~~ | ~~[Efficiency gate_passed Bug](78-efficiency-gate-passed-bug.md)~~ | ~~S~~ | **Done** (2026-08-25) | 0 |
| ~~86~~ | ~~[Gate Compile Tool Bypass](archive/86-gate-compile-tool-bypass.md)~~ | ~~XS~~ | ~~0~~ | **Done** (2026-08-24) |
| ~~87~~ | ~~[Task Parser Duplicate IDs](archive/87-task-parser-duplicate-ids.md)~~ | ~~S~~ | ~~0~~ | **Done** (2026-08-24) |

### P1 — High

Direct cost, quality, or reliability impact; blocking the self-hosting loop.

| # | Title | Size | Wave |
|---|---|---|---|
| 03 | [Context Injection Scoping](03-context-injection-scoping.md) | M | 3 |
| 04 | [Compile Auto-Fix Path](04-compile-autofix-path.md) | S | 3 |
| 18 | [ACP Spec Upgrade & Refactor](18-acp-spec-upgrade-and-refactor.md) | XL | 6 |
| 21 | [Landing Page Fake Metrics](21-landing-page-fake-metrics.md) | S | 3 |
| ~~33~~ | ~~[CLI Gist Scrubbing](33-cli-gist-scrubbing.md)~~ | ~~S~~ | **Done** (2026-08-25) | 1 |
| 45 | [ACP Tool Permission Gate](45-acp-tool-permission-gate.md) | M | 2 |
| ~~48~~ | ~~[Serve Auth Default Posture](archive/48-serve-auth-default.md)~~ | ~~S~~ | ~~1~~ | **Done** (2026-08-24) |
| ~~49~~ | ~~[Serve CORS Restrictive](49-serve-cors-restrictive.md)~~ | ~~S~~ | **Done** (2026-08-25) | 1 |
| ~~50~~ | ~~[Serve Rate and Body Limits](50-serve-rate-body-limits.md)~~ | ~~S~~ | **Done** (2026-08-25) | 1 |
| ~~51~~ | ~~[Serve Agent Name Validation](51-serve-path-traversal.md)~~ | ~~S~~ | **Done** (2026-08-25) | 1 |
| 56 | [ACP Single-Agent Chat: Tools Require Client Capability Declaration](56-acp-single-agent-tools.md) | M | 2 |
| 60 | [Safety Dispatch Hardening](60-safety-dispatch-hardening.md) | M | 1 |
| 76 | [Example Config Quality](76-example-config-quality.md) | M | 3 |
| ~~77~~ | ~~[CLI UX Consistency](77-cli-ux-consistency.md)~~ | ~~S~~ | ~~3~~ | **Done** (2026-08-31) |
| ~~79~~ | ~~[Doctor/Onboarding Diagnostics](79-doctor-onboarding-diagnostics.md)~~ | ~~M~~ | ~~3~~ | **Done** (2026-08-31) |
| ~~88~~ | ~~[Handler Future Lifecycle Panic](archive/88-handler-future-lifecycle-panic.md)~~ | ~~XS~~ | ~~2~~ | **Done** (2026-08-24) |
| ~~89~~ | ~~[Rate Limiter Mutex Poisoning](archive/89-rate-limiter-mutex-poisoning.md)~~ | ~~XS~~ | ~~2~~ | **Done** (2026-08-24) |
| 90 | [UX34 Override Learning Isolation](90-ux34-override-learning-isolation.md) | M | 2 |
| 93 | [Plugin Runtime Verification](93-plugin-runtime-verification.md) | S | 3 |
| ~~95~~ | ~~[Config Loader Robustness](95-config-loader-robustness.md)~~ | ~~S~~ | ~~2~~ | **Done** (2026-08-28) |
| ~~96~~ | ~~[API Key Timing-Safe Comparison](archive/96-serve-api-key-timing-safe.md)~~ | ~~XS~~ | ~~1~~ | **Done** (2026-08-24) |
| ~~97~~ | ~~[Snapshot Backup & Staging Cleanup](97-snapshot-backup-and-staging-cleanup.md)~~ | ~~S~~ | **Done** (2026-08-26) | 2 |
| 101 | [Async Runtime Anti-Patterns](101-async-runtime-anti-patterns.md) | M | 2 |
| ~~107~~ | ~~[Plan Run UX Friction](107-plan-run-ux-friction.md)~~ | ~~M~~ | ~~3~~ | **Done** (2026-08-31) |
| ~~108~~ | ~~[TUI Live Feedback Gaps](108-tui-live-feedback-gaps.md)~~ | ~~L~~ | ~~3~~ | **Done** (2026-08-31; 5/6 sub-items wired) |
| ~~109~~ | ~~[TUI Real-Time Streaming Parity](archive/109-tui-realtime-streaming-parity.md)~~ | ~~XL~~ | ~~4~~ | **Superseded** |
| 110 | [Deprecate JSONL / StateHub-only](110-deprecate-jsonl-statehub-only.md) | XL | 6 |
| 111 | [Screenshot command completion](111-screenshot-command-completion.md) | S | 3 |
| ~~113~~ | ~~[CLI JSON output mode](113-cli-json-output-mode.md)~~ | ~~M~~ | ~~3~~ | **Done** (2026-08-31) |
| ~~119~~ | ~~[TUI recovery keybindings](archive/119-tui-recovery-keybindings.md)~~ | ~~M~~ | ~~3~~ | **Superseded by #233/#255** |
| ~~120~~ | ~~[Plan run preflight checks](120-plan-run-preflight-checks.md)~~ | ~~S~~ | ~~3~~ | **Done** (2026-08-31) |
| ~~131~~ | ~~[PRD/cloud-worker migration to runner-v2](131-prd-cloud-worker-runner-v2-migration.md)~~ | ~~S~~ | ~~2~~ | **Done** (2026-08-31) |
| ~~132~~ | ~~[orchestrate.rs freeze/retirement](132-orchestrate-rs-freeze-retirement.md)~~ | ~~M~~ | **Done** (confirmed 2026-08-29; file removed) |
| ~~133~~ | ~~[dangerously_skip_permissions opt-in](133-dangerously-skip-permissions-opt-in.md)~~ | ~~XS~~ | **Done** (2026-08-25) | 1 |
| ~~134~~ | ~~[Replan-on-gate-failure in runner-v2](archive/134-replan-on-gate-failure-runner-v2.md)~~ | ~~M~~ | ~~2~~ | **Superseded by #252** |
| ~~135~~ | ~~[Adaptive gate thresholds in runner-v2](archive/135-adaptive-gate-thresholds-runner-v2.md)~~ | ~~S~~ | ~~2~~ | **Superseded by #250/#253** |
| ~~136~~ | ~~[Safety denial audit events](136-safety-denial-audit-events.md)~~ | ~~S~~ | ~~1~~ | **Done** (2026-08-28) |
| ~~137~~ | ~~[Router + threshold state in crash snapshot](archive/137-router-threshold-state-crash-snapshot.md)~~ | ~~XS~~ | ~~2~~ | **Superseded by #251** |
| 138 | [Executor-neutral crash/resume proof matrix](138-crash-resume-proof-matrix.md) | M | engine DAG |
| ~~146~~ | ~~[Plan CLI control commands](146-plan-cli-control-commands.md)~~ | ~~M~~ | ~~3~~ | **Done** (2026-08-31) |
| ~~159~~ | ~~[Plan Run Report Phase Latency](archive/159-plan-run-report-phase-latency.md)~~ | ~~S~~ | ~~3~~ | **Done** (2026-08-24) |
| ~~160~~ | ~~[--fresh Worktree/Branch Cleanup](archive/160-fresh-worktree-branch-cleanup.md)~~ | ~~XS~~ | ~~3~~ | **Done** (2026-08-24) |
| ~~163~~ | ~~[Shared Build Cache for Worktrees](archive/163-worktree-shared-build-cache.md)~~ | ~~S~~ | ~~3~~ | **Done** (2026-08-22) |
| ~~164~~ | ~~[SchedulerNoProgress Timeout Too Short](archive/164-scheduler-progress-timeout-too-short.md)~~ | ~~XS~~ | ~~3~~ | **Done** (2026-08-22) |
| 166 | [Gate verify pre-existing failure filtering](166-gate-verify-preexisting-filter.md) | S | 3 |
| 170 | [Adaptive task-verify scoping](170-adaptive-verify-scoping.md) | S | 3 |
| 178 | [Conductor supervisor loop](178-conductor-supervisor-loop.md) | M | 3 |
| ~~206~~ | ~~[CARGO_BUILD_JOBS limit](206-cargo-build-jobs-limit.md)~~ | ~~XS~~ | ~~3~~ | **Done** (2026-08-31) |
| ~~222~~ | ~~[Provider configuration UX](222-provider-config-ux.md)~~ | ~~L~~ | ~~3~~ | **Done** (2026-08-31) |
| 223 | [Interactive setup wizard (ratatui)](223-setup-wizard-tui.md) | XL | 3 |
| 224 | [Platforms & output transports](224-platforms-and-transports.md) | XL | 3 |
| 225 | [Telegram bot integration](225-telegram-bot-integration.md) | L | 3 |
| ~~226~~ | ~~[Plan generate lock scope reduction](226-plan-generate-lock-scope.md)~~ | ~~S~~ | ~~3~~ | **Done** (2026-08-31) |
| ~~227~~ | ~~[`--from-backlog` plan generation](227-from-backlog-plan-generation.md)~~ | ~~M~~ | ~~3~~ | **Done** (2026-08-31) |
| ~~228~~ | ~~[Dogfood session evidence bundle](228-dogfood-session-evidence-bundle.md)~~ | ~~M~~ | ~~3~~ | **Source complete; final live fixtures pending** |
| 229 | [Backlog and plan state reconciliation](229-backlog-plan-state-reconciliation.md) | M | 3 |
| 231 | [Cross-crate change-impact scoping](231-cross-crate-change-impact-scoping.md) | M | 5 | **Partial** |
| 232 | [TUI connected-mode data bridge](232-tui-connected-mode-data-bridge.md) | M | 3 |
| 233 | [Executor-neutral TUI command channel](233-tui-runner-command-channel.md) | M | engine DAG |
| 234 | [Gate output streaming to TUI](234-gate-output-streaming-tui.md) | M | 3 |
| ~~286~~ | ~~[FAST hard-deadline interposition](286-fast-hard-deadline-interposition.md)~~ | ~~M~~ | ~~3~~ | **Source complete; final matrix pending** |

### P2 — Medium

Efficiency, UX quality, maintainability, and feedback loop closure.

| # | Title | Size | Wave |
|---|---|---|---|
| 01 | [T0 Reflex Store](01-t0-reflex-store.md) | M | 5 |
| 02 | [Reactive Agent Mode](02-reactive-agent-mode.md) | L | 7 |
| 05 | [Express Mode](05-express-mode.md) | M | 5 |
| ~~10~~ | ~~[Daimon TUI View](10-daimon-tui-view.md)~~ | ~~S~~ | ~~4~~ | **Done** (2026-08-31) |
| 12 | [E2E Test Harness](12-e2e-test-harness.md) | M | 3 |
| 13 | [Historical Cost Calibration](13-historical-cost-calibration.md) | S | 5 |
| 14 | [Plan Mutation Protocol](14-plan-mutation-protocol.md) | M | 5 |
| 15 | [Post-Gate Reflection](15-post-gate-reflection.md) | M | 5 |
| ~~20~~ | ~~[Event Loop Decomposition](archive/20-event-loop-decomposition.md)~~ | ~~XL~~ | ~~6~~ | **Superseded by #246-#261** |
| 22 | [Chat Inline Decomposition](22-chat-inline-decomposition.md) | M | 6 |
| ~~34~~ | ~~[PRD Cascade Learning](34-prd-cascade-learning.md)~~ | ~~S~~ | ~~5~~ | **Done** (2026-08-28) |
| 35 | [CLI Output Redesign](35-cli-output-redesign.md) | M | 3 |
| 37 | [Multi-Process Locking](37-multi-process-locking.md) | S | 2 |
| 38 | [Provider Error UX](38-provider-error-ux.md) | S | 3 |
| 39 | [ACP Learning-Pipeline Parity](39-learning-pipeline-acp-parity.md) | M | 5 |
| 40 | [Gate Rung Input Completion](40-gate-rung-input-completion.md) | S | 2 |
| 43 | [Clippy Suppression Removal](43-clippy-suppression-removal.md) | M | 6 |
| 44 | [Calibration Feedback Loop](44-calibration-feedback-loop.md) | M | 5 |
| 46 | [ACP Test Coverage](46-acp-test-coverage.md) | S | 2 |
| 47 | [ConfigLayer Elimination](47-configlayer-elimination.md) | L | 6 |
| 52 | [MCP Stderr Capture & CostTable Gaps](52-mcp-stderr-costtable.md) | S | 3 |
| 53 | [Immune System Adaptive Screening](53-immune-adaptive-screening.md) | L | 5 |
| ~~54~~ | ~~[Graph Engine Runner-v2 Parity](archive/54-graph-engine-runner-parity.md)~~ | ~~XL~~ | ~~6~~ | **Superseded by #242-#285** |
| 55 | [AgentPool Runtime Integration](55-agent-pool-runtime-integration.md) | M | 5 |
| 57 | [Plan Generation Escalation](57-plan-generation-escalation.md) | S | 5 |
| 58 | [Performance Hot-Path Fixes](58-perf-hot-path-fixes.md) | M | 6 | **Source complete; final batch pending** |
| ~~61~~ | ~~[Agent Dispatch Consolidation](archive/61-dispatch-consolidation.md)~~ | ~~XL~~ | ~~6~~ | **Superseded by #243-#247/#253/#274** |
| 62 | [Relay Topic Namespace Migration](62-relay-topic-migration.md) | M | 6 |
| 67 | [HDC Prompt Assembly Wiring](67-hdc-prompt-assembly.md) | M | 5 |
| 68 | [Budget Pre-Dispatch Admission](68-budget-pre-dispatch.md) | S | 5 |
| 69 | [SSE Parsing Deduplication](69-sse-parsing-dedup.md) | S | 3 |
| 70 | [ACP Novel Workflow Gaps](70-acp-workflow-gaps.md) | M | 5 |
| 72 | [Pool Architecture Reconciliation](72-pool-architecture-reconciliation.md) | S | 6 |
| 80 | [Learning Subsystem Data Quality](80-learning-subsystem-data-quality.md) | M | 5 |
| ~~81~~ | ~~[Layer-Check False Positives](81-layer-check-false-positives.md)~~ | ~~S~~ | **Done** (2026-08-26) | 2 |
| ~~82~~ | ~~[Graph Stub Cell Warnings](82-graph-stub-cell-warnings.md)~~ | ~~S~~ | ~~6~~ | **Done** (2026-08-28) |
| ~~83~~ | ~~[Dream Consolidation Deadlock](83-dream-consolidation-deadlock.md)~~ | ~~S~~ | ~~5~~ | **Done** (2026-08-28) |
| 84 | [Cascade Router Task Category Awareness](84-cascade-router-task-category-awareness.md) | M | 5 |
| ~~85~~ | ~~[Plan Generation TOML Reliability](85-plan-generation-toml-reliability.md)~~ | ~~S~~ | ~~3~~ | **Done** (2026-08-31) |
| ~~91~~ | ~~[Experiment Assignment Stability](91-experiment-assignment-stability.md)~~ | ~~S~~ | ~~5~~ | **Done** (2026-08-28) |
| 92 | [Hindsight Causal Inference](92-hindsight-causal-inference.md) | M | 5 |
| 94 | [ProcessSupervisor Untracked Tasks](94-process-supervisor-untracked-tasks.md) | S | 2 |
| 98 | [GitHub Workflow Robustness](98-github-workflow-robustness.md) | M | 3 |
| 100 | [CLI Error Message Quality](100-cli-error-message-quality.md) | S | 3 |
| 102 | [Cross-Crate Code Duplication](102-cross-crate-code-duplication.md) | M | 6 |
| 103 | [Plan Execution Resilience](103-plan-execution-resilience.md) | M | 3 |
| 104 | [Doctor Diagnostic Coverage](104-doctor-diagnostic-coverage.md) | M | 3 |
| 105 | [HTTP API Design Consistency](105-http-api-design-consistency.md) | M | 6 |
| 106 | [Memory Allocation Hot Paths](106-memory-allocation-hot-paths.md) | M | 6 |
| ~~112~~ | ~~[Plan run continuous screenshots](112-plan-run-continuous-screenshots.md)~~ | ~~M~~ | ~~4~~ | **Done** (2026-08-31) |
| ~~114~~ | ~~[Diagnose command enrichment](114-diagnose-command-enrich.md)~~ | ~~XS~~ | **Done** (2026-08-27) | 3 |
| ~~115~~ | ~~[Structured log wire verification](115-structured-log-wire-verification.md)~~ | ~~XS~~ | ~~3~~ | **Done** (2026-08-28) |
| ~~116~~ | ~~[Queue manifest](116-queue-manifest.md)~~ | ~~L~~ | ~~5~~ | **Done** (2026-08-31) |
| ~~117~~ | ~~[Plan-level wave computation](117-plan-wave-computation.md)~~ | ~~M~~ | ~~5~~ | **Done** (2026-08-31) |
| 118 | [Express mode auto-fix](118-express-mode-autofix.md) | M | 5 |
| 121 | [TUI data model unification](121-tui-data-model-unification.md) | L | 4 |
| 122 | [Remove legacy page system](122-remove-legacy-page-system.md) | S | 4 |
| ~~123~~ | ~~[ROSEDUST palette port](123-rosedust-palette.md)~~ | ~~S~~ | ~~4~~ | **Done** (2026-08-31) |
| ~~124~~ | ~~[Header bar Mori parity](124-header-bar-mori-parity.md)~~ | ~~M~~ | ~~4~~ | **Done** (2026-08-31) |
| ~~125~~ | ~~[Plan tree wave hierarchy widget](125-plan-tree-wave-hierarchy-widget.md)~~ | ~~M~~ | ~~4~~ | **Done** (2026-08-31) |
| ~~126~~ | ~~[Error digest widget](126-error-digest-widget.md)~~ | ~~S~~ | ~~4~~ | **Done** (2026-08-31) |
| ~~127~~ | ~~[F7 inspect view parity](127-inspect-view-f7-parity.md)~~ | ~~M~~ | ~~4~~ | **Done** (2026-08-31) |
| 139 | [Per-plan agent-handle map for concurrency](139-per-plan-agent-handle-map.md) | M | 5 |
| ~~140~~ | ~~[Merge success/conflict proof](140-merge-success-conflict-proof.md)~~ | ~~S~~ | ~~5~~ | **Done** (2026-08-31) |
| ~~141~~ | ~~[Per-turn efficiency events](141-per-turn-efficiency-events.md)~~ | ~~XS~~ | ~~4~~ | **Done** (2026-08-24) |
| 142 | [Knowledge write-back proof](142-knowledge-write-back-proof.md) | S | 5 |
| ~~143~~ | ~~[Dream consolidation trigger](143-dream-consolidation-trigger.md)~~ | ~~S~~ | ~~5~~ | **Done** (2026-08-28) |
| ~~144~~ | ~~[Daimon affect persistence](144-daimon-affect-persistence-metadata.md)~~ | ~~S~~ | ~~5~~ | **Done** (verified already wired, 2026-08-28) |
| 145 | [Prompt section effectiveness proof](145-prompt-section-effectiveness-loop-proof.md) | S | 5 |
| ~~147~~ | ~~[Backlog import command](147-backlog-import-command.md)~~ | ~~M~~ | ~~6~~ | **Done** (2026-08-31) |
| ~~149~~ | ~~[Full prompt text logging](149-full-prompt-text-logging.md)~~ | ~~XS~~ | ~~4~~ | **Done** (2026-08-28) |
| 151 | [TUI PNG Snapshot Rendering](151-tui-png-snapshot-rendering.md) | M | 4 |
| 152 | [Screenshot Diff/Compare Engine](152-screenshot-diff-compare-engine.md) | M | 4 |
| ~~156~~ | ~~[Per-Model/Provider Cost Stats TUI](156-per-model-provider-cost-stats-tui.md)~~ | ~~S~~ | ~~4~~ | **Done** (2026-08-31) |
| ~~157~~ | ~~[Context-Sensitive Keybind Hints](157-context-sensitive-keybind-hints.md)~~ | ~~S~~ | ~~4~~ | **Done** (2026-08-31) |
| 158 | [Codex CLI Provider Kind](158-codex-cli-provider-kind.md) | M | 7 |
| 162 | [PRD/Research Provider Routing](162-prd-provider-routing.md) | S | 7 |
| 172 | [Deep Hermes / Nous Research Integration](172-deep-hermes-nous-integration.md) | XL | 7 |
| 174 | [Novel Nous Integration Architectures](174-nous-novel-integrations.md) | XL | 7 |
| 177 | [Hermes Agent Patterns to Adopt](177-hermes-agent-patterns.md) | M | 7 |
| 180 | [Per-plan config overrides](180-per-plan-config-overrides.md) | M | 5 |
| 181 | [Per-role context/effort](181-per-role-context-effort.md) | S | 5 |
| ~~182~~ | ~~[Lightweight status file](182-lightweight-status-file.md)~~ | ~~XS~~ | ~~3~~ | **Done** (2026-08-31) |
| 187 | [Per-worktree MCP config](187-per-worktree-mcp-config.md) | S | 5 |
| 188 | [TUI config editing persistence](188-tui-config-editing.md) | S | 5 |
| ~~189~~ | ~~[Agent status panel](189-agent-status-panel.md)~~ | ~~S~~ | ~~4~~ | **Done** (2026-08-31) |
| ~~193~~ | ~~[Gateway runner-v2 wiring](archive/193-gateway-runner-v2-wiring.md)~~ | ~~L~~ | ~~6~~ | **Superseded by #243-#247/#274** |
| 194 | [Post-merge regression](194-post-merge-regression.md) | M | 5 |
| 197 | [Structured reviewer JSON](197-structured-reviewer-json.md) | S | 5 |
| ~~200~~ | ~~[Plan validate --dag](200-plan-validate-dag.md)~~ | ~~S~~ | ~~5~~ | **Done** (2026-08-31) |
| ~~201~~ | ~~[TUI notification toasts](201-tui-notification-toasts.md)~~ | ~~S~~ | ~~4~~ | **Done** (2026-08-31) |
| 202 | [Directive classification/routing](202-directive-classification-routing.md) | M | 5 |
| 203 | [Error pattern sharing](203-error-pattern-sharing.md) | S | 5 |
| 204 | [Review cap force-commit](204-review-cap-force-commit.md) | S | 5 |
| 205 | [get_plan_context MCP tool](205-get-plan-context-mcp-tool.md) | S | 5 |
| 207 | [Per-task routing hints](207-per-task-routing-hints.md) | S | 5 |
| 208 | [Canonical layer-safe runtime event schema](208-unified-event-schema.md) | M | engine DAG |
| 209 | [Provider proof matrix](209-provider-proof-matrix.md) | M | 5 |
| 210 | [Code index prompt section](210-code-index-prompt-section.md) | S | 5 |
| 211 | [Prompt snapshot tests](211-prompt-snapshot-tests.md) | XS | 5 |
| ~~212~~ | ~~[run_id in snapshots and events](212-run-id-snapshot-events.md)~~ | ~~XS~~ | ~~3~~ | **Done** (2026-08-31) |
| 213 | [Hardcoded backend string cleanup](213-hardcoded-backend-strings.md) | XS | 3 |
| 214 | [Timeout policy object](214-timeout-policy-object.md) | S | 3 |
| ~~215~~ | ~~[HTTP run-scoped event query](215-http-run-scoped-event-query.md)~~ | ~~M~~ | ~~5~~ | **Done for new runs; historical repair open** |
| 217 | [TUI log search/filter](217-tui-log-search-filter.md) | S | 4 |
| 218 | [Gate failures JSONL](218-gate-failures-jsonl.md) | XS | 3 |
| 219 | [TUI plan filter](219-tui-plan-filter.md) | S | 4 |
| 220 | [Disabled providers list](220-disabled-providers-list.md) | XS | 3 |
| 221 | [Per-role agent toggles](221-per-role-agent-toggles.md) | XS | 5 |
| 230 | [Feature-gate chain/Alloy from the default CLI build](230-chain-alloy-default-build-feature-gating.md) | M | 6 | **Source complete; compile/bench pending** |
| 235 | [TUI render-path disk I/O elimination](235-tui-render-path-disk-io-elimination.md) | S | 4 |
| 236 | [TUI empty state messages](236-tui-empty-state-messages.md) | S | 4 |
| 237 | [TUI keyboard model fixes](237-tui-keyboard-model-fixes.md) | M | 4 |
| 238 | [Plan detail enrichment](238-plan-detail-enrichment.md) | M | 4 |
| 239 | [Header bar enrichment](239-header-bar-enrichment.md) | S | 4 |
| 240 | [NET/DSK system metrics sampling](240-net-dsk-system-metrics-sampling.md) | XS | 4 |
| 241 | [TUI visual density improvements](241-tui-visual-density-improvements.md) | S | 4 |

### P3 — Low

Nice-to-have features, Phase 2+ scope, or speculative improvements.

| # | Title | Size | Wave |
|---|---|---|---|
| 09 | [Recursive Safety Patterns](09-recursive-safety-patterns.md) | L | 7 |
| ~~11~~ | ~~[Justfile](11-justfile.md)~~ | ~~XS~~ | ~~7~~ | **Done** (2026-08-22) |
| 16 | [Warm Agent Spawning](16-warm-agent-spawning.md) | M | 7 |
| ~~19~~ | ~~[Contextual Bandit Dead Code](archive/19-contextual-bandit-dead-code.md)~~ | ~~XS~~ | **Done** (2026-08-26) | 6 |
| ~~41~~ | ~~[TUI Push-Mode Panel Data](41-tui-push-mode-panel-data.md)~~ | ~~S~~ | ~~7~~ | **Done** (2026-08-31) |
| ~~42~~ | ~~[Duplicate Type Consolidation](42-duplicate-type-consolidation.md)~~ | ~~S~~ | ~~6~~ | **Done** (2026-08-28) |
| 59 | [HuggingFace Provider](59-huggingface-provider.md) | S | 7 |
| 63 | [Zero-Config Onboarding Wizard](63-zero-config-onboarding.md) | M | 7 |
| 64 | [Model Discovery UX](64-model-discovery-ux.md) | S | 7 |
| ~~65~~ | ~~[CLI Verb Consolidation](65-cli-verb-consolidation.md)~~ | ~~L~~ | ~~7~~ | **Done** (2026-08-31) |
| 66 | [Context Sources & Editor Integration](66-context-editor-integration.md) | L | 7 |
| ~~71~~ | ~~[TUI Design System Alignment](71-tui-design-alignment.md)~~ | ~~S~~ | ~~7~~ | **Done** (2026-08-31) |
| 73 | [UX Backlog Rollup](73-ux-backlog-rollup.md) | M | 7 |
| 74 | [Unified Evaluation Framework](74-evaluation-framework.md) | XL | 7 |
| 99 | [Examples & Documentation Gaps](99-examples-documentation-gaps.md) | M | 7 |
| ~~128~~ | ~~[Adaptive frame rate](128-adaptive-frame-rate.md)~~ | ~~XS~~ | ~~7~~ | **Done** (2026-08-27) |
| ~~129~~ | ~~[Metric exponential smoothing](129-metric-exponential-smoothing.md)~~ | ~~XS~~ | ~~7~~ | **Done** (2026-08-27) |
| 130 | [Tab content badges](130-tab-content-badges.md) | XS | 7 |
| 148 | [TUI god objects decomposition](148-tui-god-objects-decomposition.md) | L | 6 |
| ~~150~~ | ~~[Cybernetic feature wire-or-remove audit](archive/150-cybernetic-feature-wire-or-remove-audit.md)~~ | ~~M~~ | ~~6~~ | **Superseded by engine audit/#242-#285** |
| 153 | [Automated Visual Assessment Loop](153-automated-visual-assessment-loop.md) | L | 7 |
| 154 | [HTTP Monitoring Workflow Documentation](154-http-monitoring-workflow-documentation.md) | XS | 7 |
| ~~155~~ | ~~[HDC Math Dead Code Archival](155-hdc-math-dead-code-archival.md)~~ | ~~XS~~ | **Done** (2026-08-26) | 6 |
| ~~161~~ | ~~[Codex Cost Reporting](161-codex-cost-reporting.md)~~ | ~~XS~~ | ~~7~~ | **Done** (2026-08-24) |
| 179 | [Batch controller](179-batch-controller.md) | XS | 7 |
| 183 | [Crash report file](183-crash-report-file.md) | S | 7 |
| 184 | [Artifact freshness checking](184-artifact-freshness-checking.md) | XS | 7 |
| 185 | [MCP result caching](185-mcp-result-caching.md) | XS | 7 |
| 186 | [MCP token savings tracking](186-mcp-token-savings-tracking.md) | XS | 7 |
| 190 | [CorticalState wiring](190-corticalstate-wiring.md) | M | 6 |
| ~~191~~ | ~~[EnrichedCell dispatch](archive/191-enriched-cell-dispatch.md)~~ | ~~M~~ | ~~6~~ | **Superseded by #253/#265/#268-#270** |
| 192 | [Native agent telemetry](192-native-agent-telemetry.md) | L | 7 |
| ~~195~~ | ~~[File overlap analysis](195-file-overlap-analysis.md)~~ | ~~XS~~ | ~~7~~ | **Done** (2026-08-31) |
| 196 | [Critical path ETA](196-critical-path-eta.md) | XS | 7 |
| 198 | [Supervisor auto-recovery](198-supervisor-auto-recovery.md) | S | 7 |
| 199 | [TUI resizable panes](199-tui-resizable-panes.md) | S | 7 |
| 216 | [TUI queue overview modal](216-tui-queue-overview-modal.md) | S | 7 |

---

## Dependency Graph

Items that must be completed before another item can begin. Dependencies are strict (blocked)
unless marked "soft" (benefits from but doesn't require predecessor).

```
# Wave 0 → Wave 1: No hard dependencies; Wave 1 items are independent security fixes.

# Wave 1 → Wave 2
96 (API key timing-safe) → 48 (serve auth default)  [soft: both are auth hardening]
133 (skip_permissions opt-in) → 136 (safety denial audit)  [soft]

# Wave 2 → Wave 3
132 (orchestrate.rs freeze) → 131 (PRD/cloud-worker to runner-v2)
273 (neutral mutation kernel) → 252 (graph replan controller)
251 + lifecycle extensions → 282; 256 + 257 + 282 → 284 → 138

# Wave 3 → Wave 4
107 (plan run UX friction) → 108 (TUI live feedback gaps)
108 (TUI live feedback) → 121 (TUI data model unification)  [remaining model consolidation]
121 (data model unification) → 122 (remove legacy page system)
121 (data model unification) → 124 (header bar Mori parity)

# Wave 4 → Wave 5
116 (queue manifest) → 117 (plan-level wave computation)
117 (wave computation) → 125 (plan tree wave hierarchy widget)
83 (dream consolidation deadlock fix) → 143 (dream consolidation trigger)
111 (screenshot command) → 151 (TUI PNG snapshot rendering)
151 (PNG snapshot) → 152 (screenshot diff/compare engine)
127 (F7 inspect view parity) → 156 (per-model cost stats TUI)  [soft]
233 (executor-neutral command channel) → 255 (graph control/cancellation)
111 + 112 + 151 + 152 → 153 (automated visual assessment loop)

# Wave 3 new items
212 (run_id in events) → 215 (HTTP run-scoped event query)
208 (canonical events) → 233 (executor-neutral TUI command channel)
232 (TUI connected-mode data bridge) → 234 (gate output streaming to TUI)  [soft: bridge provides the streaming path]

# Wave 4 new items
201 (TUI notification toasts) → 178 (conductor supervisor loop)  [soft: conductor produces events for toasts]
232 (TUI data bridge) → 238 (plan detail enrichment)  [soft: enriched data needs the bridge to display]
124 (header bar Mori parity) → 239 (header bar enrichment)

# Wave 5 new items
116 (queue manifest) → 200 (plan validate --dag)
117 (wave computation) → 196 (critical path ETA)
183 (crash report) → 198 (supervisor auto-recovery)
116 (queue manifest) → 216 (TUI queue overview modal)

# Engine convergence
The engine program has a large packet-level DAG whose exact edges live in each packet header.
See tmp/engine-audit/IMPLEMENTATION-ROADMAP.md for the authoritative graph, ready queue,
path leases, integration nodes, and release-window gate. Do not infer strict edges from ranges.
```

---

## Removed Items (already implemented)

The following items from the original backlog are fully implemented and have been removed
from the active index:

| # | Title | Status |
|---|---|---|
| 06 | Output Budgeting | Implemented in `crates/roko-gateway/src/output_budget.rs` |
| 07 | Inference Cache L1/L2 | Implemented in `crates/roko-gateway/src/cache.rs` |
| 08 | Key Rotation | Implemented in `crates/roko-gateway/src/provider.rs` |
| 36 | Atomic File I/O | Implemented in `crates/roko-fs/src/atomic.rs`; all runner/learn persistence paths use `atomic_write` |
| 11 | Justfile | Created by Codex CLI via `roko plan run` during 2026-08-22 dogfood. Commit `0cadafdf4` on `roko/attempt/attempt-25210c0d6aff0a2102ab` |
| 48 | Serve Auth Default Posture | PR #54, plan `serve-auth-default` |
| 75 | Graph Example Schema Drift | Previously implemented in PR #54; **regressed and reopened** on 2026-08-31 |
| 86 | Gate Compile Tool Bypass | PR #54, plan `gate-compile-fail-closed` |
| 87 | Task Parser Duplicate IDs | PR #54, plan `task-parser-duplicate-ids` |
| 88 | Handler Future Lifecycle Panic | PR #54, plan `handler-future-double-poll` |
| 89 | Rate Limiter Mutex Poisoning | PR #54, plan `rate-limiter-poison-recovery` |
| 96 | API Key Timing-Safe Comparison | Plan `api-key-timing-safe` |
| 141 | Per-turn efficiency events | PR #56, plan `per-turn-efficiency-events` |
| 159 | Plan Run Report Phase Latency | PR #54, plan `exit-code-report-separation` |
| 160 | --fresh Worktree/Branch Cleanup | PR #55, plan `fresh-branch-cleanup-prefix` |
| 161 | Codex Cost Reporting | PR #56, plan `codex-cost-reporting` |
| 49 | Serve CORS Restrictive | PR #57, plan `serve-cors-restrictive` |
| 133 | dangerously_skip_permissions opt-in | PR #57, plan `skip-permissions-opt-in` |
| 50 | Serve Rate and Body Limits | PR #58, plan `serve-rate-body-limits` |
| 51 | Serve Agent Name Validation | PR #58, plan `serve-agent-name-validation` |
| 33 | CLI Gist Scrubbing | PR #59, plan `cli-gist-scrubbing` |
| 78 | Efficiency gate_passed Bug | PR #59, plan `efficiency-gate-passed-fix` |
| 81 | Layer-Check False Positives | PR #60, plan `layer-check-false-positives` |
| 97 | Snapshot Backup & Staging Cleanup | PR #60, plan `snapshot-backup-staging` |
| 19 | Contextual Bandit Dead Code | Already removed; confirmed via `bandit-dead-code` plan run |
| 155 | HDC Math Dead Code Archival | Plan `hdc-math-dead-code`, feature-gated 5 modules |
| 137 | Router + threshold state in crash snapshot | Plan `gate-threshold-crash-recovery` |
| 114 | Diagnose command enrichment | Plan `diagnose-enrichment` |
| 128 | Adaptive frame rate | Plan `tui-adaptive-framerate` |
| 129 | Metric exponential smoothing | Plan `tui-metric-smoothing` |
| 34 | PRD Cascade Learning | PR #66, runner-dispatched |
| 42 | Duplicate Type Consolidation | PR #68, two-task runner plan |
| 82 | Graph Stub Cell Warnings | PR #64; `Cell::is_stub` and CLI warnings verified in current code |
| 83 | Dream Consolidation Deadlock | PR #71; `spawn_blocking` replaced by async trigger path |
| 91 | Experiment Assignment Stability | PR #69, runner-dispatched |
| 95 | Config Loader Robustness | PR #64, config safety batch |
| 115 | Structured Log Wire Verification | PR #64; flushed `--log-file` hooks and tests present |
| 136 | Safety Denial Audit Events | PR #67, runner-dispatched |
| 143 | Dream Consolidation Trigger | PR #72, runner-dispatched |
| 144 | Daimon Affect Persistence | Runner verification found persistence already wired; no code change required |
| 149 | Full Prompt Text Logging | PR #70; opt-in bounded prompt logs |
| 163 | Shared Build Cache for Worktrees | Dogfood infrastructure fix; archived spec |
| 164 | SchedulerNoProgress Timeout Too Short | Dogfood infrastructure fix; archived spec |
| 77 | CLI UX Consistency | PR #73, UX/TUI/Workflow Mori Parity batch |
| 79 | Doctor/Onboarding Diagnostics | PR #73, UX/TUI/Workflow Mori Parity batch |
| 107 | Plan Run UX Friction | PR #73, core items wired |
| 108 | TUI Live Feedback Gaps | PR #73, 5/6 sub-items wired |
| 113 | CLI JSON output mode | PR #73, UX/TUI/Workflow Mori Parity batch |
| 116 | Queue manifest | PR #73, UX/TUI/Workflow Mori Parity batch |
| 117 | Plan-level wave computation | PR #73, UX/TUI/Workflow Mori Parity batch |
| 119 | TUI recovery keybindings | Historical keybinding portion implemented; residual work moved to #233/#255 |
| 120 | Plan run preflight checks | PR #73, UX/TUI/Workflow Mori Parity batch |
| 123 | ROSEDUST palette port | PR #73, UX/TUI/Workflow Mori Parity batch |
| 124 | Header bar Mori parity | PR #73, UX/TUI/Workflow Mori Parity batch |
| 125 | Plan tree wave hierarchy widget | PR #73, UX/TUI/Workflow Mori Parity batch |
| 126 | Error digest widget | PR #73, UX/TUI/Workflow Mori Parity batch |
| 127 | F7 inspect view parity | PR #73, UX/TUI/Workflow Mori Parity batch |
| 131 | PRD/cloud-worker migration to runner-v2 | PR #73, UX/TUI/Workflow Mori Parity batch |
| 140 | Merge success/conflict proof | PR #73, UX/TUI/Workflow Mori Parity batch |
| 146 | Plan CLI control commands | PR #73, UX/TUI/Workflow Mori Parity batch |
| 147 | Backlog import command | PR #73, UX/TUI/Workflow Mori Parity batch |
| 156 | Per-Model/Provider Cost Stats TUI | PR #73, UX/TUI/Workflow Mori Parity batch |
| 157 | Context-Sensitive Keybind Hints | PR #73, UX/TUI/Workflow Mori Parity batch |
| 182 | Lightweight status file | PR #73, UX/TUI/Workflow Mori Parity batch |
| 189 | Agent status panel | PR #73, UX/TUI/Workflow Mori Parity batch |
| 195 | File overlap analysis | PR #73, UX/TUI/Workflow Mori Parity batch |
| 200 | Plan validate --dag | PR #73, UX/TUI/Workflow Mori Parity batch |
| 201 | TUI notification toasts | PR #73, UX/TUI/Workflow Mori Parity batch |
| 206 | CARGO_BUILD_JOBS limit | PR #73, UX/TUI/Workflow Mori Parity batch |
| 212 | run_id in snapshots and events | PR #73, UX/TUI/Workflow Mori Parity batch |
| 226 | Plan generate lock scope reduction | PR #73, UX/TUI/Workflow Mori Parity batch |
| 227 | --from-backlog plan generation | PR #73, UX/TUI/Workflow Mori Parity batch |
| 10 | Daimon TUI View | PR #73, UX/TUI/Workflow Mori Parity batch |
| 41 | TUI Push-Mode Panel Data | PR #73, UX/TUI/Workflow Mori Parity batch |
| 65 | CLI Verb Consolidation | PR #73, UX/TUI/Workflow Mori Parity batch |
| 71 | TUI Design System Alignment | PR #73, UX/TUI/Workflow Mori Parity batch |
| 85 | Plan Generation TOML Reliability | PR #73, UX/TUI/Workflow Mori Parity batch |
| 112 | Plan run continuous screenshots | PR #73, UX/TUI/Workflow Mori Parity batch |
| 222 | Provider configuration UX | PR #73, UX/TUI/Workflow Mori Parity batch |

---

## Status Notes

**Development-speed reconciliation (2026-08-31):** #215 is complete for newly indexed runs, with
offline historical repair deliberately separate. #228's schema-v2 development harness and #286's
hard-deadline/salvage/settlement source paths are complete, but their named final live/kill-point
fixtures remain unchecked. #58 and #230 are source-complete but retain final compile/test/clippy or
benchmark evidence. #231 remains partial because conservative syntax/Cargo-graph impact selection
does not replace a complete semantic, macro-aware, cross-language call-site index. The tracked
implementation ledger is `tmp/dev-audit/11-implementation-status.md`.

**Partial implementations (code exists but is not fully wired):**

- Item **04** (Compile Auto-Fix): types exist in `roko-gate/` but the final wiring pieces
  are incomplete. The spec details exactly what exists vs. what is missing.

- Item **05** (Express Mode): scaffolding exists; final wiring incomplete.

- Item **15** (Post-Gate Reflection): store, dedup, and injection scaffolding exist, but the
  actual LLM call is replaced by deterministic pattern synthesis.

- Item **16** (Warm Agent Spawning): `WarmPool` container and integration points exist, but
  inserts placeholder structs instead of real pre-spawned processes.

- Item **46** (ACP Test Coverage): Gap 1 (stdin-EOF clean exit) is resolved; Gaps 2-3 (MCP
  crash surfacing, cross-provider tool matrix) remain open.

**Items 111-150** originated in the 2026-08-19 synthesis pass. Their historical specs remain
available, but completed/superseded items may now live under `tmp/backlog/archive/`; filesystem
location and each file's status banner are authoritative.

**New items 178-221** were added 2026-08-29 from the mori→roko cross-reference audit
(28 parallel agents cross-referenced `tmp/mori-diffs/` (40 audit files), `tmp/mori-old/`
(17 comparison docs + synthesis), and the existing backlog). These 44 new specs cover:
- 25 MO-prefixed items from `_mori-old-gaps.md` that were never promoted to numbered specs
- 8 items from `_mori-diffs-gaps.md` that had number collisions (renumbered as 208-215)
- 11 additional TUI/UX/observability gaps found during cross-reference with zero prior coverage

**PR #73 UX/TUI/Workflow Mori Parity (2026-08-31):** 37 of 53 checklist items implemented across 10 batches. 107 files changed (+10,186/-1,094). Remaining items are tracked in `tmp/tui-parity/`.

**Size legend:**

| Code | Meaning |
|---|---|
| XS | Under 2 hours — single focused change |
| S | Half-day to 1 day |
| M | 2-3 days |
| L | 3-7 days |
| XL | More than 1 week; consider splitting |

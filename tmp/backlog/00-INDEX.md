# Backlog — Master Implementation Index

> **What is this?** Self-contained backlog items for the roko workspace. Each doc
> specifies a problem, what already exists, what needs building, where to put it,
> and how to verify it.
>
> **How to use:** Pick an item, read the spec, implement it. Update `.roko/GAPS.md` when done.
>
> Last reviewed: 2026-08-19 (150 specs, reorganized from 3 synthesis audits + 100 original items)

---

## Implementation Order

Items are grouped into waves by dependency and impact. Complete earlier waves first; later
waves contain items that either depend on earlier ones or are lower risk to defer.

### Wave 0: Critical Fixes (do first — crashes, data loss, security)

These are correctness failures that silently produce wrong output or crash the system.

| # | Title | Size |
|---|---|---|
| 75 | [Graph Example Schema Drift](75-graph-example-schema-drift.md) | S |
| 78 | [Efficiency gate_passed Bug](78-efficiency-gate-passed-bug.md) | S |
| 86 | [Gate Compile Tool Bypass](86-gate-compile-tool-bypass.md) | XS |
| 87 | [Task Parser Duplicate IDs](87-task-parser-duplicate-ids.md) | S |
| 17 | [ACP Stability Hardening](17-acp-stability-hardening.md) | L |

### Wave 1: Safety & Security Hardening

Security posture items that should be resolved before any public-facing deployment.

| # | Title | Size |
|---|---|---|
| 48 | [Serve Auth Default Posture](48-serve-auth-default.md) | S |
| 49 | [Serve CORS Restrictive](49-serve-cors-restrictive.md) | S |
| 50 | [Serve Rate and Body Limits](50-serve-rate-body-limits.md) | S |
| 51 | [Serve Agent Name Validation](51-serve-path-traversal.md) | S |
| 96 | [API Key Timing-Safe Comparison](96-serve-api-key-timing-safe.md) | XS |
| 60 | [Safety Dispatch Hardening](60-safety-dispatch-hardening.md) | M |
| 33 | [CLI Gist Scrubbing](33-cli-gist-scrubbing.md) | S |
| 133 | [dangerously_skip_permissions opt-in](133-dangerously-skip-permissions-opt-in.md) | XS |
| 136 | [Safety denial audit events](136-safety-denial-audit-events.md) | S |

### Wave 2: Core Runtime Correctness

P1 items fixing wrong behavior in the primary execution path.

| # | Title | Size |
|---|---|---|
| 88 | [Handler Future Lifecycle Panic](88-handler-future-lifecycle-panic.md) | XS |
| 89 | [Rate Limiter Mutex Poisoning](89-rate-limiter-mutex-poisoning.md) | XS |
| 90 | [UX34 Override Learning Isolation](90-ux34-override-learning-isolation.md) | M |
| 95 | [Config Loader Robustness](95-config-loader-robustness.md) | S |
| 97 | [Snapshot Backup & Staging Cleanup](97-snapshot-backup-and-staging-cleanup.md) | S |
| 101 | [Async Runtime Anti-Patterns](101-async-runtime-anti-patterns.md) | M |
| 131 | [PRD/cloud-worker migration to runner-v2](131-prd-cloud-worker-runner-v2-migration.md) | S |
| 132 | [orchestrate.rs freeze/retirement](132-orchestrate-rs-freeze-retirement.md) | M |
| 134 | [Replan-on-gate-failure in runner-v2](134-replan-on-gate-failure-runner-v2.md) | M |
| 135 | [Adaptive gate thresholds in runner-v2](135-adaptive-gate-thresholds-runner-v2.md) | S |
| 137 | [Router + threshold state in crash snapshot](137-router-threshold-state-crash-snapshot.md) | XS |
| 138 | [Crash/resume proof matrix](138-crash-resume-proof-matrix.md) | S |

### Wave 3: Execution UX Foundation

Items that make self-hosting reliable and observable for the first time.

| # | Title | Size |
|---|---|---|
| 107 | [Plan Run UX Friction](107-plan-run-ux-friction.md) | M |
| 108 | [TUI Live Feedback Gaps](108-tui-live-feedback-gaps.md) | L |
| 119 | [TUI recovery keybindings](119-tui-recovery-keybindings.md) | M |
| 120 | [Plan run preflight checks](120-plan-run-preflight-checks.md) | S |
| 03 | [Context Injection Scoping](03-context-injection-scoping.md) | M |
| 04 | [Compile Auto-Fix Path](04-compile-autofix-path.md) | S |
| 77 | [CLI UX Consistency](77-cli-ux-consistency.md) | S |
| 79 | [Doctor/Onboarding Diagnostics](79-doctor-onboarding-diagnostics.md) | M |
| 93 | [Plugin Runtime Verification](93-plugin-runtime-verification.md) | S |
| 111 | [Screenshot command completion](111-screenshot-command-completion.md) | S |
| 113 | [CLI JSON output mode](113-cli-json-output-mode.md) | M |
| 114 | [Diagnose command enrichment](114-diagnose-command-enrich.md) | XS |
| 115 | [Structured log wire verification](115-structured-log-wire-verification.md) | XS |
| 146 | [Plan CLI control commands](146-plan-cli-control-commands.md) | M |

### Wave 4: TUI Quality

Improvements to the TUI that make monitoring and debugging sessions productive.

| # | Title | Size |
|---|---|---|
| 109 | [TUI Real-Time Streaming Parity](109-tui-realtime-streaming-parity.md) | XL |
| 121 | [TUI data model unification](121-tui-data-model-unification.md) | L |
| 122 | [Remove legacy page system](122-remove-legacy-page-system.md) | S |
| 123 | [ROSEDUST palette port](123-rosedust-palette.md) | S |
| 124 | [Header bar Mori parity](124-header-bar-mori-parity.md) | M |
| 125 | [Plan tree wave hierarchy widget](125-plan-tree-wave-hierarchy-widget.md) | M |
| 126 | [Error digest widget](126-error-digest-widget.md) | S |
| 127 | [F7 inspect view parity](127-inspect-view-f7-parity.md) | M |
| 10 | [Daimon TUI View](10-daimon-tui-view.md) | S |
| 112 | [Plan run continuous screenshots](112-plan-run-continuous-screenshots.md) | M |
| 141 | [Per-turn efficiency events](141-per-turn-efficiency-events.md) | XS |
| 149 | [Full prompt text logging](149-full-prompt-text-logging.md) | XS |

### Wave 5: Learning & Knowledge Loop Closure

Items that close feedback loops so the system actually improves from runs.

| # | Title | Size |
|---|---|---|
| 116 | [Queue manifest](116-queue-manifest.md) | L |
| 117 | [Plan-level wave computation](117-plan-wave-computation.md) | M |
| 118 | [Express mode auto-fix](118-express-mode-autofix.md) | M |
| 05 | [Express Mode](05-express-mode.md) | M |
| 14 | [Plan Mutation Protocol](14-plan-mutation-protocol.md) | M |
| 15 | [Post-Gate Reflection](15-post-gate-reflection.md) | M |
| 34 | [PRD Cascade Learning](34-prd-cascade-learning.md) | S |
| 80 | [Learning Subsystem Data Quality](80-learning-subsystem-data-quality.md) | M |
| 84 | [Cascade Router Task Category Awareness](84-cascade-router-task-category-awareness.md) | M |
| 90 | [UX34 Override Learning Isolation](90-ux34-override-learning-isolation.md) | M |
| 91 | [Experiment Assignment Stability](91-experiment-assignment-stability.md) | S |
| 92 | [Hindsight Causal Inference](92-hindsight-causal-inference.md) | M |
| 139 | [Per-plan agent-handle map for concurrency](139-per-plan-agent-handle-map.md) | M |
| 140 | [Merge success/conflict proof](140-merge-success-conflict-proof.md) | S |
| 142 | [Knowledge write-back proof](142-knowledge-write-back-proof.md) | S |
| 143 | [Dream consolidation trigger](143-dream-consolidation-trigger.md) | S |
| 144 | [Daimon affect persistence](144-daimon-affect-persistence-metadata.md) | S |
| 145 | [Prompt section effectiveness proof](145-prompt-section-effectiveness-loop-proof.md) | S |

### Wave 6: Architecture Cleanup

Decomposition, consolidation, and dead code removal. Safe to do in parallel with Wave 5.

| # | Title | Size |
|---|---|---|
| 43 | [Clippy Suppression Removal](43-clippy-suppression-removal.md) | M |
| 47 | [ConfigLayer Elimination](47-configlayer-elimination.md) | L |
| 61 | [Agent Dispatch Consolidation](61-dispatch-consolidation.md) | XL |
| 102 | [Cross-Crate Code Duplication](102-cross-crate-code-duplication.md) | M |
| 105 | [HTTP API Design Consistency](105-http-api-design-consistency.md) | M |
| 106 | [Memory Allocation Hot Paths](106-memory-allocation-hot-paths.md) | M |
| 110 | [Deprecate JSONL / StateHub-only](110-deprecate-jsonl-statehub-only.md) | XL |
| 147 | [Backlog import command](147-backlog-import-command.md) | M |
| 148 | [TUI god objects decomposition](148-tui-god-objects-decomposition.md) | L |
| 150 | [Cybernetic feature wire-or-remove audit](150-cybernetic-feature-wire-or-remove-audit.md) | M |
| 19 | [Contextual Bandit Dead Code](19-contextual-bandit-dead-code.md) | XS |
| 42 | [Duplicate Type Consolidation](42-duplicate-type-consolidation.md) | S |

### Wave 7: Nice-to-Have / Future

P3 items and Phase 2+ features. Do last or defer indefinitely.

| # | Title | Size |
|---|---|---|
| 09 | [Recursive Safety Patterns](09-recursive-safety-patterns.md) | L |
| 11 | [Justfile](11-justfile.md) | XS |
| 16 | [Warm Agent Spawning](16-warm-agent-spawning.md) | M |
| 41 | [TUI Push-Mode Panel Data](41-tui-push-mode-panel-data.md) | S |
| 59 | [HuggingFace Provider](59-huggingface-provider.md) | S |
| 63 | [Zero-Config Onboarding Wizard](63-zero-config-onboarding.md) | M |
| 64 | [Model Discovery UX](64-model-discovery-ux.md) | S |
| 65 | [CLI Verb Consolidation](65-cli-verb-consolidation.md) | L |
| 66 | [Context Sources & Editor Integration](66-context-editor-integration.md) | L |
| 71 | [TUI Design System Alignment](71-tui-design-alignment.md) | S |
| 73 | [UX Backlog Rollup](73-ux-backlog-rollup.md) | M |
| 74 | [Unified Evaluation Framework](74-evaluation-framework.md) | XL |
| 99 | [Examples & Documentation Gaps](99-examples-documentation-gaps.md) | M |
| 128 | [Adaptive frame rate](128-adaptive-frame-rate.md) | XS |
| 129 | [Metric exponential smoothing](129-metric-exponential-smoothing.md) | XS |
| 130 | [Tab content badges](130-tab-content-badges.md) | XS |

---

## Full Priority Index

### P0 — Critical

Crashes, silent data loss, or correctness failures that corrupt the learning pipeline.

| # | Title | Size | Wave |
|---|---|---|---|
| 17 | [ACP Stability Hardening](17-acp-stability-hardening.md) | L | 0 |
| 75 | [Graph Example Schema Drift](75-graph-example-schema-drift.md) | S | 0 |
| 78 | [Efficiency gate_passed Bug](78-efficiency-gate-passed-bug.md) | S | 0 |
| 86 | [Gate Compile Tool Bypass](86-gate-compile-tool-bypass.md) | XS | 0 |
| 87 | [Task Parser Duplicate IDs](87-task-parser-duplicate-ids.md) | S | 0 |

### P1 — High

Direct cost, quality, or reliability impact; blocking the self-hosting loop.

| # | Title | Size | Wave |
|---|---|---|---|
| 03 | [Context Injection Scoping](03-context-injection-scoping.md) | M | 3 |
| 04 | [Compile Auto-Fix Path](04-compile-autofix-path.md) | S | 3 |
| 18 | [ACP Spec Upgrade & Refactor](18-acp-spec-upgrade-and-refactor.md) | XL | 6 |
| 21 | [Landing Page Fake Metrics](21-landing-page-fake-metrics.md) | S | 3 |
| 33 | [CLI Gist Scrubbing](33-cli-gist-scrubbing.md) | S | 1 |
| 45 | [ACP Tool Permission Gate](45-acp-tool-permission-gate.md) | M | 2 |
| 48 | [Serve Auth Default Posture](48-serve-auth-default.md) | S | 1 |
| 49 | [Serve CORS Restrictive](49-serve-cors-restrictive.md) | S | 1 |
| 50 | [Serve Rate and Body Limits](50-serve-rate-body-limits.md) | S | 1 |
| 51 | [Serve Agent Name Validation](51-serve-path-traversal.md) | S | 1 |
| 56 | [ACP Single-Agent Chat: Tools Require Client Capability Declaration](56-acp-single-agent-tools.md) | M | 2 |
| 60 | [Safety Dispatch Hardening](60-safety-dispatch-hardening.md) | M | 1 |
| 76 | [Example Config Quality](76-example-config-quality.md) | M | 3 |
| 77 | [CLI UX Consistency](77-cli-ux-consistency.md) | S | 3 |
| 79 | [Doctor/Onboarding Diagnostics](79-doctor-onboarding-diagnostics.md) | M | 3 |
| 88 | [Handler Future Lifecycle Panic](88-handler-future-lifecycle-panic.md) | XS | 2 |
| 89 | [Rate Limiter Mutex Poisoning](89-rate-limiter-mutex-poisoning.md) | XS | 2 |
| 90 | [UX34 Override Learning Isolation](90-ux34-override-learning-isolation.md) | M | 2 |
| 93 | [Plugin Runtime Verification](93-plugin-runtime-verification.md) | S | 3 |
| 95 | [Config Loader Robustness](95-config-loader-robustness.md) | S | 2 |
| 96 | [API Key Timing-Safe Comparison](96-serve-api-key-timing-safe.md) | XS | 1 |
| 97 | [Snapshot Backup & Staging Cleanup](97-snapshot-backup-and-staging-cleanup.md) | S | 2 |
| 101 | [Async Runtime Anti-Patterns](101-async-runtime-anti-patterns.md) | M | 2 |
| 107 | [Plan Run UX Friction](107-plan-run-ux-friction.md) | M | 3 |
| 108 | [TUI Live Feedback Gaps](108-tui-live-feedback-gaps.md) | L | 3 |
| 109 | [TUI Real-Time Streaming Parity](109-tui-realtime-streaming-parity.md) | XL | 4 |
| 110 | [Deprecate JSONL / StateHub-only](110-deprecate-jsonl-statehub-only.md) | XL | 6 |
| 111 | [Screenshot command completion](111-screenshot-command-completion.md) | S | 3 |
| 113 | [CLI JSON output mode](113-cli-json-output-mode.md) | M | 3 |
| 119 | [TUI recovery keybindings](119-tui-recovery-keybindings.md) | M | 3 |
| 120 | [Plan run preflight checks](120-plan-run-preflight-checks.md) | S | 3 |
| 131 | [PRD/cloud-worker migration to runner-v2](131-prd-cloud-worker-runner-v2-migration.md) | S | 2 |
| 132 | [orchestrate.rs freeze/retirement](132-orchestrate-rs-freeze-retirement.md) | M | 2 |
| 133 | [dangerously_skip_permissions opt-in](133-dangerously-skip-permissions-opt-in.md) | XS | 1 |
| 134 | [Replan-on-gate-failure in runner-v2](134-replan-on-gate-failure-runner-v2.md) | M | 2 |
| 135 | [Adaptive gate thresholds in runner-v2](135-adaptive-gate-thresholds-runner-v2.md) | S | 2 |
| 136 | [Safety denial audit events](136-safety-denial-audit-events.md) | S | 1 |
| 137 | [Router + threshold state in crash snapshot](137-router-threshold-state-crash-snapshot.md) | XS | 2 |
| 138 | [Crash/resume proof matrix](138-crash-resume-proof-matrix.md) | S | 2 |
| 146 | [Plan CLI control commands](146-plan-cli-control-commands.md) | M | 3 |

### P2 — Medium

Efficiency, UX quality, maintainability, and feedback loop closure.

| # | Title | Size | Wave |
|---|---|---|---|
| 01 | [T0 Reflex Store](01-t0-reflex-store.md) | M | 5 |
| 02 | [Reactive Agent Mode](02-reactive-agent-mode.md) | L | 7 |
| 05 | [Express Mode](05-express-mode.md) | M | 5 |
| 10 | [Daimon TUI View](10-daimon-tui-view.md) | S | 4 |
| 12 | [E2E Test Harness](12-e2e-test-harness.md) | M | 3 |
| 13 | [Historical Cost Calibration](13-historical-cost-calibration.md) | S | 5 |
| 14 | [Plan Mutation Protocol](14-plan-mutation-protocol.md) | M | 5 |
| 15 | [Post-Gate Reflection](15-post-gate-reflection.md) | M | 5 |
| 20 | [Event Loop Decomposition](20-event-loop-decomposition.md) | XL | 6 |
| 22 | [Chat Inline Decomposition](22-chat-inline-decomposition.md) | M | 6 |
| 34 | [PRD Cascade Learning](34-prd-cascade-learning.md) | S | 5 |
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
| 54 | [Graph Engine Runner-v2 Parity](54-graph-engine-runner-parity.md) | XL | 6 |
| 55 | [AgentPool Runtime Integration](55-agent-pool-runtime-integration.md) | M | 5 |
| 57 | [Plan Generation Escalation](57-plan-generation-escalation.md) | S | 5 |
| 58 | [Performance Hot-Path Fixes](58-perf-hot-path-fixes.md) | M | 6 |
| 61 | [Agent Dispatch Consolidation](61-dispatch-consolidation.md) | XL | 6 |
| 62 | [Relay Topic Namespace Migration](62-relay-topic-migration.md) | M | 6 |
| 67 | [HDC Prompt Assembly Wiring](67-hdc-prompt-assembly.md) | M | 5 |
| 68 | [Budget Pre-Dispatch Admission](68-budget-pre-dispatch.md) | S | 5 |
| 69 | [SSE Parsing Deduplication](69-sse-parsing-dedup.md) | S | 3 |
| 70 | [ACP Novel Workflow Gaps](70-acp-workflow-gaps.md) | M | 5 |
| 72 | [Pool Architecture Reconciliation](72-pool-architecture-reconciliation.md) | S | 6 |
| 80 | [Learning Subsystem Data Quality](80-learning-subsystem-data-quality.md) | M | 5 |
| 81 | [Layer-Check False Positives](81-layer-check-false-positives.md) | S | 2 |
| 82 | [Graph Stub Cell Warnings](82-graph-stub-cell-warnings.md) | S | 6 |
| 83 | [Dream Consolidation Deadlock](83-dream-consolidation-deadlock.md) | S | 5 |
| 84 | [Cascade Router Task Category Awareness](84-cascade-router-task-category-awareness.md) | M | 5 |
| 85 | [Plan Generation TOML Reliability](85-plan-generation-toml-reliability.md) | S | 3 |
| 91 | [Experiment Assignment Stability](91-experiment-assignment-stability.md) | S | 5 |
| 92 | [Hindsight Causal Inference](92-hindsight-causal-inference.md) | M | 5 |
| 94 | [ProcessSupervisor Untracked Tasks](94-process-supervisor-untracked-tasks.md) | S | 2 |
| 98 | [GitHub Workflow Robustness](98-github-workflow-robustness.md) | M | 3 |
| 100 | [CLI Error Message Quality](100-cli-error-message-quality.md) | S | 3 |
| 102 | [Cross-Crate Code Duplication](102-cross-crate-code-duplication.md) | M | 6 |
| 103 | [Plan Execution Resilience](103-plan-execution-resilience.md) | M | 3 |
| 104 | [Doctor Diagnostic Coverage](104-doctor-diagnostic-coverage.md) | M | 3 |
| 105 | [HTTP API Design Consistency](105-http-api-design-consistency.md) | M | 6 |
| 106 | [Memory Allocation Hot Paths](106-memory-allocation-hot-paths.md) | M | 6 |
| 112 | [Plan run continuous screenshots](112-plan-run-continuous-screenshots.md) | M | 4 |
| 114 | [Diagnose command enrichment](114-diagnose-command-enrich.md) | XS | 3 |
| 115 | [Structured log wire verification](115-structured-log-wire-verification.md) | XS | 3 |
| 116 | [Queue manifest](116-queue-manifest.md) | L | 5 |
| 117 | [Plan-level wave computation](117-plan-wave-computation.md) | M | 5 |
| 118 | [Express mode auto-fix](118-express-mode-autofix.md) | M | 5 |
| 121 | [TUI data model unification](121-tui-data-model-unification.md) | L | 4 |
| 122 | [Remove legacy page system](122-remove-legacy-page-system.md) | S | 4 |
| 123 | [ROSEDUST palette port](123-rosedust-palette.md) | S | 4 |
| 124 | [Header bar Mori parity](124-header-bar-mori-parity.md) | M | 4 |
| 125 | [Plan tree wave hierarchy widget](125-plan-tree-wave-hierarchy-widget.md) | M | 4 |
| 126 | [Error digest widget](126-error-digest-widget.md) | S | 4 |
| 127 | [F7 inspect view parity](127-inspect-view-f7-parity.md) | M | 4 |
| 139 | [Per-plan agent-handle map for concurrency](139-per-plan-agent-handle-map.md) | M | 5 |
| 140 | [Merge success/conflict proof](140-merge-success-conflict-proof.md) | S | 5 |
| 141 | [Per-turn efficiency events](141-per-turn-efficiency-events.md) | XS | 4 |
| 142 | [Knowledge write-back proof](142-knowledge-write-back-proof.md) | S | 5 |
| 143 | [Dream consolidation trigger](143-dream-consolidation-trigger.md) | S | 5 |
| 144 | [Daimon affect persistence](144-daimon-affect-persistence-metadata.md) | S | 5 |
| 145 | [Prompt section effectiveness proof](145-prompt-section-effectiveness-loop-proof.md) | S | 5 |
| 147 | [Backlog import command](147-backlog-import-command.md) | M | 6 |
| 149 | [Full prompt text logging](149-full-prompt-text-logging.md) | XS | 4 |

### P3 — Low

Nice-to-have features, Phase 2+ scope, or speculative improvements.

| # | Title | Size | Wave |
|---|---|---|---|
| 09 | [Recursive Safety Patterns](09-recursive-safety-patterns.md) | L | 7 |
| 11 | [Justfile](11-justfile.md) | XS | 7 |
| 16 | [Warm Agent Spawning](16-warm-agent-spawning.md) | M | 7 |
| 19 | [Contextual Bandit Dead Code](19-contextual-bandit-dead-code.md) | XS | 6 |
| 41 | [TUI Push-Mode Panel Data](41-tui-push-mode-panel-data.md) | S | 7 |
| 42 | [Duplicate Type Consolidation](42-duplicate-type-consolidation.md) | S | 6 |
| 59 | [HuggingFace Provider](59-huggingface-provider.md) | S | 7 |
| 63 | [Zero-Config Onboarding Wizard](63-zero-config-onboarding.md) | M | 7 |
| 64 | [Model Discovery UX](64-model-discovery-ux.md) | S | 7 |
| 65 | [CLI Verb Consolidation](65-cli-verb-consolidation.md) | L | 7 |
| 66 | [Context Sources & Editor Integration](66-context-editor-integration.md) | L | 7 |
| 71 | [TUI Design System Alignment](71-tui-design-alignment.md) | S | 7 |
| 73 | [UX Backlog Rollup](73-ux-backlog-rollup.md) | M | 7 |
| 74 | [Unified Evaluation Framework](74-evaluation-framework.md) | XL | 7 |
| 99 | [Examples & Documentation Gaps](99-examples-documentation-gaps.md) | M | 7 |
| 128 | [Adaptive frame rate](128-adaptive-frame-rate.md) | XS | 7 |
| 129 | [Metric exponential smoothing](129-metric-exponential-smoothing.md) | XS | 7 |
| 130 | [Tab content badges](130-tab-content-badges.md) | XS | 7 |
| 148 | [TUI god objects decomposition](148-tui-god-objects-decomposition.md) | L | 6 |
| 150 | [Cybernetic feature wire-or-remove audit](150-cybernetic-feature-wire-or-remove-audit.md) | M | 6 |

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
134 (replan-gate-failure) → 14 (plan mutation protocol)  [soft: 134 specializes 14]
135 (adaptive gate thresholds) → 80 (learning data quality)  [soft]
137 (router state in snapshot) → 138 (crash/resume proof matrix)

# Wave 3 → Wave 4
107 (plan run UX friction) → 108 (TUI live feedback gaps)
108 (TUI live feedback) → 109 (TUI real-time streaming parity)
109 (TUI streaming) → 110 (deprecate JSONL / StateHub-only)
109 (TUI streaming) → 121 (TUI data model unification)
121 (data model unification) → 122 (remove legacy page system)
121 (data model unification) → 124 (header bar Mori parity)

# Wave 4 → Wave 5
116 (queue manifest) → 117 (plan-level wave computation)
117 (wave computation) → 125 (plan tree wave hierarchy widget)
83 (dream consolidation deadlock fix) → 143 (dream consolidation trigger)

# Wave 5 → Wave 6
110 (deprecate JSONL) → 61 (dispatch consolidation)  [soft: both reduce duplication]
14 (plan mutation protocol) → 134 (replan-gate-failure runner-v2)  [already captured]
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

---

## Status Notes

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

**New items 111-150** have spec files written by the synthesis agent pass on 2026-08-19.
Each file is self-contained with background, current state, implementation plan, acceptance
criteria, verification checklist, and files-to-modify table. All 40 spec files exist at
`tmp/backlog/111-*.md` through `tmp/backlog/150-*.md`.

**Size legend:**

| Code | Meaning |
|---|---|
| XS | Under 2 hours — single focused change |
| S | Half-day to 1 day |
| M | 2-3 days |
| L | 3-7 days |
| XL | More than 1 week; consider splitting |

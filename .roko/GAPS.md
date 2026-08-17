# Roko Project Status & Outstanding Work

> **Single source of truth** for human readers on implementation status, architectural gaps,
> and outstanding work. For agent execution protocols and task-level checklists, see
> `tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md`.
>
> Last updated: 2026-08-17

---

## Programme Summary

**48 epics total** across the roko self-hosting programme.

| Category | Epics | Count |
|---|---|---|
| **Accepted complete in the canonical roll-up** | E01-E48 | 48 |
| **Active / Partial epics** | None | 0 |
| **Not started (greenfield epics)** | None | 0 epics / 0 manifest tasks |
| **Self-heal (SH)** | SH01-SH06 | 57/57 tasks done |

The greenfield epic-manifest backlog is **0 tasks**. This does not mean the product or
programme is release-complete: the master checklist still contains **20 unchecked non-epic
markers** and **11 partial markers**, including non-epic plans, documentation convergence,
dogfood/release proof, and broader product/runtime residuals. Those residuals include E32
adapter/Component work, E37 renderer and native-source work, native Agent-to-E33 telemetry
publication, provider-internal security visibility, remaining Runner-v2 Graph parity,
additional E29 transports and protocol integrations, durable E38 marketplace services,
deployed/networked E39 registries, the broader E40 eval/flywheel/on-chain system, and live
E41 DeFi/risk/venue adapters. Loop 4 structural adaptation, ADAS/HGM-style autonomous
generation, and continuous recursive-safety wrapping of every Flow also remain roadmap work.

The executable plan queue is narrower and machine-derived: **30 plans / 124 tasks / 124
done / 0 remaining (100%)**. All 30 executable plans are complete. The final
`architecture-production-residuals` tranche delivered a supervised concrete connector,
bounded relay replay plus durable exact-room subscription execution, a live durable local
arena service, and bounded meta-agent lineage/recursive safety. Superseded queues (4 plans /
92 tasks) and fixtures (6 plans / 19 tasks) are explicitly excluded from these totals.

The raw E01-E48 manifest metadata count is **393/447**. It is not a completion score:
several older, canonically accepted manifests retain stale task counters or aggregate status.

## Remaining Work Estimate

There are no ready executable-plan tasks, but the wider release/product backlog is not empty:
the master checklist has **22 non-complete markers** (8 unchecked, 14 partial) after the
2026-08-17 comprehensive 25-agent verification sweep that verified and resolved 12 previously
unchecked items (moving 9 to done and 3 to partial). A practical delivery estimate is about
**four substantial tranches**, with scope varying by whether the large architecture items are
accepted as follow-up debt:

1. Integration provenance and release closure for the current dirty implementation tree,
   followed by fresh-clone/post-merge verification.
2. Documentation truth and generated validation: 3/71 DOC tasks are done and 68 remain
   ready; individual supersession evidence and legacy/repo-wide link work remain. The exact
   status/source registries, maintained operator-corpus links, and `plans/INDEX.md` drift gate
   are now bounded and checked.
3. Real process-level lifecycle proof for Graph, crash/restart equivalence, and deterministic
   self-host repair.
4. Cross-surface durable projection agreement so the converged frontend DataHub, TUI, API,
   and CLI project the same authoritative durable state.
5. Architecture consolidation: EventBus/type families and continued `event_loop.rs`
   extraction.

The latest bounded closure delivered deterministic documentation integrity, the first
source-reconciled DOC-v2 wave (Signal/Cell, Agent, Memory/Learning/Cross-Cuts), and direct
durable Runner projection correctness. The maintained operator corpus passes 17/17 checker
fixtures plus the live bounded link/anchor scan; all 109 status files and the exact 746-source
registry/637-row manifest contracts are accounted for; `plans/INDEX.md` has a blocking
non-mutating CI drift check. Direct TUI/show/API/workspace adapters now share the verified
canonical Runner loader, but StateHub baseline/overlay rebasing, cursor-atomic SSE/typed
capture, and a single immutable resume generation remain open. Graph Runner-v2 parity plus
provider-internal visibility and adaptive immune learning remain major runtime semantics gaps.

---

## Recently Closed Epic Status

No epic is currently partial in the canonical manifest roll-up. The latest residual closure
pass completed R01-R04: one supervised HTTP JSON connector, bounded relay replay and durable
subscription execution, a live authorized local arena service, and a bounded meta-agent
lifecycle composed from the existing safety primitives. The preceding pass accepted E29 and
E38-E41, completed P28 and the E39/DeFi critical path, hardened E43, and closed P34. These
closures retain explicit local and scoped boundaries; broader product residuals remain
tracked separately.

| Epic | Description | Done/Total | Acceptance outcome |
|---|---|---|---|
| E29 | Connectivity / relay | 9/9 contract tranche + R01/R02 runtime | The five-method async `Connect` contract now has one supervised HTTP JSON adapter with bounded health/reconnect and authenticated secret-safe lifecycle operations. The canonical relay envelope is consumed by a count/byte-bounded server and supervised client with atomic `Subscribe(last_seq)` recovery, session-safe supersession, and durable exact-room subscription execution; ACK follows only terminal-receipt and cursor persistence. Additional transports, startup discovery, MCP/A2A/x402 execution, finality/reorg processing, and dashboard integration remain product work |
| E38 | Artifact marketplace | 9/9 contract/stub tranche | Artifact/package/publish/economics/fork/capability contracts, upstream TraceRank attribution, auth-classified HTTP shapes, and seven `roko market` commands are present. The HTTP/CLI surfaces are intentional stubs; durable storage, real publish/install execution, ratings/indexing, and ERC-8004 anchoring remain product work |
| E39 | Registries / identity | 8/8 local state-machine tranche + critical-path wiring | Transferable identity and narrow delegation, challengeable knowledge, reputation TraceRank, transport-neutral gossip discovery, marketplace reputation effects, durable authenticated local registry routes, and an optional bounded/manual read-only event index are implemented. A best-effort legacy agent-registration dual-write exists, but it is not integrated with the local passport lifecycle and may not match the target deployed ABI. Deployed compatible contracts, gossip transport, ABI decoding, automatic WebSocket polling, PostgreSQL/SSE delivery, and a verified write adapter remain runtime/daeji work |
| E40 | Arenas / evals | 8/8 local tranche + R03 service | The arena/scoring registry now backs an authenticated, owner-aware, restart-safe HTTP lifecycle. Attempts bind external scoring evidence; attempt terminalization, prize/reputation effects, leaderboard state, and the durable event outbox commit atomically. Eval orchestration, the seven-stage flywheel, token/on-chain settlement, and cross-arena transfer remain product work |
| E41 | DeFi products | 8/8 local/stub tranche | Local instrument types, checked bond and insurance lifecycles, reputation-option pricing, synthetic indices, affect sizing, and provider-neutral rate effects are implemented. Authenticated scope/RBAC-classified routes intentionally return structured 501 responses; durable/on-chain services, the risk engine, and venue execution remain product work |
| E23 | Agent cognitive autonomy | 10/10 | EFE routing and GoalTree now join the lifecycle, vitality, CorticalState, energy, timescale, and affect foundations. Runner-v2 applies numeric low-vitality cost pressure, hard phase tier caps, restart-durable Terminal skips, and post-spawn energy charges; exact lifecycle/phase/energy/goal tests pass |
| E34 | Security / IFC | 8/8 strict manifest tasks | Trust-origin IFC remains explicitly separate from data classification; monotonic TaintTracker propagation, exact Cell × Graph × Space capabilities, persistent transitive quarantine incidents, and a mandatory audited taint/corrigibility production hook chain close the remaining contracts |
| E25 | Advanced learning loops | 10/10 | HDC defragmentation, append-only hindsight relabeling, governed c-factor recommendations, statistically gated prompt experiments, Variance Inequality checks, autocatalytic metrics, and when/then playbook enrichment are implemented; the live runner records post-run governance/compounding evidence. The complete `roko-learn` library suite passes 980/980 |
| E36 | Payments | 8/8 | Core pricing contracts, x402 settlement batching, MPP sessions, reputation pricing, separate durable payment costs, paid-feed HTTP 402 enforcement, and dashboard events are wired. Focused core/chain/learn/serve suites and the live paid-feed route cases pass |
| E42 | Config evolution | 8/8 | Priority/provenance, all seven invariants, pre-deserialization migration, per-field merge provenance, transactional reload, inherited domain profiles, freshness persistence, and doctor diagnostics are wired. Unknown top-level fields are diagnosed; nested unknown fields remain serde-compatible and silent |
| E44 | Cross-cut functors | 8/8 | Memory, Daimon, Dreams, and mandatory Safety functors compose through six natural transforms and conflict-only VCG arbitration. Real gate failures now launch a non-blocking Memory → Daimon → Dreams cascade; default/HDC composition checks and focused functor tests pass |
| E24 | Advanced memory | 10/10 | Balance demurrage and exact reinforcement, falsifier lifecycle, streaming HDC role/filler lookup, temporal querying and GC, distillation, cross-domain transfer, configurable tier progression, Dream consolidation, and CLI lifecycle hooks are implemented with focused neuro/dream/CLI coverage |
| E26 | Inference gateway | 12/12 | The new `roko-gateway` workspace crate owns the nine-stage inference pipeline, routing and fallback, two-layer caching, tool/output/thinking controls, convergence, cost accounting, rotating keys, three-level backpressure, handles, and batching; authenticated serve routes use the live gateway state |
| E27 | Continuous feeds | 10/10 | `FeedCell`, the runtime registry, Bus bridge, file/provider-health/episode-outcome/derived feeds, recipe DAG/store, lifecycle and discovery APIs, and feed/recipe CLI commands are live. The retired rate-oracle vertical was not reintroduced |
| E28 | Agent groups | 8/8 | Persisted group membership, invitations, permissions, coordination, knowledge and pheromone state, messages/events, Bus publication, runtime routes, configuration, CLI surfaces, and privacy-filtered group prompt context are wired |

R04 closes a separate scoped architecture residual rather than a new epic. Meta-agent
proposals, validation, activation, exact one-step morph/rollback, and deactivation are
owner-scoped and restart-durable. Children cannot widen tools, data, network, cost, spawn,
expiry, or lineage limits; activation requires the canonical ordered five-head safety Graph
and single-use R03 evidence bound to the complete artifact. This does **not** implement Loop
4, ADAS/HGM, autonomous execution of generated agents, or continuous recursive-safety
wrapping for every Flow.

E33 is runtime-complete at the telemetry ingress boundary: all 39 protocol variants have
production evidence. Registered agents commit typed lifecycle observations through
`POST /api/agents/{id}/observation`; the server validates canonical regimes/modes/phases,
legal lifecycle transitions, monotonic vitality, real completed ticks, slot deltas, agent
identity, and restart-durable transport sequences before best-effort Lens fanout. E23 now
has native AdaptiveClock, five-phase vitality, target type-state, SlotManager, revisioned
mode owners, EFE routing, GoalTree, and live runner dispatch constraints. Native Agent owners
still do not publish the E33 observation payload directly; that is a product integration
residual outside the completed E23 and E33 manifests.

E32 is complete against its 8/8 authoritative manifest. WIT/Component-model hostcalls and
OpenClaw/legacy one-shot parity remain separate roadmap work; they are not E32 task
acceptance requirements.

E37 is complete against its 9/9 shared contract/backend manifest: typed projections,
Inbox/autonomy events, twelve object types, five StateHub-backed HTTP routes, OpenAPI
discovery, and the legacy-tab compatibility mapping are implemented. Product rendering is
still partial: the legacy TUI does not render every named surface, SurfaceEvents do not yet
enter a command path, no production Inbox publisher/action consumer is wired, `pending_human`
has no live source, autonomy config has no live store, Canvas uses dashboard plan IDs, and
Minimap uses an explicitly labelled deterministic layout rather than HDC coordinates. The
five OpenAPI paths still use generic JSON response schemas, and replaying an unresolved
Inbox receive event recomputes its receipt timestamp because the durable event has none.

---

## Architectural Gaps

### event_loop.rs is a ~23.1K-line god object -- OPEN

`crates/roko-cli/src/runner/event_loop.rs` (23,074 lines at this audit). Extraction has begun:
`gate_dispatch.rs`, `persist.rs`, `snapshot_writer.rs`, `merge.rs`, and
`branch_cleanup.rs` now own coherent slices. The main `run` path still owns plan execution,
agent dispatch, terminal gates, snapshots, merge, cancellation, learning, dreams, cleanup,
and thousands of lines of tests. Merge conflicts remain common and testing individual
subsystems still requires compiling the entire CLI crate.

**Fix:** Continue feature-sized extraction along the existing module boundaries, with focused
tests after each move, until `event_loop.rs` is a thin orchestrator.

### Provider outcome feedback and health routing -- RESOLVED

Canonical workflow services and runner-v2 now load the persisted
`ProviderHealthRegistry` from `.roko/learn/provider-health.json`. `ModelCallService`
records every live attempt at the provider-call boundary, including a failed primary
followed by a successful fallback, under the exact configured provider ID. CLI
subprocess settlement records its confirmed outcome once, bridge calls record directly,
and the learning event subscriber no longer mirrors provider events and double-counts
them.

Runner-v2 and both shared workflow factories map effective model slugs back to provider
IDs and exclude unhealthy providers before cascade selection. `roko-serve` uses that
same persisted registry for dispatch, routing, and provider-health APIs rather than a
shadow in-memory tracker. Focused tests cover exact-provider success,
primary-failure/fallback-success accounting, CLI provider/model fallback, shared API
state, and health-filtered candidate selection.

ACP automatic selection now consumes that shared health state plus canonical RPM/TPM
snapshots. Plain Anthropic streaming, OpenAI tool loops, and every internal turn of the
Anthropic native tool loop update the same limiter and exact-provider outcome recorder.
The native backend accepts those shared runtime hooks explicitly, and focused coverage
proves per-turn request/token accounting plus success and rate-limit feedback.

### loop_tick.rs architecture claim does not match production -- RESOLVED (2026-08-16)

The architecture truth is now explicit. Production `roko run` uses `WorkflowEngine`, plan
execution uses Runner-v2 or Graph, and none claims to call a universal `loop_tick` skeleton.
The core implementation is exported as `select_compose_verify_persist`, accurately scoped to
query/route/compose/verify/persist/react. It does not claim ACT, BROADCAST, cancellation, or
resource enforcement. Historical `loop_tick`, `loop_tick_with_config`, `TickConfig`, and
`TickOutcome` names remain deprecated compatibility APIs, with documentation stating that the
old configuration was never enforced. The active architecture guide, source index, CLI guide,
and integration test now use the production owners and helper name consistently.

### HTTP MCP tools retain executable clients -- RESOLVED (2026-08-13)

The default catalog count is internally consistent: 16 executable local standard tools
plus 19 GitHub MCP definitions (35 total), with 17 optional chain entries producing 52
under the `chain` feature. Dispatch-class tests prove all names are classified: MCP
entries are remote definitions, while the chain entries resolve to explicit typed
"not wired" placeholders rather than silently missing handlers.

`McpRuntime` now retains discovered definitions, initialized clients, and the Tokio
runtime ownership needed by synchronous provider construction. The common HTTP tool
registry composes built-in handlers with `McpHandlerResolver`; OpenAI-compatible,
Anthropic API, Gemini compat/native, Perplexity, and Cerebras adapters all use it.
Definition-only MCP tools are rejected rather than advertised without handlers, and
ambiguous server names fail before spawn. A non-ignored two-turn provider test discovers
an stdio MCP tool, receives a model tool call, executes `tools/call`, and supplies the
result to the next HTTP model request.

### Gate threshold flush interval configurable -- RESOLVED (2026-08-13)

`learning.gate_threshold_flush_interval` is part of the canonical schema, config
layering and `config set` path. Runner-v2 reads it once per run; the compatibility
default is 10 and zero is normalized to one. Persistence tests prove no early write,
write/reset at the configured boundary, round-trip, and repeated flush.

### Workspace lock coverage -- RESOLVED (2026-08-13)

`acquire_workspace_lock()` uses `flock` across state-mutating PRD and plan commands,
`daemon start`, `serve`/`up`, `roko run`, `roko do`/`develop`, and bounded managed-agent
lifecycle commands (`create`, `delete`, `start`, and `stop`). The lock guard spans the
complete mutation, and contention preserves the live owner's PID for an actionable
diagnostic. Read-only agent queries and interactive/long-running agent runtimes do not
claim the workspace writer lock.

### GitHub workflow integration -- RESOLVED (2026-08-14)

E46 is 12/12 complete. The reusable rate-limited `GitHubClient` backs both the MCP
server and a `LiveGitHubOps` adapter whose blocking calls stay off the Tokio runtime.
Runner-v2 owns one ordered side-effect path: it opens a draft plan PR, reports terminal
task gates, tracks terminal failures as issues, publishes the exact cumulative accepted
commit, then requires local regression and GitHub CI success before merging. Publication,
CI, and API failures leave the PR open with diagnostics.

Verified webhook ingestion graduates exact plan-label, requested-changes, and failed-check
events into durable trigger signals without executing plans in the HTTP handler. `roko
github status` reports config, authentication, plan PR/CI state, and failure issues without
requiring the server. The previous ~8/12 headline came from similarly numbered webhook
batch items rather than the authoritative E46 task manifest; the manifest and master
checklist now use the actual T01-T12 contract.

### E18 docs, config, and operations -- RESOLVED (2026-08-14)

E18 is 15/15 complete. The declared MSRV matches CI and the release image. The audited
working-tree CI/release definitions run strict default-target/default-feature workspace
clippy and the default workspace test suite; coverage no longer ignores test failures, and
cargo-deny, docs-drift, and tracked-plan validation are represented as blocking gates. Those
workflow edits still require integration before they govern tags from `main`; the stricter
all-target/all-feature gate below is not implied by them. Clean-checkout Docker builds use a
committed default configuration. Core validation is authoritative for provider/model
resolution, config diagnostics and secret redaction are surfaced consistently, and the
doctor reports the single runner-v2-dead conductor switch without misclassifying live watcher
thresholds or cold-storage settings. The audited operator corpus and GitHub integration guide
match the runtime and CLI.

### Workspace release verification -- RESOLVED (2026-08-16)

At the historical P34 local checkpoint on 2026-08-16, `cargo fmt --all -- --check`,
`cargo check --workspace`, strict default-target/default-feature workspace clippy,
the default `cargo test --workspace` package/integration/doctest run, and
`cargo build --release -p roko-cli` passed. The checkpoint's release CLI started, reported
its version, completed `doctor`, validated the full plan tree, and strictly validated P28,
P34, the DeFi critical path, the production residual queue, and E43. The binary had no
dynamic OpenSSL/crypto linkage, and the release configuration contained neither `sccache`
nor swallowed `|| true` failures. This is historical evidence for P34, not a claim that the
subsequently modified dirty tree has passed the full release suite or the stricter
all-target/all-feature final gate.

The sweep also fixed failures that only appeared under complete or repeated execution:
canonical snapshot fixtures, CLI budget serialization, TOML trigger fixtures, bounded
probe timeouts, race-free environment and sidecar tests, API 404/SPA fallback ordering,
deterministic learning timestamps, Mirage request accounting, and collision-free MCP-code
temporary workspaces. P34 is now 4/4 complete.

### Worker deployment callbacks authenticated -- RESOLVED (2026-08-13)

Route-created workers receive an opaque callback ID and a deployment-scoped bearer
token before the backend is invoked. The worker receives the plaintext token only via
its environment; deployment persistence stores a SHA-256 verifier. Callback lookup
accepts the opaque ID, compares verifiers in constant time, and rejects missing or wrong
tokens. Worker-token authentication is scoped into the global serve-auth middleware, so
enabling API-key auth no longer blocks legitimate callbacks before route validation.
CLI Railway deployments reuse the control-plane callback token and assign a distinct
opaque callback ID per worker.

### Runner VCG feedback loop -- RESOLVED (2026-08-16)

Runner prompt assembly now passes the configured composition strategy and warmup threshold
to the canonical `PromptComposer` and retains its exact composition manifest through the
terminal gate. Every eligible bidder observes the completed allocation round, while only
sections actually included in the prompt receive success/failure posterior and cost credit.
This lets a fresh `Auto` runner move from density-greedy to VCG exactly when every active
bidder is warm; explicit DensityGreedy and VCG remain stable. Bidder state survives prompt
cache refreshes and is atomically saved on clean shutdown. Missing state cold-starts, while
oversized, malformed, or identity-inconsistent state fails closed and is preserved.

Attention-bidder updates still use a clean-shutdown snapshot rather than the new
ExperimentStore attempt receipt, so a crash after a terminal gate can lose the newest bidder
observation. Exact provider-cost attribution is also not fed into this bidder update. Those
are durability/cost-attribution residuals, not VCG reachability failures.

### Prompt-experiment coverage across runtimes -- PARTIAL

Runner-v2 now selects variants under a durable run/plan/task/attempt receipt, replaces the
exact canonical section before scoring and budgeting, and records raw-content-free attribution
in prompt diagnostics. It transactionally binds the exact final prompt immediately after
model/safety admission and before either provider launch. Durable terminal events drive
idempotent settlement; startup reconciliation scans oldest archive through the live event log,
deduplicates replays, and refuses contradictory terminal facts. Excluded sections, assembly
errors, confirmed spawn failures, and other pre-launch exits are abandoned without a trial.
Capacity deferrals retain the same treatment, but reservations from an abandoned older run do
not bias allocation in a new run. Settlement removes raw content while retaining the audit
hash/tombstone. A process crash in the narrow interval after the dispatch hash is committed but
before the provider actually starts can still leave a dispatched receipt without launch proof;
closing that interval requires a provider-start acknowledgement or transactional launcher.

All production prompt-store mutations in runner, serve feedback, LearningRuntime, and ACP now
use the same strict sibling-lock read/mutate/atomic-write transaction. Malformed or oversized
state is preserved, concurrent writers do not lose updates, and outcome updates prefer exact
experiment-plus-variant identity rather than ambiguous global variant IDs.

The remaining product gap is cross-runtime semantic parity. Serve template dispatch and ACP
still inject ephemeral experiment context instead of replacing a canonical named section under
a durable assignment/dispatch receipt; their process-local dedup can therefore double-count
after a crash. LearningRuntime's legacy variant-only WAL projection can replay an already
committed outcome when a later snapshot/truncation step fails, its permissive startup cache can
hide malformed state, and concurrent disk commits can publish cache snapshots out of order.
Attention-bidder feedback also remains clean-shutdown rather than attempt-replay durable. Add
runtime-appropriate assignment receipts and durable replay IDs to those paths, make runtime
cache publication monotonic/strict, then define bounded tombstone compaction before the 64 MiB
strict store ceiling can become operationally relevant.

### Tier progression after live knowledge ingestion -- RESOLVED (2026-08-13)

The runner's `NeuroKnowledgeIngestor` records successful gate-backed feedback as a
confirmation and distinct plan/task context, persists the admitted candidate, invokes
the canonical `TierProgression::evaluate_tier_progression_v2`, and atomically writes
the tier change. The live sink test proves the confirmation/context evidence and a
Transient-to-Working promotion. Manual imports and offline consolidation remain
separate ingestion paths and do not claim this live hook.

### Playbook selection wired at dispatch -- RESOLVED

`PlaybookStore` is instantiated in the runner (line 1795), playbook IDs are
stored per-task at dispatch (line 8128-8131), and `record_outcome` is called
on gate pass/fail (line 3289-3294). The feedback loop is closed.

### events.jsonl retention -- RESOLVED (2026-08-13)

`.roko/events.jsonl` is bounded by both StateHub line compaction and the canonical
`resources.log_rotation_max_mb` size threshold. Appenders, compaction, and rotation
share an advisory lock; rotations use unique archive names, readers include archived
episode generations, and GC expires old archives without deleting the live log.

### Resource and disk lifecycle (E47) -- RESOLVED (2026-08-13)

Runner v2 performs ordered pre-plan log rotation, stale-target cleanup, filesystem GC,
and the disk admission check. It measures live worktree growth, reserves aggregate disk
headroom before dispatch, temporarily serializes admission under pressure, publishes
canonical `worktree_count` and `disk_budget_remaining` metrics, and cleans task build
artifacts only after the final gate and durable terminalization. `roko doctor disk`
reports free space, stale targets, orphan worktrees, oversized JSONL files, and aggregate
workspace storage without mutating it. Resource cleanup is policy-gated and non-fatal.

### Cross-crate duplicate type families -- PARTIAL

The July 19-family census is stale. `DashboardSnapshot`, `StateHub`, `GateVerdict`,
`RetentionPolicy`, and `Engram` now have one exact public definition, and the TUI consumes
the canonical dashboard snapshot. A current source audit still finds roughly 14 conceptual
families requiring case-by-case consolidation or explicit semantic separation, including
`AgentState`, `TaskStatus`, `GateFeedback`, `EventBus`, `Cell`, and `Plan`.

### `#[allow(dead_code)]` sites -- HYGIENE / DEFERRED

The reproducible 2026-08-16 scan finds 48 exact standalone attributes under `crates/`, or
49 attribute lines containing `dead_code` across 30 Rust files when compound attributes are
included. E12-T04 completed the classified keep/wire/remove pass. The retained attributes
are compatibility, test, platform, or future-owner boundaries; this is a dated hygiene
inventory, not an unfinished runtime gap. New suppressions should still require an explicit
owner and rationale.

### Knowledge store consulted for model routing -- RESOLVED (2026-08-13)

Runner-v2 opens the live `KnowledgeStore`, builds model-specific positive and
negative routing advice from persisted knowledge, and supplies that advice to
`select_for_frequency_among_with_knowledge`. The cascade router applies the
resulting score hints during candidate selection. Focused tests cover both
positive/negative advice and the no-relevant-knowledge path.

### Config init/global model slug collision -- RESOLVED (2026-08-13)

`roko init --profile rust` generates `[models.claude-sonnet-4-6]` with `slug = "claude-sonnet-4-6"`. The global config (`~/.roko/config.toml`) has `[models.claude-sonnet]` with the same slug. After merge, the `AmbiguousModelSlug` validation rejects the run. Local config cannot override global for same-slug models. Additionally, the init template generates `[github]` and `[resources]` sections not recognized by the unified loader.

Project models now override global models by both table key and trimmed provider slug.
Same-layer duplicate slugs still fail validation, and an inherited global default is
remapped to the winning local key. `[github]` and `[resources]` are recognized schema
sections and have init-template regression coverage.

### Scheduler deadlock after AgentCompleted -- RESOLVED (2026-08-13)

During dogfood `plan run`, the agent completed T1 at 12:52:29 (confirmed by logs). The runner then sat idle for 10 minutes until `SchedulerNoProgress` timeout at 13:01:55. The `AgentCompleted` state was set but the event loop never transitioned to gate dispatch. Gates never ran, T2 never started.

Runner v2 now resolves the synthetic `Enriching` phase through prompt assembly before
dispatching an authored task. Legacy snapshots stuck in `Enriching` are recovered, and
an authored completion observed in that phase advances through implementation to the
gate. Both normal and recovery paths have focused regression tests.

### Stale executor snapshot blocks fresh plan runs -- RESOLVED (2026-08-13)

`roko prd plan` internally generates a `demo-hello` plan and writes to `state-snapshot.json`. When the user later runs `roko plan run` on a different plan, resume validation rejects because the snapshot has plans not in the current run. Requires `--fresh` flag which users won't know about.

Runner and executor snapshots with zero current-plan overlap now start fresh
automatically and do not import unrelated learned gate thresholds. A partially
overlapping snapshot still fails closed, preserving strict fingerprint validation.

### core.fsmonitor breaks worktree checkout -- RESOLVED (2026-08-13)

Git's security policy treats `core.fsmonitor` as an unsafe extension in worktree operations. VS Code and many developer tools enable this. The former orchestrator worktree manager made no accommodation, causing silent failures with: `unsafe git execution policy: unsupported checkout extension 'core.fsmonitor'`.

Every managed Git probe and mutation uses `--no-pager -c core.fsmonitor=false`.
Repository or user fsmonitor configuration is therefore inert for the managed
operation and remains unchanged; create, health-check, and removal are covered.

### Cascade router silently overrides configured model -- RESOLVED (2026-08-14)

ACP adaptive selection is exact opt-in via `ROKO_ACP_CASCADE_SELECT=1`, applies only
to direct non-slash/non-pipeline prompts, and skips valid explicit session model or
provider selections. Overrides log requested/selected config keys plus the maturity
stage and persist `cascade_selected_model`/`cascade_stage` in the episode. The global
`--force-model` alias uses the existing highest-precedence CLI model override and now
fails closed when that model cannot be resolved.

### Dream automatic scheduling -- RESOLVED (2026-08-15)

Daemon mode owns a resident `DreamSchedulePolicy` loop. It queues cron fires while
agents are active, runs them at the next idle boundary, supports episode-count and
adaptive idle triggers, enforces the unconsolidated-episode minimum, deduplicates cron
polls, restores the latest dream checkpoint, and rejects invalid cron/quality policy
before daemon-info publication and background-loop startup. Manual execution remains available. Bus-reactive and
intensive-backlog scheduling are separate future refinements, not missing cron wiring.

---

## Built-but-Unwired Subsystems

| Subsystem | Where | Status |
|---|---|---|
| **Telemetry Lens runtime** | `roko-core/src/telemetry_observe.rs`, `roko-core/src/lens_registry.rs`, `roko-graph/src/engine.rs`, `roko-runtime/src/lens_executor.rs`, `roko-serve/src/agent_lifecycle.rs`, `roko-serve/src/routes/projections.rs` | Runtime-wired. All 11 specified built-ins have bounded, fail-closed implementations. Raw stacking and Cost→Trend→Anomaly-style chains flow through queued Graph delivery, typed aggregation, durable StateHub history, CLI output, breaker diagnoses/operator controls, and REST/SSE surfaces. Time retention is configurable (7 days by default), central producer fanout is live, and all 39 production event variants have evidence through real subsystem owners or the registered-agent lifecycle ingress |
| **Corrigibility** | `roko-core/src/corrigibility.rs`, `roko-graph/src/cells/corrigibility.rs` | Runner/ACP dispatch, embedded effects, five literal Verify Cells, and a fixed fail-closed Graph are live. Provider-owned internal calls/results remain opaque unless surfaced as host-visible Signals |
| **AgentPool / MultiAgentPool** | `roko-agent/src/pool.rs`, `multi_pool.rs` | Pool management built; TUI has modal for it; no runtime instantiation in runner |
| **Cold substrate archival** | `roko-fs/src/cold_substrate.rs`, `roko-serve/src/lib.rs` | Runtime-wired: configurable scheduled age/batch policy archives before pruning hot storage; archive writes are deduplicated |

---

## Deprecated Rate-Oracle Vertical -- RESOLVED (2026-08-13)

The former rate-oracle vertical has been removed end to end: chain keeper/source/
submit/bootstrap/clearing modules, standard tools, CLI command, serve routes and
state, feed agents, relay adapter, config, dashboard projections, and active docs.
Generic chain watching, gas, on-chain analytics, and the system heartbeat remain.
`roko-chain` is now optional for `roko-std`; default builds expose 35 tools, while
the `chain` feature adds 17 chain-domain tools for a total of 52.

---

## Deferred Work (Phase 2+)

### roko-chain: local-only and shelved runtime modules

These modules contain tested Rust logic but still lack a production runtime caller or one of
their required adapters. They are not all blocked on the same dependency: deployed contracts
and consensus integration depend on the daeji devnet, while other gaps require Roko service,
persistence, authorization, market-data, or venue adapters. daeji owns
node/BFT/precompiles/consensus in a separate repository; its design documents live at
`tmp/agentchain-v2/02-daeji/`.

| Module | Production integration boundary |
|---|---|
| `witness.rs` | daeji witness registry contract |
| `x402.rs` | ERC-3009 + state channels need live token |
| `korai_token.rs` | KORAI.sol not deployed |
| `marketplace.rs` | Job-market logic plus local E38 fork/economics contracts exist; artifact HTTP/CLI remains stub-only with no durable or on-chain adapter |
| `agent_registry.rs` | Local transferable identity/delegation lifecycle is durably wired through authenticated serve routes; owner request values are admin-controlled lifecycle strings, not cryptographic bearer bindings. The separate best-effort `register(string,bytes32)` dual-write on agent registration is legacy and is neither passport-integrated nor evidence of a compatible deployed ERC-8004 adapter |
| `reputation_registry.rs` | Local reputation records exist; no deployed registry/runtime adapter |
| `validation_registry.rs` | Local validation logic exists; no deployed registry/runtime adapter |
| `knowledge_registry.rs` | Local publish/validate/challenge lifecycle is durably wired through authenticated serve routes; no deployed contract/governance adapter exists |
| `indexer.rs` | Optional provider-neutral client and bounded, finality-aware JSONL index are wired for authenticated manual sync/rebuild; no background worker, ABI decoder, transaction-hash enrichment, PostgreSQL/SSE service, or deployment path exists |
| `gossip.rs` | Transport-neutral messages and TTL peer registry exist; no network gossip transport |
| `trace_rank.rs` | Local reputation and fork-attribution algorithms exist; no production outcome consumer or persistence path |
| `arena.rs` | Local arena/escrow/effect/snapshot state machine exists; HTTP is 501-only with no live service or settlement adapter |
| `defi.rs` | Local product, pricing, index, insurance, and rate-effect primitives exist; HTTP is 501-only with no risk, venue, durable, or on-chain adapter |
| `collusion.rs` | No multi-agent marketplace yet |
| `nelson_siegel.rs` | No term-structure consumers |
| `futures_market.rs` | Local futures and E41 product types exist; no live derivative venue or execution path |
| `gate/mev_gate.rs` | Not in 7-rung gate pipeline |
| `gate/tx_sim_gate.rs` | Not in 7-rung gate pipeline |
| `gate/wallet_gate.rs` | Not in 7-rung gate pipeline |
| `heartbeat_ext.rs` | No chain-aware lifecycle yet |

### Greenfield epic queue

None. E01-E48 are accepted in the canonical epic roll-up, with **0 remaining
epic-manifest tasks**. The local/contract/stub closures above do not close the Phase 2 and
product-runtime boundaries in this section.

### Phase-2 stubs (intentional)

`crates/roko-daimon/src/phase2_stubs.rs` (4 items) and `crates/roko-dreams/src/replay.rs`
(1 item) are intentional `#[allow(dead_code)]` placeholders. No action until Phase 2
affect-engine work begins.

---

## Technical Debt

### AgentContract and strict E34 enforcement are live; broader security coverage remains partial

Eight core roles have compile-time bundled contract assets. Runner-v2 resolves the
task role with `RestrictedFallback`; invalid and unknown roles preserve an explicit
empty allowlist. Role/task allowlists intersect, denials union, and denial wins. Claude
CLI receives enforceable allow/deny flags (including deny-all); Codex and provider kinds
without a live policy hook reject policy-bearing dispatches. Bridge dispatch carries
the structured contract into provider `SafetyLayer` evaluation and filters the
advertised built-in/MCP registry with the same policy.

E34 is 8/8 against its strict manifest, and the supported live chain is no longer
structural-only. Runner
and ACP pre-dispatch evaluate immutable action context through five distinct, fixed-order
verifier components; the same heads are literal independently hosted Cells in a fixed,
fail-closed Graph. Embedded provider tool loops carry monotonic taint from
external/MCP/plugin/web results and gate later privileged effects; Signal derivation and
Graph Cell boundaries cannot lower input classification; and structured tools plus
subprocess admission consume the config-selected five-level sandbox policy. Five typed
immune stage Cells and their single-concurrency decision Graph preserve the pure policy
order and reject skipped/reordered stages. Opaque Claude/Codex-owned internal
results still cannot expose per-result taint or per-call corrigibility. Configured
CPU/memory/process caps and fail-closed network allow/deny reach provider workloads and
readiness/version subprocesses uniformly; Hermes/OpenClaw probes are bounded by configured
whole-probe deadlines. KnowledgeStore ingress/import/read/rewrite paths preserve monotonic
labels. The trust-origin lattice remains separate from data classification, derived signals
propagate it monotonically, and exact Cell × Graph × Space capability intersections are
cached at load time. Persistent quarantine supports idempotent batch review, cycle-safe
transitive incidents, statistics, and atomic restart recovery. Every production
`ToolDispatcher` runs mandatory taint then corrigibility hooks after custom parameter
transforms and emits redacted structured audits. Its universal return seam bounds and
recursively scrubs content, errors, and artifacts, then sends every host-visible result
through the fixed five-stage immune Graph. Suspicious remote/plugin/MCP results are withheld;
the exact content-addressed body/identity and security metadata are retained in strict,
bounded evidence and quarantine ledgers. Medium findings create a durable per-tool cooldown,
High findings isolate the tool/source, and same-scope incidents are linked transactionally.
Provider tool-call ingress is count/byte bounded before rendering or checkpointing, handler
panic payloads are suppressed, and checkpoints are capped and atomically replaced.

Every agent returned by the canonical provider factory is wrapped by a fail-closed primary
final-output boundary. It buffers streaming content until acceptance, verifies attached
Ed25519 attestations, runs the same Graph, and commits High/Critical isolation under a
canonical workspace immune root independent of disposable attempt worktrees. Locked
authority/evidence/vault files fail closed on malformed, oversized, future-dated, or
internally inconsistent state. Receipts prove historical internal consistency;
authenticating a wholesale rewrite still requires an external anchored digest, MAC, or key.
Provider-owned internal calls/results, provider trace Signals, broad semantic detection, and
adaptive immune-memory feedback remain product scope outside strict E34 acceptance.

### Serve RBAC and credential boundaries -- RESOLVED (2026-08-15)

`roko-serve` resolves four typed workspace roles against eleven permissions and applies
one route-permission layer to every mutation, sensitive reads, terminal routes, and
relay routes. JWT roles come from persisted membership rather than caller headers or
JWT role claims. API-key and agent-token registries are restart-safe and atomically
updated; agent capabilities are checked per operation. JWKS refresh/rotation,
issuer-bound multi-provider validation, invitation acceptance, and the shared auth
audit trail are live and tested. Stored Privy CLI credentials now use Bearer auth.

Agent-to-agent relay credentials are parent-linked, capability-narrowed, expiry-capped,
and depth-bounded. Every bearer use validates the complete chain to a live root agent
token; root and intermediate revocation invalidate their descendant subtrees. Relay
records share the restart-loaded, locked, atomic credential registry, and only secret
hashes are persisted. The recognized pre-T06 single-use format is intentionally
invalidated on upgrade because it has no trustworthy parent edge.

Local CLI commands still operate directly on the local workspace rather than traversing
serve RBAC. Device flow and the Cell/Graph-shaped auth pipeline remain target-architecture
work outside the completed E35 manifest.

### ACP mutation consent boundary -- RESOLVED (2026-08-15)

`write_file`, `edit_file`, and `bash` now emit a reply-channel permission event before
execution. The parent ACP stream sends `session/request_permission` to the editor and
only resumes the tool on `Allow` or `AlwaysAllow`; rejection, disconnect, cancellation,
a dropped reply, and transport failure deny without performing the mutation.
`AlwaysAllow` persists through the workspace trust store and suppresses later outbound
prompts for the same action. E17-T07/T08 subsequently added persisted session budgets
and health/rate-aware provider selection, completing the epic.

### Runtime gate dependency inversion -- RESOLVED (2026-08-15)

Canonical gate rung and determinism metadata now travel on the injected
`GateVerdict` contract. `roko-gate` populates that metadata and `roko-runtime`
consumes it without name-based classification helpers. The runtime crate no longer
has a normal dependency on `roko-gate`; the remaining edge is test-only.

### roko-acp compile issues -- RESOLVED (2026-08-13)

`roko-acp` is a workspace member and compiles on the current stable toolchain, including
through the `roko-cli` and `roko-serve` dependency graph. The former `PipelineConfig`
and import errors are stale audit findings.

### roko-orchestrator test failures -- RESOLVED (2026-08-13)

The authoritative crate suite is green in both default parallel mode and with
`--test-threads=1`: 576 unit tests, 3 integration tests, and 6 runnable doctests
(585 passed total), with 2 intentionally ignored doctests and no failures.

### Immune system screening coverage -- PARTIAL

The pure five-stage policy and fixed five-Cell runtime decision Graph now run automatically
for canonical provider primary outputs and every host-visible `ToolDispatcher` result.
Suspicious output is withheld and durably quarantined under canonical workspace authority;
High/Critical provider findings survive failed-attempt worktree deletion, while tool findings
enact durable cooldown/isolation and reciprocal incident links. Tool ingress, results, errors,
artifacts, panic reporting, and checkpoints have fixed count/byte ceilings and secret-safe
diagnostics. Provider evidence binds exact content identity plus provenance/attestation
security metadata, and replay recomputes the detector and policy decision.

The remaining gap is visibility and adaptation, not basic enforcement: provider-owned
internal calls/results and provider trace Signals are outside the primary-output boundary;
detectors are deliberately bounded rather than general semantic classifiers; no adaptive
immune-memory update loop exists; and historical receipts are not external signatures of the
current authority/vault state or proof against a wholesale rewrite of all local ledgers.

### Graph Engine incomplete -- PARTIAL

Bounded topological-wave execution now honors `max_concurrent_nodes` on the production
path, preserves outputs between waves, and retains in-process Hot Graph outputs between
ticks when `persist_tick_state` is enabled. Success, failure, always, and dotted
JSON-output equality edge conditions now select inputs and branches consistently across
sequential, bounded-parallel, snapshot-resume, and live-Flow execution; an untaken route
is represented explicitly and does not fail an otherwise successful Graph. The
converted plan path now dispatches real providers through an injected
`GraphTaskDispatcher`; explicit dry-run is diagnostic-only and unconfigured registries
fail closed. It durably records each successful Activity, resumes only an exact
schema/run/graph-fingerprint match, emits `GraphResumed`, and honors `--fresh`,
`--force-resume`, `--max-retries`, `--max-tasks`, `--budget-override`, and `--no-budget`.
The live dispatcher records actual provider cost for successful and unsuccessful paid
calls, supplies the tighter Cell/plan remainder to routing, blocks later Activities at a
non-overridden ceiling, marks last-call overage as plan/checkpoint failure, and reports
per-plan plus total cost. Seven typed cognitive Cells, T0
short-circuit, five Verify Cells, and the immune decision Graph are live. Hot Graphs now
commit graph-fingerprinted manifests and fsynced Activity logs per successful tick, restore
retained outputs and cumulative budgets after restart, replay interrupted Activities without
re-execution, archive exact state on fresh/forced replacement, and expose background
persistence failures; `roko agent serve` uses an agent/graph-scoped durable checkpoint.
The plan-cost ledger is now a schema-v2, atomic sidecar bound to plan, run, and graph
fingerprint. Spend plus in-flight reservations survive restart; missing, corrupt, mismatched,
or crash-reserved state fails closed, and admission/reservation is one mutex transaction so
parallel calls cannot collectively over-admit. The Graph host also enforces plan-level
dependencies that cannot be represented inside one
converted plan Graph. It validates the exact selected plan set before dry-run and before the
workspace lock is created, rejects duplicate IDs plus self, missing, and cyclic dependencies,
uses a deterministic topological order, and admits a dependent plan only after every
prerequisite has succeeded. Conversion, validation, execution, and budget failures therefore
block downstream plans transitively instead of allowing an unsafe partial batch.
Requesting `--approval` with Graph now fails before the workspace lock or provider setup
instead of warning and dispatching unapproved work; an actual Graph approval channel is still
required for parity.

The remaining Graph gap is Runner-v2
gates/replan/approval/worktree/merge/full-state-persistence/cancellation parity. One provider
call can still report more than its reservation after completion because the provider bridge
has no enforceable pre-call maximum-cost API; without `max_turn_usd`, calls serialize by
reserving all remaining capacity.
See `crates/roko-graph/src/` for details.

---

## Recently Resolved

### Batch 2026-08-14

- P19 ACP cascade integration completed: real selected-key dispatch, explicit session
  precedence, Daimon context, accurate direct-prompt observation, decision logs and
  episode metadata, plus fail-closed forced-model behavior
- P21 ACP streaming completed: CLI producers fan out human and structured progress,
  the bridge streams stdout/stderr live, correlates success/failure/retry tool calls,
  propagates child failures, and cancels the process tree without terminal completion
- E02 episode storage converged at `.roko/episodes.jsonl`; layout V3 migrates and
  archives root/learn/memory inputs, quarantines malformed bytes, deduplicates records,
  and removes legacy active logs after canonical replacement

### Batch 2026-08-13 (dogfood)

- First end-to-end dogfood run of roko against itself: `init` → `prd idea` → `prd draft` → `prd plan` → `plan run`
- PRD workflow (idea/draft/list/plan) works correctly, produces quality output
- Agent dispatch works: kimi-k2.5 dispatched via cascade router, wrote correct code (6 LOC)
- Four serial blockers prevented plan completion (config conflict, stale snapshot, fsmonitor, scheduler deadlock); all four now have focused regression fixes, with a clean dogfood rerun still pending
- E47 resource management completed: bounded concurrent JSONL append/rotation,
  retention-safe GC, symlink-safe target cleanup, disk-aware task admission, lifecycle
  cleanup, canonical worktree metrics, and the read-only `roko doctor disk` report
- Knowledge-informed model routing verified with focused positive, negative, and
  no-signal tests against the live `KnowledgeStore` path
- Scheduled cold-substrate archival verified end to end: only aged signals move to
  deduplicated cold storage, and hot data is pruned only after archive success
- The stale `roko-orchestrator` failure baseline was retired after 585 tests passed in
  both parallel and serial modes
- Provider health is fed directly by live workflow, bridge, and CLI outcomes without
  event-bus duplication; runner-v2 filters unhealthy knowledge-routing candidates
- Full debrief at `tmp/dogfood-2026-08-13/DOGFOOD-DEBRIEF.md`

### Batch 2026-08-12/13

- `Engram` renamed to `Signal` (type alias preserved for compatibility)
- `eprintln!` to `tracing` conversion (partial -- remaining sites are general workspace hygiene)
- `.expect()` to `Result` conversion (partial -- remaining sites are general workspace hygiene)
- docs/v1 deprecation headers added to legacy documentation
- docs/v2 status markers added to active documentation
- Playbook selection wired at dispatch time (PlaybookStore + record_outcome)
- roko-chain duplicate `Engram` struct removed from `identity_economy_markets.rs`

### Earlier resolutions

- **AgentContract runtime tool policy** -- role/task allowlists and denials are merged
  with deny precedence; unknown roles preserve deny-all; bridge and supported CLI paths
  enforce or reject instead of silently weakening the policy
- **Workspace locking** -- `flock`-based single-writer coverage across plan, PRD, run/do/develop, serve/daemon, and bounded managed-agent lifecycle mutations
- **RBAC** -- four typed roles, eleven permissions, persisted JWT membership resolution,
  route-wide enforcement, escalation guards, and structured audited denials in `roko-serve`
- **CLI compile errors** -- fixed (SH01-T07), missing types added
- **Incremental gate-threshold flush** -- `maybe_flush_gate_thresholds()` uses the
  configured `[learning].gate_threshold_flush_interval` (default 10)
- **FsObservabilitySinks** -- `flush_all()` on shutdown, 13 standard metrics
- **Runner v2 as default engine** -- since E01-T01; `--engine graph` is an opt-in
  live-dispatch path with incomplete Runner-v2 lifecycle parity

---

## Backlog — Specced Implementation Items

> Added 2026-08-17. Fully specced backlog items extracted from historical architecture docs
> (`tmp/architecture-archive/`) and ACP analysis (`tmp/acp-features/`, `tmp/acp-runner/`).
> Master index at `tmp/backlog/00-INDEX.md`. Each spec is self-contained.

### P0 — Critical

| # | Item | Size | Source |
|---|---|---|---|
| 17 | ACP stability hardening (7 P0 panics, 12 race conditions) | L | ACP analysis |

### P1 — High

| # | Item | Size | Source |
|---|---|---|---|
| 03 | Context injection scoping (per-role context sizing) | M | Orch gaps §5 |
| 04 | Compile auto-fix path (cargo fix before agent retry) | S | Orch gaps §2 |
| 07 | Inference cache L1/L2 (blake3 + SimHash) | L | Gateway spec |
| 18 | ACP spec upgrade v0.12→v0.13 + bridge_events refactor | XL | ACP analysis |

### P2 — Medium

| # | Item | Size | Source |
|---|---|---|---|
| 01 | T0 reflex store (zero-cost repeated decisions) | M | Agent runtime |
| 02 | Reactive agent mode (trigger-based wake/sleep) | L | Agent runtime |
| 05 | Express mode (skip strategist for trivial fixes) | M | Orch gaps §1 |
| 10 | Daimon TUI view (PAD gauges, somatic markers) | S | TUI ops |
| 12 | E2E test harness (multi-component spawn/health/cleanup) | M | TUI ops |
| 13 | Historical cost calibration (efficiency.jsonl → predictor) | S | Visual comp |
| 14 | Plan mutation protocol (typed PlanMutation enum) | M | Visual comp |
| 15 | Post-gate reflection (lightweight failure analysis agent) | M | Orch gaps §4 |

### P3 — Low / Phase 2+

| # | Item | Size | Source |
|---|---|---|---|
| 09 | Recursive safety continuous monitoring | L | Meta layer |
| 11 | Justfile (developer convenience) | XS | TUI ops |
| 16 | Warm agent spawning (pre-spawn during gates) | M | Orch gaps §6 |

Items 06, 07, 08 (output budgeting, inference cache, key rotation) were found to already
be implemented in `crates/roko-gateway/` during research. Their specs document remaining
wiring gaps (e.g., non-gateway code paths).

Items 04, 05, 15, 16 have partial implementations — types/scaffolding exist but runner
wiring is incomplete. Specs detail exactly what's present vs. missing.

---

## Executable Plan Status

`plans/INDEX.md` is deterministically rendered from inline task `status` values and checked
byte-for-byte without mutation in blocking CI. All executable plans are complete.

| Plan | Done/Total | Acceptance outcome |
|---|---|---|
| `architecture-production-residuals` | 4/4 | R01 supervised HTTP JSON connector; R02 bounded relay plus durable exact-room subscription execution; R03 authorized durable local arena service; R04 bounded meta-agent lineage and recursive safety |

The executable total is **124/124**. This is not interchangeable with the master
checklist's **20 unchecked** and **11 partial** raw markers: that wider document retains
repeated references, procedural/operational controls, documentation and dogfood proofs,
and product work that is not represented as a ready task manifest.

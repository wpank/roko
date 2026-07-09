# PRD / Implementation Audit Board

Audit date: 2026-04-22

This board is the working split between done, active, and blocked PRD files under:

- `tmp/04-21-26/demo-parity`
- `tmp/04-21-26/PRDs`
- `tmp/04-21-26/PRDs/impl`
- `tmp/04-21-26/PRDs/impl2`

Archive rule: a file only moves to `tmp/archive` when it is implemented, wired, and covered by a passing verification command. If a file still contains mocks, acceptance drift, missing scripts, missing live wiring, or only partial implementation, it stays active.

## Archived In This Pass

Moved to `tmp/archive/04-21-26/demo-parity/`:

| File | Why archived | Verification |
|---|---|---|
| `B1-job-types.md` | Canonical job model/store exists in `crates/roko-core/src/job.rs` and is exported from `roko-core`. | `cargo test -p roko-serve --test job_lifecycle --test job_runner_integration` passed |
| `B2-job-routes.md` | `/api/jobs` CRUD/lifecycle routes are wired through `roko-serve`. | same serve integration tests, 39 total tests passed |
| `B3-incremental-watchers.md` | Incremental JSONL tailer is wired into TUI dashboard refresh. | `cargo test -p roko-cli --no-run` passed earlier; code evidence in `tui/jsonl_tailer.rs` and `tui/dashboard.rs` |
| `B4-server-persistence.md` | `ServerStateSnapshot`, restore on startup, save on shutdown, and 30s auto-save are wired. | `cargo check -p roko-serve` passed |
| `B5-research-jobs.md` | Serve-owned job runner executes research jobs and writes `.roko/research/{job_id}.md`. | `research_job_creates_artifact_via_api` passed in `job_runner_integration` |
| `B7-heartbeats.md` | Heartbeat payloads, `/api/heartbeats`, `/api/network/stats`, and events are wired. | `heartbeat_post_and_list` and `network_stats_aggregates_heartbeats` passed |
| `B8-ws-enrichment.md` | Job events and heartbeat events serialize with type tags and flow through existing WS filters. | `test_job_events_fire_on_websocket` passed |
| `B9-auth-middleware.md` | API key and Bearer auth are accepted by serve middleware. | `cargo check -p roko-serve` passed; middleware tests are present in `routes/middleware.rs` |
| `B10-integration-test.md` | Bash smoke harness exists, is executable, matches current heartbeat payload contracts, and exercises jobs + heartbeats over HTTP. | `bash tmp/04-21-26/demo-parity/integration-test.sh` passed |
| `C1-marketplace-tab.md` | F8 Marketplace tab, subviews, header/status hints, and data loading are wired. | `cargo test -p roko-cli --test tui_tabs` passed |
| `C2-atelier-tab.md` | F9 Atelier tab and PRD/task view wiring are in place. | `cargo test -p roko-cli --test tui_tabs` passed |
| `C3-inspect-subviews.md` | F7 Inspect subviews render through typed `TuiState`; Engram DAG rows include confidence bars and explicit scroll/selection clamping; Knowledge Browse filtering uses `ViewState.search_query`. | `cargo test -p roko-cli --lib context_view -- --nocapture`; `cargo test -p roko-cli --lib signal_ -- --nocapture`; `cargo test -p roko-cli --test tui_tabs` passed |
| `C4-config-subviews.md` | F6 provider/model subviews are reachable via per-tab subview state and consistently use `Block::bordered()`. | `cargo test -p roko-cli --lib tui::` passed |
| `C6-header-stats.md` | Header stats show active-agent count, gate pass-rate ISFR, no-data fallback, and file fallback when agent data is empty. | `cargo test -p roko-cli --lib tui::` passed |

## Active Demo-Parity Files

| File | Status | Reason it stays active |
|---|---|---|
| `00-INDEX.md` | ACTIVE INDEX | Updated to point to this board and archive location. |
| `A1-project-setup.md` | PARTIAL / EXTERNAL | Dashboard repo has many target files, but exact PRD verification was not run and the app still has legacy/live-mock mixtures. |
| `A2-api-layer.md` | PARTIAL / EXTERNAL | Dashboard uses `rokoApi.ts`/`rokoWs.ts` rather than the exact `api.ts`/`ws.ts` plan; needs typecheck and contract audit. |
| `A3-landing-page.md` | PARTIAL / EXTERNAL | Landing components exist, but live data/reduced-motion/keyboard verification is not proven. |
| `A4-observatory-pages.md` | PARTIAL / EXTERNAL | Pages exist, but query invalidation and live endpoint contract verification remain unproven. |
| `A5-network-pages.md` | PARTIAL / EXTERNAL | Dashboard still has mock network/knowledge data paths. |
| `A6-marketplace-pages.md` | ACTIVE GAP | Backend `/api/jobs` now exists, but dashboard page still needs live hooks instead of mock jobs. |
| `A7-remaining-pages.md` | PARTIAL / EXTERNAL | Pages exist, but chat/research/atelier/settings workflows need live verification. |
| `A8-integration-polish.md` | PARTIAL / EXTERNAL | WS invalidation/right panel files exist, but browser verification and offline/reconnect behavior are not proven. |
| `A9-demo-rehearsal.md` | NOT DONE | Rehearsal flows have not been executed and recorded. |
| `B6-coding-jobs.md` | PARTIAL | Coding jobs transition, but the runner does not yet collect PRD/plan/gate/artifact payloads. |
| `C5-bug-fixes.md` | PARTIAL | Git parser hardening is in place; dashboard unified-log cache consumption still needs follow-up. |

## Active impl2 Gap PRDs

| File | Status | Reason it stays active |
|---|---|---|
| `00-INDEX.md` | ACTIVE INDEX | Keep as navigation for remaining gap PRDs. |
| `01-chain-integration.md` | PARTIAL | Chain config/watcher pieces exist, but tool registry, handler, and runtime context integration are not proven complete. |
| `02-config-unification.md` | PARTIAL | Some config routing improved, but old/new config split is not fully removed. |
| `03-event-bridge-and-serve-gaps.md` | PARTIAL | Event bridge/sidecar mounting improved; sidecar `/research` LLM dispatch remains open. |
| `04-gates-safety-supervisor.md` | PARTIAL / NOT DONE | Gate rung oracles, Claude safety enforcement, supervisor spawn wiring, and learned bidder propagation remain open. |
| `05-learning-neuro-corrections.md` | PARTIAL | Some docs/cache corrections landed; distillation fallback and experiment CLI are not fully proven. |
| `06-audit-evidence.md` | REFERENCE | Evidence file, not an implementation target. Keep until all linked PRDs close. |
| `07-dead-code-backend-gaps.md` | PARTIAL | Some backend gaps improved, but pool wiring, dead module cleanup, backend adapters, and dispatch health remain open. |

## Broad PRD Folders

`tmp/04-21-26/PRDs/impl/STATUS.md` already captures the honest high-level state: none of the broad architecture impl checklists are fully complete end-to-end. No files from `PRDs/impl` or parent `PRDs` were archived in this pass.

Keep these as active source/roadmap material:

| Path | Status |
|---|---|
| `PRDs/00-INDEX.md` and `PRD-*.md` | ACTIVE PRODUCT SOURCE |
| `PRDs/IMPL-*.md` | ACTIVE / SUPERSEDED BY SPLIT CHECKLISTS |
| `PRDs/impl/00-INDEX.md` | ACTIVE EXECUTION DAG |
| `PRDs/impl/STATUS.md` | ACTIVE STATUS SOURCE |
| `PRDs/impl/01-runtime/*` | NOT ARCHIVED |
| `PRDs/impl/02-cognitive-engine/*` | NOT ARCHIVED |
| `PRDs/impl/03-context-engineering/*` | NOT ARCHIVED |
| `PRDs/impl/04-knowledge-and-stigmergy/*` | NOT ARCHIVED |
| `PRDs/impl/05-domains-and-arenas/*` | NOT ARCHIVED |
| `PRDs/impl/06-isfr-and-instruments/*` | NOT ARCHIVED |
| `PRDs/impl/07-korai-chain/*` | NOT ARCHIVED |
| `PRDs/impl/08-surfaces-and-ux/*` | NOT ARCHIVED |
| `PRDs/impl/09-extensibility-and-multichain/*` | NOT ARCHIVED |
| `PRDs/impl/10-dashboard-and-tui/*` | NOT ARCHIVED |
| `PRDs/impl/11-demo-sprint/*` | NOT ARCHIVED |

## Broad PRD File Inventory

Parent `PRDs` files, all kept active:

| File | Status |
|---|---|
| `PRDs/00-INDEX.md` | ACTIVE SOURCE INDEX |
| `PRDs/PRD-01-OVERVIEW.md` | ACTIVE PRODUCT SOURCE |
| `PRDs/PRD-02-AGENT-RUNTIME.md` | ACTIVE PRODUCT SOURCE |
| `PRDs/PRD-03-COGNITIVE-ENGINE.md` | ACTIVE PRODUCT SOURCE |
| `PRDs/PRD-04-CONTEXT-ENGINEERING.md` | ACTIVE PRODUCT SOURCE |
| `PRDs/PRD-05-KNOWLEDGE-AND-STIGMERGY.md` | ACTIVE PRODUCT SOURCE |
| `PRDs/PRD-06-DOMAINS-AND-ARENAS.md` | ACTIVE PRODUCT SOURCE |
| `PRDs/PRD-07-ISFR-AND-INSTRUMENTS.md` | ACTIVE PRODUCT SOURCE |
| `PRDs/PRD-08-DEPLOYMENT-AND-UX.md` | ACTIVE PRODUCT SOURCE |
| `PRDs/PRD-09-EXTENSIBILITY-AND-MULTICHAIN.md` | ACTIVE PRODUCT SOURCE |
| `PRDs/PRD-10-DASHBOARD-AND-TUI.md` | ACTIVE PRODUCT SOURCE |
| `PRDs/IMPL-01-RUNTIME.md` | ACTIVE / SPLIT PLAN EXISTS |
| `PRDs/IMPL-02-COGNITIVE-ENGINE.md` | ACTIVE / SPLIT PLAN EXISTS |
| `PRDs/IMPL-03-CONTEXT.md` | ACTIVE / SPLIT PLAN EXISTS |
| `PRDs/IMPL-04-KNOWLEDGE.md` | ACTIVE / SPLIT PLAN EXISTS |
| `PRDs/IMPL-05-DOMAINS.md` | ACTIVE / SPLIT PLAN EXISTS |
| `PRDs/IMPL-06-ISFR.md` | ACTIVE / SPLIT PLAN EXISTS |
| `PRDs/IMPL-07-CHAIN.md` | ACTIVE / SPLIT PLAN EXISTS |
| `PRDs/IMPL-08-SURFACES.md` | ACTIVE / SPLIT PLAN EXISTS |
| `PRDs/IMPL-09-EXTENSIBILITY-AND-MULTICHAIN.md` | ACTIVE / SPLIT PLAN EXISTS |
| `PRDs/IMPL-10-DASHBOARD-AND-TUI.md` | ACTIVE / SPLIT PLAN EXISTS |
| `PRDs/IMPL-10-DEMO.md` | SUPERSEDED BY `demo-parity`, NOT ARCHIVED |

Split `PRDs/impl` files, all kept active because `STATUS.md` says the broad tracks are not end-to-end complete:

| File | Status |
|---|---|
| `impl/00-INDEX.md` | ACTIVE INDEX |
| `impl/STATUS.md` | ACTIVE STATUS SOURCE |
| `impl/01-runtime/00-overview.md` | ACTIVE |
| `impl/01-runtime/01-foundation-and-extraction-checklist.md` | NOT STARTED / ACTIVE |
| `impl/01-runtime/02-migration-verification-and-cutover.md` | NOT STARTED / ACTIVE |
| `impl/01-runtime/03-heartbeat-timescales-inference-gateway-and-ops.md` | PARTIAL / ACTIVE |
| `impl/02-cognitive-engine/00-overview.md` | ACTIVE |
| `impl/02-cognitive-engine/01-prediction-gating-and-triage-checklist.md` | NOT STARTED / ACTIVE |
| `impl/02-cognitive-engine/02-native-harness-costs-and-verification.md` | NOT STARTED / ACTIVE |
| `impl/02-cognitive-engine/03-thresholds-cascade-router-and-measurement.md` | PARTIAL / ACTIVE |
| `impl/03-context-engineering/00-overview.md` | ACTIVE |
| `impl/03-context-engineering/01-workspace-bidders-and-policy-checklist.md` | PARTIAL / ACTIVE |
| `impl/03-context-engineering/02-caching-chain-and-worldgraph-checklist.md` | NOT STARTED / ACTIVE |
| `impl/03-context-engineering/03-context-mesh-measurement-and-persistence.md` | NOT STARTED / ACTIVE |
| `impl/04-knowledge-and-stigmergy/00-overview.md` | ACTIVE |
| `impl/04-knowledge-and-stigmergy/01-knowledge-pipeline-and-hdc-checklist.md` | PARTIAL / ACTIVE |
| `impl/04-knowledge-and-stigmergy/02-publishing-dreams-and-chain-checklist.md` | NOT STARTED / ACTIVE |
| `impl/04-knowledge-and-stigmergy/03-insightstore-resonance-lifecycle-and-measurement.md` | NOT STARTED / ACTIVE |
| `impl/05-domains-and-arenas/00-overview.md` | ACTIVE |
| `impl/05-domains-and-arenas/01-domain-runtime-and-arenas-checklist.md` | NOT STARTED / ACTIVE |
| `impl/05-domains-and-arenas/02-domain-extensions-hf-and-market-checklist.md` | NOT STARTED / ACTIVE |
| `impl/05-domains-and-arenas/03-profile-catalog-custom-domains-and-scaling.md` | NOT STARTED / ACTIVE |
| `impl/06-isfr-and-instruments/00-overview.md` | ACTIVE |
| `impl/06-isfr-and-instruments/01-oracle-prediction-and-perps-checklist.md` | NOT STARTED / ACTIVE |
| `impl/06-isfr-and-instruments/02-clearing-runtime-and-verification-checklist.md` | NOT STARTED / ACTIVE |
| `impl/06-isfr-and-instruments/03-publication-states-economics-and-credibility.md` | NOT STARTED / ACTIVE |
| `impl/07-korai-chain/00-overview.md` | ACTIVE |
| `impl/07-korai-chain/01-consensus-execution-and-precompiles-checklist.md` | NOT STARTED / ACTIVE |
| `impl/07-korai-chain/02-insightstore-tokenomics-and-hdc-checklist.md` | NOT STARTED / ACTIVE |
| `impl/07-korai-chain/03-identity-registries-proof-log-and-rollout.md` | NOT STARTED / ACTIVE |
| `impl/08-surfaces-and-ux/00-overview.md` | ACTIVE |
| `impl/08-surfaces-and-ux/01-cli-chat-and-tui-checklist.md` | PARTIAL / ACTIVE |
| `impl/08-surfaces-and-ux/02-web-mcp-packaging-and-dx-checklist.md` | NOT STARTED / ACTIVE |
| `impl/08-surfaces-and-ux/03-product-surfaces-deployment-onboarding-security-and-observability.md` | PARTIAL / ACTIVE |
| `impl/09-extensibility-and-multichain/00-overview.md` | ACTIVE |
| `impl/09-extensibility-and-multichain/01-package-and-runtime-loading-checklist.md` | NOT STARTED / ACTIVE |
| `impl/09-extensibility-and-multichain/02-ingestion-discovery-and-worldgraph-checklist.md` | NOT STARTED / ACTIVE |
| `impl/09-extensibility-and-multichain/03-finetuning-integration-and-acceptance.md` | NOT STARTED / ACTIVE |
| `impl/09-extensibility-and-multichain/04-attention-allocation-publishing-and-ecosystem-completion.md` | NOT STARTED / ACTIVE |
| `impl/10-dashboard-and-tui/00-overview.md` | ACTIVE |
| `impl/10-dashboard-and-tui/01-stabilization-and-nexus-checklist.md` | PARTIAL / ACTIVE |
| `impl/10-dashboard-and-tui/02-dashboard-rewrite-checklist.md` | NOT STARTED / ACTIVE |
| `impl/10-dashboard-and-tui/03-tui-polish-and-cross-surface-verification.md` | PARTIAL / ACTIVE |
| `impl/10-dashboard-and-tui/04-page-catalog-widgets-data-contracts-and-network-intelligence.md` | PARTIAL / ACTIVE |
| `impl/11-demo-sprint/00-overview.md` | ACTIVE |
| `impl/11-demo-sprint/01-dashboard-stream-checklist.md` | NOT STARTED / ACTIVE |
| `impl/11-demo-sprint/02-backend-and-tui-stream-checklist.md` | PARTIAL / ACTIVE |
| `impl/11-demo-sprint/03-rehearsal-and-demo-acceptance.md` | NOT STARTED / ACTIVE |

## Verification Run

- `cargo test -p roko-serve --test job_lifecycle --test job_runner_integration` passed: 21 + 18 tests.
- `cargo test -p roko-cli --test tui_tabs` passed: 10 tests.
- Earlier branch verification still applies: `cargo check -p roko-serve`, `cargo test -p roko-agent-server --no-run`, `cargo test -p roko-cli --test cli_fallback`, and `cargo test -p roko-cli --no-run` passed.

## Next Best Work Queue

1. Finish `B6-coding-jobs.md`: artifact and gate-result collection for coding jobs.
2. Finish dashboard Stream A live-data cleanup, starting with `A6-marketplace-pages.md` because the backend is ready.
3. Finish the remaining TUI polish gap in `C5-bug-fixes.md`.
4. Then return to `impl2/03`, `impl2/02`, and `impl2/07` for the remaining integration gaps.

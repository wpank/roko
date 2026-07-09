# Implementation Status — Honest Audit (2026-04-22)

**None of the impl checklists are fully complete end-to-end.**
Most are PARTIAL at best. The codebase has a "built but not wired" pattern
where code exists but data doesn't flow to it.

## Legend
- DONE: Fully works, tested, verified end-to-end
- PARTIAL: Some items wired, some stubbed or broken
- NOT STARTED: Code may exist but nothing is connected or functional

---

## 01-runtime/ — Runtime Extraction
| File | Status | Notes |
|------|--------|-------|
| 01-foundation-and-extraction | NOT STARTED | orchestrate.rs is still 20K LOC monolith. No AgentRuntime type extracted. |
| 02-migration-verification | NOT STARTED | Blocked by 01 |
| 03-heartbeat-timescales-ops | PARTIAL | ServerEvent::Heartbeat exists (10s interval), but no timescales, no inference gateway, no supervision hardening |

## 02-cognitive-engine/ — Prediction Gating
| File | Status | Notes |
|------|--------|-------|
| 01-prediction-gating-triage | NOT STARTED | T0/T1/T2 gating not in dispatch loop. Everything goes to LLM. |
| 02-native-harness-costs | NOT STARTED | No native harness for T0 operations |
| 03-thresholds-cascade-router | PARTIAL | CascadeRouter persists to disk, adaptive thresholds EMA works, but feedback loop from gate results to routing decisions is incomplete |

## 03-context-engineering/ — Context Assembly
| File | Status | Notes |
|------|--------|-------|
| 01-workspace-bidders-policy | PARTIAL | ContextAssembler wired into orchestrate.rs, VCG auction code exists, 3 bidder types (Neuro/Task/Research) exist. Not all bidders produce real bids. |
| 02-caching-chain-worldgraph | NOT STARTED | No WorldGraph, no chain context, no context caching |
| 03-context-mesh-measurement | NOT STARTED | No context mesh, no section-effect measurement |

## 04-knowledge-and-stigmergy/ — Knowledge Pipeline
| File | Status | Notes |
|------|--------|-------|
| 01-knowledge-pipeline-hdc | PARTIAL | HDC vectors computed, neuro store built, fingerprints per-episode. But pipeline not connected end-to-end for retrieval at dispatch time. |
| 02-publishing-dreams-chain | NOT STARTED | roko-dreams crate exists (scaffold only). No chain publishing. |
| 03-insightstore-resonance | NOT STARTED | No live InsightStore queries or publishing |

## 05-domains-and-arenas/ — Domain Specialization
| File | Status | Notes |
|------|--------|-------|
| 01-domain-runtime-arenas | NOT STARTED | Domain profiles exist in config schema only. No arena framework. |
| 02-domain-extensions-hf | NOT STARTED | No HuggingFace integration, no work markets |
| 03-profile-catalog-scaling | NOT STARTED | No domain catalog |

## 06-isfr-and-instruments/ — Financial Primitives
| File | Status | Notes |
|------|--------|-------|
| 01-oracle-prediction-perps | NOT STARTED | ISFR types in roko-chain but no live oracle |
| 02-clearing-runtime | NOT STARTED | No clearing runtime |
| 03-publication-economics | NOT STARTED | No solver economics |

## 07-korai-chain/ — Blockchain
| File | Status | Notes |
|------|--------|-------|
| 01-consensus-execution | NOT STARTED | Chain crate has types/client/wallet, no consensus engine |
| 02-insightstore-tokenomics | NOT STARTED | No on-chain InsightStore |
| 03-identity-registries | NOT STARTED | No identity registries |

## 08-surfaces-and-ux/ — CLI, Chat, TUI
| File | Status | Notes |
|------|--------|-------|
| 01-cli-chat-tui | **PARTIAL (80%)** | CLI 36+ commands work. `roko chat` works with reconnect. TUI F1-F9 work with push updates. Agent lifecycle commands wired. F8 Marketplace and F9 Atelier tabs restored. Grouped help added. Error hints added. Env var overrides (ROKO_MODEL/EFFORT/ROLE/QUIET/LOG_FORMAT). Confirmation prompts for destructive ops. |
| 02-web-mcp-packaging | NOT STARTED | roko-mcp-code built but not called from runtime. No web packaging. |
| 03-product-surfaces-deploy | PARTIAL | `roko doctor` has 10 checks. Deployment parity check exists. BUT: AI Studio, Agent Studio, OpenClaw are design docs only. No onboarding flows. |

## 09-extensibility-and-multichain/ — Package System
| File | Status | Notes |
|------|--------|-------|
| 01-package-runtime-loading | NOT STARTED | No package system |
| 02-ingestion-worldgraph | NOT STARTED | No multi-chain ingestion |
| 03-finetuning-integration | NOT STARTED | No fine-tuning export |
| 04-attention-publishing | NOT STARTED | No attention allocation |

## 10-dashboard-and-tui/ — Dashboard + TUI Stabilization
| File | Status | Notes |
|------|--------|-------|
| 01-stabilization-nexus | **PARTIAL (70%)** | Jobs backend with state machine works. Auth consolidated (Bearer + API key). WS topic-based filtering works. Relay types and route wired. Server persistence improved. Jobs auto-reload on startup. BUT: Nexus relay not fully functional beyond types+route. |
| 02-dashboard-rewrite | NOT STARTED | This is the nunchi-dashboard React repo. Not touched. |
| 03-tui-polish | PARTIAL (60%) | F1-F9 render, push-based updates work, connection indicator exists. F8/F9 wired (Marketplace + Atelier tabs, views, input handlers, header + status bar hints). No cross-surface parity verification. No command palette. |
| 04-page-catalog-widgets | PARTIAL (30%) | ParityMatrix type exists, WidgetState enum exists, ProjectionEnvelope exists. BUT: not integrated into actual views. Projection catalog route exists. |

## 11-demo-sprint/ — Demo Preparation
| File | Status | Notes |
|------|--------|-------|
| 01-dashboard-stream | NOT STARTED | Dashboard rewrite not done |
| 02-backend-tui-stream | **PARTIAL (60%)** | Jobs backend works, heartbeat publisher works, retention policy exists. TUI marketplace/atelier views restored. Job routes functional. Incremental JSONL tailer infrastructure for TUI tick optimization. |
| 03-rehearsal | NOT STARTED | Never rehearsed end-to-end |

---

## What Actually Works Right Now

The **core self-hosting loop** works:
```bash
roko init                    # creates workspace
roko prd idea "..."          # captures idea
roko prd draft new "..."     # creates PRD
roko prd plan <slug>         # generates plan + tasks.toml
roko plan run plans/         # executes (agents + gates + persist)
roko plan run --approval     # with TUI (F1-F9, all tabs wired)
roko dashboard               # standalone TUI (file polling)
roko serve                   # HTTP API (85+ routes)
roko status                  # health check
roko doctor                  # 10 diagnostics
roko chat --agent <id>       # chat with reconnect
```

## What's Broken or Rough

1. **TUI in --approval mode** — was crashing, fixed in this session (verdicts.rs runtime, ws_client.rs fallback, PanicHookRestoreGuard). May still have edge cases.
2. ~~**F8/F9 tabs reverted**~~ — FIXED: Marketplace (F8) and Atelier (F9) fully wired in Tab enum, input dispatch, views, header bar, status bar
3. **Agent lifecycle** — `agent list/start/stop/status` were added then reverted by linter
4. **Nested .roko** — if you run from inside `.roko/`, it creates `.roko/.roko/` and TUI reads wrong one
5. **Plan discovery** — picks up INDEX.md as a plan when it shouldn't
6. **Config v1 warnings** — spams "roko.toml uses config version 1" on every operation

## Priority to Make UX Not Suck

1. ~~Fix F8/F9 tabs~~ — DONE (Marketplace + Atelier fully wired)
2. Re-add agent list/start/stop/status commands
3. Stop INDEX.md from being treated as a plan
4. Suppress config v1 warning spam
5. Add progress indicators for plan execution
6. Make --approval mode rock-solid

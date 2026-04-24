# Roko Subsystem Audits

## Runner Status (2026-04-28)

### Arch Runner (Phase 0-4: Foundation)

Phases 0-4 of the [MASTER-IMPLEMENTATION-PLAN.md](MASTER-IMPLEMENTATION-PLAN.md) are **complete**. The arch runner (`tmp/runners/arch/`) executed 16 batches via Codex (gpt-5.5), all passing structural + compilation + anti-pattern verification:

| Phase | Batches | Status | Branch |
|-------|---------|--------|--------|
| Phase 0: Core Types & Traits | P0A-P0C (3) | **Done** | `codex/arch-run-20260428-012508` |
| Phase 1: Foundation Services | P1A-P1D (4) | **Done** | same |
| Phase 2: Execution Engine | P2A-P2D (4) | **Done** | same |
| Phase 3: Adapters | P3A-P3C (3) | **Done** | same |
| Phase 4: Wiring | P4A-P4B (2) | **Done** | same |

**New modules created**: `runtime_event.rs`, `foundation.rs`, `model_call_service.rs`, `prompt_assembly_service.rs`, `feedback_service.rs`, `gate_service.rs`, `pipeline_state.rs`, `task_scheduler.rs`, `effect_driver.rs`, `workflow_engine.rs`, `acp_adapter.rs`, `adapters.rs` (serve), `jsonl_logger.rs`, `projection.rs`

**Anti-patterns addressed**: #1 (shell out to claude → ModelCallService), #2 (inline prompts → PromptAssemblyService), #3 (3 runtimes → 1 WorkflowEngine), #6 (feedback afterthought → FeedbackService), #7 (copy between runtimes → shared services), #10 (god file → focused services).

### Converge Runner (87 batches: wiring + tests + security + demo)

The converge runner (`tmp/runners/converge/`) wired the arch runner's foundation services into live code paths, added integration tests, CLI output formatting, security hardening, and layer enforcement. **83/87 succeeded**, 4 failed (R-track: feature-gating orchestrate.rs).

| Track | Batches | Status | Summary |
|-------|---------|--------|---------|
| F (Foundation) | 6 | **Done** | Fix crate cycle, unify trait duplication |
| S (Services) | 13 | **Done** | ModelCall, PromptAssembly, Feedback, Gate production-ready |
| E (Engine) | 8 | **Done** | Config loading, checkpoint/resume, commit, save |
| W (Wiring) | 8 | **Done** | Connect to CLI, plan run, ACP, serve |
| O (Observability) | 6 | **Done** | JsonlLogger, Projection, StateHub, CLI progress |
| R (Retirement) | 1/5 | **Partial** | R01 done; R02-R05 failed (orchestrate.rs too complex) |
| C (CLI/Demo) | 12 | **Done** | Output format, --share, dashboard pages |
| T (Tests) | 5 | **Done** | WorkflowEngine, CLI flags, share endpoint |
| D (Daimon) | 4 | **Done** | AffectPolicy trait, DaimonPolicy, wiring |
| G (Gateway) | 9 | **Done** | Unified provider abstractions, gateway events |
| K (Knowledge) | 5 | **Done** | Knowledge routing, injection, feedback loop |
| X (Security) | 2 | **Done** | Fail-closed contracts (X01), parser consolidation |
| L (Layering) | 4 | **Done** | Layer metadata, layer-check, cargo-deny, CI |

**Post-merge audit**: 10 critical, 25 warning, 20 note issues found. 4 critical fixed. See **[converge-runner/](converge-runner/)** for full audit, fixes applied, and open issues checklist.

Per-subsystem workspace for refactoring roko from hardcoded monolith to dynamic, composable, configurable building blocks. Each folder contains:

- `AUDIT.md` — Current state assessment, anti-patterns, gaps
- `GOALS.md` — Desired end state, key properties, feature gaps
- `FEATURES.md` — Feature inventory with status (where applicable)
- `PLAN.md` — Implementation plan (TODO)
- `ISSUES.md` — Known issues and blockers (TODO)

## Start Here

- **[MASTER-IMPLEMENTATION-PLAN.md](MASTER-IMPLEMENTATION-PLAN.md)** — Prioritized implementation plan: T0-T8 tiers, 100+ tasks, dependency graph, cross-references to all sources
- **[VISION.md](VISION.md)** — Master vision: roles as config, composable pipelines, cybernetic prompts, self-learning, visual canvas, marketplace
- **[UNIFIED-IMPLEMENTATION-PLAN.md](UNIFIED-IMPLEMENTATION-PLAN.md)** — 80+ tasks across 7 phases to converge all runtimes (technical detail for T0-T1)
- **[ANTI-PATTERNS-V2.md](ANTI-PATTERNS-V2.md)** — current reusable anti-pattern catalog from the 05-01 and converge audits; use this for future agent prompts
- **[AGENT-FAILURE-PATTERNS.md](AGENT-FAILURE-PATTERNS.md)** — compact runner prompt/review checklist for avoiding the same agent mistakes
- **[ANTI-PATTERNS.md](ANTI-PATTERNS.md)** — legacy 2026-04-28 anti-patterns; several "resolved" statuses were later disproven by the 05-01 audit
- **[acp-protocol/FEATURES.md](acp-protocol/FEATURES.md)** — Full ACP feature inventory from code exploration + UX showcase mockups
- **[gtm/ADAPTER-PHILOSOPHY.md](gtm/ADAPTER-PHILOSOPHY.md)** — Design principle: adapter-first extensibility. 7 ecosystem patterns, adapter trait rules, implementation path
- **[gtm/INTEGRATIONS.md](gtm/INTEGRATIONS.md)** — 18 high-value integrations ranked by ROI with adapter interface designs
- **[gtm/ADVANCED-PATTERNS.md](gtm/ADVANCED-PATTERNS.md)** — 12 multiplicative design patterns: event sourcing, CAS, effect systems, middleware stacks, federated learning, and more
- **[gtm/NEW-MARKETS.md](gtm/NEW-MARKETS.md)** — 24 integration categories unlocking new user segments: DevOps/SRE, security, data engineering, compliance, IoT, fintech, bioinformatics, smart-contract audit, FHIR healthcare, quant finance, and more
- **[gtm/SYNERGY-PATTERNS.md](gtm/SYNERGY-PATTERNS.md)** — 15 composability patterns: network effects, data flywheels, compounding learning, marketplace dynamics, interoperability as moat, Cursor unbundling thesis, continuous compliance attestation, bandit experiment promotion
- **[gtm/MOAT-ANALYSIS.md](gtm/MOAT-ANALYSIS.md)** — Honest moat stack analysis: 5-layer defensibility (data, ecosystem, standards, workflow-embedding, chain) with empirical evidence from Terraform/Zapier/Airbyte/Vanta, gateway position economics, compound rates, and pitch-ready framing
- **[gtm/PITCH-INTELLIGENCE.md](gtm/PITCH-INTELLIGENCE.md)** — Investor thesis mapping: Aubakirova vocabulary to adapter surfaces, competitive positioning vs LangGraph/AutoGen/CrewAI, HAL benchmark cost data, demo-as-product adapter mapping, Keycard/ERC-8004 sovereignization as AuthAdapter generalization

## Subsystem Folders

### Runtime & Execution

| Folder | Audit | LOC | Summary |
|---|---|---|---|
| [orchestration/](orchestration/) | 3 runtimes, 2 state machines | ~25K | ACP pipeline + Runner v2 + orchestrate.rs monolith (21K dead). Features silently deactivate when switching runtimes. |
| [acp-protocol/](acp-protocol/) | JSON-RPC editor integration | ~6.5K | Cleanest architecture (pure FSM), but isolated silo — no learning, no safety, no episodes. |
| [gate-pipeline/](gate-pipeline/) | 7-rung verification | ~19K | 3 separate dispatch paths. Rungs 3-6 return stubs. LLM judge bypasses ModelCallService. |

### Intelligence & Learning

| Folder | Audit | LOC | Summary |
|---|---|---|---|
| [learning-feedback/](learning-feedback/) | 10 learning components | ~15K | CascadeRouter, experiments, playbooks, conductor — all fully built, all only wired from dead code. |
| [cognitive-layer/](cognitive-layer/) | Neuro, dreams, daimon, pheromones | ~110K | Neuro + dreams = keep. Daimon 40K LOC = replace with FailureTracker. Pheromones 68K = delete. |
| [code-intelligence/](code-intelligence/) | Symbol graphs, HDC, MCP | ~8.2K | Solid but under-utilized. HDC similarity disabled in prompt assembly. Index rebuilt fresh every time. |

### Prompt & Dispatch

| Folder | Audit | LOC | Summary |
|---|---|---|---|
| [inference-dispatch/](inference-dispatch/) | 13+ LLM call sites | varies | 4 spawn mechanisms, 4 copies of stream-json parsing, CascadeRouter only from dead code. |
| [prompt-assembly/](prompt-assembly/) | 9-layer SystemPromptBuilder | ~5K | Only 1 of 6+ entry points uses full builder. VCG auction overengineered. Inline prompts everywhere. |

### User Interface

| Folder | Audit | LOC | Summary |
|---|---|---|---|
| [cli-chat-tui/](cli-chat-tui/) | 5 modes, 2 terminal systems | ~45K | chat_inline.rs (4.1K, 2 near-identical loops). Inline + fullscreen TUI share zero code. |
| [ux/](ux/) | Task management UX across all surfaces | ~52K | Board→Epic→Task hierarchy, DAG-first execution, agent-driven enrichment. 156 features inventoried: 22% wired, 56% not built. Mori reference analysis from 19 screenshots + full code exploration. |

### Infrastructure

| Folder | Audit | LOC | Summary |
|---|---|---|---|
| [http-persistence/](http-persistence/) | ~175 routes, 50+ persistence files | ~6K serve | Good patterns (StateHub, atomic JSON, ArcSwap). Issues: persistence duplication, no transactional multi-file writes. |
| [safety-agent/](safety-agent/) | 8 backends, 10-stage tool dispatch | ~5K+ | Architecturally sound. Critical: contracts fail open on missing YAML. Recovery actions never invoked. |
| [config-tools-events/](config-tools-events/) | Config, 16 tools, 32-kind signals, plugins | ~33K | Foundational plumbing. No config hot-reload. Signal decay not learned. Plugin system has no isolation. |
| [chain-deploy-demo/](chain-deploy-demo/) | Blockchain, Railway, tournament | ~42K | Chain: 14K+ dormant code. Deploy: fully wired Railway/Docker/daemon. Demo: standalone benchmark harness. |

### New Subsystems (Planned)

| Folder | Audit | Summary |
|---|---|---|
| [gateway/](gateway/) | LLM inference proxy (from bardo-gateway) | Transparent HTTP proxy with 3-layer caching, 20+ provider routing, cost tracking, budget enforcement, safety, micropayments. Clean reimplementation as `roko-gateway` library crate + standalone binary. Business model for nunchi hosted service. |

### Cross-Cutting: GTM & Extensibility

| Folder | Audit | Summary |
|---|---|---|
| [gtm/](gtm/) | Adapter-first extensibility & go-to-market integrations | 10 documents: adapter philosophy (+ R5: Bevy-style plugin trait, fn-as-plugin blanket impl, derive macro, conformance test crate; + R6: substrate vs gateway ordering, 90-day shipping sequence, recipe.toml Chain E pattern, 3-system minimum workflow; + R8: Codex CLI forces sharper adapter-trait messaging, vendor-neutral OTel as marketing hook, recipe.toml observability shape, contrast frame vs Codex), per-subsystem adapter map, 18 integrations ranked by ROI (+ R5: 5 operational integration specs with LOC estimates -- octocrab, gen_ai OTel, Linear AgentSession, Sentry Seer MCP, slack-morphism; recipe.toml schema; + R6: re-ordered GitHub->Linear->Slack->OTel->Sentry with competitor evidence table, killer demo Chain D, Langfuse partnership, Sentry Seer as partial competitor; + R8: Linear protocol deep-dive -- two latency budgets 5s/10s, emit-then-async orchestration, 5 activity types, promptContext XML, OAuth actor=app scopes, HMAC-SHA256 signing, Linear-Delivery dedup, 11+ shipped agents with Cursor broken, no usable Rust SDK, 10-14 day v1 estimate, graphql_client v0.14.0 workflow; corrected observability -- Langfuse acquired by ClickHouse Jan 2026, Arize Phoenix is ELv2 not Apache-2.0, opentelemetry-langfuse bus-factor-1 warning, vendor-neutral OTel via opentelemetry-otlp directly, Langfuse partnership process, Helicone out, Laminar as alternative), 7 ecosystem patterns from 12 platforms (+ R5: verified contributor funnel data; + R6: Supabase activation keystone, event-driven emails, Day-2 shareable artifact, personal outreach 2-3x, k=0.2, Roko Week, n8n 3-system template, crates.io download mechanics, TWiR, 5 compounding chains A-E with named precedents, dominant template shape, retention insight; + R8: AAIF 170+ orgs in <4 months surpassing CNCF, no EU Platinum member in MCP governance, SEP process public/free, MCPCon Europe/NA dates, MCP 10K servers with 52% abandonment, A2A v1.0 150+ orgs, Shai-Hulud 3 Bitwarden CLI attack, 7 ranked ecosystem touchpoints), gateway adapter stack (+ R5: gen_ai OTel semantic conventions; + R6: Langfuse preferred partner, Arize Phoenix backup, skip Helicone/Datadog, gen_ai.* agent spans; + R8: Linear wire protocol specification -- webhook header/payload, action values, activity types, OAuth scopes, HMAC verification, graphql_client workflow, custom scalars module, tokezooo/linear-agent-bridge reference, schema-per-release locking), 12 advanced design patterns (+ R5: Bevy + conformance test references; + R6: 5 chains as pattern, Sigstore/in-toto at agent boundary, recipe-as-template Chain E, build-time to agent-action-time verification, Days 2-7 retention patterns, 3-tier commercial offering, free-tier auto-pause, Ferrous/HashiCorp/PlanetScale precedents; + R8: 6 Common Paper DPA gotchas, Yetto DPA reference, Temporal correction -- inherited Cadence customers not greenfield, services-attached-to-software portable tactic, Ferrous training-as-funnel model), 24 new market categories (+ R5: gen_ai OTel for DevOps/SRE; + R6: Sigstore/supply-chain post-Shai-Hulud, Linear AgentSession entry point, Berlin grants detailed -- NLnet/STF/Fellowship/Rust Foundation specifics, Rust demographics 48.8% production + JetBrains 30% newcomers + Cargo/uv most-admired, conference priority ranking, Berlin salary calibration EUR 95-110k, TWiR submission; + R8: corrected Berlin connector roster -- Albini/Oxide, Klock/AWS, Hertleif/Cologne, verified Gilchers/Liran/Rediger/Gruber/Goffart/Hausmann, Ferrous Wallstr. 59 details + IEC 61508 SIL 2 + Sonair/Kiteshield customers, grant corrections -- NLnet 14th call June 1 + STF renamed Sovereign Tech Agency + Fellowship 2026 closed + Rust Foundation low-thousands bands, KubeCon NA Nov 9-12 2026 + KubeCon EU 2028 Berlin + AI Engineer World's Fair June 29-July 2), 21 synergy/composability patterns (+ R5: Pattern 16 Recipe Compression, Pattern 17 Verification Badge Gravity; + R6: Pattern 18 Compounding Integration Chains with 5 named chains A-E, Pattern 19 Event-Triggered Shareable Artifacts, Pattern 20 Design Partner Revenue Loop -- Common Paper v1.3 + $48k ARR in 90 days + Temporal precedent; + R8: Pattern 21 AAIF Governance Leverage -- SEP authorship + Technical Committee as under-represented EU voice, protocol-level influence, adapter standard alignment, reference implementation status), honest moat stack analysis (+ R5: ecosystem acceleration evidence; + R6: Temporal 18-month monetization timeline, Linear free-agent pricing moat, Rust 10-second window performance moat, Sigstore post-Shai-Hulud demand, ERC-8004 too early narrative re-anchor, relicensing cautionary tales, Ferrous pricing precedent; + R8: Codex CLI competitive collision -- Apache-2.0 Rust 72K stars collapses generic positioning, 4-pillar differentiation that survives, Apache-2.0 niche uncontested for runtimes, emit-then-async is the real moat, Linear ecosystem crowded but Cursor broken), pitch intelligence (+ R5: Rust Foundation Maintainers Fund, RustConf 2026, Supabase growth model; + R6: $48k ARR target, Common Paper DPA v1.3 benchmarks, Temporal analog, Berlin grants EUR 80-150k stack, EuroRust pitch venue, investor framing for Sigstore narrative, OpenAI/Anthropic Rust acquisitions, "stars don't gate monetization"; + R8: La Famiglia->GC merger, Cavalry->NAP rebrand, 468 Capital $1.3B+ Berlin-anchored AI thesis, Air Street $232M Fund III largest solo-GP Europe, Cherry Fund V $500M, AAIF SEP as governance lever, Ferrous as channel partner, @roko vs @cursor demo script). |

## Cross-Cutting Anti-Patterns

| # | Pattern | Subsystems Affected | Status |
|---|---|---|---|
| 1 | Shell out / raw provider dispatch | inference-dispatch, ACP, chat, cognitive-layer | Still recurring in surface paths |
| 2 | Inline prompt strings / prompt bypass | prompt-assembly, ACP, chat | Partially addressed; live path coverage incomplete |
| 3 | Build another runtime / shadow runtime | orchestration, ACP, serve, run | Still recurring via legacy fallbacks and divergent entry points |
| 4 | Features in wrong layer | gate-pipeline, orchestration, demos | Still recurring when surfaces patch shared-layer gaps |
| 5 | Hardcoded role/model/provider behavior | prompt-assembly, config, gate-pipeline | Still recurring through defaults and fallback strings |
| 6 | Feedback as afterthought / optional feedback | learning-feedback, gate-pipeline, chat, ACP | Still recurring in live entry points |
| 7 | Copy between runtimes | inference-dispatch, gate-pipeline, state/event code | Still recurring; see V2 categories A/H |
| 8 | Parse output/debug strings as contracts | inference-dispatch, cli-chat-tui, runtime projection | Still recurring; see V2 category G |
| 9 | Transient or lossy state | learning-feedback, http-persistence, workflow reports | Partially addressed; report truth still inferred from events |
| 10 | God file / accumulation | orchestration, ACP bridge, runner types | Still a risk; needs CI fitness checks |

## LOC Summary

| Category | LOC | % of ~177K |
|---|---|---|
| Active + well-designed | ~40K | 23% |
| Active + needs refactoring | ~25K | 14% |
| Built but only wired from dead code | ~30K | 17% |
| Dormant / overengineered | ~82K | 46% |

**Net plan:** ~110K LOC to delete/simplify, ~5K replacement code.

## Related Resources

- `tmp/workflow/` — Original audit docs (01-17) and comparison documents
- `tmp/mori-diffs/` — 41-document audit package from mori migration
- `tmp/mori-diffs/29-CURRENT-RUNTIME-GAP-LEDGER.md` — Canonical gap tracker
- `tmp/acp-features/00-ACP-FEATURES.md` — Original ACP features checklist (source for acp-protocol/FEATURES.md)
- `tmp/unified/` — v3.0 spec documents (Cell/Graph kernel, cybernetic loops, Generative Canvas)
- `~/Downloads/roko_acp_showcase_v2.jsx` — v2 UX showcase JSX (~4K LOC, 9 scenarios, 16 message types, full right-rail panels)

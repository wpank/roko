# ux-impl Batches

Runner: `tmp/runners/ux-impl/run.sh`.
Tracker: `ISSUE-TRACKER.md`.
Source plans: `tmp/ux/implementation-plans/{01..12}-*.md`.

## Wave M — Mirage extraction (plan 01)

| Batch | Title | Scope | Deps | Verify |
|-------|-------|-------|------|--------|
| M01 | Audit no live consumers | docs/v2/MIRAGE-CONSUMER-AUDIT.md | — | quick |
| M02 | Drop chain → dashboard-api implication | apps/mirage-rs/Cargo.toml | M01 | quick |
| M03 | Slim http_health.rs | apps/mirage-rs/src/{http_health,main,lib}.rs | M02 | quick |
| M04 | Delete chain/ http_api/ roko_bridge/ | apps/mirage-rs/{Cargo.toml, src/{lib,main}.rs} | M03 | full |
| M05 | Drop chain_* JSON-RPC methods | apps/mirage-rs/src/rpc.rs | M04 | quick |
| M06 | Rewire scenario.rs if needed | apps/mirage-rs/src/scenario.rs | M04, M05 | quick |
| M07 | Update docs + close plan | apps/mirage-rs/README.md, CLAUDE.md, plan 02 | M04, M05, M06 | quick |

## Wave AG — Aggregator backends (plan 02)

| Batch | Title | Scope | Deps | Verify |
|-------|-------|-------|------|--------|
| AG01 | Capture legacy fixtures | tmp/runners/ux-impl/fixtures/ | — | quick |
| AG02 | InsightBoardReader | crates/roko-chain/src/insight_board.rs | — | quick |
| AG03 | AgentCardFetcher | crates/roko-chain/src/agent_card_fetcher.rs | — | quick |
| AG04 | KnowledgeSource enum on AppState | crates/roko-serve/src/state.rs | AG02, AG03 | quick |
| AG05 | Replace knowledge handlers | crates/roko-serve/src/routes/aggregator.rs | AG04, AG01 | quick |
| AG06 | PheromoneField module | crates/roko-serve/src/{pheromone,lib}.rs | — | quick |
| AG07 | Pheromone routes | crates/roko-serve/src/routes/aggregator.rs | AG06, AG01 | quick |
| AG08 | Chain event subscription → invalidate cache | crates/roko-serve/src/{lib,state}.rs | AG02, AG05 | quick |
| AG09 | Compat regression tests | crates/roko-serve/tests/{knowledge,pheromone}_compat.rs | AG05, AG07 | full |
| AG10 | OpenAPI + CLAUDE.md + close plan | crates/roko-serve/src/openapi.rs, CLAUDE.md, plan 04 | AG07, AG09 | quick |

## Wave CH — ERC-8004 chain discovery (plan 04)

| Batch | Title | Scope | Deps | Verify |
|-------|-------|-------|------|--------|
| CH01 | IdentityRegistry.sol updateAgentCard + capabilityMask | contracts/{src,test} | — | quick |
| CH02 | IdentityRegistryReader in roko-chain | crates/roko-chain/src/identity_registry.rs | CH01 | quick |
| CH03 | roko-core::capability_bits | crates/roko-core/src/capability_bits.rs | — | quick |
| CH04 | Agent-server registration uses bit 15 | crates/roko-agent-server/src/registration.rs | CH01, CH03 | quick |
| CH05 | Aggregator merges chain ∪ local | crates/roko-serve/src/{routes/aggregator,state}.rs | CH02, CH03, AG03 | quick |
| CH06 | Debug /api/agents/discover-chain | crates/roko-serve/src/routes/aggregator.rs | CH05 | quick |
| CH07 | Demo bootstrap of 5 passports | crates/roko-demo/src/scenarios/ | CH01, CH04 | quick |
| CH09 | docs/v2/chain-discovery.md + close plans | docs/v2/, CLAUDE.md, plans 03 + 06 | CH04, CH05, CH07 | quick |

(CH08 = network-only dashboard mode is a manual track — see `manual-tracks/03-dashboard-url-migration/CHECKLIST.md`.)

## Wave TU — TUI event parity (plan 05)

| Batch | Followup | Title | Scope | Deps | Verify |
|-------|----------|-------|-------|------|--------|
| TU01 | 76 | Learning trio incremental | tui/dashboard.rs | — | quick |
| TU02 | 73 | Episode log incremental | tui/dashboard.rs | — | quick |
| TU03 | 71 | Gate signals incremental | tui/dashboard.rs | — | quick |
| TU04 | 72 | Task outputs watched per-file | tui/{dashboard,task_outputs}.rs | — | quick |
| TU05 | 74 | Event log incremental | tui/{dashboard,json_array_tailer,mod}.rs | — | quick |
| TU06 | 78 | Generation counter durable | tui/{dashboard_gen_persist,dashboard,mod}.rs | — | quick |
| TU07 | 70 | Agent panel WS multiplex | tui/{agent_streams,views/agents_view,state,mod}.rs | TU01 | quick |

## Wave MC — MCP coverage audit (plan 06)

| Batch | Title | Scope | Deps | Verify |
|-------|-------|-------|------|--------|
| MC01 | Audit + matrix | docs/v2/MCP-AUDIT.md | — | quick |
| MC02 | Bucket decisions | docs/v2/MCP-AUDIT.md | MC01 | quick |
| MC03 | Integration tests for shipped-default | crates/roko-mcp-*/tests/integration.rs | MC02 | full |
| MC04 | roko.toml.example + MCP-INTEGRATION.md | roko.toml.example, docs/v2/ | MC02 | quick |
| MC05 | CLAUDE.md rows | CLAUDE.md | MC04 | quick |
| MC06 | Close followup item 34 | tmp/ux/ux-followup/05-*.md | MC03, MC05 | quick |

## Wave FG — Phase-2 feature gating (plan 07)

| Batch | Title | Scope | Deps | Verify |
|-------|-------|-------|------|--------|
| FG01 | default-members in root Cargo.toml | Cargo.toml | — | full |
| FG02 | Weekly phase2-build CI | .github/workflows/ | FG01 | quick |
| FG03 | roko-plugin audit + decision | crates/roko-plugin/, plan tracker | — | quick |
| FG04 | CLAUDE.md + docs/v2/WORKSPACE.md | CLAUDE.md, docs/v2/ | FG01 | quick |

## Wave BP — Agent backend parity (plan 08)

| Batch | Followup | Title | Scope | Deps | Verify |
|-------|----------|-------|-------|------|--------|
| BP01 | 38 | parity_kit.rs helper | crates/roko-agent/tests/_helpers/ | — | quick |
| BP02 | 38 | Claude tests via kit (golden) | crates/roko-agent/tests/claude_parity.rs | BP01 | quick |
| BP03 | 36 | Codex 10-turn conformance | crates/roko-agent/tests/codex_conformance.rs + fixtures | BP01 | quick |
| BP04 | 37 | Cursor streaming parity | crates/roko-agent/{src/cursor_agent.rs, tests/cursor_parity.rs} | BP01 | quick |
| BP05 | 38 | Gemini parity | crates/roko-agent/tests/gemini_parity.rs | BP01 | quick |
| BP06 | 38 | Perplexity parity | crates/roko-agent/tests/perplexity_parity.rs | BP01 | quick |
| BP07 | 38 | Ollama parity | crates/roko-agent/tests/ollama_parity.rs | BP01 | quick |
| BP08 | 39 | ExecAgent / ClaudeCli consolidation | crates/roko-agent/src/{cli_agent,exec,claude_cli_agent,lib}.rs | BP02 | full |
| BP09 | 40 | File-layout cleanup (Gemini, Ollama, Perplexity) | crates/roko-agent/src/{gemini,ollama,perplexity,lib}.rs | BP05, BP06, BP07 | full |
| BP10 | 40a + 60c | Cascade router integration test | crates/roko-learn/tests/cascade_router_integration.rs | BP01 | full |
| BP11 | — | docs/v2/AGENT-BACKENDS.md + CLAUDE.md | docs/v2/, CLAUDE.md, followup files | BP02-BP07, BP10 | quick |

## Wave HY — Hygiene + test coverage (plan 10)

| Batch | Followup | Title | Scope | Deps | Verify |
|-------|----------|-------|-------|------|--------|
| HY01 | 19 | SystemPromptBuilder snapshots | crates/roko-compose/tests/ | — | quick |
| HY02 | 19 | 6-layer presence assertion | (same file) | HY01 | quick |
| HY03 | 56 | # Errors/# Panics in roko-core, roko-runtime | crates/roko-{core,runtime}/src/lib.rs | — | quick |
| HY04 | 56 | Same for roko-gate, roko-compose | crates/roko-{gate,compose}/src/lib.rs | — | quick |
| HY05 | 58 | Flaky timeouts in roko-agent/exec.rs | crates/roko-agent/src/exec.rs | — | quick |
| HY06 | 87 | Per-gate timeline render | tui/{verdicts,views/dashboard_view}.rs | — | quick |
| HY07 | 45 carry-over | Smoke: learning loop | crates/roko-cli/tests/smoke_learning_loop.rs | — | full |
| HY08 | 45 carry-over | Smoke: TUI events | crates/roko-cli/tests/smoke_tui.rs | — | full |

## Manual tracks (no Codex batches)

| Wave | Plan | Where to run | Steps |
|------|------|--------------|-------|
| DB | 03 — Dashboard URL migration | sibling repo `nunchi-dashboard/` | `manual-tracks/03-dashboard-url-migration/CHECKLIST.md` |
| DC | 09 — Stale docs and drift sweep | locally with sed + reviewer eyes | `manual-tracks/09-stale-docs-and-drift/CHECKLIST.md` |
| RH | 11 — Runner hardening | `tmp/tui-parity/` (bash) | `manual-tracks/11-runner-hardening/CHECKLIST.md` |
| PH | 12 — Phase-2 vision | parked | `manual-tracks/12-phase2-vision/CHECKLIST.md` |

## Dependency graph (text form)

```
Wave M:      M01 → M02 → M03 → M04 → {M05, M06} → M07
                          (M04 = full verify)

Wave AG:     {AG01, AG02, AG03} → AG04 → AG05 ┐
             AG02 → AG08                       ├→ AG09 → AG10
             AG06 → AG07 ────────────────────┐ │
                                              └─┘
             (AG09 = full verify)

Wave CH:     CH01 → CH02 ┐
             CH03 ────────┼→ CH04
             AG03 ────────┤
                          ├→ CH05 → CH06
             CH01,CH04 → CH07
             {CH04,CH05,CH07} → CH09

Wave TU:     all independent except TU07 → TU01

Wave MC:     MC01 → MC02 → {MC03, MC04} → MC05 → MC06
             (MC03 = full)

Wave FG:     FG01 → {FG02, FG04};  FG03 independent
             (FG01 = full)

Wave BP:     BP01 → {BP02, BP03, BP04, BP05, BP06, BP07, BP10}
             BP02 → BP08
             {BP05, BP06, BP07} → BP09
             {BP02..BP07, BP10} → BP11
             (BP08, BP09, BP10 = full)

Wave HY:     all independent
             (HY07, HY08 = full)
```

## Suggested wave order for sequential runs

When running `--group X` one at a time, this order minimises wave-gate
breakage and unblocks downstream waves earliest:

```
RH (manual)  →  DC (manual)  →  AG  →  HY  →  TU  →  MC  →  FG  →  BP  →  CH  →  M  →  DB (manual)
```

Rationale: AG unblocks DB which unblocks M. CH and M can run in parallel
once AG is green. The hygiene + parity work has no dependencies on the
dashboard story.

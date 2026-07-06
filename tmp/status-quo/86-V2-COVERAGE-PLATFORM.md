# V2 Platform Specs (08-19) — Implementation Coverage

> Status-quo audit · **re-verified against code 2026-07-08 @ HEAD 5852c93c0** (prior pass 2026-07-07) · sources: 12 specs (`docs/v2/08`–`19`, 13,424 lines), 31 crates + 3 apps (agent-relay, mirage-rs, roko-chain-watcher), 288 raw serve routes, `.roko/GAPS.md`, cross-checked against sibling audits 18/33/48/52/55/57/70/75.
>
> **2026-07-08 re-verify deltas** (all confirmed by grep at HEAD 5852c93c0): (1) **17-AUTH workspace roles upgraded 🟡→✅** — `team.rs` now has a durable `.roko/team/members.json` store (`team.rs:274,296`) AND route-level `require_role(&["owner","admin"])` enforcement (`team.rs:171,216,245,351`, tested `:425`), resolving prior open-question #5. (2) All other load-bearing claims re-confirmed unchanged: 288 serve routes, 31 crates, Group/Lens/CamelTag/MPP = 0 grep hits, `Observe`/`Connect`/`Trigger` traits at `traits.rs:400/408/420` with **test-only impls** (`phase1_integration.rs:188/237/294`), extension.rs 760 lines, x402.rs 958 lines, feed.rs 301 lines, gateway routes at `gateway.rs:42-44`. No platform-spec concept changed status downward. See also ecosystem-doc deltas (cold-archival timer, on-chain AgentRegistry serve path).

Status legend: ✅ wired · 🔌 built-not-wired · 🟡 partial · ❌ missing · 🕰️ v1-shape equivalent (capability exists under pre-v2 name/architecture)

## Summary

The platform specs split cleanly into three bands. **Real and wired** (12-EXTENSIONS, 17-AUTH, 19-CONFIG, and the registry halves of 09-FEEDS/11-CONNECTIVITY): the Extension trait with 8 layers + 16/22 hooks is consumed by orchestrate/runner/serve/do; Privy JWKS + API-key scopes + team routes guard roko-serve; the 4-layer config merge matches the spec's section reference almost table-for-table. **V1-shape equivalents** (08-GATEWAY, 13-TRIGGERS, 14-TOOLS, 15-TELEMETRY, 16-SECURITY): the *capability* exists — gateway routes + ThinkingCap/Convergence cells, cron/file-watch EventSources, gates/routers/composers, StateHub + /metrics, safety hook chain + Taint provenance + immune types — but the v2 *architecture* (Cell pipeline graphs, TriggerBinding, Lens system, Capability<T> 3-layer stack) does not exist; the v2 protocol traits `Observe`/`Connect`/`Trigger` are defined (`roko-core/src/traits.rs:400,408,420`) with **zero production impls** (test-only, `roko-core/tests/phase1_integration.rs`). **Missing** (10-GROUPS entirely; the MPP/dispute half of 18-PAYMENTS; recipes half of 09): no Group/RelayRoom/GroupContextBidder type anywhere; MPP has 0 hits; x402 is a genuine 958-line manager (`roko-chain/src/x402.rs`) but runs in-process against mocks with no HTTP 402 middleware. Two sibling-audit corrections landed here: taint **does** exist (`Taint` enum + `Engram.provenance`, contra a prior sub-audit), and team/workspace-role routes **do** exist in roko-serve.

## Coverage matrix

| Spec | Concept | Code location | Status | Evidence |
|---|---|---|---|---|
| 08-GATEWAY | Gateway HTTP surface | `crates/roko-serve/src/routes/gateway.rs:42-46` | ✅ | `/inference/complete`, `/gateway/stats`, `/gateway/models`, `/inference/batch/*`; mounted at `routes/mod.rs:171` |
| 08-GATEWAY | ThinkingCapCell | `crates/roko-agent/src/model_call_service.rs:104,134,1350-1390` | ✅ | Per-model thinking budgets, default 16,384; `with_thinking_budget()` :254 |
| 08-GATEWAY | ConvergenceDetectionCell | `crates/roko-agent/src/model_call_service.rs:106,135,1410` | ✅ | `new(5, 0.85, 3)` window/similarity/consecutive |
| 08-GATEWAY | LoopDetectCell | `crates/roko-conductor/src/stuck_detection.rs`, `roko-learn/src/anomaly.rs:243` | 🕰️ | Loop detection lives in conductor ErrorPattern::LoopDetected + anomaly 5-identical-hash rule, not a gateway Verify cell |
| 08-GATEWAY | CacheLookupCell L1 hash / L2 semantic | — | ❌ | `grep CacheLookup\|hash_cache\|semantic_cache` roko-agent = 0 |
| 08-GATEWAY | ToolPruneCell / OutputBudgetCell / CacheStoreCell | — | ❌ | 0 hits in crates/ |
| 08-GATEWAY | Pipeline as TOML graph of 9 cells | — | ❌ | Logic hardcoded in ModelCallService, no `[graph]` definition |
| 09-FEEDS | Feed core types (FeedInfo/FeedKind/FeedAccess/FeedRuntimeStatus) | `crates/roko-core/src/feed.rs:81` (301 lines), exported `lib.rs:212` | ✅ | Registry + list_by_kind/list_by_agent/search |
| 09-FEEDS | FeedRegistry runtime instantiation | `roko-cli/src/commands/plan.rs:451`, `do_cmd.rs:536`, `serve_runtime.rs:567` | ✅ | Arc<Mutex<FeedRegistry>> constructed in live paths; `roko-cli/tests/phase0_wiring.rs:295` |
| 09-FEEDS | Feed HTTP API | `crates/roko-serve/src/routes/feeds.rs:29-36` | ✅ | `/feeds` CRUD + `/feeds/catalog` + `/feeds/runtime/{id}` |
| 09-FEEDS | FeedPublisherExt (bus publishing, no hidden channels) | — | ❌ | No Pulse-on-Bus feed data flow; registry is metadata-only |
| 09-FEEDS | Derived/composite/meta feed composition | — | ❌ | No feed-consumes-feed graph |
| 09-FEEDS | Payment-gated feeds (x402/MPP), ERC-8004 advertisement | — | ❌ | Feed structs carry no payment fields |
| 09-FEEDS | Recipes / RecipeCell (spec §12) | — | ❌ | Zero code |
| 10-GROUPS | Group / GroupIdentity / CoordinationMode / GroupContextBidder / RelayRoom | — | ❌ | `grep 'pub struct Group\b\|GroupIdentity\|GroupContextBidder\|CoordinationMode'` crates/+apps/ = 0; no `room` in `apps/agent-relay/src/` |
| 10-GROUPS | Membership protocol, group knowledge store, group pheromones, dashboard | — | ❌ | None of the ~15 spec routes exist |
| 10-GROUPS | Adjacent 🕰️ substrate | `apps/agent-relay/src/bus.rs:29-34`; `roko-orchestrator/src/coordination.rs`; `roko-serve/src/routes/team.rs:79-84` | 🕰️ | Relay topics (not rooms), mesh stigmergy (not group-scoped), human team roles (not agent groups) |
| 11-CONNECTIVITY | Connect protocol trait | `crates/roko-core/src/traits.rs:408` | 🔌 | `Connect: Cell` defined; impls only in `roko-core/tests/phase1_integration.rs:237` |
| 11-CONNECTIVITY | ConnectorKind/ConnectorConfig/ConnectorHealth + registry | `crates/roko-core/src/connector.rs`; `roko-serve/src/routes/connectors.rs:24-29` | ✅ | 6/7 kinds; `/connectors` CRUD + `/connectors/{name}/health`; wired in phase0 tests + serve_runtime |
| 11-CONNECTIVITY | ConnectorManifest, ReconnectStrategy, finality oracle | — | ❌ | 0 hits |
| 11-CONNECTIVITY | MCP exoskeleton | `roko-agent` MCP client + `roko-mcp-{code,github,slack,scripts,stdio}` | 🟡 | Real wiring with P0 config-shape bugs (see 48-MCP-CRATES.md); no Signal/Pulse-over-MCP framing, no auto-registration as Connectors |
| 11-CONNECTIVITY | A2A agent cards, ERC-8004 ZK-HDC, x402 intents as exoskeleton | `roko-chain/src/{agent_registry,x402}.rs` | 🔌 | Chain-side types exist but are not joined to the Connector layer |
| 11-CONNECTIVITY | Workspace discovery via relay | `apps/agent-relay/src/protocol.rs:13,90,134`; `roko-serve/src/routes/relay_proxy.rs:26` | 🟡 | Relay + Hello frames + serve proxy real; discovery/announce semantics per spec unverified |
| 12-EXTENSIONS | Extension trait, 8 layers, ExtensionChain + short-circuit | `crates/roko-core/src/extension.rs:168-596` (760 lines) | ✅ | ExtensionLayer Foundation(0)→Recovery(7); chain sorts by layer |
| 12-EXTENSIONS | Runtime consumption | `roko-cli/src/{orchestrate.rs,runner/extension_loader.rs,serve_runtime.rs,commands/{do_cmd,plan}.rs}` | ✅ | ExtensionChain loaded on live paths |
| 12-EXTENSIONS | 22 hooks | `extension.rs` | 🟡 | 16/22 present; missing on_budget_exceeded + on_tick_start/end + on_slot_assigned/completed |
| 12-EXTENSIONS | Decision enums | `extension.rs` | 🟡 | ActionDecision/ToolDecision/RecoveryAction/Adjustment ✅; FilterDecision, BudgetAction ❌ |
| 12-EXTENSIONS | CaMeL IFC on extensions (CamelTag, propagation, no-laundering) | — | ❌ | `grep -ril CamelTag crates/` = 0 (CaMeL dual-LLM exists elsewhere: `roko-agent/src/safety/data_llm.rs`) |
| 12-EXTENSIONS | Manifest + packaging tiers, hook timeout, circuit breaker | `roko-plugin/src/manifest.rs` (partial) | 🟡 | TOML manifest tier real; native/WASM tiers, 5s timeout, 5-failure breaker ❌ |
| 13-TRIGGERS | Trigger protocol trait | `crates/roko-core/src/traits.rs:420` | 🔌 | `Trigger: Cell` (arm/disarm); prod impls 0; test `phase1_integration.rs:294` |
| 13-TRIGGERS | Cron | `roko-plugin/src/lib.rs:148,262`; wired `roko-cli/src/event_sources.rs:9,41` | 🕰️ | `CronEventSource::from_config(config.scheduler)` — EventSource shape, not TriggerBinding |
| 13-TRIGGERS | FileWatch | `roko-plugin/src/lib.rs:82,335` | 🕰️ | FileWatchEventSource (notify); also TUI fs_watch + config hot-reload watchers |
| 13-TRIGGERS | Webhook | `roko-plugin/src/lib.rs:71`; `roko-serve/src/routes/event_ingest.rs` | 🟡 | EventSourceKind::Webhook variant + manifest kind exist; **no WebhookEventSource impl struct**; HTTP ingest route is the de-facto path |
| 13-TRIGGERS | ChainEvent | `apps/roko-chain-watcher/src/{watcher,block_observer,reactions,rpc_client}.rs` | 🕰️ | Standing watcher app, not a TriggerBinding kind |
| 13-TRIGGERS | Manual | `roko run` / `roko inject` (roko-cli) | 🕰️ | Equivalent, unnamed |
| 13-TRIGGERS | Bus, SignalPattern (+HDC similarity) | — | ❌ | 0 hits |
| 13-TRIGGERS | TriggerBinding + `.roko/triggers/` persistence, ConcurrencyPolicy, chaining, `roko trigger` CLI | `roko-plugin/src/manifest.rs:158-192` (schema only) | ❌ | TriggerDef TOML parses 3 kinds but nothing drives it at runtime; no engine/persistence/policy |
| 14-TOOLS | Catalog capabilities (stores/scorers/gates/routers/composers) | `roko-gate/src/*` (11 gates), `roko-learn/src/cascade_router.rs`, `roko-compose/src/{prompt,auction}.rs`, `roko-fs`, `roko-std` (builtin tools) | 🕰️ | Substance exists per-crate in pre-Cell shape |
| 14-TOOLS | Catalog as Cell impls | `roko-graph/src/cells/` (graduation.rs, task_executor.rs + stubs) | 🟡 | ~5 of ~46 spec cells exist as Cells; 7 cognitive-loop cells are PassthroughCell (`.roko/GAPS.md:16`) |
| 14-TOOLS | TOML builtin-cell registration | — | ❌ | No name→Cell registry keyed by spec names (`file-store`, `llm-scorer`, …) |
| 15-TELEMETRY | Observe protocol | `crates/roko-core/src/traits.rs:400` | 🔌 | `Observe: Cell` defined; prod impls 0 (test `phase1_integration.rs:188`) |
| 15-TELEMETRY | Lens type / LensScope / 11 named lenses | — | ❌ | `grep 'trait Lens\|struct .*Lens'` crates/ = 0 |
| 15-TELEMETRY | Lens metrics as non-Lens code | `roko-learn/src/{efficiency,drift-adjacent modules}`, `roko-cli/src/orchestrate.rs` (CFactorSummary), budget tracking | 🕰️ | ~7/11 lens metrics computed (cost, latency, quality, efficiency, error, budget, c-factor) |
| 15-TELEMETRY | C-factor + sub-metrics | `roko-cli/src/orchestrate.rs` CFactorSummary | 🟡 | Composite computed; the 5 sub-lens decomposition + learned weights not typed as lenses |
| 15-TELEMETRY | StateHub + projections + /metrics | `roko-runtime/src/state_hub.rs`; `roko-serve/src/routes/{metrics.rs:1-35,projections.rs}` | ✅ | Prometheus endpoint + projection contract live |
| 16-SECURITY | Safety enforcement core | `roko-agent/src/safety/{mod,hooks,capabilities,contract,authz,allowlist,bash,git,hallucination}.rs` + `contracts/` | ✅ | Role auth + pre/post hook chain enforced in ToolDispatcher (see 33-AGENT-SAFETY) |
| 16-SECURITY | Taint lattice IFC | `roko-core/src/provenance.rs:24` (`pub enum Taint`), `engram.rs:81,121` | 🟡 | Taint enum + `Engram.provenance` + `is_tainted()` in content hash; **monotonic lattice-join propagation rules not implemented** |
| 16-SECURITY | CaMeL dual-LLM | `roko-agent/src/safety/data_llm.rs` | 🔌 | Built (quarantined data-LLM); CamelTag/provenance-tag IFC of spec §4.3-4.6 absent |
| 16-SECURITY | Immune system | `roko-core/src/immune.rs` (573 lines); consumers: `roko-dreams/src/phase2/advanced.rs`, `roko-agent/src/lifecycle.rs`, `roko-chain/src/{isfr,identity_economy_identity}.rs` | 🔌 | Incident/quarantine/IncidentLink types consumed, but no 5-layer immune pipeline **graph** |
| 16-SECURITY | Capability<T> 3-layer stack (cell decl / graph allow-list / space grant) + resolution | `safety/capabilities.rs` (flat tiers only) | ❌ | No 3-layer intersection algorithm |
| 16-SECURITY | 5-head lexicographic corrigibility; declassification-needs-human | — | ❌ | No ordered Verify-cell head pipeline |
| 17-AUTH | VerifyJwt (Privy) + JWKS caching | `roko-serve/src/jwks.rs:1-249` | ✅ | ES256, 1h TTL, stale-while-revalidate, no PRIVY_APP_SECRET (public JWKS only :17,147) |
| 17-AUTH | API keys: 4 scopes, scope→route map, 403 detail | `roko-serve/src/routes/middleware.rs:166-196,356-386,420-432` | ✅ | SHA-256 hash check; `required_scope_for()`; insufficient_scope body |
| 17-AUTH | Agent bearer tokens | `middleware.rs:220-247` | 🟡 | Hash+expiry validated; spec §5.4 rotation grace window absent |
| 17-AUTH | `roko login` (API key + browser callback) | `roko-cli/src/commands/auth.rs:6-150`; `roko-cli/src/credentials.rs:29-147` | ✅/🟡 | Works; stores `~/.roko/credentials.json` 0600 — **not OS keychain** |
| 17-AUTH | Device flow | — | ❌ | No `/auth/device/*` endpoints |
| 17-AUTH | Workspace roles / team sharing | `roko-serve/src/routes/team.rs:81-85,171,274,351` | ✅ | `/team/me`, `/team/members`, `/team/invite`, `PUT/DELETE /team/members/{did}` with owner/admin/member/viewer; **durable `.roko/team/members.json` store** (`team.rs:274,296`) + **`require_role(&["owner","admin"])` enforced** on mutating routes (`:171,216,245`), last-owner guard (`:224,253`), unit-tested (`:425`) — **upgraded 🟡→✅ 2026-07-08** (prior "durable store + role enforcement unverified" now resolved) |
| 18-PAYMENTS | x402 manager | `roko-chain/src/x402.rs` (958 lines): `create_payment_request:241`, `verify_authorization:269`, nonce `:302`, channels `:315-465`, phase2 bridge `:471` | 🔌 | Real logic incl. ERC-3009-style auth verify — but in-process/mock; no HTTP 402 middleware, no serve routes, no settlement tx |
| 18-PAYMENTS | MPP streaming sessions | — | ❌ | `grep MppSession\|Micropayment` crates/+apps/ = 0 |
| 18-PAYMENTS | Reputation-based pricing (5 tiers) | `roko-chain/src/identity_economy_markets.rs` (ReputationTier + markup_bps) | 🔌 | Never invoked from a payment path |
| 18-PAYMENTS | Disputes (flow, 72h auto-resolve, credits) | `roko-chain/src/phase2.rs` (Dispute/DisputeReason/DisputeVerdict types) | 🔌/❌ | Types only; no submission/resolution/credit lifecycle |
| 18-PAYMENTS | Relay payment flow / per-message draw | — | ❌ | agent-relay has no payment hooks |
| 19-CONFIG | 4-layer merge + priority + env convention | `roko-core/src/config/loader.rs:33-128` | ✅ | CLI > env (`ROKO__SECTION__FIELD`) > TOML > defaults; `${VAR}` interpolation |
| 19-CONFIG | Section reference parity | `roko-core/src/config/schema.rs` | ✅ | [project][server][serve][serve.auth][agent][[agents]][providers][models][routing][budget][conductor] all present as structs |
| 19-CONFIG | Verify cell / invariants | `schema.rs` validation + `roko config validate` | 🟡 | Validation real; exact 7-invariant matrix not encoded 1:1 |
| 19-CONFIG | Hot reload | `roko-core/src/config/hot_reload.rs:36-99`; `roko-serve/src/routes/config.rs:70-92` | 🟡 | ConfigSection.is_hot_reloadable + manual `PUT /api/config`; **no file-watch ConfigWatchTrigger, no `config.reloaded` Bus topic** |
| 19-CONFIG | Config-as-Signal, demurrage, L4 proposals, migration chain | — | ❌ | No Kind::Config Signal wrapping; `LoadConfigError::NoMigrationPath` exists with zero registered migrations |

## Per-spec deep notes

- **08-GATEWAY** — 2 of the spec's cells (ThinkingCap, Convergence) were implemented *inside* `ModelCallService` with spec names, and the gateway routes exist; but the spec's core value (loop-detect → cache L1/L2 → tool-prune → output-budget as a composable Verify/Route/Compose graph) is absent. `InferenceRequest/Response` in code carry plan_id/task/role rather than the spec's messages/tools/thinking shape. Loop detection exists at the *conductor* level (stuck_detection), a different architectural station than pre-inference.
- **09-FEEDS** — `feed.rs`'s own header says it implements the registry from `docs/v2/11-CONNECTIVITY.md` §7 — i.e., what shipped is the *connectivity* view of feeds (registry + HTTP CRUD), not the 09 machinery (publisher extension, bus pulses, kernel decomposition, recipes). `isfr_feed.rs:9` explicitly disclaims being a FeedRegistry feed. No feed data has ever flowed as a Pulse.
- **10-GROUPS** — the only spec with **zero dedicated code**. Every primitive (Group, RelayRoom, invitation flow, group store/pheromone partitions, GroupContextBidder) greps to nothing. Building blocks exist separately: relay topic rings, mesh stigmergy in `roko-orchestrator/src/coordination.rs`, c-factor in orchestrate.rs, human team roles in serve. Group-scoped ERC-8004 identity also absent (agent_registry.rs is per-agent).
- **11-CONNECTIVITY** — split personality: ConnectorRegistry/Kind/Health + serve routes are live, and the FeedRegistry (§7) landed; but the trait story (`Connect` prod impls), ConnectorManifest, and all four exoskeleton protocol *integrations* are missing. MCP is the strongest real connector but is wired through `agent.mcp_config`, not through the Connector layer — two parallel systems.
- **12-EXTENSIONS** — the healthiest platform spec. One deliberate scope cut visible: layer count matches (8), hook count doesn't (16/22 — budget + tick/slot lifecycle hooks dropped), and the CaMeL-tag IFC integration was skipped entirely even though the dual-LLM half of CaMeL exists in roko-agent.
- **13-TRIGGERS** — classic v1/v2 seam: cron/file-watch/webhook shipped as `EventSource` (pull-config, push-Engram) with real CLI wiring, while the v2 `Trigger` protocol, TriggerBinding persistence, concurrency policies, and the two novel kinds (Bus, SignalPattern/HDC) have no production code. `roko-plugin/src/manifest.rs` TriggerDef parses trigger TOML that nothing executes.
- **14-TOOLS** — inventory is v1-shaped: gates/routers/composers/stores/tools all exist and are wired via orchestrate.rs, but almost none satisfy the `Cell` trait or register under the spec's kebab-case catalog names. Per GAPS.md the cognitive-loop cells that *should* be the catalog's flagship instances are PassthroughCell stubs.
- **15-TELEMETRY** — the metrics exist; the *architecture* doesn't. `Observe` is defined-but-unimplemented, `Lens`/`LensScope` are entirely absent (0 grep hits, confirmed twice across audits), yet StateHub, projection contract, Prometheus /metrics, efficiency events, and CFactorSummary provide most lens outputs through tracing/axum plumbing.
- **16-SECURITY** — enforcement core is real and stricter than earlier audits claimed: taint exists (`Taint` enum, `Engram.provenance`, tainted-bit hashed into content hash) and immune types are consumed by 4 crates outside core. What's missing is the *composition*: no lattice-join propagation, no 3-layer capability resolution, no lexicographic 5-head pipeline, no immune pipeline graph, no CamelTag laundering guarantees.
- **17-AUTH** — now **4 of 5 spec surfaces genuinely wired** (JWT/JWKS, API keys+scopes, agent tokens minus grace, **workspace roles with durable store + enforcement**). 2026-07-08 correction: team roles DO gate routes — `require_role` runs before member add/update/remove and refuses to demote/remove the last owner; members persist to `.roko/team/members.json`. Remaining gaps: device flow, OS keychain (still `~/.roko/credentials.json` 0600), agent-token rotation grace window.
- **18-PAYMENTS** — x402 is much more than stubs (channel lifecycle + authorization verification + nonce replay protection) but is an unreachable island: nothing in roko-serve or agent-relay returns 402 or calls X402Manager. MPP, disputes-as-flow, relay draws: no code.
- **19-CONFIG** — strongest match of any spec's "reference" section to code; the aspirational §1 (config-as-Signal) and §5 (watch trigger + `config.reloaded` Bus topic) are the unbuilt 20%.

## Designed-but-unbuilt list

Zero (or near-zero) code, explicit:

1. **All of 10-GROUPS** — Group, RelayRoom, CoordinationMode, membership/invitation protocol, group store/pheromone partitions, GroupContextBidder, group dashboard routes.
2. **Gateway cache + shaping cells** (08) — CacheLookupCell L1/L2, CacheStoreCell, ToolPruneCell, OutputBudgetCell, LoopDetectCell-as-gateway-cell, TOML gateway graph.
3. **FeedPublisherExt + feed-data-as-Pulse + recipes/RecipeCell** (09).
4. **ConnectorManifest, ReconnectStrategy, finality oracle, A2A agent cards** (11).
5. **CamelTag IFC** (12/16) — tag struct, propagation rules, no-laundering, declassification approval.
6. **FilterDecision, BudgetAction, 6 missing hooks, packaging tiers 2-4, hook timeout/circuit-breaker** (12).
7. **Trigger runtime** (13) — TriggerBinding + `.roko/triggers/`, ConcurrencyPolicy, Bus + SignalPattern kinds, chaining, `roko trigger` CLI/API.
8. **Builtin-cell registry** (14) — spec-named Cell catalog + TOML registration.
9. **Lens system** (15) — Lens/LensScope + 11 lens types + composition; prod `Observe` impls.
10. **3-layer capability stack, lattice-join, 5-head corrigibility, immune pipeline graph** (16).
11. **Device flow, OS keychain storage, agent-token grace rotation** (17).
12. **MPP sessions, dispute flow, relay payment draws, settlement execution, reputation-priced payments** (18).
13. **Config-as-Signal, ConfigWatchTrigger, `config.reloaded` Bus topic, config demurrage, L4 proposals, registered migrations** (19).

## Migration checklist

- [ ] **[P0]** Decide 10-GROUPS: implement a minimal Group = relay-topic + membership list, or mark the spec deferred — verify: `grep -rn 'pub struct Group' crates/ apps/` non-empty or spec header carries a status tag
- [ ] **[P0]** Fix the MCP config-shape P0s so the strongest connector path is trustworthy (per 48-MCP-CRATES.md) — verify: `cargo run -p roko-cli -- doctor` reports MCP config parse OK against a spec-shaped `roko.toml`
- [ ] **[P0]** Make x402 reachable or explicitly shelve it: add a 402 middleware/route in roko-serve that calls `X402Manager::verify_authorization` — verify: `grep -rn 'X402Manager' crates/roko-serve/ apps/agent-relay/` non-empty
- [ ] **[P1]** Ship production `Trigger` impls by adapting `CronEventSource`/`FileWatchEventSource` to the protocol + add TriggerBinding persistence — verify: `grep -rln 'impl Trigger for' crates/*/src/` non-empty and `.roko/triggers/` created on arm
- [ ] **[P1]** Add a `WebhookEventSource` impl (EventSourceKind::Webhook currently has no struct); route serve `event_ingest` through it — verify: `grep -n 'struct WebhookEventSource' crates/roko-plugin/src/lib.rs`
- [ ] **[P1]** Implement Lens minimally (trait + LensScope + wrap CFactorSummary/efficiency as first two lenses feeding StateHub) — verify: `grep -rn 'trait Lens' crates/roko-core/`
- [ ] **[P1]** Close the 12-EXTENSIONS deltas: FilterDecision, BudgetAction, the 6 missing hooks, 5s hook timeout — verify: hook count in `extension.rs` == 22
- [ ] **[P1]** Wire config hot-reload end-to-end: file watcher → recompose → validate → publish `config.reloaded` on the bus — verify: `grep -rn 'config.reloaded' crates/`
- [ ] **[P2]** Implement taint lattice-join propagation on Signal derivation (Taint exists; propagation doesn't) — verify: property test that derived Engram taint >= max(parents)
- [ ] **[P2]** Feed data path: FeedPublisherExt publishing Pulses on the bus, consumed via relay topics — verify: integration test where agent A's feed pulse reaches agent B
- [ ] **[P2]** Gateway shaping cells (cache L1 hash first — biggest cost win), registered in a TOML gateway graph — verify: `/gateway/stats` reports cache hit-rate > 0 on repeated identical prompt
- [ ] **[P2]** Auth completeness: device flow endpoints + keyring crate for credentials + agent-token grace window — verify: `roko login --device` on headless host
- [ ] **[P3]** Builtin Cell catalog: register existing gates/routers/composers under spec kebab-case names behind the Cell trait (depends on GAPS.md PassthroughCell work) — verify: `grep -rn 'PassthroughCell' crates/roko-graph/src/cells/` = 0
- [ ] **[P3]** 5-head corrigibility as ordered Verify pipeline over existing safety hooks; immune pipeline graph over `roko-core/src/immune.rs` types — verify: pipeline TOML exists and is loaded by a runtime path

## Open questions

1. **Two connector systems**: MCP wires through `agent.mcp_config` while ConnectorRegistry lives beside it — is the Connector layer meant to absorb MCP config (spec 11 §5 says auto-register), or stay a parallel inventory?
2. **Who owns triggers** — `roko-plugin` EventSource (shipped), manifest TriggerDef (parsed, unexecuted), or the `Trigger` protocol (trait-only)? Three partial systems, no declared winner.
3. **Is 10-GROUPS funded?** It has a full 933-line spec, an INDEX-only v2-depth section, and zero code — the largest spec/code delta in the platform band.
4. **Payments deployment path**: X402Manager and ReputationTier are real but mock-backed — is the intent mirage-rs → testnet, or should 18-PAYMENTS be feature-gated out until a chain exists?
5. ~~**team.rs vs spec §7/§9**~~ — **RESOLVED 2026-07-08**: durable store IS `.roko/team/members.json` (`team.rs:274`), and mutating routes DO enforce role via `require_role` (`:171,216,245,351`) beyond API-key scope. Remaining sub-question: does the store reconcile with Privy DIDs / JWT `sub`, or is it a standalone member list? (`get_me` derives caller from JWT; membership is keyed by `did`.)
6. **Spec 08's cells live in roko-agent, not a gateway graph** — was the intent to migrate ModelCallService internals into `roko-graph` cells, or to keep the gateway as a service facade? Affects where cache cells should land.

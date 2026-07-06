# roko-runtime / roko-fs / roko-std — Substrate, Bus, Builtins

> Status-quo audit · verified 2026-07-07 · **re-verified 2026-07-08** (HEAD 5852c93c05) · sources: ~45 source files across roko-runtime (25 modules), roko-fs (14), roko-std (33) + workspace consumers; docs/v1/00-architecture/07-substrate-trait.md, 07b-bus-transport-fabric.md, docs/v2-depth/02-block/store-and-bus-duality.md, docs/v1/18-tools/01-builtin-tools.md, docs/v2/14-TOOLS.md; sibling audits 32/41/44/55; git commits 8f34970, ba0cd40, 322f531
>
> **Re-verification note (2026-07-08):** Every load-bearing claim below re-checked against current code and holds. Confirmed unchanged: `TOOL_COUNT == 37` asserted at `registry.rs:119`; six `STATUS: NOT WIRED` headers in roko-runtime (`delta_consumer`, `theta_consumer`, `energy`, `heartbeat_attention`, `heartbeat_probes`, `task_scheduler`) + one `STATUS: WIRED` (`demurrage_consumer.rs:1`); `ProcessSupervisor` constructed in PlanRunner (`orchestrate.rs:4633,4868,5096` via `CancelToken::new()` at `:4514,4755,4983`); orphan `roko-core/src/{pulse_bus,state_hub}.rs` still absent from any `mod` tree; `signals.jsonl` split-brain at `runner/event_loop.rs:1147-1163`; `archive_batch` (`cold_substrate.rs:218-242`) still appends without a `contains()`/dedupe check (copy-not-move, re-appends hourly); `SubstrateMigrator` (`cold_substrate.rs:305-334`) still has NO `migrate()` method (only `new`/`with_thresholds` + 3 threshold fields); `FileSubstrate` has NO `query_similar` override; malformed-line skip `tracing_line_error` (`file_substrate.rs:210-214`) is a **no-op** whose "roko-fs doesn't depend on tracing" comment is FALSE (`roko-fs/Cargo.toml:27` — `tracing = { workspace = true }`). Line numbers elsewhere may have drifted ±a few lines since the 07-07 pass; anchor by symbol name, not absolute line.

## Summary

These three crates are the healthiest tier of the workspace: small, tested, honest (roko-runtime modules carry literal `//! STATUS: NOT WIRED` headers). The hot substrate (`FileSubstrate` → `.roko/engrams.jsonl`) is the real production store; **scheduled cold archival landed** (CLAUDE.md item 14 is done: hourly serve timer + post-plan hook + CLI), but it is a **copy, not a move** — no call site prunes the hot store, and `archive_batch` never dedupes against the cold index, so every hourly tick re-appends the same aged engrams to `.roko/cold/YYYY-MM.jsonl`. The v2 store/bus duality is **half realized**: Store-side contracts (put/get/query/prune, decay, ColdStore) exist and run; Bus-side types (`Pulse`, `Topic`, `TopicFilter`, `Bus` trait, `GraduationConfig`) all exist but production event traffic still flows through typed `EventBus<E>` enums — there are **three parallel Pulse-bus implementations** and only one narrow, config-gated consumer (ISFR relay bridge). roko-std's tool stack is genuinely load-bearing: `handler_for` is the builtin resolver for every raw LLM provider loop, and `TOOL_COUNT` is **37** (16 std + 17 chain + 4 ISFR), not the "19" CLAUDE.md claims nor the 16 the v1 doc counts (16 = std subset only).

## Current state table

| Component | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| **roko-runtime** | | | | |
| `EventBus<E>` (broadcast + replay ring) | 07b-bus-transport-fabric (precursor) | `roko-runtime/src/event_bus.rs:236-315` | ✅ | global `RokoEvent` bus (`:339`), type-keyed `runtime_event_bus` (`:347`); consumers: `prd.rs:43` (PrdPublished), `roko-serve/src/routes/prds.rs:26` + `prd_publish_subscriber`, `orchestrate.rs:160`, `inference_observer.rs:26-59`, `roko-serve/src/adapters.rs:83`, `roko-acp/src/runner.rs:29` |
| `RokoEvent` heartbeat variants (BEAT-05) | v2 heartbeat/tick loop | `event_bus.rs:151-196` | 🔌 | only emitter is `HeartbeatPolicy` (`heartbeat.rs:816-868`), never constructed outside tests (`heartbeat.rs:2264`); `orchestrate.rs:5649-5653` explicitly drops all 5 heartbeat variants |
| `PulseBus` (impl `roko_core::Bus`) | store-and-bus-duality §9 (`BroadcastBus`) | `roko-runtime/src/pulse_bus.rs:35-81` | 🔌 | zero non-test consumers: only hits are its own tests + `lib.rs:86` re-export |
| `ProcessSupervisor` + `ProcessHandle` | mori `agent/connection.rs` extraction | `process.rs:839-1263` | ✅ | `orchestrate.rs:4633,4868,5096` (PlanRunner), `roko-serve/src/state.rs:934`, sidecar spawn `roko-serve/src/routes/agents.rs:804,869-892`; Drop force-kills live children (`process.rs:1248-1260`) |
| `ProcessSessionLedger` (resume/audit) | — | `process.rs:321`, path helper `:48` | ✅ | `roko-cli/src/status.rs:180,220`, `roko-serve/src/routes/status/dashboard.rs:11`, `agents.rs:881`; tests `roko-runtime/tests/process_supervisor.rs` |
| `CancelToken` (hierarchical) | — | `cancel.rs:37-120` | ✅ | root token per PlanRunner (`orchestrate.rs:159`), serve `state.rs:31`, `routes/plans.rs:19`; propagates via `child()` into supervisor + spawned agents (`process.rs:876-933`) |
| `WorkflowEngine`/`EffectDriver`/`PipelineStateV2` | v2 runner | `workflow_engine.rs`, `effect_driver.rs` | ✅ | `roko-cli/src/run.rs:53-55,756`, `commands/do_cmd.rs:844`, `roko-acp/src/runner.rs:27-31,489`, `roko-orchestrator/src/service_factory.rs:24` |
| `RunLedger` → `.roko/state/run-ledger.jsonl` | 60-STATE-PERSISTENCE | `run_ledger.rs:18-45` | ✅ | runner v2 `runner/event_loop.rs:570-579,1085-1095,5809`; serve `routes/shared_runs.rs:18` |
| `StateHub` / `StateSnapshot` / `RuntimeProjection` | 15-TELEMETRY (StateHub) | `state_hub.rs`, `state_snapshot.rs`, `projection.rs` | ✅ | `roko-serve/src/state.rs:446,839-842`, `runner/event_loop.rs:3410`, `runner/persist.rs:17`, `routes/runs.rs:8` |
| `JsonlLogger` → `.roko/runtime-events.jsonl` | — | `jsonl_logger.rs:32-33` | ✅ | `roko-serve/src/state.rs:450,959`, `roko-acp/src/runner.rs:27`; contract test forbids Debug-format serialization (`lib.rs:154-167`) |
| `HttpEventSink` | — | `http_event_sink.rs` | ✅ | `roko-acp/src/event_forward.rs:4`, `runner/types.rs:1328`, `runner/event_loop.rs:31` |
| `demurrage_consumer` | duality §2.2 (Store demurrage) | `demurrage_consumer.rs:1` | ✅ | header: "WIRED -- called from `roko-serve::start_demurrage_timer()`"; `roko-serve/src/lib.rs:1987,2020-2080` (confidence decay + `apply_demurrage`) |
| `delta_consumer` (3-phase dream loop) | BEAT-02 | `delta_consumer.rs:1,301-340` | 🔌🕰️ | header "NOT WIRED"; all 3 phases return zeroed stubs; duplicates `roko-dreams/src/cycle.rs` (41-DREAMS.md concurs) |
| `theta_consumer`, `energy`, `heartbeat_attention`, `heartbeat_probes`, `task_scheduler` | BEAT/heartbeat family | each file line 1 | 🔌 | self-declared `STATUS: NOT WIRED -- built but no non-test runtime caller` |
| `metrics::MetricRecorder`, `resource::ResourceAccount` | — | `metrics.rs:50`, `resource.rs:12` | 🔌 | no consumers outside crate; roko-orchestrator has its own `executor/resource_budget.rs` |
| `lifecycle` (typestate machine) | LIFE-0x | `lifecycle.rs` | 🟡 | types consumed by `roko-cli/src/knowledge_helpers.rs:13` + `RokoEvent::AgentLifecycleTransition` handled at `orchestrate.rs:5641`; but agent sidecar start/stop uses raw PID+`libc::kill`, not this (44-AGENT-SERVER.md:48,68 — two lifecycle trackers) |
| **roko-fs** | | | | |
| `FileSubstrate` (JSONL hot store) | 07-substrate-trait §3.2; v2/14 `file-store` | `file_substrate.rs:23-334` | ✅ | replay + in-memory index, dedupe, `put_batch` (`:137`), `compact` (`:88`), HDC fingerprint tag on put (`:343-357`, feature `hdc`); consumers: `orchestrate.rs:2318,16434`, `run.rs:1224`, `prd.rs:403`, `commands/util.rs:416,1122`, `tui/verdicts.rs:108`, serve `state.rs:1557` |
| `Store::query_similar` on File/Memory substrates | 07-substrate-trait §2.5, duality §2.3 (native HDC query) | `roko-core/src/traits.rs:52-60` default returns `Ok(vec![])` | ❌ | neither `FileSubstrate` nor `MemorySubstrate` overrides it; only `roko-neuro/src/knowledge_store.rs:651,1716` implements similarity — fingerprints are stored but unqueryable in the substrate |
| `ArchiveColdSubstrate` (ColdStore) | traits.rs:82-151; CLAUDE.md item 14 | `cold_substrate.rs:42-299` | ✅/🟡 | monthly JSONL + `index.json`; **not compressed** despite claims (`cold_substrate.rs:3`, `traits.rs:87`, serve `lib.rs:2085`); `archive_batch` appends without `contains()` check (`:218-242`) |
| Cold archival triggers | item 14 "no cron/trigger" — **stale** | serve timer `roko-serve/src/lib.rs:2096-2160` (spawned at `:344,800`); post-plan `orchestrate.rs:8601-8606→3875-3914`; CLI `commands/knowledge.rs:175-247` | ✅ | config `[cold_storage]` `roko-core/src/config/schema.rs:1560-1620` (enabled by default, hourly, 7d, batch 500); landed in commits 322f531/8f34970 |
| Hot-side prune after archival | ColdStore doc: "removed from the hot substrate by the caller" (`traits.rs:105-106`) | none of the 3 call sites | ❌ | `knowledge.rs:239-243` comment admits: "hot-side cleanup happens via the normal prune path on the next dream cycle" — copy-not-move |
| `SubstrateMigrator` | "encapsulates the migration logic" | `cold_substrate.rs:301-340` | 🔌🕰️ | struct holds 3 thresholds, **has no migrate() method**; the migration loop is instead copy-pasted at the 3 archival sites |
| `GcEngine` + `RetentionPolicy` | GC/retention | `gc.rs:97-145` | 🔌 | only callers are its own tests; `roko knowledge gc` uses `roko-neuro` `KnowledgeStore::gc` (`knowledge_store.rs:1022`); serve has separate `retention.rs` (own `RetentionPolicy` type, name collision) + `start_workspace_gc`/`start_handle_gc` (`lib.rs:345-346,1880,1961`) |
| `RokoLayout` + `LayoutVersion` | 55-DATA-DIR | `layout.rs:31-506` | ✅ | `.roko/VERSION` (V1), `ensure_dirs`; but `engrams_path_legacy()` (`:213`) == `signals_path()` (`:219`) — same file, two docs |
| `FsObservabilitySinks` (traces + tool metrics) | §36.99 | `observability.rs:18-116` | ✅ | `orchestrate.rs:85,4558,4793,5021`, `main.rs:3104`; writes `.roko/traces/`, `.roko/metrics/tool_metrics.jsonl` |
| `atomic` write helpers | — | `atomic.rs:29` | ✅ | `demo_seed.rs:13,1279` + used by sinks |
| `PointerStore`, `ToolAuditLog`, `MetricsLog`, `BanditStore` | §36 pointer/audit; metric.rs | `pointer.rs:25`, `tool_audit.rs:52`, `metrics.rs:33`, `bandit.rs:44` | 🔌 | no non-test consumers found outside roko-fs (PointerStore reachable only via roko-std `expand_pointer`, itself unconsumed) |
| **roko-std** | | | | |
| `ROKO_BUILTIN_TOOLS` / `TOOL_COUNT` | v1/18-tools (says 16), v2/14 (cell catalog) | `tool/builtin/mod.rs:44-72` | ✅ | 37 = 16 std + 17 chain (`roko_chain::tools::CHAIN_DOMAIN_TOOLS`) + 4 ISFR; asserted `registry.rs:119` |
| `StaticToolRegistry` | §36.9 | `tool/registry.rs:21-58` | ✅ | serve `dispatch.rs:46`, cli `run.rs:58`, `orchestrate.rs:165,4136`, `dispatch_helpers.rs:19`, `prompt_helpers.rs:28`, openai_compat.rs:43; schema validation is top-level-type-only (`registry.rs:67-97`) |
| `handler_for` / `HandlerRegistry` (16 executable handlers) | §36.b | `tool/handlers.rs:26-88` | ✅ | builtin resolver for every raw provider loop: `anthropic_api/tool_loop.rs:38,690`, `openai_compat.rs:391`, `cerebras.rs:64`, `gemini/adapter.rs:49,103`, `perplexity/adapter.rs:174`; cli `run.rs:2302,2457`, `orchestrate.rs:16750`, `chain_registry.rs:51` (chain handlers live in roko-cli, fall back to std) |
| `MockToolDispatcher` | test scaffold | `tool/mock_dispatcher.rs:31` | ✅(test-only) | no consumers outside roko-std — intra-crate test utility, as designed |
| `sandbox` path guard | §36.46 | `tool/builtin/sandbox.rs:33-46` | ✅ | `require_within_worktree` used by all fs handlers; conservative symlink handling |
| `roles` (RoleToolProfile, denied_tools_for_role) | role allowlists | `roles.rs` | ✅ | `roko-cli/src/task_parser.rs:17` (production deny-list per role) |
| `MemorySubstrate` | 07-substrate-trait §3.1; v2/14 `memory-store` | `memory.rs:20-125` | ✅(test-tier) | roko-gate tests, roko-learn curriculum, roko-std `universal_loop.rs`; no `query_similar` |
| NoOp impls, `Sum/Mul/ConstScorer`, `First/HighestScore/RoundRobin` routers | kernel defaults | `noop.rs`, `scorer.rs`, `router.rs` | 🟡 | `SumScorer` in `orchestrate.rs:166`, `NoOpScorer` in `run.rs:57`, `NoOpGate` in roko-gate tests; routers test-only |
| `math.rs`, `greeting.rs` | — | `greeting.rs:1-3` | 🕰️ | day-one scaffolding (`format_greeting("…Roko is ready.")`) still shipped |
| **Bus (cross-crate)** | | | | |
| `Bus` trait | duality §3.1 (async, replay_since/current_seq/ring_len) | `roko-core/src/traits.rs:385-…` | 🟡 | actual trait is sync, `publish`/`subscribe` only — no `replay_since`/`current_seq` in the trait (replay is a concrete method on impls) |
| Pulse bus impls ×3 | duality §9 wants ONE `BroadcastBus` | (1) `roko-runtime/src/pulse_bus.rs`; (2) `roko-core/src/pulse_bus.rs`; (3) `roko-core/src/bus_backends.rs:27,109,211` (BroadcastBus/MemoryBus/MultiBus) | 🕰️ | (2) is an **orphan file — no `mod pulse_bus` anywhere in roko-core, never compiled** (same for orphan `roko-core/src/state_hub.rs`, 32-EVENTS concurs); (1) has zero consumers; (3) has exactly one production consumer |
| Live Pulse traffic | duality §§3-5 | ISFR relay bridge only: `roko-serve/src/lib.rs:2570-2591` (`BroadcastBus::new()` + `ISFRFeed`), `roko-agent-server/src/features/relay_subscriber.rs` | 🟡 | config-gated on `relay.url`; domain-specific (ISFR rates), not the kernel event fabric |
| Graduation (Pulse→Signal) | duality §4 | config `roko-core/src/config/graduation.rs:38-177`; `GraduationCell` `roko-graph/src/cells/graduation.rs:37-146` | 🟡 | cell in `roko_graph::default_registry()`; runtime paths = graph plan runner (`commands/plan.rs:1565-1644`) and `roko agent serve` cognitive loop (`agent_serve.rs:385-428`) — reachable but not the default orchestrate.rs path |
| Projection (Signal→Pulse `store.signal.written`) | duality §5 | — | ❌ | no publisher anywhere; `FileSubstrate::put` does not notify any bus |

## Builtin tool census

All 37 (`tool/builtin/mod.rs:50-72`, names `:75-97`): **std 16** — `read_file`, `write_file`, `edit_file`, `multi_edit`, `glob`, `grep`, `bash`, `ls`, `web_fetch`, `web_search`, `notebook_edit`, `todo_write`, `task` (module `task_agent`), `exit_plan_mode`, `apply_patch`, `run_tests`; **chain 17** — from `roko_chain::tools::CHAIN_DOMAIN_TOOLS` (defs in roko-chain, handlers registered in roko-cli `chain_registry.rs:51`); **ISFR 4** — `isfr::all_tool_defs()`.

- All 16 std handlers are real implementations, not stubs: `web_fetch` does reqwest GET with redirect cap + network capability (`web_fetch.rs:19-24,291`), `web_search` enforces capability/timeouts (`web_search.rs:157-190`), `task_agent` dispatches via delegate (`task_agent.rs:156-170`). The `handlers.rs:6-8` doc-comment ("a few are day-one stubs") is stale.
- Registration path: defs from `StaticToolRegistry` → per-role filter (`roles.rs`) → execution via `handler_for` closure injected into each provider's tool loop (`dispatcher/mod.rs:30,67`). Hosted backends (Claude CLI) use the CLI's own tools instead.
- Doc drift: CLAUDE.md says "19 builtin tools" (never true in this tree); `docs/v1/18-tools/01-builtin-tools.md:24` says "Current tool count: 16" (std subset only, predates chain/ISFR extension).
- Tests: `roko-std/tests/golden_tools.rs` (schema goldens, count assert), `builtin_handlers.rs` (behavioral, per-handler), `property_tests.rs`, `universal_loop.rs` (query→score→route→compose→act→verify→write on `MemorySubstrate`).

## Substrate & bus reality (vs store-and-bus-duality)

**Store half: realized.** Pull-based, durable, content-hash identity, decay-aware prune (`file_substrate.rs:317-325`), demurrage applied on a serve timer (`lib.rs:2020-2080`), hot→cold aging via ColdStore. Missing from the design contract: `query_similar` (defaulted to empty on both substrates despite fingerprints being computed and stored at put time) and prune-after-archive.

**Bus half: dormant.** The duality doc names today's `EventBus<E>` as the thing `BroadcastBus` replaces (§9) — that replacement exists **three times** but carries no kernel traffic. Production "bus" reality is typed enum buses: global `RokoEvent` (PlanRevision/PrdPublished actually consumed), type-keyed `RuntimeEvent` bus (runner→SSE/ACP/JSONL adapters), serve `ServerEvent` wrapper (`roko-serve/src/event_bus.rs:21-33`), `DashboardEvent` via StateHub, plus a separate `roko-learn` events bus (`event_loop.rs:1138`). Topic routing, `TopicFilter`, and graduation run only inside roko-graph's engine paths and the ISFR bridge. Projection is absent. **Verdict: duality is types-complete, transport-unwired — "built but never connected" in its purest form.** The `TODO(arch)` at `event_bus.rs:364-367` records the inverted dependency (roko-core depends on roko-runtime) that forces the type-keyed `Box::leak` bus registry hack and blocks a clean single Bus.

## V2-aligned

- Honest wiring headers (`STATUS: WIRED/NOT WIRED`) across roko-runtime — matches audit tooling expectations.
- `roko-core::Bus`/`Pulse`/`TopicFilter`/`GraduationConfig` + `GraduationCell` in the graph default registry; `Engram::from_pulse_synthetic`/`from_pulses` bridge types (`traits.rs:182,220`).
- ColdStore trait + scheduled archival + `[cold_storage]` config (items 14 done at the trigger level).
- HDC fingerprint attached at `FileSubstrate::put` (`file_substrate.rs:343-357`) — groundwork for `query_similar`.
- Contract-guard tests in `roko-runtime/src/lib.rs:104-285` (no duplicate foundation contracts; JSON round-trip for all 18 `RuntimeEvent` variants).
- roko-std universal-loop test proves the 1-noun/6-verbs kernel end-to-end.

## Old paradigm & tech debt

1. **Cold archival duplicates data hourly**: `archive_batch` (`cold_substrate.rs:218-242`) appends every candidate line and overwrites the index entry; hot store is never pruned, so the same ≤500 engrams re-archive every hour into the month file. Unbounded growth, wasted I/O.
2. **Triple Pulse bus** + orphan files: `roko-core/src/pulse_bus.rs` and `roko-core/src/state_hub.rs` are not in any module tree (dead code on disk); `roko-runtime::PulseBus` consumer-less.
3. **signals.jsonl vs engrams.jsonl split-brain** (55-DATA-DIR P0 confirmed): runner v2 appends raw `GateVerdict` JSON (non-Engram schema) to `signals.jsonl` (`runner/event_loop.rs:1147-1167`) while all substrate traffic uses `engrams.jsonl`.
4. **`SubstrateMigrator` is a husk** — thresholds struct, no behavior; migration logic copy-pasted ×3 (serve `lib.rs:2166-2187`, `orchestrate.rs:3875-3914`, `knowledge.rs:192-244`).
5. **roko-fs `GcEngine` never runs**; serve reimplements retention (`roko-serve/src/retention.rs:20` — second `RetentionPolicy` type; a third exists in `roko_learn::episode_logger`).
6. **Silent error swallow** in `file_substrate.rs:210-214`: malformed JSONL lines are skipped by a **no-op fn** (`tracing_line_error` — body is a comment only, no `tracing::warn!` call) whose comment claims "roko-fs doesn't depend on tracing" — **provably false** (`roko-fs/Cargo.toml:27`: `tracing = { workspace = true }`). The one-line fix is available with no new dependency. Same theme as commit ba0cd40 ("surface suppressed errors"), which fixed orchestrate/serve but not this.
7. **Heartbeat family duplication**: roko-runtime `heartbeat.rs` (~2.3k lines, producer never constructed) vs roko-cli's own `heartbeat.rs` (`HeartbeatClock` snapshot persister, wired into orchestrate) — two clocks, neither the design's tick loop.
8. **DeltaConsumer stub** duplicating roko-dreams (41-DREAMS.md), plus theta/energy/attention/probes/task_scheduler all unwired.
9. Two lifecycle trackers for agent sidecars — CLI raw-PID `agents.json` vs serve `ProcessSupervisor` (44-AGENT-SERVER.md:68).
10. Uncompressed "compressed" archives; `Bus` trait diverges from designed surface (sync, no replay_since/current_seq).
11. Leftover scaffolding: `greeting.rs`/`math.rs`; stale doc-comments (`handlers.rs` "day-one stubs"; layout's dual naming for one path).

## Not implemented

- `Store::query_similar` on File/Memory substrates (design: native HDC read primitive).
- Projection bridge (`store.signal.written` Pulse after put) — duality §5.
- Hot-store prune/compact after cold archival (move semantics).
- Kernel-wide Pulse traffic / topic-routed bus as the event fabric (only ISFR bridge + graph-engine islands).
- `GcEngine` runtime schedule (layout GC for runs/cache/episodes).
- Compression for cold archives; `purge_before` removes index entries but never deletes archive lines/files (`cold_substrate.rs:269-294`).
- Distributed bus backends (NATS/Kafka/ChainBus — duality §10); `MultiBus` exists in `bus_backends.rs:211` but unused.
- Heartbeat/tick cognitive loop as a running producer (BEAT-05).

## Migration checklist

- [ ] **[P0]** Make cold archival move-not-copy: after `archive_batch`, prune archived hashes from hot + `compact()`, and skip candidates already in the cold index (`roko-serve/src/lib.rs:2166-2187`, `roko-cli/src/orchestrate.rs:3875-3914`, `commands/knowledge.rs:237-244`, `cold_substrate.rs:218-242`) — verify: run two timer ticks in a fixture; `wc -l .roko/cold/$(date +%Y-%m).jsonl` unchanged on tick 2 and `roko status` engram count drops
- [ ] **[P0]** Stop the `signals.jsonl` split-brain: write runner-v2 gate verdicts as Engrams to `engrams_path()` (or dedicated `gate-verdicts.jsonl`) (`crates/roko-cli/src/runner/event_loop.rs:1147-1167`) — verify: `roko plan run <plan> && test ! -s .roko/signals.jsonl`
- [ ] **[P1]** Delete orphan `crates/roko-core/src/pulse_bus.rs` and `crates/roko-core/src/state_hub.rs` (never compiled) — verify: `grep -rn "mod pulse_bus\|mod state_hub" crates/roko-core/src` empty before and `cargo build -p roko-core` green after
- [ ] **[P1]** Pick ONE Pulse bus (recommend `roko_core::bus_backends::BroadcastBus`, the only one with a consumer) and delete `roko-runtime::PulseBus` — verify: `grep -rn "PulseBus" crates/ --include='*.rs' | grep -v test` empty
- [ ] **[P1]** Implement `query_similar` on `FileSubstrate` using the stored `hdc_fingerprint` tags (`file_substrate.rs:343-357`, trait default `roko-core/src/traits.rs:52-60`) — verify: new unit test returns ranked matches; `roko knowledge query` can hit it
- [ ] **[P1]** Surface the malformed-line skip in replay as `tracing::warn!` (`crates/roko-fs/src/file_substrate.rs:203-214`) — verify: corrupt-line test asserts a warning is logged (crate already depends on tracing)
- [ ] **[P2]** Give `SubstrateMigrator` a real `migrate(hot, cold) -> MigrationReport` and call it from all 3 archival sites (`cold_substrate.rs:301-340`) — verify: `grep -rn "archive_batch" crates/ --include='*.rs' | grep -v roko-fs` shows one call site
- [ ] **[P2]** Wire or delete the unwired roko-runtime six: `delta_consumer` (prefer delete — roko-dreams owns this), `theta_consumer`, `energy`, `heartbeat_attention`, `heartbeat_probes`, `task_scheduler` — verify: `grep -rln "STATUS: NOT WIRED" crates/roko-runtime/src` empty
- [ ] **[P2]** Unify retention/GC: either schedule `roko_fs::GcEngine` or fold its scope into serve `retention.rs` and delete; de-collide the three `RetentionPolicy` types — verify: `grep -rn "struct RetentionPolicy" crates/ --include='*.rs'` returns one
- [ ] **[P2]** Unify sidecar lifecycle onto `ProcessSupervisor` (see 44-AGENT-SERVER P0) — verify: `rg -n 'libc::kill' crates/roko-cli/src/agent_serve.rs` empty
- [ ] **[P3]** Compress cold archives (zstd) and make `purge_before` reclaim disk, or fix the "compressed" claims (`cold_substrate.rs:3`, `roko-core/src/traits.rs:87`, `roko-serve/src/lib.rs:2085`) — verify: docs/code agree; archive file shrinks after purge
- [ ] **[P3]** Align `roko_core::Bus` trait with design surface (async, `replay_since`, `current_seq`, `ring_len`) or annotate the divergence in 07b — verify: trait diff vs duality §3.1
- [ ] **[P3]** Doc fixes: CLAUDE.md "19 builtin tools" → 16 std / 37 total; v1/18-tools count caveat; `handlers.rs:6-8` stale stub comment; remove `greeting.rs`/`math.rs` or mark demo-only — verify: `grep -rn "19 builtin" CLAUDE.md` empty

## Open questions

1. **Which bus is the v2 kernel?** roko-graph's engine pulses, `bus_backends::BroadcastBus` (ISFR), or a wired `PulseBus`? 32-EVENTS-BUS-STATEHUB.md asks the same; a decision unblocks deleting two implementations.
2. **Dependency direction**: `roko-core` currently depends on `roko-runtime` (see `event_bus.rs:364-367` TODO). Reversing it (runtime → core, per the architecture reference) would let `RuntimeEvent` use one typed global bus and kill the `Box::leak` type-registry. Who owns that refactor?
3. **engrams vs signals naming**: v2 renames Engram→Signal (alias exists); does the file follow (`engrams.jsonl`→`signals.jsonl`, reversing the v1 migration) or stay? Blocks P0 item 2's target filename.
4. **Should graduation run on the default plan path?** GraduationCell only fires in graph-engine/cognitive-loop paths; if orchestrate.rs remains the default executor, Pulse graduation never runs in normal self-hosting.
5. **PointerStore/ToolAuditLog/BanditStore/MetricsLog**: four roko-fs persistence utilities with no callers — earmarked for which subsystems (context-pack pointers, §36 audit, learn bandits, run metrics), or delete?

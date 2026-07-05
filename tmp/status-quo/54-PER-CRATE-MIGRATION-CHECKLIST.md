# 54 — Per-Crate Migration Checklist (Zero-Debt Roadmap)

**Verification:** HEAD `5852c93c05`, branch `main`, 2026-07-08.
**Companion to:** `03-CRATE-AUDIT.md` (exhaustive per-crate reference).
**Scope:** all 34 workspace packages (31 crates + 3 apps) + the `tests/` member.

This is the crate-level ownership map for reaching **zero structural debt**. Each entry lists the
concrete work to migrate a package from "compiles + partly wired" to "canonical, no dup types, no
dead code, deps clean." Cross-cutting migrations (the `Engram` rename, the Legacy-island quarantine,
the HDC unification) are enumerated first because they touch many crates at once.

Legend: `[ ]` open · `[~]` partially done · `[x]` believed done. Status tags: **Live / Partial / Unwired / Legacy / App** (see doc 03).

---

## 0. Cross-cutting migrations (do these first; they unblock many crates)

### M1 — Rename `Engram` canonical, retire `Signal`/`Store` aliases
- [ ] `roko-core`: make `Engram` the only public noun; keep `Signal`/`Store` as **deprecated** aliases (`#[deprecated]`) for one release, then delete (`roko-core/src/signal.rs`, `signal_kinds.rs`).
- [ ] Sweep every crate's doc comments + type names that say "Signal"/"signal stream" where they mean Engram (conductor, compose, learn headers all do).
- [ ] Rename `.roko/signals.jsonl` handling in `roko-fs` to Engram-aware naming (keep on-disk path for back-compat, add migration in `roko-fs/src/layout.rs`).

### M2 — Quarantine the Legacy execution island (~52K LOC)
Affects `roko-cli/orchestrate.rs`, `roko-orchestrator`, `roko-conductor`.
- [ ] Confirm no live command reaches `orchestrate.rs::PlanRunner` (verified 2026-07-08: only self-tests + `lib.rs:156` re-export). Delete the re-export, then delete `orchestrate.rs` or move to `crates/roko-cli/legacy/` behind a `legacy-orchestrator` feature (off by default).
- [ ] Delete `roko-orchestrator::executor` (superseded by `runner/task_dag` + `runner/merge`). Keep only types still imported elsewhere (audit `roko-serve`, `roko-acp` imports first).
- [ ] Rewire or retire `roko-conductor` (see its entry).

### M3 — Delete duplicate safety subsystem
- [ ] Delete `roko-orchestrator/src/safety/` (~3.4K LOC: `loop_guard`, `capability_tokens`, `permit`, `audit_chain`, `sandboxing`, `taint_propagation`). Canonical safety is `roko-agent/src/safety/`. Repoint any importer.

### M4 — Unify HDC (two incompatible implementations today)
- [ ] Make `roko-index` depend on `roko-primitives` and delete `roko-index/src/hdc.rs`; re-export `roko_primitives::hdc`.
- [ ] Decide the optional-`roko-primitives` feature in `roko-fs`, `roko-compose`, `roko-neuro`, `roko-serve`: either turn it on by default (HDC is real) or delete the gated code. No half-compiled HDC.

### M5 — Converge on one DAG engine
- [ ] Pick Runner v2 (`roko-cli/runner/task_dag`) as canonical. Either wire `roko-graph` cells to real dispatch/resume and promote it, or fold Graph's `Cell`/loader concepts into the runner and retire the standalone engine. Delete the orchestrator executor (M2).

### M6 — Fix the runtime layer inversion
- [ ] Remove `roko-runtime → roko-gate` (and `→ roko-compose`, `→ roko-learn`) edges in `roko-runtime/Cargo.toml`. Runtime should consume gate/compose/learn via injected traits/results, not concrete crate deps.

### M7 — Move reusable runtime concepts out of `roko-cli`
- [ ] Promote `runner/state.rs`, `runner/projection.rs`, `runner/snapshot_writer.rs` state/projection contracts into `roko-runtime` (align with its `StateHub`), leaving `roko-cli` as a thin wiring layer.

---

## Tier 1 — Kernel

### roko-core — **Live** — Keep (split)
- [ ] M1 (owner). Pick canonical nouns + result types; keep config schema as the single source of truth.
- [ ] Extract runtime/state copies: `state_hub.rs`, `pulse_bus.rs`, and stale `obs/` runtime pieces → quarantine/delete after proving `roko-runtime::StateHub` is the live one.
- [ ] Wire or delete `loop_tick.rs` (defined, never called on live path).
- [ ] Audit the 60+ `pub mod`s; move domain types (chain `isfr_feed`, affect) toward their crates where cycle-free.

### roko-primitives — **Partial** — Keep
- [ ] Become the sole HDC provider (M4).
- [ ] Quarantine/delete unused math (`manifold`, `sheaf`, `tda`, `tropical`) unless a live consumer is proven. Keep `hdc`, `tier`, `robust_stats`, `codebook`, `pad`.
- [ ] Keep as a leaf library with zero workspace deps (currently clean).

### roko-fs — **Live** — Keep
- [ ] Enforce `.roko` layout exclusively through `layout.rs` helpers; add a migration command for legacy paths (M1).
- [ ] Wire `cold_substrate.rs` to a runtime trigger or delete it (dormant today; matches CLAUDE.md #14).
- [ ] Dedupe `bandit.rs` against `roko-learn`'s bandits; pick one home.
- [ ] Resolve the optional `roko-primitives` feature (M4).

### roko-runtime — **Partial** — Keep (prune)
- [ ] M6 (owner): drop gate/compose/learn deps.
- [ ] Make `StateHub` + event-envelope the canonical projection contract (M7); retire stale copies in `roko-core`.
- [ ] Delete `workflow_engine.rs` (dead) and dormant consumers (`theta/delta/demurrage_consumer`, `effect_driver`) unless a live caller is added.

### roko-std — **Live** — Keep
- [ ] Make stub/noop tool handlers return explicit "unsupported" errors instead of silent success.
- [ ] Register tool metrics/audit consistently through `roko-fs` sinks.
- [ ] Consider splitting default/noop primitives from chain-coupled tool handlers (relieves `roko-std → roko-chain`).
- [ ] Pin the builtin-tool count in one authoritative list (drifted across docs).

---

## Tier 2 — Verify / Compose / SPI

### roko-gate — **Live** — Keep
- [ ] Ensure Graph and Runner use the same gate contract (`gate_service`/`gate_pipeline`).
- [ ] Make absent-tool gates **fail explicitly** rather than stub-pass.
- [ ] Persist adaptive thresholds consistently (`.roko/learn/gate-thresholds.json`); confirm one writer.
- [ ] Audit SPC modules (`pelt`, `hotelling`, `spc`, `ratchet`) for live use; gate or trim if speculative.
- [ ] Note the `roko-gate → roko-agent` dep is why M6 matters (runtime must not pull gate→agent).

### roko-compose — **Live** — Keep (prune)
- [ ] Decide VCG: make `auction::vcg_allocate` the default allocator or delete it + the stale docs (greedy path dominates today).
- [ ] Narrow `roko-compose → roko-agent/learn/neuro` toward trait boundaries; the `→ roko-learn` back-edge is dev-only (keep it that way).
- [ ] Audit large low-traffic modules (`cognitive_workspace`, `symbol_resolver`, `compaction`, `strategy`) for live coverage.

### roko-plugin — **Unwired** — Quarantine
- [ ] Decide: is this the v2 extension SPI, or archive? Today it exports only `manifest` while real extension loading lives in `roko-cli/runner/extension_loader.rs` + `roko-core/src/extension.rs`.
- [ ] If archiving: remove from `default`/`members` consumers or gate behind a feature; drop the unused `cron`/`notify`/`globset` deps.
- [ ] If keeping: actually implement the event-source/feedback-collector runtime and wire it into Runner v2.

---

## Tier 3 — Agent / Learning / Knowledge / Affect

### roko-agent — **Live** — Keep (split)
- [ ] Ensure every provider/tool path uses the shared safety/metrics/routing (canonical safety is here — see M3).
- [ ] Fix `AgentContract` permissive-default fallback when YAML is missing (CLAUDE.md "Partial safety contracts").
- [ ] Prove or delete experimental backends: `hermes`, `openclaw`, `metamorphosis`, `nl_to_format`.
- [ ] Split candidate: `roko-agent-core` (trait + dispatcher + tool_loop + safety) vs `roko-agent-backends` (10 providers).
- [ ] Keep `roko-agent → roko-learn` as dev-only (runtime edge is learn→agent; no cycle).

### roko-agent-server — **Live** — Keep / merge?
- [ ] Clarify relationship to `roko-serve` aggregator + `agent-relay` (all three expose agent visibility). Decide canonical surface; merge if the sidecar can be a serve feature.
- [ ] Add tests (undertested for its route complexity).

### roko-learn — **Live** — Keep (prune)
- [ ] Collapse duplicate cost/reward roots: `cost_table` / `costs_db` / `costs_log` / `local_reward` → one store.
- [ ] Implement knowledge-informed routing: consult `roko-neuro` in `cascade_router` (CLAUDE.md #13).
- [ ] Learn from manual `force_backend` overrides in the cascade router (CLAUDE.md #15 / UX34).
- [ ] Preserve routing-source fidelity; handle feedback loss explicitly (don't drop on error).
- [ ] Consume HDC fingerprints via `roko-primitives` (already does — keep aligned under M4).

### roko-neuro — **Live** — Keep
- [ ] Decide the canonical knowledge-ingestion path and the HDC/retrieval feature (optional `roko-primitives`) — resolve under M4.
- [ ] Expose neuro to `roko-learn::cascade_router` for M-learn routing item.

### roko-dreams — **Partial** — Keep (gate Phase 2)
- [ ] Consume `routing_advice` in the cascade router or document it as a non-goal.
- [ ] Feature-gate `phase2.rs` stubs out of the default build.
- [ ] Add a real scheduler (cron/delta/BusPulse) or document that dreams fire only when Runner v2 constructs `DreamRunner` (`runner/event_loop.rs:5491`).

### roko-daimon — **Live** — Keep (gate Phase 2)
- [ ] Reconcile duplicate affect representations: `roko-core/src/affect.rs` vs this crate's PAD state.
- [ ] Feature-gate Phase-2 stubs (`goals`, `mortality`, `life_review`).
- [ ] Keep proving live dispatch modulation (`DaimonTaskHook` in `runner/event_loop.rs:3028+`) with a regression test.

### roko-conductor — **Legacy** — Merge / rewire
- [ ] Port the critical watchers (stuck-detection, circuit breaker, health) into Runner v2's event loop (today it sets `conductor_load: 0.0` and never calls the conductor).
- [ ] `roko_conductor::` is referenced only in `orchestrate.rs` + own tests — after M2, this crate has **zero live callers**. Either rewire (above) or mark explicitly legacy-only and drop from `roko-cli`/`roko-serve` deps.
- [ ] Make the `roko-conductor → roko-learn` coupling event-driven if it survives.

### roko-orchestrator — **Legacy** — **Quarantine**
- [ ] Delete `src/safety/` (M3).
- [ ] Delete `src/executor/` (M5 — superseded by Runner v2).
- [ ] Audit which types are still imported by `roko-serve`/`roko-acp`/`roko-cli`; keep only those as a thin `roko-orchestrator-types` library, or inline them. Target: shrink from 20K LOC to near-zero.

### roko-graph — **Partial** — Keep or fold (M5)
- [ ] Wire `AgentCell`/`ComposeCell`/gate cells to real dispatch + resume (they dry-run today).
- [ ] Decide: promote as the v2 engine, or fold `Cell`/`loader`/`topo` into Runner v2 and retire.
- [ ] Remove the `orchestrate.rs::run_with_v2_engine` bridge once the decision lands.

---

## Tier 4 — Chain / HTTP / Editor / Intelligence

### roko-chain — **Partial** — Keep (split)
- [ ] Split mock / local / live-chain authority behind clear features (`alloy-backend` = live).
- [ ] Wire chain tools (`tools.rs`, surfaced via `roko-std`) into normal workflows or mark experimental.
- [ ] Gate the speculative market/ISFR surface (`futures_market`, `identity_economy_*`, `x402`, `korai_token`, `phase2`) so it doesn't ship silently.
- [ ] Add live-chain proof gates before claiming witness anchoring (Phase 2+, CLAUDE.md #16).

### roko-serve — **Live** — Keep (prune)
- [ ] Produce a route manifest + auth matrix (~85+ routes; some are stubs).
- [ ] Decide persistence for currently in-memory state; document StateHub projection contract (`projection_contract.rs`, `truth_map.rs`, `parity.rs`).
- [ ] Document serve vs `mirage-rs` `/api/*` as **separate** surfaces.
- [ ] Resolve agent-visibility overlap with `roko-agent-server` + `agent-relay`.

### roko-acp — **Live** — Keep
- [ ] Permission/capability/session parity with ACP spec; MCP env passthrough.
- [ ] Drop the `roko-acp → roko-orchestrator` edge once the orchestrator executor is quarantined (M2).

### roko-index — **Partial** — Keep
- [ ] M4: depend on `roko-primitives`, delete private `hdc.rs`.
- [ ] Integrate index freshness with serve/TUI; auto-invoke (or explicitly opt-in) during plan execution.
- [ ] Keep parser contract aligned with the three lang crates.

### roko-lang-rust / -typescript / -go — **Live** — Keep
- [ ] Keep `BuildSystem`/`LanguageProvider` contracts aligned with `roko-index`.
- [ ] Improve TS/Go parsers (thin single-file regex parsers) or adopt tree-sitter as `roko-lang-rust` does.
- [ ] Decide if these are public extension points; if not, they can merge into `roko-index`.

### roko-mcp-code — **Live** — Keep
- [ ] Enforce workspace-root + safety policy consistently.

### roko-mcp-stdio — **Live** — Keep
- [ ] Make this the single MCP transport; fold the unified MCP-config + ACP-passthrough story around it.

### roko-mcp-github / -slack / -scripts — **App** — Keep / merge
- [ ] Route auth/env through common secret handling; document required env.
- [ ] `roko-mcp-scripts`: maintain allowlist + timeouts + a safety proof (executes scripts).
- [ ] Consider merging slack/scripts/github into one multi-tool MCP crate (each is a single file today) sharing `roko-mcp-stdio`.

### roko-cli — **Live** — Keep (**shrink**)
- [ ] M2: delete/quarantine `orchestrate.rs` (22K LOC dead-by-default).
- [ ] M7: move state/projection/snapshot runtime contracts to `roko-runtime`.
- [ ] Unify `dispatch.rs` vs `dispatch_v2.rs`.
- [ ] Fix engine defaults/resume so the CLI always drives Runner v2; delete the Legacy fallbacks.
- [ ] Drop `roko-orchestrator` + `roko-conductor` deps after M2/M3.

### roko-demo — **Partial** — Move under apps
- [ ] Move `crates/roko-demo` → `apps/roko-demo` (it's an app, not a library layer).
- [ ] Clarify supported demo status; document the stub-provider default.

---

## Apps

### mirage-rs (app) — **App** — Keep (separate)
- [ ] Document `/api/*` route semantics as distinct from `roko-serve`.
- [ ] Keep feature layering explicit: base (pure EVM) / `chain` (HDC+pheromone) / `roko` (core-trait bridge).

### agent-relay (app) — **App** — Keep / merge
- [ ] Decide: standalone relay vs `roko-serve` relay proxy as canonical; if serve wins, fold in `bus.rs`/`chain_watcher.rs`.

### roko-chain-watcher (app) — **App** — Merge candidate
- [ ] Add live-chain proof gates + config docs (has real `block_observer` + dry-run modes).
- [ ] Evaluate merging into `roko-chain` (as an observer) or `roko-serve` (as a feed-agent) instead of a standalone binary.

### tests (member) — Keep
- [ ] Add default-path (Runner v2) execution tests, route tests, state-migration tests, and frontend-contract tests. The live runner is under-tested at the integration level.

---

## Layer inversions & crate-map drift (reference)

- `roko-runtime → roko-gate` (also `→ roko-compose`, `→ roko-learn`): **layer inversion** (M6).
- `roko-core` carries runtime/state/bus/obs copies that overlap `roko-runtime::StateHub` (M7 / core split).
- `roko-std → roko-chain`: chain coupling in the std tool layer; split if pressure grows.
- `roko-compose → roko-agent/learn/neuro`: target trait boundaries.
- `roko-acp → roko-orchestrator`: inherits Legacy quarantine risk.
- `docs/v1/00-architecture/15-crate-map.md` names non-existent target crates
  (`roko-bus`, `roko-hdc`, `roko-spi`, `roko-defaults`, `roko-tools`, `roko-compose-core`,
  `roko-templates`); `README.md` undercounts the 34 current members. Update both.
- Old audits list `roko-benches` / `roko-test-utils`; neither is a member at HEAD `5852c93c05`.

## Keep / Merge / Quarantine / Delete verdict (one-line each)

| Package | Verdict | One-line reason |
|---|---|---|
| roko-core | Keep + split | Kernel truth, but hosts runtime/state copies to extract |
| roko-primitives | Keep | Sole HDC provider once M4 lands; trim unused math |
| roko-fs | Keep | Substrate; wire/delete cold_substrate |
| roko-runtime | Keep + prune | StateHub canonical; kill workflow_engine + inverted deps |
| roko-std | Keep | Make stub tools error; split chain coupling |
| roko-gate | Keep | Verify stack; fix stub-pass gates |
| roko-compose | Keep + prune | Decide VCG; narrow deps |
| roko-plugin | **Quarantine** | 2-file facade, no live loader |
| roko-agent | Keep + split | Central; split core vs backends |
| roko-agent-server | Keep/merge | Overlaps serve/relay |
| roko-learn | Keep + prune | Collapse cost/reward dup roots |
| roko-neuro | Keep | Decide HDC-retrieval + ingestion path |
| roko-dreams | Keep (gate P2) | v2-only; gate phase2; consume routing advice |
| roko-daimon | Keep (gate P2) | Live modulation; reconcile affect dup |
| roko-conductor | **Merge/rewire** | Zero live callers after M2 |
| roko-orchestrator | **Quarantine** | Legacy executor + dup safety; shrink to types |
| roko-graph | Keep or fold | Third DAG engine; pick one (M5) |
| roko-chain | Keep + split | Mock-default; gate speculative markets |
| roko-serve | Keep + prune | Route/auth manifest; persistence |
| roko-acp | Keep | Drop orchestrator edge |
| roko-index | Keep | Unify HDC (M4); auto-invoke |
| roko-lang-rust/ts/go | Keep | Align with index; improve thin parsers |
| roko-mcp-code | Keep | Enforce root+safety |
| roko-mcp-stdio | Keep | Canonical MCP transport |
| roko-mcp-github/slack/scripts | Keep/merge | Thin single-file; unify + secret handling |
| roko-cli | Keep + **shrink** | Delete orchestrate.rs; move runtime out |
| roko-demo | **Move → apps/** | It's an app, not a crate layer |
| mirage-rs | Keep (separate) | Distinct product surface |
| agent-relay | Keep/merge | Decide vs serve relay |
| roko-chain-watcher | Merge candidate | Could be chain observer / feed-agent |
| tests | Keep + expand | Under-tests the live runner path |

# UX Implementation — Master Issue Tracker

**Source plans**: `tmp/ux/implementation-plans/00-INDEX.md` (and the 12
plan files alongside it).
**Created**: 2026-05-01.
**Verified**: 2026-05-01 against `agent-refinements` HEAD.
**Scope**: Every issue still open in `tmp/ux/` after the 2026-04-20
re-audit, mapped to either a Codex batch in this runner or a manual
checklist in `manual-tracks/`.

This tracker is the single source of truth. Every batch in
`batches.toml` corresponds to exactly one row here. Tick a row when its
batch lands and verifies green; strike it through if it turns out to be
already-done after closer inspection.

## Status legend

| Mark | Meaning |
|------|---------|
| `[ ]` | open, batch defined |
| `[~]` | partial / multi-batch fix in flight |
| `[x]` | verified fixed (close the row, leave it for history) |
| ~~strike~~ | verified obsolete after deeper audit (no batch needed) |
| `(M)` | manual track — see `manual-tracks/<plan>/CHECKLIST.md` |
| `(P)` | parked — see `manual-tracks/12-phase2-vision/CHECKLIST.md` |

---

## Wave M — Mirage extraction (plan 01-mirage-extraction-final)

> Drives `apps/mirage-rs` from "EVM substrate **plus** dashboard backend"
> to "EVM substrate **only**". Phase 3 of the architecture vision.
> Precondition: Wave AG and Wave DB must be merged before Wave M Step 4.

| Batch | Title | Touches | Status |
|-------|-------|---------|--------|
| M01 | Audit no live consumers depend on mirage REST routes | (audit only) | `[ ]` |
| M02 | Drop the `chain` → `dashboard-api` implication | `apps/mirage-rs/Cargo.toml` | `[ ]` |
| M03 | Add slim `http_health.rs` (`/health` + `/stats`) | `apps/mirage-rs/src/http_health.rs` (NEW), `main.rs` | `[ ]` |
| M04 | Delete `chain/`, `http_api/`, `roko_bridge/` modules | `apps/mirage-rs/src/{chain,http_api,roko_bridge}/`, `lib.rs`, `Cargo.toml` | `[ ]` |
| M05 | Drop `chain_*` JSON-RPC methods from `rpc.rs` | `apps/mirage-rs/src/rpc.rs` | `[ ]` |
| M06 | Rewire `scenario.rs` if it referenced ChainContext | `apps/mirage-rs/src/scenario.rs`, fixtures | `[ ]` |
| M07 | Update README + CLAUDE.md + close plan | `apps/mirage-rs/README.md`, `CLAUDE.md`, `tmp/ux/02-mirage-extraction.md` | `[ ]` |

---

## Wave AG — Aggregator backends (plan 02-aggregator-knowledge-pheromones)

> Closes the two remaining gaps in the aggregator's mirage-compatible
> surface so nunchi-dashboard can switch its base URL (Wave DB) without
> losing the InsightBoard and pheromone views.

| Batch | Title | Touches | Status |
|-------|-------|---------|--------|
| AG01 | Capture legacy mirage-rs response fixtures | `tmp/runners/ux-impl/fixtures/*` (NEW) | `[ ]` |
| AG02 | `InsightBoardReader` in `roko-chain` (read-only) | `crates/roko-chain/src/insight_board.rs` (NEW) | `[ ]` |
| AG03 | `AgentCardFetcher` (HTTP / IPFS / data-URI) | `crates/roko-chain/src/agent_card_fetcher.rs` (NEW) | `[ ]` |
| AG04 | `KnowledgeSource` enum on `AppState` | `crates/roko-serve/src/state.rs` | `[ ]` |
| AG05 | Replace knowledge handlers in aggregator | `crates/roko-serve/src/routes/aggregator.rs` | `[ ]` |
| AG06 | `PheromoneField` module + persistence | `crates/roko-serve/src/pheromone.rs` (NEW) | `[ ]` |
| AG07 | Add `/api/pheromones/*` routes | `crates/roko-serve/src/routes/aggregator.rs` | `[ ]` |
| AG08 | Chain event subscription → cache invalidation | `crates/roko-serve/src/lib.rs` | `[ ]` |
| AG09 | Compat tests vs captured fixtures | `crates/roko-serve/tests/{knowledge,pheromone}_compat.rs` (NEW) | `[ ]` |
| AG10 | OpenAPI rows + CLAUDE.md + close plan | `crates/roko-serve/src/openapi.rs`, `CLAUDE.md`, `tmp/ux/04-dashboard-migration.md` | `[ ]` |

---

## Wave DB — Dashboard URL migration (plan 03-dashboard-url-migration) `(M)`

> Sibling repo `nunchi-dashboard/` (TypeScript / Next.js). The Rust
> runner cannot drive this repo; checklist for hand execution.
> See `manual-tracks/03-dashboard-url-migration/CHECKLIST.md`.

| Step | Title | Touches | Status |
|------|-------|---------|--------|
| DB01 | Verify roko-serve answers every aggregator route used by dashboard | (smoke probe) | `[ ]` `(M)` |
| DB02 | Split env vars: `VITE_CHAIN_RPC_URL` + `VITE_API_URL` | `nunchi-dashboard/.env.example`, `src/services/constants.ts` | `[ ]` `(M)` |
| DB03 | Reroute every `MIRAGE_BASE` usage in `src/services/` | `nunchi-dashboard/src/services/*.ts` | `[ ]` `(M)` |
| DB04 | Two-pill connectivity probe (REST + JSON-RPC) | `nunchi-dashboard/src/stores/connectivityStore.ts` | `[ ]` `(M)` |
| DB05 | Smoke script `scripts/smoke-aggregator.sh` + CI | `nunchi-dashboard/scripts/`, workflow | `[ ]` `(M)` |
| DB06 | README + post-rollout cleanup window | `nunchi-dashboard/README.md` | `[ ]` `(M)` |

---

## Wave CH — ERC-8004 chain discovery (plan 04-erc8004-chain-discovery)

> Make agent discovery a chain primitive, not a roko-serve-internal
> registry. Capability bitmask bit 15 + `"roko"` domain tag.

| Batch | Title | Touches | Status |
|-------|-------|---------|--------|
| CH01 | Contract: add `updateAgentCard(passportId,uri,mask)` | `contracts/src/IdentityRegistry.sol`, `test/` | `[ ]` |
| CH02 | `IdentityRegistryReader` in roko-chain | `crates/roko-chain/src/identity_registry.rs` (NEW) | `[ ]` |
| CH03 | `roko-core::capability_bits` constants | `crates/roko-core/src/capability_bits.rs` (NEW) | `[ ]` |
| CH04 | Agent-server registration uses v2 selector + bit 15 | `crates/roko-agent-server/src/registration.rs` | `[ ]` |
| CH05 | Aggregator merge: chain ∪ local registry | `crates/roko-serve/src/routes/aggregator.rs`, `state.rs` | `[ ]` |
| CH06 | `GET /api/agents/discover-chain` debug route | `crates/roko-serve/src/routes/aggregator.rs` | `[ ]` |
| CH07 | Demo bootstrap: 5 chain-registered passports | `crates/roko-demo/src/scenarios/bootstrap_passports.rs` (NEW) | `[ ]` |
| CH08 | Network-only mode: dashboard-side chain enumeration `(M)` | `nunchi-dashboard/src/services/discovery-chain.ts` | `[ ]` `(M)` |
| CH09 | docs/v2/chain-discovery.md + close plans | `docs/v2/chain-discovery.md` (NEW), `CLAUDE.md`, `tmp/ux/03-auth-and-discovery.md` | `[ ]` |

---

## Wave TU — TUI event parity (plan 05-tui-event-parity-final)

> Eliminate the remaining "re-parse whole file on stamp change" sites in
> the TUI. Items 70-78 from `tmp/ux/ux-followup/12-tui-event-parity.md`.

| Batch | Followup | Title | Touches | Status |
|-------|----------|-------|---------|--------|
| TU01 | 76 | Learning trio uses `IncrementalTailer` | `crates/roko-cli/src/tui/dashboard.rs` | `[ ]` |
| TU02 | 73 | Episode log incremental tail | `crates/roko-cli/src/tui/dashboard.rs` | `[ ]` |
| TU03 | 71 | Gate signals incremental tail | `crates/roko-cli/src/tui/dashboard.rs` | `[ ]` |
| TU04 | 72 | Task outputs watched per-file via `fs_watch` | `crates/roko-cli/src/tui/{dashboard,task_outputs}.rs` | `[ ]` |
| TU05 | 74 | Event log: choose JSONL conversion or array tailer | `crates/roko-cli/src/tui/dashboard.rs`, `crates/roko-runtime/src/event_bus.rs` | `[ ]` |
| TU06 | 78 | Persist generation counter to `.roko/state/dashboard-gen.json` | `crates/roko-cli/src/tui/dashboard_gen_persist.rs` (NEW) | `[ ]` |
| TU07 | 70 | Agent panel from aggregator `/api/ws` (WS multiplex) | `crates/roko-cli/src/tui/{views/agents_view,state,ws_client}.rs`, `crates/roko-cli/src/tui/agent_streams.rs` (NEW) | `[ ]` |

---

## Wave MC — MCP coverage audit (plan 06-mcp-coverage-audit)

> Audit + bucket each `roko-mcp-*` crate. Per-crate decision drives the
> follow-on per-crate batches.

| Batch | Title | Touches | Status |
|-------|-------|---------|--------|
| MC01 | Symbol + call-site + handshake audit; produce matrix | `docs/v2/MCP-AUDIT.md` (NEW) | `[ ]` |
| MC02 | Bucket decision per crate (shipped-default / opt-in / deprecate / WIP) | `docs/v2/MCP-AUDIT.md` | `[ ]` |
| MC03 | Integration test for each shipped-default crate | `crates/roko-mcp-*/tests/integration.rs` (NEW) | `[ ]` |
| MC04 | Update `roko.toml.example` + `docs/v2/MCP-INTEGRATION.md` | `roko.toml.example`, `docs/v2/MCP-INTEGRATION.md` | `[ ]` |
| MC05 | CLAUDE.md Key crates rows reflect new buckets | `CLAUDE.md` | `[ ]` |
| MC06 | Close followup item 34 | `tmp/ux/ux-followup/05-partially-wired-subsystems.md` | `[ ]` |

---

## Wave FG — Phase-2 feature gating (plan 07-phase2-feature-gating)

> `default-members` for the shipped slice. `--workspace --all` still
> builds Phase 2 crates. CI weekly cron job catches bit-rot.

| Batch | Followup | Title | Touches | Status |
|-------|----------|-------|---------|--------|
| FG01 | 32 + 33 | Add `default-members` to root `Cargo.toml` | `Cargo.toml` | `[ ]` |
| FG02 | — | Add weekly `phase2-build` CI job | `.github/workflows/phase2-build.yml` (NEW) | `[ ]` |
| FG03 | 54 | `roko-plugin` audit + decision | `crates/roko-plugin/`, `tmp/ux/ux-followup/08-phase-2-vision.md` | `[ ]` |
| FG04 | — | CLAUDE.md "Default-built" column | `CLAUDE.md`, `docs/v2/WORKSPACE.md` (NEW) | `[ ]` |

---

## Wave BP — Agent backend parity (plan 08-agent-backend-parity)

> Bring Codex / Cursor / Gemini / Perplexity / Ollama to test-parity
> with the Claude reference. Cascade router gets an end-to-end test.
> Items 36, 37, 38, 39, 40, 40a + 60c.

| Batch | Followup | Title | Touches | Status |
|-------|----------|-------|---------|--------|
| BP01 | 38 | Build `parity_kit.rs` test helper | `crates/roko-agent/tests/_helpers/parity_kit.rs` (NEW) | `[ ]` |
| BP02 | 38 | Rewrite Claude tests around the kit (golden) | `crates/roko-agent/tests/claude*.rs` | `[ ]` |
| BP03 | 36 | Codex 10-turn conformance fixture + harness | `crates/roko-agent/tests/codex_conformance.rs` (NEW), fixtures | `[ ]` |
| BP04 | 37 | Cursor `send_turn_streaming` parity | `crates/roko-agent/src/cursor_agent.rs`, `tests/cursor_parity.rs` (NEW) | `[ ]` |
| BP05 | 38 | Gemini parity tests | `crates/roko-agent/tests/gemini_parity.rs` (NEW) | `[ ]` |
| BP06 | 38 | Perplexity parity tests | `crates/roko-agent/tests/perplexity_parity.rs` (NEW) | `[ ]` |
| BP07 | 38 | Ollama parity tests | `crates/roko-agent/tests/ollama_parity.rs` (NEW) | `[ ]` |
| BP08 | 39 | ExecAgent / ClaudeCliAgent consolidation (Option A) | `crates/roko-agent/src/{cli_agent,exec,claude_cli_agent}.rs` | `[ ]` |
| BP09 | 40 | Gemini single-file; Ollama single-file; Perplexity directory cleanup | `crates/roko-agent/src/{gemini,ollama,perplexity}/` | `[ ]` |
| BP10 | 40a + 60c | Cascade router integration test | `crates/roko-learn/tests/cascade_router_integration.rs` (NEW) | `[ ]` |
| BP11 | — | `docs/v2/AGENT-BACKENDS.md` coverage table | `docs/v2/AGENT-BACKENDS.md` (NEW), `CLAUDE.md` | `[ ]` |

---

## Wave DC — Stale docs and drift sweep (plan 09-stale-docs-and-drift) `(M)`

> Markdown / docs only. Cheap to do mechanically by hand, also cheap to
> codify as a runner batch — kept under `manual-tracks/` because the
> work has zero compile risk and benefits from human review of the
> banned-term replacements.

| Step | Followup | Title | Touches | Status |
|------|----------|-------|---------|--------|
| DC01 | 47 + 64 | Banner sweep across `bardo-backup/tmp/roko-progress/*.md` | `bardo-backup/tmp/roko-progress/*.md` | `[ ]` `(M)` |
| DC02 | 65 | Replace `grimoire`/`styx`/`clade` outside `bardo-backup/` | `*.md`, `*.rs`, `*.toml` (live) | `[ ]` `(M)` |
| DC03 | 66 | Replace `mortal`/`death`/`reincarnation` in live docs | `*.md` (live) | `[ ]` `(M)` |
| DC04 | 65 + 66 | CI guard: banned terminology in live `*.md` | `.github/workflows/banned-terms.yml` (NEW) | `[ ]` `(M)` |
| DC05 | 46 + 67 | Add stale banner to `MORI-PARITY-CHECKLIST.md` (Path B) | `bardo-backup/tmp/roko-progress/MORI-PARITY-CHECKLIST.md` | `[ ]` `(M)` |
| DC06 | 67a | Refresh `tmp/implementation-plans/00-INDEX.md` status | `tmp/implementation-plans/00-INDEX.md` | `[ ]` `(M)` |
| DC07 | 45 | CLAUDE.md "What to work on" 1-9 carry "smoke test pending" qualifier | `CLAUDE.md` | `[ ]` `(M)` |
| DC08 | — | Create `docs/v2/RENAMES.md` mapping | `docs/v2/RENAMES.md` (NEW) | `[ ]` `(M)` |

---

## Wave HY — Hygiene + test coverage (plan 10-hygiene-and-test-coverage)

> SystemPromptBuilder snapshots, clippy doc cleanup, flaky test fix,
> per-gate timeline render, smoke tests deferred from DC.

| Batch | Followup | Title | Touches | Status |
|-------|----------|-------|---------|--------|
| HY01 | 19 | SystemPromptBuilder snapshot tests (one per role) | `crates/roko-compose/tests/system_prompt_snapshots.rs` (NEW) | `[ ]` |
| HY02 | 19 | 6-layer-presence assertion across all roles | (same file) | `[ ]` |
| HY03 | 56 | Fill `# Errors`/`# Panics` in roko-core, roko-runtime | `crates/roko-{core,runtime}/src/**/*.rs` | `[ ]` |
| HY04 | 56 | Same for roko-gate, roko-compose | `crates/roko-{gate,compose}/src/**/*.rs` | `[ ]` |
| HY05 | 58 | Flaky-timeout audit + fix (mock time / CI scaling) | `crates/roko-agent/src/exec.rs` and siblings | `[ ]` |
| HY06 | 87 | Per-gate timeline rendered in TUI Gate tab | `crates/roko-cli/src/tui/{verdicts,views/dashboard_view}.rs` | `[ ]` |
| HY07 | 45 carry-over | Smoke test for learning loop | `crates/roko-cli/tests/smoke_learning_loop.rs` (NEW) | `[ ]` |
| HY08 | 45 carry-over | Smoke test for TUI event responsiveness | `crates/roko-cli/tests/smoke_tui.rs` (NEW) | `[ ]` |

---

## Wave RH — Runner hardening (plan 11-runner-hardening) `(M)`

> Bash-only fixes for the legacy TUI-parity runner. Outside the Rust
> runner's normal scope. See `manual-tracks/11-runner-hardening/CHECKLIST.md`.

| Step | Followup | Title | Touches | Status |
|------|----------|-------|---------|--------|
| RH01 | 27 | Postmortem of 2026-04-16 silent stop | `tmp/tui-parity/POSTMORTEM-20260416.md` (NEW) | `[ ]` `(M)` |
| RH02 | 28 | Env knobs `TUI_PARITY_MAX_BATCHES`, `TUI_PARITY_MAX_RETRIES` | `tmp/tui-parity/lib/common.sh` | `[ ]` `(M)` |
| RH03 | 28 | Trailer-on-exit trap into `status.tsv` | `tmp/tui-parity/lib/common.sh`, `run-tui-parity.sh` | `[ ]` `(M)` |
| RH04 | 27a | Log retention + `.gitignore` + cleanup script | `.gitignore`, `tmp/tui-parity/lib/cleanup-logs.sh` (NEW) | `[ ]` `(M)` |
| RH05 | — | Apply same fixes to `tmp/ux-followup-runner/` if present | `tmp/ux-followup-runner/` | `[ ]` `(M)` |

---

## Wave PH — Phase-2 vision (plan 12-phase2-vision) `(P)`

Parked. None of these run. They are listed here so a future planner can
discover them. See `manual-tracks/12-phase2-vision/CHECKLIST.md` for
entry conditions and design questions to resolve before promotion.

| Item | Followup | Title | Bucket |
|------|----------|-------|--------|
| PH01 | 49 | Roko-golem chain-witness | `(P)` |
| PH02 | 50 | Roko-chain primitives (write surface) | `(P)` |
| PH03 | 51 | Roko-dreams full cycle | `(P)` |
| PH04 | 52 | Full-Mori TUI features | `(P)` |
| PH05 | 53 | HTTP server Phase 2 (auth, multi-tenant) | `(P)` |
| PH06 | 54 | `roko-plugin` extensibility — see FG03 for audit | `(P)` |

---

## Verified-already-done (closed during audit, no batch needed)

These were marked open in `tmp/ux/ux-followup/` but a re-read of the
codebase on 2026-05-01 shows they are in fact done. Keeping them here
as audit history.

| Followup item | Why it's done |
|---------------|---------------|
| ~~35a — gate pipeline 4-of-7 unwired~~ | `crates/roko-cli/src/orchestrate.rs::run_gate_rung` (line ~17656) now dispatches all 7 canonical rungs through `roko_gate::rung_dispatch::run_rung` (`crates/roko-gate/src/rung_dispatch.rs:83+`). |
| ~~81 — snapshot migration framework~~ | `crates/roko-cli/src/snapshot_migrate.rs` exists with v0→v1→v2 upgrades and dispatch on `schema_version`. |

---

## Wave-to-plan map

| Wave | Plan file | Codex batches | Manual steps |
|------|-----------|---------------|--------------|
| M | `01-mirage-extraction-final.md` | 7 | 0 |
| AG | `02-aggregator-knowledge-pheromones.md` | 10 | 0 |
| DB | `03-dashboard-url-migration.md` | 0 | 6 |
| CH | `04-erc8004-chain-discovery.md` | 8 | 1 (CH08) |
| TU | `05-tui-event-parity-final.md` | 7 | 0 |
| MC | `06-mcp-coverage-audit.md` | 6 | 0 |
| FG | `07-phase2-feature-gating.md` | 4 | 0 |
| BP | `08-agent-backend-parity.md` | 11 | 0 |
| DC | `09-stale-docs-and-drift.md` | 0 | 8 |
| HY | `10-hygiene-and-test-coverage.md` | 8 | 0 |
| RH | `11-runner-hardening.md` | 0 | 5 |
| PH | `12-phase2-vision.md` | 0 | (parked) |
| **Total** | | **61** | **20** |

---

## How to use this tracker

1. **Reading**: open
   `tmp/ux/implementation-plans/<plan>.md` for the full context of a
   batch's wave. Each prompt under `tmp/runners/ux-impl/prompts/<id>.prompt.md`
   is the mechanical implementation of one row here.
2. **Running**: `bash tmp/runners/ux-impl/run.sh --group AG --pause` runs
   one wave with manual review between batches.
3. **Updating**: when a batch lands and verifies green on the runner's
   wave gate, change `[ ]` → `[x]` here. Include the run id in the row's
   notes if you want a forensics trail.
4. **Adding**: do **not** add new rows to this tracker without updating
   `batches.toml` and writing a `prompts/<id>.prompt.md`. The 1-1
   correspondence is the contract.

## Cross-cutting expectations

- A wave is "done" when every `[ ]` in that section is `[x]` and the
  wave gate (`cargo check --workspace` + `cargo clippy --workspace
  --no-deps -- -D warnings`) is green.
- The end-of-run gate runs `cargo test --workspace` against the merged
  branch. If a wave reports green but the test gate fails, the failing
  batch must be re-opened and a `[~]` mark applied.
- Manual `(M)` steps don't go through the runner; the agent owning the
  step ticks the row by hand and pastes evidence (PR link, command
  output) into the row's notes.
- Parked `(P)` items don't get ticked. Their entry conditions live in
  `manual-tracks/12-phase2-vision/CHECKLIST.md`.

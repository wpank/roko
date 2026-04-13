# Implementation Plans — tmp/ux/ Triage and Plan Index

> **Created**: 2026-05-01.  Re-audits the original `tmp/ux/` (architecture vision,
> circa 2026-04-14) and `tmp/ux/ux-followup/` (post-PR-13 audit, circa
> 2026-04-22) catalogues against the actual codebase as of `agent-refinements`
> branch.
>
> **Bottom line**: ~80 % of `tmp/ux/` is *already shipped*. The remaining ~20 %
> falls into 12 self-contained tracks in this folder. Each track is a single
> markdown file in `tmp/ux/implementation-plans/`, written so a fresh agent
> with no prior context can complete it.

---

## How to read this folder

| File | Track | Effort | Risk | Required for |
|------|-------|--------|------|--------------|
| `01-mirage-extraction-final.md` | Architecture Phase 3 | 3-5 days | Medium | Removing dead code in `apps/mirage-rs` |
| `02-aggregator-knowledge-pheromones.md` | Architecture Phase 2 (close) | 4-6 days | Medium | Dashboard parity for InsightBoard / pheromone views |
| `03-dashboard-url-migration.md` | Architecture Phase 2 (close) | 1-2 days | Low | Sam's nunchi-dashboard switching to roko-serve |
| `04-erc8004-chain-discovery.md` | Architecture Phase 3 | 5-8 days | Medium | True peer-to-peer agent discovery without roko-serve |
| `05-tui-event-parity-final.md` | Followup file 12 | 3-4 days | Low | "Polling is a bug" closeout (items 70-78) |
| `06-mcp-coverage-audit.md` | Followup item 34 | 2-3 days + per-crate days | Low | Honest "MCP wired" claim in CLAUDE.md |
| `07-phase2-feature-gating.md` | Followup items 32, 33 | 4-6 hours | Low | CI cost trim |
| `08-agent-backend-parity.md` | Followup items 36-40, 40a | 8-12 days | Medium | Cascade router actually safe to use |
| `09-stale-docs-and-drift.md` | Followup items 45-47, 64-67, 67a | 1-2 days | Low | Newcomer onboarding sanity |
| `10-hygiene-and-test-coverage.md` | Followup items 19, 56, 58, 87 | 4-6 days | Low | Reduce test/lint debt |
| `11-runner-hardening.md` | Followup items 27, 27a, 28 | 1-2 days | Low | Runner doesn't silently die again |
| `12-phase2-vision.md` | Followup items 49-54 | Multi-week, parked | High | Not now — parking doc |

Read in priority order: **03 → 02 → 05 → 11 → 09 → 10 → 06 → 07 → 08 → 01 → 04 → 12**.

The first three close the dashboard story (highest user-facing leverage).
The middle batch removes silent footguns. `01` and `04` are large structural
moves that can wait until Phase 2 has been validated end-to-end. `12` is a
parking lot for vision items that should not be scheduled before everything
above is green.

---

## Triage of every file in `tmp/ux/`

### Top-level architecture docs (`tmp/ux/00-06`)

| Doc | Status | Plan(s) covering remaining work |
|-----|--------|--------------------------------|
| `00-architecture-overview.md` | **Phase 1 + most of Phase 2 shipped.** `roko-agent-server` exists with all the documented features (`crates/roko-agent-server/`). The aggregator on `roko-serve` exists at `crates/roko-serve/src/routes/aggregator.rs`. Phase 3 (mirage cleanup, full chain discovery) is partial. | `01`, `02`, `03`, `04` |
| `01-agent-server-design.md` | **Implemented.** `AgentServer::builder()` in `crates/roko-agent-server/src/lib.rs` matches the spec — features `messaging`, `predictions`, `research`, `tasks`, plus always-on `health`/`capabilities`/`stats`/`logs`. Bearer auth and registration via `AgentCard` are wired. Heartbeat loop to `roko-serve` is wired. *Open work*: the `on_start` ERC-8004 chain push (Agent Card URI on `IdentityRegistry`) is stubbed (registration goes via `roko-serve` HTTP, not the chain). | `04` (chain-side only) |
| `02-mirage-extraction.md` | **Half done.** The `chain` Cargo feature plus the renamed `dashboard-api` flag (`apps/mirage-rs/Cargo.toml:88-112`) gate the REST surface. `cargo build -p mirage-rs --no-default-features --features binary` already builds a pure-EVM mirage. *Open*: the production default (`default = ["binary", "chain"]`) still lights up `dashboard-api`, the dashboard is still wired to mirage on `:8545`, no deprecation timeline is documented, and `ChainContext` + `http_api/` are still present. | `01` |
| `03-auth-and-discovery.md` | **HTTP auth done, chain discovery missing.** Bearer tokens implemented (`crates/roko-agent-server/src/auth/bearer.rs`). Discovery currently goes via roko-serve's HTTP registry (`/api/agents` aggregator), not via 8004 chain enumeration. The Agent Card has the right *shape* (`AgentCard { endpoints: { rest, websocket, ... }, ... }` in `registration.rs`) but is published to roko-serve, not to `IdentityRegistry`. | `04` |
| `04-dashboard-migration.md` | **Aggregator built; URL swap not yet performed on the production dashboard.** The current sibling repo `nunchi-dashboard` (Sam's) reads `MIRAGE_BASE = VITE_CHAIN_URL ?? http://127.0.0.1:8545` (`nunchi-dashboard/src/services/constants.ts:28`) and routes 90 % of REST through it. The local `demo/demo-app/` already talks to roko-serve at port 6677. | `03` |
| `05-build-phases.md` | **Phase 1 + most of Phase 2 shipped.** Phase 3 cleanup not started. | `01`, `02` |
| `06-open-questions.md` | Mostly resolved by the aggregator pattern (questions 5, 9). Questions 1 (loading states), 2 (add-an-agent flow), 3 (health viz), 4 (network-only user), 6 (config ownership), 7 (C-Factor aggregation), 8 (WS topology), 10 (8004 filtering) are dashboard UX or chain decisions and live in `03` and `04`. | `03`, `04` |

### Followup catalogue (`tmp/ux/ux-followup/`)

The catalogue's own headers claim **40 open items as of 2026-04-20**.  A
direct re-audit of `agent-refinements` head (commit window early May 2026)
shows additional silent closures: items **35a** (gate pipeline 7-rung
wiring) and **81** (snapshot migration framework) are now done in code even
though the catalogue still lists them as open. The remaining genuinely-open
items consolidate to **~33** entries, mapped below.

| Followup file | Item(s) genuinely open | Plan |
|---------------|-----------------------|------|
| `01-verified-p0-bugs.md` | none | — |
| `02-high-impact-quick-wins.md` | none | — |
| `03-non-batch-followups.md` | 19 (SystemPromptBuilder snapshot test) | `10` |
| `04-t9-t19-residuals.md` | 27, 27a, 28 (runner hardening) | `11` |
| `05-partially-wired-subsystems.md` | 32 (roko-dreams gate), 33 (chain/daimon gate), 34 (MCP audit) | `06`, `07` |
| `06-advanced-agent-backends.md` | 36, 37, 38, 39, 40, 40a | `08` |
| `07-spec-code-drift.md` | 45 (CLAUDE.md smoke tests), 46 (MORI-PARITY regen), 47 (bardo-backup banners) | `09` |
| `08-phase-2-vision.md` | 49, 50, 51, 52, 53, 54 (parked) | `12` |
| `09-hygiene-and-test-coverage.md` | 56 (clippy missing_*_doc), 58 (flaky tests), 60c (cascade e2e — overlaps `08`) | `10`, `08` |
| `10-stale-docs.md` | 64, 65, 66, 67, 67a | `09` |
| `11-execution-plan.md` | T23 carry-forward = items 27/27a/28; everything else under T24-T32 closed | `11` |
| `12-tui-event-parity.md` | 70, 71, 72, 73, 74, 76, 78 | `05` |
| `13-session-state-mgmt.md` | none (item 81 closed in `crates/roko-cli/src/snapshot_migrate.rs`) | — |
| `14-observability-gaps.md` | 87 (per-gate timeline) | `10` |
| `15-safety-and-learning-closure.md` | none | — |

Anything not listed here is either DONE or is a docstring update best
folded into `09`.

---

## Cross-cutting principles for every plan

These apply to every file in this folder. Each plan repeats the relevant
ones in context.

### Anti-patterns to avoid

1. **Don't re-introduce a "single backend assumption".** Sam's nunchi-dashboard,
   the demo-app, and the TUI must all keep working when one or more agent
   servers are unreachable. Any new fan-out call must degrade gracefully.
2. **Don't add new `mirage_rs::http_api/*` routes.** The `chain` and
   `dashboard-api` features are deprecating. Any new `/api/*` endpoint
   should land on `roko-serve` (preferably in `routes/aggregator.rs`).
3. **Don't proxy through roko-serve for agent-to-agent traffic.** roko-serve
   spawns agents and aggregates reads. Agents talk to each other directly
   via the Agent Card endpoints.
4. **Don't add timing-based assertions in tests.** Use `tokio::time::pause`
   or scale timeouts via `CI=true`. We have prior incidents (`exec.rs:504`).
5. **Don't introduce new `unwrap()`s in libraries**, especially in
   `crates/roko-core`, `crates/roko-runtime`, `crates/roko-gate`. Hot files:
   `system_prompt_builder.rs` and `routes/middleware.rs` were just cleaned
   to zero — keep them there.
6. **Don't poll `.roko/*` from a thread.** Use `notify` (already wired in
   `crates/roko-cli/src/tui/fs_watch.rs`). New TUI panels must subscribe.
7. **Don't write JSONL files without versioned headers if the consumer is
   long-lived.** See `crates/roko-cli/src/snapshot_migrate.rs` for the
   pattern.
8. **Don't update `.cursor/rules`, `~/.claude/`, `roko.toml`** as a
   side-effect of these tasks. All persistent config landed in PR #13's
   wake; touching them creates merge friction.
9. **Don't skip `cargo clippy --workspace --no-deps -- -D warnings`** before
   marking a track done. The workspace gate is `-D warnings`; per-crate
   `#[allow(clippy::missing_errors_doc)]` lists are also tracked (item 56).
10. **Don't restate code in comments.** Comments must explain *why* (intent,
    constraint, trade-off), not *what*. A reviewer will reject obvious
    narrative comments; CI does not enforce this but humans do.

### Required reads for every plan

Every track expects the runner to have read:

- `CLAUDE.md` — canonical project status and self-hosting workflow.
- `crates/roko-core/src/lib.rs` — workspace trait surface and the `Engram`
  type.
- `crates/roko-runtime/src/event_bus.rs` — `RokoEvent` variants.
- `tmp/ux/00-architecture-overview.md` — 5-layer model.

Plan-specific required reads are listed in each file.

### Cargo and tooling rules

- Always use `yarn` rather than `npm` in JS packages (user rule). Apply to
  `demo/demo-app` and any sibling dashboard work.
- `cargo build -p <crate> --all-features` for crate-scoped work; full
  workspace builds only when extraction or feature flags change.
- New deps go through `cargo add` to pull the latest version; do not pin to
  arbitrary versions.

### Testing rules

- Every public function added must have a doctest or unit test in the same
  crate.
- New HTTP routes get an integration test under `crates/<crate>/tests/`.
- New event variants on `RokoEvent` get a round-trip serde test.
- Snapshot tests use `insta` (already a dev-dep) — see
  `crates/roko-cli/tests/` for examples.
- Any cross-process test must clean up its tempdir on exit. Use
  `tempfile::TempDir`, not `/tmp/<hardcoded>`.

### "Done when" requirements

A plan is done when:

1. `cargo build --workspace` is clean.
2. `cargo clippy --workspace --no-deps -- -D warnings` is clean.
3. `cargo test --workspace` passes (modulo flaky tests already on the watch
   list).
4. The plan's specific "Done when" assertions all succeed.
5. `CLAUDE.md` and the plan-relevant `tmp/ux/ux-followup/*` evidence trail
   entry are updated to reflect the closure.

---

## Sequencing recommendation

Two parallelizable lanes:

```text
Lane A (dashboard story, 1-2 weeks)
  03 (URL swap)  →  02 (knowledge/pheromones)  →  01 (mirage cleanup)
                                              →  04 (chain discovery)

Lane B (hardening, ~1 week)
  11 (runner) ┐
  09 (docs)   ├→ 10 (hygiene + tests)  →  08 (backend parity)  →  06 (MCP audit)
  07 (gating) ┘
```

`05` (TUI event parity) is independent and can run in either lane. `12`
(Phase 2 vision) stays parked.

---

## Out of scope for this folder

- Restarting the `bardo→roko` rename (already done; `bardo-backup/` is the
  archive).
- Deleting `crates/roko-plugin` outright. Item 54 in
  `08-phase-2-vision.md` parks the audit; the plan in `12` only documents
  the audit inputs.
- Designing on-chain economics (staking curves, slashing). Out of band.
- Replacing `mirage-rs` itself with a different EVM simulator. The crate
  stays; only its bolted-on application state leaves.

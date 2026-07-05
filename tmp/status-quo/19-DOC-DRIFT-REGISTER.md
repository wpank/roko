# Doc Drift Register

**Verified**: 2026-07-08 against `main` @ `5852c93c05`. Every row below was line-checked
against current source. This register records claims in *maintained narrative docs*
(root `CLAUDE.md`, `README.md`, `docs/v2/*`, deploy/demo docs) that currently mislead
implementation planning and onboarding.

Format: **Claim (where) → Current truth (file:line) → Action**. P-tags mark severity.

---

## P0 — Actively misleading; block self-hosting or core mental model

| # | Claim (source) | Current truth (evidence) | Action |
|---|---|---|---|
| P0-1 | "Roko can self-host with `roko plan run plans/`" / "the main orchestration loop" (README:49, CLAUDE.md status table, README CLI ref:423) | Default `plan run` engine is Graph via Clap `#[arg(long, default_value = "graph")]` (`main.rs:1361`). The enum's own `#[default]` is `RunnerV2` (`main.rs:1301-1303`) but Clap's string default overrides it. Graph task execution runs `TaskExecutorCell::default()` with `dry_run: true` — synthetic output, no agent dispatch. The **real** live engine is Runner v2 (`crates/roko-cli/src/runner/event_loop.rs`). | Every real-work example must use `--engine runner-v2` until Graph dispatches real tasks. Update README, CLAUDE, all v2 CLI/execution docs. Add smoke test. |
| P0-2 | "Everything in roko is a **Signal**" / "1 noun (Signal) + 6 verb traits" (README:120-122, CLAUDE.md Architecture) | The core noun in code is **`Engram`**, not `Signal`. `roko-core/src/engram.rs:63` defines `pub struct Engram`. There is **no** `pub struct Signal` in `roko-core`. The sidecar even returns `engram_id` in README's own curl example (README:302). v2 docs adopt "Signal" as target vocabulary; code + `.roko` state use Engram. | Update README/CLAUDE architecture sections to state Engram is the code noun and Signal is the v2 target term. Fence v2 "Signal" docs as target-state. |
| P0-3 | `roko plan run plans/ --resume-plan` / `roko resume` reliably resumes snapshots (README:52, CLAUDE.md `--resume`) | `roko resume` hardcodes `engine: PlanEngine::Graph` (`main.rs:2699`) and delegates to the dry-run Graph path. It finds a snapshot, then hands it to an engine that does no real work. `--resume` (bare) is stale syntax; the flag is `--resume-plan` (`main.rs:1366`). | Do not document `roko resume` as reliable plan resume until it routes to Runner v2. Fix hardcoded engine or route resume to snapshot-capable engine. |
| P0-4 | `orchestrate.rs` is "the wired runtime heart" (CLAUDE.md: dozens of "Wired … orchestrate.rs" rows) | `orchestrate.rs` is **not compiled by default**. `lib.rs:94` gates it behind `#[cfg(feature = "legacy-orchestrate")]`; `Cargo.toml:16,108` defines the opt-in feature. The lib.rs comment (lib.rs:90-93) says it is a "legacy 21K-line engine … no longer compiled by default. The v2 event_loop.rs in runner/ is the sole execution engine." Dozens of CLAUDE.md status rows crediting orchestrate.rs are stale-by-default. | Rewrite CLAUDE.md status table: move every "Wired … orchestrate.rs" row to Runner v2 (`crates/roko-cli/src/runner/`), or mark legacy/opt-in. |

## P1 — Wrong counts, stale surfaces, safety framing

| # | Claim (source) | Current truth (evidence) | Action |
|---|---|---|---|
| P1-1 | "18 crates", "~177K/~200K LOC", "1,600+ tests", "19 builtin tools" (README:7,460; CLAUDE.md header + roko-std row) | 35 Cargo workspace members (31 crates + 3 apps + tests pkg); ~728K Rust LOC; ~9,968 `#[test]`/`#[tokio::test]` attrs (not a pass count). Builtin tools: `TOOL_COUNT = 37` (`roko-std/src/tool/builtin/mod.rs:44` — 16 std + 17 chain + 4 ISFR; note the array literal at :77-93 holds 17 std entries, comment says 16). | Regenerate counts from a workspace/tool script; state 35 members / 37 tools. Drop "1,600+ tests" or replace with attribute count + proof-tier note. |
| P1-2 | "Seven tabs, accessible via F1-F7" dashboard (README:72-83; CLAUDE.md F1-F7) | 10 TUI tabs. `header_bar.rs:326-334` maps F1-F9 (Dashboard, Plans, Agents, Git, Logs, Config, Inspect, Marketplace, Atelier); the 10th tab Learning is bound to `0` (`input.rs:587`). `status_bar.rs:142-157` enumerates all ten. | Replace F1-F7 seven-tab table with the current 10-tab surface (Dashboard, Plans, Agents, Git, Logs, Config, Inspect, Marketplace, Atelier, Learning). |
| P1-3 | "Safety falls back permissively when YAML missing" (CLAUDE.md safety-contracts row: "falls back to permissive default") | Fail-closed now. Missing/unloadable contract → `AgentContract::restricted` (deny-everything) per `contract.rs:88-89,133-140`; confirmation channel "always denies (fail-closed default)" (`authz.rs:224-225`). `permissive()` is retained only "for tests and adapter shims" (contract.rs:134-138). | Rewrite safety row to fail-closed for bundled/missing contracts. Keep advanced-hook / operator-carve-out gaps as the real remaining risk. |
| P1-4 | `roko neuro query/stats` is the knowledge CLI (README:224-227,430) | CLI surface migrated to `roko knowledge …` (CLAUDE.md CLI reference already uses `knowledge`; `roko neuro` examples in README are stale). | Replace all `roko neuro` examples with `roko knowledge`. |
| P1-5 | `roko serve --listen HOST:PORT` (docker/demo compose examples) | Current CLI flags are `--bind HOST` and `--port PORT` (README:245 already uses these). `--listen` is stale. | Fix compose files and any `--listen` docs to `--bind`/`--port`. |
| P1-6 | State snapshot lives at `.roko/state/executor.json` (CLAUDE.md) | Both exist: `executor.json` (legacy) **and** the unified checksummed `.roko/state/state-snapshot.json` (`runner/persist.rs:45-46,73,331`). The Runner-v2 canonical snapshot is `state-snapshot.json`. | Document `state-snapshot.json` as the current Runner-v2 snapshot; note `executor.json` compat file. |

## P2 — API/route/ops drift (mostly docs/v2/* + deploy docs)

| # | Claim (source) | Current truth (evidence) | Action |
|---|---|---|---|
| P2-1 | "~85 routes" (README:152,248; CLAUDE.md) | 288 raw `.route(` declarations under `crates/roko-serve/src` (272 by the tighter status-pack count; 337 workspace-wide incl. agent-server/relay/Mirage). "~85" understates by ~3x. | State route count as "~270 serve routes" from a generated manifest; "~85" is a soft undercount, not fatal, but should be corrected. |
| P2-2 | API docs list `/healthz`, `/readyz` (docs/v2/25-DEPLOYMENT, docs/v2-depth/20-deployment) | Current serve exposes `/health`, `/ready`, `/metrics`, `/api/health`. `healthz`/`readyz` are old/target-state. | Update deployment/API docs to `/health`, `/ready`, `/api/health`. |
| P2-3 | API auth defaults off; StateHub ring default 512 (docs/v2 API refs) | Code default enables serve auth; StateHub ring buffer default is 1024. | Update docs/config reference. |
| P2-4 | Terminal stream at `/api/terminal/sessions/{id}/stream` (docs/v2 API) | Actual stream is `/ws/terminal/{id}`. | Update API + frontend references. |
| P2-5 | Generic public `/webhook/{source}` (docs/v2 API) | Public routes are GitHub/Slack; generic webhook is authenticated. | Update API docs. |
| P2-6 | Docker/Railway/Fly "production-ready"; Fly configs agree (deploy/docker docs) | Root Docker/Railway/dev-compose assume a tracked root `roko.toml`; main compose uses stale `--listen`. Root `fly.toml` uses GHCR/port 3000/`/api/health`; CLI-generated Fly config uses Dockerfile/port 6677. | Pick one deploy source of truth; require clean-checkout build/compose smoke; reconcile Fly port/health. |
| P2-7 | Demo `plan run plans/` scripts are current product surface (demo/demo-resources) | React demo app is live; `demo/demo-web` and `tmp/demo-uis` are legacy/static/prototype; some demo scripts run the dry-run default `plan run`. | Label demo static assets; patch demo scripts to `--engine runner-v2` or mark historical. |

## P3 — Framing / consolidation debt (not a hard bug, but misleading)

| # | Claim (source) | Current truth | Action |
|---|---|---|---|
| P3-1 | VCG is fully live OR fully dead (CLAUDE.md "Partial") | Built, diagnostic/test-reachable, but production selection is effectively unreachable without warmed/updating bidders. | Use "built, test/diagnostic reachable, production-selection blocked." |
| P3-2 | Dreams have "no runtime trigger/cron" (CLAUDE.md dreams row) | Runtime triggers exist; the missing piece is v2 cron/delta/BusPulse trigger architecture, not all triggers. | Replace broad "no trigger" with edge-specific gap. |
| P3-3 | Gate verdicts are not first-class Signals (v2 verdict docs) | `.roko/signals.jsonl` has GateVerdict entries; the graph gate cell remains a stub. | Update v2 verdict docs; fix graph cell. |
| P3-4 | `DispatchPlan`/`RunLedger`/`GateStatus` absent (older audits) | Exist in fragments across core/runtime/gate/CLI/learn — consolidation debt, not absence. | Reframe as consolidation debt in foundation-contracts work. |
| P3-5 | v1 docs status labels ("scaffold/specified") read as current | Many pieces now exist; v1 status labels are stale. | Supersede v1 status with status-quo pack; add banners. |
| P3-6 | ACP advertises no image support / permission gates protect all tools | ACP has image-block conversion; capability reporting inconsistent; `request_permission` not universally enforced for builtin execution. | Make ACP capabilities truthful; wire production enforcement. |
| P3-7 | v2-depth INDEX counts / `12-connectivity` empty / extension unified-doc link | Tree has ~185 md files; `12-connectivity` has relay/connectivity docs; `08-extension-system` links to a missing `docs/unified/08-EXTENSION-SYSTEM.md`. | Regenerate v2-depth manifest; repair link; add stale-index banner. |
| P3-8 | Test counts imply CI proof | Many tests are env/feature/live-service gated; Playwright/Foundry outside CI; coverage ignores run failures. | Split proof tiers (see `74`, `25`). |

---

## Rule

Any roadmap item that depends on docs must first pass a source check against current code
or this register. Do **not** treat v1/v2/v2-depth status labels as current implementation
facts without verification. The dominant cross-cutting drift is P0-1 through P0-4: the
narrative docs still describe the legacy `orchestrate.rs` / default-`plan run` / "Signal"
world, while the live code is Runner v2 / `--engine runner-v2` / `Engram`.

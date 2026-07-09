# Command Example Drift Ledger

**Verified**: 2026-07-08 against `main` @ `5852c93c05`. This ledger captures command
snippets that look actionable but are unsafe, stale, target-state, or incomplete against
current code. Each replacement is backed by a file:line check.

## Replacement Rules

| Stale pattern | Current replacement | Why (evidence) |
|---|---|---|
| `roko plan run plans/` for real work | `roko plan run plans/ --engine runner-v2` | Default `plan run` engine is Graph (`main.rs:1361` `default_value="graph"`); Graph task execution is `TaskExecutorCell::default()` with `dry_run:true` — synthetic output, no agent dispatch. Runner v2 (`crates/roko-cli/src/runner/event_loop.rs`) is the live engine. |
| `cargo run -p roko-cli -- plan run plans/` for real work | `cargo run -p roko-cli -- plan run plans/ --engine runner-v2` | Same default-engine issue. |
| `roko plan run ... --resume .roko/state/executor.json` | `roko plan run ... --engine runner-v2 --resume-plan .roko/state/state-snapshot.json` after verifying Runner v2 snapshot compatibility | Flag is `--resume-plan` (`main.rs:1366`), not `--resume`. Canonical Runner-v2 snapshot is `.roko/state/state-snapshot.json` (`runner/persist.rs:45-46`); `executor.json` is a legacy compat file. |
| `roko resume` as reliable plan resume | Do not document as reliable until it routes to a snapshot-capable engine. | `roko resume` hardcodes `engine: PlanEngine::Graph` (`main.rs:2699`) → delegates to the dry-run Graph path, discarding the snapshot's value. |
| `roko neuro ...` | `roko knowledge ...` | CLI surface migrated. README:224-227,430 still use `roko neuro`; CLAUDE.md CLI reference already uses `knowledge`. |
| `roko serve --listen HOST:PORT` | `roko serve --bind HOST --port PORT` | Current CLI exposes `--bind`/`--port` (README:245). `--listen` survives in `docker/docker-compose.yml`. |
| `/healthz`, `/readyz` | `/health`, `/ready`, `/api/health` as appropriate | Current serve routes; `healthz`/`readyz` are old/target-state docs. |
| `F1-F7` dashboard tabs (seven tabs) | Current 10-tab TUI | `header_bar.rs:326-334` maps F1-F9; Learning is `0` (`input.rs:587`). Tabs: Dashboard, Plans, Agents, Git, Logs, Config, Inspect, Marketplace, Atelier, Learning. |
| `18 crates`, `19 builtin tools`, `~200K LOC`, `1,600+ tests` | Generated workspace/tool counts | 35 workspace members (31 crates + 3 apps + tests); `TOOL_COUNT = 37` (`roko-std/src/tool/builtin/mod.rs:44`); ~728K LOC; ~9,968 test attrs (not a pass count). |
| "Everything is a **Signal**" architecture prose | "Core noun is `Engram`; Signal is the v2 target term" | Code defines `pub struct Engram` (`roko-core/src/engram.rs:63`); no `struct Signal` in roko-core; sidecar returns `engram_id` (README:302). |
| "Railway/Docker production-ready" | "configured but blocked until clean-checkout build and smoke proofs pass" | Root `roko.toml` tracking gap, compose `--listen` flag, Fly port/health mismatch remain. |

## Priority Locations

| Location | Examples to fix | Status |
|---|---|---|
| `README.md` | Default `plan run` (:49), `--resume-plan` (:52), `roko neuro` (:224-227,430), F1-F7 (:72-84), Signal architecture (:120-143), 18-crate/200K/1,600-test counts (:7,460), `plan run` CLI-ref row (:423). | Maintained doc; rewrite first. |
| `CLAUDE.md` | Default `plan run`, `--resume`, F1-F7, safety permissive-fallback, 19 tools, 18 crates, `orchestrate.rs`-centered status table, Signal noun, `executor.json` state path. | Maintained contributor doc; rewrite first. |
| `docs/v2/CLI-REFERENCE.md` | `plan run` engine semantics + resume semantics. | Useful; needs current banners/patches. |
| `docs/v2/ARCHITECTURE-GUIDE.md` | Runtime counts, resume examples, builtin tool counts, engine framing. | Patch after architecture decisions linked. |
| `docs/v2/INTEGRATION-GUIDE.md` | `plan run`, `--resume`, F1-F7. | Patch with safe examples. |
| `docs/v2/04-EXECUTION.md` | `plan run` and resume as live-engine examples. | Patch after engine decision. |
| `docs/v2/20-SURFACES.md`, `docs/v2/28-ROADMAP.md` | F1-F7 seven-tab, 19-tool counts. | Patch with current TUI/tool status. |
| `docs/v2/25-DEPLOYMENT.md`, `docs/v2-depth/20-deployment/*` | `/healthz`, `/readyz`, target deployment shapes. | Label target-state or update to current routes. |
| `docker/docker-compose.yml` | `roko serve --listen 0.0.0.0:9092`. | Code/config fix required, not just docs. |
| `docker/README.md`, `docker/RAILWAY.md`, `deploy/README.md` | Clean-checkout `roko.toml`, compose flag, Fly/Railway port+health. | Ops fix + doc update. |
| `demo/demo-resources/**` | Default `roko plan run plans/` examples. | Patch to `--engine runner-v2` or label historical. |
| `tmp/research3`, `tmp/dogfood`, `tmp/unified*`, `tmp/runners`, `tmp/archive`, `tmp/doc-convergence/**` | 18-crate/plan-run/resume/F1-F7/Signal historical claims. | Archive/banner; do not mass-rewrite unless promoted. |

## Checklist

- [ ] Add a docs lint for `roko plan run` examples that do not mention engine semantics.
- [ ] Add a docs lint banning `roko neuro`, `/healthz`, `/readyz`, `--listen`, `--resume ` (bare), and "F1-F7" outside historical/archive folders.
- [ ] Add a docs lint flagging "Everything is a Signal" / "1 noun (Signal)" outside v2 target-state docs.
- [ ] Generate workspace/tool counts (35 members, 37 tools) before updating README/CLAUDE.
- [ ] Add command smoke tests for every example kept in maintained docs.
- [ ] Move historical tmp examples behind archive banners instead of polishing them as current docs.

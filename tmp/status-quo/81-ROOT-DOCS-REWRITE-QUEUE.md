# Root Docs Rewrite Queue

**Verified**: 2026-07-08 against `main` @ `5852c93c05`. This queue covers maintained
user/contributor docs that should be rewritten from the status-pack **before** any
v1/v2/tmp source is copied forward. Rewrite in priority order; each row lists the concrete
drifts (with file:line) and the status-pack source files to draw from.

## Cross-Cutting Drift (fix in every doc)

Five stale threads run through all maintained narrative docs. The navigation layer must
capture these; each doc rewrite must scrub them:

1. **Engine**: default `roko plan run` = Graph (dry-run); real engine is Runner v2 (`--engine runner-v2`). `main.rs:1361`, `runner/event_loop.rs`.
2. **Noun**: code noun is `Engram` (`roko-core/src/engram.rs:63`), not `Signal`. Signal is v2 target vocabulary only.
3. **orchestrate.rs**: legacy, `#[cfg(feature = "legacy-orchestrate")]`, not compiled by default (`lib.rs:94`). Runner v2 is the live path.
4. **Counts**: 35 workspace members / 37 builtin tools / ~728K LOC — not 18 crates / 19 tools / ~200K LOC / 1,600+ tests.
5. **Surfaces**: 10 TUI tabs (F1-F9 + `0`), not 7 (F1-F7); safety fails closed, not permissive; `roko knowledge` not `roko neuro`; `--bind/--port` not `--listen`; `/health`+`/ready` not `/healthz`+`/readyz`.

## Rewrite Priority

| Doc | Current drift (evidence) | Rewrite source |
|---|---|---|
| `README.md` | 18 crates / ~200K LOC / 1,600+ tests (:7,460); teaches default `roko plan run plans/` (:49) and CLI-ref row (:423); `--resume-plan` framed as reliable (:52); `roko neuro` (:224-227,430); seven F1-F7 tabs (:72-84); "Everything is a Signal" architecture (:120-143); "~85 routes" (:152,248). Note the sidecar curl already emits `engram_id` (:302) — self-contradicting. | `01`, `12`, `19`, `37`, `45`, `62`, `73`, `74`, `80`, `82`. |
| `CLAUDE.md` | Dated 2026-04-20; 18 crates / ~177K LOC; status table centers `orchestrate.rs` (legacy, feature-gated at `lib.rs:94`); `--resume` (bare, stale); "safety falls back to permissive default" (now `restricted`/fail-closed, `contract.rs:88-89,133-140`); F1-F7; 19 builtin tools (actual `TOOL_COUNT=37`); "1 noun (Signal)" (code = `Engram`); `.roko/state/executor.json` (Runner-v2 canonical is `state-snapshot.json`, `persist.rs:45-46`). | `01`, `14`, `18`, `19`, `31`, `36`, `37`, `38`, `43`, `75`, `82`. |
| `docs/v2/CLI-REFERENCE.md` | Default `plan run` examples omit engine semantics; resume/default language unsafe. | `19`, `37`, `62`, `73`, `81`, `82`. |
| `docs/v2/ARCHITECTURE-GUIDE.md` | 18-crate/19-tool counts, old resume syntax, over-confident runtime framing, Signal-noun framing. | `13`, `18`, `19`, `30`, `31`, `36`, `54`, `76`, `82`. |
| `docs/v2/INTEGRATION-GUIDE.md` | `roko plan run`/`--resume` examples, F1-F7 TUI language. | `19`, `37`, `43`, `62`, `64`, `82`. |
| `docs/v2/04-EXECUTION.md` | Treats `plan run` + resume as live engine without Graph/Runner split. | `01`, `19`, `31`, `36`, `37`, `73`. |
| `docs/v2/20-SURFACES.md` | Seven TUI tabs; old surface framing. | `19`, `43`, `45`, `59`, `66`. |
| `docs/v2/25-DEPLOYMENT.md` | `/healthz`/`/readyz`; target deployment shapes not matching ops assets. | `19`, `77`, `09`, `58`. |
| `docker/README.md`, `docker/RAILWAY.md`, `deploy/README.md` | Clean-checkout `roko.toml`, compose `--listen` flag, Fly/Railway port+health mismatch. | `19`, `77`, `61`, `75`, `82`. |
| `demo/demo-resources/**/README.md` | Some scripts tell users to run default `roko plan run plans/`. | `19`, `73`, `82`. |

## Rewrite Rules

- Root docs should say current code has **35 workspace members** (31 crates + 3 apps + 1 tests package) and **37 builtin tools**, unless a generated workspace/tool-count script says otherwise. Drop "1,600+ tests"; if a test figure is used, cite the ~9,968 attribute count with a proof-tier caveat.
- Any real plan-execution example must use `--engine runner-v2` until default Graph execution dispatches real tasks.
- Any Graph example must be labeled loader/topology/gate-cell proof unless it dispatches real plan tasks.
- Architecture prose must call the core noun `Engram`; mention `Signal` only as the v2 target term with a banner.
- `orchestrate.rs` must be described as legacy/opt-in (`legacy-orchestrate` feature); credit runtime facts to `crates/roko-cli/src/runner/`.
- `roko neuro` examples must become `roko knowledge`.
- Dashboard docs must use the current 10-tab surface (Dashboard, Plans, Agents, Git, Logs, Config, Inspect, Marketplace, Atelier, Learning; keys F1-F9 + `0`), not F1-F7.
- Safety prose must say fail-closed (`restricted`/deny-everything) on missing/unloadable contracts; permissive is test/adapter-only.
- Deployment docs must distinguish top-level `/health`/`/ready`/`/metrics`, `/api/health`, worker health, and Mirage relay health body semantics; use `--bind`/`--port` not `--listen`.
- Serve route count should come from a generated manifest (~270 serve routes), not "~85".
- Strategic research/deck docs must cite `78` and cannot set current product claims without proof.

## Checklist

- [ ] Rewrite `README.md` from the status-pack navigation layer (scrub the 5 cross-cutting threads).
- [ ] Rewrite `CLAUDE.md` as contributor runtime facts (Runner v2 status table), not historical self-assessment.
- [ ] Patch v2 CLI/execution/integration docs with engine semantics + safe command examples.
- [ ] Patch v2 surfaces docs with current 10-tab TUI / API / frontend / ACP status.
- [ ] Patch deployment docs with clean-checkout, compose, Fly/Railway, Docker health, and release gates.
- [ ] Add historical/source banners to docs that are not maintained current docs.
- [ ] Add the docs-lint rules from `82` to CI so the drift cannot silently return.

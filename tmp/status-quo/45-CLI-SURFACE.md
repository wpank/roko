# roko-cli — Command Surface

> Status-quo audit · re-verified 2026-07-08 against git HEAD `5852c93c0` · sources: `crates/roko-cli/src/main.rs` (5,085 LOC) + `commands/` (28 modules) + `chat.rs`/`unified.rs`/`unified/`; docs/v2/CLI-REFERENCE.md (last touched 2026-05-05), docs/v1/12-interfaces/00–03, docs/v2-depth/16-surfaces/02, CLAUDE.md, `.roko/GAPS.md`; `tmp/tmp-feedback/2/` bug reports (05, 13, 26, 27); 27 integration test files. Census: **45 top-level subcommands** (clap enum `Command`, main.rs:324–897), ~155 leaf commands, 4 bare invocation modes.
>
> **Re-verify note (2026-07-08):** all P0 claims below re-confirmed at cited line numbers on HEAD `5852c93c0`. `plan run` engine still defaults to `graph` (main.rs:1361); `roko resume` still hard-codes `PlanEngine::Graph` (main.rs:2699) with the graph path warning+ignoring the snapshot (plan.rs:258–264) → **resume remains broken**. Three cross-surface CLI bugs from `tmp/tmp-feedback/2/` are still **OPEN**: `/search` sends Perplexity a batch `{"queries":[…]}` body (roko-agent/perplexity/search.rs:150) that the API rejects 400; ACP `/plan-resume` still emits `--resume` not `--resume-plan` (roko-acp/bridge_events.rs:3617) so ACP resume restarts from scratch; raw `eprintln!` is used for user-facing output in **299 calls across 29 CLI files** (up from the ~147 the report estimated) while `output_format.rs` sits mostly unused. Partial fix landed: `--context <PATH>` now exists on `do` (main.rs:375) and `plan generate` (main.rs:1396), closing part of tmp-feedback/05 Gap 1.

Status vocab: ✅ works end-to-end · 🔌 built but not wired · 🟡 partial/stub inside · ❌ registered but unimplemented · 🕰️ legacy/design-only.

## Summary

The CLI is far larger than any doc admits: 45 top-level subcommands vs ~24 in CLAUDE.md and ~33 in docs/v2/CLI-REFERENCE.md. Almost every command has a real handler — there are no empty `todo!()` stubs at the clap layer. The single biggest problem is **`roko plan run`'s default engine**: `--engine` defaults to `graph` (main.rs:1361), and the Graph Engine path maps every task to `TaskExecutorCell`, whose live dispatch is **not implemented** — it logs "live dispatch not yet implemented; using dry-run fallback" (roko-graph/src/cells/task_executor.rs:84–86, registered at engine.rs:356). So the flagship self-hosting command, run with defaults, does not execute agents; real execution requires `--engine runner-v2`. Git history shows this flip-flopped: commit `9423998a7 "fix: make runner-v2 the default plan engine"` was later overridden back to graph (Wave-5 "Engine as Default", per GAPS.md Task 102). `roko resume` is worse: it hard-codes `engine: Graph` while passing a snapshot (main.rs:2697–2709), which the graph path explicitly ignores (plan.rs:260–264) — resume-by-sugar is effectively broken. Meanwhile the legacy story is clean: `legacy-orchestrate` (orchestrate.rs run path) is **off by default** (Cargo.toml:15–16); `roko run`/`roko do`/`roko develop` all route through the v2 WorkflowEngine (do_cmd.rs:140–143, util.rs:297). Docs are 2 generations behind: v2 CLI-REFERENCE omits 12+ live commands and documents `plan run` with runner-v2 semantics as the default; CLAUDE.md still describes `roko run` as the universal-loop entry and a `roko chat` command that doesn't exist (it's bare `roko` unified chat + `roko agent chat`).

For the condensed command-by-command migration ledger, see [62-CLI-COMMAND-LEDGER.md](62-CLI-COMMAND-LEDGER.md).

## Command census

Bare invocation modes (dispatch at main.rs:2302–2319):

| Invocation | Handler | Status |
|---|---|---|
| `roko` (TTY, no subcommand) | `unified::cmd_unified_chat` — auth auto-detect (Claude CLI→API key), in-process `ChatAgentSession`, optional background serve (`serve.auto_start`, `--no-serve`) | ✅ unified.rs:28 |
| `roko "<prompt>"` | `unified::cmd_oneshot_inline` — one-shot via ChatAgentSession (tools+MCP+safety; deliberately not WorkflowEngine) | ✅ unified.rs:136 |
| `echo x \| roko` | `commands::util::cmd_pipe` | ✅ util.rs:63 |
| `roko --headless` | `commands::util::cmd_headless` | ✅ util.rs:88 |

Global flags (main.rs:235–310): `--config --role --model --repo --resume --effort --json --log-format --quiet -v/--verbose --no-replan --skip-validate --headless --color --timing --no-serve` + env fallbacks `ROKO_MODEL/EFFORT/ROLE/QUIET/LOG_FORMAT` (main.rs:2895–2953).

Top-level tree (clap enum `Command`, main.rs:313–889; handler cited):

| Command | Subcommands / key flags | Handler | Status |
|---|---|---|---|
| `init` | `[path] --cloud --profile --demo` | util.rs:95 | ✅ |
| `do` | `--plan --complexity --dry-run --yes --ghost --compare --continue [id] --no-cascade --context --provider` | do_cmd.rs:14 → ScopeResolver + WorkflowEngine (do_cmd.rs:65,143) | ✅ v2 path |
| `develop` | `--dry-run --yes --continue --provider` | develop.rs:14 (plan-first: generate→approve→execute) | ✅ v2 path |
| `run` | `--serve --share --provider --max-retries` | **routes to `cmd_do` unless --serve/--share/--max-retries** (main.rs:2340–2358); else util.rs:232 → WorkflowEngine (util.rs:297) | ✅ v2 path |
| `status` | `--quick --cfactor --surfaces` | util.rs `cmd_status` (re-exported commands/status.rs:3); `--surfaces` → surface_inventory.rs:131 | ✅ |
| `show` | `--live --follow --serve-url [subject: costs/agents/knowledge/plans/learning/history/<id>]` | show.rs (1,102 LOC); `--live` → TUI | ✅ |
| `doctor` | `--serve-url` | util.rs:1071 | ✅ |
| `setup` | `--yes` | setup.rs (interactive provider wizard) | ✅ |
| `layer-check` | — | `roko_cli::layer_check::run_layer_check` (lib.rs:86) | ✅ |
| `plan` | `list show create validate run generate regenerate` | plan.rs:23 | 🟡 (run — see below) |
| `plan run` | `--engine graph\|runner-v2 (default: graph) --resume-plan --approval --max-retries --max-tasks --dry-run --fresh --force-resume` | plan.rs:220; graph → `cmd_plan_run_engine` plan.rs:1567; runner-v2 → plan.rs:270+ | 🟡 **graph default = dry-run fallback** (task_executor.rs:84–86); ignores `--resume-plan` (warn, plan.rs:260) and silently drops `--approval/--max-retries/--max-tasks/--fresh/--force-resume`. `--engine runner-v2` = ✅ full executor (lock, fresh-archive, preflight, snapshot, approval TUI) |
| `prd` | `idea list status plan consolidate draft{new edit promote --auto-execute list}` | prd.rs (962 LOC) | ✅ |
| `agent` | `create delete list start stop status serve chat{--agent --serve-url --provider}` | commands/agent.rs:6 → agent_serve.rs:630 | ✅ (delete = 8-step shutdown, agent_serve.rs:1299) |
| `research` | `topic --deep, enhance-prd, enhance-plan, enhance-tasks, analyze, list, search --domains --recency` | research.rs (888 LOC) | ✅ (needs Perplexity key) |
| `think` | question (read-only research) | think.rs | ✅ |
| `note` | `--tag` (no-LLM capture) | note.rs | ✅ |
| `tune` | `routing gates budget model <name>` | tune.rs:140 | ✅ (overlaps `learn tune`) |
| `knowledge` | `query stats gc backup --top-n restore --min-confidence sync archive --older-than` + `dream{run report schedule journal archive}` + `custody{list show verify}` | knowledge.rs (1,098 LOC); dispatches to internal `NeuroCmd` (main.rs:1625) | ✅ |
| `learn` | `all route experiments efficiency episodes tune [gates\|routing\|budget]` | learn.rs (768 LOC) | ✅ |
| `job` | `list create match show execute cancel` | job.rs:433 (`match`/`execute` via serve) | ✅ |
| `bench` | `demo --real` · `swe --dataset --agent-mode gold\|prediction-file\|command …` | bench.rs → roko_cli::bench | ✅ |
| `demo` | `setup warm` | demo_cmd.rs (main.rs:2458–2474) | ✅ |
| `config` | `init(=wizard) show path doctor edit set set-secret check-secrets validate migrate export` + `providers{list health test available} models{list route} subscriptions{list add remove enable disable} events experiments{model create show list} plugins{list install remove audit} secrets{set get list rotate} mcp{list test add}` — 38 leaves | config_cmd.rs (2,864 LOC), experiment.rs, secrets.rs | ✅ |
| `index` | `build rebuild search --strategy stats` | util.rs:1251 | ✅ (auto-rebuild after plan/prd/research, main.rs:2426,2432,2439) |
| `graph` | `run validate show <toml>` | graph.rs:47 → roko-graph GraphEngine | ✅ (cells may be Passthrough stubs per GAPS.md) |
| `isfr` | `start --poll-interval, status, sources` | isfr.rs:51 → roko-chain ISFRKeeper | 🟡 mock sources default (isfr.rs:98–101); relay "Phase 2" (isfr.rs:111–113) |
| `feed` | `list, status <id>` | feed.rs:26 — HTTP client to serve `/api/feeds/runtime` | ✅ (requires running serve) |
| `dev` | `--no-frontend` (serve + demo frontend) | dev.rs | ✅ |
| `up` | serve + all `[[agents]]` | server.rs `cmd_up` | ✅ |
| `serve` | `--bind --port --tui --enable-terminal` | main.rs:2508–2571 → roko-serve ServerBuilder; `--tui` embeds dashboard on StateHub | ✅ |
| `acp` | editor-integration stdio server (early-exits before tracing, main.rs:2020–2049) | roko_acp | ✅ |
| `daemon` | `start --foreground stop status logs -f reload restart install uninstall` | server.rs `cmd_daemon` | ✅ |
| `deploy` | `railway --with-mirage --workers --unsafe-public, fly, docker --registry` | server.rs `cmd_deploy` | ✅ (external creds required) |
| `worker` | `--port` | roko_cli::worker::run_worker | ✅ |
| `dashboard` | `--page --list-pages --text --high-contrast --reduced-motion` | dashboard.rs:7 → tui `App` | ✅ (TUI audited separately) |
| `login` / `logout` / `whoami` | `--api-key --check --dashboard-url` (Privy browser or API key) | commands/auth.rs | ✅ |
| `vision-loop` | `<file> --goal --url --max-iter --target-score …` | vision_loop/ | ✅ wired (needs vision model + reachable URL) |
| `resume` | `[run_id]` — sugar for plan run --resume-plan | main.rs:2650–2709 | ❌ **broken**: hard-codes `engine: Graph` (main.rs:2699) which discards the snapshot |
| `replay` | `<hash> --forensic --as-of --format` | util.rs:1114 | ✅ |
| `history` | `[session-id]` chat session summaries | main.rs:2718–2775 → chat_history | ✅ |
| `inject` | `<session> <payload> --kind` | util.rs `cmd_inject` | ✅ |
| `completions` | `bash zsh fish` | util.rs:1420 | ✅ |
| `new` | 9 types: `gate scorer router policy substrate composer domain template event-source` | scaffold.rs:21–46 | ✅ (no `probe` type from design) |
| `explain` | `<topic> --depth 1..3` (≥8 topics; `explain topics` lists) | explain.rs (tests: explain.rs:438–441) | ✅ |

Routes into excluded subsystems: **TUI** ← `dashboard`, `serve --tui`, `show --live`, `plan run --approval` (runner-v2 only, plan.rs:7). **orchestrate.rs** ← only behind non-default `legacy-orchestrate` feature (run.rs:22,1481,1510; lib.rs:94).

## Doc drift (code vs docs/v2/CLI-REFERENCE.md vs CLAUDE.md)

| Command | In code | v2 CLI-REFERENCE (2026-05-05) | CLAUDE.md (2026-04-20) |
|---|---|---|---|
| `do` | ✅ primary intent verb | ✅ documented ("preferred entry point") | ❌ absent |
| `develop` | ✅ | ❌ absent | ❌ absent |
| `show` / `setup` / `think` / `note` / `history` | ✅ | ❌ absent | ❌ absent |
| `tune` (top-level) / `demo` / `dev` | ✅ | ❌ absent | ❌ (`tune` only as `learn tune`) |
| `graph` / `isfr` / `feed` | ✅ | ❌ absent | ❌ absent |
| `up` / `acp` / `login` / `logout` / `whoami` / `vision-loop` / `resume` / `layer-check` / `bench` | ✅ | ✅ all documented | ❌ absent (except implied) |
| `plan run --engine` (graph default, dry-run fallback) | ✅ flag exists | ❌ not mentioned; §575–641 describes runner-v2 semantics (resume/approval/replan) as default behavior | ❌ describes orchestrate.rs as "the main orchestration loop" (🕰️ feature-gated off) |
| `plan run --max-tasks/--force-resume` | ✅ | ❌ | ❌ |
| `status --quick/--surfaces` | ✅ | ❌ | ❌ |
| `config doctor/export/mcp` | ✅ | ❌ | ❌ (`config` list partial) |
| `agent delete` | ✅ | ✅ | ❌ |
| `roko chat` (top-level) | ❌ never existed — bare `roko` + `agent chat` | (not claimed) | 🕰️ claims "`roko chat` CLI — Wired — chat.rs" |
| `prd draft promote --auto-execute`, `research search`, `knowledge sync/backup/restore`, `job match`, `bench swe` | ✅ | ✅ mostly | partial |
| v1/v2-depth design verbs `ask watch inspect connect plugin orchestrate neuro episode import mesh provider repl dream` (docs/v1/12-interfaces/01 §55–607; v2-depth/16-surfaces/02 §36–69) | ❌ as named | — | — | (folded: neuro→`knowledge`, dream→`knowledge dream`, provider→`config providers`, plugin→`config plugins`, repl→bare `roko`, ask→`do`/`think`, watch→`show --follow`/`dashboard`, inspect→`show <id>`/`replay`; `connect`/`import`/`mesh`/`orchestrate` dropped) |

Internal doc rot: main.rs:4–8 module comment still lists `dream, neuro, subscription, event-sources, experiment` as top-level subcommands (all moved); `explain` content references non-existent `roko neuro search` (explain.rs:96).

## Cross-surface CLI/ACP bugs (from tmp/tmp-feedback/2 — re-verified 2026-07-08, all OPEN)

These are navigation-layer bugs that reach the CLI but originate in sibling crates. All confirmed against HEAD `5852c93c0`.

| Bug | Report | Status | Evidence | Impact |
|---|---|---|---|---|
| `/search` (and `roko research search`) sends wrong Perplexity body | 13-SEARCH-COMMAND-BROKEN | ❌ **open, P0** | `roko-agent/src/perplexity/search.rs:150` still builds `{"queries":[…]}`; `MAX_BATCH_SIZE=5`/`TooManyQueries` (search.rs:18,36) intact; tests mock the poster (search.rs:377+) so they never hit the real 400 | Every real `/search` / `research search` returns HTTP 400 "query is required" — Perplexity has no batch endpoint |
| ACP `/plan-resume` uses wrong flag | 26-SLASH-COMMAND-BUGS #1 | ❌ **open, P0** | `roko-acp/src/bridge_events.rs:3617` pushes `--resume` but CLI flag is `--resume-plan` (main.rs plan run) | ACP resume silently restarts plan from scratch, losing progress |
| Raw `eprintln!` for user output | 27-RAW-EPRINTLN | 🟡 **open, P2** | 299 `eprintln!` across 29 files under `crates/roko-cli/src/` (grep count 2026-07-08); `output_format.rs` primitives used mainly by `run.rs`/inline path | No color/spinner, tracing noise mixed with output, `--quiet` not honored in prd/research/develop |
| `--context <PATH>` for context injection | 05-CLI-WORKFLOW-GAPS Gap 1 | 🟡 **partial** | Now present on `do` (main.rs:375) + `plan generate` (main.rs:1396); NOT on `develop`; no ACP `/context` slash command | Partial: file/dir context works on two verbs; develop + ACP still gapped |

Note: the ACP `/plan-run` model-passthrough gap and `/develop` missing-slash-command gap (26-SLASH-COMMAND-BUGS #2, #4) live in the ACP surface (roko-acp), not roko-cli, and are catalogued there — cross-referenced here because they change what the CLI receives.

## Current state table

| Component | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| One binary, 6 invocation modes | v2-depth/16-surfaces/02 §2 | bare chat / one-shot / pipe / headless / daemon / serve all present (`repl` = bare mode) | ✅ | main.rs:2302–2319 |
| `roko do` classified WorkflowEngine entry | v2 CLI-REF §268 | ScopeResolver → simple/planned/architectural; `--compare` cascade preview | ✅ | do_cmd.rs:14,65,105,143 |
| `plan run` Graph Engine as default | GAPS.md Task 102 "Engine as Default" | default `graph` but TaskExecutorCell = dry-run fallback; flags dropped | 🟡 | main.rs:1361; plan.rs:258–267; task_executor.rs:84–86 |
| `plan run --engine runner-v2` | tmp/MASTER-TASKS Wave 5 | full streaming executor: lock, fresh, preflight, snapshot copy, metrics, approval TUI | ✅ | plan.rs:270–460 |
| Legacy orchestrate.rs run path | CLAUDE.md ("universal loop") | compiled only with `legacy-orchestrate` (non-default); v2 `run_once` replaces it | 🕰️ | Cargo.toml:14–16; run.rs:1481 |
| `legacy-runner-v2` cargo feature | GAPS.md Task 102 | declared + default, but gates **only** tests/phase0_wiring.rs — plan.rs has no cfg; engine choice is runtime-only | 🟡 gate is vestigial | Cargo.toml:15,20; phase0_wiring.rs:12; plan.rs (no cfg hits) |
| `roko resume` sugar | v2 CLI-REF §2086 | delegates to graph engine which ignores snapshots | ❌ | main.rs:2697–2709 |
| Progressive `explain` (3 depths) | v1/12-interfaces/03; v2-depth §5 | levels 1–3 via `--depth`, ≥8 topics, unknown-topic help | ✅ | explain.rs:352–371 |
| Error-as-teacher (`TeachingError` struct) | v2-depth §5 | ad-hoc `error_hint()` pattern-match + weekly deprecation hints — no structured TeachingError | 🟡 | main.rs:2194–2250; hints.rs:12 |
| Scaffolders (`roko new`) | v1/12-interfaces/02 | 9 types incl. composer/template/event-source; `probe` missing | ✅ (−probe) | scaffold.rs:21–46 |
| Semantic exit codes 10–15 | v2-depth §8 | only 0/1/2 | ❌ | main.rs:77–82 |
| Slash commands / diff-first hunk review in CLI shell | v2-depth §3, §7 | not in CLI layer (inline chat has its own primitives; per-hunk approval absent) | ❌ | inline/ modules |
| Chat REPL backends | CLAUDE.md, v2 CLI-REF §1005 | `agent chat`: sidecar (agents.json lookup) → serve fallback → per-message retry; `--provider anthropic_api\|openai_compat` direct; bare `roko`: in-process ChatAgentSession | ✅ | chat.rs:54–61,76–93; commands/agent.rs:43–49 |
| 9-verb model (`ask/watch/inspect/connect…`) | v1 00-cli-overview §32–44 | folded into do/show/replay/config; verbs not exposed | 🕰️ superseded | census above |
| Command-group help | v2 CLI-REF §TOC | `after_long_help` groups incl. undocumented `up/acp/bench/think/note` | ✅ | main.rs:219–233 |
| CLI tests | — | 27 integration files (e2e, smoke, cli_fallback, doctor, job_cli, plan_validate/validation, prd_pipeline×2, resume_cycle_e2e, tui_tabs, e2e_self_host…); unit tests in plan.rs:1717+, explain.rs, scaffold.rs, unified.rs | ✅ | crates/roko-cli/tests/ |

## V2-aligned

- `do`/`develop`/`run` all dispatch through WorkflowEngine + ScopeResolver (do_cmd.rs:143; util.rs:297) — no default-build path touches legacy orchestrate.rs.
- Unified bare-`roko` chat with auth auto-detect, in-process dispatch, opt-in background serve (unified.rs:28–121).
- `plan`/`prd`/`research` trigger automatic index rebuild (main.rs:2423–2440); plans resolve `.roko/plans/` canonically with `./plans/` fallback (main.rs:2859–2880).
- `graph run/validate/show` exposes the Graph Engine directly (graph.rs) — the v2 "CLI as Graph interpreter" story has a real entry point.
- Progressive explain, scaffolders, completions, layered config with per-field source tags (`config show`), profile-aware secrets, MCP config management.

## Old paradigm & tech debt

- 🕰️ `legacy-orchestrate` feature + ~40 cfg blocks in run.rs await deletion (run.rs:1999–2001 notes v2 supersedes it).
- 🟡 `legacy-runner-v2` feature is default-on but gates only one test file — either gate the plan.rs runner-v2 arm or delete the feature (GAPS.md Task 102 text is stale: no cfg in plan.rs today).
- Duplication: top-level `tune` (tune.rs) vs `learn tune` (learn.rs) do overlapping gate/routing/budget tuning; `config set-secret` vs `config secrets set`.
- `roko run` silently becomes `roko do` unless --serve/--share/--max-retries (main.rs:2340) — surprising semantics, undocumented.
- Stale module docs (main.rs:4–8), stale explain hint (`roko neuro search`, explain.rs:96), deprecation-hint infra exists (hints.rs) but the run→do hint isn't invoked from the run path.
- Internal `NeuroCmd`/`DreamCmdLegacy` mirror enums kept for dispatch (main.rs:1622–1664).

## Not implemented

- TaskExecutorCell live dispatch — default `plan run` cannot execute agents (task_executor.rs:84–86; GAPS.md Task 101).
- Graph-engine snapshot/resume, parallel execution, conditional edges (GAPS.md Tasks 101–103); `--approval/--max-retries/--max-tasks/--fresh/--force-resume` on graph path.
- `roko resume` (broken via graph default), semantic exit codes 10–15, TeachingError struct, slash-command palette + per-hunk diff review in CLI shell, `roko new probe`, design verbs `ask/watch/inspect/connect/import/mesh/orchestrate` as named commands.

## Migration checklist

- [ ] **[P0]** Make `plan run` default engine actually execute: either implement TaskExecutorCell live dispatch or flip clap default back to `runner-v2` (main.rs:1361; task_executor.rs:70–86) — verify: `cargo run -p roko-cli -- plan run plans/ --dry-run` then a real 1-task plan produces agent output, not "dry-run fallback" warnings
- [ ] **[P0]** Fix `roko resume`: route to runner-v2 (or graph resume once it exists) instead of hard-coded `PlanEngine::Graph` (main.rs:2699) — verify: `roko plan run plans/ --engine runner-v2`, interrupt, `roko resume` continues from snapshot
- [ ] **[P0]** Fix `/search` Perplexity body: send flat `{"query": …}` per-query instead of `{"queries":[…]}` batch (roko-agent/src/perplexity/search.rs:150); drop `MAX_BATCH_SIZE`/`TooManyQueries`; fix date format to MM/DD/YYYY (research.rs); update mock tests to real API shape — verify: `roko research search "hdc"` returns results, not HTTP 400
- [ ] **[P0]** Fix ACP `/plan-resume` flag: `--resume` → `--resume-plan` (roko-acp/src/bridge_events.rs:3617) — verify: `/plan-resume` in Zed continues from snapshot instead of restarting
- [ ] **[P2]** Route CLI user-facing output through `output_format.rs`/`inline` instead of 299 raw `eprintln!` (29 files) so `--quiet`, color, and spinners work uniformly — verify: `roko prd list --quiet` emits nothing to stderr
- [ ] **[P2]** Extend `--context <PATH>` to `develop` and add ACP `/context` (currently only `do` main.rs:375 + `plan generate` main.rs:1396) — verify: `roko develop --context ./crates "x"` injects context
- [ ] **[P1]** Update docs/v2/CLI-REFERENCE.md: add `--engine/--max-tasks/--force-resume` to plan run; add develop/show/setup/think/note/tune/demo/graph/isfr/feed/dev/history/config{doctor,export,mcp}/status{--quick,--surfaces} — verify: diff doc headings against `roko --help` tree
- [ ] **[P1]** Update CLAUDE.md CLI table: add `do`/`develop`/`show`/`setup`/`graph`, remove "`roko chat`", stop describing orchestrate.rs as the main loop — verify: every listed command exists in `roko --help`
- [ ] **[P1]** Decide `legacy-runner-v2` feature fate: gate the plan.rs runner-v2 arm behind it or remove the feature (Cargo.toml:20) — verify: `cargo build -p roko-cli --no-default-features` and `--all-features` both compile with expected `--engine` values
- [ ] **[P2]** Emit deprecation hint on `roko run`→`do` alias path using hints.rs (main.rs:2340) — verify: `roko run "x"` prints one-time hint
- [ ] **[P2]** Consolidate `tune` vs `learn tune` into one surface — verify: `roko tune gates` and `roko learn tune gates` share output or one aliases the other
- [ ] **[P2]** Fix stale module doc (main.rs:4–8) and `roko neuro search` reference (explain.rs:96) — verify: grep for `neuro search` returns nothing
- [ ] **[P3]** Implement semantic exit codes 10–15 per v2-depth §8 — verify: gate-failure run exits 10
- [ ] **[P3]** Add `roko new probe` scaffold (v1/12-interfaces/02 §162) — verify: `roko new probe mem-check` compiles
- [ ] **[P3]** Graph-engine parity: resume, approval, max-parallel on `cmd_plan_run_engine` (plan.rs:1567) — verify: flags no longer warn/ignore

## Open questions

1. Which engine is *intended* as default? Git flip-flops (`9423998a7` runner-v2 → current graph) vs GAPS.md Task 102 "Engine as Default" — was reverting to graph-with-dry-run-fallback deliberate staging or a merge accident?
2. Should `roko run` remain a silent alias for `do`, become a documented compatibility verb (per v1 §34), or be deprecated with hints.rs?
3. Is the `--provider` direct-chat path (`anthropic_api`/`openai_compat` only, agent_serve.rs:138–140) meant to grow to all 5+ backends, or is sidecar/serve routing the strategic path?
4. `isfr`/`feed`/`graph`/`layer-check` are absent from every doc — are they operator-facing or internal dev tools that should be hidden behind a `dev` namespace?
5. v2-depth §3's 9-verb model: adopt (`watch`/`inspect` as aliases for `show --follow`/`show <id>`) or formally retire the design?

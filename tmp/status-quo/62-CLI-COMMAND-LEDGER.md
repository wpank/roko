# CLI Command Ledger

> Re-verified 2026-07-08 against git HEAD `5852c93c05`. All 45 top-level variants confirmed in the clap `Command` enum (`crates/roko-cli/src/main.rs:324–897`). For the full expanded surface (flags, handlers, bare modes, doc drift), see [45-CLI-SURFACE.md](45-CLI-SURFACE.md); this file is the condensed command-by-command status ledger.
>
> **Deep second pass (HEAD `5852c93c05`, 2026-07-08):** verified the enum count = **exactly 45** top-level variants (`Init`→`Explain`, main.rs:324–575; variants after that in the sed window are `KnowledgeCmd` leaves). Added the full leaf sub-subcommand ledger (§ "Leaf sub-subcommand ledger"), the broken/dry-run tally, and per-leaf handler modules. **Ground-truth count: 2 broken top-level paths (`resume`, `research search`), 1 dry-run-by-default (`plan run`), 3 partial (`graph`, `isfr`, `feed`).** CLAUDE.md documents ~24 groups; 6 are undocumented (`graph`/`isfr`/`feed`/`dev`/`up`/`acp`) and CLAUDE.md still references a nonexistent `roko chat`.

The CLI is the main product surface and the largest convergence point. It is real, but it mixes live runtime paths, legacy commands, demos, server operations, and migration tools.

## Top-Level Shape

`crates/roko-cli/src/main.rs` declares the main parser and command enum. Major command groups:

- Core workflow: `init`, `do`, `develop`, `run`, `status`, `show`, `doctor`, `setup`, `layer-check`.
- Planning: `plan`, `prd`, `research`, `think`, `note`.
- Runtime/agents: `agent`, `serve`, `up`, `acp`, `daemon`, `worker`, `dashboard`, `resume`.
- Learning/knowledge: `knowledge`, `learn`, `tune`, `index`, `graph`, `isfr`, `feed`.
- Operations: `job`, `bench`, `demo`, `config`, `deploy`, `vision-loop`, `dev`.
- Utilities: `replay`, `history`, `inject`, `completions`, `new`, `explain`, `login/logout/whoami`.

## Command-by-command status ledger

Status: ✅ works · 🟡 partial/caveated · ❌ broken · 🕰️ legacy/superseded. Handler = source module. All line refs on HEAD `5852c93c0`.

| Command | Handler module | What it does | Status |
|---|---|---|---|
| `init` | commands/util.rs (→init.rs) | create `.roko/` + `roko.toml`; `--cloud --profile --demo` | ✅ |
| `do` | commands/do_cmd.rs:14 | ScopeResolver classify → WorkflowEngine (simple/planned/architectural); `--context --compare --continue` | ✅ v2 |
| `develop` | commands/develop.rs:14 | plan-first: generate → approve → execute | ✅ v2 |
| `run` | main.rs:2340 → do_cmd / util.rs:232 | aliases to `do` unless `--serve/--share/--max-retries`, then WorkflowEngine | 🟡 silent alias, undocumented |
| `status` | commands/status.rs / util.rs | signal/episode counts; `--quick --cfactor --surfaces` | ✅ |
| `show` | commands/show.rs | costs/agents/knowledge/plans/learning/history/`<id>`; `--live`→TUI | ✅ |
| `doctor` | commands/util.rs:1071 | workspace/bootstrap diagnosis | ✅ |
| `setup` | commands/setup.rs | interactive provider wizard | ✅ |
| `layer-check` | lib.rs:86 layer_check | architecture layer lint | ✅ |
| `plan run` | commands/plan.rs:220 | **default `--engine graph` = dry-run fallback** (task_executor.rs:84); `runner-v2` = full executor | 🟡 default no-op |
| `plan list/show/create/validate/generate/regenerate` | commands/plan.rs | plan CRUD + lint + generate | ✅ |
| `prd` | commands/prd.rs | idea/list/status/plan/consolidate/draft{new,edit,promote,list} | ✅ |
| `agent` | commands/agent.rs → agent_serve.rs | create/delete/list/start/stop/status/serve/chat | ✅ |
| `research` | commands/research.rs | topic/enhance-*/analyze/list/**search** | 🟡 `search` broken (Perplexity 400, search.rs:150) |
| `think` | commands/think.rs | read-only research | ✅ |
| `note` | commands/note.rs | no-LLM tagged capture | ✅ |
| `tune` | commands/tune.rs:140 | routing/gates/budget/model — mutates config | 🟡 overlaps `learn tune` (read-only) |
| `knowledge` | commands/knowledge.rs | query/stats/gc/backup/restore/sync/archive + dream + custody | ✅ |
| `learn` | commands/learn.rs | all/route/experiments/efficiency/episodes/tune | ✅ |
| `job` | commands/job.rs:433 | list/create/match/show/execute/cancel (match/execute via serve) | ✅ |
| `bench` | commands/bench.rs | demo `--real`; swe `--dataset --agent-mode` | ✅ |
| `demo` | commands/*(demo_cmd) | setup/warm | ✅ |
| `config` | commands/config_cmd.rs | ~38 leaves: providers/models/secrets/plugins/mcp/experiments… | ✅ |
| `index` | commands/util.rs:1251 | build/rebuild/search/stats; auto-rebuild after plan/prd/research | ✅ |
| `graph` | commands/graph.rs:47 | run/validate/show `<toml>` → GraphEngine (cells may be Passthrough stubs) | 🟡 |
| `isfr` | commands/isfr.rs:51 | start/status/sources → roko-chain ISFRKeeper | 🟡 mock sources; relay "Phase 2" |
| `feed` | commands/feed.rs:26 | list/status → HTTP client to serve | ✅ (needs serve) |
| `dev` | commands/dev.rs | serve + demo frontend | ✅ |
| `up` | commands/server.rs | serve + all `[[agents]]` | ✅ |
| `serve` | main.rs:2508 → roko-serve | ~85 routes on :6677; `--tui` embeds dashboard | ✅ |
| `acp` | main.rs:2020 → roko_acp | editor-integration stdio server | ✅ (slash-cmd bugs, see below) |
| `daemon` | commands/server.rs | start/stop/status/logs/reload/restart/install/uninstall | ✅ |
| `deploy` | commands/server.rs | railway/fly/docker | ✅ (external creds) |
| `worker` | lib.rs worker | run as deployed worker | ✅ |
| `dashboard` | commands/dashboard.rs:7 | ratatui TUI F1–F7 | ✅ |
| `login/logout/whoami` | commands/auth.rs | Privy browser / API key | ✅ |
| `vision-loop` | vision_loop/ | screenshot→score→iterate | ✅ (needs vision model + URL) |
| `resume` | main.rs:2650 | sugar for plan run --resume-plan | ❌ **broken**: hard-codes `PlanEngine::Graph` (main.rs:2699), snapshot ignored |
| `replay` | commands/util.rs:1114 | walk signal DAG `<hash>` | ✅ |
| `history` | main.rs:2718 | chat session summaries | ✅ |
| `inject` | commands/util.rs | `<session> <payload> --kind` | ✅ |
| `completions` | commands/util.rs:1420 | bash/zsh/fish | ✅ |
| `new` | scaffold.rs:21 | 9 scaffold types (no `probe`) | ✅ |
| `explain` | commands/explain.rs | `<topic> --depth 1..3` | ✅ (stale `neuro search` ref, explain.rs:96) |

## Leaf sub-subcommand ledger (deep pass, HEAD `5852c93c05`)

Every leaf of the multi-level subcommands, with its defining enum (`file:line`) and status. Status: ✅ works · 🟡 partial · ❌ broken · needs-serve = requires `roko serve` running.

| Group → leaf | Enum def | Status / note |
|---|---|---|
| `plan list/show/create/validate/generate/regenerate` | `PlanCmd` main.rs:1307 | ✅ all CRUD + lint + generate live |
| `plan run` | main.rs:1357; commands/plan.rs:220 | 🟡 **dry-run default**: `--engine graph` (clap `default_value="graph"`) → `cmd_plan_run_engine` (plan.rs:1567) → `TaskExecutorCell{dry_run:true}` (cells/task_executor.rs:33) emits synthetic engram, no LLM. `--engine runner-v2` = live executor |
| `prd idea/list/status/consolidate` | `PrdCmd` main.rs:1410 | ✅ |
| `prd draft new/edit/promote/list` | `PrdDraftCmd` main.rs:1438 | ✅ |
| `prd plan <slug>` | `PrdCmd::Plan` main.rs:1426 | ✅ agent generates tasks.toml |
| `agent create/delete/list/start/stop/status` | `AgentCmd` agent_serve.rs:33 | ✅ |
| `agent serve` | `AgentCmd::Serve` agent_serve.rs:97 | 🟡 cognitive loop may start with stub cells |
| `agent chat` | `AgentCmd::Chat` agent_serve.rs:99 | ✅ |
| `research topic/enhance-prd/enhance-plan/enhance-tasks/analyze/list` | `ResearchCmd` main.rs:1462 | ✅ |
| `research search` | `ResearchCmd::Search` main.rs:1491 | ❌ **broken**: `search_batch` posts `{"queries":[…]}` (perplexity/search.rs:150) but Perplexity has no batch endpoint → HTTP 400 |
| `tune routing/gates/budget/model` | `TuneCmd` main.rs:1504 | 🟡 mutates config; overlaps read-only `learn tune` |
| `knowledge query/stats/gc/backup/restore/sync/archive` | `KnowledgeCmd` main.rs:896 | ✅ |
| `knowledge dream run/report/schedule/journal/archive` | `KnowledgeDreamCmd` main.rs:993 | ✅ (no cron trigger; manual only) |
| `knowledge custody list/show/verify` | `KnowledgeCustodyCmd` main.rs:1035 | ✅ |
| `learn all/route/experiments/efficiency/episodes/tune` | `LearnCmd` main.rs:1064 | ✅ (`tune` here is read-only) |
| `job list/create/match/show/execute/cancel` | `JobCmd` main.rs:1534 | ✅ (`match`/`execute` need-serve fallback) |
| `bench` (demo/swe) | commands/bench.rs | ✅ |
| `demo setup/warm` | `DemoCmd` main.rs:1110 | ✅ |
| `config init/show/path/doctor/edit/set/set-secret/check-secrets/validate/migrate/export/events/experiments/plugins/secrets` | `ConfigCmd` main.rs:1713 | ✅ (~38 leaves total) |
| `config providers list/health/test/available` | `ConfigProviderCmd` main.rs:1887 | ✅ |
| `config models list/route` | `ConfigModelCmd` main.rs:1916 | ✅ |
| `config subscriptions list/add/remove/enable/disable` | `ConfigSubscriptionCmd` main.rs:1944 | ✅ |
| `config mcp list/test/add` | `ConfigMcpCmd` main.rs:1974 | ✅ |
| `index build/rebuild/search/stats` | `IndexCmd` main.rs:1220 | ✅ |
| `graph run/validate/show <toml>` | `GraphCmd` commands/graph.rs:17 | 🟡 loader requires `[graph]` table; 4/7 examples parse, 2 have live cells (see doc 73) |
| `isfr start/status/sources` | `IsfrCmd` commands/isfr.rs:13 | 🟡 mock sources; relay "Phase 2" |
| `feed list/status` | `FeedCmd` commands/feed.rs:16 | ✅ needs-serve (HTTP client to :6677) |
| `daemon start/stop/status/logs/reload/restart/install/uninstall` | `DaemonCmd` main.rs:1259 / daemon.rs:47 | ✅ |
| `deploy railway/fly/docker` | `DeployCmd` main.rs:1669 | ✅ (external creds) |

### Broken / dry-run tally (deep pass)

| Class | Count | Members |
|---|---:|---|
| ❌ Broken (non-functional path) | **2** | `resume` (hard-codes `PlanEngine::Graph`, main.rs:2709 → snapshot ignored); `research search` (Perplexity 400, search.rs:150) |
| 🟡 Dry-run by default | **1** | `plan run` (Graph engine → `TaskExecutorCell{dry_run:true}`; live path only via `--engine runner-v2`) |
| 🟡 Partial / mock / needs-serve | **3** | `graph` (stub/no-op cells), `isfr` (mock sources), `feed` (needs `serve`) |

## Cross-cutting ACP/search drift (re-verified 2026-07-08, all OPEN)

Navigation-layer bugs that flow through the CLI or its ACP mirror. Confirmed on HEAD `5852c93c0`.

| Bug | Where | Status | Effect |
|---|---|---|---|
| `/search` batch body | roko-agent/src/perplexity/search.rs:150 (`{"queries":[…]}`) | ❌ P0 | Perplexity 400; `roko research search` + ACP `/search` 100% broken |
| ACP `/plan-resume` flag | roko-acp/src/bridge_events.rs:3617 (`--resume` not `--resume-plan`) | ❌ P0 | ACP resume restarts plan from scratch |
| ACP `/plan-run` no `--model` | roko-acp bridge_events (session model dropped) | 🟡 | Plan runs with default model, ignores selection |
| ACP `/develop` missing | roko-acp session/bridge | 🟡 | `develop` unusable from editor; must use `/do` |
| Raw `eprintln!` user output | 299 calls / 29 CLI files | 🟡 P2 | No `--quiet`/color/spinner; tracing noise |
| `--context` coverage | on `do`+`plan generate` only (main.rs:375,1396) | 🟡 | `develop` + ACP `/context` still gapped |

## Ordered P0/P1 roadmap

1. **[P0]** `roko resume`: route to `runner-v2` instead of hard-coded `PlanEngine::Graph` (main.rs:2699).
2. **[P0]** `plan run` default engine: flip clap default to `runner-v2` OR implement TaskExecutorCell live dispatch (main.rs:1361; task_executor.rs:84).
3. **[P0]** `/search` Perplexity body: flat `{"query":…}` per query (search.rs:150); fix date + response parsing.
4. **[P0]** ACP `/plan-resume`: `--resume` → `--resume-plan` (bridge_events.rs:3617).
5. **[P1]** ACP `/plan-run` `--model` passthrough; wire `/develop` slash command.
6. **[P1]** Reconcile `PlanEngine::default()` (RunnerV2) vs clap default (graph); resolve `legacy-runner-v2` feature fate.
7. **[P1]** Doc sync: CLI-REFERENCE.md + CLAUDE.md add develop/show/setup/graph/isfr/feed/dev; drop nonexistent `roko chat`; stop calling orchestrate.rs the main loop.
8. **[P2]** Route CLI output through `output_format.rs`; extend `--context` to develop/ACP; consolidate `tune` vs `learn tune`.

## P0 CLI Drift

| Area | Current state | Fix |
|---|---|---|
| Plan engine default | `PlanEngine` derives `Default` as `RunnerV2`, but Clap arg for `plan run --engine` has `default_value = "graph"` | Remove contradictory default or make Clap/runtime/docs agree. |
| Graph default | Help says Graph Engine default; code path warns resume unsupported; task execution is not live parity | Make Graph unsupported for live plan tasks or complete dispatch parity. |
| Resume command | `roko resume` is sugar for `plan run --resume-plan` but constructs `engine: PlanEngine::Graph` | Route resume to Runner v2 until Graph resume exists. |
| `--resume-plan` | Runner v2 honors/copies snapshot; Graph ignores with warning | Fail closed on Graph or implement snapshot hydration. |
| Init hint | `cmd_init` still prints a stale `--resume` hint while the plan flag is `--resume-plan`/`--resume-state` | Update hint text and add a regression test. |
| Surface inventory | `surface_inventory.rs` claims single-source status but includes stale/legacy names that no longer match Clap | Regenerate from Clap/handler registry or clearly mark as historical. |
| Legacy command mirrors | `NeuroCmd` and `DreamCmdLegacy` remain as internal mirrors | Keep hidden compatibility or retire after `knowledge` command parity. |

## Command Groups Needing Ownership Decisions

| Group | Risk | Decision needed |
|---|---|---|
| `run`, `do`, `develop`, `plan run` | Multiple execution abstractions | Which command is the default "do work" story? |
| `graph` and `plan --engine graph` | Graph examples can imply production parity | Is Graph experimental or production? |
| `serve`, `up`, `daemon`, `worker`, `deploy` | Overlapping server lifecycle and deployment language | Which command starts local product mode? |
| `knowledge`, `learn`, `dream`, `neuro` aliases | Historical noun drift | Public nouns and migration aliases. |
| `job`, serve jobs, chain jobs | Local JSON vs marketplace/chain | One job lifecycle or explicit modes. |
| `demo` | Static and current demos overlap | Supported demo command target. |

## Partial/Compatibility Paths

| Command/path | Current caveat | Action |
|---|---|---|
| `roko run <prompt>` | Compatibility path; simple prompt delegates toward `do`, while `--serve`/`--share`/retry options use workflow-template code | Document or collapse into `do`/workflow engine. |
| `roko do --compare` | Preview/dry comparison, not execution of both paths | Rename/help text should say preview. |
| `roko do --continue` | Points users at `roko resume`, which is currently routed to Graph | Fix resume first. |
| `roko develop` | Advertises plan-first approval but currently includes dry-run preview and existing-plan approval before generation/execution | Align behavior with spec or soften help text. |
| `roko learn tune` | Read-only reporting, while top-level `tune` mutates config | Rename or clarify. |
| Local `job execute` | Falls back to `run_once` compatibility path without full server parity | Move to canonical runtime path. |
| `agent serve` cognitive loop | Can start with stub cells | Mark experimental or require real cells. |

## CLI Migration Checklist

- [ ] Add `roko plan run --help` assertion test for engine/default wording.
- [ ] Add parse test that `PlanEngine::default()` and Clap default agree or document why they differ.
- [ ] Change `roko resume` to a snapshot-capable engine.
- [ ] Add `roko plan run --engine graph --resume-plan` test that fails or resumes correctly.
- [ ] Add test for init resume hint text.
- [ ] Add CLI surface inventory test that rejects stale removed commands.
- [ ] Mark experimental commands in help text when they are not production paths.
- [ ] Add command inventory generation to docs CI.
- [ ] Remove or hide legacy `neuro`/`dream` mirrors after `knowledge` parity.
- [ ] Ensure `--json` output is stable for status, plan, learn, config, and route diagnostics.

## Recommended User-Facing Wording

Until Graph is live:

```text
roko plan run plans/ --engine runner-v2
```

Use this as the documented live execution command. Describe Graph as:

```text
Graph Engine: v2 graph execution target; currently suitable for graph loading/topology/cell tests, not the default live agent execution path.
```

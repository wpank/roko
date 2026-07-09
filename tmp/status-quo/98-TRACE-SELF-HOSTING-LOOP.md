# 98 — TRACE: The Self-Hosting Loop, Command by Command

> **Verification header**
> - Repo: `/Users/will/dev/nunchi/roko/roko`
> - Git HEAD: `5852c93c05` on `main`
> - Date: 2026-07-08
> - Method: read the real handler modules; traced each hop to `file:line`.
> - Companion docs: `91-PRD-RESEARCH`, `36-ORCHESTRATION-RUNNERS`, `95-ENGINE-DRIFT`.

Status tags: **[WORKS]** verified real path · **[STUB]** runs but produces synthetic/no effect ·
**[BROKEN]** fails at runtime · **[PARTIAL]** works with caveats · **[FOOTGUN]** works but the
default silently does the wrong thing.

The advertised loop (CLAUDE.md, "Self-hosting workflow"):

```
prd idea → prd draft new → research enhance-prd / topic → prd plan <slug>
        → plan run plans/ → gates → episodes/learning writeback → prd status / status
```

---

## 0. Pipeline diagram (real code path)

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│ roko prd idea "<text>"                                        commands/prd.rs:308  │
│   → prd::cmd_idea()                                                   prd.rs:652   │
│   ⇒ APPENDS one line to .roko/prd/ideas.md            [WORKS] no agent, pure fs    │
└──────────────────────────────────────────────────────────────────────────────────┘
                                     │
                                     ▼
┌──────────────────────────────────────────────────────────────────────────────────┐
│ roko prd draft new "<title>"                                 commands/prd.rs:325   │
│   scaffold write → repo_context pack → run_agent_capture_silent (role=scribe,     │
│                                             allowed_tools="none")     prd.rs:459   │
│   ⇒ WRITES .roko/prd/drafts/<slug>.md (+ .context.json + .validation.json)         │
│   ⇒ episode → .roko/episodes.jsonl                    [WORKS] real agent dispatch  │
└──────────────────────────────────────────────────────────────────────────────────┘
                                     │
                                     ▼
┌──────────────────────────────────────────────────────────────────────────────────┐
│ roko research enhance-prd <slug>                            commands/research.rs:499│
│   run_agent_capture_silent (role=researcher, allowed_tools="Read,Write,Edit")     │
│   ⇒ EDITS the PRD in place + writes .roko/research/enhance-<slug>.md               │
│   ⇒ episode                                           [PARTIAL] Claude-CLI only ok  │
│                                                                                    │
│ roko research topic "<t>"                                  commands/research.rs:27  │
│   provider ladder: deep(sonar) → gemini grounding → perplexity chat → claude      │
│   ⇒ .roko/research/<slug>.md                          [PARTIAL] provider-dependent  │
│                                                                                    │
│ roko research search "<q>"                                commands/research.rs:718  │
│   PerplexitySearchClient.search_batch()             perplexity/search.rs:141       │
│   ⇒ 422 at runtime; unit tests self-referentially green   [BROKEN]                 │
└──────────────────────────────────────────────────────────────────────────────────┘
                                     │
                                     ▼
┌──────────────────────────────────────────────────────────────────────────────────┐
│ roko prd plan <slug>                                        commands/prd.rs:745    │
│   find_prd → resolve model (role=strategist) → preflight →                         │
│   prd::generate_plan_from_prd_with_model()                          prd.rs:999     │
│   ⇒ agent WRITES plans/<slug>/plan.md + tasks.toml    [WORKS] real agent dispatch  │
│   prints "Next: roko plan run plans/<slug>/"                        prd.rs:778     │
└──────────────────────────────────────────────────────────────────────────────────┘
                                     │
                                     ▼
┌──────────────────────────────────────────────────────────────────────────────────┐
│ roko plan run plans/                                        commands/plan.rs:220   │
│   validate_before_run → (dry_run?) →                                              │
│   ── engine defaults to "graph" (clap default_value) ──         main.rs:1361      │
│   if PlanEngine::Graph  → cmd_plan_run_engine()                     plan.rs:258   │
│        plan_to_graph → every node cell_type="task-executor"    convert.rs:63      │
│        TaskExecutorCell.execute() → ALWAYS synthetic output   task_executor.rs:62 │
│   ⇒ NO agent, NO gates, NO file edits, NO episodes    [STUB / FOOTGUN]             │
│                                                                                    │
│   only if `--engine runner-v2` is passed:                          plan.rs:269    │
│        runner::event_loop::run() → real dispatch + gates +                         │
│        feedback_facade sinks (episodes, routing, knowledge)    plan.rs:471-654    │
│   ⇒ real agent, gates, .roko/episodes.jsonl, cascade router   [WORKS]             │
└──────────────────────────────────────────────────────────────────────────────────┘
                                     │
                                     ▼
┌──────────────────────────────────────────────────────────────────────────────────┐
│ roko prd status                     prd.rs:753  │  roko status (signals/episodes)  │
│   hardcoded 5-col table (PRD/Status/Plans/Tasks/Done)  [PARTIAL] static columns    │
└──────────────────────────────────────────────────────────────────────────────────┘
```

---

## 1. Per-stage trace

### Stage 1 — `roko prd idea "<text>"`  **[WORKS]**

- Handler: `commands/prd.rs:308` (`PrdCmd::Idea`) → `roko_cli::prd::cmd_idea` (`prd.rs:652`).
- Agent? **No.** Pure filesystem append.
- Persists: appends `- <timestamp> — <text>\n` to `ideas_path()` = **`.roko/prd/ideas.md`** (`prd.rs:654-663`).
- Next-step hint printed (`prd.rs:311`).
- Break/stub: none. This is the one hop with zero moving parts.

### Stage 2 — `roko prd draft new "<title>"`  **[WORKS]**

- Handler: `commands/prd.rs:325` (`PrdDraftCmd::New`).
- Flow: slugify → acquire workspace lock (`prd.rs:333`) → resolve model key for role **`scribe`** (`prd.rs:359`) → provider preflight (`prd.rs:372`) → write skeleton scaffold (`prd.rs:381`) → build repo-context pack if source present (`prd.rs:409`) → **`run_agent_capture_silent`** with `allowed_tools: Some("none")` (`prd.rs:459-470`).
- Agent? **Yes — real dispatch** via `agent_exec::run_agent_capture_silent`. Provider path = whatever the resolved `scribe` model maps to (default Claude CLI).
- Output handling: agent may write the file directly OR return markdown as text; `materialize_agent_markdown_output` writes it (`prd.rs:481-522`).
- Persists:
  - **`.roko/prd/drafts/<slug>.md`** (the PRD).
  - **`.roko/prd/drafts/<slug>.context.json`** (repo-context sidecar, `prd.rs:576`).
  - **`.roko/prd/drafts/<slug>.validation.json`** (grounding report, `prd.rs:579`).
  - Episode → **`.roko/episodes.jsonl`** via `persist_capture_episode` (`prd.rs:604`).
- Post-checks: `check_grounding_section` + `validate_prd_grounding` warn on missing `## Repository Grounding` and on duplicate-crate proposals (`prd.rs:550-573`).
- Break/stub: none in the happy path. Caveat: `allowed_tools="none"` means the model must return the PRD inline; a non-Claude provider that ignores that and expects file tools will produce an empty draft (handled: file removed, exit 1).

### Stage 3a — `roko research enhance-prd <slug>`  **[PARTIAL]**

- Handler: `commands/research.rs:499` (`ResearchCmd::EnhancePrd`).
- Reads the PRD (`research.rs:501`), builds a `researcher` system prompt, dispatches **`run_agent_capture_silent`** with `allowed_tools: Some("Read,Write,Edit")` (`research.rs:515-525`).
- Agent? **Yes.** Edits the PRD in place and also writes **`.roko/research/enhance-<slug>.md`**. Episode persisted (`research.rs:530`).
- Break/stub: `allowed_tools="Read,Write,Edit"` is the **tool-alias footgun** — on non-Claude providers the alias normalization in `roko-agent/src/dispatcher/tool_selector.rs` drops/renames these tools, so the agent runs with no file tools and cannot edit in place (see `95-ENGINE-DRIFT`). Works reliably only on the Claude CLI backend.

### Stage 3b — `roko research topic "<topic>"`  **[PARTIAL]**

- Handler: `commands/research.rs:27`. Provider **ladder**, first match wins:
  1. `--deep` → Perplexity `sonar-deep-research`, async 15 s polling (`research.rs:32-160`). Writes `.roko/research/<slug>-deep.md`.
  2. `config.gemini.grounding_model` set → Gemini grounded research (`research.rs:164-343`). Writes `.roko/research/<slug>.md` with grounding citations.
  3. `config.perplexity.default_search_model` set → Perplexity chat agent (`research.rs:346-457`). Writes `.roko/research/<slug>.md`.
  4. else Claude-CLI fallback with `allowed_tools="Read,Write,Edit"` (`research.rs:461-497`).
- Agent? **Yes** in all branches. Each persists an episode.
- Break/stub: depends entirely on which providers are configured + keyed. The Claude fallback carries the same tool-alias caveat. This is the `research topic` path — **distinct from `research search`** below.

### Stage 3c — `roko research search "<query>"`  **[BROKEN]**

- Handler: `commands/research.rs:718` (`ResearchCmd::Search`) → `PerplexitySearchClient::search_batch` (`perplexity/search.rs:141`).
- Two contract mismatches vs the real Perplexity `POST /search` API:
  1. **Request body**: code sends `{"queries":[{...}]}` (`search.rs:150`). The live `/search` endpoint expects a top-level `{"query": ...}` shape → **HTTP 422** at runtime.
  2. **Response parse**: code expects `results` to be an array of `{query, results}` groups and deserializes each item as `SearchResponse` (`search.rs:181-186`). The live API returns `results` as a flat array of result objects, so parsing would fail even if the request were accepted.
- Why tests are false-green: every test feeds a `MockPoster` whose canned body (`canned_results`/`canned_batch`, `search.rs:269-313`) is built to the code's own wrong shape. The suite validates the client against itself, never against the real wire contract. All 20+ tests pass; the feature has never worked against production.
- Net: `roko research search` is **100% broken** end-to-end. `research topic`/`enhance-prd` do NOT use this client, so they are unaffected.

### Stage 4 — `roko prd plan <slug>`  **[WORKS]**

- Handler: `commands/prd.rs:745` (`PrdCmd::Plan`).
- Flow: lock (`prd.rs:748`) → `find_prd` in published/ or drafts/ (`prd.rs:749`, `prd.rs:865`) → resolve model for role **`strategist`** (`prd.rs:750`) → provider preflight (`prd.rs:763`) → **`generate_plan_from_prd_with_model`** (`prd.rs:767` → `prd.rs:999`).
- Agent? **Yes — real dispatch**, `strategist` role. The agent reads the PRD and emits plan directories. Escalation chain haiku→sonnet→opus on validation failure (`prd.rs:1026`).
- Persists: **`plans/<slug>/plan.md`** and **`plans/<slug>/tasks.toml`** (task DAG with tier / model_hint / context / verify). Episode persisted inside the generator.
- Next hint: `Next: roko plan run plans/<slug>/` (`prd.rs:778`).
- Break/stub: none in generation. The hint sends the user straight into the FOOTGUN below.

### Stage 5 — `roko plan run plans/`  **[STUB by default / WORKS with a flag] — the central break**

- Handler: `commands/plan.rs:220` (`PlanCmd::Run`).
- Order of operations: resolve workdir/plans dir → `validate_before_run` (`plan.rs:248`) → if `--dry-run` show summary (`plan.rs:253`) → **engine dispatch** (`plan.rs:258`).
- **The default engine is `graph`.** The clap arg is declared `#[arg(long, default_value = "graph", value_enum)] engine: PlanEngine` at `main.rs:1361`. Note the enum itself marks `RunnerV2` as `#[default]` (`main.rs:1301`), but **the clap `default_value = "graph"` wins for CLI parsing** — so bare `roko plan run plans/` takes the Graph branch. This enum-default vs clap-default disagreement is the footgun.
- Graph branch (`plan.rs:258` → `cmd_plan_run_engine` `plan.rs:1567`):
  - `plan_to_graph` sets **every** node `cell_type = "task-executor"` (`convert.rs:63`).
  - `default_registry` binds `task-executor` → `TaskExecutorCell` (`engine.rs:356`).
  - `TaskExecutorCell::execute` (`task_executor.rs:62`): `dry_run` branch returns `task-output:dry-run:<label>`; the "live" branch logs *"live dispatch not yet implemented"* and returns `task-output:stub:<label>` (`task_executor.rs:80-92`).
  - Consequence: **no agent is dispatched, no gate runs** (the gate cells in `default_registry` are never wired into the converted graph — only `task-executor` nodes exist), **no files change, no episodes are written.** The command prints `SUCCESS` regardless.
- Runner-v2 branch — the REAL path (`plan.rs:269`, reached only via `--engine runner-v2`):
  - Workspace lock, config load, provider preflight, gate-dep preflight (`plan.rs:272-341`).
  - Auto-git-init so agents have git tooling (`plan.rs:384-409`).
  - Loads plans, scaffolds missing crates (`plan.rs:411-424`).
  - Builds `FeedbackFacade` with three sinks: **EpisodeSink** → `.roko/episodes.jsonl`, **RoutingObservationSink** → cascade router, **KnowledgeIngestionSink** → neuro store (`plan.rs:471-489`).
  - `runner::event_loop::run(...)` does real agent dispatch + the gate pipeline + cost accounting (`plan.rs:654`).
- Break/stub summary: `plan run` **appears** wired but the advertised default runs a stub. Real execution + gates + learning writeback require `--engine runner-v2`.

### Stage 6 — Gates

- Only reached on the **runner-v2** path (inside `event_loop::run`, per `36-ORCHESTRATION-RUNNERS`). Max rung is derived from config (`plan.rs:525`): `skip_tests ? clippy_enabled : 2`.
- On the Graph default path gates never run — the `gate.*` cells exist in `default_registry` (`engine.rs:314-339`) but the plan→graph converter emits no gate nodes.

### Stage 7 — Episodes / learning writeback

- `prd draft new`, `research *`, `prd plan` each write an episode via `persist_capture_episode` regardless of engine (they are their own agent calls).
- Plan-execution episodes + routing + knowledge writeback happen **only** on runner-v2 (`plan.rs:471-489`). The Graph default writes nothing to `.roko/`.

### Stage 8 — `roko prd status` / `roko status`  **[PARTIAL]**

- `prd status` → `prd::cmd_status` (`commands/prd.rs:320` → `prd.rs:753`).
- Prints a **hardcoded 5-column table**: `PRD | Status | Plans | Tasks | Done` (`prd.rs:758-765`). Columns are static `println!` width specifiers, not data-driven; no per-PRD gate/coverage rollup beyond counts. Iterates published + drafts (`prd.rs:767`).
- `roko status` is a separate handler (`commands/status.rs`) reporting signal + episode counts.

---

## 2. Auto-plan trigger (server path, parallel to CLI)

When `roko serve` is running and a PRD is **published/promoted**, plan generation can fire automatically — but only if BOTH flags are on:

- Gate: `config.serve.auto_orchestrate && config.prd.auto_plan` (`roko-serve/src/routes/prds.rs:165`, again at `prds.rs:587`).
- Subscriber: `spawn_prd_publish_subscriber` listens on the event bus (`prds.rs:223`); `start_prd_publish_subscriber` also tails an episode audit file (`prds.rs:256`).
- On `PrdPublished`: `handle_prd_published_event` → `queue_plan_generation_after_publish` (`prds.rs:158-185`), which calls the runtime's `generate_plan_from_prd` (`runtime.rs:260`, `job_runner.rs:635`).
- This queues **generation only** (Stage 4 equivalent). It does NOT auto-run the plan, so it never touches the Stage-5 stub. Default config leaves `auto_plan` off, so this is dormant unless explicitly enabled.

---

## 3. Per-stage status table

| Stage | Command | Handler (file:line) | Real agent? | Persists | Status | Evidence |
|---|---|---|---|---|---|---|
| 1 | `prd idea` | `prd.rs:308`→`prd.rs:652` | no | `.roko/prd/ideas.md` | **[WORKS]** | pure fs append |
| 2 | `prd draft new` | `prd.rs:325`→`:459` | yes (scribe) | `drafts/<slug>.md` + 2 sidecars + episode | **[WORKS]** | `run_agent_capture_silent` |
| 3a | `research enhance-prd` | `research.rs:499` | yes (researcher) | PRD in place + `research/enhance-<slug>.md` | **[PARTIAL]** | tool-alias footgun on non-Claude |
| 3b | `research topic` | `research.rs:27` | yes (ladder) | `research/<slug>.md` | **[PARTIAL]** | provider-config dependent |
| 3c | `research search` | `research.rs:718`→`search.rs:141` | n/a (HTTP) | nothing | **[BROKEN]** | 422 body + flat-array parse; self-referential mocks |
| 4 | `prd plan <slug>` | `prd.rs:745`→`:999` | yes (strategist) | `plans/<slug>/{plan.md,tasks.toml}` + episode | **[WORKS]** | real generation |
| 5a | `plan run` (default) | `plan.rs:258`→`:1567` | **no** | **nothing** | **[STUB/FOOTGUN]** | `TaskExecutorCell` synthetic output `task_executor.rs:62` |
| 5b | `plan run --engine runner-v2` | `plan.rs:269`→`:654` | yes | episodes, router, knowledge, gates | **[WORKS]** | `FeedbackFacade` sinks `plan.rs:471` |
| 6 | gates | inside `event_loop::run` | — | gate results | **[WORKS on 5b only]** | `max_gate_rung` `plan.rs:525` |
| 7 | learning writeback | `plan.rs:471-489` | — | `.roko/episodes.jsonl` + cascade + neuro | **[WORKS on 5b only]** | three sinks |
| 8 | `prd status` | `prd.rs:753` | no | — | **[PARTIAL]** | hardcoded columns `prd.rs:758` |
| + | server auto-plan | `prds.rs:158-185` | yes | plans dir | **[PARTIAL]** | gated by 2 flags, off by default |

---

## 4. Verdict: can Roko self-host today?

**Yes — but only with two non-default deviations, and with one advertised subcommand fully broken.**

The generative half of the loop (idea → draft → topic/enhance → plan) genuinely dispatches
real agents and writes real artifacts. The executive half (`plan run`) is where the default
path silently no-ops: the advertised `roko plan run plans/` runs the `TaskExecutorCell`
dry-run stub and reports success without touching a single file. To actually execute a plan
you must override the engine. And `roko research search` never works at all.

### The command sequence that DOES self-host (copy-paste)

```bash
# 1. capture (works)
roko prd idea "Wire X into Y"

# 2. draft via real agent (works)
roko prd draft new "wire-x-into-y"

# 3. enrich — use `topic`/`enhance-prd`, NOT `search`; run on the Claude backend
roko research enhance-prd wire-x-into-y        # (Claude CLI backend to avoid tool-alias strip)

# 4. generate plan via real agent (works)
roko prd plan wire-x-into-y

# 5. EXECUTE FOR REAL — the flag is mandatory
roko plan run plans/wire-x-into-y/ --engine runner-v2

# 6. inspect
roko prd status
roko status
```

Avoid: bare `roko plan run plans/` (stub), and `roko research search` (422).

---

## 5. Checklist — make the DEFAULT path self-host cleanly

- [ ] **Flip the `plan run` default engine to `runner-v2`.** Change `main.rs:1361`
      `default_value = "graph"` → `"runner-v2"` (or drop `default_value` so clap honors the
      enum `#[default]` at `main.rs:1301`). This is the single highest-impact fix.
- [ ] **Or implement `TaskExecutorCell` live dispatch** (`task_executor.rs:80-92`) to delegate
      to the runner-v2 agent path, and wire gate nodes into `plan_to_graph` (`convert.rs`)
      so the Graph engine stops being a stub.
- [ ] **Fix `research search` wire contract** (`perplexity/search.rs:119-188`): send the real
      `/search` request shape and parse the flat `results` array. Then rewrite the mocks
      (`search.rs:269-313`) against captured real responses so the suite stops self-certifying.
- [ ] **Fix the tool-alias strip on non-Claude providers** (`dispatcher/tool_selector.rs`; see
      `95-ENGINE-DRIFT`) so `research enhance-prd`/`topic` and Runner v2 agents keep their
      Read/Write/Edit tools across backends.
- [ ] **Make `prd status` data-driven** (`prd.rs:753-765`): derive columns/rollup from actual
      plan + gate state rather than a fixed 5-column `println!` template.
- [ ] **Surface the engine choice in the `prd plan` next-step hint** (`prd.rs:778`): print
      `roko plan run plans/<slug>/ --engine runner-v2` until the default is flipped, so the
      guided workflow doesn't route users into the stub.
- [ ] **Guard against silent stub success**: have the Graph path emit a loud warning (or
      non-zero exit) when it produces only `task-output:dry-run|stub` engrams.

---

## 6. Corrections to prior docs

- **`36-ORCHESTRATION-RUNNERS`**: confirm it states the *default* `plan run` engine is
  `graph`/stub, not runner-v2. The enum default (`RunnerV2`) is misleading; clap's
  `default_value="graph"` (`main.rs:1361`) is authoritative for the CLI.
- **`91-PRD-RESEARCH`**: ensure it distinguishes `research topic`/`enhance-prd` (real agents,
  work) from `research search` (broken Perplexity `/search` client). They share a crate but
  not a code path — only `search` is broken.
- No contradictions found with `95-ENGINE-DRIFT`; this doc extends it with the exact
  `TaskExecutorCell` and `convert.rs` file:line for the stub-execution drift.

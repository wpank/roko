# PRD Lifecycle + Research — Self-Hosting Pipeline
> Status-quo audit · verified 2026-07-07 · sources: 27 (roko-cli: prd.rs 3,849 LOC, commands/prd.rs 962, research.rs 1,069, commands/research.rs 888, prd_prompt.rs 210, plan.rs, workspace_paths.rs, index.rs, main.rs clap defs; roko-serve: lib.rs, routes/prds.rs, routes/mod.rs; roko-core config: project.rs, serve.rs, schema.rs; tests: prd_pipeline.rs, prd_pipeline_workspace.rs, e2e_self_host.rs; live data: `.roko/prd/**`, `.roko/research/**`, `.roko/episodes.jsonl`, `.roko/signals.jsonl`, root `plans/`; git log incl. d38043d66; sibling audits 31, 36, 45)

Status vocab: ✅ works end-to-end · 🔌 built-not-wired · 🟡 partial · ❌ missing · 🕰️ legacy/stale.

> **Re-verified 2026-07-08** against HEAD. New/confirmed P0s: (a) `research search` is **broken three ways** against the real Perplexity Search API; (b) a **cross-cutting tool-alias bug** silently strips all tools from every `research`/`prd`/`analyze` agent when routed to a non-Claude provider. Both are masked by mock-only tests. Details in the two P0 call-outs below.

## ⚠️ P0 call-outs (verified against live code + real Perplexity API contract)

### P0-A · `research search` / `/search` — broken three ways, HTTP 400/422
`commands/research.rs:718–790` → `PerplexitySearchClient::search_batch` (`crates/roko-agent/src/perplexity/search.rs`). Cross-checked against the live Perplexity Search API OpenAPI (`POST https://api.perplexity.ai/search`, fetched 2026-07-08). Three independent contract violations, **each fatal**, **all masked by a `MockPoster` in 20 unit tests** that echoes roko's own invented shapes:

1. **Request wrapper wrong** (search.rs:150): sends `{"queries":[{...}]}`. The API's `ApiSearchRequest` has a top-level required `query` field (string *or* array) and **no `queries` key** → 422 Validation Error / 400.
2. **Response parse wrong** (search.rs:176–187): expects `results[]` to be nested `{query, results[]}` *groups*. The API returns `results[]` as a **flat array of pages** (`{title,url,snippet,date,last_updated}`) plus a top-level `id`. Even if the request succeeded, parsing would fail with "failed to parse result item".
3. **Field name wrong** (`perplexity/types.rs` `SearchResult`): reads `content`; the API field is `snippet`. Would deserialize to empty even past faults 1–2.

Also: `date_range` is emitted as `YYYY-MM-DD` (search.rs:124–126) but the API's `Date` type is **MM/DD/YYYY**. Net: this command has **never returned a real result**; the green test suite is a false positive because no test exercises `ReqwestPoster` against the real schema. Fix = single-query body `{"query":q, "max_results":N, ...}`, flat-array parse, `snippet` field, MM/DD/YYYY dates. Verify: contract test that asserts request body has top-level `query` (not `queries`) and parses `{"results":[{"title":...,"snippet":...}],"id":...}`.

### P0-B · Tool-alias mismatch strips all tools on non-Claude providers (research/prd/analyze)
`crates/roko-agent/src/provider/openai_compat.rs:252–260` `parse_allowed_tools_csv` collects the CSV verbatim into a `HashSet<&str>` = `{"Read","Write","Edit"}` (Claude PascalCase). The filter at **:341–349** does `allowed.contains(tool.name.as_str())` where `tool.name` is canonical **snake_case** (`read_file`/`write_file`/`edit_file`) → **zero intersection → empty tool list**. Agent runs 0 iterations, emits "I'll read the file and write…" text, writes nothing. `canonical_of_claude()` **exists** (`roko-core/src/tool/aliases.rs:113`, round-trip tested) but is **never called** in the parser. Confirmed live at HEAD. Affects **every** command that passes Claude-alias `allowed_tools` when routed to OpenAI-compat / Gemini / Ollama: `research analyze` (`Read,Write,Edit`, research.rs:683), `research topic` fallback, all four `enhance-*`, and (by the same pattern) `prd draft/edit/plan` when on a non-Claude model. On the Claude CLI path it works (Claude understands its own names natively), which is why it hid. Fix = resolve aliases in `parse_allowed_tools_csv` via `canonical_of_claude(name).unwrap_or(name)`. Verify: cross-provider test asserting a non-Claude agent given `"Read,Write,Edit"` receives ≥3 tools.

## Summary

The PRD lifecycle is the **most production-hardened authoring subsystem in roko** — far beyond what CLAUDE.md's one-line "Wired" suggests. `prd draft new` does workspace locking, role-based model selection, provider preflight, repo-context grounding packs, agent-vs-text write detection, substantive-content checks, grounding validation with `context.json`/`validation.json` sidecars, and episode persistence (commands/prd.rs:325–639). `prd plan` extracts fenced TOML from a strategist agent, applies deterministic TOML repair + field-name autocorrection, retries twice with **model-tier escalation** (haiku→sonnet→opus, prd.rs:1026–1055, 1290–1398), preserves done-task statuses when regenerating legacy plans (prd.rs:270–384), updates the PRD's `plans_generated` frontmatter (prd.rs:1962), and emits `prd:plan:generated|partial_success|failed` signals (prd.rs:1603–1651). The auto-plan event path is real and cross-process: CLI promote writes a `prd_published` audit episode to `.roko/episodes.jsonl` (prd.rs:414–447) which roko-serve polls every 500ms (routes/prds.rs:188–221) in addition to an in-process bus subscription (prds.rs:223–262), with a 60s dedupe window (prds.rs:60–74).

**The chain breaks at the last link, twice.** (1) `prd plan` prints "Next: roko plan run plans/{slug}/" (commands/prd.rs:778–780) — which defaults to the Graph Engine dry-run no-op (main.rs:1361; verified in 31-GRAPH-CELLS-ENGINE.md), so the advertised hand-off silently does nothing. (2) The serve-side auto-plan generator is a **different, weaker implementation**: it calls `runtime.run_once` with a prompt telling the agent to write `plan.md`/`tasks.toml` directly under `.roko/plans` (prds.rs:804–823, 913) — no TOML extraction, no validation/repair, no escalation, no `plans_generated` update, and it targets `.roko/plans/` while the CLI generator targets root `plans/` when it exists (plan.rs:15–22). The genuinely closed loop today is CLI-only: `prd draft promote <slug> --auto-execute` with `[prd] auto_plan = true`, which generates via the validated path and executes via **Runner v2 in-process** (prd.rs:946–967 → `crate::runner::run`) — bypassing the hollow graph default entirely. But `auto_plan` defaults to `false` (project.rs:56–61) and this repo has **no roko.toml at all**, so nothing auto-plans here.

**Usage evidence says the subsystem was exercised for one week and then abandoned**: 6 ideas (2026-05-02→05-08), 4 drafts, 1 published PRD (`dry-run-flag`, plans never generated), **0 research artifacts** (`.roko/research/INDEX.md`: "_(none)_"), 0 `prd:plan:*` signals, 0 prd episodes in the current episodes.jsonl. The two plans that were generated (May 6) sit in the **old location** `.roko/prd/plans/` with the **old schema** (`[[tasks]]`, `dependencies`, `[task_groups]`) that the current parser's modern-fields check would reject. Research commands are misdescribed by CLAUDE.md as "(Perplexity)": only `research search` hard-requires `PERPLEXITY_API_KEY` (commands/research.rs:730–731); `topic` cascades Gemini-grounding → Perplexity → Claude-CLI fallback (commands/research.rs:164, 346, 460–497), and all four `enhance-*`/`analyze` commands are plain Claude-CLI researcher agents with Read/Write/Edit.

## Lifecycle trace (stage-by-stage)

| # | Stage | Status | What actually happens | Evidence |
|---|---|---|---|---|
| 1 | **Idea capture** `prd idea` | ✅ | Appends `- <timestamp> — <text>` to `.roko/prd/ideas.md`; prints next-step hint ("roko develop … or prd draft new") | prd.rs:652–665; commands/prd.rs:308–315 |
| 1b | Idea via HTTP `POST /api/prds/ideas` | 🟡 divergent | Writes per-idea file `.roko/prd/ideas/<slug>-<uuid4>.md` with frontmatter, *plus* legacy append — two storage schemes for one concept | prds.rs:403–429 |
| 2 | **Draft** `prd draft new <title>` | ✅ | Workspace lock → scribe-role model resolution + provider preflight (commands/prd.rs:333–373) → scaffold write → `prd_agent_prompt` (PRD_SYSTEM_PROMPT + master index + PRD index + recent PRDs + ideas, prd.rs:1677–1745) → repo-context pack from slug/title keywords (commands/prd.rs:402–443) → `run_agent_capture_silent` with `allowed_tools: "none"` (agent must emit markdown as text, :459–470) → detects direct file write vs text output, materializes, deletes file if empty/failed (:480–522) → grounding-section check + `validate_prd_grounding` (duplicate-crate = error, false-negative = warning; :549–600) → `<slug>.context.json` + `<slug>.validation.json` sidecars (:574–581) → episode `prd-draft-new` + phase timing | commands/prd.rs:325–639 |
| 3 | `prd draft edit <slug>` | ✅ | Researcher-quality refine prompt (citations, mermaid, verifiable criteria); agent gets full tools; mtime-based write detection; episode `prd-draft-edit` | commands/prd.rs:640–726 |
| 4 | **Promote** `prd draft promote <slug> [--auto-execute]` | ✅ | Refuses overwrite of existing published; requires substantive content; flips `status:` only inside frontmatter; atomic write → remove draft → `prd_published` audit episode to `.roko/episodes.jsonl` → `RokoEvent::PrdPublished` on global bus → `maybe_generate_plan_after_promote` | prd.rs:822–872 |
| 5 | Auto-plan (CLI side) | ✅ gated | `auto_plan_enabled` reads `[prd] auto_plan` from roko.toml, else resolved config (**default false**, project.rs:61). On success + `--auto-execute`: `run_generated_plans` → `crate::runner::load_plans` + `crate::runner::run` = **Runner v2, real execution** | prd.rs:874–967, 969–988 |
| 6 | Auto-plan (serve side) | 🟡 | `start_prd_publish_orchestrator` in both server variants (lib.rs:336, 791) = bus subscriber (prds.rs:223–254) **+** episodes.jsonl follower @500ms for cross-process CLI publishes (prds.rs:188–221). Gate: `serve.auto_orchestrate` (default true, serve.rs:28,62) **AND** `prd.auto_plan` (default false) (prds.rs:164–168). Dedupe 60s (prds.rs:60–74,124). Then `queue_plan_generation_op` → `runtime.run_once(workdir, prompt)` — free-form agent writes files itself under `.roko/plans`; no validation, no escalation, no signals, no `plans_generated` update | prds.rs:118–130, 804–823, 893–949 |
| 6b | HTTP promote / manual plan | ✅ | `POST /api/prds/:slug/promote` (rename + audit episode + same queue, prds.rs:542–621); `POST /api/prds/:slug/plan` fires generation without publishing (prds.rs:623–645) |
| 7 | **Research enhance** `research enhance-prd <slug>` | ✅ (no key needed) | Claude-CLI researcher agent, tools `Read,Write,Edit`, updates PRD in place + saves `.roko/research/enhance-<slug>.md`; episode `research-enhance-prd`. Works on drafts or published (`find_prd`) | commands/research.rs:499–544 |
| 7b | `research topic [--deep]` | ✅ cascade | `--deep`: Perplexity `sonar-deep-research` async w/ 15s poll heartbeat (:32–161). Else Gemini grounding if `gemini.grounding_model` set (:164–344) → Perplexity if `perplexity.default_search_model` set (:346–458) → **Claude CLI fallback** where the *agent* writes `.roko/research/<slug>.md` via Write tool (:460–497). Citations appended as `## Sources` | commands/research.rs:27–498 |
| 7c | `research search` | ❌ **BROKEN** | Requires `PERPLEXITY_API_KEY` (only hard-key command), then sends a **malformed request** the real API rejects (see **P0-A**): `{"queries":[...]}` wrapper (no such key), nested-group response parse (API returns flat pages), `content` vs `snippet` field, `YYYY-MM-DD` vs `MM/DD/YYYY` dates. 20 mock tests pass; real call = 422/400. Never returned a real result | commands/research.rs:718–790; perplexity/search.rs:150,176–187 |
| 7d | `research enhance-plan/enhance-tasks/analyze/list` | 🟡 / ❌ on non-Claude | Claude agents editing plan files in place / writing execution-analysis.md; **all pass `allowed_tools: "Read,Write,Edit[,Bash,Glob]"` (Claude PascalCase) → on OpenAI/Gemini/Ollama the tool-alias bug (P0-B) yields ZERO tools → agent writes nothing.** Plus: prompt text hardcodes `.roko/plans/{plan}/…` while the existence check uses `plans_dir()` which resolves to root `plans/` in this repo (:552, :611); enhance-tasks says "assign … model_hint" (:616) which the prd-plan validator explicitly forbids | commands/research.rs:545–717,683; openai_compat.rs:252,348 |
| 8 | **Plan generation** `prd plan <slug> [--dry-run]` | ✅ | Lock → strategist model (role-based) + preflight → PRD inline (truncated 8K chars) + template guidance (`plan_template` frontmatter, prd.rs:1071–1073) + repo context → agent with `allowed_tools: "Read,Grep,Glob"` (cannot write; must emit fenced ```toml) → extract fenced toml/tasks.toml/unfenced fallback (prd.rs:1859–1960) → `validate_and_fix_generated_plan`: `repair_toml`, unknown-field autocorrect (edit-distance), required meta/task fields, `meta.plan` slug fix, model_hint prohibition (prd.rs:2181+) → ≤2 retries with tier escalation when `agent.escalation.escalate_model` (prd.rs:1290–1398; escalation reverted on auth/transport errors) → writes `<plans_root>/<slug>/tasks.toml` + `plan.md` (minimal plan.md synthesized if absent) via atomic write (prd.rs:1400–1429) → `update_prd_plans_generated` frontmatter (prd.rs:1962–2001) → legacy-plan regeneration sweep (prd.rs:1486–1498) → `plan_validate` over plans root → signal `prd:plan:*` via FileSubstrate (prd.rs:1603–1651) → episode `prd-plan-generate`. `plans_root` = root `plans/` if it exists, else `.roko/plans/` (plan.rs:15–22) | commands/prd.rs:745–782; prd.rs:1057–1652 |
| 9 | **Consolidate** `prd consolidate` | 🟡 | Strategist agent over first-50-lines of every PRD + ideas.md; asked to report duplicates/gaps/inconsistencies/stale/promotable AND "create new drafts" — but output is only printed; no artifact contract, no validation of anything it writes | commands/prd.rs:783–838 |
| 10 | **Execute** `plan run plans/` | ❌ by default | Default `--engine graph` = TaskExecutorCell dry-run no-op, no lock/episodes/snapshot (see 31-GRAPH-CELLS-ENGINE.md:9). Real: `--engine runner-v2`, or the promote `--auto-execute` path which calls Runner v2 directly | main.rs:1361; prd.rs:946–967 |
| 11 | Index refresh | ✅ | Every `prd`/`research`/`plan` command triggers `index::rebuild_all` → `.roko/prd/INDEX.md`, `.roko/research/INDEX.md`, `.roko/plans/INDEX.md` (executable vs non-executable sections from d38043d66) | main.rs:2423–2440; index.rs:46–118, 390 |

## Current state table

| Component | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| Idea capture (CLI) | CLAUDE.md self-hosting step 1 | `cmd_idea` | ✅ | prd.rs:652; live: 6 ideas May 2–8 |
| Idea capture (HTTP) | serve routes | `post_idea` | 🟡 divergent storage (`ideas/` dir + uuid slug vs ideas.md) | prds.rs:403–429 |
| Draft agent-write + grounding validation | tmp/ux-followup §14 (markers "§14.2/14.4/14.6" in code) | `PrdDraftCmd::New` + `validate_prd_grounding` | ✅ | commands/prd.rs:325–639, 150–228; sidecars exist for costs/cursor-composer/dry-run-flag |
| PRD quality system prompt | bardo PRD corpus | `PRD_SYSTEM_PROMPT` | ✅ but 🕰️ contaminated: cites "Grimoire", "Golem", "Heartbeat", "PAD vectors and somatic markers" as the quality bar | prd_prompt.rs:16–17, 193–195 |
| Promote + publish event + audit | §14.2/14.4 | `cmd_promote` | ✅ | prd.rs:822–872 |
| `prd.auto_plan` config | CLAUDE.md item 10 | `PrdConfig` | ✅ wired, **default false**; repo has no roko.toml → inactive here | project.rs:53–61; prd.rs:969–988 |
| prd_publish_subscriber | CLAUDE.md "PRD auto-plan trigger" | bus subscriber + episodes poller | ✅ started in both serve variants; 🟡 generation quality (see next row) | lib.rs:336,791; prds.rs:188–262 |
| Serve-side plan generation | — | `queue_plan_generation_op` → `runtime.run_once` | 🟡 unvalidated free-write; targets `.roko/plans` even when `plans/` is canonical; no plans_generated/signal | prds.rs:804–823, 893–949 |
| `has_plan` in `GET /api/prds` | — | `has_plan_for_slug` | 🕰️ checks flat `.roko/plans/<slug>.{json,toml}` — directory-layout plans never match → always false | prds.rs:291–303 |
| Plan generation (CLI) w/ repair + escalation | CLAUDE.md item "Plan generation from PRD" | `generate_plan_from_prd_with_outcome` | ✅ | prd.rs:1057–1652 |
| Model escalation chain | — | `next_tier_model` (haiku-4-5→sonnet-4-6→opus-4-6, or `agent.tier_models`) | ✅ | prd.rs:1026–1055 |
| Legacy plan regeneration | d38043d66 "plan lifecycle statuses" | `regenerate_old_format_plans` + `preserve_completed_task_status` | ✅ (invoked after every non-dry `prd plan`) | prd.rs:246–400, 1486–1498 |
| plans_generated frontmatter update | — | `update_prd_plans_generated` | ✅ | prd.rs:1962–2001; **but** published dry-run-flag.md still `[]` — predates wiring |
| Plan output location | — | root `plans/` preferred, `.roko/plans/` fallback | ✅ CLI / ❌ serve (`.roko/plans` hardcoded) | plan.rs:15–22 vs prds.rs:809 |
| `prd status` coverage report | CLAUDE.md "coverage report" | `cmd_status` | 🟡 global plan/task/done totals only; per-PRD columns all "—"; `coverage` frontmatter never computed | prd.rs:753–819 |
| `prd consolidate` | CLAUDE.md | agent analysis | 🟡 print-only, no artifact validation | commands/prd.rs:783–838 |
| PRD action hints | dc6b053e6 "show PRD action hints" | `cmd_list` Actions section (slash-command style for ACP) | ✅ | prd.rs:718–747 |
| Research topic (3-backend cascade) | CLAUDE.md "(Perplexity)" | Gemini→Perplexity→Claude | ✅; CLAUDE.md attribution misleading | commands/research.rs:27–498 |
| Research deep (sonar-deep-research) | — | async polling agent | ✅ needs `PERPLEXITY_API_KEY` at dispatch | commands/research.rs:32–161, 812–888 |
| Research search | — | `PerplexitySearchClient` | ❌ **broken vs real API** (P0-A): `queries` wrapper + nested-group parse + `content`/`snippet` mismatch; mock tests mask it | commands/research.rs:718–790; perplexity/search.rs:150,176 |
| enhance-prd/plan/tasks/analyze | CLAUDE.md | Claude researcher agents | ✅ on Claude / ❌ on non-Claude (P0-B tool-alias strips tools) / 🟡 path+model_hint contradictions | commands/research.rs:499–717,683; openai_compat.rs:252,348 |
| Tool-alias resolution for agent dispatch | roko-core aliases | `parse_allowed_tools_csv` (openai_compat) | ❌ **P0-B**: doesn't call `canonical_of_claude`; PascalCase set vs snake_case `tool.name` → empty tools on all non-Claude providers | openai_compat.rs:252–260,341–349; aliases.rs:113 |
| Research artifact store + semantic search | — | `save_research_with_citations/grounding`, `build_research_index`, `search_research` (embeddings + cosine) | 🔌 index/search built, no CLI subcommand calls `search_research` (only `list`) | research.rs:37–120, 417–507, 653–712 |
| Research INDEX | — | `rebuild_research_index` | ✅ (rebuilt Jul 6) | index.rs:389–484 |
| PRD INDEX | — | `rebuild_prd_index` | ✅ | index.rs:45–118 |
| Tests | — | prd.rs 63 unit; prd_pipeline.rs 9 (no-LLM helpers); prd_pipeline_workspace.rs 2 (mock-dispatcher fixtures, `--repo` isolation incl. write-then-fail draft); serve prds.rs 6; research.rs 15 | ✅ unit/integration | tests/ census via grep |
| E2E self-host proof | e2e_self_host.rs (idea→draft→enhance→promote(auto-plan)→validate→run→episodes/snapshot/gates asserts) | `#[ignore = "…needs ROKO_DISPATCHER fixture; run manually"]` — and its `plan run plans` step would hit the graph default which writes no `executor.json` it asserts | ❌ not CI-verified | e2e_self_host.rs:14–130 |

## Real-data evidence (`.roko/`)

- **Ideas**: 6, all 2026-05-02 → 2026-05-08 (`.roko/prd/ideas.md`). Themes: --dry-run flag, Cursor ACP backend ×4, cost accounting.
- **Drafts**: 4 — `costs` (22.7KB, May 8, full context+validation sidecars), `cursor-composer-backend` (4.8KB, May 6, sidecars), `self-developing-workflow` (9KB, May 6, no sidecars/frontmatter date), `test-quick` (271B stub). Plus **orphaned sidecars** `dry-run-flag.context.json`/`.validation.json` — promote moves only the `.md` (prd.rs:824–856), leaving sidecars stranded in drafts/.
- **Published**: 1 — `dry-run-flag.md` (22.8KB, May 2), `plans_generated` still empty → published but never planned.
- **Generated plans**: 2 under **legacy location** `.roko/prd/plans/{cursor-composer-backend,self-developing-workflow}/tasks.toml` (May 6) in **legacy schema** (`[meta] slug=`, `[[tasks]]`, `dependencies`, `[task_groups]`) — unparseable as modern `[[task]]`/`depends_on` format; invisible to today's `plans_dir()` resolution because root `plans/` exists.
- **Research**: **zero artifacts ever** — `.roko/research/` contains only the auto-generated INDEX.md ("_(none)_", rebuilt 2026-07-06).
- **Signals/episodes**: 0 `prd:plan:*` signals (signals.jsonl = 467 GateVerdict only); 0 prd episodes in current episodes.jsonl (27 entries). May activity predates current logs.
- **Root `plans/`**: ~30 dirs (P08–P34 + arch plans) — authored via `plan generate`/hand, not via `prd plan` (no slug matches any PRD).
- Conclusion: PRD tooling saw one real week of use (May 2–8, matching the `d38043d66`/`7c9f096c5` stabilization burst); research CLI has never produced an artifact in this workspace; INDEXes ticking on Jul 6 show only incidental command runs.

## Self-hosting verdict (exact working command path today)

Roko **can** self-host PRD→plan→execution today, but only on this exact CLI path with a configured provider (Claude CLI default) and — for full autonomy — a `roko.toml` containing `[prd] auto_plan = true`:

```bash
roko prd idea "…"                                  # ✅ always works (no LLM)
roko prd draft new "Title"                          # ✅ agent-written, validated draft
roko research enhance-prd <slug>                    # ✅ optional; Claude backend, no Perplexity key needed
roko prd draft promote <slug> --auto-execute        # ✅ THE closed loop: validated plan gen + Runner v2 execution (prd.rs:870,946–967)
# — or manually —
roko prd plan <slug>                                # ✅ writes plans/<slug>/{tasks.toml,plan.md}
roko plan run plans/ --engine runner-v2             # ✅ real execution (REQUIRED flag)
```

**Hollow/broken variants**: `roko plan run plans/` (default graph = dry-run no-op, prints SUCCESS); the hint `prd plan` itself prints (commands/prd.rs:778–780) routes into that no-op; `roko resume` (hardcoded graph engine, main.rs:2699); serve auto-plan (unvalidated output into `.roko/plans/`, which subsequent `plans_dir()` resolution ignores when root `plans/` exists — generated plans can be **orphaned in a directory nothing reads**). In this repo specifically, auto-plan is inert (no roko.toml, default false).

## V2-aligned

- Role-based model selection (`scribe`/`strategist`/`researcher`) + provider preflight per command (commands/prd.rs:359–373, 750–764).
- Typed artifact outcomes: `GenerationOutcome`/`ArtifactValidationReport` from `roko_learn::runtime_feedback`, projected into `ArtifactOutcome` (prd.rs:42–119) — validation-as-data, sidecar-persisted.
- Event-driven publish: bus emit + durable audit episode + cross-process poller — a working two-transport event pattern (prd.rs:858–869; prds.rs:188–262).
- Signals into the Engram substrate (`FileSubstrate.put`, `Provenance::trusted("roko.prd")`, prd.rs:402–412).
- Deterministic repair before LLM retry, and escalation that distinguishes format failures (escalate) from auth/transport failures (don't) — prd.rs:1374–1396.
- `--dry-run` uses a copy-on-write `DryRunWorkspace` temp overlay (prd.rs:1076–1083).
- Promote `--auto-execute` drives Runner v2 (the real v2 engine) directly rather than shelling to the CLI (prd.rs:946–967).
- Auto-index rebuild after every prd/research/plan command (main.rs:2423–2440).

## Old paradigm & tech debt

- 🕰️ **Bardo vocabulary in the PRD system prompt**: quality bar tells agents to study PRDs demonstrating "30+ academic citations with PAD vectors and somatic markers", "Grimoire", "Golem", "Heartbeat" (prd_prompt.rs:16–17, 193–195) — copied from the Mori/bardo corpus; misleading on any non-bardo workspace.
- **Two plan generators**: CLI validated/escalating/fenced-output path (prd.rs:1057) vs serve free-write `run_once` path (prds.rs:893) with different prompts, different output roots (`plans/` vs `.roko/plans/`), and no shared validation. Serve's also re-instructs `mcp_servers` per task (prds.rs:815) which the CLI validator discourages.
- **Stale serve helpers**: `has_plan_for_slug` checks flat `<slug>.json|.toml` files (prds.rs:291–303); coverage endpoint counts `.roko/plans` entries only (prds.rs:652).
- **Legacy artifacts in tree**: `.roko/prd/plans/**` (old location + `[[tasks]]` schema) — the regeneration sweep only scans the *current* plans root, so these are permanently orphaned.
- **Prompt/path drift in research**: enhance-plan/enhance-tasks prompts hardcode `.roko/plans/{plan}` while resolution uses `plans_dir()` (root `plans/` here) — commands/research.rs:552, 611; enhance-tasks tells the agent to add `model_hint` (:616) which `validate_and_fix_generated_plan` and the regeneration prompt forbid (prd.rs:1174, 299).
- **Promote leaves sidecars behind** (observed for dry-run-flag) — drafts/ accumulates `.context.json`/`.validation.json` for published PRDs.
- **Coverage is decorative**: `coverage:` frontmatter never computed; `prd status` per-PRD row is all dashes (prd.rs:795–801); INDEX Plans/Coverage columns come straight from (stale) frontmatter.
- **Duplicated validation code**: `check_grounding_section`/`validate_prd_grounding` exist in both prd.rs (:2770, :2803) and commands/prd.rs (:25, :150) — the parallel-development duplication CLAUDE.md warns about, inside the PRD module itself.
- **ideas.md vs ideas/ dir** double storage (CLI vs HTTP).
- **Next-step hints route into the hollow engine** (commands/prd.rs:778; also `cmd_promote` output implies readiness).

## Not implemented

- Research → planning coupling: `prd plan` never reads `.roko/research/**`; `enhance-prd` must be run manually before plan; no freshness/staleness tracking of research per PRD.
- Per-PRD coverage computation (PRD ↔ plan ↔ task done-ratio join); `prd status` plan columns.
- `research search`/semantic index surfaced in CLI: `build_research_index`/`search_research` (research.rs:653–712) have no subcommand (`research list` only lists files).
- Consolidate as a real pipeline: no draft-merge/dedupe artifacts, no INDEX mutation, no validation of what the agent claims to have created.
- Serve-side generation parity: validation, escalation, signals, `plans_generated`, correct plans root.
- CI-run E2E: `e2e_self_host.rs` is `#[ignore]` and its assertions (executor.json, agent_turn episodes, gate_verdict engrams) are incompatible with the current `plan run` graph default anyway.
- Draft→publish watcher outside serve (no `roko` daemonless auto-plan; CLI promote is the only non-serve trigger).
- Idea→draft automation (`prd draft new` takes a title, not an idea reference; ideas are never consumed/marked-done).

## Migration checklist

- [ ] **[P0-A]** Rewrite `PerplexitySearchClient::search_batch` to the real API contract: top-level `{"query":q,"max_results":N,...}` body (drop the `queries` wrapper), flat `results[]`-of-pages parse, `snippet` field, `MM/DD/YYYY` dates. Add a contract test that does NOT use the echo `MockPoster` (assert request body top-level key is `query`; parse `{"results":[{"title","url","snippet"}],"id"}`) — verify: `cargo test -p roko-agent perplexity_search` includes a real-schema fixture and `roko research search "rust async"` returns results with a live key.
- [ ] **[P0-B]** Resolve Claude aliases in `parse_allowed_tools_csv` (`openai_compat.rs:252`) via `roko_core::tool::aliases::canonical_of_claude(name).unwrap_or(name)` so `Read,Write,Edit` map to `read_file,write_file,edit_file` — verify: a cross-provider unit test that a non-Claude agent given `"Read,Write,Edit"` gets ≥3 tools, and `roko research analyze` on gpt-* actually writes `.roko/research/execution-analysis.md`.
- [ ] **[P0]** Fix the post-`prd plan` hint and self-hosting docs to a working invocation (either flip `plan run` default to runner-v2 or print `--engine runner-v2`) — verify: `cargo run -p roko-cli -- prd plan <slug> 2>&1 | grep 'Next:'` shows a command that actually executes agents
- [ ] **[P0]** Unify serve auto-plan onto the validated CLI generator (`generate_plan_from_prd_with_outcome` or an extracted lib fn) and write to `plans_dir()` not hardcoded `.roko/plans` — verify: publish via `POST /api/prds/:slug/promote` with `auto_plan=true` produces `plans/<slug>/tasks.toml` that passes `roko plan validate plans/`
- [ ] **[P0]** Add a `roko.toml` to this repo (or ship `roko init` defaults) with `[prd] auto_plan` decided explicitly — verify: `roko prd draft promote <slug>` prints either plan generation or a "auto_plan disabled" notice
- [ ] **[P1]** Un-ignore/rework `e2e_self_host.rs` with the mock-dispatcher fixture in CI, pinning `--engine runner-v2` — verify: `cargo test -p roko-cli --test e2e_self_host` passes in CI
- [ ] **[P1]** Fix enhance-plan/enhance-tasks prompts to use the resolved plans path and drop the `model_hint` instruction — verify: `grep -n '.roko/plans/{plan}\|model_hint' crates/roko-cli/src/commands/research.rs` returns nothing
- [ ] **[P1]** Migrate or delete legacy `.roko/prd/plans/**`; extend `regenerate_old_format_plans` (or a `prd migrate` subcommand) to sweep the legacy root — verify: `find .roko/prd/plans -name tasks.toml` empty or parseable by `TasksFile::parse`
- [ ] **[P1]** Fix serve `has_plan_for_slug` + coverage to the directory layout and `plans_dir()` — verify: `curl :6677/api/prds` shows `has_plan:true` for a slug with `plans/<slug>/tasks.toml`
- [ ] **[P2]** Move sidecars on promote (or write them to a slug-keyed dir) — verify: after promote, `ls .roko/prd/drafts/<slug>.*` is empty
- [ ] **[P2]** Replace bardo vocabulary/quality-bar in `PRD_SYSTEM_PROMPT` with roko-native references — verify: `grep -in 'grimoire\|golem\|heartbeat\|somatic' crates/roko-cli/src/prd_prompt.rs` empty
- [ ] **[P2]** Compute real coverage: join `plans_generated` → tasks.toml statuses in `prd status` and INDEX — verify: `roko prd status` shows non-dash Plans/Tasks/Done for a planned PRD
- [ ] **[P2]** Deduplicate `validate_prd_grounding`/`check_grounding_section` (single module) — verify: `grep -rn 'fn validate_prd_grounding' crates/ | wc -l` = 1
- [ ] **[P3]** Unify idea storage (per-file with index, consumed by `draft new <idea-ref>`) — verify: `roko prd idea` + HTTP idea land in the same store
- [ ] **[P3]** Expose `search_research` as `roko research find "<query>"` and feed top-k into `prd plan` prompt — verify: plan prompt contains research excerpts when artifacts exist
- [ ] **[P3]** Give `prd consolidate` an artifact contract (report file + validated draft creation) — verify: consolidate writes `.roko/prd/consolidation-<date>.md`

## tmp-feedback/2 disposition (re-verified 2026-07-08)

Cross-checked the six PRD/research items from `tmp/tmp-feedback/2` (dated 2026-05-08) against HEAD:

| # | Issue | Disposition | Evidence |
|---|---|---|---|
| 13 | `/search` Perplexity format mismatch | ✅ **CONFIRMED still broken** (P0-A) — three-way contract violation, worse than the original one-line claim | perplexity/search.rs:150,176; live API OpenAPI |
| 14 | `prd list` missing slugs | ✅ **FIXED** — `cmd_list` prints `slug:` for published (prd.rs:683) and drafts (prd.rs:695); no regression test though | prd.rs:668–697 |
| 15 | `analyze` no tools dispatched | ✅ **CONFIRMED still broken** (P0-B) — `parse_allowed_tools_csv` still no alias resolution; `canonical_of_claude` exists but uncalled | openai_compat.rs:252,348; aliases.rs:113 |
| 16 | `prd draft` broken/redundant | 🟡 **partly stale**: `allowed_tools:"none"` (prd.rs:468) is by design (agent emits markdown as text, materialized after) — not a bug; but idea↔draft linkage still absent (`draft new` takes free-text title, never consults ideas.md) and validation-failure is still non-blocking (draft written regardless). The `legacy-runner-v2` feature-flag it references is gone (now `--engine` value-enum). | prd.rs:459–522 |
| 17 | `prd status` disconnected | ✅ **CONFIRMED**: per-PRD Plans/Tasks/Done are hardcoded `"—"` (prd.rs:799); only global totals computed (prd.rs:774–793); no slug↔plan join | prd.rs:753–819 |
| 21 | Plan-generation bugs | 🟡 **partly stale/partly live**: graph-default no-op **confirmed live** (main.rs:1361 `default_value="graph"`; `resume` hardcodes Graph at main.rs:2699) — this is P0. But CLI `prd plan` **does** validate/repair TOML (contra "no validation" — that critique was about `plan generate`, not `prd plan`). "double-generate" via `develop→do_cmd` not re-verified here (out of PRD scope). | main.rs:1361,2699; prd.rs:2181+ |

## Open questions

1. **Is the serve-side free-write generator intentional** (agent-with-file-tools philosophy) or just pre-dating the CLI's fenced-output+validation design? They will drift further apart with every fix applied to only one.
2. **Which plans root is canonical** — top-level `plans/` (CLI, gitignored since f6938565b) or `.roko/plans/` (serve, docs)? Three consumers currently disagree (plan.rs:15, prds.rs:809, research prompts).
3. Should `prd.auto_plan` remain default-false while CLAUDE.md advertises the auto-plan trigger as "Wired"? Wired-but-off is indistinguishable from broken for a new user.
4. Is the May-6 `.roko/prd/plans` schema (`[[tasks]]`, task_groups) an abandoned generator revision worth a migration, or safe to delete? (`preserve_completed_task_status` suggests someone cared about not losing done-status.)
5. `research topic` prefers Gemini grounding over Perplexity when both configured (commands/research.rs:164 before :346) — deliberate ranking, or accident of ordering? CLAUDE.md says Perplexity.
6. Ideas are captured but never consumed — is idea→draft matching (the `self-developing-workflow` draft PRD describes exactly this: note clustering, synthesis) the intended next PRD to execute as a self-hosting dogfood?

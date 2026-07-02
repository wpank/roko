# PRD-06 — Builtin Workflow Catalog

**Status**: Draft
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-25
**Crate**: `roko-builtin-*` family (new crates per category)
**Prerequisites**: PRD-00, PRD-02, PRD-03, PRD-04, PRD-05

---

## 0. Scope

This document inventories the Workflows and Modules that ship with roko in v1. The catalog is large by design — the synergy thesis is "more lego pieces = more value" — but every Workflow is small in concept. Each is a few-Module composition with a focused purpose, a small macro surface, and one or two slots where users plug in their own pieces.

This is not implementation order. Implementation priority is in PRD-12 §X. This is the catalog the system will eventually ship with.

Naming convention: kebab-case noun-or-noun-phrase. Workflows describe outcomes; Modules describe operations.

---

## 1. Catalog Categories

```
01. Authoring          Doc / PRD / plan / spec creation and refinement
02. Verification       Audit, gate, review, fact-check, completeness
03. Research           Web, academic, citation, knowledge import
04. Execution          Plan run, refactor, test, build
05. Deploy             Multi-target deployment with smoke tests and rollback
06. Operations         Watch, cron, backup, GC, sync, archive
07. Maintenance        Doc-update, doc-1to1, dead-code, deps, costs
08. Code Intelligence  Index, search, navigate, type-check
09. Knowledge          Ingest, link, decay, dream-cycle, gc, migrate
10. Observation        Endpoint-health, page-health, perf-regression, alert
11. Communication      Slack-summary, GitHub-comment, email-digest
12. Workflow Meta      Workflow-author, workflow-test, workflow-publish
```

---

## 2. Authoring

### `doc-ingest` — directory of mixed markdown → PRDs + plans + tasks

The flagship workflow described in PRD-07. Walks a directory, segments and classifies markdown, clusters into PRDs, synthesizes, optionally enriches with web research, audits, refines, generates plans, persists. Macros: `enable_audit`, `enable_web_research`, `max_refine_iterations`, `synthesizer_model`, `cluster_granularity`, `budget_usd`. Slot: `researcher`.

### `prd-draft` — capture an idea → published PRD

Replaces `roko prd idea` + `prd draft new` + `prd draft promote`. A single Workflow with three states (idea, draft, published). User can re-enter at any state. Modules: `prd-idea-capture`, `prd-draft-synthesize`, `prd-publish-validate`. Macros: `auto_advance`, `model`. Slot: none.

### `prd-enrich` — flesh out an existing PRD with research

Replaces `research enhance-prd`. Reads a PRD, identifies gaps, runs research, weaves in citations, returns refined PRD. Macros: `research_depth`, `max_citations`, `prefer_academic`. Slot: `researcher`.

### `prd-plan` — PRD → tasks.toml plan

Replaces `prd plan`. Reads a published PRD, produces a structured plan with task graph. Macros: `plan_template` (default | strict | compact), `model`. Slot: none.

### `plan-validate` — lint a plan without running it

Replaces `plan validate`. Static analysis on tasks.toml: cycles, missing dependencies, orphan tasks, ambiguous acceptance criteria. Macros: `severity_cutoff`. Slot: none.

### `plan-iterate` — refine a plan from feedback

User provides feedback or audit findings; workflow regenerates plan preserving completed work. Macros: `preserve_done`, `model`. Slot: none.

### `idea-fragment-collect` — sweep notes / Slack / issues for ideas

Pulls from configured sources (markdown notes, Slack channel via trigger, GitHub issues) and produces a deduplicated idea list ready for `prd-draft` chaining. Macros: `dedupe_threshold`, `since`. Slots: `slack-source`, `github-source`, `note-source`.

### `spec-from-conversation` — Slack/discussion → PRD

Reads a long discussion thread, produces a structured spec. Macro: `format` (prd | rfc | adr). Slot: `transcript-source`.

---

## 3. Verification

Visual-gate2's evaluation framework lives here. Every Profile is a Workflow.

### `visual-gate` — UI quality evaluation

Replaces the existing `vision_loop/`. Captures screenshots, runs criteria (saliency, hierarchy, accessibility, brand-consistency, regression-against-baseline), produces a Verdict. Macros: `profile` (preset | custom), `viewport`, `compare_to`. Slots: `criteria` (multi).

### `endpoint-audit` — automated endpoint health check

Walks an OpenAPI / route manifest, fires a request to each endpoint, captures status / shape / timing / errors. Produces a per-endpoint scorecard and an aggregate Verdict. Macros: `concurrency`, `timeout_ms`, `auth_strategy`. Slot: `auth-provider`.

### `page-audit` — automated dashboard page check

Headless browser walks all pages of a UI, captures screenshots + console + network, runs `visual-gate` and accessibility checks per page. Used both as automation (cron) and as a manual sweep. Macros: `pages` (all | list), `viewport`, `auth_strategy`. Slot: `auth-provider`.

### `code-review` — automated PR review

Reads a diff, runs criteria (style, security, perf, complexity, test-coverage-delta, breaking-API). Posts findings as PR comments via GitHub trigger response. Macros: `strictness`, `focus_areas` (multi-enum: security, perf, style, complexity, tests). Slot: `reviewer-agent`.

### `security-audit` — code/infra security pass

Static analysis (semgrep, cargo-audit, npm-audit), dependency CVE scan, SBOM extraction, secret detection, IaC review. Macros: `scope` (changed | full | repo), `severity_cutoff`. Slot: `custom-scanner` (multi).

### `doc-completeness` — PRD coverage check

Compares PRDs to an inferred required spec (e.g., "every public API method has a corresponding section"). Reports gaps. Macros: `coverage_target` (api | architecture | ux), `required_sections`. Slot: `coverage-strategy`.

### `doc-1to1` — docs match codebase

Walks docs claiming to describe code (file paths, function names, types) and verifies each claim against the current code. Reports stale references. Macros: `code_root`, `doc_root`, `staleness_threshold`. Slot: none.

### `prd-audit` — PRD self-audit

Reads a PRD, looks for: contradictions, vague language, missing acceptance criteria, unsupported claims, broken references. Used inside `doc-ingest` as the audit pass. Macros: `severity_cutoff`. Slot: `auditor-agent`.

### `citation-check` — verify citations are real and accurate

For every citation in a doc, fetch the source, confirm the cited claim is supported. Flags hallucinated citations (a critical step for marketplace doc artifacts). Macros: `strictness`, `cache_ttl_h`. Slot: `fetcher`.

### `fact-check` — check claims against a corpus

Given a doc and a corpus (knowledge store, web), check every factual claim for support. Reports unsupported claims with confidence. Macros: `corpus_scope`, `min_confidence`. Slot: `corpus-source`.

### `regression-detect` — compare new vs baseline artifact

Generic regression: visual, performance, output-shape. Macros: `baseline_ref`, `dimension` (visual | perf | output), `tolerance`. Slot: `comparator`.

---

## 4. Research

### `research-sweep` — multi-source deep-research on a topic

Web + academic + knowledge-store, parallel sources, synthesized into a research artifact with citations. Replaces `research topic`. Macros: `depth` (quick | standard | deep), `sources` (multi), `prefer_academic`, `budget_usd`. Slots: `web-source`, `academic-source`.

### `web-enrich` — add web context to a doc

Lighter than `research-sweep`; runs a few targeted queries given a doc's identified gaps. Macros: `max_queries`, `recency`. Slot: `web-source`.

### `academic-search` — arXiv + Semantic Scholar + Google Scholar

Specialized academic-paper search with citation graph traversal. Macros: `since`, `field`, `max_papers`, `cluster`. Slot: `academic-source`.

### `knowledge-import` — pull from external knowledge graph

Notion, Confluence, custom MD repo, Are.na — into roko's knowledge store with provenance. Macros: `dedupe`, `tag_strategy`. Slot: `source`.

### `competitive-monitor` — periodic check of competitor product

Scrape, screenshot, diff against last snapshot, surface meaningful changes. Macros: `comparison_axes`, `alert_threshold`. Slot: `target-source`.

---

## 5. Execution

### `plan-execute` — run a plan's task DAG

Replaces `plan run`. Wraps existing task DAG executor as a Workflow. Macros: `concurrency`, `dispatch_strategy`, `gate_profile`. Slots: `agent-pool`, `gate-pipeline`.

### `task-execute` — run a single task

Replaces single-task dispatch. Used internally by `plan-execute`. Macros: `model`, `role`, `effort`. Slot: `gate-pipeline`.

### `refactor-batch` — apply a refactor pattern across files

Given a refactor description and a target directory, agent applies the change file-by-file, runs gates per file, commits in a branch. Macros: `pattern`, `target_paths`, `commit_strategy`, `branch_prefix`. Slot: `gate-pipeline`.

### `test-run` — run a test suite, parse results

Wraps `cargo test`, `npm test`, `pytest`, `jest`. Macros: `target` (all | changed | tagged), `parallelism`. Slot: `runner`.

### `build` — compile / bundle artifact

`cargo build`, `vite build`, `webpack`, `gradle`. Macros: `profile` (debug | release), `target`. Slot: `runner`.

### `script-run` — execute an arbitrary script with capability gating

Generic shell-runner Workflow for use in triggers ("when X happens, run my script"). Macros: `working_dir`, `env`, `timeout`. Slot: `script` (the script itself is a slot filling).

---

## 6. Deploy

### `deploy` — generic deploy Workflow

Reads workspace deploy targets, executes the chosen target via target-specific Module, runs post-deploy checks, optionally rolls back on failure. Macros: `target` (workspace target name), `dry_run`, `skip_smoke_test`, `rollback_on_failure`. Slots: `pre-deploy-check`, `post-deploy-check`, `smoke-test`.

### `deploy-railway` — Railway target Module

Wrapped by `deploy`. Builds, pushes to Railway, awaits service health, returns deployment metadata. Capabilities: `net.railway`, `secrets.railway_token`, `shell`.

### `deploy-fly` — Fly.io target Module

`flyctl deploy` wrapper with health check and rollback hook. Capabilities: `net.fly`, `secrets.fly_api_token`, `shell`.

### `deploy-vercel` — Vercel target Module

Build + `vercel deploy`. Capabilities: `net.vercel`, `secrets.vercel_token`, `shell`.

### `deploy-gcp` — GCP target Module

Cloud Run / GKE deploy. Capabilities: `net.gcp`, `secrets.gcp_sa_key`, `shell`.

### `deploy-ovh` — OVH target Module

Public Cloud / Bare Metal deploy. Capabilities: `net.ovh`, `secrets.ovh_credentials`, `shell`.

### `deploy-shell` — generic shell-script deploy

For custom infra. User provides a shell script; capability sandbox surrounds it. Capabilities: as declared.

### `smoke-test` — post-deploy verification Workflow

Runs `endpoint-audit` + `page-audit` against the freshly deployed target. Macros: `target_url`, `critical_paths`, `timeout_s`. Slot: `auth-provider`.

### `rollback` — revert a failed deploy

Calls target-specific rollback path. Capabilities match the deploy target.

### `release` — version bump + tag + changelog + deploy + announce

Composite: bumps version files, updates changelog, creates git tag, runs `deploy`, posts to Slack/Discord/email. Macros: `bump_kind` (patch | minor | major), `targets`, `announce_channels`. Slots: `versioner`, `changelog-generator`, `announcer`.

### `canary` — staged rollout

Deploy to a fraction (e.g., 10%), monitor, expand on healthy signals. Macros: `stages`, `bake_time_per_stage_s`, `metrics_to_watch`. Slot: `health-source`.

---

## 7. Operations

### `watch` — file/folder watcher → arbitrary action

Trigger primitive wrapped as a workflow for declarative configuration. Maps a watched path through a filter to a target Workflow invocation. (Most users author triggers directly; `watch` exists for the visual editor to render watch-as-workflow.)

### `cron-tick` — workflow that fires another workflow on schedule

Same — used by visual editor and as a composable building block.

### `backup` — workspace state to remote

Snapshot `.roko/`, encrypt, push to S3 / Tigris / configured remote. Macros: `target`, `encryption`, `retention_days`. Slot: `remote`.

### `restore` — restore from backup

Inverse of `backup`. Macros: `snapshot_id`, `target_workspace`. Slot: `remote`.

### `gc` — garbage collect old runs / artifacts / episodes

Configurable retention. Macros: `keep_runs`, `keep_artifacts_days`, `keep_episodes_days`. Slot: none.

### `sync` — peer sync of knowledge / artifacts

Mesh sync between workspaces or machines. Replaces / extends `knowledge sync`. Macros: `peer`, `direction` (push | pull | bidirectional), `filter`. Slot: `peer-channel`.

### `archive` — cold storage of old artifacts

Move artifacts older than threshold to cold tier (Glacier-class). Macros: `older_than_days`, `tier`. Slot: `cold-store`.

### `cost-report` — periodic cost analysis

Generates per-workflow / per-agent / per-model cost summary, posts to dashboard + optional channel. Macros: `period` (day | week | month), `breakdown_by`. Slot: `notifier`.

### `dependency-update` — bump deps + run tests

`cargo update`, `npm update`, `pip-compile`. Runs gate pipeline; opens PR if green. Macros: `scope` (patch | minor | major), `runner`. Slot: `gate-profile`.

### `dependency-audit` — check deps for CVEs / abandonment

`cargo audit`, `npm audit`, scans for unmaintained deps. Macros: `severity_cutoff`. Slot: none.

---

## 8. Maintenance

### `doc-update` — keep docs in sync after a code change

Triggered on code change; identifies docs likely affected (via `doc-1to1` reverse-lookup); proposes patches; opens PR. Macros: `scope`, `auto_open_pr`. Slot: `editor-agent`.

### `dead-code-cleanup` — find and remove unused code

Static analysis + import-graph; agent removes safely. Macros: `dry_run`, `confidence_min`. Slot: `analyzer`.

### `lint-fix` — auto-fix linter findings

Runs linter, applies safe auto-fixes, reports unfixable. Macros: `linter` (clippy | eslint | etc), `aggressive`. Slot: `runner`.

### `format-all` — format the codebase

`cargo fmt`, `prettier`, `black`. Macros: `paths`. Slot: `formatter`.

### `migration-runner` — apply a database / schema / API migration

Reads migration files, runs in dependency order, validates pre/post invariants, supports rollback. Macros: `target_version`, `dry_run`. Slot: `runner`.

### `cleanup-orphans` — find files / artifacts / branches not referenced

Sweeps the workspace for orphaned objects. Macros: `kinds` (files | branches | artifacts), `older_than`. Slot: none.

### `i18n-extract` — extract translatable strings

Walks UI code, extracts strings, produces / updates translation files. Macros: `formats` (po | json | csv), `locales`. Slot: none.

---

## 9. Code Intelligence

### `index-build` — build the code-intel index

Wraps `roko-mcp-code` index build. Macros: `paths`, `incremental`. Slot: none.

### `code-search` — semantic + symbolic search

Wraps the existing index. Macros: `kinds`, `top_k`. Slot: none.

### `type-check` — language-specific type checker

Cargo check, tsc, mypy. Macros: `target`, `strict`. Slot: `checker`.

### `symbol-graph` — build a symbol-relationship graph

Used by other Workflows for impact analysis. Macros: `language`, `scope`. Slot: none.

### `impact-analysis` — what does this change affect?

Given a diff, walks symbol graph + tests + docs to report impact. Macros: `depth`, `include_docs`, `include_tests`. Slot: `change-source`.

---

## 10. Knowledge

### `knowledge-ingest` — generic doc/code/research → knowledge store

Macros: `kind`, `tags`, `source_lineage`. Slot: `embedder`, `chunker`.

### `knowledge-link` — discover cross-domain bridges

HDC fingerprint similarity + LLM bridge generation. Wraps existing resonance. Macros: `min_similarity`, `top_k`. Slot: none.

### `knowledge-decay` — apply Ebbinghaus decay to old entries

Macros: `decay_rate_factor`. Slot: none.

### `dream-cycle` — sleep-inspired consolidation (NREM → REM → integration)

Wraps existing dream system as a workflow. Macros: `phases` (multi). Slot: none.

### `knowledge-validate` — re-check knowledge against current sources

Periodic: walks knowledge entries, refreshes lineage validity, marks stale. Macros: `kind`, `older_than`. Slot: none.

### `knowledge-prune` — remove low-utility / stale knowledge

Macros: `min_score`, `min_age_days`, `dry_run`. Slot: none.

### `knowledge-export` — bundle knowledge for sharing / migration

Macros: `format` (jsonl | parquet | engram-archive), `filter`. Slot: none.

---

## 11. Observation

### `endpoint-health` — periodic endpoint probe

Cron-fired `endpoint-audit` with alerting on regressions. Macros: `targets`, `alert_channels`. Slot: `notifier`.

### `page-health` — periodic page probe

Cron-fired `page-audit`. Macros: `pages`, `viewport`, `alert_channels`. Slot: `notifier`.

### `perf-regression` — periodic perf benchmark + comparison

Wraps `cargo bench`, `benchmark.js`, etc. Compares to rolling baseline. Macros: `baseline_window`, `regression_threshold_pct`. Slot: `runner`.

### `synthetic-monitor` — run a critical user journey

Headless browser walks a scripted journey (login → search → buy). Alerts on failure. Macros: `journey`, `frequency`, `alert_channels`. Slot: `journey-script`.

### `log-summarize` — periodic log digest

Reads logs, identifies patterns / anomalies, posts summary. Macros: `period`, `severity_cutoff`. Slot: `log-source`.

### `alert-route` — route alerts to channels

Receives alerts from any Workflow's events, applies routing rules, dispatches. Macros: `routing_rules`. Slots: `notifier-slack`, `notifier-email`, `notifier-pagerduty`.

### `incident-triage` — initial triage on incident alert

Triggered by alert, gathers context, summarizes, suggests probable cause. Macros: `context_radius_minutes`. Slot: `context-collector`.

---

## 12. Communication

### `slack-summary` — periodic team summary to Slack

Reads recent runs, episodes, knowledge, posts a digest. Macros: `period`, `channel`, `format`. Slot: `notifier`.

### `slack-respond` — respond to Slack message via Workflow

Triggered by Slack message; produces response; posts back. Macros: `model`. Slots: `responder`, `notifier`.

### `github-comment` — post results as a PR/issue comment

Used inside `code-review`, `endpoint-audit` etc. Macros: `format`, `collapse_long`. Slot: none.

### `email-digest` — periodic email summary

Macros: `period`, `recipients`, `template`. Slot: `notifier`.

### `discord-broadcast` — periodic announcement

Macros: `channel`, `format`. Slot: `notifier`.

---

## 13. Workflow Meta

These are workflows for working *with* workflows. They make the system self-improving.

### `workflow-author` — guided creation of a new Workflow

User describes goal in natural language; agent generates initial Workflow TOML; user iterates. Used by the visual editor to bootstrap. Macros: `style` (recipe | graph | dsl). Slot: `coding-agent`.

### `workflow-test` — exercise a Workflow against canned inputs

Runs Workflow with fixture inputs, asserts output shape and key invariants. The CI for Workflows. Macros: `fixtures_dir`. Slot: `test-runner`.

### `workflow-publish` — sign + bundle + push to marketplace

Validates capabilities, signs artifact, uploads. Macros: `visibility` (public | org | private). Slot: `marketplace-target`.

### `workflow-fork` — local fork of a Workflow with lineage

Macros: `new_name`, `relax_pins`. Slot: none.

### `workflow-import` — install a marketplace artifact

Reviews capabilities with user, downloads, validates checksum, registers. Macros: `version_req`, `auto_grant_capabilities`. Slot: none.

### `workflow-benchmark` — measure cost / time / quality of a Workflow

Runs Workflow N times against a fixture, reports cost / wall / cache-hit / output stability. Used to compare candidate refactors of a Workflow. Macros: `n`, `fixture`. Slot: none.

### `workflow-compare` — diff two Workflow versions

Visual + structural diff between two versions. Used in marketplace fork-chain visualization. Macros: `from`, `to`. Slot: none.

---

## 14. Synergy Patterns

The catalog above is designed for synergies. Examples:

| Pattern | Workflow A | Workflow B | Trigger | Effect |
|---|---|---|---|---|
| Doc → Plan | `doc-ingest` | `prd-plan` | workflow-completion | Ingest produces PRDs; plan auto-fires per PRD |
| PR → Review → Comment | external GitHub PR open | `code-review` | github | Auto-review on every PR, comment posted back |
| Code change → Doc update | filesystem change | `doc-update` | folder-watch | Docs stay in sync automatically |
| Code change → Tests → Bench | filesystem change | `test-run` → `perf-regression` | folder-watch + chain | Local CI |
| Deploy → Smoke → Notify | manual | `deploy` → `smoke-test` → `slack-summary` | chain | One-line ship-with-confidence |
| Research → PRD → Plan → Execute | manual | `research-sweep` → `prd-draft` → `prd-plan` → `plan-execute` | chain | Idea-to-shipped pipeline |
| Episode → Cascade-improve | every Workflow run | implicit cascade-router learning | event-bus | The system gets better at model selection per Module |
| Knowledge GC → Resonance → Bridges | cron | `gc` → `knowledge-link` | chain | Pruning + new connections weekly |
| Cost spike → Investigate | budget event | `incident-triage` | event-bus filter | Auto-triage when costs run high |
| Page change detected → Visual gate | folder-watch on dist/ | `page-audit` → `visual-gate` → `slack-summary` | chain | Continuous UI quality monitoring |
| New artifact → enrich → audit | artifact-change | `web-enrich` → `prd-audit` | chain | Every fresh artifact passes through audit |
| Idea fragment → group → draft | new note | `idea-fragment-collect` → `prd-draft` | folder-watch + chain | Notes auto-promote into PRDs |

These are not coded as fixed pipelines; they're emergent from chaining individual Workflows via Triggers. Users discover useful chains and publish them as composite Workflows in the marketplace (PRD-12).

---

## 15. Implementation Order

| Tier | When | Workflows |
|---|---|---|
| Tier 0 (kernel for self-host) | First | `prd-draft`, `prd-plan`, `plan-execute`, `task-execute`, `prd-enrich`, `research-sweep` (replaces existing CLI commands) |
| Tier 1 (enable doc-ingest) | First | `doc-ingest`, `markdown-classify`, `doc-cluster`, `web-enrich`, `prd-audit`, `citation-check`, `artifact-persist`, `fs-walk` |
| Tier 2 (deploy + verification) | Soon | `deploy`, `deploy-railway`, `deploy-fly`, `deploy-vercel`, `smoke-test`, `endpoint-audit`, `page-audit`, `visual-gate`, `code-review` |
| Tier 3 (operations) | Soon | `watch`, `cron-tick`, `backup`, `gc`, `cost-report`, `dependency-update`, `doc-update`, `doc-1to1` |
| Tier 4 (knowledge + observation) | Mid | `knowledge-ingest`, `knowledge-link`, `dream-cycle`, `endpoint-health`, `perf-regression`, `synthetic-monitor` |
| Tier 5 (communication + meta) | Late | `slack-summary`, `slack-respond`, `email-digest`, `workflow-author`, `workflow-test`, `workflow-publish` |

---

## 16. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Every Workflow in the Tier 0 catalog ships in v1 with TOML + Module impls. | `roko workflow list` returns the full Tier 0 list. |
| Each Workflow validates clean (no schema errors, no unresolved refs). | `roko workflow validate <name>` passes for all builtin. |
| Tier 0 + Tier 1 enables `doc-ingest` end-to-end. | PRD-07 acceptance test. |
| Tier 2 enables `deploy` to Railway / Fly / Vercel from a single command. | Integration test against staging targets. |
| Synergy patterns from §14 work via trigger chaining without bespoke code. | Multi-step integration test (research → prd-draft → prd-plan → plan-execute). |
| Every catalog entry's TOML is in `<roko-install>/builtin/workflows/`. | Filesystem invariant check. |
| Every Module declares capabilities; capability summary computable per Workflow. | `roko workflow capabilities <name>` returns aggregated capability set. |

---

## 17. Open Questions

- Should there be Workflows for *other workflows' debug/forensics* — like `workflow-replay <run-id>` that re-runs a past run with the same inputs to debug? Probably yes; add to Tier 5.
- Should we ship a `chat` Workflow that wraps an interactive REPL with a single agent? It exists today; absorb as builtin Workflow for symmetry.
- Should there be a `swarm` Workflow that fans out to many agents in parallel and merges results? Likely yes for the agent-fleet patterns; add to Tier 4.
- Are there missing workflows the user needs in their day-to-day that this catalog doesn't cover? This is a living document — every PRD-driven addition expands the catalog.

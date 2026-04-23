# Implementation Plans — Roko Subsystem Audits

Generated: 2026-05-01

This folder contains **detailed, agent-ready implementation plans** for every
open issue surfaced by `tmp/subsystem-audits/` (the per-subsystem audits and
the `05-01/` deep audit + consolidated backlog).

Each plan is **self-contained**: a fresh agent with no prior chat context can
pick one file, read it top-to-bottom, and implement the change without
reading any other audit document. Plans include:

- Background and "why this exists"
- Exact files, line numbers, and code references (verified 2026-05-01)
- Implementation steps, in order
- Tests to add
- Verification commands
- Anti-patterns specific to that change
- "Do not do" boundaries
- Rollback strategy when relevant

---

## How To Use This Folder

1. Read `01-CONTEXT.md` once to understand the codebase layout, build commands, and crate map.
2. Read `02-ANTI-PATTERNS.md` once. These rules apply to **every** task.
3. Pick a plan from the table below. Read it top-to-bottom.
4. Run the pre-commit gate (see `02-ANTI-PATTERNS.md`) before every commit.
5. After landing, mark the task `[x]` in this index and in the originating audit doc.

---

## Source Material

These plans consolidate, verify, and expand on:

- `tmp/subsystem-audits/05-01/41-consolidated-backlog.md` — 42-item tier list (T0–T5)
- `tmp/subsystem-audits/05-01/35-current-state-checklist.md` — what's done vs partial vs open
- `tmp/subsystem-audits/05-01/36-deep-audit-acp-terminal-safety.md` — ACP/terminal/safety deep audit
- `tmp/subsystem-audits/05-01/37-learning-feedback-dead-code.md` — learning system dead code
- `tmp/subsystem-audits/05-01/38-serve-routes-security.md` — serve route security
- `tmp/subsystem-audits/05-01/39-config-schema-phantom-fields.md` — config phantom fields
- `tmp/subsystem-audits/05-01/40-gate-pipeline-dispatch-audit.md` — gate pipeline / dispatch
- `tmp/subsystem-audits/05-01/42-workflow-redesign-suggestion.md` — UX/workflow redesign idea
- Per-subsystem `AUDIT.md`/`PLAN.md`/`GOALS.md`/`ISSUES.md` in
  `acp-protocol/`, `cli-chat-tui/`, `cognitive-layer/`, `code-intelligence/`,
  `config-tools-events/`, `chain-deploy-demo/`, `gate-pipeline/`, `gtm/`,
  `http-persistence/`, `inference-dispatch/`, `learning-feedback/`,
  `orchestration/`, `prompt-assembly/`, `safety-agent/`, `ux/`, `gateway/`
- `tmp/subsystem-audits/INDEX.md` and `MASTER-IMPLEMENTATION-PLAN.md`

If a plan in this folder conflicts with anything in those docs, **the plan in
this folder wins** — it is the verified, latest version. The originals are
kept as historical context.

---

## Status Legend

- `[x]` — Verified complete in current worktree (HEAD as of 2026-05-01)
- `[~]` — Partial: skeleton/types/types-only landed, product migration not finished
- `[ ]` — Open: no verified implementation
- `[--]` — Cancelled / superseded / no longer applicable

Verified by spot-checking files at the line ranges named in
`05-01/41-consolidated-backlog.md` against the current worktree, and by
reviewing `git log --oneline | rg 'T[0-9]+-'`.

---

## Plan Index

### Reference (read once)

| File | Purpose |
|---|---|
| [`01-CONTEXT.md`](./01-CONTEXT.md) | Codebase layout, crate map, common commands, file paths |
| [`02-ANTI-PATTERNS.md`](./02-ANTI-PATTERNS.md) | Global do-not-do rules; pre-commit gate |
| [`03-VERIFICATION.md`](./03-VERIFICATION.md) | How to prove an item is done |

### Tier-Aligned Plans (priority order)

| File | Tier | Status (overall) | Items covered |
|---|---|---|---|
| [`10-tier0-stop-bleeding.md`](./10-tier0-stop-bleeding.md) | T0 | All `[x]` (kept for completeness) | T0-1..T0-7 |
| [`11-tier1-data-corruption.md`](./11-tier1-data-corruption.md) | T1 | All `[x]` (kept for completeness) | T1-8..T1-15 |
| [`12-tier2-delete-dead-code.md`](./12-tier2-delete-dead-code.md) | T2 | All `[ ]` open | T2-16..T2-21 |
| [`13-tier3-security-hardening.md`](./13-tier3-security-hardening.md) | T3 | Mostly `[ ]` open | T3-22..T3-28 |
| [`14-tier4-feedback-loops.md`](./14-tier4-feedback-loops.md) | T4 | Mostly `[ ]` open | T4-29..T4-34 |
| [`15-tier5-architectural.md`](./15-tier5-architectural.md) | T5 | All `[ ]` open | T5-35..T5-42 |

### Subsystem-Cross-Cutting Plans (deeper work, may span tiers)

| File | Subsystem | Status | Drives |
|---|---|---|---|
| [`20-orchestrate-rs-extraction.md`](./20-orchestrate-rs-extraction.md) | orchestration | open | T5-35; orchestrate.rs is 22.7K lines |
| [`21-acp-protocol-completion.md`](./21-acp-protocol-completion.md) | acp-protocol | partial | residual ACP issues from doc 36 |
| [`22-dispatch-streaming-completion.md`](./22-dispatch-streaming-completion.md) | inference-dispatch | partial | T5-36, T5-37; `DispatchPlan` + `ModelCallService` migration |
| [`23-config-validation-pipeline.md`](./23-config-validation-pipeline.md) | config-tools-events | partial | T5-38; provenance, strict load, dangerous overrides |
| [`24-runtime-ledger-migration.md`](./24-runtime-ledger-migration.md) | orchestration | partial | T5-40; gates/artifacts/events → ledger |
| [`25-learning-feedback-completion.md`](./25-learning-feedback-completion.md) | learning-feedback | partial | T4-29..T4-32; close all feedback loops |
| [`26-terminal-demo-truth.md`](./26-terminal-demo-truth.md) | cli-chat-tui / chain-deploy-demo | partial | T5-41; CommandEvent, demo automation |
| [`27-ci-fitness-checks.md`](./27-ci-fitness-checks.md) | infrastructure | open | promote inventory scripts to blocking gates |
| [`28-safety-agent-hardening.md`](./28-safety-agent-hardening.md) | safety-agent | partial | restricted defaults, recovery actions, override audit |
| [`29-gate-pipeline-rungs-3-5-6.md`](./29-gate-pipeline-rungs-3-5-6.md) | gate-pipeline | open | construct Symbol/PropertyTest/Integration gates |
| [`30-prompt-assembly-completion.md`](./30-prompt-assembly-completion.md) | prompt-assembly | partial | playbook wiring, HDC re-enable, single builder path |
| [`31-cognitive-layer-cleanup.md`](./31-cognitive-layer-cleanup.md) | cognitive-layer | open | delete pheromones (~68K LOC), simplify daimon |
| [`32-http-persistence-followups.md`](./32-http-persistence-followups.md) | http-persistence | partial | transactional writes, persistence consolidation |
| [`33-code-intelligence-followups.md`](./33-code-intelligence-followups.md) | code-intelligence | partial | re-enable HDC similarity, incremental index |
| [`34-chain-deploy-cleanup.md`](./34-chain-deploy-cleanup.md) | chain-deploy-demo | partial | delete dormant chain code, demo truth |

### Forward-Looking Plans (new direction; not strictly bug fixes)

| File | Topic | Source |
|---|---|---|
| [`40-workflow-progressive-formality.md`](./40-workflow-progressive-formality.md) | 5-verb UX redesign (`do`/`think`/`show`/`tune`/`undo`) | doc 42 |
| [`41-acp-as-universal-backend.md`](./41-acp-as-universal-backend.md) | one session type behind every surface | doc 42 idea E |
| [`42-work-items-first-class.md`](./42-work-items-first-class.md) | replace PRD/plan/task hierarchy | doc 42 idea B |

---

## Quick Status Snapshot (verified 2026-05-01)

### Tier 0: Stop Active Bleeding — 7/7 done

All 7 items landed. Commits visible in `git log` as `T0-1` through `T0-7`.
Plan file `10-tier0-stop-bleeding.md` retains the implementation detail for
historical reference / regression check.

### Tier 1: Silent Data Corruption — 8/8 done

T1-8 through T1-15 landed. Last 4 (T1-11, T1-13, T1-14, T1-15) shipped on the
current branch; the other four shipped earlier. Plan file
`11-tier1-data-corruption.md` retains the detail for regression-check use.

### Tier 2: Delete Dead Code — 0/6 done

The 4 orphan files (`resonant_patterns.rs`, `signal_metabolism.rs`,
`shapley.rs`, `kalman.rs`) still exist. The 14 unused learn modules still
listed in `crates/roko-learn/src/lib.rs`. The 7 phantom config sections are
still in the schema. Conductor / dream sinks still constructed in
`commands/plan.rs:392-396`.

### Tier 3: Security Hardening — ~1/7 done

- T3-22 (auth default): a non-loopback bind auto-enables auth, but the explicit
  default is still `false`. Partial.
- T3-23 (rate limiting): no `tower::limit::RateLimitLayer` or `governor`
  middleware in `roko-serve`. Open.
- T3-24 (body size limit): a global `RequestBodyLimitLayer::new(32 MiB)` exists,
  but per-endpoint webhook caps and the audit's recommended 4 MiB global do not.
  Partial.
- T3-25 (loopback bind): `PORT` env still binds `0.0.0.0:$PORT`. Open.
- T3-26 (WS message size): no `max_message_size` / `max_frame_size`. Open.
- T3-27 (path traversal + TOML injection): `toml_quote()` exists but agent
  manifest is still string-interpolated; no path canonicalization. Open.
- T3-28 (CORS methods/headers): still `allow_methods(Any).allow_headers(Any)`
  in `routes/middleware.rs:438-462`. Open.

### Tier 4: Feedback Loop Completion — 0/6 done

- T4-29: `KnowledgeIngestionSink::with_ingestor()` exists but `commands/plan.rs:389`
  uses `::at()` only — no ingestor wired.
- T4-30: `RoutingObservationSink::on_event` documents the reason for using
  `record_confidence_outcome` over `observe_multi_objective`. Real
  `RoutingContext` is not threaded through dispatch.
- T4-31: `UsageObservation` exists in `roko-core`. OpenAI-compatible and
  Perplexity parsers preserve it; Anthropic, Ollama, Gemini, Cerebras, Cursor
  parsers do not. Partial.
- T4-32: playbook store exists; `SystemPromptBuilder` is not consuming it.
- T4-33: no JSONL rotation seen in the writer paths.
- T4-34: chat `/model` is still partial-mutate-then-error in some failure modes.

### Tier 5: Architectural Extraction — 0/8 done

- T5-35: `dispatch_agent_with` still ~2K lines in `orchestrate.rs:14575+`.
  Whole file is 22,756 lines.
- T5-36: many serve routes still construct `reqwest::Client` directly.
- T5-37: `dispatch_direct.rs` still in production paths (chat_inline, unified, lib).
- T5-38: provenance types exist; broad load is not migrated.
- T5-39: Ollama path still does not have a runner-budget guardrail.
- T5-40: ledger exists; gates/artifacts/events/resume not yet ledger-first.
- T5-41: demo automation still scrapes prompts.
- T5-42: only OpenAI-compatible streaming uses provider-native messages reliably.

---

## Anti-Pattern Reminder

Before implementing **anything** in this folder, internalize these:

1. **Skeletons ≠ migrations.** A new type does not mean the product path uses it.
2. **Unknown ≠ zero.** Missing usage / cost / context stays `None`.
3. **No silent fallback.** Failed resolution → typed error, not synthesized config.
4. **Missing config → restricted.** Never grant permissions on load failure.
5. **No regex prompt scraping.** Consume typed events.
6. **No string-interpolated payloads.** Use `serde` / `toml` serializers.
7. **No new dispatch path.** Improve `ModelCallService` / `DispatchResolver`.
8. **One item per commit.** No scope expansion mid-PR.
9. **No `unwrap()`/`panic!()` in changed code.** Return typed errors.
10. **No unrelated edits.** Don't drive-by refactor neighbors.

Full reasoning and examples in [`02-ANTI-PATTERNS.md`](./02-ANTI-PATTERNS.md).

---

## Implementation Order Recommendation

If you're picking up this work cold, do it in this order:

1. **Read** `01-CONTEXT.md`, `02-ANTI-PATTERNS.md`, `03-VERIFICATION.md`.
2. **Land** T2 (delete dead code) — pure subtraction, lowest risk, clears the
   field for everything else. ~1 session.
3. **Land** T3-22..T3-28 (security hardening) — small, mechanical, deployment
   blockers. ~2 sessions.
4. **Land** T4-29, T4-31, T4-33, T4-34 (feedback loop closures and atomic /model).
   ~2-3 sessions.
5. **Land** plan 28 (safety hardening) and plan 27 (CI fitness checks). These
   prevent regressions while you do the heavy lifting next.
6. **Land** plan 22 (dispatch streaming) and plan 24 (runtime ledger). Big.
7. **Land** plan 20 (orchestrate.rs extraction). This is T5-35 spread across
   many commits.
8. **Land** plans 25, 26, 30 (learning, terminal demo, prompt assembly).
9. **Land** plans 23, 28 (config validation, safety contract integration).
10. Forward-looking work in plans 40-42 only after the engine is clean.

Total estimate: 6-10 sessions of focused implementation work, depending on
parallelism. Most plans can be done by a single agent in a single session;
plan 20 (orchestrate extraction) is the only multi-session item.

# B7: Evidence from real runs (what Roko did when it ran real plans, against V2–V8)
Agent tldr-B7 · 2026-09-29 · sources read:
- `tmp/dogfood/`: all 6 sessions.
- `tmp/dev-audit/`: 00, 02, 04, 06, 09 and 11.
- `tmp/portal-audit/`: 01 and 06.
- Commit `44a2cfc63`, companion-audit telemetry notes, and `.roko/` in five roots (read-only).

**Method.**
- An **attempt** is one `agent_efficiency_event/v1` dispatch row in `.roko/learn/efficiency.jsonl`.
  - Period: 09-05 to 09-29 07:41Z.
  - Sources: the main tree plus the worktrees `roko-portal-wt`, `roko-portal2-wt`, `roko-backend-wt` and `roko-backend2-wt`.
- An attempt's **verdict** is the adjacent `gate_pass`/`gate_failure` row for the same plan and task.
  - The window is −30 s to +300 s.
  - The gate row follows the dispatch row before the 09-28 fix and precedes it after.
- **Recorded** success means the dispatch outcome is `success`.
- **Conservative** success counts verified passes only. A success with a failing gate (a *false green*) or with no gate row (*unverified*) counts as a failure.

## TL;DR
- **Before the 09-28 fix, success labels could not be trusted.** Of 373 recorded successes (430 attempts, 31 plans):
  - 101 (27%) had a failing gate;
  - 96 (26%) had no verdict.

  The conservative pass rate is **40.9%** [95% CI 36.4–45.6] against 86.7% recorded. Run metrics claimed that 132 of 137 runs completed every task.
- **After the fix, the labels are honest.** Of 168 attempts, 151 (89.9%) were recorded as passes and all 151 were verified: 0 false greens and 0 unverified. 138 of 148 worktree tasks passed first time.
- **Roko built the portal.**
  - Scope: 16 plans and 173 tasks, of which 168 were gate-verified.
  - Cost: **$174.87** of agent spend and 25.6 agent-hours. The median task cost $0.83 (p90 $1.83).
  - **Every attempt ran `claude-sonnet-4-6`** through the Claude CLI.
- **V3 was not exercised on real work.** Every portal task pins `model_hint = "claude-sonnet-4-6"`. Cheap models (Cerebras `gpt-oss-120b`, 142 attempts) ran demo, test and bench tasks, plus 10 self-development attempts, of which 50% were verified.
- **Parallelism was modest, and the cross-plan part was done by hand.**
  - Within a plan, mean concurrency was 1.2–2.2 against a `max_parallel` of 3–4.
  - Backend plans were serial chains.
  - Across plans, four worktrees the operator made by hand ran 24.1 agent-hours in 15.1 busy hours: a mean of **1.59** concurrent agents, peak 5.
- **The gates checked parts, not the product.**
  - A narrow test gate let plan 01 ship a crate regression.
  - No gate ran `rustfmt` or clippy; 59 files were unformatted at merge.
  - After every unit and DOM gate had passed, browser passes still found about 20, then 12, 5 and 4 defects. A pre-flight smoke then found 3 more that blocked hello-world.
- **A frontier session did the hard part.** About 20 Claude Code subagents authored and audited plans, fixed about 25 engine defects, built the worktrees, merged three times and supervised. Their cost is not recorded in `.roko/`.
- **V7 has no evidence.** The cold/warm comparison matrix against manual Claude or Codex has never been run.
- **V8: data is recorded but not learned from.** All 148 worktree router observations are for one pinned model; experiments are never assigned; thresholds are never applied; `signals.jsonl` is empty in all 5 roots; worktree telemetry never returns.

## Findings
| Mechanism / feature | What the docs claim (doc §) | Reality (status + anchor) | Vision claims | Core? | Recommendation |
|---|---|---|---|---|---|
| Authoring loop (PRD → plan → run) | CLAUDE.md "Self-hosting workflow": `prd plan` → `plan run`, "each step … exists today" | **PARTIAL.** Claude Code subagents (p1, p2) wrote the portal plans. `roko prd plan` and `plan generate` were not used. Roko's sentence → plan path has passed only with a fake agent (plan 04: 33/33); 09-acceptance T03 has not run. Two frontier audit rounds made about 65 edits to the first 87 tasks, including 2 blockers and 1 shipped regression (R-1..R-7). The generator's role table was wrong (bug-f43cf8, done). Positive: each browser pass → plan → run round took 1–2.5 h | V2, V3 | core | fix: `plan validate` must catch what the audits caught (find-70edcb); run 09 T03 |
| Cheap models on mechanical tasks | CascadeRouter and bandits (CLAUDE.md, Learning) | **Not demonstrated.** All 210 portal attempts ran `claude-sonnet-4-6`. Cheap models appear only as judge and enrichment calls: `gpt-4o-mini`, 25 calls, $0.014 in total. `gpt-oss-120b` verified rates: bench 98% (n=49), test 87% (n=23), demo 57% (n=60), self-development 50% (n=10) | V3, V7 | core | wire: no default `model_hint`; cheapest model first, escalating on a failed gate |
| DAG parallelism | `max_parallel`; `[conductor] max_parallel_plans` (`06-PARALLEL-PLANS.md` §1; `plan_set.rs::PlanSetScheduler`) | **PARTIAL.** In-plan concurrency was 1.23–2.17 against caps of 2–4. Backend DAGs were 1–2 tasks wide. The cross-plan scheduler was built on 09-28, but the lanes were worktrees the operator made, run in tmux | V4 | core | wire: run programmes with `--max-parallel-plans`; author wider DAGs |
| Isolation and merge | worktree manager and merge step (CLAUDE.md, Runner support modules) | **BUILT-UNWIRED, and unsafe where it runs.** `accept_attempt` (`orchestrator/worktree/mod.rs:885`) has no production caller, so `--worktree-per-task` strands edits (gap-3b5361). `GitDeliveryBackend::git_merge` (`graph_execution/delivery.rs:116`) runs `git checkout` in the operator's tree (bug-a3760a, p1). In practice: hand-made worktrees, pinned binaries, and 3 hand merges with semantic breaks that no gate caught | V5 | core | redesign: a worktree per plan, with a gate that runs on the merged tree |
| Authored verify gates | "19 gates, 7-rung pipeline, adaptive thresholds" (CLAUDE.md) | **WIRED since 09-28**: `settle_task_verification` (`graph_task_dispatch.rs:1807`), with verdicts stored under `GATE_VERDICT_EXTENSION` (`graph_checkpoint.rs:62`). False greens went from 101/373 to 0/151. Still missing: the judge and fact-check rungs never run (gap-85f102), and a reviewer BLOCK does not gate (gap-f4b935). In-plan live checks passed where authored: 03 12/12, 03b 58/58, 04 33/33, 03c 24/24 | V6 | core | fix: add product-level and hygiene gates (change 1) |
| Cheaper and faster | FAST SLO p50 ≤ 300 s; scorecard targets (dev-audit 02, 06) | **Not demonstrated.** "The representative matrix has not yet been run" (dev-audit 06); there is no manual baseline. Frontend tasks cost $0.63–0.68 at about 3 min per attempt. Rust tasks cost $1.19–1.63 at 14–17 min per attempt, because of whole-crate cargo gates | V7 | core | run the matrix, including a manual Claude Code lane and the orchestrator's cost |
| Learning from runs | "Learning feedback … operational" (dogfood 09-20 readiness list) | **PARTIAL / ORPHANED.** Every router observation is for one model (bug-55b151). Experiments are unassigned (gap-fdd27f). Thresholds are recorded but not applied (find-4b4344). `signals.jsonl` is empty. Before the fix, 101 false-green labels fed the learners (bug-521f08, done) | V8, V9 | core | merge worktree telemetry; unpin models |
| Run metrics | backlog #169 | **PARTIAL.** `plan_runner.rs:1578` sets `completed = if succeeded { plan_tasks } else { 0 }`. All 4 failed portal runs therefore recorded 0 completed tasks, although 7/9, 7/14, 12/16 and 6/18 had passed | V10, V8 | supporting | fix: count tasks from the verdict extension |

### Top 10 failure modes seen in practice
| # | Failure mode | Evidence | Work items |
|---|---|---|---|
| 1 | **False greens** (force-accept at the cycle cap, episodes written before the gate, resume replaying the output) | 101 of 373 recorded successes had a failing gate. On 09-25, "all graph verify steps passed" was logged over a `BLOCK — implementation not applied` verdict. After the fix, 0 of 151 | bug-82d47b, bug-521f08, bug-06e2d1 (done); bug-6dc672 (in progress); bug-7e1b6b, gap-f4b935 |
| 2 | **Gates check parts, not the product** | Plan 01's gate ran one test file while two crate tests went red (R-1). No fmt or clippy gate: 59 files needed `rustfmt`, and 03c needed lint fixes. Browser passes found about 20, 12, 5 and 4 defects after green gates, and a pre-flight smoke found 3 more | find-70edcb, gap-161be1; fmt/clippy/end-to-end gate **not filed** |
| 3 | **Plan-authoring defects that only frontier audits caught** | Plan 02 crossed the crate boundary in the wrong direction, so T03 would always fail and T04–T09 would be skipped. Two negative-grep gates failed correct code. Two verify steps were vacuous. One `files` list declared 1 file for about 40 deletions. 36 references to a gitignored `tmp/` file stopped a worktree lane from starting | find-70edcb; "validate in the run tree" **not filed** |
| 4 | **Roles denied the tools their tasks need** | `architect`, `quick-reviewer` and `reviewer` tasks could not edit files, so they failed 3 identical attempts and were then force-accepted. 14 of 87 tasks were rewritten as `implementer` | bug-f43cf8 (done, PLAN_036); bug-e37197 |
| 5 | **One provider, with session limits** | All 210 portal attempts ran on one CLI subscription. Its limits stopped plan 02 (T08 failed 3 times in 2 s each) and plan 05 at 7/14. Failover did not fire because the loader stripped `fallback_models` (fixed 09-28) | find-229e9c, bug-35379d |
| 6 | **Turn caps and timeouts set by guesswork** | Caps of 10/10/20/30 failed 3 of 8 attempts in plan 05, logged only as `exit 1` with `turns: 0`. After raising the caps to 40/60/90/120, tasks still hit them: 03b-T06 at 61/60 and 03c-T20 at 91/90. The 600 s timeout killed 01-T10, 08e-T06 and 08f-T05 (twice, 09-29) | gap-3870d9 (done); gap-a791b4, gap-34b2ed, find-43768e |
| 7 | **Shared-tree contamination between parallel tasks** | 08b-T08 failed 3 attempts on a sibling's half-written file, and FailFast skipped 3 more tasks. Nobody noticed for 69 min. With sibling-settle (`3049b7fcf`), 08d ran 4 wide cleanly. 17 `write_all` sites lost records under load (fixed in `352f13f23`) | gap-4f3063, find-d1a883, bug-0b668a |
| 8 | **Isolation and merge done by hand** | `--worktree-per-task` strands edits, and the merge step checks out the operator's tree. `roko dashboard` killed a live run's agents (fixed by e7). 3 hand merges broke `GraphPlanRunParams` and `AgentOptions` | gap-3b5361, bug-a3760a, gap-0001a1 |
| 9 | **The operator cannot trust the display** | The TUI exited after plan 1 of 8, batched status and showed $0.00; these were fixed on 09-28. Still wrong: run metrics for failed plans, timeouts recorded at $0, and unverified tasks counted as passed | bug-690dc6, bug-7e1b6b, gap-f0e191; run metrics **not filed** |
| 10 | **Learning loops record but do not learn** | The router learns over a single arm; experiments are never assigned; thresholds are never applied; no Signals are written; worktree state is stranded | bug-55b151, bug-c34782, bug-8da8ba, gap-fdd27f, find-34a4b5, find-4b4344 |

### What the portal programme shows about real parallel plan execution
| Lane (worktree) | Plans | Tasks | Agent $ | Span | Agent-h | Mean concurrency while busy (max) |
|---|---|---|---|---|---|---|
| Frontend (`roko-portal-wt`) | 05→08 | 43 | 27.27 | 3.0 h (~1 h limit outage) | 2.26 | 05 1.00 (1), 06 1.46 (3), 07 1.33 (3), 08 1.58 (3) |
| Polish (`roko-portal2-wt`) | 08b→08e | 39 | 26.54 | 3.8 h | 2.08 | 08b 2.17 (4), 08c 1.23 (2), 08d 1.90 (4), 08e 1.40 (3) |
| Backend A (`roko-backend-wt`) | 03→03b→03c | 45 | 73.53 | 13.9 h | 14.52 | 1.00–1.39 (2) |
| Backend B (`roko-backend2-wt`) | 04→04b | 21 | 24.90 | 5.0 h | 4.81 | 1.14, 1.00 (2) |
| All lanes, 09-28 11:42Z → 09-29 07:41Z | 13 (+08f) | 148 | 152.24 | 20.0 h | 24.1 | **1.59 (5)**; ≥3 agents for 1.5 h; no agent running for 4.8 h |

- **Real parallelism came from lanes the operator built.** Four git worktrees in tmux, with pinned binaries, ran about 1.6× faster than the same agent time run serially. Roko's DAG did not provide it.
- **Within a plan, Roko reached about 40–55% of the configured cap.** For example, 08b averaged 2.17 with `max_parallel = 4`. Narrow waves and long tail tasks set the limit.
- **The frontend parallelized; the backend could not.**
  - Frontend tasks touched disjoint files, and their gates (`tsc`, `vitest`) took seconds.
  - Every backend plan writes `roko-cli` or `roko-serve`, so footprint admission (`06-PARALLEL-PLANS.md` §3) serializes them.
  - Cargo gates cost about 25 min per task; 03b was at 6/18 after 2.5 h.
  - Disk (a 144 GB `target/` with 114 GB free) rules out a Rust target per worktree.
- **A shared tree needs settle-and-reverify.** 08b's first run aborted on a sibling's edit; with the fix, 08d ran 4 wide cleanly.
- **Integration and supervision were not automated.**
  - There were five merges, three of them with semantic breaks, plus hand fixes to formatting and lints (see
    Corrections).
  - `cargo test --workspace` reached 0 failures (250 binaries) only after operator repair; on the morning of 09-28 it had 29 failures.
  - An abort went unseen for 69 min.
- **The output was real:** 68 files, about 14.4k LOC and 645 tests; minimum text contrast 4.95:1 (WCAG AA); checked live in four browser passes.

### The 5 changes that would most improve real outcomes
1. **Product-level and hygiene gates by default** (failure modes 2 and 3; find-70edcb, gap-161be1).
   - Every plan ends with a whole-tree gate: `cargo fmt --check`, clippy, and the tests of the touched crates.
   - UI plans also get a browser drive against the real server, with the real event order.
   - A vacuity check runs each verify step against the base tree and flags any that already pass.
   - `plan validate` runs in the tree where the plan will run.
2. **Roko owns plan-level isolation and merge** (failure mode 8; gap-3b5361, bug-a3760a, gap-0001a1).
   - Each plan gets a worktree from a snapshot commit.
   - Plans merge into an integration branch, and the change-1 gate runs on the merged tree.
   - This replaces the hand-made worktrees and hand merges.
3. **Unpin models and make cheap-first routing real** (failure modes 5 and 10; bug-55b151, bug-c34782, bug-35379d, find-229e9c).
   - Generated tasks no longer carry a `model_hint` by default.
   - `mechanical` and `focused` tiers try the cheapest capable model first and escalate when the gate fails.
   - The router learns from outcomes settled by the gate, across models.
   - This gives the first V3 and V7 evidence and removes the dependence on one subscription.
4. **A run supervisor with budgets calibrated from data** (failure modes 6 and 7; gap-ebd656, spec-a0403b, find-43768e, gap-a791b4).
   - It reports a non-zero exit or a stall immediately.
   - It classifies the cause: turn cap, timeout, session limit or sibling contamination.
   - It resumes with adjusted budgets, as e4's turn-cap resume already does.
   - It sets caps and timeouts from the p95 of successful tasks in each tier.
5. **One truthful run record, then the benchmark** (failure mode 9 and V7; bug-7e1b6b, bug-690dc6, gap-09e478).
   - Derive run metrics, the dashboard and costs (including timeouts) from the verdict extension.
   - Write Signals.
   - Merge the worktrees' `.roko/` back into the workspace.
   - Then run the 5 cold / 5 warm matrix against a manual Claude Code lane.

Proposed items (not filed): binary run metrics (`plan_runner.rs:1578`); no fmt/clippy/merge gate; gitignored context passes validation; worktree telemetry never merged.

## Gaps against the vision (V1–V10), most important first
1. **V6.** Verdicts are honest now, but the gates see parts, not the product. Every product defect was found by a human-directed browser pass or audit.
2. **V3 and V7.** There is no real-work evidence that cheap models are enough, and no measurement of "cheaper or faster". Everything ran on pinned Sonnet, and the orchestrator's cost is not recorded.
3. **V5.** Isolation and merge are manual. Roko's own primitives are unwired (`accept_attempt`) or unsafe (`git_merge`).
4. **V8 and V9.** Learning has no effect on outcomes. The loop that actually changed parameters (caps, timeouts, gate scope, plan shape) was the operator, working from dogfood notes.
5. **V4.** Parallelism is bounded by DAG width, Cargo and disk. The practical parallel unit is the plan lane, not the task.
6. **V2.** Authoring is fast with a frontier session, but plans needed frontier audits to become runnable. Roko's own generator is unproven with a real model.
7. **V10.** Much improved on 09-28. Run metrics and timeout costs still misreport; supervision is a person watching tmux.

## Research basis (for mechanisms in scope)
All keys are corpus-verified.
- `cemri2025why`: a taxonomy of multi-agent failures. Modes 3–4 are "specification" failures and 1–2 "verification" failures.
- `wang2025solved`, `aleithan2024swebench`, `qi2015analysis`: weak test oracles accept patches that look plausible but are wrong. Supports change 1.
- `petrovic2018state`, `jia2011analysis`: mutation testing as a check on test adequacy. Supports the vacuity check. Verified via Crossref.
- `luo2014flaky`: flaky tests are mostly asynchrony and concurrency bugs. Matches the N-9 `write_all` race.
- `wahi2026llm`, `zheng2023judging`: LLM judges are not oracles. Supports keeping deterministic verify steps as the authority.
- `pan2024feedback`: feedback loops with LLMs drive in-context reward hacking. Supports settling labels before any learner uses them.
- `chen2024frugalgpt`, `ong2025routellm`: cascades and routers cut cost at matched quality. This is the V3 and V7 premise, and it cannot be tested while tasks pin one model.
- `zhu2025abc`: task and outcome validity for agentic benchmarks. Applies to the V7 matrix.
- `ashby1960design`: ultrastability. Here the second-order loop was the operator.

## Questions for the author
1. Is Sonnet 4.6 the intended executor tier, or must V3 be proven with Haiku- or `gpt-oss`-class models? If Sonnet is the target, change 3 drops in priority.
2. Should Roko own plan-level worktrees and merges despite Rust targets of about 140 GB? The alternative is to keep the shared tree with footprint admission. The answer decides the design of change 2.
3. Should V7 comparisons count the cost of the orchestrating frontier session? The answer decides whether "cheaper" is measured per task or per programme.

## Sources (files, commits, work items)
**Dogfood logs** (`tmp/dogfood/`)
- `2026-09-{18,19,20-final,21}-session.md`
- `2026-09-25-portal-programme-run.md` (P0-1..P4, N-1..N-9, R-1..R-8)
- `2026-09-28-portal-programme-continuation.md`

**Audits**
- `tmp/dev-audit/{00,02,04,06,09,11}*.md`
- `tmp/portal-audit/{01-FINDINGS,06-PARALLEL-PLANS}.md`
- `companion-audit/telemetry/{FALSE-GREENS,FINDINGS,SELF-HOSTING-LEDGER}.md`

**Telemetry** (read-only, `roko/` and `roko-{portal,portal2,backend,backend2}-wt/`): `.roko/learn/{efficiency,costs,run-metrics}.jsonl`, `.roko/episodes.jsonl`, `.roko/state/graph/*/checkpoint.json`.

**Code:** `settle_task_verification`, `GATE_VERDICT_EXTENSION`, the `plan_runner.rs` run-metrics block, `PlanSetScheduler`, `accept_attempt`, `git_merge`.

**Commits:** `44a2cfc63`, `91b4745f8`, `725f21e05`, `3049b7fcf`, `352f13f23`, `98e0af4d6`, `4ca38b5c8`, `188c43c8d`, `3d0ee4d02`, `f7c542b5a`.

**Work items:** the ids cited above.

## Corrections (tldr-V1 verification, 2026-09-29)
- Integration on 09-28/29 took 5 merges by hand (`3d0ee4d02`, `f7c542b5a`, `98e0af4d6`, `4ca38b5c8`, `188c43c8d`), not 3 (B2).
  Three of the five had semantic breaks the text merge could not see, not all of them (2026-09-29; `evidence/field/CASES.md`
  CASE-005).
- `accept_attempt` is ORPHANED, not BUILT-UNWIRED: Runner-v2 called it.
- Within-plan caps were 2–4 (this note's own table), not 3–4.

Status: reviewed · budget 550 words · owner gap-353d57

# 1 Introduction

## 1.1 Capable or cheap

Unattended agent work forces a choice. Frontier models complete more tasks but cost more per attempt; cheap
models cost little but fail more often. Models fail more as tasks get longer, and small ones far sooner
[@kwa2025measuring; @sinha2025illusion]; §2 gives the settings.

Roko bets on the harness, the code that wraps a model and decides when its work counts as done. On work that
splits into small tasks with executable checks, the bet is that specs, checks, retries, escalation, integration
gates and feedback supply most of the dependability, so cheap models inside the harness can match a frontier
model working alone, for less. Studies support parts of this. A frontier model that split long-document
questions into subtasks for a small local model kept 97.9% of its quality at 5.7× lower cost
[@narayan2025minions]; cascades and routers cut cost at matched quality on single-turn queries
[@chen2024frugalgpt; @ong2025routellm]; across six agentic benchmarks, multi-agent set-ups helped decomposable
work and hurt sequential work [@kim2025towards]. None ran a repository-scale plan, and we claim nothing for
sequential or integrative work.

## 1.2 Built and designed

A frontier model writes the plan: small tasks, each with the commands that prove it done. The Graph engine
(`crates/roko-graph/src/engine.rs`) runs it as a dependency graph with durable checkpoints and resume
(WIRED@a17d4dadd), and each task's verify commands decide its verdict (WIRED@a17d4dadd), though a task without any
still counts as passed (PARTIAL@a17d4dadd, spec-e9d7ec). Roko is designed to run independent tasks in parallel on
the cheapest model that passes, escalate on failure, merge and check the whole plan, and regulate itself: keep cost
per verified task and the share of wrong passes in bounds, and audit its own regulators. Today, tasks run one at a
time by default (PARTIAL@a17d4dadd, spec-a78d57); the tier ladder is MISSING@a17d4dadd and escalation
ORPHANED@a17d4dadd (spec-98f76d); merging is ORPHANED@a17d4dadd and the whole-plan check MISSING@a17d4dadd
(spec-a0e40a); and the regulators and audits are MISSING@a17d4dadd, while the learning loops that run
(PARTIAL@a17d4dadd) have no measured benefit (spec-6ac537).

## 1.3 The evidence so far

Roko ran most of the build of its own web portal: 16 plans and 173 tasks, 168 gate-verified, for $174.87 of
recorded agent spend.[^1-portal] Since a verdict fix on 2026-09-28, 0 of 151 recorded passes had a failing gate, against
101 of 373 before it.[^1-verdicts] Two facts limit what this shows. All 210 attempts pinned one mid-tier model,
`claude-sonnet-4-6`, so the cheap-model half of the thesis is UNPROVEN@a17d4dadd (spec-567e52). And supervising
frontier-model Claude Code sessions wrote and audited the plans, fixed engine defects, set up worktrees, merged by
hand and checked the assembled product (§7), costing an estimated 16–20× Roko's recorded spend over the same
days.[^1-operator] Across 42 captured runs, Roko recovered from a failure by itself twice and people stepped in
39 times: an autonomy index of 2/41.[^1-autonomy]

## 1.4 Contributions

- **A design:** eight research-backed rules (§2), the architecture and its control stack (§3), and the golden
  path in eleven steps, each with its status (§4).
- **Cybernetic mechanisms** that regulate the loop and audit the regulators (§5), and three measures that make
  trust checkable (§6).
- **Field evidence** from the portal build, with the operator's share stated (§7).
- **An evaluation plan,** with what would count against the thesis (§8).
- **Status:** limitations and the order of work (§9), related work (§10), and every mechanism's tag at one
  commit (appendix).

[^1-portal]: Research note B7, frozen as `evidence/2026-09-29-b7-real-run-evidence.md` (sha256 `799b6a2b6184`),
    "TL;DR": Roko's records for the portal plans, attempts to 2026-09-29 07:41Z; costs as recorded, without a cost
    source or the supervising sessions.

[^1-verdicts]: B7 as above, "Method" and "TL;DR": recorded successes whose own gate failed. Before the fix
    (`725f21e05`): 430 attempts in 31 plans from 2026-09-05; after it, 168 attempts to 2026-09-29 07:41Z. Items
    bug-82d47b, bug-521f08, bug-06e2d1.

[^1-operator]: Assessment note W12, table F2, frozen as `evidence/2026-09-29-w12-operator-loop-cost.md` (sha256
    `82676de5eee4`): an estimate from the sessions' token counts, 2026-09-25 to 09-29, at API list prices and
    including research work, against $172.80 recorded by Roko. The harvester (gap-263de5) is built; measured
    figures await the daily rollup (gap-ccb87e).

[^1-autonomy]: Field rollup 2026-09-29T14:37:51, corrected by bug-7b37c4, frozen as
    `evidence/2026-09-29-field-rollup.md` (sha256 `7bade1532a6d`), "Totals": 42 runs from 2026-08-22, 124 notes.
    Index: automatic recoveries over automatic recoveries plus interventions. Observational.

Status: reviewed · budget 150 words · owner gap-353d57

# Abstract

Frontier models finish more agent work but cost more; cheap models fail more often, and sooner as tasks get
longer. Roko is a harness for spec'd agent work: a frontier model writes a plan of small tasks, each with an
executable check, and Roko runs it as a checkpointed, resumable graph, each task judged by its own checks
(WIRED@a17d4dadd). Roko is designed to run independent tasks in parallel on the cheapest model that
passes, escalate on failure, check the whole plan, audit its own passes and learn from verified outcomes; the
tier ladder, whole-plan check and audits are MISSING@a17d4dadd, escalation and merging ORPHANED@a17d4dadd. Roko
ran most of its own web portal's build: 16 plans and 173 tasks (168 gate-verified) for $174.87 of recorded agent
spend; the frontier sessions that supervised it cost an estimated 16–20× Roko's spend.[^0-portal] All 210 attempts
pinned one mid-tier model, so the cheap-model half of the thesis is UNPROVEN@a17d4dadd. The evaluation plan compares
cost per verified task with Claude Code on Opus 5.5 on hidden-test tasks (§8).

[^0-portal]: Research note B7, frozen by gap-29a64e as `evidence/2026-09-29-b7-real-run-evidence.md` (sha256
    `799b6a2b6184`), "TL;DR": Roko's records for the portal plans, attempts to 2026-09-29 07:41Z, costs as
    recorded. The supervising sessions: assessment note W12, frozen as
    `evidence/2026-09-29-w12-operator-loop-cost.md` (sha256 `82676de5eee4`), an API-equivalent estimate for
    2026-09-25 to 09-29 against the $172.80 Roko recorded over those days (§7).

Status: draft · budget 150 words · owner gap-353d57

# Abstract

Frontier models finish more agent work but cost more; cheap models fail more often, and sooner as tasks get
longer. Roko is a harness for spec'd agent work: a frontier model writes a plan of small tasks, each with an
executable check, and Roko runs it as a checkpointed, resumable graph, each task judged by its own checks
(WIRED@a17d4dadd). Roko is designed to run independent tasks in parallel on the cheapest model that
passes, escalate on failure, check the whole plan, audit its own passes and learn from verified outcomes; the
tier ladder, whole-plan check and audits are MISSING@a17d4dadd, escalation and merging ORPHANED@a17d4dadd. Roko
ran most of the build of its own web portal: 16 plans and 173 tasks (168 gate-verified) for $174.87 of recorded
agent spend, excluding the supervising sessions.[^0-portal] All 210 attempts pinned one mid-tier model, so the
cheap-model half of the thesis is UNPROVEN@a17d4dadd. The evaluation plan compares cost per verified task with
Claude Code on Opus 5.5 on hidden-test tasks (§8).

[^0-portal]: Roko's records for the portal plans, attempts to 2026-09-29 07:41Z, costs as recorded; §7 cites the
    frozen copy (gap-29a64e).

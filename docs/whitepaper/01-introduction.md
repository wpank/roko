Status: draft · budget 600 words · owner spec-ce1484

# 1 Introduction

This section sets out the problem, Roko's approach, the design in brief and the paper's contributions.

## 1.1 Variable workers

Language-model agents are capable but variable workers. On SWE-bench Verified, single-run pass rates vary by 2.2 to
6.0 percentage points between runs [@bjarnason2026randomness]. Agents fail more often as tasks get longer
[@kwa2025measuring; @sinha2025illusion], and an agent that can see the tests may delete a failing one instead of
fixing the code [@zhong2025impossiblebench]. Model fees can differ by two orders of magnitude [@chen2024frugalgpt].
Parallel work must come back together, and in open-source projects some merges that apply cleanly still break the
build [@brun2011proactive]. The harness, the code that decides what a model may do and when its work counts as done,
changes results even with the model held fixed [@lewis2026same].

## 1.2 Orchestration as regulation

Roko treats orchestration as regulation in the sense of classical cybernetics: keeping a few chosen quantities within
bounds despite disturbances such as a flaky test, a provider outage or an ambiguous spec
[@wiener1948cybernetics; @ashby1956introduction]. Its unit is the feedback loop, which measures a result, compares it
with a reference (the goal) and acts on the difference. Roko sits above agent loops such as Claude Code and Codex,
and runs them and API models as workers (§3).

**The executable verdict is the one fact every loop reads.** Each attempt settles into one verdict record, and its
learning label counts only a pass of the task's checks as a success (§6). The attempt loop retries and escalates on
the verdict, the plan loop integrates on it, the learning loops learn from it, and the audit level checks it.

## 1.3 The design in brief

The paper's thesis is:

> Roko is a cybernetic multi-agent orchestration harness. Its design is a set of feedback loops nested by time
> scale, from a single tool call to the audits of its own learning. Each loop steers toward a reference written
> before the work starts, measures with a sensor the actor cannot change, acts within bounds that people set, and
> leaves records that slower loops read. That structure is how Roko is built to do its job well: to turn plans into
> verified, integrated work across many agents and models.

Planning is the feedforward half: it writes each task's checks before any agent runs. Five feedback loops do the
rest (§5):

- **The tool-call loop (L0, seconds)** permits, bounds and screens every action against the worker's role contract.
- **The attempt loop (L1, minutes)** screens each attempt for tampering, runs its checks and retries with distilled
  gate feedback; repeated agent failures climb an escalation ladder of model tiers, and a provider failure switches
  provider sideways.
- **The plan loop (L2, hours)** runs tasks whose files do not overlap in parallel, each in its own worktree, gates the
  integrated result on a whole-plan check, and keeps checkpoints so that a run can stop and resume.
- **The learning loops (L3, days)** learn from verified outcomes which model a task goes to, how many retries it gets,
  and which playbooks and knowledge enter its prompt.
- **The audit level (L4, weeks)** estimates the false-green rate (the share of passes that a stronger, independent
  check would fail) from random deep audits, tests each learning loop against a holdout, and moves one setting one
  notch when an essential variable leaves its bounds.

## 1.4 Contributions

This paper describes Roko's design: its loops, the mechanisms that implement them, and the reasons for each choice.
It contributes a design of orchestration as nested regulation around one executable verdict (§2, §3); attempt and
plan loops that judge work only by checks written before it (§4); learning from verified outcomes, under audit (§5,
§6); and bounds at every level (§8). §7 shows Roko in use, §9 weighs trade-offs and open problems, and §10 covers
related work.

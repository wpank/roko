Status: reviewed · budget 550 words · owner gap-c19902

# 9 Status, limitations and roadmap

## 9.1 Status

The [status matrix](appendix-status-matrix.md) tags the paper's 71 mechanisms against the code at `a17d4dadd`: 21
wired, 23 partial, 2 broken, 7 orphaned, 7 built but unwired, 10 missing and 1 removed (Figure 3).[^9-matrix] What is
WIRED@a17d4dadd is the core loop: plan generation and validation, the Graph engine with resume, verify commands,
retries and budgets.
Merges on 2026-09-29 changed scheduling (`bbf6517fc`), retry budgets (`99adacd6d`), the cost of timed-out attempts
(`d4be4e872`), environment isolation (`1d923e377`) and learning loops (`ce3bdcbb8`, `33e107da1`).

Figure 3: The status matrix at `a17d4dadd`: mechanisms per status.

```text
WIRED          21  #####################
PARTIAL        23  #######################
BROKEN          2  ##
ORPHANED        7  #######
BUILT-UNWIRED   7  #######
MISSING        10  ##########
REMOVED         1  #
```

None of the ten vision claims is fully met:

| Claim | Status | Main epics |
|---|---|---|
| V1 Domain-agnostic orchestration | PARTIAL@a17d4dadd | No epic |
| V2 Fast authoring, frontier planner | PARTIAL@a17d4dadd | spec-e57870 |
| V3 Cheapest capable model per task | MISSING@a17d4dadd | spec-98f76d, spec-e57870 |
| V4 Parallel execution | PARTIAL@a17d4dadd | spec-a78d57 |
| V5 Isolation and integration | PARTIAL@a17d4dadd | spec-ba7bea, spec-a0e40a |
| V6 Trust from gates | PARTIAL@a17d4dadd | spec-e9d7ec, spec-9230a9, spec-6ac537 |
| V7 Cheaper and faster than top models | UNPROVEN@a17d4dadd | spec-567e52 |
| V8 Improves over time | PARTIAL@a17d4dadd | spec-6ac537 |
| V9 Cybernetic throughout | PARTIAL@a17d4dadd | spec-6ac537 |
| V10 Observable and controllable | PARTIAL@a17d4dadd | No epic; bug-8208a6 |

## 9.2 Limitations

- **One harness, one model.** All the evidence comes from Roko's own development, and all 210 portal attempts
  pinned `claude-sonnet-4-6`.[^9-model] The cheap-model half of the thesis is untested (V3, V7).
- **Observational evidence.** §7 has no control arm, so it supports no causal claim, and supervising frontier
  sessions did much of the work, at an estimated 16–20× Roko's recorded spend.[^9-operator]
- **Code-first in practice.** Shell-command verifiers are the only check not tied to code, and triggers cannot start
  agent work (appendix rows DM1–DM3). No epic covers other domains.
- **Safety.** There is no OS sandbox in v1 (decided 2026-09-29; spec-ba7bea), and agents work in the operator's
  checkout by default. Since `1d923e377`, gates and provider CLIs drop known provider keys (bug-7d7200), but agent
  tool shells and MCP servers still inherit them (bug-0d9ac4, bug-0eb8e2), the key files stay readable (bug-a66941),
  and the git guard misses reset, stash and clean (bug-7de5df). Environment isolation is PARTIAL@a17d4dadd.
- **No measured learning.** A benefit from any learning loop is UNPROVEN@a17d4dadd: the router saw one model on the
  portal runs, and learning cannot yet be frozen for a comparison (gap-644040).

## 9.3 Roadmap

After the release blockers (spec-ae5f94), four goals follow in order, each epic ending in an executable exit check.

1. **Truth:** honest verdicts (spec-e9d7ec), secrets and the git guard (spec-ba7bea), and one settled record per
   attempt (spec-b7303f). Exit: fixture tests show honest verdicts everywhere, no leaked keys, and one record per
   attempt with its verdict, executed model and cost source.
2. **Golden path:** the tier ladder (spec-98f76d), integration (spec-a0e40a), the scheduler (spec-a78d57), specs a
   cheap model can execute (spec-e57870), the attempt diff check (spec-9230a9) and supervision (spec-edda86). Exit
   (spec-f09094): a fixture plan runs on real cheap models, escalates on failure, merges as a branch and passes
   formatting, lint and test checks unattended, three runs out of three.
3. **Proof:** the pilot of §8 (spec-567e52) and a record of the work itself (spec-f2463d). Exit: a committed pilot
   report, and a rollup of cost per merged item.
4. **Cybernetic core** (spec-6ac537): routing that learns from verified failures and a frozen-learning mode, then the
   audits, self-model and controller of §5. Exit: tests for router labels, playbook credit and frozen runs.

## 9.4 Proposed parking

A proposal, not yet decided, moves subsystems with no measured benefit behind feature flags and out of the default
build: the chain and marketplace code, offline batch consolidation, the conductor, affect, and the knowledge store's
extra modules. The four crates it would park whole hold 6.1% of the Rust in `crates/` at `1f4481133`; with the other
modules, about 9%, an estimate.[^9-park]

[^9-matrix]: gap-35a614's matrix, each tag re-checked against the code at `a17d4dadd` (2026-09-29).
[^9-model]: Research note B7, frozen as `evidence/2026-09-29-b7-real-run-evidence.md` (sha256 `799b6a2b6184`),
    "TL;DR": portal plans, attempts to 2026-09-29 07:41Z, from Roko's efficiency records; also spec-f09094 and
    gap-e21595.
[^9-operator]: Assessment note W12, table F2, frozen by gap-29a64e as
    `evidence/2026-09-29-w12-operator-loop-cost.md` (sha256 `82676de5eee4`): an estimate of about $2.7–3.4k
    API-equivalent for 2026-09-25 to 2026-09-29, research programme included, against the $172.80 Roko recorded.
[^9-park]: Lines of the tracked `.rs` files in `crates/roko-chain`, `crates/roko-dreams`, `crates/roko-conductor` and
    `crates/roko-daimon` at `1f4481133` (`wc -l`): 64,066 of 1,052,880. The 9%, about 94k lines, adds the other
    modules, estimated at `d9e79e9d8`.

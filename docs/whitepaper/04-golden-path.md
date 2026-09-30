Status: reviewed · budget 950 words · owner gap-ac4646

# 4 The golden path, step by step

The golden path is the thesis in motion: eleven steps from a request to verified, integrated work that improves the
next plan. Step N is §4.N. Each gives the design, tagged as in the status matrix (the appendix, at `41228d7b2`), and
the principle from §2 behind it; §4.12 names the epics that would close the gaps.

![Figure 2: golden-path diagram](figures/fig2-golden-path.svg)

**Figure 2:** The golden path. Failures loop back from step 8, and outcomes feed the next plan; tags are at
`41228d7b2`.

## 4.1 Author

A frontier model drafts the plan with the author, who edits and regenerates it quickly: PARTIAL@41228d7b2. Since
`28db9c789` one setting picks the planner model on every generate and revise path but one (gap-853b31, bug-8b1bf8).
But the prompt (`crates/roko-cli/src/prd.rs`) cuts the PRD at 8,000 characters and lets the planner read at most five
files, and the portal's plans were written outside Roko (§7).
Principle: *put ambiguity back into authoring*.

## 4.2 Compile the spec

Each task compiles into exact context, acceptance criteria bound to checks, and planner-written tests proven to fail
on the base: PARTIAL@41228d7b2. Context packs are WIRED@41228d7b2 (`render_declared_context`), and since `af51b7a61`
the planner's tests named in a task's accept table are pinned outside the working tree and run first
(WIRED@41228d7b2, AU7; gap-d14a43). Acceptance criteria are still prompt text, the spec-quality lint is opt-in
(PARTIAL@41228d7b2, AU3), and nothing in Roko proves that a check fails before the change. Principle: *the planner
writes the checks*.

## 4.3 Size and split

Task size follows the executor tier's measured pass rate, and tasks that may run together write disjoint files:
PARTIAL@41228d7b2. Plan validation checks task size against fixed per-tier limits, not measured pass rates
(gap-1d1fa6), and flags tasks that may run together but share files (gap-a8d786). Principles: *size tasks for the executor*; *split on independent outputs*.

## 4.4 Route

Each task runs on the cheapest model that passes its checks, chosen from a role × tier ladder: PARTIAL@41228d7b2.
Since `a13873ad2` a task that neither `--model` nor a hint pins starts on the rung its role and tier select, from
gpt-oss-120b up to Sonnet, and the learned router's pick is only logged (RC2). No real run has used the cheap rungs
yet (gap-e21595). Principle: *count cost per verified task*.

## 4.5 Schedule

Ready tasks dispatch at once, as wide as the DAG and the write sets allow: WIRED@41228d7b2. Since `bbf6517fc` each
task starts when its own dependencies settle, and a failure skips only its dependants (`SkipFailed`, gap-96d348);
tasks whose declared files overlap never run together, and since `626e182a9` a plan that omits `max_parallel` runs as
wide as its graph when its tasks declare their files (gap-272448). An authored value still wins: 102 of the 132
tracked plans set it to 1, 25 of them with tasks that could run together.[^4-plans] Principle: *split on independent outputs*.

## 4.6 Isolate

Each task works in its own workspace, out of reach of the operator's secrets: PARTIAL@41228d7b2. Tasks edit the
operator's checkout by default; the opt-in per-task worktree starts from its plan's branch (§4.9). Agents, their tools
and gates start without roko's provider keys, as canary C2 confirms (WIRED@41228d7b2, IS5); a best-effort guard keeps Claude CLI agents from the key files and
destructive git commands (WIRED@41228d7b2, IS4). A provider CLI keeps a key of its own that the user exported, and
there is no OS sandbox (IS6).

## 4.7 Verify

Every attempt faces its visible checks plus checks it cannot see or edit: PARTIAL@41228d7b2. Since `7f9d1fcbc`
a screen diffs each attempt first and fails edits to tests, verify scripts or pinned acceptance tests
(WIRED@41228d7b2, QA4); then the task's own visible verify commands run, and the workspace's rungs. Verdicts have
been honest since the fix of 2026-09-28, with 0 false greens in 151 passes, down from about 27% of recorded
successes,[^4-verdicts] but the dashboard still counts `Unverified` as a pass. Hidden tests are MISSING@41228d7b2. Principle: *assume visible checks are gamed*.

## 4.8 Recover

A failed check earns two cheap retries with the distilled gate errors, then one rung up the ladder, then a split or
replan: PARTIAL@41228d7b2. Retries with the parsed gate output are WIRED@41228d7b2, three by default or as gate
history sets (`99adacd6d`), and since `ce12e86d8` two failures blamed on the agent move a task one ladder rung up
(WIRED@41228d7b2, EX7). Since `dd192cb82` a watchdog cancels and retries an agent
that has gone silent (WIRED@41228d7b2, EX9). Split or replan is not: the `ReplanController` is
BUILT-UNWIRED@41228d7b2. Principle: *retry cheaply, then escalate*.

## 4.9 Integrate

Each verified task commits to a plan branch, plans merge through a queue, and a whole-plan gate checks the merged
result: WIRED@41228d7b2 (IS2, IS3). Under per-task worktrees, `accept_attempt` folds each passed attempt onto its
plan's branch, and since `9c0b9aed0` each plan whose tasks passed merges into a batch branch in turn, where its
whole-plan check runs (`207f91da2`); a failure takes the merge back out. The check defaults to fmt, clippy and the
affected crates' tests in a Cargo workspace; other projects must write one. On 2026-09-28, before these merges, portal
plans 05 to 08 passed every gate while the assembled product was unusable (§7).
Principle: *merge, then verify*.

## 4.10 Review

An optional hold lets a person approve each task's diff before it merges: MISSING@41228d7b2. `--approval` only opens
the TUI, and the per-task diff route looks for `agent/*/<task>` branches that Graph runs never create. gap-0d64d5,
under E6 integration, covers this step.

## 4.11 Learn

Verified outcomes teach Roko how to route, size and specify tasks: PARTIAL@41228d7b2. Merges on 2026-09-29
(`ce3bdcbb8`, `33e107da1`, `91cfe0467`) re-wired playbook credit, prompt experiments, knowledge write-back, retry
budgets from gate history and the router's learning from failures; later ones made learners read settled verdicts and
fed failures back as error patterns (`74eaf5c8f`, `ed471a8c1`). But no loop has a measured benefit, the knowledge read path, BROKEN@41228d7b2, injects nothing
(bug-86117a), and on the portal the router saw one pinned model (§4.12). Principle: *count cost per verified task*.
§5 covers the loops.

## 4.12 Where the path stands

Each step's tag, its appendix rows, and the epics that would change it:

| Step | Status | Matrix rows | Next |
|---|---|---|---|
| 1 Author | PARTIAL@41228d7b2 | AU1, AU2, V2 | E8 specs (spec-e57870) |
| 2 Compile the spec | PARTIAL@41228d7b2 | AU3, AU4, AU6, AU7 | E8 specs |
| 3 Size and split | PARTIAL@41228d7b2 | AU2, V3 | E8 specs; E7 scheduler (spec-a78d57) |
| 4 Route | PARTIAL@41228d7b2 | RC2, V3 | E5 tier ladder (spec-98f76d; gap-e21595) |
| 5 Schedule | WIRED@41228d7b2 | EX3, V4 | E7 scheduler |
| 6 Isolate | PARTIAL@41228d7b2 | IS1, IS4, IS5, IS6 | E6 integration (spec-a0e40a); E3 secrets (spec-ba7bea) |
| 7 Verify | PARTIAL@41228d7b2 | QA1, QA2, QA3, QA4, QA5 | E2 verdicts (spec-e9d7ec); E17 audits |
| 8 Recover | PARTIAL@41228d7b2 | EX6, EX7, EX8, EX9, QA7 | Split or replan (gap-3b170b, parked) |
| 9 Integrate | WIRED@41228d7b2 | IS2, IS3 | E6 integration: worktrees by default (gap-4ec59f) |
| 10 Review | MISSING@41228d7b2 | SS6 | E6 integration (gap-0d64d5) |
| 11 Learn | PARTIAL@41228d7b2 | V8, LM3, RC2 | E17 cybernetic core (spec-6ac537) |
| Whole path: cheaper at equal quality | UNPROVEN@41228d7b2 | V7 | E11 acceptance tests (spec-f09094); E12 pilot (spec-567e52) |

Steps 5 and 9 do what their design asks, but the central promise is untested. Every portal attempt pinned
`claude-sonnet-4-6`,[^4-model] before the ladder existed, so no real task has yet run on a cheap executor with
escalation. E11's end-to-end run (gap-f30b8e) is designed as the first test of the whole path (§9.3), and §8 plans
the comparison with a frontier agent.

[^4-plans]: Counted at `41228d7b2` over the 132 tracked `tasks.toml` files under `plans/`;
    `git grep -l -E '^max_parallel *= *1( |#|$)' 41228d7b2 -- 'plans/*tasks.toml'` lists the 102. Tasks could run
    together when two sit at the same depth of the plan's dependency graph (parsed with `tomllib`).
[^4-verdicts]: Research note B7, frozen as `evidence/2026-09-29-b7-real-run-evidence.md` (sha256 `799b6a2b6184`),
    "TL;DR", and CASE-001 in `evidence/2026-09-29-field-cases.md`; also spec-e9d7ec. Visible verify steps only:
    recorded successes from 2026-09-05 to the fix, then passes to 2026-09-29 07:41Z. Not an audited false-green rate.
[^4-model]: B7 as above, "TL;DR": all 210 portal attempts, to 2026-09-29 07:41Z, pinned one model; also
    spec-f09094 and gap-e21595.

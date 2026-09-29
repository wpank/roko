Status: reviewed · budget 950 words · owner gap-ac4646

# 4 The golden path, step by step

The golden path is the thesis in motion: eleven steps from a request to verified, integrated work that improves the
next plan. Step N is §4.N. Each gives the design, tagged as in the status matrix (the appendix, at `ed0c33bd5`), and
the principle from §2 behind it; §4.12 names the epics that would close the gaps.

![Figure 2: golden-path diagram](figures/fig2-golden-path.svg)

**Figure 2:** The golden path. Failures loop back from step 8, and outcomes feed the next plan; tags are at
`ed0c33bd5`.

## 4.1 Author

A frontier model drafts the plan with the author, who edits and regenerates it quickly: PARTIAL@ed0c33bd5. Since
`28db9c789` one setting picks the planner model on every generate and revise path but one (gap-853b31, bug-8b1bf8).
But the prompt (`crates/roko-cli/src/prd.rs`) cuts the PRD at 8,000 characters and lets the planner read at most five
files, and the portal's plans were written outside Roko (§7).
Principle: *put ambiguity back into authoring*.

## 4.2 Compile the spec

Each task compiles into exact context, acceptance criteria bound to checks, and planner-written tests proven to fail
on the base: PARTIAL@ed0c33bd5. Context packs are WIRED@ed0c33bd5 (`render_declared_context`), and since `af51b7a61`
the planner's tests named in a task's accept table are pinned outside the working tree and run first
(WIRED@ed0c33bd5, AU7; gap-d14a43). Acceptance criteria are still prompt text, the spec-quality lint is opt-in
(PARTIAL@ed0c33bd5, AU3), and nothing in Roko proves that a check fails before the change. Principle: *the planner
writes the checks*.

## 4.3 Size and split

Task size follows the executor tier's measured pass rate, and tasks that may run together write disjoint files:
MISSING@ed0c33bd5. The generator's prompt prefers "the fewest cohesive tasks", and no plan lint compares the files of
tasks that may run together (gap-a8d786). Principles: *size tasks for the executor*; *split on independent outputs*.

## 4.4 Route

Each task runs on the cheapest model that passes its checks, chosen from a role × tier ladder: MISSING@ed0c33bd5.
Today a person picks the model, through `--model` or the task's hint; failing both, the learned router picks, and it
is PARTIAL@ed0c33bd5 because it learns from the provider's success flag, before gates run (bug-c34782). The tier
names plans use, such
as `focused` and `mechanical`, all route as Standard. Principle: *count cost per verified task*.

## 4.5 Schedule

Ready tasks dispatch at once, as wide as the DAG and the write sets allow: PARTIAL@ed0c33bd5. Since `bbf6517fc` each
task starts when its own dependencies settle, and a failure skips only its dependants (`SkipFailed`, gap-96d348);
since `1697fea53` tasks whose declared files overlap never run together (gap-439794). But `max_parallel` is 1 by
default and in the generator's template: 102 of the 132 tracked plans set it to 1, 25 of them with tasks that could
run together.[^4-plans] Principle: *split on independent outputs*.

## 4.6 Isolate

Each task works in its own workspace, out of reach of the operator's secrets: PARTIAL@ed0c33bd5. Tasks edit the
operator's checkout by default, and the opt-in per-task worktree never merges its edits back. Since `1d923e377`, gates
start from an allowlisted environment and provider CLIs drop the key variables they recognise (bug-7d7200), and since
`0728a2817` a best-effort guard keeps Claude CLI agents from the key files and destructive git commands
(WIRED@ed0c33bd5, IS4). But agent tool shells and MCP servers still inherit the keys, and there is no OS sandbox.

## 4.7 Verify

Every attempt faces its visible checks plus checks it cannot see or edit: PARTIAL@ed0c33bd5. Only the task's own,
visible verify commands run. Verdicts have been honest since the fix of 2026-09-28, with 0 false greens in 151 passes,
down from about 27% of recorded successes,[^4-verdicts] but `Unverified` still counts as a pass for learning and the
dashboard. Tamper, scope and hidden-test checks are MISSING@ed0c33bd5. Principle: *assume visible checks are gamed*.

## 4.8 Recover

A failed check earns two cheap retries with the distilled gate errors, then one rung up the ladder, then a split or
replan: PARTIAL@ed0c33bd5. Retries with the parsed gate output are WIRED@ed0c33bd5, on the same model: three by
default, or a budget set from gate history since `99adacd6d`. Escalation was deleted with Runner-v2
(ORPHANED@ed0c33bd5), the `ReplanController` is BUILT-UNWIRED@ed0c33bd5, and a watchdog for stalled agents is
MISSING@ed0c33bd5. Principle: *retry cheaply, then escalate*.

## 4.9 Integrate

Each verified task commits to a plan branch through a merge queue (ORPHANED@ed0c33bd5), and a whole-plan gate checks
the merged result (MISSING@ed0c33bd5). Nothing merges on the Graph path: `MergeQueue` and `accept_attempt` have no
production caller. On 2026-09-28 portal plans 05 to 08 passed every gate while the assembled product was unusable (§7).
Principle: *merge, then verify*.

## 4.10 Review

An optional hold lets a person approve each task's diff before it merges: MISSING@ed0c33bd5. `--approval` only opens
the TUI, and the per-task diff route looks for `agent/*/<task>` branches that Graph runs never create. gap-0d64d5,
under E6 integration, covers this step.

## 4.11 Learn

Verified outcomes teach Roko how to route, size and specify tasks: PARTIAL@ed0c33bd5. Merges on 2026-09-29
(`ce3bdcbb8`, `33e107da1`, `91cfe0467`) re-wired playbook credit, prompt experiments, knowledge write-back, retry
budgets from gate history and the router's learning from failures. But no loop has a measured benefit, the knowledge read path, BROKEN@ed0c33bd5, injects nothing
(bug-86117a), and on the portal the router saw one pinned model (§4.12). Principle: *count cost per verified task*.
§5 covers the loops.

## 4.12 Where the path stands

Each step's tag, its appendix rows, and the epics that would change it:

| Step | Status | Matrix rows | Next |
|---|---|---|---|
| 1 Author | PARTIAL@ed0c33bd5 | AU1, AU2, V2 | E8 specs (spec-e57870) |
| 2 Compile the spec | PARTIAL@ed0c33bd5 | AU3, AU4, AU6, AU7 | E8 specs |
| 3 Size and split | MISSING@ed0c33bd5 | V3 | E8 specs; E7 scheduler (spec-a78d57) |
| 4 Route | MISSING@ed0c33bd5 | RC2, V3 | E5 tier ladder (spec-98f76d) |
| 5 Schedule | PARTIAL@ed0c33bd5 | EX3, V4 | E7 scheduler |
| 6 Isolate | PARTIAL@ed0c33bd5 | IS1, IS4, IS5, IS6 | E6 integration (spec-a0e40a); E3 secrets (spec-ba7bea) |
| 7 Verify | PARTIAL@ed0c33bd5 | QA1, QA2, QA3, QA4, QA5 | E2 verdicts (spec-e9d7ec); E9 diff check (spec-9230a9); E17 audits |
| 8 Recover | PARTIAL@ed0c33bd5 | EX6, EX7, EX8, EX9, QA7 | E5 tier ladder; E10 watchdog (spec-edda86) |
| 9 Integrate | ORPHANED@ed0c33bd5, MISSING@ed0c33bd5 | IS2, IS3 | E6 integration |
| 10 Review | MISSING@ed0c33bd5 | SS6 | E6 integration (gap-0d64d5) |
| 11 Learn | PARTIAL@ed0c33bd5 | V8, LM3, RC2 | E17 cybernetic core (spec-6ac537) |
| Whole path: cheaper at equal quality | UNPROVEN@ed0c33bd5 | V7 | E11 acceptance tests (spec-f09094); E12 pilot (spec-567e52) |

No step yet does everything its design asks, and the central promise is untested. Every portal attempt pinned
`claude-sonnet-4-6`[^4-model] and escalation is not wired, so no real task has yet run on a cheap executor with
escalation. E11's end-to-end run (gap-f30b8e) is designed as the first test of the whole path (§9.3), and §8 plans
the comparison with a frontier agent.

[^4-plans]: Counted at `ed0c33bd5` over the 132 tracked `tasks.toml` files under `plans/`;
    `git grep -l -E '^max_parallel *= *1( |#|$)' ed0c33bd5 -- 'plans/*tasks.toml'` lists the 102. Tasks could run
    together when two sit at the same depth of the plan's dependency graph (parsed with `tomllib`).
[^4-verdicts]: Research note B7, frozen as `evidence/2026-09-29-b7-real-run-evidence.md` (sha256 `799b6a2b6184`),
    "TL;DR", and CASE-001 in `evidence/2026-09-29-field-cases.md`; also spec-e9d7ec. Visible verify steps only:
    recorded successes from 2026-09-05 to the fix, then passes to 2026-09-29 07:41Z. Not an audited false-green rate.
[^4-model]: B7 as above, "TL;DR": all 210 portal attempts, to 2026-09-29 07:41Z, pinned one model; also
    spec-f09094 and gap-e21595.

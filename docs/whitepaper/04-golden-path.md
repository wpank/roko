Status: draft · budget 950 words · owner gap-ac4646

# 4 The golden path, step by step

The golden path is the thesis in motion: eleven steps from a request to verified, integrated work that improves the
next plan. Step N is §4.N. Each gives the design, tagged as in the status matrix (the appendix, at `a17d4dadd`), and
the principle from §2 behind it; §4.12 names the epics that would close the gaps.

Figure 2: The golden path. Failures loop back from step 8, and outcomes feed the next plan; tags are at `a17d4dadd`.

<!-- gap-d1d92c draws this as figures/fig2-golden-path.svg; the image include then replaces the text diagram. -->

```text
PLAN         1  Author               PARTIAL
frontier     2  Compile the spec     PARTIAL
model        3  Size and split       MISSING     <-- split or replan, from 8
                    |
                    v
EXECUTE      4  Route                MISSING     <-- retry or one rung up, from 8
cheapest     5  Schedule             PARTIAL
capable      6  Isolate              PARTIAL
model        7  Verify               PARTIAL     --> fail: 8, pass: 9
             8  Recover              PARTIAL
                    |
                    v
INTEGRATE    9  Integrate            ORPHANED, MISSING
            10  Review               MISSING
                    |
                    v
LEARN       11  Learn                PARTIAL     --> steps 1 to 4, next plan
```

## 4.1 Author

A frontier model drafts the plan with the author, who edits and regenerates it quickly: PARTIAL@a17d4dadd. Only some
CLI paths can choose the planner model (gap-853b31), the prompt (`crates/roko-cli/src/prd.rs`) cuts the PRD at 8,000
characters and lets the planner read at most five files, and the portal's plans were written outside Roko (§7).
Principle: *put ambiguity back into authoring*.

## 4.2 Compile the spec

Each task compiles into exact context, acceptance criteria bound to checks, and planner-written tests proven to fail
on the base: PARTIAL@a17d4dadd. Context packs are WIRED@a17d4dadd (`render_declared_context`). Acceptance criteria are
prompt text; the pinned `accept/` tests that served the portal are a hand convention the agent can still edit
(gap-d14a43); and nothing proves that a check fails before the change. Principle: *the planner writes the checks*.

## 4.3 Size and split

Task size follows the executor tier's measured pass rate, and tasks that may run together write disjoint files:
MISSING@a17d4dadd. The generator's prompt prefers "the fewest cohesive tasks", and nothing compares the files of
concurrent tasks. Principles: *size tasks for the executor*; *split on independent outputs*.

## 4.4 Route

Each task runs on the cheapest model that passes its checks, chosen from a role × tier ladder: MISSING@a17d4dadd.
Today a person picks the model, through `--model` or the task's hint; failing both, the learned router picks, and it
is PARTIAL@a17d4dadd because its learned stage updates on successes only (bug-8da8ba). The tier names plans use, such
as `focused` and `mechanical`, all route as Standard. Principle: *count cost per verified task*.

## 4.5 Schedule

Ready tasks dispatch at once, as wide as the DAG and the write sets allow: PARTIAL@a17d4dadd. Since `bbf6517fc` each
task starts when its own dependencies settle, and a failure skips only its dependants (`SkipFailed`, gap-96d348). But
`max_parallel` is 1 by default and in the generator's template: 102 of the 132 tracked plans set it to 1, 25 of them
with tasks that could run together,[^4-plans] and nothing admits tasks by write set. Principle: *split on independent
outputs*.

## 4.6 Isolate

Each task works in its own workspace, out of reach of the operator's secrets: PARTIAL@a17d4dadd. Tasks edit the
operator's checkout by default, and the opt-in per-task worktree never merges its edits back. Since `1d923e377`, gates
start from an allowlisted environment and provider CLIs drop the key variables they recognise (bug-7d7200), but agent
tool shells and MCP servers still inherit the keys, there is no OS sandbox, and the git guard lets `reset`, `stash`
and `clean` through.

## 4.7 Verify

Every attempt faces its visible checks plus checks it cannot see or edit: PARTIAL@a17d4dadd. Only the task's own,
visible verify commands run. Verdicts have been honest since the fix of 2026-09-28, with 0 false greens in 151 passes,
down from about 27% of recorded successes,[^4-verdicts] but `Unverified` still counts as a pass for learning and the
dashboard. Tamper, scope and hidden-test checks are MISSING@a17d4dadd. Principle: *assume visible checks are gamed*.

## 4.8 Recover

A failed check earns two cheap retries with the distilled gate errors, then one rung up the ladder, then a split or
replan: PARTIAL@a17d4dadd. Retries with the parsed gate output are WIRED@a17d4dadd, on the same model: three by
default, or a budget set from gate history since `99adacd6d`. Escalation was deleted with Runner-v2
(ORPHANED@a17d4dadd), the `ReplanController` is BUILT-UNWIRED@a17d4dadd, and a watchdog for stalled agents is
MISSING@a17d4dadd. Principle: *retry cheaply, then escalate*.

## 4.9 Integrate

Each verified task commits to a plan branch through a merge queue (ORPHANED@a17d4dadd), and a whole-plan gate checks
the merged result (MISSING@a17d4dadd). Nothing merges on the Graph path: `MergeQueue` and `accept_attempt` have no
production caller. On 2026-09-28 portal plans 05 to 08 passed every gate while the assembled product was unusable (§7).
Principle: *merge, then verify*.

## 4.10 Review

An optional hold lets a person approve each task's diff before it merges: MISSING@a17d4dadd. `--approval` only opens
the TUI, and the per-task diff route looks for `agent/*/<task>` branches that Graph runs never create. No epic covers
this step yet.

## 4.11 Learn

Verified outcomes teach Roko how to route, size and specify tasks: PARTIAL@a17d4dadd. Merges on 2026-09-29
(`ce3bdcbb8`, `33e107da1`) re-wired playbook credit, prompt experiments, knowledge write-back and retry budgets from
gate history. But no loop has a measured benefit, the knowledge read path, BROKEN@a17d4dadd, injects nothing
(bug-86117a), and on the portal the router saw one pinned model (§4.12). Principle: *count cost per verified task*.
§5 covers the loops.

## 4.12 Where the path stands

Each step's tag, its appendix rows, and the epics that would change it:

| Step | Status | Matrix rows | Next |
|---|---|---|---|
| 1 Author | PARTIAL@a17d4dadd | AU1, AU2, V2 | E8 specs (spec-e57870) |
| 2 Compile the spec | PARTIAL@a17d4dadd | AU3, AU4, AU6, AU7 | E8 specs |
| 3 Size and split | MISSING@a17d4dadd | V3 | E8 specs; E7 scheduler (spec-a78d57) |
| 4 Route | MISSING@a17d4dadd | RC2, V3 | E5 tier ladder (spec-98f76d) |
| 5 Schedule | PARTIAL@a17d4dadd | EX3, V4 | E7 scheduler |
| 6 Isolate | PARTIAL@a17d4dadd | IS1, IS4, IS5, IS6 | E6 integration (spec-a0e40a); E3 secrets (spec-ba7bea) |
| 7 Verify | PARTIAL@a17d4dadd | QA1, QA2, QA3, QA4, QA5 | E2 verdicts (spec-e9d7ec); E9 diff check (spec-9230a9); E17 audits |
| 8 Recover | PARTIAL@a17d4dadd | EX6, EX7, EX8, EX9, QA7 | E5 tier ladder; E10 watchdog (spec-edda86) |
| 9 Integrate | ORPHANED@a17d4dadd, MISSING@a17d4dadd | IS2, IS3 | E6 integration |
| 10 Review | MISSING@a17d4dadd | SS6 | None yet |
| 11 Learn | PARTIAL@a17d4dadd | V8, LM3, RC2 | E17 cybernetic core (spec-6ac537) |
| Whole path: cheaper at equal quality | UNPROVEN@a17d4dadd | V7 | E11 acceptance tests (spec-f09094); E12 pilot (spec-567e52) |

No step yet does everything its design asks, and the central promise is untested. Every portal attempt pinned
`claude-sonnet-4-6`[^4-model] and escalation is not wired, so no real task has yet run on a cheap executor with
escalation. E11's end-to-end run (gap-f30b8e) is designed as the first test of the whole path: a fixture plan runs
through the ladder on real cheap models and merges green, three runs out of three. §8 plans the comparison with a
frontier agent.

[^4-plans]: Counted at `a17d4dadd` over the 132 tracked `tasks.toml` files under `plans/`;
    `git grep -l -E '^max_parallel *= *1( |#|$)' a17d4dadd -- 'plans/*tasks.toml'` lists the 102. Tasks could run
    together when two sit at the same depth of the plan's dependency graph (parsed with `tomllib`).
[^4-verdicts]: Research note B7, frozen as `evidence/2026-09-29-b7-real-run-evidence.md` (sha256 `799b6a2b6184`),
    "TL;DR", and CASE-001 in `evidence/2026-09-29-field-cases.md`; also spec-e9d7ec. Visible verify steps only:
    recorded successes from 2026-09-05 to the fix, then passes to 2026-09-29 07:41Z. Not an audited false-green rate.
[^4-model]: B7 as above, "TL;DR": all 210 portal attempts, to 2026-09-29 07:41Z, pinned one model; also
    spec-f09094 and gap-e21595.

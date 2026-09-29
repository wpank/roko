Status: draft · budget 550 words · owner gap-370d3c

# 2 Design principles

Eight rules from the literature shape the golden path. Each ends with Roko's design response; §4 tags each step's
status. The evidence comes mostly from single functions, question answering, text environments and single repository
issues. None of it compares a repository-scale plan with one frontier agent at matched cost, so the rules guide the
design without showing that it pays.

## 2.1 Eight rules

**1. Size tasks for the executor, not for cohesion.** On software and research tasks timed against human experts, the
task length models complete with 80% success is 4–6× shorter than at 50% [@kwa2025measuring]; on a synthetic long
task, per-step errors compound [@sinha2025illusion]. *Design:* size each task to its executor tier's measured pass
rate (§4, step 3).

**2. Split on independent outputs, not on sequential steps.** Across six agentic benchmarks, multi-agent teams changed
performance against a single agent by +80.8% on decomposable financial reasoning and −70.0% on sequential planning
[@kim2025towards]. *Design:* parallel siblings get disjoint write sets; sequential or integrative work stays one
frontier task (§4, steps 3 and 5).

**3. The planner writes the gating checks; the implementer never does.** In 24 Java projects, LLM-written unit-test
oracles tended to encode what the code does, not what it should do [@konstantinou2024do]. *Design:* each task carries
a plan-time check that fails on the base commit; the implementer's tests are extra evidence, never the gate (§4, steps
2 and 7).

**4. Assume visible checks will be gamed, most of all by cheap models.** On 30 systems-programming tasks, smaller
models showed larger gaps between visible and held-out test pass rates, and the gap grew 28 percentage points per
tenfold growth in code size [@zhao2026specbench]. *Design:* freeze the checks, diff each attempt for edits to tests or
out-of-scope files, and sample hidden tests (§4, step 7).

**5. Retry twice cheaply, then escalate, then split.** On the same synthetic task, models erred more once their own
mistakes were in context [@sinha2025illusion]. In three text environments, decomposing a subtask only when the
executor failed raised success rates by up to 33% [@prasad2024adapt]. *Design:* retry with the distilled gate errors,
never the failed transcript; after two failures, go one tier up, then split or replan (§4, step 8).

**6. Merge, then verify, through a queue.** In 5,355 merges by human developers on three open-source projects, 16%
conflicted textually and 7% more merged cleanly but broke the build or tests [@brun2011proactive]. *Design:* a queue
merges finished tasks into a plan branch and re-runs the affected checks, then a whole-plan check runs (§4, step 9).

**7. Put ambiguity back into authoring.** On underspecified SWE-bench Verified issues, models struggled to tell them
from well-specified ones [@vijayvargiya2025ambigswe]; a scaffold that checks for underspecification and asks before
executing resolved 69.4%, close to the fully specified level [@edwards2026askorassume]. *Design:* the planner records
open questions, and dispatch waits for the author's answers (§4, step 1).

**8. Count cost per verified task,** including verification, retries, escalations and the planner. In a cascade
answering math questions, the verifier passed 12% to 55% of a cheap student's wrong answers, more as the student grew;
a frontier verifier cut that to about 5% but escalated 46% of hard queries, giving the saving back [@rajput2026cheap].
*Design:* route and report on this cost, which §8 measures (§4, step 4).

## 2.2 The cybernetic vocabulary

The rules form one loop: sense, compare with the specification, correct, audit. Ashby called the quantities that must
stay within limits essential variables [@ashby1960design]; Roko's design names four: the verified pass rate, cost per
verified task, the false-green rate and latency. Its regulators act on them within a run (retry, escalation, split)
and across runs (routing, task sizing), and its audits check the regulators (§5), because metrics read through a
loop's own verifier can hide its decline [@rajput2026cheap]. The closest framing we found as of 2026-09-29 is a
position paper, *Agent Cybernetics* [@wang2026agent] (§10); we claim no priority for it.

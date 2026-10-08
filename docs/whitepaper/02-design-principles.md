Status: draft · budget 650 words · owner spec-ce1484

# 2 Design goals and principles

This section states Roko's design goals, defines the cybernetic terms the paper uses, and sets out eight principles.

## 2.1 Goals

**Table 2.1.** Design goals, their main mechanisms and sections.

| Goal | Main mechanisms | § |
|---|---|---|
| **Trustworthy completion:** done means a check the worker cannot change has passed | Planned checks; the pre-verify screen; the gate rungs; the verdict record | 4, 6 |
| **Right-sized models:** the cheapest model that passes | The escalation ladder; provider failover; the learned model router; per-call budget reservations | 4, 5 |
| **Parallel without collisions** | Write sets; a worktree per task; plan and run branches; the whole-plan check on the merge | 4 |
| **Recoverable:** stop and resume, keeping verified work | Checkpoints; verdict records; operator controls | 4 |
| **Learns from outcomes, not self-report** | One learning label, read by every learner; the loop registry | 5 |
| **Checks its own checks and learning** | Random deep audits; the loop audit; the bounded controller; guarded commit | 5, 6 |
| **Safe by default:** unknown actions are denied | The tool-call guard; role contracts; fail-closed tools; the git guard; credential scrubbing; isolation controls | 8 |
| **Provider- and domain-neutral** | One adapter per provider kind; any shell command as a check | 3, 4 |
| **Observable** | Content-addressed signals with lineage; verdict, episode and cost records; the state hub | 3, 6 |

## 2.2 Cybernetic terms

A *feedback loop* measures a result with a *sensor*, compares it with a *reference* (its goal) in a *comparator*, and
acts on the difference, the *error*, through an *actuator*; *feedforward* acts before a problem shows up in the result
[@wiener1948cybernetics]. By the *law of requisite variety*, a regulator can absorb only as many kinds of
*disturbance* (anything that pushes a result off target) as it has kinds of response [@ashby1956introduction].
*Essential variables* are the few quantities that must stay within bounds for a system to keep doing its job; an
*ultrastable* system adds a slow loop that retunes a fast one only when one of them leaves its bounds
[@ashby1960design]. A *regulator card* lists a loop's goal, reference, sensor, comparator, actuator, bounds, clock and
records.

## 2.3 Principles

**1. Make every loop explicit.** An undeclared loop cannot be audited, and implicit feedback loops are hidden technical
debt in machine-learning systems [@sculley2015hidden]. Each Roko loop has a regulator card, and a registry declares
each learning loop's decision, default and receipt.

**2. Write the reference before the work.** The planner, never the implementer, writes each task's checks: in Java
projects, model-written test oracles tended to encode what code does, not what it should do [@konstantinou2024do]. A
check for new behaviour must fail on the unchanged base.

**3. Keep each sensor out of its actor's reach.** On systems-programming tasks, agents passed visible tests more often
than held-out ones, smaller models most [@zhao2026specbench]. Roko pins the planner's acceptance tests outside every
working tree, screens attempts for tampering before tests run, and audits with hidden tests.

**4. Separate the clocks.** Each loop runs much slower than the loop it tunes and changes its settings, never its
decisions in flight [@ashby1960design]; correction that is too fast or too strong oscillates [@wiener1948cybernetics].

**5. Judge by a few essential variables:** verified pass rate, cost per verified success, false-green rate and p90
latency, against bounds that people set. In a model cascade on math questions, the error its dashboard showed stayed
flat while the delivered error rose [@rajput2026cheap], so false greens are estimated from independent audits.

**6. Match variety to disturbance.** Each declared disturbance has a designed response: retry with gate feedback, a
stronger model, a provider switch, a split, a replan, or a halt with an alert. The failure's blame picks among them
(§4).

**7. Bound and isolate every action.** Each attempt runs in its own worktree, and integration runs the whole-plan
check on the merge, since clean merges can still break the build [@brun2011proactive].
Regulators tune how Roko spends and checks, never what counts as correct: no slower loop widens permissions or removes
an authored check, and guarded commit can roll back any self-modification.

**8. Record once, audit every loop.** Every decision leaves one attributable record, and every learner reads the same
learning label. Deutero-learning asks whether learning helped [@argyris1978organizational]; the loop audit asks it of
every learning loop against a holdout, and a loop that shows no benefit reverts to its default.

Status: draft · budget 450 words · owner spec-ce1484

# 9 Trade-offs and open problems

This section sets out the design's trade-offs, when Roko is the wrong tool, and four problems it shares with the field.

## 9.1 Trade-offs

**Table 9.1.** Five design choices, with their gains and costs.

| Choice | Gains | Costs |
|---|---|---|
| A worktree per attempt | Attempts cannot disturb each other or the operator's checkout; every change traces to one attempt | Disk (3 GiB reserved per attempt) and merge work: a conflicting attempt is redone on the plan branch's tip[^9-disk] |
| Independent audits | An estimate of the false-green rate, which no gate can give about itself | Extra runs on work that already passed |
| Deterministic regulators | Behaviour that people can predict, replay and audit | Slower adaptation when the pool of models changes |
| Fail-closed policy | No unknown action runs unnoticed | Autonomy: a task that needs an unlisted tool waits for a person |
| Small tasks | Local failures, narrow checks that suit cheap models, parallel work | Planning effort and more integration points |

In each row Roko takes a property that a slower loop can check over a saving that would leave nothing to check.
Planning is the cost that agent spend hides, as in the portal build (§7).

## 9.2 When Roko is the wrong tool

Three kinds of work do not repay Roko's planning, worktrees, checks and integration: one-off interactive edits, where
a person watching one session is the check; work without executable checks, which gives every loop above it a weak
reference; and tasks that cannot be split, which leave nothing to schedule in parallel and no narrow check to run.

## 9.3 Open problems

**Audit independence versus cost.** Passing tests and being correct differ [@wang2025solved; @yu2025utboost]. Each
step an audit takes away from the work (a clean re-run, hidden tests from another model family, mutation) costs runs,
and model families may share blind spots. How to price independence against the false greens it reveals is open.

**Checks becoming targets.** A measure that is optimized against tends to stop measuring what it was chosen for
[@manheim2018categorizing], and frontier models have tampered with the code that scores them [@vonarx2025recent].
Roko keeps checks out of the actor's reach, screens diffs for tampering and rotates hidden suites; whether that keeps
pace with stronger models is open.

**Regulator autonomy.** Ashby's ultrastable system changes its own parameters until its essential variables return
within limits [@ashby1960design]; Roko's bounded controller is a narrow version. Wider autonomy might find better
settings sooner, but interacting learners form feedback paths that nobody declared [@sculley2015hidden]; where the line
belongs is open.

**Spec quality.** Every loop steers toward references written before the work, so a vague spec yields faithful work on
the wrong thing, and agents struggle to tell well-specified tasks from underspecified ones
[@vijayvargiya2025ambigswe]. Roko scores specs and refuses checks that cannot fail; measuring a spec's quality before
the work is open.

[^9-disk]: Design default (`1f860d408`): `WORKTREE_GROWTH_ESTIMATE_MB` in
    `crates/roko-cli/src/graph_execution/disk_admission.rs`.

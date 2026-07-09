# E — Process Rewards & Evaluation Lifecycle (DEFERRED)

Docs `07` and `09` describe a target-state learning architecture. They should not be treated as current verification runtime scope.

---

## Audit Posture

This material is **research-grade / deferred**.

The parity job here is not to enumerate every absent concept again. It is to make the reader stop confusing data foundations with a shipped process-reward system.

---

## What Is Real Today

Keep only these small truths in present tense:

- efficiency events are persisted
- completed runs and episodes are persisted
- gate outcomes feed downstream learning records
- some narrow skill/playbook updates happen after successful runs

Those are data and learning foundations. They are **not** the same thing as Promise/Progress scoring or a live 14-loop evaluation architecture.

---

## What Must Be Deferred

Move these to explicit future-work / target-state language:

- Promise score
- Progress score
- process reward models
- PRM / ThinkPRM / DPO / RLAIF layers
- Gauntlet benchmark harness
- the 14-loop lifecycle as a current runtime claim
- Karpathy-property enforcement as an implemented system guarantee

---

## Recommended Wording

Use wording like:

- “data foundations exist”
- “some feedback loops are partially instrumented”
- “the reward-model architecture is planned / deferred”

Avoid wording like:

- “the system currently scores Promise and Progress”
- “the 14-loop evaluation lifecycle is wired”
- “process rewards already govern retries or replanning”

---

## Ownership

If this material is revived later, it belongs with learning-focused work, not verification-core parity work.

For this batch, the correct action is simple:

- keep the small current truths
- mark the rest `DEFERRED`

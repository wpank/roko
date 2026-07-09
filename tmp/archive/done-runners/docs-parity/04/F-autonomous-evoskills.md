# F — Autonomous Evaluation & EvoSkills (DEFERRED)

Docs `10` and `11` should be treated as target-state material, not current verification runtime.

---

## Audit Posture

This section is **DEFERRED**.

The old parity note mixed together three very different things:

- real learning foundations
- a consumer-side generated-test gate
- a large autonomous-eval / EvoSkills research program

Those need to be separated.

---

## What Is Real Today

These parts are current-code truths:

- episodes are real
- playbook rules are real
- skill extraction and skill injection exist in narrow form
- generated-test support exists on the gate-consumer side

That is enough to say the project has learning and reuse primitives.

---

## What Is Not Current Runtime

These should be explicitly deferred:

- autonomous test-writer agent loops
- adversarial generation/validation workflows
- cheap-model convergence loops for autonomous eval
- EvoSkills promotion/evolution systems
- cross-model validation claims
- MAP-Elites, speciation, AURORA, CMA-ES, or similar evolutionary layers

The docs should not imply these are implementation gaps inside `04`. They are later-stage research ideas.

---

## GeneratedTestGate Wording

One narrow truth is worth keeping:

- `GeneratedTestGate` exists as a verification-side consumer

But the pack should immediately clarify:

- the autonomous generator side is not part of the current shipped system

That is the cleanest split between “exists” and “planned.”

---

## Recommended Wording

Use wording like:

- “episodes / skills are current”
- “autonomous evaluation generation is planned”
- “EvoSkills remains deferred research”

Avoid wording like:

- “the system already runs autonomous eval generation”
- “EvoSkills is an active optimization layer”

---

## Ownership

If this work is revived later, it belongs with learning/research planning, not the shipped verification-core parity story.

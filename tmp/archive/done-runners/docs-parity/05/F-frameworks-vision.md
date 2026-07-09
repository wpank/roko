# F — Frameworks + Vision

Audit-corrected parity view of the framework-mapping and long-range vision docs.

---

## Keep The Useful Part

The framework docs are still useful when they do one narrow thing well:

- map existing Roko modules onto outside ideas,
- explain why a reader might recognize a pattern,
- point at the shipping code rather than pretending the paper itself was implemented whole.

That part should stay.

## What Must Be Narrowed

- FEP / Friston / VSM language should be treated as **academic framing**, not as the engineering plan.
- `active_inference.rs` already exists, so the framework prose should not imply that the main missing work is to import that theory.
- c-factor should remain a measurement and exploration surface, not the canonical objective function for the whole system.

## What Must Be Deferred

These concepts should be explicitly labeled `planned`, `target-state`, or `future work`:

- worldviews
- replication-ledger / claim-tracking systems
- constitutional constraints and improvement-governance layers
- demurrage as the learning economy
- ADAS and EvoSkills as runtime optimization systems
- exponential-scaling or autocatalytic theses as engineering commitments

## Practical Rewrite Guidance

When touching the framework docs:

1. keep the concrete framework-to-module mappings that are already true,
2. move worldview / replication-ledger / constitutional material into explicit future-work sections,
3. keep the language grounded in code that exists today.

## Batch-Ready Follow-Up

- `L8`: demote the overscoped framework and thesis material to explicit future work

## Source Anchors

- `crates/roko-learn/src/playbook_rules.rs:341` — Reflexion-like confidence updates
- `crates/roko-learn/src/skill_library.rs` — experience extraction / Voyager-style accumulation
- `crates/roko-learn/src/prompt_experiment.rs:135` — prompt experiments
- `crates/roko-learn/src/cascade_router.rs:994` — routing core
- `crates/roko-learn/src/active_inference.rs:17` — active inference already lives in code
- `crates/roko-learn/src/cfactor.rs` — c-factor as a live measurement surface

## Bottom Line

The vision material still has value, but only if it stops speaking about research hypotheses as if they already define the runtime. The parity refresh should preserve the mapping value and explicitly defer the rest.

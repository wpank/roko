# S10 — TypedContext × domain profiles × Gate → Auditable domain safety

> Domain profiles package the behavior of a specific operating domain. TypedContext carries the
> structured situation. Gates evaluate typed predicates instead of free-text guesses, and Custody
> records who acted, why, and with what evidence. Domain-sensitive actions remain inspectable
> after the fact without each team inventing its own ad hoc logging stack.

**Status**: Analysis — partially live / target-state synergy  
**Primitives involved**: TypedContext × P10 domain profiles × Gate operator  
**Reality check**: The Gate operator is Built. TypedContext exists in Scaffold form. Domain
profiles (P10) are Scaffold. Custody recording is Specified. Full auditable domain safety is
target-state.  
**Last reviewed**: 2026-04-19

---

## Primitives Involved

| Primitive | Role in this synergy |
|---|---|
| TypedContext | A structured record of the current situation: who is acting, in what role, with what authorizations, and under what constraints — passed through the pipeline in a type-safe way |
| [P10 domain profiles](../../subsystems/) | Bundles of policy, gate configuration, role roster, and heuristic overrides for a specific operating domain (e.g., "financial compliance", "code review", "medical") |
| Gate operator | The typed predicate evaluator in the pipeline; evaluates whether an action is permitted given the current TypedContext and the active domain profile's gate configuration |
| Custody record | The immutable audit log entry written when a gate fires — recording who acted, what decision was made, what evidence was presented, and under which domain profile |

---

## What the Synergy Unlocks

### The ad hoc safety problem

Every team that deploys an AI system in a sensitive domain independently solves the same set
of problems: "who is allowed to do X?", "what evidence did they have when they did it?", "can
we explain this decision after the fact?". Without a shared structure, each team builds its own
logging, its own permission checks, and its own audit trail — with different schemas, different
coverage guarantees, and different failure modes.

The synergy provides a shared structural answer: if domain-sensitive actions always pass through
typed gates that read from a TypedContext and write to a Custody record, then auditability is
a property of the architecture, not of each individual team's implementation.

### How it works

1. When an agent operates in a sensitive domain, the session is initialized with the appropriate
   domain profile. The profile specifies which gates are active, what thresholds they use, and
   what TypedContext fields are required.
2. Every action candidate is passed through the gate pipeline with the current TypedContext.
   Gates evaluate typed predicates (e.g., `context.authorization_level >= profile.required_level`)
   rather than free-text strings or regex heuristics.
3. When a gate fires (allow or deny), it writes a Custody record: a structured, immutable log
   entry that captures the full decision context — the TypedContext values, the predicate
   evaluated, the outcome, and the active domain profile version.
4. The Custody record is stored in Substrate. It is first-class queryable history, not a
   supplementary log file.
5. After the fact, an auditor (human or automated) can query Substrate for all Custody records
   in a time range and reconstruct exactly what happened, why, and under what domain
   configuration — without needing application logs.

The result: domain-sensitive actions are inspectable by design. The audit trail is not a side
effect of implementation; it is a structural guarantee from the architecture.

### Why typed predicates matter

Free-text gate conditions ("check if the user has permission to do this") are not auditable.
They are opaque to tooling, cannot be validated at deploy time, and cannot be tested without
running the full LLM pipeline. Typed predicates are the opposite: they are machine-readable,
statically checkable, and produce deterministic results given the same TypedContext.

The switch from "ask the model" to "evaluate a typed predicate" is what makes the gate output
reproducible and therefore auditable.

---

## What Flows

```
Session initialization:
  domain_profile → load gate configuration, required TypedContext fields
  → activate profile for session

Per-action gate evaluation:
  action_candidate + TypedContext → Gate.evaluate(predicate, context)
  → outcome: Allow | Deny | Escalate

Custody write (on every gate evaluation):
  Custody.record({
    action_id,
    agent_id,
    typed_context_snapshot: TypedContext.snapshot(),
    gate_id,
    predicate_evaluated,
    outcome,
    domain_profile_version,
    timestamp
  }) → Substrate

Audit query:
  Substrate.query(filter=CustodyRecord, time_range=...) → ordered log
```

---

## Invariants

1. Every gate evaluation that produces an outcome writes a Custody record. There are no
   "silent" gate evaluations.
2. Custody records are immutable. They cannot be modified after write. They can only be
   queried or (eventually) archived.
3. The domain profile version is recorded in every Custody record. If the profile is updated,
   all subsequent records carry the new version; previous records are unaffected.
4. A TypedContext snapshot is taken at gate-evaluation time, not at action-initiation time.
   This ensures the Custody record reflects the actual state at the decision point.
5. Gates do not have side effects other than the Custody write. They do not send messages,
   update counters, or modify Substrate records beyond the Custody log.

---

## Failure Modes

| Failure | Mechanism | Mitigation |
|---|---|---|
| Predicate bypass | An action reaches execution without passing through the gate pipeline | Enforce gate evaluation as a mandatory step in the Cognitive Loop's ACT phase; fail-closed if skipped |
| TypedContext spoofing | A malicious or buggy agent injects false values into TypedContext before gate evaluation | TypedContext is constructed by the orchestrator, not by the acting agent; agents cannot write to it |
| Custody log overflow | High-frequency actions produce a very large Custody log quickly | Custody records are first-class Engrams subject to demurrage; old, low-priority records decay per S1 |
| Profile version mismatch | An action is evaluated against an old profile version due to caching | Invalidate profile cache on version bump; re-load profile at session start |
| Gate predicate error | A typed predicate throws an exception at evaluation time | Fail-closed: predicate errors produce Deny outcomes and write an error Custody record |

---

## Relationship to Other Synergies

- **S5** (Plugin SPI × Substrate × Bus): Domain profiles are installed via the Plugin SPI.
  S5 explains how profiles are loaded and activated; S10 explains what they do at runtime.
- **S1** (Demurrage × HDC): Custody records are Engrams in Substrate. Over time, demurrage
  applies to them. Long-past audit records eventually decay unless they are flagged as
  long-retention by the domain profile.
- **S3** (c-factor × Bus × HDC): If domain profiles restrict the pool of eligible agents, c-factor
  still monitors for monoculture within that pool and can signal for rotation.

---

## Today vs. Planned

**Today**: Gate operator is Built. TypedContext exists in Scaffold form. Domain profiles as
formalized install-able bundles do not exist. Custody records are not written to Substrate as
first-class queryable Engrams.

**Planned**: TypedContext gains required fields per domain profile. Domain profile versioning
ships. The Gate operator writes Custody records to Substrate on every evaluation. Audit query
tooling is added to the CLI.

---

## Cross-References

- [`analysis/integration-map/safety-x-composition.md`](../integration-map/safety-x-composition.md) — M13: safety-composition integration edge
- [`analysis/integration-map/safety-x-agents.md`](../integration-map/safety-x-agents.md) — M8 (wired): safety-agents integration edge
- [`analysis/readiness-audit/subsystem-safety.md`](../readiness-audit/subsystem-safety.md) — safety subsystem gaps
- [`analysis/synergy-map/synergy-05-plugin-spi-ecosystem.md`](synergy-05-plugin-spi-ecosystem.md) — S5: domain profile installation
- [`analysis/synergy-map/99-master-synergy-table.md`](99-master-synergy-table.md) — synergy index

---

## Open Questions

- What is the minimum required TypedContext schema? At minimum: actor identity, authorization
  level, domain scope, action kind, timestamp. Are session lineage fields required?
- Should Custody records have a configurable retention policy per domain profile, or should
  all Custody records share a single system-wide retention setting?
- Can the gate predicate language be extended by domain profiles (custom predicates), or is
  the predicate set fixed in the core? Custom predicates raise auditability questions if they
  can call arbitrary code.

# Simpler Target Architecture

This file proposes leaner mechanisms for the areas where the original
refinements got too abstract, too universal, or too hard to validate.

## The simpler shape

The target-state can be described without the heaviest rhetoric:

1. durable records live in `Substrate`;
2. live messages move on `Bus`;
3. user surfaces consume `Projection`s;
4. `Policy` reacts;
5. `Calibrator` learns;
6. domain profiles package tools, gates, and defaults;
7. safety and observability wrap the whole system.

That is already enough architecture to build a strong product.

## Better replacements for the most overbuilt mechanisms

### 1. Replace universal active inference with expectation/outcome loops

Instead of:
- every operator as a full active-inference agent;
- broad `prediction.error.*` ideology everywhere;

prefer:
- operator-local expectation/outcome records;
- calibration updates on observed mismatch;
- bandits or threshold tuning where outcomes are frequent.

Why this is better:
- easier to test;
- easier to explain;
- still gives learning loops without importing a total theory.

### 2. Replace demurrage-first memory with retention tiers plus pressure

Instead of leading with:
- demurrage as the memory model;

prefer:
- hot / warm / cold retention tiers;
- promotion and demotion thresholds;
- optional retention pressure as a tuning factor;
- reinforcement based on use, citation, and surprise.

Why this is better:
- operators already understand tiered retention;
- the system can ship useful memory behavior before nailing the economics;
- the docs stop overselling one metaphor.

### 3. Replace worldview algebra with contradiction management

Instead of:
- worldview objects;
- dissonance stacks;
- broad epistemic theater;

prefer:
- typed claims;
- heuristic specs;
- contradiction queue;
- challenger slots and re-test workflows.

Why this is better:
- contradictions become work items;
- diversity gets a concrete mechanism;
- the system stays empirical rather than philosophical.

### 4. Replace c-factor control doctrine with coordination health plus challengers

Instead of:
- a single collective-intelligence scalar driving runtime behavior;

prefer:
- `coordination health` as an observability concept first;
- challenger slots to keep alternative strategies alive;
- periodic re-tests of minority heuristics;
- explicit cohort-health projections in UI.

Why this is better:
- the signal can mature before it governs decisions;
- users can inspect it without treating it as magic;
- diversity is maintained by policy, not one number.

### 5. Replace registry-first platform language with capability-first lifecycle

Instead of:
- marketplace, registry, ecosystem, ABI promises;

prefer:
- local install;
- inspect and audit;
- explicit enable;
- capability-scoped runtime behavior;
- disable and remove.

Why this is better:
- safer local-first extensibility;
- better operator control;
- less platform theater.

### 6. Replace raw-event UX with projection-first UX

Instead of:
- exposing raw transport concepts to most users;

prefer:
- projection and session streams as the public product model;
- raw topics for privileged and debug consumers only;
- one query+subscribe contract for all surfaces.

Why this is better:
- simpler mental model;
- more stable clients;
- less surface-specific state code.

## Architecture cuts that improve the redesign immediately

### Cut 1. Do not make `Datum` a public doctrine

If a shared either-medium enum is useful internally, keep it internal or narrow.
Do not make it the center of the architecture story.

### Cut 2. Do not standardize all transports and all projections at once

First define:
- cursor;
- subscription;
- state;
- delta;
- query;
- replay.

Everything else can layer on top.

### Cut 3. Do not let one noun carry both user meaning and implementation meaning

Examples:
- `Projection` vs `StateHub`
- `Policy` vs `Calibrator`
- `domain profile` vs `runtime shape`

This cut alone removes a lot of ambiguity.

### Cut 4. Do not let safety become governance sprawl

Keep a small set of enforceable contracts:
- action authorization;
- capability boundaries;
- provenance and audit;
- human approval points.

### Cut 5. Do not let the browser define the product model

The web surface should prove the shared projection contract, not invent parallel
state machinery or demand premature parity.

## Lean build order

If the redesign follows this simpler target, the first meaningful sequence is:

1. three-lane rule and canonical vocabulary;
2. cursor, subscription, projection contracts;
3. session as shared work object;
4. policy/calibrator split;
5. heuristic spec/calibration split;
6. contradiction queue;
7. capability-gated plugin lifecycle;
8. retention tiers, then optional retention pressure;
9. coordination-health projections, then optional actuation.

## Short conclusion

The strongest version of the redesign is not smaller in ambition. It is smaller
in doctrine. It keeps the powerful seams and drops the parts that try to make
the whole system answer to one theory, one metric, or one metaphor.

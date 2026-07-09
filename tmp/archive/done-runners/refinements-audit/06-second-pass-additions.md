# Second-Pass Additions

This file adds net-new candidate refinements beyond the original 35. These are
not code-reality complaints. They are missing target-state seams that would
make the redesign more buildable, more legible, and easier to operate.

## High-priority additions

### ADD01. Three-lane kernel rule

The redesign needs one explicit law:
- durable record lane: `Engram` + `Substrate`;
- live transport lane: `Pulse` + `Bus`;
- derived view lane: `Projection` + `StateHub` host.

If a public concept straddles lanes, split it.

Why add this:
- it reduces category confusion;
- it makes docs easier to reason about;
- it gives a hard test for future abstractions.

### ADD02. Cursor and subscription contract

The public model should resume from a `Cursor`, not expose raw bus sequence
numbers as the main mental model.

Also promote `Subscription` as the public lifecycle object:
- subscribe;
- receive state or events;
- resume from cursor;
- unsubscribe.

Why add this:
- replay and resume become one consistent story;
- CLI, TUI, web, and external clients can share semantics;
- it prevents low-level transport details from leaking upward.

### ADD03. Projection versioning and migration

Every projection should carry:
- schema version;
- compatibility policy;
- migration rule;
- tests for state and delta evolution.

Why add this:
- projections will become a public contract;
- UI and integrations will otherwise be brittle;
- versioning discipline is cheaper than projection churn.

### ADD04. Session as the shared work object

Make `session` the cross-surface unit of work. A session should own:
- transcript;
- cursor;
- permissions;
- active domain profile;
- replay state;
- current task or plan lineage.

Why add this:
- "resume" becomes portable across CLI, TUI, chat, and web;
- users think in sessions more naturally than in transport internals;
- the product gains one stable object to organize around.

### ADD05. Command registry behind every surface

Define commands once, render them many ways:
- CLI subcommand;
- slash command;
- command palette;
- button or menu item.

Why add this:
- surface parity becomes a rendering problem instead of a documentation problem;
- help text, permissions, and auditability can attach to one command registry;
- UX stops drifting across interfaces.

### ADD06. Split Policy from Calibrator

`Policy` should react and decide.
`Calibrator` should learn from outcomes and update weights, thresholds, and
heuristics.

Why add this:
- control and learning are different jobs;
- it prevents one omnipotent "policy" bucket from swallowing everything;
- it creates a cleaner seam for experiments.

### ADD07. Split HeuristicSpec from Calibration

A heuristic should not be one undifferentiated blob. Split it into:
- `HeuristicSpec`: rule, scope, rationale, counterexample shape;
- `Calibration`: hit rate, drift, confidence, challenge history.

Why add this:
- rules and evidence evolve at different speeds;
- the UI can show "what this heuristic says" separately from "how well it is
  doing";
- promotion and retirement become simpler.

### ADD08. Contradiction queue

When two applicable heuristics disagree, create a first-class work item instead
of treating disagreement as a vague runtime mood.

A contradiction queue should track:
- conflicting heuristics or claims;
- affected domain or task;
- severity;
- owner;
- resolution state.

Why add this:
- contradiction becomes actionable;
- it gives learning a concrete backlog;
- it is a smaller, better mechanism than broad worldview algebra.

### ADD09. Citation gate for research-derived knowledge

Before external research becomes a runtime claim or heuristic, require:
- resolvable source;
- quote or evidence extraction;
- citation validity check;
- provenance record.

Why add this:
- it blocks folklore from entering the system as truth;
- it makes research-to-runtime legible;
- it is much cheaper than a full replication ledger first.

### ADD10. Intent fingerprint for risky actions

For risky domains, represent intended action in a typed form and require
proposed execution to match it before approval or dispatch.

Applies especially to:
- blockchain transactions;
- production ops changes;
- destructive file or deployment actions.

Why add this:
- it is a concrete safety mechanism;
- it is more buildable than abstract custody rhetoric alone;
- it creates a clean preflight gate.

### ADD11. Capability-gated plugin lifecycle

Do not make plugin discovery equivalent to plugin activation.

Recommended lifecycle:
1. install
2. inspect
3. audit
4. enable
5. observe
6. disable or remove

Why add this:
- local-first extensibility gets a safety story without registry theater;
- permissions become explicit;
- operators can reason about extension state.

### ADD12. State movement verbs

`export/import` is too coarse. Add explicit operations for runtime state:
- backup;
- restore;
- clone;
- migrate;
- split;
- merge.

Why add this:
- deployment docs become more operationally useful;
- state movement can be dry-run and validated;
- multi-instance and team workflows get a cleaner story.

### ADD13. Maturity bands for every major concept

Mark major concepts as one of:
- `current`
- `target`
- `experimental`
- `research`

Why add this:
- the docs stop flattening all ideas into one confidence level;
- readers can separate redesign core from speculative extensions;
- roadmap arguments become easier to police.

## Best new near-term sequence

If these additions are adopted, the best early order is:

1. three-lane rule;
2. cursor and subscription contract;
3. projection versioning;
4. session as shared work object;
5. policy/calibrator split;
6. heuristic spec/calibration split;
7. contradiction queue;
8. capability-gated plugin lifecycle;
9. citation gate and intent fingerprint;
10. state movement verbs and maturity bands.

## Short conclusion

The original refinement set improved ambition. These additions improve
buildability. They make the target-state less mystical and more like a system
with clear contracts, public objects, and reversible operations.

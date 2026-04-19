# Profile Catalog, Custom Domains, And Scaling

## Scope

Use this file for the full DomainProfile catalog, custom domain creation, work-market surfaces, generalized benchmark indices, and scaling flywheel work.

## Implementation checklist

- [ ] Flesh out the predefined profile catalog.
  - coding;
  - blockchain;
  - research;
  - security;
  - docs/writing.
- [ ] For each profile, specify concrete runtime controls.
  - heartbeat timing;
  - extensions;
  - event subscriptions;
  - context weights;
  - gate stack;
  - infrastructure requirements.
- [ ] Make blockchain/research/coding lifecycle differences explicit.
  - what the agent observes;
  - what it remembers;
  - what actions it can take;
  - what learning loops are unique to the domain.
- [ ] Cover the wider arena catalog even if some are deferred.
  - immediate deployable arenas;
  - near-term arenas;
  - medium-term arenas;
  - long-term arenas.
- [ ] Add custom-domain creation tasks.
  - declarative `roko.toml` path;
  - programmatic Rust-extension path;
  - custom arena path;
  - validation and packaging expectations.
- [ ] Define generalized benchmark-index work beyond ISFR.
  - Agent Performance Index;
  - Knowledge Quality Index;
  - Security Vulnerability Index;
  - Research Impact Index;
  - criteria for when a benchmark becomes first-class.
- [ ] Add scaling and flywheel tasks.
  - more arenas -> more signal;
  - more agents per arena -> stronger learning curve;
  - concurrent arena execution;
  - cross-arena transfer and leaderboard infrastructure.

## Additional gap-closure tasks

- [ ] Add a task for profile-diff documentation generation.
  - machine-readable profile manifest diff;
  - operator-facing summary of what a profile changes;
  - test snapshot per built-in profile.
- [ ] Add a task for unsafe profile-composition detection.
  - conflicting tool permissions;
  - incompatible heartbeat cadences;
  - contradictory gate requirements;
  - deterministic conflict resolution output.
- [ ] Add a task for arena result lineage.
  - task set version;
  - model/profile version;
  - extension set version;
  - scoring formula version.
- [ ] Add a task for work-market minimum viable simulation.
  - local mock or mirage-backed job lifecycle;
  - domain-specific worker qualification logic;
  - benchmark-driven worker selection.
- [ ] Add a task for custom-domain packaging.
  - how a custom profile plus skills/prompts/extensions ships as a reusable bundle.

## Agent-ready task sequence

1. `DA-GAP-01` Profile diff generator
   - Scope: generate machine-readable and human-readable diffs for built-in profiles.
   - Touches: profile manifests, docs generation, tests/snapshots.
   - Deliverable: one profile-diff artifact per built-in profile.
   - Done when: operators can compare coding vs blockchain vs research without manual inspection.

2. `DA-GAP-02` Unsafe profile-composition detector
   - Scope: detect conflicting tools, gates, timing, or infra requirements during profile composition.
   - Touches: composed-profile resolver, config validation.
   - Deliverable: deterministic error/warning output for unsafe combinations.
   - Depends on: `DA-GAP-01`.
   - Done when: a deliberately conflicting profile pair fails validation with an actionable explanation.

3. `DA-GAP-03` Arena result lineage schema
   - Scope: attach task-set version, model/profile version, extension set version, and scoring-version metadata to results.
   - Touches: arena result types, persistence, reporting.
   - Deliverable: versioned arena-result lineage record.
   - Depends on: `DA-GAP-01`.
   - Done when: two arena runs can be compared with full provenance.

4. `DA-GAP-04` Work-market simulation slice
   - Scope: build a local or mirage-backed simulation of domain-qualified worker selection and task lifecycle.
   - Touches: arena/work-market scaffolding, mock marketplace state.
   - Deliverable: minimal simulated work-market loop that exercises domain qualification logic.
   - Depends on: `DA-GAP-03`.
   - Done when: one simulated job is posted, matched, run, and scored.

5. `DA-GAP-05` Custom-domain package bundle
   - Scope: define how a profile plus bundled prompts/skills/extensions ships as a reusable unit.
   - Touches: package manifest design, custom-domain docs, install path.
   - Deliverable: one bundle format and example custom-domain package.
   - Depends on: `DA-GAP-02`.
   - Done when: a packaged custom domain can be installed and validated in a fixture workspace.

## Verification checklist

- [ ] Each built-in profile has a documented runtime diff from the others.
- [ ] Custom-domain validation can fail with actionable errors.
- [ ] Deferred arenas still have enough metadata to enter the roadmap cleanly.
- [ ] Scaling claims are tied to measurable counters or benchmarks.

## Acceptance criteria

- The full PRD-06 domain catalog is represented as tasks or staged backlog, not just the first two arenas.
- Custom-domain creation is a real product surface, not an undefined future feature.
- Benchmark-index generalization and scaling work are concretely scoped.

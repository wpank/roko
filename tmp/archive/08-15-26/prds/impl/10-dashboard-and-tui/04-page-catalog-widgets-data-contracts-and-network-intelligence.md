# Page Catalog, Widgets, Data Contracts, And Network Intelligence

## Scope

Use this file for the parts of PRD-10 that need exhaustive surface coverage: page catalog completion, shared widgets, route/data-contract work, network-intelligence displays, and jobs-system integration.

## Implementation checklist

- [ ] Turn every page group in the PRD into explicit implementation tasks.
  - landing and onboarding;
  - command/chat and research;
  - observatory pages;
  - network pages;
  - marketplace pages;
  - agent-studio pages;
  - atelier pages;
  - settings pages.
- [ ] For each page, specify:
  - dashboard route;
  - TUI mapping or parity note;
  - primary data sources;
  - required components;
  - interactions;
  - loading/empty/error states.
- [ ] Turn the widget catalog into a shared component backlog.
  - status badge;
  - progress bar;
  - sparkline;
  - regime badge;
  - cognitive tier indicator;
  - context gauge;
  - cost display;
  - gate result row;
  - token counter;
  - freshness bar;
  - DAG view;
  - markdown renderer;
  - error digest.
- [ ] Turn data contracts into concrete backend/frontend tasks.
  - existing routes that can be consumed as-is;
  - new routes needed;
  - websocket event schema updates;
  - payload versioning and TS/Rust type alignment.
- [ ] Add network-intelligence display tasks.
  - ISFR visualization;
  - C-Factor display;
  - knowledge density;
  - network-size and domain breakdown;
  - swarm/global aggregate views.
- [ ] Add jobs-system integration tasks beyond generic job CRUD.
  - contract-address/config handling;
  - job lifecycle states;
  - worker tier mapping;
  - validator committee state;
  - on-chain vs local/demo fallback behavior.
- [ ] Add explicit stabilization tasks from PRD-10 section 14.
  - auth middleware upgrade;
  - in-memory state persistence;
  - polling-to-streaming migration;
  - aggregator cache invalidation;
  - error handling gap closure.

## Additional gap-closure tasks

- [ ] Add a task for page-by-page parity tracking.
  - explicit parity matrix across dashboard route, TUI tab/subview, CLI fallback, and backend data source.
- [ ] Add a task for widget state semantics.
  - stale vs loading vs degraded vs error visual language;
  - reduced-motion behavior;
  - accessibility text equivalents.
- [ ] Add a task for network-intelligence drilldowns.
  - C-Factor provenance;
  - knowledge-density methodology;
  - ISFR source/freshness drilldown;
  - agent/domain contribution views.
- [ ] Add a task for jobs-system multi-source truth handling.
  - local file-backed jobs vs chain-backed jobs;
  - demo fallback rules;
  - clear UI marking when state is simulated or mirrored.
- [ ] Add a task for StateHub/projection contract hardening.
  - projection naming/versioning;
  - projection cache invalidation rules;
  - recovery behavior after server restart.

## Agent-ready task sequence

1. `SURF-GAP-01` Cross-surface parity matrix
   - Scope: produce a page-by-page matrix across dashboard, TUI, CLI fallback, and backend source.
   - Touches: surface docs, route inventory, TUI tab/subview mapping.
   - Deliverable: one parity matrix artifact used to assign implementation work.
   - Done when: every major page group has a parity status and owner path.

2. `SURF-GAP-02` Widget state semantics contract
   - Scope: define stale/loading/degraded/error semantics for shared widgets, including accessibility and reduced-motion behavior.
   - Touches: dashboard design system, TUI widget docs, component props/contracts.
   - Deliverable: one shared state-semantics contract for widgets.
   - Depends on: `SURF-GAP-01`.
   - Done when: the same backend state maps to consistent widget behavior on both surfaces.

3. `SURF-GAP-03` Network-intelligence drilldown spec
   - Scope: define drilldowns for C-Factor, knowledge density, ISFR freshness, and contribution provenance.
   - Touches: page specs, backend route requirements, widget composition.
   - Deliverable: one drilldown spec with required data contracts.
   - Depends on: `SURF-GAP-01`.
   - Done when: each top-level network metric has a traceable drilldown path.

4. `SURF-GAP-04` Multi-source jobs truth model
   - Scope: define how local file-backed, chain-backed, and demo fallback job states coexist and are labeled.
   - Touches: jobs routes/types, dashboard job pages, TUI marketplace views.
   - Deliverable: one source-of-truth model for jobs across dev/demo/live modes.
   - Depends on: `SURF-GAP-01`.
   - Done when: UI can always label whether job state is local, mirrored, or simulated.

5. `SURF-GAP-05` Projection/StateHub contract hardening
   - Scope: version projection names, invalidation rules, and restart recovery behavior.
   - Touches: `crates/roko-serve/src/routes/projections.rs`, state cache/projection docs, consumers.
   - Deliverable: one hardened projection contract with cache invalidation policy.
   - Depends on: `SURF-GAP-01`.
   - Done when: a server restart does not leave projection consumers in undefined states.

## Relevant current files

- `crates/roko-serve/src/routes/`
- `crates/roko-serve/src/events.rs`
- `crates/roko-cli/src/tui/views/`
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/`
- `/Users/will/dev/nunchi/nunchi-dashboard/src/components/`
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/endpoint-spec.md`

## Verification checklist

- [ ] Every page group in PRD-10 maps to at least one concrete task file or section.
- [ ] Widget backlog distinguishes reusable primitives from page-specific composites.
- [ ] Backend route/event gaps are enumerated rather than hidden.
- [ ] Network-intelligence and jobs-system features have real data-contract work attached.

## Acceptance criteria

- PRD-10’s surface inventory is fully represented as implementation work.
- Shared widgets and data contracts are scoped separately from page polish.
- The dashboard/TUI plan now covers page semantics, data wiring, and backend gaps comprehensively.

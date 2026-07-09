# Source Index — Verification Anchors

Quick reference for the post-audit architecture-doc refresh.

Use these anchors to spot-check whether a parity sentence is grounded, stale, or clearly
future-state. They are verification aids. They are **not** proof that a full design is
implemented end to end.

---

## How To Use This File

1. Open the architecture-doc anchor that contains the claim or proposal.
2. Compare it against a code spot-check anchor or audit anchor.
3. Rewrite the parity sentence as `shipped`, `partial`, `planned`, or `deferred`.

If you cannot find a code surface for a concept, the correct outcome is usually weaker wording,
not a stronger anchor table.

## Architecture Doc Anchors (Docs 23-35)

| Doc | Anchors To Open | Use To Verify |
|-----|-----------------|---------------|
| `23-architectural-analysis-improvements.md` | `27` (executive summary), `172` (roko-conductor violation), `521` (novel proposals), `694` (prioritized improvements) | This doc mixes present-state analysis with future enhancements; parity wording should split them |
| `24-cross-section-integration-map.md` | `157` (currently wired flows), `401` (missing integration points), `624` (proposed bus as kernel primitive), `972` (integration priority roadmap) | Live flows exist, but large sections still describe target-state integrations and sequencing |
| `25-attention-as-currency.md` | `60` (attention token model), `199` (VCG auction), `352` (CascadeRouter as spender), `605` (target-state integration) | The document is a design proposal; do not let its internal wiring sections read like shipped runtime behavior |
| `26-cognitive-immune-system.md` | `50` (scope boundary), `148` (taint propagation), `293` (custody-linked incident-response proposal), `425` (target-state integration) | The live safety spine is narrower than the full CIS stack; separate existing taint/attestation from target-state layers |
| `27-temporal-knowledge-topology.md` | `47` (Allen's interval algebra), `213` (event calculus), `335` (temporal knowledge graph), `652` (target-state integration) | Temporal topology remains architecture design material, not a shipped subsystem |
| `28-emergent-goal-structures.md` | `43` (sources of goal emergence), `172` (goal emergence engine), `375` (goal competition), `586` (target-state integration) | Goal emergence is still proposal-heavy and should remain future-state in parity files |
| `29-cognitive-energy-model.md` | `44` (energy pool), `106` (depletion functions), `321` (energy-affect coupling), `638` (target-state integration) | The energy model is a design reference, not implementation evidence |
| `30-cross-pollination-innovations.md` | `25` (REF19 alignment), `56` (innovation 1), `2815` (cross-innovation interactions), `2881` (implementation priority) | This file is research synthesis and prioritization, not proof that the proposed integrations exist |
| `31-implementation-readiness-audit.md` | `61` (section scorecard), `536` (crate implementation status), `567` (prioritized gap list), `716` (recommended execution order) | Useful as audit input and planning context, but not as direct proof of feature completeness |
| `32-comprehensive-test-strategy.md` | `45` (current state), `334` (property-based strategy), `600` (benchmarks), `1242` (test execution model), `1410` (test count roadmap) | Separate current test baseline from target test scale and future test programs |
| `33-refactor-plan-phases.md` | `26` (Phase A), `55` (Phase B), `83` (Phase C), `113` (Phase D) | This is sequencing guidance for possible refactors, not a statement of current architecture |
| `34-synergy-integration-map.md` | `23` (ten load-bearing primitives), `44` (synergy matrix), `155` (what the matrix is, and is not), `180` (moat restated) | The matrix is an aspirational design aid; parity notes should not treat it as live moat evidence |
| `35-consolidated-roadmap.md` | `38` (sequencing principles), `57` (critical path), `94` (quarter-by-quarter roadmap), `182` (parallel tracks and team shape), `253` (not-doing list) | Keep dependency ordering; treat quarter plans and staffing shape as planning-only material |

## Code Spot-Check Anchors

These anchors show where to inspect current code when a parity sentence needs grounding. One code
anchor can confirm that a surface exists or that a term is live. It cannot, by itself, prove that
an entire architecture section is fully implemented.

| Claim Area | Anchor | Use To Verify |
|------------|--------|---------------|
| Engram is the live durable kernel noun | `crates/roko-core/src/engram.rs:39` | The durable kernel surface is `Engram` |
| Six core traits are live | `crates/roko-core/src/traits.rs:34`, `:78`, `:102`, `:124`, `:143`, `:166` | Keep the six-trait story grounded; do not promote `Datum` to a live trait surface |
| `loop_tick()` is shared, but not universal | `crates/roko-core/src/loop_tick.rs:77` | Wording should stay partial rather than claiming a single universal loop owner |
| Three operating frequencies are live | `crates/roko-core/src/operating_frequency.rs:16` | Gamma / Theta / Delta are safe current-state language |
| Runtime event reality | `crates/roko-runtime/src/event_bus.rs:103` | The live `RokoEvent` enum has exactly two variants |
| Generic event bus exists at runtime level | `crates/roko-runtime/src/event_bus.rs:167` | A generic `EventBus<E>` exists, but that is not the same thing as a shipped kernel `Bus<E>` trait |
| TUI surface is real | `crates/roko-cli/src/tui/` | The CLI includes a real TUI surface; do not call it a placeholder |
| Serve surface is real | `crates/roko-serve/src/routes/` | `roko-serve` is wired and exposes a substantial route set |
| Real learning and cognitive crates exist | `crates/roko-learn/src/`, `crates/roko-neuro/src/lib.rs:128`, `crates/roko-daimon/src/lib.rs:1462`, `crates/roko-dreams/src/cycle.rs:333` | These crates justify present-tense wording about existing subsystem surfaces, but not speculative meta-features |
| Compare against stale architecture wording | `docs/00-architecture/INDEX.md:189` | Useful for detecting older status language that the parity pack should now correct |

Do not add code anchors for `Pulse`, `Datum`, `Demurrage`, `Worldview`, or `Custody` until those
concepts have real production code.

If an anchor points into a planning artifact, use it to weaken wording rather than to argue the
planned system into existence.

## Audit Anchors

| Audit File | Use To Verify |
|------------|---------------|
| `tmp/refinements-audit/00-MASTER-SUMMARY.md` | master verdicts, priority ordering, and corrected facts for the pack |
| `tmp/refinements-audit/01-foundation-audit.md` | REF01-09 narrowing guidance |
| `tmp/refinements-audit/05-integrator-audit.md` | why the synergy matrix and long roadmap need planning-language posture |
| `tmp/refinements-audit/06-codebase-reality-check.md` | counts, rename status, event-bus reality, and interface wiring facts |
| `tmp/refinements-audit/07-doc-quality-audit.md` | stale wording patterns and status drift that should be fixed in parity files |

When architecture docs, code, and older parity wording disagree, the correct order of trust is:

1. current code for existence claims,
2. audit files for corrected factual posture,
3. architecture docs for the source claim being rewritten,
4. older parity wording last.

# Integrator Arc Audit (Refinements 31-35)

Auditor: Claude Opus 4.6 | Date: 2026-04-17 | Scope: Docs 31-35 vs actual codebase

This audit covers the five "integrator" documents that attempt to stitch the
previous 30 refinement docs into a coherent whole. The central question: do
these integrators actually integrate, or do they add another layer of
abstraction on top of an already-abstract stack?

---

## REF31: Synergy & Integration Map

**Verdict: SKEPTICAL**

### What the doc claims

A 10x10 matrix of ten "load-bearing primitives" (Engram, Pulse, Bus,
Substrate, HDC, Demurrage, Heuristics, c-factor, Replication ledger, Plugin
SPI), with every cell describing what one primitive gives to another. Ten
worked synergy examples. The thesis: Roko's moat is the interaction density
of the full matrix, not any single feature.

### What actually exists in code

Of the ten primitives:

| Primitive | Exists in code? | Status |
|---|---|---|
| P1 Engram | YES | `roko-core/src/engram.rs` -- real, tested, used everywhere |
| P2 Pulse | NO | No `Pulse` struct exists anywhere in the codebase |
| P3 Bus trait | NO | `EventBus<E>` exists in `roko-runtime/src/event_bus.rs` as a concrete struct, not a trait |
| P4 Substrate trait | YES | `roko-core/src/traits.rs` -- real, working |
| P5 HDC fingerprint | PARTIAL | `HdcVector` exists in `roko-primitives/src/hdc.rs`; `text_fingerprint` used by episode logger; but `query_similar` does not exist on Substrate |
| P6 Demurrage | NO | Zero occurrences of "demurrage" or "ReinforceKind" in any crate |
| P7 Heuristics | MINIMAL | `HeuristicRule` in `roko-neuro/src/tier_progression.rs` only; no `Heuristic` engram kind, no Calibrator, no Wilson CI |
| P8 c-factor | PARTIAL | `CFactorPolicy` in `roko-core/src/cfactor.rs`, `CFactor` struct in `roko-learn/src/cfactor.rs` -- exists, wired to Policy trait |
| P9 Replication ledger | NO | No `Claim`, `Paper`, or replication ledger code exists |
| P10 Plugin SPI | NO | No `roko-spi` crate; no plugin manifest schema; no tier system |

**Score: 3 of 10 primitives exist in meaningful form.** Two more exist
partially. Five are entirely aspirational.

### Specific problems

1. **The synergy matrix is aspirational fiction.** A matrix documenting
   interactions between things that do not exist is not a synergy map -- it
   is a wish list. Synergy 3.1 ("Demurrage x HDC -> self-trimming semantic
   memory") cites three primitives that collectively have zero lines of
   production code implementing the described interaction.

2. **The moat claim is circular.** The doc argues the moat is the matrix.
   The matrix mostly describes things that do not exist. Therefore the moat
   does not exist. This is fine for vision documents, but dangerous when
   framed as "the competitive edge we already have."

3. **The honest non-synergies section (section 7) is the best part.** It is
   the only section that demonstrates intellectual honesty about what
   actually connects. More of this, less of the wish-list cells.

4. **Section 8 claims three "emergent properties"** (self-improvement,
   inspectability, substrate neutrality) that the composition has. But only
   one (inspectability, via lineage chains on Engrams) is even partially
   real today. The other two depend on primitives P5-P10 that mostly do
   not exist.

### Practical alternative

Strip the matrix to the 3-4 primitives that actually exist (Engram,
Substrate, EventBus, HdcVector partial). Document only the synergies that
are live today: lineage-based auditability, content-addressed storage +
gate verdicts, HDC fingerprinting in episode logger. Mark everything else
as "planned" with a dependency on actual implementation. This would be
approximately 2 pages instead of 10.

---

## REF32: Safety, Sandbox, and Provenance

**Verdict: SIMPLIFY -- has a real foundation, but proposes 10x more than exists**

### What the doc claims

A comprehensive safety spine: `authorize(principal, action, target, ctx)`
returning `AuthzDecision`, plugin sandboxes across 5 tiers, taint tracking
with an enum (`Taint`), cryptographic attestation, custody records, egress
control, secrets management, multi-tenancy isolation, conflict-of-interest
policies, threat model, audit CLI, and incident response procedures.

### What actually exists in code

The safety layer at `crates/roko-agent/src/safety/` is **real and well-built**:

- `SafetyLayer` struct with `check_pre_execution()` -- works, tested
- `BashPolicy` -- command allowlist/denylist for shell tools
- `GitPolicy` -- branch protection (blocks force push to main)
- `NetworkPolicy` -- URL allowlist for web tools
- `PathPolicy` -- worktree escape prevention
- `ScrubPolicy` -- API key scrubbing from outputs
- `RateLimiter` -- per-tool/per-role rate limits
- `AgentContract` -- declarative constraints with governance rules
- `AgentWarrant` -- OCaps-style capability checking
- `Capability` enum -- Tool, ReadPath, WritePath, Exec, Network

This is a solid, practical safety layer. Tests exist. It works.

### The gap between real code and the proposal

| Feature | In safety/mod.rs | In REF32 |
|---|---|---|
| Tool authorization | YES (check_pre_execution) | Proposed as `authorize()` with different signature |
| Role-based tools | YES (ToolWhitelist per role) | Proposed as Principal/Action/Target/Context model |
| Secret scrubbing | YES (ScrubPolicy) | Proposed as `Secret` wrapper type |
| Rate limiting | YES (RateLimiter) | Not mentioned |
| Contract system | YES (AgentContract, GovernanceRule, Invariant) | Not mentioned at all |
| Capability/warrant | YES (AgentWarrant, Capability enum) | Not mentioned |
| Taint tracking | BOOLEAN ONLY (`Provenance.tainted: bool`) | Proposed as rich `Taint` enum with propagation |
| Attestation | YES (`Attestation` struct in roko-core, Ed25519 sign/verify) | Proposed with `AttestationLevel` enum (not in code) |
| Custody records | NO | Proposed as `Custody` struct |
| Plugin sandboxes | NO (no plugin system) | Proposed as 5-tier sandbox |
| Egress control | PARTIAL (NetworkPolicy blocks URLs) | Proposed as `Egress` trait |
| TypedContext | NO | Proposed as safety context carrier |
| Multi-tenancy | NO | Proposed |
| Conflict-of-interest | NO | Proposed |
| Audit CLI | NO | Proposed 7 commands |

### Specific problems

1. **The doc does not acknowledge the contract system.** The existing
   `AgentContract` with `Invariant` and `GovernanceRule` is a real, working
   authorization framework. REF32 proposes replacing it with a different
   model (`authorize()` function) without acknowledging what exists. This
   violates rule 1 from CLAUDE.md: "NEVER reimplement what already exists."

2. **The doc proposes custody, taint enum, TypedContext, plugin sandboxes,
   egress trait, multi-tenancy, and audit CLI** -- none of which exist.
   That is approximately 8-10 weeks of engineering for one developer. The
   staging section claims "two months of focused safety work" but doesn't
   account for the fact that this is one developer + AI agents.

3. **The permission table (section 3)** is well-designed and would be
   useful, but it implies a role system with researcher/planner/implementer/
   reviewer/ops roles that is more granular than what the existing contract
   system provides.

4. **The attestation model already partially exists.** `roko-core/src/attestation.rs`
   has `Attestation`, `Ed25519Signature`, `PublicKey`, `ChainAttestation`,
   plus working `sign()` and `verify()` functions. REF32 proposes adding
   `AttestationLevel` (LocalAgent/OrgRole/ChainWitness), which is a
   reasonable extension but should be framed as extending the existing code,
   not as a new design.

5. **The threat model (section 13) is genuinely useful.** It is the right
   shape, names the right things, and would be worth writing to
   `docs/security/threat-model.md` as-is.

### Practical alternative

Start from the existing `SafetyLayer` and extend it:

- Phase 1: Add `AttestationLevel` to existing `Attestation` struct. Expand
  `Provenance.tainted` from bool to a `Taint` enum. Write the threat model
  doc. (1 week)
- Phase 2: Add `Custody` engram kind and logging for destructive actions.
  (1 week)
- Phase 3: Everything else is deferred until a plugin system exists.

---

## REF33: Observability & Telemetry

**Verdict: DEFER -- useful spec, almost no existing infrastructure for it**

### What the doc claims

Four telemetry surfaces (structured logs, Prometheus metrics, OpenTelemetry
traces, Bus events), 60+ named metrics across 6 categories, default Grafana
dashboards, alert rules, cost dashboards, replay-with-override CLI, and
self-observability.

### What actually exists in code

- `roko-runtime/src/metrics.rs` -- an append-only JSONL metric recorder
  using `serde::Serialize`. This is NOT Prometheus exposition format. It is
  a simple file-based metric writer with `MetricRecorder::record()`.
- `tracing` crate is used for structured logging throughout (standard Rust
  ecosystem).
- `roko-learn/src/episode_logger.rs` -- append-only JSONL episode log.
  This IS working observability, but it is file-based, not
  Prometheus/OpenTelemetry.
- `roko-learn/src/efficiency.rs` -- per-turn efficiency events. Working.
- `roko-core/src/state_hub.rs` -- `StateHub` exists as a broadcast channel
  for dashboard events. Working but simple.
- OpenTelemetry: ONE grep match in the entire codebase
  (`roko-orchestrator/src/worktree.rs`). No OTLP exporter, no span creation,
  no trace context propagation.
- Prometheus: NO `/metrics` endpoint exists. No Prometheus client library
  in dependencies.

### Specific problems

1. **No Prometheus infrastructure exists.** The doc assumes a `/metrics`
   endpoint. There is no Prometheus client crate in the workspace. The
   existing metrics are JSONL files, not exposition format.

2. **No OpenTelemetry infrastructure exists.** The doc proposes detailed
   span trees (`op.sense`, `op.assess`, `op.compose`, etc.) but there is
   no OpenTelemetry SDK integration. Adding it to an 18-crate workspace is
   not trivial.

3. **The 60+ proposed metrics reference primitives that do not exist.** The
   "Roko-specific metrics" sections reference c-factor (partially exists),
   demurrage (does not exist), HDC diversity (does not exist), replication
   ledger (does not exist), Bus ring occupancy (EventBus has no ring buffer
   concept). Of the 6 metric categories, only "Gate pipeline" (section 5.4)
   maps cleanly to existing code.

4. **The cost metrics (section 5.6) are buildable** -- the cascade router
   tracks model selection, and efficiency events track per-turn tokens/cost.
   This is the most realistic section.

5. **The staging section estimates "two months of focused observability work"**
   which is honest but does not account for the dependency on primitives
   (Bus trait, Pulse, demurrage, etc.) that also do not exist yet.

6. **What actually exists is better than nothing.** The JSONL episode log,
   efficiency events, and StateHub broadcast are a pragmatic observability
   surface. The doc should acknowledge this as the Phase 0 baseline and
   build from there, rather than presenting a greenfield Prometheus+OTLP
   architecture.

### Practical alternative

Phase 0 (now): Document the existing observability (JSONL episode log,
efficiency events, StateHub, `tracing`-based structured logs) as the real
baseline.

Phase 1 (when needed): Add `metrics` crate for Prometheus exposition to
`roko-serve`. Wire the 10-15 metrics that correspond to actual subsystems
(gate verdicts, model routing decisions, token costs, agent turn durations).

Phase 2 (much later): OpenTelemetry spans, Grafana dashboards, alert rules.
These are useful but not urgent for a single-developer project.

---

## REF34: Glossary

**Verdict: SIMPLIFY -- partially accurate, but defines many terms for things that do not exist**

### What the doc claims

A canonical A-Z glossary of every term introduced or reclaimed across the
33 earlier refinement docs.

### Cross-reference against actual code names

I checked every bolded term against the codebase:

| Term | In glossary | In code | Match? |
|---|---|---|---|
| Agent | Yes | `roko-agent/` | YES |
| Attestation | Yes | `roko-core/src/attestation.rs` | YES |
| Balance (demurrage) | Yes | NO | NO -- does not exist |
| Bus (trait) | Yes | NO (EventBus<E> struct) | MISMATCH -- glossary says "proposed to become a trait" (honest) |
| c-factor | Yes | `roko-core/src/cfactor.rs`, `roko-learn/src/cfactor.rs` | YES |
| CascadeRouter | Yes | `roko-learn/src/cascade_router.rs` | YES |
| Claim | Yes | NO | NO -- does not exist |
| Cohort | Yes | NO | NO -- no struct |
| Composer | Yes | `roko-core/src/traits.rs` | YES |
| ContentHash | Yes | `roko-core/src/hash.rs` | YES |
| Context | Yes | `roko-core/src/context.rs` | YES |
| Custody | Yes | NO | NO -- does not exist |
| Daimon | Yes | `roko-core/src/affect.rs` (PadVector, EmotionalTag) | PARTIAL |
| Datum | Yes | NO | NO -- does not exist |
| Decay | Yes | `roko-core/src/decay.rs` | YES |
| Demurrage | Yes | NO | NO -- does not exist |
| Engram | Yes | `roko-core/src/engram.rs` | YES |
| Episode | Yes | `roko-learn/src/episode_logger.rs` | YES |
| EventBus (retired) | Yes | `roko-runtime/src/event_bus.rs` | NOTE: still the live code, not "retired" |
| Falsifier | Yes | NO | NO -- does not exist |
| Fingerprint (HDC) | Yes | `roko-primitives/src/hdc.rs` | PARTIAL (HdcVector exists; not "on every Engram at put time") |
| Fleet | Yes | NO | NO -- no Fleet struct |
| Gate | Yes | `roko-core/src/traits.rs` | YES |
| GateVerdict | Yes | `roko-core/src/kind.rs` | YES |
| Golem (retired) | Yes | NO | CORRECT -- retired |
| Graduation | Yes | NO | NO -- no graduation code |
| Grimoire (retired) | Yes | NO | CORRECT -- renamed to Neuro |
| Heuristic | Yes | `roko-neuro/src/tier_progression.rs` (HeuristicRule) | PARTIAL |
| HdcVector | Yes | `roko-primitives/src/hdc.rs` | YES |
| Kind | Yes | `roko-core/src/kind.rs` | YES |
| Lineage | Yes | `Engram.lineage: Vec<ContentHash>` | YES |
| loop_tick | Yes | `roko-core/src/loop_tick.rs` | YES |
| MCP | Yes | `roko-mcp-code/`, etc. | YES |
| Neuro | Yes | `crates/roko-neuro/` | YES |
| Operator (6 traits) | Yes | `roko-core/src/traits.rs` (6 traits) | YES |
| Paper | Yes | NO | NO -- does not exist |
| Plan | Yes | `Kind::Plan` in kind.rs | YES |
| Playbook | Yes | `Kind::PlaybookRule`, `roko-learn/src/playbook.rs` | YES |
| Policy | Yes | `roko-core/src/traits.rs` | YES |
| Provenance | Yes | `roko-core/src/provenance.rs` | YES |
| Projection (StateHub) | Yes | NO (StateHub exists but no typed Projection) | PARTIAL |
| Pulse | Yes | NO | NO -- does not exist |
| query_similar | Yes | NO | NO -- not on Substrate trait |
| Router | Yes | `roko-core/src/traits.rs` | YES |
| Score | Yes | `roko-core/src/score.rs` | YES |
| Scorer | Yes | `roko-core/src/traits.rs` | YES |
| Signal (retired) | Yes | Still used in trait doc comments ("Store a signal") | NOTE: partially renamed |
| StateHub | Yes | `roko-core/src/state_hub.rs` | YES |
| Substrate | Yes | `roko-core/src/traits.rs` | YES |
| Taint | Yes | `Provenance.tainted: bool` | PARTIAL -- bool, not the proposed enum |
| TypedContext | Yes | NO | NO -- does not exist |
| Worldview | Yes | NO | NO -- does not exist |

### Summary count

- 24 terms match actual code
- 7 terms partially match (exist in simpler form)
- 15 terms describe things that do not exist

### Specific problems

1. **The glossary labels `EventBus<E>` as "historical, being retired for
   `Bus` trait with `Pulse` payload"** -- but `EventBus<E>` is the live
   production code and no Bus trait or Pulse type exists. Marking production
   code as "retired" in a glossary is misleading.

2. **The glossary calls `Signal` "historical, retired in 877:5 rename"** --
   but the Substrate trait doc comments in `traits.rs` still say "Store a
   signal" and "signal" appears throughout the trait docs. The rename is
   incomplete.

3. **15 glossary entries define terms for things that do not exist in code.**
   This is fine for a vision glossary but dangerous if treated as a
   canonical reference -- newcomers will search for these types and find
   nothing.

### Practical alternative

Split into two sections: "Terms with code" (the 24 matches + 7 partials)
and "Planned terms" (the 15 that do not exist). This prevents confusion
when someone searches the codebase for `Pulse` or `Custody` and finds
nothing.

---

## REF35: Consolidated Roadmap

**Verdict: REJECT as timeline, SHIP as priority ordering**

### What the doc claims

A six-to-twelve-month roadmap across four quarters, requiring 5-7
engineers. Q1: two-medium kernel. Q2: learning substrate. Q3: ecosystem and
UX. Q4: scale, safety, domains. Q5-Q6: phase 2 (chain, mesh, dreams).

### The single-developer reality

The doc says "minimum team to land Q1-Q4 in 12 months" is 5-7 engineers.
The actual team is one developer (Will) and AI agents. This is not a minor
discrepancy -- it is a 5-7x staffing mismatch.

### Specific problems

1. **Q1 proposes a kernel refactor (Pulse, Bus trait, Datum, operator
   generalization, seven-step loop)** -- this is rewriting `roko-core` and
   `roko-runtime`. For one developer, this is not a quarter; it is likely
   3-6 months of careful, test-covered migration with high risk of breaking
   the existing working system.

2. **Q2 proposes HDC on every Engram, demurrage, heuristics as a type,
   c-factor measurement, and research-to-runtime.** These are five
   substantial features, each requiring new types, new storage, new CLI
   commands, and integration tests. For one developer, this is another
   3-6 months.

3. **Q3 proposes Plugin SPI, StateHub rearchitecture, realtime wire
   protocol, developer UX, user UX, CLI parity, web UI, rich UX primitives,
   and deployment UX.** This is nine workstreams. The doc estimates this as
   one quarter with 2 UX engineers + 1 platform engineer. For one developer,
   this is easily a year.

4. **Q4 proposes six domain profiles, safety spine, replication ledger,
   multi-tenancy, c-factor actuation, scaling instrumentation, and
   commons.** Another year for one developer.

5. **Total realistic timeline for one developer**: Q1-Q4 as described would
   take 3-5 years, not 12 months. The doc is calibrated for a startup
   team, not for the actual situation.

6. **The dependency graph (section 2) is useful** regardless of timeline.
   The ordering is correct: 01-09 must precede 10-16 must precede 17-25
   must precede 26-30.

7. **The not-doing list (section 9) is excellent.** It is the most honest
   and useful section of the document.

8. **The twelve-year view (section 11) is aspirational vaporware.** "The
   substrate architecture is studied in graduate compilers courses" is a
   prediction, not a plan.

### Practical alternative

Keep the dependency ordering and not-doing list. Discard the quarterly
timeline. Replace with a priority queue sized for one developer:

1. Finish self-hosting loop (items 10-11 from CLAUDE.md: auto plan
   generation + feedback loop). These are the highest ROI items and do NOT
   require any kernel refactor.
2. Tighten existing safety layer (extend Attestation, expand taint,
   threat model doc).
3. Wire existing observability (document the JSONL baseline, add a few
   Prometheus counters to roko-serve).
4. Address the ux-followup P0 items.
5. Everything else from Q1-Q4 goes into a "when the system needs it" pile.

---

## Meta-verdict: Do these integrator docs actually integrate?

**No. They add a layer of abstraction over an already-abstract refinement
stack.**

Here is the structural problem: the 30 prior refinement docs propose
changes to the codebase. Some of those proposals are grounded in real code
(the safety layer, the episode logger, the cascade router). Many are
aspirational (Pulse, Bus trait, demurrage, heuristics-as-type, replication
ledger, plugin SPI).

The integrator arc (31-35) attempts to weave the aspirational proposals
together into a coherent story. But the weave is between proposed features,
not between existing code. This produces a coherent vision document that
has very little grip on the actual system.

### What each integrator doc actually provides

| Doc | Claimed function | Actual function |
|---|---|---|
| 31 Synergy map | Shows how 10 primitives reinforce each other | Aspirational interaction matrix; 7 of 10 primitives do not exist |
| 32 Safety spine | Consolidates safety across all layers | Good extension of real safety code; buries the existing contract system under a new model |
| 33 Observability | Single instrumentation spec | Greenfield Prometheus/OTLP architecture that ignores existing JSONL-based observability |
| 34 Glossary | Canonical vocabulary | 60% accurate to code; 40% defines nonexistent types |
| 35 Roadmap | Sequencing for 5-7 engineers over 12 months | Correct ordering; 5-7x overstaffed timeline for actual team size |

### The integration problem

True integration would mean: "Here is what exists. Here is how these
existing pieces connect. Here is the minimal set of new code needed to
strengthen those connections." Instead, the integrator arc says: "Here is
what we plan to build. Here is how all of those plans connect to each
other."

The difference matters because:

1. Plans connecting to plans produces a dependency graph of unbuilt
   features. This is useful for architecture docs but dangerous for
   execution -- it can make the next step look like it requires 10
   prerequisites.

2. The existing system already works end-to-end (plan-execute-gate-persist).
   The refinements and their integrators risk making the system feel
   incomplete when it is actually functional.

3. A single developer's scarce attention should go to the gaps between
   existing, working code -- not to elaborate architectures for features
   that may never be built.

### Recommendation

**Ship:** REF32 section 13 (threat model), REF34 (glossary, with the
"exists in code / planned" split), REF35 section 9 (not-doing list),
REF35 section 2 (dependency graph as a priority ordering tool).

**Simplify aggressively:** REF32 (start from existing SafetyLayer, not a
new model), REF33 (document existing JSONL observability as baseline).

**Defer:** REF31 (synergy matrix is not useful until the primitives exist),
REF35 quarterly timeline (not calibrated for one developer).

**The core advice:** stop integrating plans with plans. Start integrating
code with code. The system works today. Make it work better before making
it work differently.

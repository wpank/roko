# Decay as a Topological Operator

**Kind**: Perspective
**Source**: `docs/00-architecture/27-temporal-knowledge-topology.md`

---

## Decay as a Mathematical Operator

In topology, an **operator** on a space transforms it into another space. Decay can be
formalized as a family of operators \( D_t \) indexed by time, where \( D_t \) maps the
knowledge topology at time 0 to the knowledge topology at time \( t \):

\[
\mathcal{K}(t) = D_t(\mathcal{K}(0))
\]

Understanding \( D_t \) mathematically enables:
- Prediction of the topology at future times given the current topology.
- Identification of which features of the topology are preserved by decay.
- Design of decay parameters that achieve desired topological outcomes.

---

## Properties of Decay Operators

### Monotonicity

A decay operator should be **monotone**: it can only remove or weaken connections, not
create new ones. If Engram A is connected to Engram B in \( \mathcal{K}(0) \), the
connection weight can only decrease over time (under decay alone). New connections are
created by ingestion and consolidation, not by decay.

Formally: \( w_{AB}(t) \leq w_{AB}(0) \) for all \( t > 0 \), where \( w_{AB} \) is the
effective connection weight between nodes A and B.

### Commutativity (Independence Assumption)

If decay operators for different Engrams are independent, they commute: the order in which
individual Engrams decay does not affect the final topology. This is an approximation
(in reality, the decay of one Engram can affect the context that determines how another
decays) but a useful one.

### Topological Persistence

**Persistent homology** (Edelsbrunner & Harer, 2010) is a mathematical tool for studying
how topological features (connected components, holes, voids) appear and disappear as a
filtration parameter (in our case, time/decay) changes.

Applied to knowledge decay: as decay progresses, connected components merge (due to some
connections dropping below a detection threshold) and holes appear (as formerly connected
regions disconnect). Persistent homology tracks the **lifetime** of each topological feature:
features that persist across a wide range of decay parameters are structurally important.
Features that appear and disappear quickly are fragile.

This provides a formal tool for identifying **topologically important Engrams**: Engrams
whose decay would produce long-lived topological changes (disconnections, new holes) are
more important to preserve than Engrams whose loss would be quickly compensated.

---

## Decay Models as Operator Families

### Exponential Decay: Smooth Erosion

Exponential decay: \( w(t) = w(0) \cdot e^{-\lambda t} \)

Topologically, exponential decay is **smooth**: it changes connection weights continuously
without sharp transitions. Under exponential decay:
- The topology evolves continuously over time.
- No phase transitions: the qualitative structure changes only gradually.
- Hub Engrams lose weight steadily but remain hubs for longer (they started higher).

**Topological character**: smooth, reversible (up to a threshold), uniform erosion.
Appropriate when the knowledge environment changes gradually.

### Step Decay: Phase Transition

Step decay: \( w(t) = w(0) \) for \( t < t_0 \), \( w(t) = 0 \) for \( t \geq t_0 \)

Topologically, step decay produces a **phase transition** at \( t_0 \): the Engram is
present and fully weighted until expiry, then instantly removed. This produces:
- A sharp discontinuity in the topology at the expiry time.
- Potential disconnection of components if the expiring Engram was a bridge.
- No graceful degradation — the agent has full knowledge until expiry, then none.

**Topological character**: discontinuous, irreversible at expiry. Appropriate for
time-limited information (event schedules, temporary authorizations) where validity has a
hard boundary.

### Linear Decay: Managed Erosion

Linear decay: \( w(t) = w(0) \cdot (1 - t/t_0) \) for \( t < t_0 \)

Topologically, linear decay is a managed erosion: weights decrease at a constant rate,
reaching zero at \( t_0 \). The agent has advance warning of complete loss (weights are
measurably declining before they reach zero).

**Topological character**: uniform, predictable. Appropriate when information should
gradually cede authority as newer information supersedes it.

### Plateau Decay: Durability Then Rapid Loss

Plateau decay: stable weight for an extended period, then rapid decline.

Topologically, plateau decay creates a **two-phase topology**: a stable phase where the
topology is constant (good for long-term reasoning) followed by a rapid transition phase
where the topology changes quickly (poor for reasoning, but brief). The transition should
ideally trigger Dreams consolidation before the rapid decline.

**Topological character**: stable → turbulent transition. Appropriate for knowledge with
a clear validity window (e.g., a policy that is stable while in effect and quickly obsolete
when superseded).

---

## Designing for Topological Stability

Given the decay operators' effects, the design goal is to produce a topology that is
**stable** — that maintains essential structural properties even as individual Engrams decay.

Design principles:
1. **Redundant paths**: ensure multiple topological paths connect important domains, so
   that the decay of any single Engram does not disconnect them.
2. **Decay-aware consolidation**: run Dreams consolidation to create bridges before
   important Engrams approach expiry.
3. **Hub protection**: identify hub Engrams (high-degree nodes) and apply longer-lived
   decay models or consolidation priority to them.
4. **Contradiction isolation**: quarantine contradicted Engrams rather than removing them
   immediately; their removal may disconnect components that depended on them.

---

## Key Reference

- **Edelsbrunner, H., & Harer, J. L. (2010).** *Computational Topology: An Introduction*.
  American Mathematical Society. Persistent homology applied to evolving topological spaces.

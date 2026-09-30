# 31-04 -- Autocatalytic Compounding

> **Parent:** [31-SELF-HOSTING.md](../../31-SELF-HOSTING.md) section 5.2
> **Primary source:** Kauffman 1993, "The Origins of Order: Self-Organization
> and Selection in Evolution"
> **Additional sources:** Argyris & Schon 1978 (triple-loop learning),
> Tria et al. 2014 (Polya urn model), Reed's Law, Metcalfe's Law
> **Implementation:** `crates/roko-learn/src/cfactor.rs`,
> `crates/roko-learn/src/aggregate.rs`
>
> **Status (2026-09-29): a hypothesis, not a result.** In short, no measurement shows
> Roko's learning compounding, and recent studies argue against expecting it (the status
> note in [08-LEARNING](../../08-LEARNING.md) section 10 cites them). The defensible
> claim is bounded, audited improvement with rollback. The metric functions in
> `aggregate.rs` have no production caller at `7c556bc0a`.

---

## 1. Autocatalytic Sets

An autocatalytic set (Kauffman 1993) is a collection of chemical reactions
where every reaction's inputs are produced by some other reaction in the set.
The defining property is **closure under catalysis**: no reaction requires
an input from outside the set. Once an autocatalytic set reaches a critical
diversity threshold, it becomes self-sustaining -- the creation of new
molecules accelerates the creation of further molecules, producing exponential
growth from a simple initial state.

Kauffman showed that autocatalytic sets emerge spontaneously when the
diversity of possible reactions exceeds a phase-transition threshold. Below
the threshold, reactions fizzle out. Above it, self-sustaining networks
crystallize from random chemistry.

---

## 2. Application to Learning Systems

Roko's learning subsystems form a potential autocatalytic set. The "reactions"
are feedback loops; the "molecules" are learning artifacts (episodes,
playbook rules, routing tables, knowledge entries, skills). Each loop consumes
artifacts produced by other loops and produces artifacts consumed downstream.

The autocatalytic thesis states: **when all loops are connected (strongly
connected feedback graph), the compound improvement rate exceeds the sum of
individual loop rates.** When loops are fragmented (disconnected components),
improvement is at most linear.

### 2.1 The Compounding Condition

```
Improvement(t) = base_rate * Product_i(loop_i_output(t))

When all loops are connected:
    d(Improvement)/dt > base_rate  (superlinear growth)

When loops are fragmented:
    d(Improvement)/dt <= base_rate  (at most linear growth)
```

The product form means that each loop multiplicatively amplifies the others.
A 10% improvement in routing (Loop C4) compounds with a 10% improvement in
playbooks (Loop C5) to produce a 21% improvement overall -- the extra 1%
comes from the interaction term (better routing produces better episodes,
which produce better playbooks, which produce better prompts for the next
routing decision).

---

## 3. Seven Compounding Loops

### C1: Demurrage-Weighted Retrieval

Knowledge entries that are retrieved and used receive reinforcement (balance
increase). Entries that are not used decay via demurrage (Gesell 1916). This
creates a natural priority queue: frequently useful knowledge rises to the
top; stale knowledge sinks.

**Input from:** C3 (cleaner codebook -> more accurate retrieval)
**Output to:** C3 (used entries -> codebook refinement), C5 (retrieved
knowledge -> better prompts -> better episodes -> better playbooks)

### C2: Heuristic Calibration

Every Cell that implements a protocol emits predictions before execution and
receives corrections after. The predict-publish-correct loop (Friston 2006)
calibrates the Cell's internal model, reducing prediction error over time.

**Input from:** C4 (collective quality signals -> individual calibration)
**Output to:** C5 (calibrated predictions -> higher-quality playbook evidence)

### C3: HDC Codebook Cleanup

The HDC codebook (10,240-bit hyperdimensional computing vectors) grows as new
episodes and knowledge entries are fingerprinted. Redundant or near-duplicate
vectors are defragmented by the HDC clustering system, improving retrieval
precision.

**Input from:** C1 (used entries -> codebook priority), C7 (plugin similarity
-> codebook entries)
**Output to:** C1 (cleaner codebook -> faster, more accurate retrieval)

### C4: C-Factor Feedback

The C-Factor measures collective agent quality through five components:
turn-taking entropy, peer prediction accuracy, citation reciprocity, delivery
rate, and HDC diversity. High C-Factor signals that the agent collective is
performing well; low C-Factor triggers governance recommendations (model
adjustments, diversity injection).

**Input from:** All loops (every loop's outputs contribute episodes that
affect collective metrics)
**Output to:** C2 (collective signal -> individual calibration), C5
(governance -> playbook adjustments)

### C5: Playbook Distillation

Episodes are compressed into playbook rules via pattern mining. Playbook rules
are compressed into meta-playbooks (generalized rules) via MDL merging. Each
level of distillation produces more compact, more general knowledge.

**Input from:** C1 (retrieved knowledge -> enriched episodes), C2 (calibrated
predictions -> higher-quality episodes), C4 (collective quality -> rule
evidence)
**Output to:** C1 (playbooks injected -> better prompts -> episodes reinforce
knowledge), C4 (better playbooks -> better outputs -> higher C-Factor)

### C6: Cross-Deployment Commons

Knowledge, playbooks, and routing tables extracted from one deployment can
accelerate another deployment on similar tasks. The cross-deployment commons
is a shared pool of heuristics that grows with each deployment.

**Input from:** C5 (distilled playbooks -> commons contributions), C7
(plugin knowledge -> commons entries)
**Output to:** C5 (commons knowledge -> faster playbook bootstrap for new
deployments), C7 (commons -> plugin discoverability)

**Status:** Target design. Knowledge sync across workspaces is not yet
connected.

### C7: Plugin Ecosystem

Portable plugins with well-defined capability contracts create network
effects: each new plugin potentially catalyzes new tasks, which produce new
episodes, which produce new playbooks and knowledge.

**Input from:** C6 (commons knowledge -> plugin development context)
**Output to:** C3 (plugin usage patterns -> codebook entries), C6 (plugin
knowledge -> commons)

**Status:** Plugin ecosystem exists (E32 8/8) but network effects require
cross-workspace connectivity (C6).

---

## 4. Seven KPIs for Compounding

These KPIs measure whether the autocatalytic condition holds -- whether the
loops are connected and producing superlinear improvement:

| KPI | Measures | Expected Curve | Source |
|-----|----------|---------------|--------|
| Time to first PR | All loops together | Steep initial drop | Plan execution timestamps |
| Median tokens/task | C1 + C3 + C5 | Monotonic decrease | Efficiency events |
| Mean confidence width | C2 heuristic calibration | Decrease with trials | Playbook rule confidence |
| HDC cache hit rate | C3 codebook quality | Asymptote toward 1.0 | HDC query telemetry |
| Cohort C-Factor trend | C4 collective quality | Monotonic increase | C-Factor snapshots |
| Retroactive improvements/week | C5 playbook distillation | Increase then plateau | Playbook change log |
| Time from install to success | C6 commons effect | Decrease as commons grows | New deployment telemetry |

### 4.1 Monotonicity Check

Self-improvement should be monotonic: the system should get better over time,
not oscillate. Monotonicity is tracked via the C-Factor trend:

```
Monotonicity score = fraction of steps where C(t) > C(t-1)

If monotonicity < 0.60 over 20+ episodes:
    -> Learning system is not converging
    -> Investigate: oscillation? regression? environmental shift?
```

---

## 5. Strong Connectivity Test

The autocatalytic condition requires that the feedback graph be strongly
connected -- every loop has at least one input from another loop. The shipped
E25 implementation computes this via Tarjan's algorithm:

```rust
pub fn check_autocatalytic(graph: &FeedbackGraph) -> AutocatalyticStatus {
    // Check 1: no orphan loops
    let orphans: Vec<LoopId> = graph.loops.iter()
        .filter(|l| graph.incoming_edges(l.id).is_empty())
        .map(|l| l.id)
        .collect();

    if !orphans.is_empty() {
        return AutocatalyticStatus::Broken { orphans };
    }

    // Check 2: strong connectivity (Tarjan's algorithm)
    let sccs = tarjan_scc(&graph.adjacency);
    if sccs.len() == 1 && sccs[0].len() == graph.loops.len() {
        AutocatalyticStatus::Connected
    } else {
        AutocatalyticStatus::Fragmented { components: sccs }
    }
}
```

**Current status:** Loops C1-C5 are connected (single SCC). C6 and C7 are
disconnected (knowledge sync and plugin network effects not yet wired). The
system is therefore **fragmented** -- it compounds locally (within a single
deployment) but not globally (across deployments).

---

## 6. Relation to Triple-Loop Learning

The autocatalytic framework connects to Argyris and Schon's triple-loop
learning:

| Learning Level | Autocatalytic Role |
|---|---|
| **Single-loop** (error correction) | Individual loop iterations (gate fail -> fix -> retry) |
| **Double-loop** (strategy revision) | Cross-loop interactions (routing failure -> prompt change -> better playbook) |
| **Triple-loop** (learning to learn) | Autocatalytic metrics monitoring the compound effect of all loops together |

The autocatalytic metrics are Roko's triple-loop learning mechanism. They do
not improve any individual loop -- they measure whether the *interaction*
between loops is producing compounding improvement. When the autocatalytic
condition breaks (orphan loops, fragmentation), triple-loop learning detects
it and surfaces a governance recommendation.

---

## 7. Polya Urn Model (Tria et al. 2014)

The Polya urn model provides a mathematical framework for the "adjacent
possible" -- the set of things that become achievable once a new capability
exists. Each successful episode expands the adjacent possible by producing
new skills, knowledge, and patterns that enable previously impossible tasks.

The connection to Roko: each plan execution that succeeds adds entries to the
knowledge store, rules to the playbook, and pass/fail statistics to the
cascade router. These additions expand the set of tasks that subsequent plans
can handle. The urn gets richer with each draw, and the probability of success
on novel tasks increases.

This is the mathematical basis for the "steep initial drop" expected in the
Time-to-First-PR KPI: early executions add the most new knowledge (the urn
is sparse), and each addition disproportionately expands the adjacent possible.

---

## References

- Kauffman, S.A. "The Origins of Order: Self-Organization and Selection in
  Evolution." Oxford University Press, 1993.
- Argyris, C. & Schon, D.A. "Organizational Learning: A Theory of Action
  Perspective." Addison-Wesley, 1978.
- Tria, F., Loreto, V., Servedio, V.D.P. & Strogatz, S.H. "The Dynamics of Correlated
  Novelties." Scientific Reports 4, 5890, 2014. doi:10.1038/srep05890.
- Friston, K. "A free energy principle for the brain." Journal of Physiology -
  Paris, 100(1-3):70-87, 2006.
- Gesell, S. "The Natural Economic Order." 1916.

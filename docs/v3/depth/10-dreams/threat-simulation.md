# Threat Simulation Theory and Adversarial Dreaming

> **v3 depth file** -- `/docs/v3/depth/10-dreams/threat-simulation.md`
> Canonical source: v1 `docs/v1/10-dreams/09-threat-simulation.md`
> Implementation: `crates/roko-dreams/src/threat.rs`, `crates/roko-dreams/src/cycle.rs`
> Status: **Wired** -- `threat_warning_entries_with_floor` generates warning-type
> knowledge entries from dream consolidation; regime-aware scheduling and FMEA/FTA
> enumeration are specified

---

## 1. Threat Simulation Theory

Revonsuo (2000, Behavioral and Brain Sciences, "The reinterpretation of dreams:
An evolutionary hypothesis of the function of dreaming") proposed that biological
dreaming evolved primarily as a **threat rehearsal mechanism**. During sleep, the
brain simulates threatening scenarios, rehearses responses, and strengthens
threat-detection circuits -- all without real-world consequences.

In Roko, threat simulation is implemented as a specialized REM imagination mode
that deliberately constructs adversarial scenarios and rehearses the agent's
responses.

---

## 2. Trigger Conditions

Adversarial dreaming fires when:

1. **Recent failure**: Gate failure in the most recent cycle
2. **Novelty detection**: Unprecedented situation (low HDC similarity to any
   existing episode)
3. **Scheduled**: Every Nth dream cycle (default: every 3rd cycle)

---

## 3. Three-Tier Threat Taxonomy

| Tier | Description | Example (Coding Agent) |
|------|-------------|----------------------|
| **Tier 1: Known Threats** | Previously experienced | Compile errors from type mismatches |
| **Tier 2: Anticipated Threats** | Inferred from knowledge | Breaking changes in dependency updates |
| **Tier 3: Novel Threats** | Creative recombination of patterns | Simultaneous test failure + dep conflict + CI timeout |

---

## 4. Threat Generation Process

**Tier 1** (Known): Construct scenarios where known failures recur in different
contexts. Identify early warning signs. Propose different responses.

**Tier 2** (Anticipated): Based on current knowledge, generate threats not yet
encountered but plausible. Consider failure modes of current strategies, edge
cases of heuristics, scenarios where assumptions break.

**Tier 3** (Novel): Uses combinational creativity to imagine compound failures.
Combine elements from unrelated failure episodes to construct scenarios where
multiple failure types occur simultaneously.

---

## 5. FMEA and FTA

### Bottom-Up: Failure Mode and Effects Analysis

FMEA proceeds component-by-component, rating severity (S), occurrence (O), and
detection difficulty (D) on 1--10 scales. Risk Priority Number = S x O x D.

| RPN Range | Action |
|-----------|--------|
| >= 200 | Immediate mitigation required |
| 100--199 | Schedule for next dream cycle |
| < 100 | Monitor |

### Top-Down: Fault Tree Analysis

FTA decomposes undesired top-level events through AND/OR logic gates to reach
basic independently quantifiable events. Minimal Cut Set (MCS) analysis reveals
critical single points of failure.

---

## 6. Threat Severity Assessment

The severity assessor integrates multiple frameworks:

- **5x5 Risk Matrix**: Likelihood x Impact, normalized to [0, 1]
- **CVSS v4.0**: For external exploitation threats (4 metric groups)
- **DREAD Model**: Rapid 5-factor average assessment
- **Bayesian Prioritization**: P(threat | evidence) updated with Bayes' theorem

```rust
pub struct ThreatSeverityAssessor {
    pub likelihood_levels: usize,          // default: 5
    pub impact_levels: usize,              // default: 5
    pub critical_threshold: f64,           // default: 0.68
    pub high_threshold: f64,               // default: 0.40
    pub medium_threshold: f64,             // default: 0.20
    pub impact_override_level: u8,         // default: 9
    pub default_prior: f64,               // default: 0.10
}
```

---

## 7. Regime-Aware Scheduling

| Context | Threat Allocation | Rationale |
|---------|-------------------|-----------|
| Normal operations | 10% of REM | Maintenance vigilance |
| Recent failures | 30% of REM | Active threat learning |
| Novel environment | 25% of REM | Anticipatory modeling |
| Post-crisis recovery | 40% of REM | Intensive rehearsal |

---

## 8. Advanced Red Teaming

The threat simulation engine integrates techniques from:

- **TAP** (Mehrotra et al., NeurIPS 2024): Tree-structured attack exploration
  with adaptive branching and pruning
- **WILDTEAMING** (NeurIPS 2024): Mining real-world failure episodes for attack
  patterns
- **MAD-MAX** (arXiv 2025): Modular atomic attack composition (97% ASR)
- **AutoRedTeamer** (arXiv 2025): Lifelong learning of attack patterns across
  sessions

```rust
pub struct AdvancedRedTeamConfig {
    pub tree_structured_attacks: bool,      // default: true
    pub max_attack_tree_depth: usize,       // default: 5
    pub max_branches_per_node: usize,       // default: 3
    pub prune_threshold: f64,               // default: 0.15
    pub mine_failure_episodes: bool,        // default: true
    pub max_primitives_per_compound: usize, // default: 4
    pub persistent_attack_knowledge: bool,  // default: true
}
```

---

## 9. Defensive Countermeasures

Constitutional Classifiers (Sharma et al. 2025, arXiv:2501.18837) provide the
defensive layer: dual-layer input + output classifiers reduce jailbreak success
from 86% baseline to 4.4%.

Dream-generated hypotheses are treated as potentially adversarial outputs that
the classifier evaluates before staging.

---

## 10. Integration with Daimon

The Daimon's affect engine interacts with threat simulation in two ways:

1. **Threat salience**: High negative arousal episodes are prioritized for
   threat simulation (somatic marker hypothesis, Damasio 1994)
2. **Depotentiation**: After threat simulation, emotional charge of rehearsed
   scenarios is reduced (Walker & van der Helm 2009)

---

## 11. Academic Citations

| Paper | Contribution |
|-------|-------------|
| Revonsuo (2000), BBS | Threat Simulation Theory |
| Walker & van der Helm (2009) | REM depotentiation of threat memories |
| Damasio (1994) | Somatic markers for threat salience |
| MITRE ATLAS v5.1.0 (2025) | AI/ML adversarial threat taxonomy |
| Perez et al. (2022, EMNLP) | LM-based red teaming strategies |
| Sharma et al. (2025), arXiv:2501.18837 | Constitutional Classifiers |
| FIRST, CVSS v4.0 (2023) | Vulnerability scoring framework |

---

## 12. Cross-References

| Document | Relevance |
|----------|-----------|
| [rem-imagination.md](rem-imagination.md) | REM phase that threat simulation operates within |
| [consolidation-and-staging.md](consolidation-and-staging.md) | Staging buffer for threat rehearsal outputs |
| [advanced-dream-concepts.md](advanced-dream-concepts.md) | Nightmare detection and containment |

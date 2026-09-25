# Cross-Domain HDC Transfer

> **v3 depth -- 09-memory** | Source: v1/06-neuro/08

HDC's algebraic structure enables automatic detection of structural analogies
across domains -- an Insight learned in coding ("high complexity -> more
review") transfers to chain ("high volatility -> more caution") because both
encode the abstract pattern `BIND(high_uncertainty, more_verification)`.

---

## How Cross-Domain Transfer Works

### The encoding mechanism

Three entries from three different domains:

**Coding:** "High-complexity modules need more code review"
```
BIND(role_risk_factor, hv_high_complexity) XOR BIND(role_response, hv_more_review)
```

**Chain:** "High-volatility assets need more caution before trading"
```
BIND(role_risk_factor, hv_high_volatility) XOR BIND(role_response, hv_more_caution)
```

**Research:** "Contradictory sources need more verification before citing"
```
BIND(role_risk_factor, hv_contradictory_sources) XOR BIND(role_response, hv_more_verification)
```

All three share: `BIND(role_risk_factor, hv_X) XOR BIND(role_response, hv_Y)`.
The shared role vectors create measurable similarity (typically 0.53--0.58)
above the 0.526 threshold.

### Role vector hierarchy

**Abstract roles** (enable cross-domain transfer):

| Role | Encodes |
|------|---------|
| `role:risk_factor` | What creates risk |
| `role:response` | How to respond |
| `role:pattern` | Observable signal |
| `role:severity` | How serious |
| `role:temporal` | Time dimension |
| `role:confidence` | Certainty level |

**Domain-specific roles** (within-domain precision only):
- Coding: `role:crate`, `role:function`, `role:module`
- Chain: `role:protocol`, `role:asset`, `role:pool`
- Research: `role:source`, `role:citation`, `role:method`

### Analogical reasoning

HDC answers "A is to B as C is to ?" using binding:
```
relationship = BIND(hv_rust, hv_borrow_checker)
answer = BIND(relationship, hv_java)
nearest(answer, codebook) --> hv_garbage_collector
```

---

## Resonance Detection

Cross-correlation loop runs when a new entry is ingested:

```rust
pub struct ResonanceConfig {
    pub cross_domain_threshold: f32,       // 0.526
    pub max_resonances_per_entry: usize,   // 5
    pub generate_descriptions: bool,        // true
}
```

Algorithm: for each new entry E, compare against all stored entries in
domains != E.domain. If similarity > 0.526, emit a Resonance alert.

### Confirmation protocol (2+ agents)

Cross-domain analogies require 2+ independent agents to confirm before
acceptance:

```
P(joint FP) = P(agent_1 FP) * P(agent_2 FP) = (7.3 x 10^-8)^2 = 5.3 x 10^-15
```

---

## Transfer Risk Assessment

Not all structural analogies are beneficial. Transfer risk factors:

1. **Surface/deep mismatch**: Shared abstract roles but different causal
   mechanisms
2. **Negative transfer**: Domain divergence exceeds critical threshold
   (Ben-David et al. 2010)
3. **Adversarial resonance**: Malicious crafted entries in multi-agent mesh

### Domain distance metric

Three-component distance: vocabulary divergence (Jaccard), structural
divergence (1 - mean HDC similarity), and outcome correlation (Pearson on
gate pass rates).

| Domain A | Domain B | Combined Distance | Safety |
|----------|----------|------------------|--------|
| Rust coding | TypeScript | 0.24 | High |
| Coding | DeFi chain | 0.52 | Moderate |
| Coding | Research | 0.43 | Moderate |
| DeFi | Research | 0.62 | Low |

---

## Academic Foundations

- Gentner, D. (1983). "Structure-mapping." *Cognitive Science*, 7(2).
- Hofstadter, D. R. (2001). "Analogy as the Core of Cognition." MIT Press.
- Ben-David, S. et al. (2010). "A theory of learning from different
  domains." *Machine Learning*, 79(1-2).
- Kanerva, P. (2009). "Hyperdimensional Computing." *Cognitive Computation*.

---

## Cross-References

- `hdc-knowledge-encoding.md` -- the encoding pipeline
- `false-positive-math.md` -- threshold derivation
- `library-of-babel.md` -- cross-collective knowledge flows

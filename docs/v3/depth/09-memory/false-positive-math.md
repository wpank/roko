# False Positive Math for HDC Similarity

> **v3 depth -- 09-memory** | Source: v1/06-neuro/09. FULL probability
> derivations.

With 10,240-bit BSC vectors, a threshold of 0.526 guarantees <1% false
positive rate against a 100K vocabulary after Bonferroni correction.

---

## Statistical Foundation

### Distribution of random similarity

For two independent random 10,240-bit binary vectors, the Hamming similarity
(fraction of matching bits) follows a distribution with:

```
Expected value:     mu    = 0.5
Variance:           sigma^2 = 1 / (4D) = 1 / (4 * 10,240) = 2.441 x 10^-5
Standard deviation: sigma   = 1 / (2 * sqrt(D)) = 1 / (2 * sqrt(10240))
                            = 1 / (2 * 101.19) = 0.00494
```

### Derivation of variance

Each bit position contributes independently. For bit position i:
- P(match) = P(both 0) + P(both 1) = 0.5 * 0.5 + 0.5 * 0.5 = 0.5
- Variance of each indicator = 0.5 * (1 - 0.5) = 0.25

The similarity is the mean of D independent Bernoulli(0.5) variables:
```
Var(sim) = Var(1/D * sum(I_i)) = 1/D^2 * D * 0.25 = 0.25/D = 1/(4D)
```

By the Central Limit Theorem, for D = 10,240:
```
sim ~ N(0.5, 0.00494^2)
```

### Z-score and false positive rate

A similarity of `s` corresponds to Z-score:
```
Z = (s - 0.5) / sigma = (s - 0.5) / 0.00494
```

False positive rate (probability of a random pair exceeding threshold s):
```
P(sim > s) = 1 - Phi(Z)
```
where Phi is the standard normal CDF.

---

## Threshold Table

| Threshold | Z-score | Per-Comparison FP Rate | Use Case |
|-----------|---------|----------------------|----------|
| 0.505 | 1.01 | 15.6% | Too low -- noise |
| 0.510 | 2.02 | 2.17% | Rough screening |
| 0.512 | 2.43 | 0.75% | Single-pair check |
| 0.515 | 3.04 | 0.12% | Conservative single-pair |
| 0.520 | 4.05 | 2.6 x 10^-5 | Moderate vocabulary |
| **0.526** | **5.26** | **7.3 x 10^-8** | **100K vocabulary (Bonferroni)** |
| 0.530 | 6.07 | 6.5 x 10^-10 | 1M vocabulary |
| 0.540 | 8.10 | < 10^-15 | Extremely conservative |
| 0.550 | 10.12 | < 10^-23 | Near-certain match |

### Derivation for 0.526 threshold

```
Z = (0.526 - 0.5) / 0.00494 = 0.026 / 0.00494 = 5.26

P(Z > 5.26) = 1 - Phi(5.26)

Using the Mills ratio approximation for large Z:
  1 - Phi(Z) ~ phi(Z) / Z

phi(5.26) = (1/sqrt(2*pi)) * exp(-5.26^2 / 2)
          = 0.3989 * exp(-13.83)
          = 0.3989 * 9.86 x 10^-7
          = 3.93 x 10^-7

P(Z > 5.26) ~ 3.93 x 10^-7 / 5.26 = 7.47 x 10^-8 ~ 7.3 x 10^-8
```

---

## Bonferroni Correction for Multiple Comparisons

When scanning N entries, probability of **at least one** false positive:
```
P(at least 1 FP) ~= N * P(single FP)    (for small P)
```

To maintain overall FP rate alpha across N comparisons:
```
P(single FP) <= alpha / N
```

### Threshold selection by vocabulary size

| Vocabulary (N) | Target alpha | Required per-comparison FP | Required Z | Threshold |
|----------------|-------------|---------------------------|-----------|-----------|
| 100 | 1% | 10^-4 | 3.72 | 0.518 |
| 1,000 | 1% | 10^-5 | 4.26 | 0.521 |
| 10,000 | 1% | 10^-6 | 4.75 | 0.523 |
| **100,000** | **1%** | **10^-7** | **5.26** | **0.526** |
| 1,000,000 | 1% | 10^-8 | 5.73 | 0.528 |
| 10,000,000 | 1% | 10^-9 | 6.00 | 0.530 |

### Derivation for N = 100,000

```
Required per-comparison FP = alpha / N = 0.01 / 100,000 = 10^-7

Need: 1 - Phi(Z) <= 10^-7

Using the Mills ratio: phi(Z) / Z <= 10^-7

Solving numerically:
  Z = 5.26 gives phi(5.26)/5.26 = 7.3 x 10^-8 < 10^-7  ✓

Threshold = 0.5 + Z * sigma = 0.5 + 5.26 * 0.00494 = 0.5 + 0.02598 = 0.526
```

---

## Multi-Agent Confirmation

Two-agent confirmation at 0.526 threshold:
```
P(joint FP) = P(agent_1 FP) * P(agent_2 FP)
            = (7.3 x 10^-8)^2
            = 5.3 x 10^-15
```

With three agents, 2-of-3 confirmation:
```
P(2+ of 3 confirm) = 3 * p^2 * (1-p) + p^3
                    ~ 3 * (7.3 x 10^-8)^2 * 1
                    = 1.6 x 10^-14
```

Effectively zero false positives with multi-agent confirmation.

---

## JL Bound Validation

| N (entries) | epsilon | Minimum D | D=10,240 sufficient? |
|-------------|---------|-----------|---------------------|
| 1,000 | 0.1 | 553 | Yes (18.5x headroom) |
| 10,000 | 0.1 | 737 | Yes (13.9x headroom) |
| 100,000 | 0.1 | 921 | Yes (11.1x headroom) |
| 100,000 | 0.05 | 3,682 | Yes (2.8x headroom) |
| 1,000,000 | 0.05 | 4,423 | Yes (2.3x headroom) |
| 1,000,000 | 0.03 | 12,286 | **No** (needs D >= 12,288) |

---

## Practical Implications

- **Within-domain queries**: Use lower threshold (0.51--0.52) because the
  domain constraint already reduces false positive space.
- **Cross-domain queries**: Use full 0.526 threshold.
- **Somatic landscape**: k-d tree uses different threshold requirements
  (lower, within single domain).

---

## Academic Foundations

- Johnson, W. B. & Lindenstrauss, J. (1984). "Extensions of Lipschitz
  mappings into a Hilbert space." *Contemporary Mathematics*, 26.
- Kanerva, P. (2009). "Hyperdimensional Computing." *Cognitive Computation*.
- Bonferroni, C. E. (1936). "Teoria statistica delle classi e calcolo delle
  probabilita." *Pubblicazioni del R. Istituto Superiore*, 8, 3--62.

---

## Cross-References

- `hdc-vsa-foundations.md` -- dimension choice
- `cross-domain-hdc-transfer.md` -- resonance detection
- `knowledge-query-api.md` -- query API parameters

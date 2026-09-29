# HDC Operations: Bind, Bundle, Permute, Similarity

> **v3 depth -- 09-memory** | Source: v1/06-neuro/05

The four algebraic operations of Binary Spatter Codes -- XOR bind,
majority-vote bundle, cyclic-shift permute, and Hamming similarity -- form a
complete algebra for encoding, composing, and querying knowledge in Neuro.

---

## 1. Bind (XOR)

### Definition

Binding associates two hypervectors into a new vector quasi-orthogonal to
both inputs. It encodes a **relationship** between two concepts:

```
bind(A, B) = A XOR B    (componentwise XOR)
```

The result is a genuinely new representation, not a blend.

### Algebraic properties

| Property | Formula | Significance |
|----------|---------|-------------|
| **Self-inverse** | bind(bind(A, B), B) = A | No separate unbind needed |
| **Commutative** | bind(A, B) = bind(B, A) | Symmetric unless permute is used |
| **Associative** | bind(A, bind(B, C)) = bind(bind(A, B), C) | Multi-way binding in any order |
| **Distributes over bundle** | bind(A, bundle(B, C)) = bundle(bind(A, B), bind(A, C)) | Structured queries work |

Distributivity makes structured queries possible:

```
record = bundle(bind(role_language, hv_rust), bind(role_topic, hv_async))
answer = bind(record, role_language)  -->  approximately hv_rust
```

### Rust implementation

```rust
impl HdcVector {
    pub fn bind(&self, other: &Self) -> Self {
        let mut bits = [0u64; 160];
        for (slot, (left, right)) in bits.iter_mut()
            .zip(self.bits.iter().zip(other.bits.iter()))
        {
            *slot = left ^ right;
        }
        Self { bits }
    }
}
```

**Performance:** 160 XOR operations on u64 words. ~5 ns scalar, ~2 ns
AVX-512.

---

## 2. Bundle (Majority Vote)

### Definition

Bundling superimposes multiple hypervectors into a single aggregate
**similar to all inputs**:

```
bundle(A, B, C)[i] = majority(A[i], B[i], C[i])
```

Where `majority(bits)` returns 1 if more than half the input bits are 1, and
0 otherwise. Ties break to 0 for determinism.

### Properties

| Property | Details |
|----------|---------|
| **Similarity preservation** | sim(bundle(A, B), A) approx sim(bundle(A, B), B) > 0.5 |
| **Capacity** | SNR = sqrt(D/K) for K bundled items; K < 100 for reliable retrieval |
| **NOT associative** | bundle(bundle(A, B), C) != bundle(A, bundle(B, C)) |
| **Requires accumulator** | Incremental bundling needs integer vote counts |

### Rust implementation

```rust
impl HdcVector {
    pub fn bundle(vectors: &[&Self]) -> Self {
        if vectors.is_empty() { return Self::zeros(); }
        let len = vectors.len();
        let mut bits = [0u64; 160];
        for (word_index, slot) in bits.iter_mut().enumerate() {
            let mut word = 0u64;
            for bit_index in 0..64 {
                let mut ones = 0usize;
                for vector in vectors {
                    ones += ((vector.bits[word_index] >> bit_index) & 1) as usize;
                }
                if ones * 2 > len {
                    word |= 1u64 << bit_index;
                }
            }
            *slot = word;
        }
        Self { bits }
    }
}
```

**Performance:** O(D x K). K=10: ~800 ns. K=100: ~8 us.

---

## 3. BundleAccumulator

Because bundling is not associative over binary vectors, incremental bundling
requires a per-bit vote accumulator:

```rust
pub struct BundleAccumulator {
    votes: Vec<i32>,    // 10,240 entries, 40 KB
    pub count: usize,
}
```

### add() -- unweighted vector addition

For each bit position: bit == 1 adds +1 to votes, bit == 0 subtracts 1.
This bipolar encoding (+1/-1) centers the vote distribution at zero, making
the majority threshold a simple sign check.

```rust
pub fn add(&mut self, hv: &HdcVector) {
    self.count += 1;
    for word_idx in 0..160 {
        let word = hv.bits[word_idx];
        for bit in 0..64 {
            let pos = word_idx * 64 + bit;
            if (word >> bit) & 1 == 1 {
                self.votes[pos] += 1;
            } else {
                self.votes[pos] -= 1;
            }
        }
    }
}
```

### add_weighted() -- scalar-weighted addition

Equivalent to calling `add()` abs(weight) times in one O(D) pass. Negative
weights subtract. Use cases: recency weighting, trust weighting, undo.

### finish() -- collapse to binary vector

```rust
pub fn finish(&self) -> HdcVector {
    let mut bits = [0u64; 160];
    for word_idx in 0..160 {
        let mut word = 0u64;
        for bit in 0..64 {
            let pos = word_idx * 64 + bit;
            if self.votes[pos] > 0 {
                word |= 1u64 << bit;
            }
        }
        bits[word_idx] = word;
    }
    HdcVector { bits }
}
```

Ties (votes == 0) break to 0 for determinism.

### decay() -- controlled forgetting

```rust
pub fn decay(&mut self, factor: f32) {
    for vote in self.votes.iter_mut() {
        *vote = (*vote as f32 * factor) as i32;
    }
}
```

Decay half-life in number of calls: `-ln(2) / ln(factor)`.

| factor | half-life (calls) |
|--------|------------------|
| 0.90 | 6.6 |
| 0.95 | 13.5 |
| 0.99 | 69.0 |

---

## 4. Permute (Cyclic Shift)

### Definition

```
permute(A, k) = cyclic_left_shift(A, k)
```

Each distinct shift count produces a quasi-orthogonal vector.

### Properties

| Property | Details |
|----------|---------|
| **Group operation** | permute(permute(A, j), k) = permute(A, j+k) |
| **Invertible** | permute(permute(A, k), D-k) = A |
| **Quasi-orthogonality** | permute(A, k) is quasi-orthogonal to A for k >= 1 |
| **Preserves similarity** | sim(permute(A, k), permute(B, k)) = sim(A, B) |

### Rust implementation

```rust
impl HdcVector {
    pub fn permute(&self, n: usize) -> Self {
        let bits_len = self.bits.len() * 64;
        let n = n % bits_len;
        if n == 0 { return *self; }
        let word_shift = n / 64;
        let bit_shift = n % 64;
        let mut bits = [0u64; 160];
        for (index, slot) in bits.iter_mut().enumerate() {
            let src0 = (index + 160 - word_shift) % 160;
            *slot = if bit_shift == 0 {
                self.bits[src0]
            } else {
                let src1 = (src0 + 159) % 160;
                (self.bits[src0] << bit_shift) | (self.bits[src1] >> (64 - bit_shift))
            };
        }
        Self { bits }
    }
}
```

**Performance:** ~10 ns. Used for:

**CausalLink encoding:**
```
causal_hv = bind(permute(hv_cause, 1), permute(hv_effect, 2))
```

**Sequence encoding:**
```
seq_hv = bundle(permute(step1, 0), permute(step2, 1), permute(step3, 2))
```

---

## 5. Similarity (Hamming Distance)

### Definition

```
sim(A, B) = 1 - hamming_distance(A, B) / D
```

### Interpretation

| Range | Meaning |
|-------|---------|
| 1.0 | Identical vectors |
| > 0.526 | Meaningful relationship (Bonferroni-corrected for 100K) |
| > 0.52 | Meaningful relationship (single-pair check) |
| 0.48 -- 0.52 | Noise band (quasi-orthogonal) |
| < 0.48 | Meaningful dissimilarity (anti-correlated) |
| 0.0 | Bitwise complement |

### Rust implementation

```rust
impl HdcVector {
    pub fn similarity(&self, other: &Self) -> f32 {
        let mut differing_bits = 0u32;
        for (left, right) in self.bits.iter().zip(other.bits.iter()) {
            differing_bits += (left ^ right).count_ones();
        }
        let differing_bits = u16::try_from(differing_bits).unwrap_or(u16::MAX);
        1.0_f32 - (f32::from(differing_bits) / 10_240.0_f32)
    }
}
```

**Performance:** 160 XOR + POPCNT operations: ~13 ns on x86 with SIMD.

---

## Advanced Operations

### Fractional binding (BSC approximation)

Probabilistic bit flipping approximates FHRR fractional binding:
- alpha = 0.0: returns self (no binding)
- alpha = 1.0: returns bind(self, other)
- alpha = 0.5: halfway between

### Word-level trigram encoding

Captures local semantic context using permutation-binding chains:
```
trigram(A, B, C) = bind(permute(A, 2), bind(permute(B, 1), C))
```

### Stochastic decay for binary vectors

Each bit flips with probability (1 - factor) / 2, creating a fading effect
where the vector gradually approaches random noise.

---

## Academic Foundations

- Kanerva, P. (2009). "Hyperdimensional Computing." *Cognitive Computation*.
- Kleyko, D. et al. (2022). "A Survey on Hyperdimensional Computing." *ACM
  Computing Surveys*, 54(6).
- Frady, E. P. et al. (2020). "Resonator Networks." *Neural Computation*.
- Plate, T. A. (2003). *Holographic Reduced Representations.* CSLI.

---

## Cross-References

- `hdc-vsa-foundations.md` -- mathematical foundations and dimension choice
- `hdc-knowledge-encoding.md` -- how these operations encode entries
- `cross-domain-hdc-transfer.md` -- structural analogy via shared roles
- `false-positive-math.md` -- similarity threshold derivation

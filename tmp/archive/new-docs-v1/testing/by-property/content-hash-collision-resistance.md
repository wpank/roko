# Content Hash Collision Resistance

> Two distinct byte sequences must not produce the same ContentHash (sampled probabilistic guarantee).

**Crate**: `roko-core`
**Test type**: Unit test (sampled)
**Enforcement**: BLAKE3 hash function
**Last reviewed**: 2026-04-19

---

## Statement

For all byte slices b₁ ≠ b₂ (sampled over test cases):
`ContentHash::from_bytes(b₁) ≠ ContentHash::from_bytes(b₂)`

This is a probabilistic guarantee: BLAKE3 provides 2^-256 collision probability, not zero.

---

## Test

Collision resistance is tested by sampling 10,000 random pairs and checking for equality. A collision in a test run would indicate a catastrophic hash function failure.

---

## See also

- [content-addressing-determinism.md](content-addressing-determinism.md)

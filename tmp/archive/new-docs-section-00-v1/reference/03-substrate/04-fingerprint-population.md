# Fingerprint Population

> When an `Engram` is stored via `put`, its HDC fingerprint must exist for `query_similar`
> to work. This page explains how and when fingerprints are populated.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Put, Get, Query](./02-put-get-query.md), [HDC Fingerprint](../10-types/hdc-fingerprint.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`Substrate::put` is responsible for populating a fingerprint if the incoming `Engram` does
not already have one. Fingerprints are derived from the `Engram`'s content using the HDC
encoding pipeline. An `Engram` with a pre-populated fingerprint is stored as-is (no
re-derivation). Records without fingerprints are stored but excluded from `query_similar`.

---

## The Pipeline

When `put(engram)` is called:

```
Engram arrives
    │
    ▼
engram.fingerprint == None?
    │ Yes                  │ No
    ▼                      ▼
Extract textual       Store as-is
or structural
content from Body
    │
    ▼
Tokenize / encode
into HDC symbol
    │
    ▼
Bind symbols into
a single binary
HdcFingerprint vector
    │
    ▼
Store with fingerprint
```

---

## Derivation Details

HDC encoding converts content into a high-dimensional binary vector. The process:

1. **Tokenise** — The `Body` variant determines the tokeniser:
   - `Body::Text(s)` — split on word boundaries; each unique word token maps to a random
     binary vector (the "item memory" for that token, seeded from the token's hash).
   - `Body::Structured(map)` — each key-value pair maps to a bound vector
     (`key_vector XOR value_vector`).
   - `Body::Embedding(vec)` — if a float embedding vector is already present, it is
     binarised via sign (positive → 1, non-positive → 0).
   - Other variants (`Body::Json`, `Body::Binary`) — serialise to bytes, then hash-encode
     chunks as symbols.

2. **Bind & Bundle** — All per-token vectors are combined by majority vote (bundle):
   ```
   fingerprint[i] = majority_vote(token_vectors[0][i], token_vectors[1][i], ...)
   ```
   This produces a single vector that is correlated with all tokens.

3. **Store** — The resulting `HdcFingerprint` (D bits, default D = 10,000) is assigned to
   `engram.fingerprint`.

---

## When Fingerprints Are Pre-Populated

Callers may supply their own fingerprint before calling `put`. This is correct when:

- A downstream model (embedding model, HDC encoder) ran prior to `put`.
- The `Engram` was received from another agent with a fingerprint already attached.
- The fingerprint was loaded from a checkpoint.

In these cases, `put` must not re-derive — it must store the supplied fingerprint verbatim.
This preserves the semantic of "the fingerprint belongs to the content it was derived from."

---

## Records Without Fingerprints

An `Engram` may legitimately have no fingerprint (e.g., a `Body::Binary` blob with no
textual content, or a record created before the fingerprint system existed). These records:

- Are stored and retrievable via `get` and `query`.
- Are excluded from `query_similar` results.
- Do not cause errors.

The runtime may emit a warning metric (`substrate.fingerprint.missing`) for monitoring
purposes. See [Performance](./13-performance.md).

---

<!-- ADDED: section inferred from architecture context -->
## Cost of Fingerprint Derivation

HDC encoding is fast — typically O(n·D/64) where n is the token count. For a 100-word
`Body::Text`, derivation takes < 1 ms on a modern CPU. This is acceptable in the STORE
step of the loop (not in the hot RECALL path). The cost is dominated by the bundle
(majority-vote) step which processes D = 10,000 bits per token.

---

## See Also

- [HDC Fingerprint](../10-types/hdc-fingerprint.md) — the vector type and its dimensionality
- [Query Similar](./03-query-similar.md) — what fingerprints are used for
- [Engram Data Type](../01-engram/README.md) — the `fingerprint` field location

## Open Questions

- Should fingerprint derivation be made pluggable (a trait on its own) so that teams can
  swap in their own embedding models?
- Should `put` return the populated fingerprint so the caller can use it immediately without
  a subsequent `get`?

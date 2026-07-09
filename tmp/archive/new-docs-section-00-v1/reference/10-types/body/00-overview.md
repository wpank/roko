# Body — Overview

> The enum that holds the actual content of an Engram: text, a vector embedding, JSON, binary data, or structured key-value fields.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Engram](../../01-engram/00-overview.md)  
**Used by**: [ContentHash](../content-hash/00-overview.md), [HDC Fingerprint](../hdc-fingerprint/00-overview.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

`Body` is a five-variant enum: `Text(String)`, `Embedding(Vec<f32>)`, `Json(serde_json::Value)`,
`Binary(Vec<u8>)`, and `Structured(BTreeMap<String, Body>)`. It is included in the Engram's
`ContentHash`. Only `Text`, `Json`, and `Structured` bodies produce HDC fingerprints —
`Binary` does not.

---

## The Idea

An Engram can carry many kinds of payloads:
- A written claim or answer (`Text`).
- A semantic vector from an LLM encoder (`Embedding`).
- A structured data record (`Json` or `Structured`).
- An opaque binary blob (`Binary`) for file content or serialized objects.

All of these are first-class Engram payloads. The `Body` enum provides a single type that
covers all cases without requiring separate Engram types.

---

## Specification

```rust
<!-- source: crates/roko-core/src/body.rs -->

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Body {
    /// A UTF-8 string. The most common payload for KnowledgeEntry and AgentOutput.
    Text(String),

    /// A dense f32 vector. Typically 384 or 768 dimensions from an embedding model.
    Embedding(Vec<f32>),

    /// An arbitrary JSON value. Used for structured events and records.
    Json(serde_json::Value),

    /// Raw binary data. Not fingerprintable. No text analysis.
    Binary(Vec<u8>),

    /// Recursive key-value structure. Each value is itself a Body.
    Structured(BTreeMap<String, Body>),
}
```

---

## Variant Summary

| Variant | Fingerprintable? | In ContentHash? | Typical use |
|---|---|---|---|
| `Text` | Yes (word tokens) | Yes | Written claims, agent outputs, reflections |
| `Embedding` | Yes (quantized bins) | Yes | Semantic vectors from LLMs |
| `Json` | Yes (leaf values) | Yes | Structured events, tool traces |
| `Binary` | **No** | Yes | File content, opaque blobs |
| `Structured` | Yes (key-value binding) | Yes | Typed multi-field records |

---

## Body Equality

`Body` derives `PartialEq`. For `Embedding`, equality is exact bit-for-bit comparison of
the `f32` bytes (no tolerance). Two embeddings that are semantically similar but not
bitwise identical are not equal.

For semantic similarity, use the [HDC fingerprint](../hdc-fingerprint/00-overview.md)
comparison rather than `==`.

---

## Open Questions

- Should `Embedding` carry the model identifier that produced the vector, to prevent
  comparing embeddings from different models? Not currently included in Body; could be
  a tag.
- Should `Text` be capped at a maximum length? No current limit; large texts will
  produce large canonical encodings.

## See Also

- [`01-variant-reference.md`](01-variant-reference.md) — detailed per-variant reference
- [`02-canonical-bytes.md`](02-canonical-bytes.md) — hash encoding
- [`../hdc-fingerprint/02-encoding-pipeline.md`](../hdc-fingerprint/02-encoding-pipeline.md) — Body to HDC

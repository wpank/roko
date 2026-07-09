# Body — Variant Reference

> Per-variant documentation: purpose, size, encoding, and usage guidance.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Last reviewed**: 2026-04-19

---

## Text

```rust
Text(String)
```

**Purpose**: A UTF-8 string carrying a written claim, answer, or natural-language
knowledge item. The most common Body for `KnowledgeEntry`, `AgentOutput`, and `Reflection`.

**Size**: No hard limit. Recommend ≤ 32 KB for a single Engram; split longer content into
a lineage chain of smaller Engrams.

**HDC encoding**: Word-level tokenization (lowercase, punctuation stripped). Short texts
(< 3 tokens) produce sparse vectors with low discrimination.

**ContentHash encoding**: Length-prefixed UTF-8 bytes. See [canonical bytes](02-canonical-bytes.md).

**When to use**:
- Written factual claims.
- Agent answers to questions.
- Reflective self-assessments.
- Error messages (prefer `ErrorRecord` Kind with `Text` body over a bare `Text` Engram).

---

## Embedding

```rust
Embedding(Vec<f32>)
```

**Purpose**: A dense floating-point vector, typically 384, 768, or 1536 dimensions,
produced by an LLM embedding model.

**Size**: 384 × 4 = 1,536 bytes; 1536 × 4 = 6,144 bytes. Stored verbatim.

**HDC encoding**: Each `f32` is quantized to a 256-bin integer; the integers are treated
as tokens for HDC projection. This loses some precision but is sufficient for near-duplicate
detection.

**ContentHash encoding**: Little-endian f32 concatenation (no length prefix beyond the
outer length prefix on the whole body).

**When to use**:
- Pre-computed embeddings from an LLM (e.g., OpenAI `text-embedding-3-small`).
- When the agent needs fast semantic search over a large Substrate without on-demand
  re-encoding.
- Store the embedding model identifier in `tags["embedding_model"]` for traceability.

**Caution**: Embeddings from different models are not comparable. Tag with model ID.

---

## Json

```rust
Json(serde_json::Value)
```

**Purpose**: An arbitrary JSON value for structured events, records, and data payloads
that don't fit into the `Structured` key-value model.

**Size**: No limit. Large JSON values make `canonical_encode()` expensive.

**HDC encoding**: DFS traversal of the JSON tree; each leaf string/number is a token.
Null values and empty arrays produce no tokens.

**ContentHash encoding**: `serde_json::to_vec()` (compact, deterministic within a single
process but not guaranteed stable across serde_json versions).

**When to use**:
- Tool trace payloads with complex nested structure.
- API responses captured as-is.
- When the schema is not known at compile time.

**Caution**: Floating-point values in JSON may serialize differently across platforms.
For hash-critical data, use `Structured` with `Text` leaves.

---

## Binary

```rust
Binary(Vec<u8>)
```

**Purpose**: An opaque byte sequence. Used for file content, serialized protobuf messages,
image thumbnails, or any payload that cannot be meaningfully tokenized.

**Size**: No limit. Very large binaries should be stored externally with a content-addressed
pointer (hash of the binary + URL) stored in a `Text` or `Json` body instead.

**HDC encoding**: **Not encodable.** `HdcEncoder::encode()` returns `None` for `Body::Binary`.
The Engram will have `fingerprint = None`.

**ContentHash encoding**: Bytes directly, without transformation.

**When to use**:
- Raw file content (with Kind::Observation or a Custom Kind).
- Serialized binary data structures.
- Cases where text analysis is meaningless.

---

## Structured

```rust
Structured(BTreeMap<String, Body>)
```

**Purpose**: A typed multi-field record where each field name maps to a nested `Body`.
The `Structured` variant is the body for well-typed records whose schema is known at
compile time.

**Size**: Total serialized size is the sum of all nested Body sizes. No explicit limit.

**HDC encoding**: Each `(key, value)` pair is encoded as `project(key) XOR encode(value)`,
then all pairs are bundled via majority vote. This binds key context to value content in
the vector space.

**ContentHash encoding**: Fields are sorted by key (BTreeMap order) and each key-value
pair is encoded with length prefixes. See [canonical bytes](02-canonical-bytes.md).

**When to use**:
- When the record schema is defined in code (`struct`-like).
- When individual fields need to be findable via key-specific search.
- `ContextAssembly` bodies (`{engrams: [id1, id2], token_count: 1024}`).
- `Prediction` bodies (`{subject: "...", expected_value: "...", confidence: 0.8}`).

**Notes**: `Structured` bodies support recursive nesting (`Body::Structured` containing
another `Body::Structured`). Depth is not limited but deeply nested structures may be
slow to encode.

---

## Choosing the Right Variant

```
Is the payload natural language?          → Text
Is the payload a pre-computed embedding?  → Embedding
Is the schema dynamic / from an API?      → Json
Is the payload opaque bytes?              → Binary
Is the schema known at compile time?      → Structured
```

---

## Open Questions

- Should `Embedding` carry a model ID field directly in the variant? Currently modelled
  via tags.
- Should `Binary` support a MIME type annotation?

## See Also

- [`00-overview.md`](00-overview.md) — Body overview
- [`02-canonical-bytes.md`](02-canonical-bytes.md) — encoding for each variant
- [`../hdc-fingerprint/02-encoding-pipeline.md`](../hdc-fingerprint/02-encoding-pipeline.md) — HDC encoding per variant

# Engram — Serialization

> Engrams serialize to JSONL for the file substrate and to a compact binary format for in-memory transfer. This page covers both formats, versioning, and migration.

**Status**: Shipping  
**Crate**: `roko-core`, `roko-fs`  
**Depends on**: [Struct reference](01-struct-reference.md)  
**Used by**: `roko-fs` (FileSubstrate), `roko-serve` (HTTP API), `roko-runtime` (IPC)  
**Last reviewed**: 2026-04-19

---

## TL;DR

The Substrate's wire format is JSONL (one JSON object per line, newline-delimited).
Each field serializes as a typed JSON value. The ContentHash serializes as a lowercase
hex string. The HdcVector serializes as a base64-encoded byte array. Schema version is
embedded in every serialized Engram for forward compatibility.

---

## The Idea

JSONL is human-readable, streamable, and append-only — properties that match the
Substrate's write pattern (append Engrams to a shard file). Binary formats are faster
but not grep-able; JSONL is a good default for a system where inspecting the substrate
is a normal debugging operation.

The binary format (`roko-core::binary`) is used for high-throughput paths: IPC between
subsystems, snapshot transmission, and the replication protocol. It uses the same field
order as the canonical encoding for the ContentHash, so hash verification is trivially
fast.

---

## Specification

### JSONL Format

Each Engram serializes to a single JSON line:

```json
{
  "schema_version": 1,
  "id": "7f83b1657...",
  "fingerprint": {
    "vector_b64": "AAAA...",
    "encoder_version": 4
  },
  "kind": "AgentOutput",
  "body": {
    "AgentOutput": {
      "text": "The answer is 42.",
      "model": "claude-3-7-sonnet",
      "prompt_tokens": 512,
      "completion_tokens": 8,
      "finished_normally": true
    }
  },
  "created_at_ms": 1745065200000,
  "decay": {
    "Demurrage": {
      "balance": 1.0,
      "idle_tax_per_day": 0.01,
      "reinforcement_per_use": 0.05
    }
  },
  "provenance": {
    "author": "roko-agent-v2.1",
    "trust": "SelfVerified",
    "tainted": false,
    "custody": []
  },
  "score": {
    "confidence": 0.85,
    "novelty": 0.4,
    "utility": 0.9,
    "reputation": 0.5,
    "precision": null,
    "salience": null,
    "coherence": null
  },
  "lineage": ["3a4c8d2e..."],
  "tags": { "session_id": "sess-abc123" }
}
```

### Field Encoding Rules

| Field | JSON encoding |
|-------|--------------|
| `id` | Lowercase hex string (64 chars) |
| `fingerprint.vector_b64` | Base64-encoded 1280-byte array (160 × u64 LE) |
| `fingerprint.encoder_version` | Integer |
| `kind` | PascalCase string matching variant name |
| `body` | Object with one key = variant name, value = variant struct |
| `created_at_ms` | Integer |
| `decay` | Object with one key = variant name, value = params struct |
| `provenance.trust` | PascalCase string: `"LocalAgent"`, `"SelfVerified"`, etc. |
| `score.*` | Float or null (extended axes are null when absent) |
| `lineage` | Array of lowercase hex strings |
| `tags` | Object (string → string) |

### Schema Version

Every serialized Engram carries `"schema_version": N`. The current version is **1**.
Deserializers must reject Engrams with a version they do not understand, not silently
ignore unknown fields.

---

## Versioning and Migration

### Forward Compatibility

When a new field is added to `Engram`:

1. The schema version is incremented.
2. The old deserializer ignores the new field (JSONL is lenient for reads).
3. The new serializer writes the new field.
4. A migration pass (offline, on-demand) backfills existing JSONL shards.

### ContentHash Stability Across Versions

Adding a field to `Engram` does **not** change the ContentHash of existing Engrams,
because the canonical encoding only includes a fixed set of fields (see
[`02-content-hash.md`](02-content-hash.md)). Adding a `schema_version` field does
not affect identity.

### Body Variant Additions

When a new `Kind` and corresponding `Body` variant are added:

1. New variant is added to `Kind` and `Body` enums.
2. `non_exhaustive` attribute ensures all match arms require a wildcard.
3. Operators that do not handle the new Kind pass through (no panic).
4. Schema version incremented to signal presence of new Body variant.

---

## Binary Format

For the binary format used in IPC and replication, see ADDED (details not in source):

The binary format is a length-prefixed sequence of fields in canonical encoding order:

```
[u32 LE schema_version] [32 bytes id] [u8 has_fingerprint] [1280 bytes vector | 0] [u32 LE encoder_version | 0] [u32 LE kind_tag] [u32 LE body_len] [body_bytes] [i64 LE created_at_ms] [u32 LE decay_tag] [decay_params] [u32 LE author_len] [author_bytes] [u8 trust] [u8 tainted] [u32 LE lineage_count] [lineage_bytes] [u32 LE tags_count] [key-value pairs]
```

<!-- ADDED: binary format details inferred from canonical encoding spec and IPC usage in roko-runtime. Not in source docs. -->

---

## Invariants

1. Schema version must be present in all serialized Engrams
2. `id` in JSONL must be exactly 64 lowercase hex chars
3. `fingerprint.vector_b64` must decode to exactly 1280 bytes
4. Deserialized Engram must pass `verify_id()` before use

---

## Failure Modes

| Failure | Cause | Recovery |
|---------|-------|----------|
| Deserialization error | Corrupted JSONL line | Skip line; log; increment error counter |
| Hash mismatch on deserialization | JSONL tampered | Reject Engram; log audit event |
| Unknown schema version | Reader is older than writer | Hard error; do not silently ignore |
| Unknown Kind variant | Writer uses a variant the reader doesn't know | Pass through with `Kind::Custom`; log warning |

---

## See Also

- [`12-invariants.md`](12-invariants.md) — invariants enforced at deserialization
- [`../10-types/content-hash/01-canonicalization.md`](../10-types/content-hash/01-canonicalization.md) — canonical encoding spec

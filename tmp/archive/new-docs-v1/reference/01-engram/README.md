# Engram — The Universal Datum

> Every durable record in Roko is an Engram. This folder is the canonical reference for the Engram type.

**Status**: Shipping  
**Crate**: `roko-core`  
**Last reviewed**: 2026-04-19

---

## What Is an Engram?

An Engram is the single data type through which every component in the Roko system
communicates durable information. Agent outputs, gate verdicts, tool traces, knowledge
entries, predictions, scoring metadata — every persistent record is an Engram.

The name comes from neuroscience: an engram is the hypothetical physical trace of a memory
in the brain (Semon 1904). In Roko, an Engram is its digital analogue — a content-addressed,
scored, decaying, lineage-tracked unit of cognition.

> **Historical note.** The shipping Rust codebase uses the identifier `Signal` for the
> Engram type, in `roko-core`. `Signal` is the retired name. All architecture documentation
> uses `Engram`. When you read code you will see `Signal`; when you read docs you will read
> `Engram`. They refer to the same struct.

---

## Contents

| # | Page | What it covers | Status |
|---|------|----------------|--------|
| [00](00-overview.md) | Overview | What an Engram is; why one universal datum | Shipping |
| [01](01-struct-reference.md) | Struct reference | Every field, every type, every invariant | Shipping |
| [02](02-content-hash.md) | ContentHash | BLAKE3 identity, canonicalization | Shipping |
| [03](03-hdc-fingerprint.md) | HDC fingerprint | 10240-bit vector, binding, similarity | Shipping |
| [04](04-kind-enum.md) | Kind enum | Every variant, decision tree | Shipping |
| [05](05-body-enum.md) | Body enum | Every Body variant, payload types | Shipping |
| [06](06-lineage-dag.md) | Lineage DAG | Parent links, cycle prevention, query patterns | Shipping |
| [07](07-builder-pattern.md) | Builder pattern | Builder API, defaults, required fields | Shipping |
| [08](08-scoring-fields.md) | Scoring fields | How Score attaches to an Engram | Shipping |
| [09](09-decay-fields.md) | Decay fields | How Decay attaches to an Engram | Shipping |
| [10](10-provenance-fields.md) | Provenance fields | How Provenance attaches to an Engram | Shipping |
| [11](11-serialization.md) | Serialization | JSONL, binary, versioning, migration | Shipping |
| [12](12-invariants.md) | Invariants | What must always be true; where it is enforced | Shipping |
| [13](13-examples.md) | Examples | 10+ worked examples, minimal to complex | Shipping |
| [14](14-api-reference.md) | API reference | Full public Rust API, method by method | Shipping |
| [15](15-rationale-and-history.md) | Rationale & history | Design choices, rejected alternatives, `Signal` retirement | Shipping |

---

## Suggested Reading Order

**New to Roko:** 00 → 01 → 04 → 05 → 13  
**Implementing a new operator:** 01 → 08 → 09 → 10 → 14  
**Understanding audit trails:** 01 → 06 → 10  
**Debugging identity issues:** 02 → 12  
**Working with HDC search:** 03 → the `10-types/hdc-fingerprint/` folder  

---

## See Also

- [`reference/10-types/score/`](../10-types/score/README.md) — the 7-axis scoring model
- [`reference/10-types/decay/`](../10-types/decay/README.md) — decay variants
- [`reference/10-types/provenance/`](../10-types/provenance/README.md) — author, trust, taint, custody
- [`reference/10-types/content-hash/`](../10-types/content-hash/README.md) — BLAKE3 identity
- [`reference/10-types/hdc-fingerprint/`](../10-types/hdc-fingerprint/README.md) — HDC vectors
- [`reference/02-pulse/README.md`](../02-pulse/README.md) — ephemeral events (the Engram's counterpart)

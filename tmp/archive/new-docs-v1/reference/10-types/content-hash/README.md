# ContentHash

> The identity digest of an Engram: a 32-byte BLAKE3 hash of its stable, canonical fields.

## Contents

| # | Page | What it covers | Status |
|---|---|---|---|
| 00 | [Overview](00-overview.md) | What ContentHash is, what it hashes, and why | Shipping |
| 01 | [Canonical Encoding](01-canonical-encoding.md) | The exact byte layout fed to BLAKE3 | Shipping |
| 02 | [API Reference](02-api-reference.md) | Every public method on ContentHash | Shipping |
| 03 | [Invariants and Collision Resistance](03-invariants.md) | Security properties, invariants, and test coverage | Shipping |
| 04 | [Examples](04-examples.md) | Worked examples for construction, verification, and edge cases | Shipping |

## Suggested reading order

For readers new to ContentHash: 00 → 01 → 03.  
For readers implementing a storage backend: 01 → 02 → 03.  
For readers debugging identity mismatches: 01 → 03 → 04.

## See also

- [`../provenance/04-hash-inclusion-rules.md`](../provenance/04-hash-inclusion-rules.md) — which Engram fields enter the hash
- [`../../01-engram/02-content-hash.md`](../../01-engram/02-content-hash.md) — ContentHash in Engram context
- [`../hdc-fingerprint/`](../hdc-fingerprint/) — the complementary semantic fingerprint

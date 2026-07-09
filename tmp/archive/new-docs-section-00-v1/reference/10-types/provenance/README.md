# Provenance

> How Roko tracks the origin, trust level, and taint of an Engram across its lifetime.

## Contents

| # | Page | What it covers | Status |
|---|---|---|---|
| 00 | [Overview](00-overview.md) | Provenance struct, its three fields, and how it relates to identity | Shipping |
| 01 | [Author Field](01-author.md) | The author string: format, conventions, and identity-hash inclusion | Shipping |
| 02 | [Trust Level](02-trust-level.md) | The four TrustLevel variants and their numeric weights | Shipping |
| 03 | [Taint Flags](03-taint-flags.md) | The optional taint set: what flags mean and how they propagate | Shipping |
| 04 | [Hash Inclusion Rules](04-hash-inclusion-rules.md) | Which provenance fields enter the ContentHash and why | Shipping |
| 05 | [Provenance Propagation](05-provenance-propagation.md) | How provenance flows through lineage chains | Shipping |
| 06 | [Trust Escalation](06-trust-escalation.md) | How TrustLevel is upgraded through peer verification and chain witness | Shipping |
| 07 | [Invariants](07-invariants.md) | All provenance invariants and enforcement locations | Shipping |
| 08 | [API Reference](08-api-reference.md) | Every public method on the Provenance type | Shipping |
| 09 | [Examples](09-examples.md) | Worked examples for creation, propagation, and escalation | Shipping |

## Suggested reading order

For readers new to provenance: 00 → 01 → 02 → 04 → 05.  
For readers implementing trust escalation: 02 → 06 → 07.  
For readers debugging identity issues: 04 → 07 → the [ContentHash overview](../content-hash/00-overview.md).

## See also

- [`../content-hash/`](../content-hash/) — how provenance contributes to Engram identity
- [`../../01-engram/10-provenance-fields.md`](../../01-engram/10-provenance-fields.md) — provenance in Engram context
- [`../../01-engram/12-invariants.md`](../../01-engram/12-invariants.md) — Engram-level invariants involving provenance

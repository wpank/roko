# Body

> The content payload of an Engram — the actual knowledge it carries.

## Contents

| # | Page | What it covers | Status |
|---|---|---|---|
| 00 | [Overview](00-overview.md) | Body enum, its role in identity, and variant summary | Shipping |
| 01 | [Variant Reference](01-variant-reference.md) | One section per Body variant: purpose, encoding, size limits | Shipping |
| 02 | [Canonical Bytes](02-canonical-bytes.md) | The exact byte encoding for each variant used in ContentHash | Shipping |
| 03 | [API Reference and Invariants](03-api-reference.md) | Methods and invariants for the Body type | Shipping |

## Suggested reading order

For new readers: 00 → 01.  
For implementers hashing or comparing bodies: 01 → 02 → 03.  
For implementers encoding to HDC: 00 → the [HDC encoding pipeline](../hdc-fingerprint/02-encoding-pipeline.md).

## See also

- [`../../01-engram/05-body-enum.md`](../../01-engram/05-body-enum.md) — Body in Engram context
- [`../hdc-fingerprint/02-encoding-pipeline.md`](../hdc-fingerprint/02-encoding-pipeline.md) — Body-to-fingerprint encoding
- [`../content-hash/01-canonical-encoding.md`](../content-hash/01-canonical-encoding.md) — Body in ContentHash

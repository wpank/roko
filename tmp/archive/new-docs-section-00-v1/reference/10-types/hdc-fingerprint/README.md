# HDC Fingerprint

> The 10,240-bit hyperdimensional binary vector that encodes an Engram's semantic content for approximate similarity search.

## Contents

| # | Page | What it covers | Status |
|---|---|---|---|
| 00 | [Overview](00-overview.md) | What an HDC fingerprint is and why Roko uses it | Shipping |
| 01 | [HdcVector Format](01-hdc-vector.md) | The `[u64; 160]` binary sparse code representation | Shipping |
| 02 | [Encoding Pipeline](02-encoding-pipeline.md) | How Body content is converted to an HdcVector | Shipping |
| 03 | [Similarity and Distance](03-similarity-distance.md) | Hamming distance, similarity threshold, and near-duplicate detection | Shipping |
| 04 | [Encoder Versioning](04-encoder-versioning.md) | How encoder upgrades are handled transparently | Shipping |
| 05 | [Invariants](05-invariants.md) | All HDC fingerprint invariants | Shipping |
| 06 | [Examples](06-examples.md) | Worked examples for encoding, comparison, and deduplication | Shipping |

## Suggested reading order

For readers new to HDC: 00 → 01 → 03.  
For implementers encoding content: 01 → 02 → 05.  
For readers building a deduplication pipeline: 03 → 06.

## See also

- [`../../01-engram/03-hdc-fingerprint.md`](../../01-engram/03-hdc-fingerprint.md) — fingerprint in Engram context
- [`../content-hash/`](../content-hash/) — the complementary identity hash

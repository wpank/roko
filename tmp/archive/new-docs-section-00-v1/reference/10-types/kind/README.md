# Kind

> The semantic type of an Engram, declaring what cognitive role the knowledge plays.

## Contents

| # | Page | What it covers | Status |
|---|---|---|---|
| 00 | [Overview](00-overview.md) | The Kind enum, its role in identity, and the full variant list | Shipping |
| 01 | [Variant Reference](01-variant-reference.md) | One-paragraph description of every Kind variant | Shipping |
| 02 | [Kind and Decay](02-kind-and-decay.md) | How Kind determines default decay parameters | Shipping |
| 03 | [API Reference and Invariants](03-api-reference.md) | Methods, encoding, and invariants | Shipping |

## Suggested reading order

For new readers: 00 → 01.  
For implementers choosing a Kind: 01 → 02.  
For implementers writing a serializer or content hash: 00 → 03.

## See also

- [`../../01-engram/04-kind-enum.md`](../../01-engram/04-kind-enum.md) — Kind in Engram context
- [`../decay/08-tier-matrix.md`](../decay/08-tier-matrix.md) — per-Kind default decay

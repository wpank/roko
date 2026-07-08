# ISFR Clearing Idempotence

> Submitting the same clearing batch twice produces the same final state as submitting it once.

**Crate**: `roko-chain`
**Test type**: Unit test
**Enforcement**: `IsfrClearingContract::clear`
**Last reviewed**: 2026-04-19

---

## Statement

For all clearing batches B and states S:
`clear(clear(S, B), B) == clear(S, B)`

---

## Why It Matters

The clearing contract must handle duplicate submissions (network retries, replay attacks) without settling positions twice.

---

## See also

- [../by-subsystem/subsystem-chain.md](../by-subsystem/subsystem-chain.md)
- [substrate-idempotence.md](substrate-idempotence.md) — same pattern at the storage layer

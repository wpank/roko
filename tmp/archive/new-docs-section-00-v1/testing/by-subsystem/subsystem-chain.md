# roko-chain — Test Coverage

> 52 tests for the Korai EVM: on-chain types, token arithmetic, HDC precompile, and ISFR clearing.

**Status**: Built (52 tests; blocked by chain deployment)
**Crate**: `roko-chain`
**Section**: 08 — Chain / Korai
**Last reviewed**: 2026-04-19

---

## Test Count: 52

Source: implementation status audit, 2026-04-17.

| Module | Approx. tests | Focus |
|---|---|---|
| `identity` | ~10 | Soulbound passport construction, field validation |
| `reputation` | ~12 | 7-domain EMA reputation update, scoring |
| `tokens` | ~12 | KORAI/DAEJI token arithmetic, demurrage |
| `hdc_precompile` | ~8 | HDC precompile at ~400 gas, correctness |
| `isfr_clearing` | ~10 | KKT certificate construction, clearing correctness |

---

## Key Test Focus Areas

### Soulbound Identity Passports

- A passport can only be minted once per address (soulbound invariant).
- Attempting to transfer a passport returns `Err(Soulbound)`.
- Passport fields (capability domains, reputation domains) are set at mint and update correctly.

Key property: [../by-property/soulbound-non-transferability.md](../by-property/soulbound-non-transferability.md).

### 7-Domain EMA Reputation

- The 7 reputation domains (quality, reliability, speed, safety, creativity, accuracy, collaboration) are independent.
- EMA update: after a positive outcome, the relevant domain's score increases by the expected EMA step.
- After 50 rounds, reputation converges toward the true mean (EMA convergence).
- Reputation scores are in [0, 1] regardless of input.

Key property: [../by-property/reputation-ema-convergence.md](../by-property/reputation-ema-convergence.md).

### Token Arithmetic (KORAI/DAEJI)

- Token balances are non-negative; a debit below balance returns `Err(InsufficientBalance)`.
- Demurrage at 1% annual rate: after 365 days, a balance is multiplied by 0.99.
- Token arithmetic is exact (no floating point; uses fixed-point arithmetic).
- Transfer is atomic: a failed transfer leaves both balances unchanged.

Key property: [../by-property/token-demurrage-rate.md](../by-property/token-demurrage-rate.md).

### HDC Precompile

- The precompile computes the same result as the in-process HDC library.
- Gas cost is bounded at ~400 gas per 10,240-bit operation.
- The precompile is deterministic: same input → same output, same gas.

### ISFR Clearing

- A valid KKT certificate is accepted by the clearing contract.
- An invalid KKT certificate (infeasible allocation) is rejected.
- Clearing is idempotent: clearing the same batch twice produces the same state as clearing it once.

Key property: [../by-property/isfr-clearing-idempotence.md](../by-property/isfr-clearing-idempotence.md).

---

## Known Gaps

- All 52 tests run against an in-process EVM simulator; no tests against a live testnet.
- The Spore/Sparrow job marketplace contracts have minimal test coverage.
- No property tests for the full clearing cycle under adversarial allocation requests.
- Testing is blocked by chain deployment (no live testnet to run integration tests against).

## See also

- [../by-property/soulbound-non-transferability.md](../by-property/soulbound-non-transferability.md)
- [../by-property/token-demurrage-rate.md](../by-property/token-demurrage-rate.md)
- [../gaps-and-roadmap.md](../gaps-and-roadmap.md) — chain testing roadmap

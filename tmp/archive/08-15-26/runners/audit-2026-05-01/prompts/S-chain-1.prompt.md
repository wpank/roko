# S-chain-1: Inventory dormant chain modules

## Task
Audit document. Inventory which `roko-chain` modules have non-test callers in production code. Identify deletion candidates. Output to `logs/S-chain-1-inventory.md`.

## Runner Context
Runner audit-2026-05-01, group S. No dependencies. Wave 1.

## Source plan
`tmp/subsystem-audits/implementation-plans/34-chain-deploy-cleanup.md` § CD-1.

## Exact changes

This batch produces an audit doc.

### 1. List `roko-chain` modules

```bash
ls crates/roko-chain/src/
```

Likely candidates per the audit:
- `marketplace.rs` (~2K LOC)
- `identity_economy_markets.rs` (similar)
- `pricing_*` (speculative pricing schemes)
- `reputation_registry.rs` (used? check)

### 2. Map callers

For each module:

```bash
rg 'use roko_chain::|roko_chain::<module>' crates/ -g '*.rs' \
  | rg -v 'crates/roko-chain/'
```

For each call site, note whether it's production or test.

### 3. Categorize

For each module:

- **Has production callers**: keep; document.
- **Has only test callers**: deletion candidate; the corresponding tests will go too.
- **Zero callers**: deletion candidate.
- **Behind feature gate `experimental`**: keep gated.

### 4. Output `logs/S-chain-1-inventory.md`

```markdown
# S-chain-1: roko-chain caller inventory

Generated: 2026-05-01

## Module callers

| Module | LOC | Production callers | Test-only callers | Disposition |
|---|---|---|---|---|
| `marketplace.rs` | ~2000 | (T5-37 deprecated) | mod tests | DELETE after T5-37 lands |
| `identity_economy_markets.rs` | ~1800 | (T5-37 deprecated) | mod tests | DELETE after T5-37 lands |
| `pricing_curves.rs` | ~600 | none | none | DELETE |
| `pricing_strategies.rs` | ~800 | none | mod tests | DELETE |
| `reputation_registry.rs` | ~400 | F07 (collusion detector) | mod tests | KEEP |
| `chain_wallet.rs` | ~600 | serve agents.rs, deploys.rs | none | KEEP |
| ...

## Deletion follow-up batches (file separately)

- `S-chain-3`: delete `pricing_curves.rs`, `pricing_strategies.rs`, `marketplace.rs`, `identity_economy_markets.rs`.
- `S-chain-4`: feature-gate experimental `*_speculative.rs` modules.

## Estimated LOC removal

- Definite deletions: ~5K
- Conditional deletions (post T5-37): ~4K
- Total: ~9K of the ~14K dormant claim
```

(The audit's "14K dormant" refers to crate-wide dormancy; subset is concretely deletable.)

## Write Scope
- `tmp/runners/audit-2026-05-01/logs/S-chain-1-inventory.md` (new)

## Verify

```bash
ls tmp/runners/audit-2026-05-01/logs/S-chain-1-inventory.md
wc -l tmp/runners/audit-2026-05-01/logs/S-chain-1-inventory.md
# Expect > 30 lines
```

## Do NOT

- Do NOT touch any source file.
- Do NOT bundle with S-chain-2.
- Do NOT mark a module deletable if it has any production caller.
- Do NOT propose deleting `roko-chain` entirely. The crate has real members (chain_wallet, reputation_registry).

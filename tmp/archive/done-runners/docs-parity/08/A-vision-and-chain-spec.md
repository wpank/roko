# A — Vision, Chain Spec, Token Economics, Status

This section covers the high-level chain vision and the documents that most
easily drift into present-tense claims. The audit direction is simple:
Korai is still a target-state design, not a shipping chain runtime.

## Verdict

| Doc Area | Refresh Direction |
|---|---|
| Vision/framing | keep, but label as target-state framing |
| Chain spec | Phase 2+ design only |
| Token economics | deferred |
| "Current status" narrative | rewrite around the minimal shipped code surface |

## What Can Be Said In Present Tense

- `roko-chain` ships client and wallet traits.
- `AlloyChainClient`, `WalletGate`, and `TxSimGate` ship.
- `ChainWitnessEngine` ships as a small witness primitive.
- Solidity demo contracts exist under `contracts/src/`.

That is the usable foundation for parity material. It is not a Korai node,
not a tokenized economy, and not a deployed chain specification.

## What Must Be Marked Target-State

- Korai block/finality parameters
- Korai RPC namespace and runtime behavior
- chain deployment modes and validator topology
- KORAI token economics
- demurrage and any knowledge-economy model
- any status language that implies the chain roadmap is already implemented

## Rewrite Guidance

When this batch points back to `docs/08-chain/00-02,24`, it should describe
those docs this way:

- `00` is architectural intent for a future chain domain.
- `01` is a planned chain-spec document, not a live runtime description.
- `02` is a deferred economics design.
- `24` should be treated as a narrow status note about the small shipped
  foundation, not as evidence that Korai itself is live.

## Carry Forward

If later work wants to build a real chain runtime, token, or deployment model,
that belongs to a separate execution phase after doc honesty is restored here.

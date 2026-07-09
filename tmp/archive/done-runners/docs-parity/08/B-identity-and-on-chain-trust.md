# B — Identity And On-Chain Trust

Verdict: `DEFERRED`

The identity and trust material in topic `08` should not be treated as a
shipping subsystem. Even where the repo contains exploratory Solidity
contracts, the parity story must stay conservative: these are precursors and
design probes, not a fully realized on-chain trust stack.

## What To Say Now

- Identity and trust are planned chain capabilities.
- Any registry or passport-like contract surface should be described as
  demo or precursor work unless and until the broader Korai runtime exists.
- The parity objective is to remove present-tense overclaiming, not to
  expand the identity narrative.

## What To Avoid

- Saying Korai Passport is a live product surface.
- Saying Ventriloquist defense or a full trust pipeline ships.
- Saying ERC-8004-style identity, reputation, and validation flows are fully
  deployed as part of a shipping Korai network.

## Minimal Handoff

Keep later agents focused on one rule: if a claim depends on a complete
on-chain identity/trust system, label it `Phase 2+` or `DEFERRED`.

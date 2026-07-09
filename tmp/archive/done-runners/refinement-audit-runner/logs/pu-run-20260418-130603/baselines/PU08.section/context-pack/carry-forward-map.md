# Carry-Forward Map — Batch 08

Use this when an item appears during batch `08` work but is better executed elsewhere.

| Item | Better Home | Keep In 08 As | Why |
|------|-------------|---------------|-----|
| Korai v1 Solidity contracts | post-self-hosting Tier-6 execution pass | roadmap note | not doc-audit work |
| `MirageChainClient` implementation | later mirage integration pass | explicit gap note | real code gap, but not the doc-audit critical path |
| libp2p / GossipSub / p2p mesh | later network pass | frontier banner | no shipping owner yet |
| x402 / payment channels / state channels | later payments pass | frontier banner | not implemented in this repo |
| ISFR solver / KKT verifier | later settlement pass | proxy-only clarification | shipping repo has only the proxy |
| TEE / ZK / Binius privacy work | later privacy pass | frontier banner | Phase 2+ |
| renaming `ChainWitnessEngine` in Rust | optional cleanup pass | disambiguation note | docs can solve confusion first |
| demo-contract migration to Korai v1 | later Solidity pass | migration note | out of scope for doc parity |

When deferring, record:

1. the exact file or gap id,
2. why it is out of scope,
3. the batch or pass that should own it,
4. the minimal status contract batch `08` still needs to leave behind.

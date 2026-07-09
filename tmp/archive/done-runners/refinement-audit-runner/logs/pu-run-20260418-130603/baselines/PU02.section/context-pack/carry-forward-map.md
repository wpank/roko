# Carry-Forward Map — Batch 02

Use this when an item appears during batch `02` work but is better executed elsewhere.

| Item | Better Home | Keep In 02 As | Why |
|------|-------------|---------------|-----|
| pool activation in orchestrator | `tmp/docs-parity/01` follow-on | runtime note | agent pools exist, but runtime scheduling owns activation |
| gate strictness by temperament | `tmp/docs-parity/04` | config + deferral note | verification owns gate policy semantics |
| adaptive routing rewards by temperament | `tmp/docs-parity/05` | learning note | learning owns reward tuning |
| domain/plugin scaffold generator | `tmp/docs-parity/03` | process note | domain activation is composition work |
| concrete feedback collectors | `tmp/docs-parity/05` | interface note | collectors matter because of learning ingestion |
| supervision-tree recovery wiring | `tmp/docs-parity/01` | restart seam note | executor recovery owns process restart policy |
| Darwin-Godel / shared memory systems | post-parity roadmap | roadmap note | Phase 2+ |
| research reasoning systems (Reflexion, ToT, ToolRAG, speculative) | post-parity or `05`-adjacent research pass | research note | these are not parity-critical runtime hardening tasks |

When deferring, record:

1. the exact file or gap id,
2. why it is out of scope,
3. the batch that should own it,
4. the minimal contract batch `02` still needs to leave behind.

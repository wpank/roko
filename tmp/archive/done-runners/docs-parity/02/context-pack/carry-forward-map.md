# Carry-Forward Map — Batch 02

Use this when an item appears during batch `02` audit work but should be handed off instead of expanded in place.

| Item | Better Home | Keep In 02 As | Why |
|---|---|---|---|
| pool activation in orchestrated runtime | `tmp/docs-parity/01` follow-on | runtime evidence note | the pools exist; scheduler activation is orchestration-owned |
| gate strictness by temperament | `tmp/docs-parity/04` | verification handoff note | gate policy semantics belong to verification |
| adaptive routing rewards by temperament | `tmp/docs-parity/05` | learning handoff note | reward tuning belongs to learning |
| response-surface crate moves | later code-execution batch | ownership seam note | real issue, but not a docs-refresh deliverable |
| `AgentEvent` enum unification | later code-execution batch | concrete integration gap note | real target, but it crosses crates |
| domain profiles and six-domain packaging | `tmp/docs-parity/03` or roadmap | explicit deferral note | present docs overstate runtime/product maturity |
| plugin SPI tiers 4-5 | roadmap | explicit deferral note | speculative relative to current user pressure |
| concrete feedback collectors | `tmp/docs-parity/05` | interface handoff note | collectors matter because of learning ingestion |
| supervision-tree recovery wiring | `tmp/docs-parity/01` | restart seam note | restart behavior is executor-owned |
| research reasoning systems | post-parity roadmap or `05`-adjacent research pass | research deferral note | not required to describe current shipped parity |
| Darwin/Godel, shared memory, archive systems | post-parity roadmap | roadmap note | Phase 2+ systems, not current audit scope |

When deferring, record:

1. the exact file, gap id, or callsite
2. the current code evidence
3. why it is out of scope for a single-agent 90-minute pass
4. the batch or roadmap bucket that should own it next
5. the minimum contract batch `02` still needs to leave behind

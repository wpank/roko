# Carry-Forward Map — Batch 00

Use this when topic `00` uncovers a real issue that should not be solved inside this docs refresh.

If a parity sentence would only become true after new code lands, the correct move here is to
rewrite the sentence and carry the implementation work forward.

| Item | Better Home | Keep In `00` As | Why |
|------|-------------|-----------------|-----|
| generic `Bus<E>` trait | future code parity / kernel cleanup | planned minimal addition | coherent idea, but not shipped today |
| event enum unification | future code parity | current-state note | the live transport issue is event-shape drift, not missing Pulse docs |
| `HdcSubstrate` / `ChainSubstrate` | later parity or code work | planned implementation note | valid target-state, not current parity debt |
| attention-token economy / VCG | later `03` + `05` parity | deferred concept | zero implementation |
| cognitive immune system layers beyond taint/attestation | `11-safety` parity | future-work note | current safety spine already exists, but the full CIS stack does not |
| temporal topology | later `05` parity | deferred concept | zero implementation |
| emergent goals / worldview clustering | later `01` + `05` parity | deferred concept | zero implementation |
| plugin tiers, web UI, gRPC | topic-specific future planning | deferred concept | outside architecture-truth refresh |
| synergy matrix / moat framing | future planning docs | planning-artifact note | current matrix overclaims the live primitive set |
| long-horizon roadmap | future planning docs | dependency ordering only | current roadmap is overscoped for the present docs-only pass |
| quarter-by-quarter staffing posture | future planning docs | planning artifact note | batch `00` should not preserve a live 5-7 engineer framing |

When deferring, preserve four things:

1. the exact doc or parity file involved,
2. the current-state truth,
3. the smallest future-state note worth keeping,
4. the better home for the full treatment.

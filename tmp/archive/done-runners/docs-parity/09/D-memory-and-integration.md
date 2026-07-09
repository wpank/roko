# D — Memory and Integration (Docs 09, 10, 11)

This section should not read as "mostly missing."

The shipping runtime already has:

- `EmotionalTag` in the shared kernel surface
- emotional congruence scoring in Neuro retrieval
- emotional provenance carried through consolidation
- live PAD-fed `SystemPromptBuilder` guidance
- live affect multipliers in the shared prompt auction
- live `DaimonPolicy` routing bias

The real parity work is to separate those shipped integrations from the
smaller set of coding-agent features that are still frontier.

Generated: 2026-04-18

---

## Current Read

| Area | Status | Parity note |
|------|--------|-------------|
| `EmotionalTag` core schema | DONE | shipping struct is stable and shared |
| emotional congruence in retrieval | PARTIAL | live in Neuro; not yet equally deep everywhere |
| emotional provenance in consolidation | DONE | durable signal already flows through Neuro |
| integration points in Doc 10 | PARTIAL | all four have some live wiring, but not equal depth |
| PAD-fed prompt guidance | DONE | `SystemPromptBuilder` integration is live |
| coding-agent integration in Doc 11 | FRONTIER | per-crate confidence, familiarity, and fatigue do not ship |

---

## D.01 — `EmotionalTag` is a shipping contract, and the docs should match it

**Status**: DONE

The main correction for Doc 09 is schema honesty.

What ships:

- PAD snapshot
- intensity
- trigger
- mood snapshot

What does not ship:

- a stored Plutchik label
- an `emotion: String` field
- discovery-emotion schema variants presented as active runtime shape

This is one of the clearest places where parity should prefer the live
struct over older examples.

---

## D.02-D.04 — Emotional retrieval already exists, but at uneven depth

The audit correction here is important: emotional retrieval is not a
future-only idea.

What already ships:

- emotional congruence scoring in Neuro retrieval
- PAD similarity in scoring
- emotional provenance persistence
- emotional diversity / validation-arc style reliability hints

What remains partial:

- broader cross-subsystem rollout of the same weighting depth
- fuller somatic-landscape-backed knowledge selection
- cleaner explanation in docs about which layer owns what

One ownership note should stay explicit:

- `ContextAssembler` lives in `roko-neuro`
- `roko-compose` re-exports it

That prevents future agents from chasing the wrong implementation home.

---

## D.05-D.07 — The four integration points are real, but not equally deep

Doc 10 should land on a per-point status view rather than a binary
built/unbuilt story.

Recommended parity read:

- behavioral state selection: shipping
- routing bias via `DaimonPolicy`: shipping
- prompt-auction affect bidding: shipping but approximate
- somatic landscape querying/modulation: shipping but still narrower than the broadest doc language

That framing preserves the main truth:

the integration path is live today.

---

## D.06 — VCG language should be narrowed, not deleted

**Status**: PARTIAL

The prompt auction already consumes affect. The parity issue is that
some doc wording still reads like the full VCG story is complete.

Better wording:

- live affect multipliers and shared auction selection ship
- diagnostic externality-style output exists
- exact fairness/payment accounting remains frontier

That keeps the real integration visible without overselling the current
economic depth.

---

## D.08-D.10 — Doc 11 is frontier, not present-tense runtime

**Status**: FRONTIER

Three ideas need explicit frontier tagging:

- per-crate confidence aggregation
- error-pattern familiarity scaling
- fatigue detection

These are plausible next steps, but they are not part of the active
Daimon runtime contract today. Parity should say that plainly.

One useful contrast helps:

- prompt affect guidance already ships
- coding-agent deepening does not

That keeps Doc 11 from flattening live and planned work into one bucket.

---

## D.11-D.12 — Prompt and UI surfaces should stay in the shipping bucket

`SystemPromptBuilder` is already a live consumer of the current affect
state. The TUI/CLI surfaces also expose affect state, even if the full
visualization/tone-mapping story remains shallower than the richest doc
wording.

So the section should read:

- prompt affect integration: shipping
- conversational/UI deepening: partial

not:

- all integration is speculative

---

## Section Outcome

| Status | Count |
|--------|-------|
| DONE | 4 |
| PARTIAL | 5 |
| FRONTIER | 3 |

The right summary for later agents is:

- emotional congruence scoring ships,
- PAD-fed prompt guidance ships,
- routing integration ships,
- coding-agent deepening remains frontier.

---

## Edit Guidance

- align Doc 09 to the real `EmotionalTag` shape
- keep Neuro retrieval and provenance work in the shipping/partial bucket
- present Doc 10 as four live integration points with uneven depth
- banner Doc 11 as frontier without hiding the prompt integration that already ships

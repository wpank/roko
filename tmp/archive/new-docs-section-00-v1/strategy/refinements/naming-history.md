# Naming History

> The narrative of how Roko's vocabulary evolved. This is where the stories go — not in the
> flat glossary, not scattered across architecture docs. If a term's origin matters, it lives here.

**Status**: Written (narrative document — no implementation status)
**Last reviewed**: 2026-04-17

---

## TL;DR

Roko went through three naming generations:

1. **The Bardo/Mori era** — a Rust multi-agent system named after Buddhist bardo (transitional
   state) and the Japanese word for death, reflecting the system's original framing around
   agent lifecycle and termination.
2. **The "one noun, six verbs" era** — the first formal architecture naming: Signal (noun),
   Substrate/Scorer/Gate/Router/Composer/Policy (verbs). Introduced the Synapse metaphor.
3. **The "two mediums, two fabrics" era** — the current vocabulary: Engram + Pulse (mediums),
   Substrate + Bus (fabrics), six operators (verbs). Retired Signal, introduced explicit
   shipping/planned distinction.

---

## Chapter 1 — Bardo/Mori → Roko

The project began as **Mori**, a Rust multi-agent coding system. The name was chosen for its
brevity and its resonance with the Japanese concept of death (森, also written 死) — a nod to
the system's ability to terminate and restart agents, and to the "mortality" framing of early
cognitive architecture experiments. Agents were mortal: they could die and be reborn with
improved state.

Mori evolved into **Bardo**, taking the Tibetan Buddhist concept of the intermediate state
between death and rebirth. The bardo framing suited a system where agents passed through
transitional states — executing, failing, consolidating, restarting — and accumulated
knowledge across lifetimes. The architecture at this stage was less structured: state
management was ad hoc, there was no formal data medium, and the "six traits" concept did
not yet exist.

Both names were retired when the project reached sufficient maturity to justify a
production-quality identity. The name **Roko** was chosen for:

- **Brevity** — two syllables, easy to say in conversation and in code (`roko plan run`)
- **Neutrality** — no loaded metaphor that would constrain future framing
- **Distinctiveness** — unique enough to be searchable and brandable
- **Phonetic appeal** — works across English, Japanese, German, and Romance languages

The transition from Bardo/Mori to Roko happened as the Synapse Architecture crystallized.
Retiring the old names was deliberate: `Bardo` and `Mori` carry conceptual baggage
(mortality, transitional states) that no longer matches how the system works. The self-hosting
loop is not about death and rebirth — it is about continuous improvement.

**Migration note**: Some internal tooling and scripts may still reference `bardo` or `mori`
in path names or variable names. These are `[retired]` and should be migrated on contact.

---

## Chapter 2 — Golem → Agent

During the Bardo era, the running process that executed tasks was called a **Golem** — a
reference to the Jewish folklore construct animated by inscription, fitting for a system where
agents were "animated" by a system prompt. The name was evocative but caused confusion:

- It implied the agent was a blind executor rather than a cognitive actor
- It made it harder to explain the system to engineers familiar with agent frameworks
- The "inscribed and animated" metaphor did not match the emergent, self-improving behavior
  the system was developing

**Agent** was adopted when the system reached maturity. It is the standard industry term, and
using it removes a barrier for new contributors. The `Golem` name is fully retired.

---

## Chapter 3 — Signal → Engram + Pulse

**Signal** was the original noun in the "one noun, six verbs" architecture. It served as the
catch-all for any piece of information the system processed — durable knowledge records,
ephemeral events, LLM outputs, tool results. All were `Signal`s.

The problem was that `Signal` was doing two fundamentally different jobs:

1. **Durable records** — content-addressed, persisted to storage, long-lived, scored, decaying
2. **Ephemeral wire events** — transient, not persisted, short-lived, for real-time coordination

These two use cases have different semantics, different consistency requirements, and different
decay behavior. Collapsing them into one type produced subtle bugs, confused documentation, and
made it impossible to reason about persistence guarantees.

The split:

- **`Engram`** (from neuroscience: the physical substrate of memory) — the durable record.
  Content-addressed via BLAKE3, 7-axis scored, four decay models, lineage DAG. The noun in
  the revised "two mediums" framing.
  
- **`Pulse`** (from neural oscillation terminology) — the ephemeral event. Transient, routed
  through `Bus`, not persisted by default. *Planned, not yet shipped.*

`Signal` is `[retired]` for both durable and ephemeral usage. In the current codebase, some
internal code paths still use `Signal` as the struct name for what is semantically an `Engram`.
These are migration targets.

---

## Chapter 4 — Grimoire → Neuro

**Grimoire** was the name for the knowledge-management cross-cut — the component responsible
for the agent's long-term memory. The name came from the magical grimoire (a book of spells),
chosen to reflect the idea of a living, growing repository of knowledge patterns.

The name was retired for practical reasons:

- It was too whimsical for a production engineering system
- New contributors did not immediately understand what it did
- It clashed with the neuroscience naming convention being adopted elsewhere

**Neuro** was chosen as the replacement — simple, descriptive, consistent with the
neuroscience-inspired vocabulary (Engram, Synapse, Gamma/Theta/Delta, HDC).

---

## Chapter 5 — Styx → Mesh

**Styx** was the planned name for the agent networking layer — the infrastructure for
multi-agent coordination and peer-to-peer Pulse routing. The name referenced the river Styx
from Greek mythology, fitting the underworld-of-agents metaphor from the Mori era.

**Mesh** replaced Styx when the Bardo/Mori metaphors were retired. The new name is
self-explanatory to any network engineer: it describes the topology (mesh networking) without
requiring mythological context.

Mesh is `[planned]` — the agent networking layer has not yet shipped.

---

## Chapter 6 — Clade → Fleet

**Clade** (from evolutionary taxonomy: a group sharing a common ancestor) was the term for the
agent roster — the set of agents in a deployment. The evolutionary framing was intentional:
agents in a clade shared lineage, could learn from each other, and adapted together.

**Fleet** replaced Clade when the architecture vocabulary shifted away from biological metaphors
toward engineering and maritime metaphors. Fleet is unambiguous, widely understood in
distributed systems contexts, and easier to explain to non-biologists.

Fleet is `[planned]` — the agent roster abstraction has not yet shipped.

---

## Chapter 7 — "One Noun, Six Verbs" → "Two Mediums, Two Fabrics"

The original architecture mnemonic was **"one noun (Signal), six verbs (Substrate, Scorer, Gate,
Router, Composer, Policy)"**. This mnemonic served its purpose during the initial design phase —
it communicated the composability principle clearly and gave every contributor a mental model
for where new functionality belonged.

When `Signal` was split into `Engram` + `Pulse`, the mnemonic broke. "One noun, six verbs"
no longer described the system accurately.

The replacement mnemonic is **"two mediums, two fabrics"**:

- **Two mediums**: `Engram` (durable) and `Pulse` (ephemeral)
- **Two fabrics**: `Substrate` (storage) and `Bus` (transport)
- **Six operators**: Substrate, Scorer, Gate, Router, Composer, Policy — unchanged

The new mnemonic captures the medium/fabric distinction that drives the most important
architectural decisions: what persists and what doesn't, what is content-addressed and what
isn't, what decays and what routes.

The retirement of "one noun, six verbs" is not a correction of an error — it was a useful
framing at a particular stage of the design. It is retired because the design outgrew it.

---

## Retired Terms Table

| Retired term | Canonical replacement | Why retired |
|---|---|---|
| `Bardo` | `Roko` | Project name replaced; Buddhist framing retired |
| `Mori` | `Roko` | Predecessor project name; fully superseded |
| `Golem` | `Agent` | Too whimsical; industry standard term adopted |
| `Signal` (durable) | `Engram` | Medium split required separate durable type |
| `Signal` (ephemeral) | `Pulse` | Medium split required separate ephemeral type |
| `Signal` (wire) | `Pulse` | Same as above; wire-event usage retired |
| `Event` (as general term) | `Pulse` | Overloaded; replaced by precise ephemeral medium |
| `Envelope` | `Pulse` | Transport wrapper concept; replaced by Pulse |
| `Message` | `Pulse` | General message term; replaced by Pulse |
| `Grimoire` | `Neuro` | Too whimsical; neuroscience naming adopted |
| `Styx` | `Mesh` | Mythology metaphor retired with Bardo era |
| `Clade` | `Fleet` | Biology metaphor retired; engineering term adopted |
| "one noun, six verbs" | "two mediums, two fabrics" | Architecture mnemonic superseded |
| `Channel` | `Topic` | Replaced by precise Pulse routing term |
| `Subject` | `Topic` | Same as Channel; replaced by Topic |

---

## See Also

- [`GLOSSARY.md`](../../GLOSSARY.md) — current canonical A-Z terms
- [`ALIASES.md`](../../ALIASES.md) — public-facing aliases ↔ canonical internal terms
- [`strategy/refinements/README.md`](README.md) — index of the refinements folder

## Open Questions

- Should a formal ADR (Architecture Decision Record) be created for each major naming
  transition? The narrative here captures the *why*, but ADRs would provide a more formal
  decision record with alternatives considered.
- The `Signal` → `Engram` migration in code is incomplete. What is the timeline for full
  codebase migration? (Tracking in implementation roadmap, not here.)

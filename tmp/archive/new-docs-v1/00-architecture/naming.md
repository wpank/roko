# Naming

> The canonical vocabulary of Roko's architecture: the current naming map, the status
> tags that distinguish shipping code from target-state design, the public-facing
> aliases, and the retired terms that must not reappear in new prose.

**Status**: Shipping
**Crate**: — (conceptual)
**Depends on**: [`vision.md`](vision.md)
**Used by**: every page in this tree, and [`../GLOSSARY.md`](../GLOSSARY.md)
**Last reviewed**: 2026-04-19

---

## What This Page Is

This is not the A-Z glossary — that is [`../GLOSSARY.md`](../GLOSSARY.md). This page
is the **naming policy**: the canonical map, the rules for using each term, the
status-tag scheme that keeps shipping code distinct from target-state design, and the
historical record of retired names.

Use this page when:

- you are writing new documentation or code and need to pick a term.
- you want to understand *why* the vocabulary looks the way it does.
- another document uses a term you do not recognize and you want to know whether
  that term is shipping, built, planned, or retired before you trust it.

For quick A-Z lookup of individual terms, use [`../GLOSSARY.md`](../GLOSSARY.md).

---

## Current Naming Map

Roko's naming mixes shipping terminology with target-state terms introduced by the
refinement series. The map below is the authoritative distinction. The `Status`
column uses the four tags defined in the [next section](#status-tags).

| Canonical term | Status | Use | Avoid |
|---|---|---|---|
| `Roko` | `[shipping]` | Project and framework name | `Bardo` / `Mori` (retired) |
| `Agent` | `[shipping]` | Running process or session | `Golem` (retired) |
| `Engram` | `[shipping]` | Durable record medium | `Signal` (retired durable term) |
| `Pulse` | `[planned]` | Target-state ephemeral transport medium | `Event`, `Envelope`, `Message`, `Signal` (retired wire terms) |
| `Substrate` | `[shipping]` | Storage fabric | legacy storage-only synonyms |
| `EventBus<E>` | `[shipping]` | Current live transport implementation | calling it retired |
| `Bus` | `[planned]` | Target-state transport fabric abstraction | presenting it as already shipped |
| `Topic` | `[planned]` | Target-state Pulse routing handle | `Channel`, `Subject` |
| `TopicFilter` | `[planned]` | Target-state subscription matcher | ad hoc routing filters |
| `Datum` | `[planned]` | Target-state polymorphic `Engram` or `Pulse` input | one-off sum types |
| `PulseSource` | `[planned]` | Target-state lightweight Pulse origin attribution | overloaded provenance terms |
| `Neuro` | `[shipping]` | Durable knowledge cross-cut | `Grimoire` (retired) |
| `Daimon` | `[built]` | Affect cross-cut; public alias `AffectBias` | old loop-step framing for affect |
| `Dreams` | `[built]` | Delta-speed consolidation cross-cut | treating Dreams as a loop step |
| `Mesh` | `[planned]` | Agent-network layer | `Styx` (retired) |
| `Fleet` | `[planned]` | Agent roster | `Clade` (retired) |
| `StateHub` | `[built]` | Current dashboard/event hub; target-state projection layer over Bus + Substrate | TUI-only framing |
| `TypedContext` | `[planned]` | Structured domain situation payload; public alias `Situation` | free-text-only context matching |
| `Calibrator` | `[planned]` | Target-state learning logic split from `Policy` | treating `Policy` as both control and learning |
| `runtime shape` | `[planned]` | Deployment form such as laptop / server / container / cluster | overloading `profile` |
| `Custody` | `[planned]` | Chain-of-custody audit record | informal approval prose |

Any term not on this map is covered in [`../GLOSSARY.md`](../GLOSSARY.md), where every
entry carries the same status tags.

---

## Status Tags

Every term carries one of four tags that distinguish what exists today from what is
still design work:

- **`[shipping]`** — Working type, module, or behaviour in the current codebase. Used
  end-to-end in the self-hosting loop. Safe to rely on.
- **`[built]`** — Code exists and has tests, but the glossary term overstates how
  fully it is wired. For example, `Attestation` is `[built]` because the Rust type
  and the sign/verify paths ship today, but the `LocalAgent` → `OrgRole` →
  `ChainWitness` level taxonomy is target-state.
- **`[planned]`** — Target-state design term. The documentation describes it, but no
  shipped type or runtime path exists yet.
- **`[retired]`** — Historical term deliberately replaced. Appears only in retirement
  notices, historical prose, and migration notes, never in new documentation as a
  live name.

The status-tag scheme exists so that documentation can talk about the target-state
architecture without drifting into fiction. A `[planned]` term is a promise; a
`[shipping]` term is a fact; a `[built]` term is something in between. Readers can
always tell which they are looking at.

---

## Public Aliases

Some internal terms are clearer for external audiences under a different name. The
internal term remains canonical in code and architecture prose. The public alias
appears in user-facing docs, CLI output, marketing, and UI. The two forms are kept
synchronised in [`../ALIASES.md`](../ALIASES.md).

The public aliases currently in use:

| Internal | Public | Where the public form appears |
|---|---|---|
| `Daimon` | `AffectBias` | End-user docs, dashboards, CLI status output |
| `c-factor` | `coordination health` | User-facing team metrics |
| `Falsifier` | `counterexample check` | End-user explanations of verification |
| `TypedContext` | `Situation` | CLI arguments and user-facing templates |
| `Demurrage` | `retention pressure` | Operator-facing memory tuning |

When writing architecture prose, use the internal term. When writing for users, use
the public alias. When you must mention both in the same document, use the internal
term first and add the public alias in parentheses.

---

## Conventions

### Capitalisation and code formatting

- Core types are TitleCase: `Engram`, `Pulse`, `Substrate`, `Bus`, `Score`, `Score`,
  `Kind`, `Body`.
- The six operator roles are TitleCase too: `Scorer`, `Gate`, `Router`, `Composer`,
  `Policy`. (They are singular nouns, not verbs — a `Scorer` scores; the scoring
  action is not itself a name.)
- The cross-cuts are TitleCase: `Neuro`, `Daimon`, `Dreams`.
- Cognitive speeds are lowercase Greek: `gamma`, `theta`, `delta`.
- Layers are `L0`–`L4` (no hyphen in prose).
- Loop steps are UPPERCASE: `SENSE`, `ASSESS`, `COMPOSE`, `ACT`, `VERIFY`, `PERSIST`,
  `BROADCAST`, `REACT`.
- Topic names are lowercase dot-separated strings in backticks, e.g.
  `` `heartbeat.gamma.tick` ``.
- Code identifiers — method names, field names, enum variants, macros — stay in
  backticks on every mention: `` `loop_tick` ``, `` `query_similar` ``, `` `claim!` ``.

### Linking

Every Rust type mentioned in prose is linked on first use per file to the page where
it is the subject. The link text is the type name itself — never `source`, `here`,
`this`, or a bare URL.

### Retired terms

Retired names may appear only in explicitly historical contexts: retirement notices,
migration notes, old-name see-also blocks. They must never appear as live names in
new prose. If you find retired vocabulary in new documentation, fix the documentation.

---

## Retired Terms

| Retired | Replaced by | Reason |
|---|---|---|
| `Bardo` / `Mori` | `Roko` | Project rename. `Bardo` carried Buddhist and theological connotations that distracted from the engineering thesis; `Mori` overlapped with an existing Rust crate. |
| `Golem` | `Agent` | `Golem` implied a singular, monolithic process. `Agent` composes with `Fleet` and `Mesh` more naturally. |
| `Grimoire` | `Neuro` | `Grimoire` misrepresented the knowledge subsystem as a static book of spells rather than a living, decaying, tier-progressing memory. |
| `Styx` (umbrella) | `Mesh` + `Korai` | The single umbrella collapsed the agent-mesh transport and the chain integration into one term. They are now distinct. |
| `Clade` | `Fleet` | `Clade` is a phylogenetic term; `Fleet` matches the deployment-scoped roster metaphor already used in ops. |
| `Signal` (as durable medium) | `Engram` | `Signal` is ambiguous — it suggests a wire-level event more than a durable record. The Rust type itself is still named `Signal` today; rename to `Engram` is Tier 0D in the implementation plan. |
| `Event` / `Envelope` / `Message` (as canonical wire type) | `Pulse` | All three were used interchangeably at different times. `Pulse` is the single canonical term for the target-state ephemeral medium. |

None of the above should appear in new prose except in this table, in
[`../GLOSSARY.md`](../GLOSSARY.md) under the retired term's entry, and in the
history sections of the pages where the rename happened.

---

## Profile vs. Domain Profile vs. Runtime Shape

The bare word `profile` was overloaded. It now splits into three terms:

- **`domain profile`** — a named bundle of tools, roles, gates, and defaults tied to a
  work domain. Coding, research, blockchain, data, ops, and writing are the current
  canonical domain bundles.
- **`runtime shape`** — a deployment form such as `laptop`, `single-server`, `container`,
  `clustered`, or `edge`. This is topology, not work domain.
- **`profile`** — reserve only for contexts where the precise split does not matter.
  When precision matters, pick one of the two above.

Using `profile` where `runtime shape` is meant (or vice versa) is a bug in the prose.

---

## How to Introduce a New Term

1. Propose the term in a refinement draft.
2. Add it to this page's naming map with the appropriate status tag and the
   "Use / Avoid" guidance.
3. Add a full entry to [`../GLOSSARY.md`](../GLOSSARY.md) pointing to its home page.
4. If the term replaces an existing one, add a retirement row to the table above and
   annotate the old term in the glossary with `[retired]`.
5. Add a `Public alias` row to [`../ALIASES.md`](../ALIASES.md) if the term needs a
   friendlier external form.

If any of those steps is skipped, the term is ambiguous and will drift.

---

## Open Questions

- Should the public aliases ever be promoted to canonical? The current bias is no —
  engineering prose benefits from distinctive names, user-facing material benefits
  from familiar ones — but this should be revisited after external beta.
- When shipping code catches up to a `[planned]` term (e.g. `Pulse` actually exists
  as a first-class Rust type), should the doc tooling auto-promote the status tag?
  Tracking in [`../_migration/section-00.md`](../_migration/section-00.md).

---

## See Also

- [`../GLOSSARY.md`](../GLOSSARY.md) — the flat A-Z lookup for every term.
- [`../ALIASES.md`](../ALIASES.md) — internal-to-public alias map.
- [`CONVENTIONS.md`](../CONVENTIONS.md) — the writing rules that enforce this vocabulary.
- [`vision.md`](vision.md) — where the core vocabulary is first introduced in context.
- [`concepts/engram.md`](concepts/engram.md) — the `Engram` (formerly `Signal`) deep dive.
- [`concepts/pulse.md`](concepts/pulse.md) — the target-state `Pulse`.
- [`concepts/bus.md`](concepts/bus.md) — the target-state `Bus` vs. shipping `EventBus<E>`.

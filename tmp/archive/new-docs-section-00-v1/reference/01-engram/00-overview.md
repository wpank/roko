# Signal — Overview

> The Signal is the single durable datum of the Roko system. Every persistent record — no exceptions — is an Signal.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [ContentHash](../10-types/content-hash/00-overview.md), [Score](../10-types/score/00-overview.md), [Decay](../10-types/decay/00-overview.md), [Provenance](../10-types/provenance/00-overview.md)  
**Used by**: every subsystem that reads or writes knowledge  
**Last reviewed**: 2026-04-19

---

## TL;DR

Roko has one durable data type: the Signal. Instead of tasks, events, messages, records,
and logs as separate types — everything is an Signal. The Signal is content-addressed
(BLAKE3), scored (7 axes), decaying (four models), lineage-tracked (audit DAG), and
semantically fingerprinted (10,240-bit HDC vector). A running agent system is a substrate
full of Signals flowing in, aging, being retrieved, producing new Signals from old ones.

---

## The Idea

Classical software architectures multiply types: tasks have one schema, events have
another, messages a third. Each new capability means a new type, a new store, a new API.
The cost is friction: components cannot easily compose because they speak different
languages.

Roko collapses this down to one type. The insight is that **all durable information in an
agent system shares the same lifecycle**: it is produced by something, has a quality, fades
over time, derives from prior information, and needs to be findable both by exact identity
and by semantic similarity. The Signal packages all of that lifecycle in one struct.

The payoffs are concrete:

1. **Universal composability.** Any Scorer can score any Signal. Any Substrate can store
   any Signal. Any Gate can verify any Signal. Components compose freely because they share
   one vocabulary.

2. **Complete audit trails.** Every Signal carries lineage — the ContentHashes of the
   Signals it derived from. This forms a directed acyclic graph (DAG). Any decision can be
   traced: "Why did the gate pass this output? Because Score X. Where did X come from?
   Signal Y. What was Y derived from? Signals A and B." Follow the DAG.

3. **Temporal dynamics.** Every Signal decays. The system's knowledge is not a static
   database but a living substrate where information has weight that evolves. Pheromone
   signals expire. Context grows stale. Relevant knowledge, when retrieved frequently, is
   reinforced. The substrate stays warm where it matters.

The **name** comes from neuroscience: an signal is the hypothetical physical trace left by
a memory in the brain (Semon 1904; Lashley 1950; Tonegawa et al. 2015). In Roko, an Signal
is the digital equivalent — a content-addressed unit of cognition that persists, decays,
and can be retrieved by exact address or by HDC similarity.

---

## What an Signal Contains

At the highest level, an Signal is a record with:

- **Identity** (`id: ContentHash`) — a BLAKE3 hash of the canonical encoding of its
  stable fields. Two Signals with the same kind, body, author, and tags have the same id.

- **Kind** (`kind: Kind`) — what category of information this is: an agent output, a gate
  verdict, a tool trace, a knowledge entry, a prediction, etc. The Kind enum tells
  operators how to interpret the Body.

- **Body** (`body: Body`) — the actual payload. An enum whose variants hold typed content
  for each Kind.

- **Score** (`score: Score`) — a 7-axis quality assessment. Not part of identity (score
  can change without changing the Signal).

- **Decay** (`decay: Decay`) — how this Signal's weight decreases over time. Four models
  are supported (Demurrage, Exponential, Step, Linear, Custom).

- **Provenance** (`provenance: Provenance`) — who produced this Signal, at what trust
  level, and any taint inherited from upstream.

- **Fingerprint** (`fingerprint: Option<HdcFingerprint>`) — a 10,240-bit
  hyperdimensional computing (HDC) vector for semantic similarity search. Optional only
  when the encoder is explicitly disabled.

- **Lineage** (`lineage: Vec<ContentHash>`) — the content hashes of the Signals this was
  derived from. Forms the edges of the audit DAG.

- **Metadata** (`tags: BTreeMap<String, String>`) — arbitrary key-value metadata, ordered
  for stable hashing.

---

## The Naming Situation

Shipping Rust code uses `Signal` for this type. `Signal` is the canonical architectural
name established in the refactor (see
[`15-rationale-and-history.md`](15-rationale-and-history.md)). The two names refer to the
exact same struct. Code will migrate to `Signal` in a subsequent refactor pass.

> **Legend for this folder:**  
> `Signal` = shipped Rust identifier for `Signal`  
> `Signal` = canonical architectural name used throughout documentation

---

## See Also

- [`01-struct-reference.md`](01-struct-reference.md) — every field with types and invariants
- [`04-kind-enum.md`](04-kind-enum.md) — all Kind variants
- [`05-body-enum.md`](05-body-enum.md) — all Body variants
- [`06-lineage-dag.md`](06-lineage-dag.md) — the audit DAG
- [`reference/02-pulse/00-overview.md`](../02-pulse/00-overview.md) — ephemeral events: the Signal's counterpart
- [`reference/10-types/score/00-overview.md`](../10-types/score/00-overview.md) — 7-axis scoring

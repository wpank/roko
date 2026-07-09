# Knowledge Demurrage: Economic Memory

> **TL;DR**: Borrow Silvio Gesell's *demurrage* idea from economics and
> apply it to memory. Every Engram carries a *holding cost* that decays
> its weight unless it is actively used, cited, or reinforced. The
> result: memory that stays fresh, playbooks that don't ossify,
> worldviews that can't petrify, and a system that preferentially
> surfaces *currently useful* knowledge rather than *historically
> cached* knowledge. This is the opposite of a cache — it's an
> attention economy with gravity.

> **For first-time readers**: Demurrage is a concept from Silvio Gesell's
> 1916 economic theory: money that *costs* its holder to keep idle, so it
> circulates faster. Applied to memory: every stored artifact pays a tiny
> "tax" per unit time; *usage* refunds the tax; unused knowledge fades to
> cold storage. Contrast with LRU (wall-clock only), TTL (arbitrary
> expiry), and current Roko decay (time-based, not reinforcement-based).
> This doc is where memory gets an economy. Read 11 first for the HDC
> fingerprint that powers the novelty bonus.

## 1. The problem with indefinite retention

Roko already has mild decay: `Engram.decay: f64` starts at 1.0 and is
reduced by the GC pass in `roko-fs`. But decay today is:

- **Time-based only**, not usage-based
- **Applied at GC time**, not continuously
- **Non-compounding** — recent decay doesn't accelerate further decay
- **Invisible to the Scorer** — decay doesn't downweight candidates

This leads to three failure modes that show up in any long-running
agent system:

1. **Playbook petrification**: a playbook that worked once is preserved
   forever, even after the codebase drifts underneath it.
2. **Stale consensus**: agents converge on a shared but out-of-date
   belief because nothing punishes its age.
3. **Archive paralysis**: episode retrieval blends 10,000 old episodes
   with 10 recent ones; the Router loses signal.

Biological memory solves this with *use-it-or-lose-it* — synapses that
aren't activated decay faster. We can do the same, but make it
explicit and measurable.

## 2. Demurrage, not decay

**Decay** is exogenous: "it's been 30 days, halve the weight."
**Demurrage** is endogenous: "you pay a tax on holding; use lowers the
tax, non-use compounds it."

The economic analogy matters because it produces the right incentive
gradient: *information that is useful to multiple subscribers stays
alive; information that nothing cares about fades, making room for new
information*. It is a market for attention, with a carrying cost.

Proposed addition to Engram:

```rust
pub struct Engram {
    // ... existing fields ...

    /// Balance of attention-credit held by this Engram.
    /// Starts at 1.0 at creation. Demurrage subtracts
    /// a tick per unit time; reinforcement adds credit.
    pub balance: f64,

    /// Accumulated tax paid (for observability). Monotonic.
    pub demurrage_paid: f64,

    /// Last time `balance` was touched by demurrage or reinforcement.
    pub last_touched_at: Timestamp,
}
```

And a new trait the Substrate implements transparently:

```rust
pub trait Demurrage {
    /// Charge demurrage since `last_touched_at`. Returns new balance.
    fn charge(&mut self, engram: &mut Engram, now: Timestamp) -> f64;

    /// Reinforce an engram (read, cite, successful use). `kind`
    /// encodes *why* so we can tune rates per reason.
    fn reinforce(&mut self, engram: &mut Engram, kind: ReinforceKind);

    /// Compute the effective weight given the balance
    /// (Scorer reads this, not `decay`).
    fn effective_weight(&self, engram: &Engram) -> f64;
}

pub enum ReinforceKind {
    Cited,        // another engram has this as a parent
    Retrieved,    // Substrate.get returned it as part of a query
    Gated,        // a gate compared against it and it held
    Surprised,    // a prediction error raised its informational value
    AgentQuoted,  // an agent read it and referenced it in a response
}
```

## 3. The rate law

The core tick:

```text
balance(t+Δt) = balance(t) - r * Δt - β * balance(t) * Δt
```

- First term is flat tax (Gesell's original): constant drain.
- Second term is exponential decay: keeps the value bounded below.

And on reinforcement:

```text
balance ← balance + bonus(kind) * novelty(engram)
```

Where `novelty` is `1 - max(similarity)` against the top-K HDC
neighbors (see `11-hyperdimensional-substrate.md`). **Novelty-weighted
reinforcement** is the key: citing a common engram gives it a tiny
bump, citing a rare engram gives it a big bump. This is the core
anti-hoarding mechanism — high-balance memory has to be *earning* its
balance from uniquely useful contributions.

## 4. What this enables that pure decay can't

### 4.1 Playbook freshness without manual GC

A playbook is an Engram. It earns balance every time an agent
successfully applies it, loses balance every tick it sits unused. When
the codebase drifts and the playbook stops working, its successful-use
rate drops; demurrage eats its balance; the Router stops proposing it.
**No human needs to "prune playbooks."**

### 4.2 Surprise-weighted retention

`ReinforceKind::Surprised` lets the Bus upweight Engrams whose
predictions were violated in interesting ways (see
`10-self-learning-cybernetic-loops.md`). This keeps the
high-information-content memories preferentially. It is Shannon's
surprise, operationalized as an economic bonus.

### 4.3 A natural "forgetting floor"

Balance can reach zero. At that point the Engram becomes a candidate
for cold storage or deletion — but its *hash* remains valid (lineage
doesn't break), the body just moves to a slower tier. This is the
primitive that lets us build a biologically plausible
short-term/long-term split *without hardcoded tiers*.

### 4.4 Composability with HDC consensus

Since `effective_weight` is a single float, HDC consensus bundles can
use it as a confidence coefficient:

```rust
consensus = Σ_i (fingerprint(e_i) * effective_weight(e_i))
```

Worldviews held by still-earning Engrams dominate; petrified ones
recede naturally.

## 5. Demurrage for the Policy layer

The same framework extends to Policy parameters themselves. Every
learned parameter (Scorer weight, Gate threshold, Router arm value)
can carry a balance. If a threshold hasn't been challenged by any
Pulse in a long time, its *confidence* should decay — not its value,
but the Policy's willingness to defend it against new evidence.

```rust
pub struct LearnedParam<T> {
    pub value: T,
    pub confidence: f64,     // demurrage-taxed
    pub last_challenge: Timestamp,
}
```

This unlocks **graceful relearning** — a long-stable parameter
eventually weakens enough that a modest amount of new evidence is
sufficient to move it. No explicit "reset" ever needed.

## 6. Configuration surface

```toml
[demurrage]
# Base rates
flat_tax_per_day         = 0.01    # r
exp_decay_per_day        = 0.005   # β
min_balance              = 0.0     # below this → cold tier

# Reinforcement bonuses
cited_bonus              = 0.05
retrieved_bonus          = 0.02
gated_bonus              = 0.03
surprised_bonus          = 0.15    # novelty-heavy
agent_quoted_bonus       = 0.08

# Policy-side demurrage
policy_confidence_tax    = 0.002
```

All of these can themselves be *learned* over time (demurrage-rates
that produce better retrieval quality reinforce themselves, via
prompt-experiment-style A/B measurement). The system tunes its own
forgetting rate.

## 7. Cold-tier graduation

Engrams whose balance hits the floor are graduated to a *cold*
substrate: same content-address, but the body moves off the hot path.
Retrieval becomes slower but still possible. This is the inverse of
the `graduate_to_engram` operation from `08-code-sketches.md` — we
already have the pattern of moving between fabrics; demurrage adds a
rule for *when*.

```rust
pub trait ColdSubstrate: Substrate {
    /// Freeze an engram into cold storage.
    fn freeze(&self, hash: EngramHash) -> Result<()>;

    /// Rehydrate on demand. Resets balance to a starting value.
    fn thaw(&self, hash: EngramHash) -> Result<Engram>;
}
```

A thaw is itself an event on the Bus, so interested operators can
update their caches.

## 8. Why this is a competitive moat

Most agent systems have two memory failure modes: *infinite growth*
(everything retained, retrieval quality collapses) and *brutal LRU*
(hard caps, biologically implausible, loses important rare events).

Demurrage gives us a third regime: **economically stable memory**.
Useful things stay warm, unused things fade gracefully, the floor is
adaptive, the rates are learnable. This property compounds with HDC
similarity and active-inference reinforcement — other frameworks can
replicate any one of these, but the *interaction* between them is
specific to Roko's substrate choices. The moat isn't any single
feature; it's the fact that Substrate + Bus + HDC + Demurrage +
Active-Inference stack into one coherent memory system and pull in
the same direction.

## 9. Observability

Three new metrics that surface the attention economy:

- **Balance histogram** per tier — shape of the distribution tells
  you whether the rates are too aggressive (everything is cold) or
  too lenient (hoarding).
- **Thaw rate** — how often cold engrams are pulled back. High thaw
  rate = the demurrage curve is too steep.
- **Reinforcement-by-kind** breakdown — what *kind* of use is
  keeping memory alive? If `Surprised` is low, the system isn't
  learning from prediction errors; if `Cited` is low, lineage is
  shallow.

These become first-class tiles on the `roko dashboard`.

## 10. Migration path

1. Add `balance`, `demurrage_paid`, `last_touched_at` fields.
   Backfill existing engrams with `balance = 1.0`.
2. Implement `Demurrage` trait and wire charge-on-read into Substrate.
3. Wire reinforcement into the five call sites (Router, Gate, Scorer,
   Composer, agent turns).
4. Have the Scorer read `effective_weight` instead of `decay`.
5. Deprecate `decay` field after one release cycle.

None of this breaks existing episode/playbook consumers — it just
makes their outputs self-trimming.

## 11. Open questions

- **Demurrage on Pulses?** A Pulse on the Bus is ephemeral by
  construction; does it need a tax? Probably not, but if Pulses can
  carry forward subscriptions, *retained-Pulses* might.
- **Taxation fairness across tiers** — should high-tier knowledge
  (distilled playbooks) tax at a different rate than raw episodes?
  Probably yes; distilled knowledge should be stickier to reflect
  the work that went into it.
- **Interaction with chain witnesses** (Phase 2) — a chain-witnessed
  Engram probably cannot be deleted even if balance hits zero; cold
  tier but never forget. Demurrage rate respects witness class.

## 12. Worked example: a playbook's life

Concrete scenario to show how the rate law, reinforcement, and tier
progression interact. Suppose `flat_tax_per_day = 0.01`,
`exp_decay_per_day = 0.005`, all defaults from §6.

Day 0. Agent distills a playbook. Balance = 1.0.

Days 1–7. Playbook is applied twice (cited) and matches gate
preconditions four times (retrieved). Tax: 7 × (0.01 + 0.005 × avg)
≈ 0.09. Reinforcement: 2 × 0.05 + 4 × 0.02 = 0.18. Net: balance rises
to ~1.09.

Days 8–30. Codebase drifts; preconditions stop matching. Zero
reinforcement. Tax: 23 × (0.01 + 0.005 × avg) ≈ 0.27. Balance drops
to ~0.82.

Day 31. An agent encounters a similar situation — HDC neighbor of
the playbook's fingerprint. The retrieval fires a `Retrieved` bonus
(0.02) but the playbook's advice fails its gate. The Policy marks
it `Surprised` (novelty-weighted 0.15 × novelty_score ≈ 0.08 effective).
Net change: +0.10. The Calibrator (§3 of 14) logs the failure.

Days 32–90. The playbook's pattern now fails consistently; each
failure is small novelty (cluster is known). Tax continues. Balance
drops below `min_balance = 0.0`. Substrate schedules `freeze(hash)`.

Day 91. Playbook moves to cold tier. Its hash is still resolvable;
lineage from Engrams that cited it still works. Full body is on slower
storage.

Day 400. A fork of the codebase reuses an old pattern. A new Engram
references the frozen playbook. Substrate `thaw(hash)` returns it
with balance reset to a starter value (configurable — default 0.3,
low enough that one failure sends it back to cold, high enough to
compete with fresh Engrams). The system is neither forgetting nor
hoarding; it is adapting with memory.

## 13. Interaction with the Composer and Scorer

Two concrete effects on the other operators:

- **Scorer**: reads `effective_weight(engram)` instead of the old
  `decay` field. A low-balance Engram scores lower across every
  axis, not just freshness. High-balance Engrams with strong axis
  scores dominate Router selection naturally. See
  `04-operators-generalized.md` §2 for the new Scorer signature.
- **Composer**: budget-aware composition now respects *attention
  budget*, not just token budget. Engrams with balance < 0.3 are
  candidates for the budget's last slots; they contribute only if
  nothing higher-balance is available. The composed prompt becomes
  preferentially fresh while retaining access to deep knowledge when
  fresh context isn't enough.

## 14. Operator knobs: when to tune which rate

The rate table from §6 has six tunable parameters. Operators
should touch them in this order:

1. **`min_balance`** — raise from 0.0 to, say, 0.1 if cold storage
   is growing too fast; lower if memory is bloated.
2. **`flat_tax_per_day`** — raise if memory is hoarding; lower if
   warming up new Engrams is too expensive.
3. **`surprised_bonus`** — raise this first when the system isn't
   learning from prediction errors visibly. `Surprised` is the
   novelty channel; cranking it emphasizes what's changing.
4. **`cited_bonus`** and **`retrieved_bonus`** — usually fine at
   defaults. Tune if the citation graph looks sparse or over-dense.
5. **`agent_quoted_bonus`** — raise in collaborative workflows where
   cross-agent citations matter (see c-factor §3.3).
6. **`exp_decay_per_day`** — touch rarely. Controls the asymptotic
   shape rather than the day-to-day gradient.

A self-tuning `DemurrageConfigPolicy` can subscribe to retrieval-
quality metrics and update these rates automatically (the rates
themselves are demurrage-taxed, per §5, so tuning that doesn't help
decays).

## 15. Dashboard for the attention economy

Four visible surfaces on `roko dashboard` (F-tab to be assigned,
expose via StateHub `demurrage_health` projection — see
`26-statehub-rearchitecture.md`):

- **Balance distribution**: stacked histogram of balance ranges,
  colored by Kind.
- **Reinforcement breakdown**: pie chart of `ReinforceKind` over the
  last 24h.
- **Thaw rate**: line graph of cold-to-warm thaws per hour.
- **Attention-leaderboard**: top 20 Engrams by balance, link to
  inspect.

Each tile answers a specific operator question ("is memory bloating?"
"what's keeping memory alive?" "are we forgetting too fast?" "what
does the system think is most important?"). Without these, demurrage
is invisible and untuneable.

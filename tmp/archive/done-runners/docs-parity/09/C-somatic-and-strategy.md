# C — Somatic and Strategy Space (Docs 06, 07, 08)

This section should also read as mostly shipping.

The live runtime already has:

- `StrategyCoordinates` and `StrategySpaceDefinition`
- `StrategySpaceComputer`
- `SomaticMarker`, `SomaticSignal`, and `SomaticLandscape`
- persisted somatic state with in-memory index rebuild
- contrarian blending in somatic retrieval

The main parity work is to stop overstating the still-frontier parts:
mind wandering, rolling-window tracking, domain-native non-coding
extractors, and benchmark-grade latency claims.

Generated: 2026-04-18

---

## Current Read

| Area | Status | Parity note |
|------|--------|-------------|
| strategy-space core | DONE | the 8D substrate is real and validated |
| somatic landscape core | DONE | markers, queries, persistence, and rebuild all ship |
| 15% contrarian retrieval | DONE | live constant and live blending behavior |
| non-coding domain extraction | PARTIAL | role-aware label projection ships; dedicated extractors do not |
| mind wandering / rolling window | FRONTIER | no shipping scheduler or tracker |
| sub-1ms latency claim | PARTIAL | plausible with `kiddo`, but not benchmarked here |

---

## C.01-C.02 — The spatial substrate is already real

The docs should say plainly that the 8D strategy surface is not just a
design sketch. It already has:

- normalized coordinate storage,
- configurable dimension labels,
- validation rules,
- a strategy computer path,
- and downstream somatic use.

That is a materially shipped substrate.

---

## C.03 — Non-coding support exists, but through a narrower fallback

**Status**: PARTIAL

The main wording fix for Doc 08 is not "non-coding strategy spaces do
not exist." They do exist, but the current implementation is narrower
than fully bespoke domain computers.

Parity stance:

- keep the role-aware label projection visible as the shipping fallback,
- state that true domain-native chain/research/trading extractors do not ship yet,
- avoid wording that suggests the fallback is absent.

This is an honest partial, not a blank area.

---

## C.04-C.08 — SomaticLandscape is a live integration surface

The audit correction matters here: `SomaticLandscape` should be treated
as shipping infrastructure.

What already ships:

- marker storage
- nearest-neighbor index
- contrarian mix
- merge/depotentiation mechanics
- persistence and reload

That makes this one of the clearer "mostly shipping" stories in topic
`09`.

---

## C.09-C.10 — Mind wandering is still frontier

**Status**: FRONTIER

Doc 07 should stop reading like the whole three-part loop-breaking stack
already exists. Today the live picture is:

- contrarian retrieval: yes
- dream/depotentiation contribution: yes
- timed mind wandering / rolling-window tracking: no

That is the exact boundary the parity pack should preserve.

---

## C.11 — Keep latency claims modest unless benchmarked

**Status**: PARTIAL

`kiddo` makes the current low-latency claim believable, but this parity
pack should not present "sub-1ms" as an enforced runtime contract unless
the docs point to benchmark evidence.

Safer wording:

- "designed for low-latency nearest-neighbor lookup"
- "expected to remain fast at realistic marker counts"

That is enough for this pass.

---

## C.12 — Plutchik cross-mapping remains explanatory only

**Status**: FRONTIER

This item belongs with A.05, not with the live strategy substrate.

If Doc 08 references Plutchik-style emotion naming, parity should keep
it clearly in the explanatory layer and away from the runtime contract.

---

## Section Outcome

| Status | Count |
|--------|-------|
| DONE | 7 |
| PARTIAL | 3 |
| FRONTIER | 2 |

The section should leave later agents with this summary:

- SomaticLandscape ships.
- Strategy-space machinery ships.
- non-coding support is narrower than the ideal, but real.
- mind wandering remains frontier.

---

## Edit Guidance

- keep the shipping somatic/strategy substrate prominent
- describe role-aware projection as the live fallback
- mark mind wandering and rolling-window tracking as frontier
- soften latency wording where the docs outrun evidence

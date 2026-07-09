# Gap Inventory — 09 Daimon

Concise audit gap list for agents working on daimon parity batches.

## Focus now

### 1. Stale `roko-golem` References Still Leak Into Active Docs — HIGH

- the active runtime is centered on `roko-core` plus `roko-daimon`
- use `roko-golem` only as historical provenance, not as a current contract

### 2. Octant / Plutchik Language Still Reads Like Live Runtime Contract — HIGH

- the shipping contract is PAD plus `BehavioralState`
- octant labels are historical or presentation-level unless code proves
  otherwise

### 3. `EmotionalTag` Schema Drift Is Still Visible — HIGH

- the shipping tag stores PAD, intensity, trigger, and mood snapshot
- `discovery_emotion` is derived provenance in Neuro, not a stored field on
  `EmotionalTag`

### 4. Behavioral-State vs. Router Hysteresis Can Be Misread — MEDIUM

- `BehavioralState::classify(...)` ships without hysteresis
- router model selection hysteresis does ship and should stay scoped to routing

### 5. Doc 11 Still Needs Explicit Frontier Tagging — MEDIUM

- per-crate confidence, error familiarity, and fatigue remain future work
- do not describe them as built just because adjacent affect plumbing exists

### 6. Doc 12 Should Stay Explicitly Frontier — MEDIUM

- no contagion runtime in `roko-daimon`
- no somatic field aggregation or C-Factor loop owned by topic `09`

## Working rule

If a daimon task requires runtime implementation instead of doc calibration,
capture the seam and defer it.

# Gap Inventory — 09 Daimon

Concise gap list for agents working on daimon parity batches.

## Focus Now

### 1. Stale `roko-golem` References Still Leak Into Active Docs — HIGH

- several docs still cite `roko-golem` as if it were an active source,
- this makes the runtime contract look more fragmented than it is.

### 2. `EmotionalTag` Schema Drift Is Under-Documented — HIGH

- Doc 09 still shows a Plutchik/emotion string field,
- the shipping struct only carries PAD, intensity, trigger, and mood snapshot.

### 3. Behavioral-State Hysteresis Is Easy To Misread — MEDIUM

- classifier hysteresis does not ship,
- router hysteresis does ship,
- the docs should separate those surfaces.

### 4. Doc 11 Overstates Frontier Coding Integration — MEDIUM

- per-crate confidence,
- error-pattern familiarity,
- fatigue detection,
- all remain design surfaces.

### 5. Doc 12 Is Fully Frontier And Should Read That Way Immediately — MEDIUM

- no contagion code,
- no somatic field,
- no C-Factor.

### 6. Doc 13 Is Good Enough To Polish, Not Rebuild — LOW

- keep it as the canonical status doc,
- improve cross-links rather than replacing it.

## Working Rule

If a daimon task requires runtime implementation instead of doc
calibration, defer it.

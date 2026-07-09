# A — Composer Core (Docs 00, 01, 06)

Parity analysis of `docs/03-composition/00-composer-trait.md`, `01-prompt-composer.md`, `06-lost-in-the-middle-u-shape.md` vs actual codebase.

---

## A.01 — Composer Trait (Doc 00)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
The `Composer` trait lives in `roko-core` (doc says `crates/roko-core/src/agent.rs`) with signature:
```rust
pub trait Composer: Send + Sync {
    fn compose(
        &self,
        engrams: &[Engram],
        budget: &Budget,
        scorer: &dyn Scorer,
        ctx: &Context,
    ) -> Result<Engram>;
}
```
Requires `Send + Sync`, synchronous (no I/O), deterministic. Takes a `&dyn Scorer` at call time (rather than consuming pre-scored engrams) so composition is always scoring-aware.

### What exists
Trait defined at `crates/roko-core/src/traits.rs:143-156` (NOT in `agent.rs` as doc claims):

```rust
pub trait Composer: Send + Sync {
    fn compose(
        &self,
        signals: &[Engram],
        budget: &Budget,
        scorer: &dyn Scorer,
        ctx: &Context,
    ) -> Result<Engram>;

    fn name(&self) -> &str;
}
```

| Aspect | Doc | Code | Match |
|--------|-----|------|-------|
| Location | `roko-core/src/agent.rs` | `roko-core/src/traits.rs:143` | FILE DIFFERS |
| Bounds | `Send + Sync` | `Send + Sync` | MATCH |
| Input param name | `engrams` | `signals` | NAME ONLY (same type) |
| Input type | `&[Engram]` | `&[Engram]` | MATCH |
| Budget type | `&Budget` | `&Budget` | MATCH |
| Scorer type | `&dyn Scorer` | `&dyn Scorer` | MATCH |
| Context type | `&Context` | `&Context` | MATCH |
| Return | `Result<Engram>` | `Result<Engram>` | MATCH |
| Extra method | — | `fn name(&self) -> &str` | ADDITION |

Doc comment at `traits.rs:137-142` matches the "assembly layer" description (prompts from sections, context packs from fragments, transactions from operations) in doc 00. The `Composer` trait is re-exported from `crates/roko-core/src/lib.rs` and imported by `roko-compose` at `prompt.rs:6-9`.

There is no `crates/roko-core/src/composer.rs` file — all trait definitions live in `traits.rs`.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| A.01.1 | Doc says trait lives in `agent.rs`; code puts it in `traits.rs` | doc 00 §1 vs traits.rs:143 | LOW (cosmetic, doc outdated) |
| A.01.2 | Doc uses param name `engrams`; code uses `signals` | traits.rs:148 | LOW (semantics identical) |
| A.01.3 | Doc omits the `name(&self) -> &str` method which code requires | traits.rs:155 | LOW (trivial addition) |

### Verify
```bash
grep -n 'pub trait Composer\|fn compose\|fn name' crates/roko-core/src/traits.rs
```

---

## A.02 — PromptSection Struct (Doc 01)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: A.01
- **Files to modify**: None

### What the doc says
`PromptSection` at `crates/roko-compose/src/prompt.rs` with 6 fields:
- `name: String`
- `content: String`
- `priority: SectionPriority`
- `cache_layer: CacheLayer`
- `placement: Placement`
- `hard_cap: Option<usize>`

Derives `Debug, Clone, Serialize, Deserialize`.

### What exists
Struct at `crates/roko-compose/src/prompt.rs:98-118` with **7 fields** (doc lists 6, code has 1 extra):

```rust
pub struct PromptSection {
    pub name: String,
    pub content: String,
    pub priority: SectionPriority,
    pub cache_layer: CacheLayer,
    pub placement: Placement,
    pub hard_cap: Option<usize>,
    #[serde(default)]
    pub bidder: AttentionBidder,
}
```

| Field | Doc | Code (line) | Match |
|-------|-----|-------------|-------|
| `name` | `String` | `String` (101) | MATCH |
| `content` | `String` | `String` (103) | MATCH |
| `priority` | `SectionPriority` | `SectionPriority` (105) | MATCH |
| `cache_layer` | `CacheLayer` | `CacheLayer` (107) | MATCH |
| `placement` | `Placement` | `Placement` (109) | MATCH |
| `hard_cap` | `Option<usize>` | `Option<usize>` (114) | MATCH |
| `bidder` | — | `AttentionBidder` (117) | ADDITION (ties to doc 10 VCG auction) |

Derives: `Clone, Debug, PartialEq, Eq, Serialize, Deserialize` (prompt.rs:98) — code adds `PartialEq, Eq` vs doc.

Builder methods: `new()` (line 123), `with_priority()` (137), `with_cache_layer()` (144), `with_placement()` (151), `with_hard_cap()` (159), `with_bidder()` (166).

Extra methods: `estimated_tokens()` (173), `enforce_hard_cap()` (182), `into_signal()` (211), `from_signal()` (228).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| A.02.1 | Code has extra `bidder: AttentionBidder` field not in doc 01 §1.1 | prompt.rs:117 | LOW (doc 01 silent; A.06 covers bidder) |

### Verify
```bash
grep -n 'pub struct PromptSection\|pub name\|pub content\|pub priority\|pub cache_layer\|pub placement\|pub hard_cap\|pub bidder' crates/roko-compose/src/prompt.rs
```

---

## A.03 — SectionPriority Enum (Doc 01)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
4-variant enum at `crates/roko-compose/src/prompt.rs`:
- `Low = 1`
- `Normal = 2`
- `High = 3`
- `Critical = 4`

Derives `Copy, PartialOrd, Ord`. Critical "never dropped, only truncated."

### What exists
Enum at `crates/roko-compose/src/prompt.rs:25-37`:

```rust
pub enum SectionPriority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
    Critical = 3,
}
```

| Variant | Doc Value | Code Value | Match |
|---------|-----------|------------|-------|
| `Low` | 1 | 0 | DIFFERS |
| `Normal` | 2 | 1 | DIFFERS |
| `High` | 3 | 2 | DIFFERS |
| `Critical` | 4 | 3 | DIFFERS |

Numeric values shifted by -1 (code is zero-indexed). The 4 variants match by name and ordering is preserved (`Low < Normal < High < Critical`).

Derives: `Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize` (line 25-26). `Normal` is `#[default]`. Uses `#[serde(rename_all = "snake_case")]`.

Critical-section guarantee enforced in composer: `prompt.rs:334-349` partitions critical from optional and returns `RokoError::BudgetExceeded` rather than silently dropping. Test `composer_never_drops_critical_sections` at line 1110 and `composer_errors_when_critical_exceeds_budget` at line 1126 cover this.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| A.03.1 | Discriminant values shifted by -1 (doc: 1-4, code: 0-3). Serialization uses names via `#[serde(rename_all = "snake_case")]` so on-wire format unaffected | prompt.rs:29-36 | LOW (cosmetic) |

### Verify
```bash
grep -n 'pub enum SectionPriority\|Low = \|Normal = \|High = \|Critical = ' crates/roko-compose/src/prompt.rs
```

---

## A.04 — CacheLayer Enum: Name Drift (Doc 01)

- **Status**: DONE (with expected name drift)
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None (doc update only, optional)

### What the doc says
4-variant enum at `crates/roko-compose/src/prompt.rs`:
- `System = 0` — "Role-level: identical across all tasks for this role"
- `Session = 1` — "Session-level: stable within a plan execution"
- `Task = 2` — "Task-level: stable within a single task's iterations"
- `Dynamic = 3` — "Dynamic: unique per request. No cache value"

Doc 06 §4 (cache alignment interaction) uses the same names (System/Session/Task/Dynamic).

### What exists
Enum at `crates/roko-compose/src/prompt.rs:43-55`:

```rust
pub enum CacheLayer {
    Role = 0,
    Workspace = 1,
    #[default]
    Plan = 2,
    Volatile = 3,
}
```

| Position | Doc Name | Code Name | Role |
|----------|----------|-----------|------|
| 0 | `System` | `Role` | Role instructions, tool defs |
| 1 | `Session` | `Workspace` | Workspace map, durable project context |
| 2 | `Task` | `Plan` | Plan/task brief |
| 3 | `Dynamic` | `Volatile` | Turn-local content (errors, feedback) |

All 4 positions match semantically but the names differ. Code's names (`Role/Workspace/Plan/Volatile`) are more specific to Roko's plan-driven model; doc's names (`System/Session/Task/Dynamic`) follow generic LLM-prompt-caching terminology.

The `cache_tag()` helper at `prompt.rs:242-249` produces the lowercase serialized strings (`"role"`, `"workspace"`, `"plan"`, `"volatile"`). Also see `#[serde(rename_all = "snake_case")]` at line 44.

Cache markers in rendered output use `roko:layer:N` (numeric, per doc 01 §2). Ordering `Role < Workspace < Plan < Volatile` is preserved by the `PartialOrd, Ord` derives and is asserted in test `cache_layer_ordering` at line 949-965.

This was the primary expected gap per the task prompt and per doc 13's "Naming Map Compliance" table (line 244-256), which does NOT flag CacheLayer as renamed even though the docs use `System/Session/Task/Dynamic` throughout.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| A.04.1 | Doc 01 §1.2 uses variant names `System/Session/Task/Dynamic`; code uses `Role/Workspace/Plan/Volatile`. Semantic positions identical, names drifted | prompt.rs:45-55 vs doc 01 §1.2 | MEDIUM (docs mislead readers) |
| A.04.2 | Doc 06 §4 "Interaction with Cache Alignment" also uses the doc names; same drift | doc 06 §4 vs prompt.rs:45 | MEDIUM (same root cause) |
| A.04.3 | Serialized tag strings are `"role"/"workspace"/"plan"/"volatile"`; doc implied `"system"/"session"/"task"/"dynamic"` | prompt.rs:242-249 | LOW (on-wire format consequence) |

### Verify
```bash
grep -n 'pub enum CacheLayer\|Role = \|Workspace = \|Plan = \|Volatile = ' crates/roko-compose/src/prompt.rs
```

---

## A.05 — Placement Enum (Doc 01, Doc 06)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
3-variant enum at `crates/roko-compose/src/prompt.rs` for U-shaped attention:
- `Start` — "Highest attention zone"
- `Middle` — "Lowest attention zone"
- `End` — "Second-highest attention zone"

Doc 06 §3.1 gives the same three variants with the same attention-zone annotations.

### What exists
Enum at `crates/roko-compose/src/prompt.rs:62-72`:

```rust
pub enum Placement {
    Start,
    #[default]
    Middle,
    End,
}
```

| Variant | Doc | Code (line) | Match |
|---------|-----|-------------|-------|
| `Start` | "Highest attention zone" | `Start` (66) with doc comment "role prompt, critical instructions" | MATCH |
| `Middle` | "Lowest attention zone" | `#[default] Middle` (68-69) with doc comment "most vulnerable to attention loss" | MATCH |
| `End` | "Second-highest" | `End` (71) with doc comment "current task, recent errors" | MATCH |

Derives: `Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize` (line 62-63). Uses `#[serde(rename_all = "snake_case")]`.

Note: `Placement` does NOT derive `PartialOrd/Ord` (unlike `SectionPriority` and `CacheLayer`) — ordering is applied via the explicit `placement_order()` helper at `prompt.rs:892-898`:

```rust
const fn placement_order(p: Placement) -> u8 {
    match p {
        Placement::Start => 0,
        Placement::Middle => 1,
        Placement::End => 2,
    }
}
```

### Gaps
None. Implementation matches spec.

### Verify
```bash
grep -n 'pub enum Placement\|Start,\|Middle,\|End,' crates/roko-compose/src/prompt.rs
```

---

## A.06 — AttentionBidder Enum (Doc 01 implicit via field, Doc 10 central)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Doc 01 does not enumerate `AttentionBidder` directly (the `bidder` field is not in doc 01's PromptSection spec). Per the task prompt, the enum should have 8 subsystems including `Neuro, Daimon, IterationMemory, ...`. This ties into doc 10's VCG attention auction.

### What exists
Enum at `crates/roko-compose/src/prompt.rs:77-95` — **8 variants**:

```rust
pub enum AttentionBidder {
    Neuro,
    Daimon,
    IterationMemory,
    CodeIntelligence,
    PlaybookRules,
    Research,
    #[default]
    TaskContext,
    Oracles,
}
```

| # | Variant | Line | Purpose |
|---|---------|------|---------|
| 1 | `Neuro` | 79 | Durable knowledge retrieved from Neuro |
| 2 | `Daimon` | 81 | Affect/somatic guidance from Daimon |
| 3 | `IterationMemory` | 83 | Recent turns, retries, prior task outputs |
| 4 | `CodeIntelligence` | 85 | Symbols, files, workspace structural context |
| 5 | `PlaybookRules` | 87 | Skills, playbooks, distilled reusable rules |
| 6 | `Research` | 89 | Research memos + external domain context |
| 7 | `TaskContext` (default) | 92 | Task brief, plan brief, verification, PRD slices |
| 8 | `Oracles` | 94 | Predictions, warnings, forecast-like outputs |

Derives: `Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize`. Uses `#[serde(rename_all = "snake_case")]`.

The `bidder_tag()` helper at `prompt.rs:251-262` produces snake-case string tags. `bidder_count()` at line 522 computes distinct-bidder diversity for emitted diagnostics.

Per-bidder affect modulation lives in `bidder_affect_multiplier()` at `prompt.rs:634-754` — each bidder has its own urgency/low-dominance/low-pleasure gain pattern. This is partial VCG-auction material (per doc 13 §4, the VCG auction is "partially implemented in the live prompt path").

### Gaps
None. Implementation matches spec with 8 distinct subsystems. Doc 01 is silent on the `bidder` field itself — see A.02.1 for that pointer.

### Verify
```bash
grep -n 'pub enum AttentionBidder' crates/roko-compose/src/prompt.rs
```

---

## A.07 — PromptComposer Struct + Assembly Algorithm (Doc 01)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: A.01-A.06
- **Files to modify**: None

### What the doc says
Doc 01 §2 specifies a 6-phase pipeline:
1. **Decode** signals into `PromptSection` structs.
2. **Partition** into Critical and Optional.
3. **Sort** optional by (CacheLayer asc, SectionPriority desc, score desc).
4. **Greedy include** under budget (Critical never dropped, optionally truncated).
5. **U-Shape ordering** by Placement.
6. **Concatenate** with section headers and `<!-- roko:layer:N -->` markers.

Doc says file is `~772 lines` with `18 tests`.

### What exists
`PromptComposer` struct at `crates/roko-compose/src/prompt.rs:285-289`:

```rust
pub struct PromptComposer {
    name: String,
    include_headers: bool,
}
```

`impl Composer for PromptComposer` at `prompt.rs:322-458`. File is **1324 lines**, **21 tests** (both exceed doc's 772/18).

Phase mapping:

| Doc Phase | Code Location | Notes |
|-----------|--------------|-------|
| 1. Decode | `prompt.rs:334-337` | `filter_map` + `PromptSection::from_signal` + `enforce_hard_cap` |
| 2. Partition | `prompt.rs:338` | `.partition(|(p, _)| p.priority == SectionPriority::Critical)` |
| 3. Sort optional | `prompt.rs:370-376` | Sorts by `bid_density` desc, then `cache_layer` asc, then priority desc |
| 4. Greedy include | `prompt.rs:378-397` via `select_optional_candidates()` at 529-585 | VCG-style auction picks winners under budget |
| 5. U-Shape ordering | `prompt.rs:400-404` | `placement_order` comparison |
| 6. Concatenate | `prompt.rs:407` via `render_sections()` at 900-917 | `--- name ---` headers per section |

**Key drift from doc spec:** The code's Phase 3 sorts primarily by `bid_density` (VCG auction score), not by the (CacheLayer, Priority) ordering doc 01 specifies. Phase 4 uses `select_optional_candidates()` which implements a full VCG-style auction with diversity boost, diminishing returns per bidder, and affect modulation (`prompt.rs:529-632`). The algorithm is richer than doc 01's greedy knapsack description.

**Cache-layer transition markers** (`<!-- roko:layer:N -->`) documented in doc 01 §2 Phase 6 are NOT emitted by `render_sections()` — it only emits `--- name ---` headers. The `roko:layer:N` markers exist in other subsystems (per doc 13 §2.1 "Cache alignment markers"), but the composer's rendered output lacks them.

**VCG diagnostics** emitted as tags on output (`prompt.rs:415-450`): `sections`, `tokens`, `distinct_bidders`, `auction_total_bid`, `auction_total_payments`, `auction_urgency`, `auction_affect_weight`, `highest_payment_section`, `highest_payment_value`. None of these are in doc 01.

**Critical-budget enforcement**: `prompt.rs:342-350` returns `RokoError::BudgetExceeded` if sum-of-critical exceeds `max_tokens`, rather than truncating. Doc 01 §2 Phase 4 says "truncate — never drop Critical" — the code's actual behavior is to **error out** when critical content alone exceeds budget (see test `composer_errors_when_critical_exceeds_budget` at line 1126). Doc and code disagree here.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| A.07.1 | Doc 01 §2 describes pure greedy knapsack sorted by (CacheLayer, Priority, score); code uses VCG auction sorted primarily by bid density | prompt.rs:370-397 vs doc 01 §2 | LOW (code is richer, doc 01 understates) |
| A.07.2 | Doc 01 §2 Phase 4 says "Critical sections truncated to fit, never dropped"; code returns `BudgetExceeded` error when critical exceeds budget | prompt.rs:342-350 vs doc 01 §2 Phase 4 | MEDIUM (documented behavior differs from actual) |
| A.07.3 | Doc 01 §2 Phase 6 specifies `<!-- roko:layer:N -->` cache-layer transition markers; `render_sections()` only emits per-section `--- name ---` headers | prompt.rs:900-917 vs doc 01 §2 Phase 6 | MEDIUM (cache-gateway assumption unmet) |
| A.07.4 | Doc claims "772 lines, 18 tests"; actual is 1324 lines, 21 tests | prompt.rs vs doc 01 header | LOW (doc stale after VCG expansion) |

### Verify
```bash
grep -n 'pub struct PromptComposer\|impl Composer for PromptComposer\|fn compose\|fn render_sections' crates/roko-compose/src/prompt.rs
grep -c '#\[test\]' crates/roko-compose/src/prompt.rs
wc -l crates/roko-compose/src/prompt.rs
```

---

## A.08 — estimate_tokens() 4-Bytes-Per-Token (Doc 01)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Doc 01 §3:
```rust
fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}
```
Byte-based heuristic (~4 bytes per token), calibrated against cl100k_base. Accuracy within ±15%. Takes <1μs.

### What exists
Free function at `crates/roko-compose/src/prompt.rs:17-20`:

```rust
#[must_use]
pub const fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}
```

| Aspect | Doc | Code | Match |
|--------|-----|------|-------|
| Signature | `fn estimate_tokens(text: &str) -> usize` | `pub const fn estimate_tokens(text: &str) -> usize` | ENHANCED (`const fn` + `#[must_use]`) |
| Math | `text.len() / 4` (truncating div) | `text.len().div_ceil(4)` (ceiling div) | DIFFERS (conservative rounding up) |
| Visibility | unspecified | `pub` | MATCH (exported) |

The `div_ceil` rounds up (e.g. "abcde" = 5 bytes → 2 tokens, not 1), making estimates more conservative than the doc. Test `estimate_tokens_rounds_up_four_byte_chunks` at `prompt.rs:1306-1311` asserts: `""` → 0, `"a"` → 1, `"abcd"` → 1, `"abcde"` → 2.

`PromptSection::estimated_tokens()` at line 173 delegates to this free function.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| A.08.1 | Code uses ceiling division (`div_ceil(4)`); doc shows truncating division (`/ 4`) | prompt.rs:19 vs doc 01 §3 | LOW (code is safer — over-estimates tokens by at most 1) |

### Verify
```bash
grep -n 'pub const fn estimate_tokens\|div_ceil' crates/roko-compose/src/prompt.rs
```

---

## A.09 — Lost-in-the-Middle U-Shape Implementation (Doc 06)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: A.05, A.07
- **Files to modify**: None

### What the doc says
Doc 06 §3.3 specifies that after budget fitting, included sections are reordered:
```
final_order = [
    sections.filter(placement == Start),   // highest attention zone
    sections.filter(placement == Middle),  // lowest attention zone
    sections.filter(placement == End),     // second-highest attention zone
]
```
Within each placement group, the `CacheLayer` ordering is preserved for cache stability.

### What exists
Implementation at `crates/roko-compose/src/prompt.rs:400-404`:

```rust
// Order by placement for final output (U-shaped).
kept.sort_by(|a, b| {
    placement_order(a.0.placement)
        .cmp(&placement_order(b.0.placement))
        .then_with(|| a.0.cache_layer.cmp(&b.0.cache_layer))
});
```

`placement_order()` at `prompt.rs:892-898` maps `Start=0, Middle=1, End=2`. Secondary sort on `cache_layer` preserves cache stability within each placement group, exactly per doc 06 §3.3 "Within each placement group, the CacheLayer ordering is preserved".

Test coverage: `composer_places_sections_in_u_shape` at `prompt.rs:1147-1178` — feeds sections in scrambled order and asserts `start_pos < middle_pos < end_pos` in the rendered output.

### Status against the full doc 06
Implemented (matches code):

| Doc 06 §§ | Feature | Status |
|-----------|---------|--------|
| §3.1 | Placement enum Start/Middle/End | Implemented (see A.05) |
| §3.2 | Section-to-Placement mapping | Applied at caller sites via `with_placement()` |
| §3.3 | U-shape ordering in PromptComposer | Implemented (prompt.rs:400-404) |
| §4 | Cache-layer tiebreak preserved within placement groups | Implemented (prompt.rs:403) |

Designed / not implemented (matches doc 06's own "Current Status and Gaps" §12):

| Doc 06 §§ | Feature | Status |
|-----------|---------|--------|
| §6.1 | `PositionAttentionModel` (exponential primacy/recency curve) | NOT IMPLEMENTED |
| §6.2 | Position-optimal section assignment (interleaving algorithm) | NOT IMPLEMENTED |
| §6.3 | `placement_adjusted_score()` helper | NOT IMPLEMENTED |
| §7 | Empirical validation plan + `ModelAttentionCurves` | NOT IMPLEMENTED |
| §9.3 | Dynamic placement from LongLLMLingua-style density ranking | NOT IMPLEMENTED |

These are all flagged as "Designed" or "Not yet" in doc 06 §12, so the doc is accurate on its own limitations.

### Gaps
None for the core U-shape ordering — implementation matches spec. Gaps A.09.1 and A.09.2 below are for doc-06-proposed extensions, intentionally filed here so the item totals stay complete:

| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| A.09.1 | `PositionAttentionModel` (doc 06 §6.1) not implemented. Current placement is a 3-bucket discretization; doc 06 proposes a continuous attention-multiplier curve | no code | LOW (doc 06 §12 flags as "Designed") |
| A.09.2 | `dynamic_placement()` (doc 06 §9.3) not implemented. Placement is assigned statically per section at caller sites, not dynamically from query-relative information density | no code | LOW (doc 06 §12 flags as "Designed") |

### Verify
```bash
grep -n 'placement_order\|U-shaped\|placement for final' crates/roko-compose/src/prompt.rs
```

---

## A.10 — PromptBuild Fluent Builder (Doc 01)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Doc 01 §4 specifies `PromptBuild` as composition metadata with 5 fields:
```rust
pub struct PromptBuild {
    pub estimated_tokens: usize,
    pub sections_included: usize,
    pub sections_dropped: usize,
    pub dropped_names: Vec<String>,
    pub tokens_per_layer: HashMap<CacheLayer, usize>,
}
```
Tracks cost prediction, dropped section names, and per-layer token distribution.

### What exists
Struct at `crates/roko-compose/src/prompt.rs:827-843` with **7 fields** — different schema from doc:

```rust
pub struct PromptBuild {
    pub prompt: String,
    pub context_strategy: ContextStrategy,
    pub cache_hit: bool,
    pub playbook_hits: usize,
    pub tokens: usize,
    pub sections_kept: usize,
    pub sections_dropped: usize,
}
```

Field-by-field comparison:

| Doc Field | Code Field | Match |
|-----------|-----------|-------|
| `estimated_tokens` | `tokens` | RENAMED |
| `sections_included` | `sections_kept` | RENAMED |
| `sections_dropped` | `sections_dropped` | MATCH |
| `dropped_names: Vec<String>` | — | MISSING |
| `tokens_per_layer: HashMap<CacheLayer, usize>` | — | MISSING |
| — | `prompt: String` | ADDITION (stores the assembled text) |
| — | `context_strategy: ContextStrategy` | ADDITION (Full/Trimmed/Retry/Minimal) |
| — | `cache_hit: bool` | ADDITION |
| — | `playbook_hits: usize` | ADDITION |

Code comment at `prompt.rs:824` says "Matches Mori's `PromptBuild` in `apps/mori/src/orchestrator/prompts/assembly.rs`" — the code is aligned with the Mori observability struct, not doc 01 §4's cost-prediction struct.

Fluent builder at `prompt.rs:845-890`:
- `new(prompt)` at 848 — seeds `tokens` via `estimate_tokens(&prompt)`
- `with_strategy()` at 864
- `with_cache_hit()` at 871
- `with_playbook_hits()` at 878
- `with_section_counts(kept, dropped)` at 885

Test `prompt_build_records_metadata` at `prompt.rs:1291-1303` covers all 4 setters.

`ContextStrategy` enum at `prompt.rs:807-819` (4 variants: `Full`, `Trimmed`, `Retry`, `Minimal`) is tangential to PromptBuild — not mentioned in doc 01.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| A.10.1 | `dropped_names: Vec<String>` field in doc 01 §4 not implemented (debugging info for dropped sections) | prompt.rs:828 vs doc 01 §4 | MEDIUM (debuggability loss for budget-pressured runs) |
| A.10.2 | `tokens_per_layer: HashMap<CacheLayer, usize>` not implemented (cache-analysis telemetry) | prompt.rs:828 vs doc 01 §4 | MEDIUM (cache analysis harder without it) |
| A.10.3 | Field names renamed: doc `estimated_tokens`/`sections_included` vs code `tokens`/`sections_kept` | prompt.rs:838-840 | LOW (cosmetic) |
| A.10.4 | Code has 4 additional fields (`prompt`, `context_strategy`, `cache_hit`, `playbook_hits`) inherited from Mori's PromptBuild — doc 01 §4 silent | prompt.rs:830-836 | LOW (doc outdated vs Mori parity) |
| A.10.5 | `PromptBuild` is NOT produced by `PromptComposer::compose()` itself; it must be constructed separately by callers. VCG diagnostics are emitted as tags on the output Engram (prompt.rs:415-450) instead | prompt.rs vs doc 01 | MEDIUM (composer output carries metadata via tags, not PromptBuild) |

### Verify
```bash
grep -n 'pub struct PromptBuild\|pub fn new\|with_strategy\|with_cache_hit\|with_playbook_hits\|with_section_counts' crates/roko-compose/src/prompt.rs
```

---

## Section Summary

| Item | Doc | Status | Parity |
|------|-----|--------|--------|
| A.01 | Composer Trait | DONE | 100% — trait + signature match, minor file-location drift (traits.rs vs agent.rs) |
| A.02 | PromptSection Struct | DONE | 115% — all 6 doc fields + extra `bidder` field |
| A.03 | SectionPriority Enum | DONE | 100% — 4 variants, discriminants shifted but serialization unaffected |
| A.04 | CacheLayer Enum | DONE | 100% semantic — 4 positions match; name drift Role/Workspace/Plan/Volatile vs doc's System/Session/Task/Dynamic |
| A.05 | Placement Enum | DONE | 100% — exact match |
| A.06 | AttentionBidder Enum | DONE | 100% — all 8 subsystems present |
| A.07 | PromptComposer + Assembly | DONE | 115% — VCG auction exceeds doc's greedy spec; 2 behavior deltas (critical-overflow errors instead of truncating; cache markers not emitted) |
| A.08 | estimate_tokens() | DONE | 100% — `const fn` with ceiling division (safer than doc) |
| A.09 | Lost-in-the-Middle U-Shape | DONE | 100% — core ordering implemented; §6 position-attention model and §9.3 dynamic placement remain designed-only (doc 06 §12 self-flags) |
| A.10 | PromptBuild Fluent Builder | DONE | 60% — struct exists with builder, but schema diverges from doc 01 §4: missing `dropped_names` + `tokens_per_layer`, added Mori-parity fields |

### Priority actions

1. **P2 (doc update)** (A.04): Rename doc 01 §1.2 + doc 06 §4 CacheLayer variants from `System/Session/Task/Dynamic` to `Role/Workspace/Plan/Volatile` (code is authoritative per user's naming policy; code names are also more specific to Roko's model).
2. **P2 (code addition)** (A.10.1, A.10.2): Extend `PromptBuild` with `dropped_names: Vec<String>` and `tokens_per_layer: HashMap<CacheLayer, usize>` — these debuggability fields are specified in doc 01 §4 and currently must be reconstructed from output Engram tags.
3. **P2 (behavior reconciliation)** (A.07.2): Decide whether `PromptComposer` should truncate or error on critical-overflow. Doc 01 §2 Phase 4 says truncate; code errors. Either update the doc or add a truncation fallback path.
4. **P3 (cache markers)** (A.07.3): Emit `<!-- roko:layer:N -->` transition markers in `render_sections()` so the inference gateway can place `cache_control` breakpoints per doc 01 §2 Phase 6.
5. **P3 (doc refresh)** (A.07.4): Update doc 01's "772 lines, 18 tests" header to the current "1324 lines, 21 tests" and add a §2 note describing the VCG auction that supersedes the greedy-knapsack description.

---

## Agent Execution Notes

### A.10 — PromptBuild Observability

Treat this as runtime debuggability, not just struct-shape parity.

Recommended slice:

1. add the missing dropped-section visibility,
2. add per-layer token accounting if it can be computed cheaply,
3. make sure at least one live consumer or test exercises the richer metadata.

Acceptance criteria:

- dropped sections are inspectable,
- layer/token accounting is available where practical,
- the patch does not fork prompt metadata into yet another parallel struct.

### A.07 / A.09 — Core Composer

These are already real. Batch `03` should only touch them when budget policy or observability requires it.

Do not use batch `03` to start building continuous attention-curve or dynamic-placement research systems.

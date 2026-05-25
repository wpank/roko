# M096 — Knowledge Kind Mapping to Engram Kind

## Objective
Map the existing `KnowledgeKind` enum variants in `roko-neuro` to the unified Engram `Kind` system in `roko-core`, and replace the separate `KnowledgeTier` retention model with demurrage-driven tier multipliers. This eliminates the parallel type hierarchy and makes knowledge entries first-class Engrams whose lifespan is governed by the same decay/demurrage mechanics as every other Engram in the system.

## Scope
- Crates: `roko-neuro`, `roko-core`
- Files: `crates/roko-neuro/src/lib.rs` (KnowledgeKind, KnowledgeTier, KnowledgeEntry), `crates/roko-core/src/kind.rs` (Kind enum), `crates/roko-core/src/lib.rs`
- Phase ref: depth doc 11-memory/01-knowledge-as-signal.md
- Depth doc: `tmp/unified-depth/11-memory/01-knowledge-as-signal.md`

## Steps
1. Discover actual type locations and variants:
   ```bash
   grep -rn 'pub enum KnowledgeKind' crates/roko-neuro/src/ --include='*.rs'
   grep -A 20 'pub enum KnowledgeKind' crates/roko-neuro/src/lib.rs
   grep -rn 'pub enum Kind' crates/roko-core/src/kind.rs
   grep -A 15 'pub enum KnowledgeTier' crates/roko-neuro/src/lib.rs
   ```

2. **Current `KnowledgeKind` variants** (in `crates/roko-neuro/src/lib.rs`):
   ```rust
   pub enum KnowledgeKind {
       Insight,          // compact causal observation
       Heuristic,        // lightweight rule of thumb
       AntiKnowledge,    // negative knowledge (what to avoid)
       Warning,          // cautionary warning about failure modes
       CausalLink,       // causal relationship between observations
       StrategyFragment, // reusable approach fragment
   }
   ```

3. **Current `KnowledgeTier` variants** (in `crates/roko-neuro/src/lib.rs`):
   ```rust
   pub enum KnowledgeTier {
       Transient,     // short-lived, aggressive decay
       Working,       // active working memory
       Consolidated,  // validated, base decay rate
       Persistent,    // highly durable, slow decay
   }
   ```

4. **Current `Kind` variants in `roko-core`** (in `crates/roko-core/src/kind.rs`):
   The `Kind` enum already has `Insight` (in the `// ─── Chain participation` section, ~line 93) and `Pheromone`, but is missing variants for the other 5 `KnowledgeKind` types. It has a `Custom(String)` escape hatch.
   The `// ─── Memory` section currently has: `Episode`, `PlaybookRule`, `Skill`, `Compound(Vec<Kind>)`.
   **Important**: `Kind::Insight` is NOT in the Memory section -- it is in Chain participation. The new knowledge variants go into the Memory section, BEFORE `Compound`.

5. In `crates/roko-core/src/kind.rs`, add missing knowledge-related Kind variants to the `// ─── Memory` section, BEFORE the `Compound(Vec<Kind>)` variant:
   ```rust
   // ─── Memory ──────────────────────────────────────────────────────────
   /// A logged episode of an agent run.
   Episode,
   /// A playbook rule extracted from patterns.
   PlaybookRule,
   /// A learned skill (reusable procedure).
   Skill,
   /// A lightweight rule of thumb or learned tendency (from KnowledgeKind::Heuristic).
   Heuristic,
   /// Negative knowledge describing what to avoid (from KnowledgeKind::AntiKnowledge).
   AntiKnowledge,
   /// A cautionary warning about a recurring failure mode (from KnowledgeKind::Warning).
   Warning,
   /// A causal relationship between two observations (from KnowledgeKind::CausalLink).
   CausalLink,
   /// A reusable approach fragment composable into larger plans (from KnowledgeKind::StrategyFragment).
   StrategyFragment,
   ```
   Also add matching arms to `Kind::as_str()`:
   ```rust
   Self::Heuristic => "heuristic",
   Self::AntiKnowledge => "anti_knowledge",
   Self::Warning => "warning",
   Self::CausalLink => "causal_link",
   Self::StrategyFragment => "strategy_fragment",
   ```

6. In `crates/roko-neuro/src/lib.rs`, add a mapping function:
   ```rust
   impl KnowledgeKind {
       /// Convert to the unified Engram Kind.
       pub fn to_engram_kind(&self) -> roko_core::Kind {
           match self {
               Self::Insight => roko_core::Kind::Insight,
               Self::Heuristic => roko_core::Kind::Heuristic,
               Self::AntiKnowledge => roko_core::Kind::AntiKnowledge,
               Self::Warning => roko_core::Kind::Warning,
               Self::CausalLink => roko_core::Kind::CausalLink,
               Self::StrategyFragment => roko_core::Kind::StrategyFragment,
           }
       }

       /// Parse from a unified Engram Kind, returning None for non-knowledge kinds.
       pub fn from_engram_kind(kind: &roko_core::Kind) -> Option<Self> {
           match kind {
               roko_core::Kind::Insight => Some(Self::Insight),
               roko_core::Kind::Heuristic => Some(Self::Heuristic),
               roko_core::Kind::AntiKnowledge => Some(Self::AntiKnowledge),
               roko_core::Kind::Warning => Some(Self::Warning),
               roko_core::Kind::CausalLink => Some(Self::CausalLink),
               roko_core::Kind::StrategyFragment => Some(Self::StrategyFragment),
               _ => None,
           }
       }
   }
   ```

7. Add tier-to-demurrage multiplier constants to `crates/roko-neuro/src/lib.rs`.
   **Important**: `KnowledgeTier` already has a `pub const fn multiplier(&self) -> f32` method (returns f32 lifetime multipliers). The new method MUST be named `demurrage_multiplier` (returns f64) to avoid collision:
   ```rust
   /// Demurrage rate multipliers per knowledge tier.
   /// Lower multiplier = slower decay = longer retention.
   pub const TIER_DEMURRAGE_MULTIPLIERS: &[(KnowledgeTier, f64)] = &[
       (KnowledgeTier::Transient, 1.0),      // full demurrage rate
       (KnowledgeTier::Working, 0.5),         // 2x slower decay
       (KnowledgeTier::Consolidated, 0.1),    // 10x slower decay
       (KnowledgeTier::Persistent, 0.01),     // 100x slower decay
   ];

   impl KnowledgeTier {
       /// Return the demurrage multiplier for this tier.
       pub fn demurrage_multiplier(&self) -> f64 {
           match self {
               Self::Transient => 1.0,
               Self::Working => 0.5,
               Self::Consolidated => 0.1,
               Self::Persistent => 0.01,
           }
       }
   }
   ```

8. Write unit tests:
   - Round-trip `KnowledgeKind` -> `Kind` -> `KnowledgeKind` mapping covers all 6 variants
   - Tier multiplier lookup returns correct values for all 4 tiers
   - `from_engram_kind` returns `None` for non-knowledge Kinds
   - No existing tests break

## Verification
```bash
cargo check -p roko-neuro -p roko-core
cargo clippy -p roko-neuro -p roko-core --no-deps -- -D warnings
cargo test -p roko-neuro --lib
cargo test -p roko-core --lib
```

## What NOT to do
- Do NOT remove `KnowledgeKind` or `KnowledgeTier` enums -- they remain as convenience types with conversion methods
- Do NOT modify `KnowledgeStore` persistence format -- that is M097
- Do NOT implement the full demurrage balance tracking -- that relies on M028-M030 (Engram demurrage fields)
- Do NOT change any enum variant names in `Kind` that already exist
- Do NOT rename the existing `Kind::Insight` variant

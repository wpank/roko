# Symbol Resolution and Anti-Pattern Detection

> **Depth file for [06-COMPOSITION.md](../../06-COMPOSITION.md)**
> Source: `crates/roko-compose/src/symbol_resolver.rs` -- workspace Rust symbol resolution

---

## Overview

The SymbolResolver resolves symbol names to their definitions in the workspace,
enabling precise context injection into agent prompts. Rather than having agents
spend tokens discovering type signatures, function parameters, and trait bounds
through file reads, the resolver pre-computes exact definitions and injects them
into the task context layer (Layer 4). Anti-pattern detection sources failure
patterns from three channels -- episode history, anti-knowledge entries, and
gate failure patterns -- and places them in the recency attention zone
(Placement::End) to exploit the U-shaped attention curve.

---

## 1. The SymbolResolver

### 1.1 Architecture

The resolver collects all `.rs` files under `crates/`, skipping `target/`,
and performs line-by-line pattern matching to extract symbol definitions:

```rust
/// Source: crates/roko-compose/src/symbol_resolver.rs
pub struct SymbolResolver {
    workdir: PathBuf,
}

impl SymbolResolver {
    /// Resolve symbols by name.
    /// Accepts bare names ("TaskDef"), qualified paths
    /// ("SystemPromptBuilder::new"), or module-qualified paths
    /// ("task_parser::TaskDef").
    pub fn resolve_symbols(&self, names: &[String]) -> Vec<ResolvedSymbol>;
}
```

### 1.2 ResolvedSymbol

```rust
pub struct ResolvedSymbol {
    pub name: String,
    pub file: String,       // Relative to workdir
    pub line: usize,
    pub signature: String,  // Extracted definition line(s)
    pub kind: SymbolKind,
}

pub enum SymbolKind {
    Struct,
    Enum,
    Trait,
    Fn,
    TypeAlias,
    Const,
    Impl,
    Unknown,
}
```

### 1.3 Resolution Strategy

The resolver uses a multi-pass approach:

1. **Exact match**: Search for `pub struct TaskDef`, `pub fn compose`, etc.
   Matches the full qualified name if provided.

2. **Module-qualified match**: For `task_parser::TaskDef`, search files in
   paths containing `task_parser` (either as a module file or directory).

3. **Impl block match**: For `SystemPromptBuilder::new`, find the `impl`
   block for `SystemPromptBuilder` and search within it for `fn new`.

4. **Disambiguation**: If multiple matches exist, prefer:
   - `pub` items over private items
   - Items in the same crate as the task's target files
   - Items with more specific module paths

### 1.4 Pattern Matching

The resolver uses line-by-line regex matching rather than a full AST parser
for two reasons:

1. **Speed**: Regex matching over source files is 10-100x faster than
   running tree-sitter or syn for the simple pattern matching needed.
   The resolver runs at task startup and must complete in < 10ms.

2. **Robustness**: The resolver does not need to handle every Rust syntax
   edge case -- it needs to find `pub struct`, `pub fn`, `pub trait`,
   `pub enum`, `pub type`, and `pub const` declarations. This is a small
   subset of Rust syntax that regex handles reliably.

Patterns recognized:

```
pub struct NAME             -> SymbolKind::Struct
pub enum NAME               -> SymbolKind::Enum
pub trait NAME              -> SymbolKind::Trait
pub fn NAME                 -> SymbolKind::Fn
pub(crate) fn NAME          -> SymbolKind::Fn
pub type NAME               -> SymbolKind::TypeAlias
pub const NAME              -> SymbolKind::Const
impl NAME                   -> SymbolKind::Impl
impl<...> NAME              -> SymbolKind::Impl
impl NAME for TARGET        -> SymbolKind::Impl
```

### 1.5 Injection into Prompts

Resolved symbols are injected into the task context layer (Layer 4) as a
dedicated section:

```
## Resolved Symbols

The following type signatures are relevant to your task:

### SystemPromptBuilder (crates/roko-compose/src/system_prompt_builder.rs:42)
```rust
pub struct SystemPromptBuilder {
    role_identity: String,
    conventions: Option<String>,
    domain: Option<String>,
    // ...
}
```

### Compose (crates/roko-core/src/traits.rs:118)
```rust
pub trait Compose: Send + Sync {
    fn compose(&self, signals: &[Signal], budget: &Budget,
               scorer: &dyn Score, ctx: &Context) -> Result<Signal>;
}
```
```

This eliminates a common failure mode: agents guessing at type signatures,
function parameters, or trait bounds. With resolved symbols, the agent has
exact definitions in its context without needing to spend tokens reading files
to discover them.

---

## 2. Anti-Pattern Detection

### 2.1 Three Source Channels

Anti-patterns (Layer 7 of the SystemPromptBuilder) are sourced from three
channels:

**Channel 1: Episode History.** Common mistakes extracted from past gate
failures. When a gate fails, the failure pattern is recorded in the episode
log with the task category, crate, and error type. Patterns that recur across
multiple episodes are promoted to explicit anti-patterns.

```
Example: "In roko-compose, do not use HashMap for tool definition serialization.
Use BTreeMap for deterministic ordering. This causes cache misses when HashMap
randomizes key order."
```

**Channel 2: Anti-Knowledge Entries.** Explicitly recorded "things that are
wrong" in the durable knowledge store. These are curated facts that counter
incorrect assumptions or common misconceptions.

```
Example: "The backward-compat alias is `pub type Engram = Signal`, not the other
way around. Do not create a new type called Engram -- Signal is the canonical
struct name; Engram is just a re-export alias."
```

**Channel 3: Gate Failure Patterns.** Recurring error patterns from similar
tasks, extracted from gate output analysis. Unlike episode-derived anti-patterns
(which are about what to avoid), gate failure patterns are about what specific
errors look like.

```
Example: "Clippy warning E0599: no method named `compose` found for struct
`PromptComposer` in the current scope. Check that you imported the Compose
trait: `use roko_core::traits::Compose;`"
```

### 2.2 Placement Strategy

Anti-patterns are placed at `Placement::End` (recency zone) to exploit the
U-shaped attention curve documented by Liu et al. (2023, arXiv:2307.03172).
The model's last impression before generating is "don't make these mistakes."

This placement is empirically validated: moving anti-patterns from Middle to
End position improves their effectiveness by approximately 30% (the attention
degradation factor for mid-context information). The Devin framework (2025)
independently arrived at the same dual-position pattern for critical
constraints.

### 2.3 Priority and Budget

Anti-patterns receive `SectionPriority::High` (not Critical). They are
important but can be dropped if the budget is extremely tight (Trivial tasks
with Surgical tier). Critical priority is reserved for role identity, safety
constraints, and task description -- sections that the agent cannot function
without.

The typical anti-pattern budget is:
- **Implementation tasks**: 4% of total budget (~480 tokens at 12K)
- **Strategy tasks**: 10% of total budget (~2400 tokens at 24K)
- **Review tasks**: 7% of total budget (~840 tokens at 12K)

Strategists get the largest anti-pattern budget because strategic errors are
more costly -- a wrong decomposition wastes entire downstream task chains.

---

## 3. Common Anti-Patterns in This Codebase

The following anti-patterns are frequently surfaced by the detection system
for the roko workspace:

### 3.1 Reimplementation Anti-Pattern

**Pattern:** Building something that already exists in the codebase.

**Detection:** When a task description mentions creating a new struct or
function, the SymbolResolver checks if a similar name already exists. If it
finds a match, the anti-pattern is injected:

```
ANTI-PATTERN: A struct/function with a similar name already exists.
Check `crates/<crate>/src/<file>.rs` line <N> before creating a new one.
This codebase has duplicate implementations from parallel development.
```

This anti-pattern is the most frequently triggered in the roko workspace
(from CLAUDE.md critical rule #1: "NEVER reimplement what already exists").

### 3.2 Build-Without-Wiring Anti-Pattern

**Pattern:** Creating new code that is never called from the runtime.

**Detection:** When a task produces new public functions or structs, the
post-gate analysis checks if any existing code references them. If the new
code has zero callers, the anti-pattern is recorded:

```
ANTI-PATTERN: New code was created but never wired into the runtime.
If your change isn't visible via `cargo run -p roko-cli -- <subcommand>`,
it's probably wrong. Check that something actually calls your new code.
```

This is CLAUDE.md critical rule #2: "WIRE, don't build."

### 3.3 Stale Import Anti-Pattern

**Pattern:** Using imports from a module that has been reorganized.

**Detection:** Gate failures with E0432 (unresolved import) or E0433
(unresolved module) errors trigger this pattern:

```
ANTI-PATTERN: Import path has changed. Check the current module structure.
Common: `use roko_core::agent::Composer` is now `use roko_core::traits::Compose`.
```

### 3.4 HashMap Ordering Anti-Pattern

**Pattern:** Using HashMap where BTreeMap is required for deterministic
ordering (cache alignment, serialization stability).

**Detection:** Clippy or test failures related to non-deterministic ordering
trigger this pattern:

```
ANTI-PATTERN: Do not use HashMap for content that must be deterministically
ordered. Use BTreeMap. HashMap randomizes key ordering between runs, which
defeats prefix caching and produces non-reproducible prompts.
```

### 3.5 Unwrap-in-Library Anti-Pattern

**Pattern:** Using `.unwrap()` in library crate code where errors should be
propagated.

**Detection:** Clippy warnings or gate failures from panics in library code:

```
ANTI-PATTERN: Never use .unwrap() in library crates (anything under crates/).
Use .map_err()?, .ok_or()?, or .unwrap_or_default() instead.
Unwrap panics crash the plan runner and lose all execution state.
```

---

## 4. Anti-Pattern Lifecycle

### 4.1 Discovery

Anti-patterns are discovered through three mechanisms:

1. **Automatic extraction**: The gate failure analyzer scans error output for
   recurring patterns. Errors that appear in 3+ episodes for the same task
   category are promoted to anti-patterns.

2. **Manual curation**: Developers add anti-knowledge entries to the knowledge
   store via `roko knowledge` commands.

3. **Playbook distillation**: The learning system's playbook rules (when/then
   patterns) can be projected as anti-patterns by inverting the "then" clause:
   "when X, then do Y" becomes "when X, do NOT do Z (the common mistake that
   Y corrects)."

### 4.2 Relevance Matching

Not all anti-patterns apply to all tasks. The anti-pattern injector filters by:

- **Crate**: Anti-patterns are tagged with the crate they apply to. A
  roko-compose anti-pattern is not injected for a roko-gate task.
- **Task category**: Implementation anti-patterns are not injected for
  documentation tasks.
- **File overlap**: Anti-patterns mentioning specific files are prioritized
  when the task touches those files.

### 4.3 Decay and Retirement

Anti-patterns have a useful lifetime. An anti-pattern that has not been
triggered (i.e., no gate failure matching its pattern) for 30 days is
demoted to Low priority. After 90 days without a trigger, it is archived.

This prevents the anti-pattern list from growing unboundedly. Old patterns
that no longer apply (because the underlying code was refactored) naturally
age out rather than cluttering the prompt.

---

## 5. Symbol Resolution + Anti-Pattern Synergy

The SymbolResolver and anti-pattern detection work together to prevent
a specific failure mode: the agent creates a new type because it does not
know the existing type exists, and the new type conflicts with or duplicates
the existing one.

The workflow:

1. Task description mentions "implement a prompt budget system."
2. SymbolResolver finds `PromptBudget` in `crates/roko-compose/src/templates/common.rs`.
3. Anti-pattern detection injects: "PromptBudget already exists. Do not create
   a new budget type. Extend or modify the existing one."
4. The agent receives both the exact type signature (from resolution) and
   the warning (from anti-pattern detection) in its context.

Without this synergy, the agent might create a new `BudgetConfig` type that
duplicates `PromptBudget`, adding to the codebase's existing problem of
parallel implementations.

---

## 6. Current Status and Gaps

| Aspect | Status |
|--------|--------|
| SymbolResolver | **Implemented** (7 SymbolKind variants) |
| ResolvedSymbol struct | **Implemented** |
| Regex-based pattern matching | **Implemented** |
| Symbol injection into Layer 4 | **Implemented** |
| Anti-pattern sourcing (3 channels) | **Implemented** |
| Anti-pattern Placement::End | **Implemented** |
| Anti-pattern relevance matching | **Implemented** (crate + category) |
| Anti-pattern decay/retirement | **Implemented** (configurable age thresholds) |
| Reimplementation detection | **Implemented** (SymbolResolver cross-check) |
| Tree-sitter AST resolution | **Not yet** (regex is sufficient for current needs) |
| Cross-crate symbol graph | **Not yet** (would enable dependency-aware injection) |

---

## Cross-References

| File | Relationship |
|------|-------------|
| `system-prompt-builder-9-layer.md` | Layer 7: Anti-Patterns |
| `lost-in-the-middle-u-shape.md` | Placement::End rationale |
| `role-templates-11.md` | Per-role anti-pattern budgets |
| `enrichment-pipeline-13-step.md` | Pre-computed artifacts that include symbol refs |
| [06-COMPOSITION.md](../../06-COMPOSITION.md) ss 10 | Parent chapter symbol resolution |

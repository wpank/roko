# Naming Changes: Complete Migration Map

Source of truth: `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/01-naming-and-glossary.md`

## Framework Names

| Old | New | Where Used |
|-----|-----|-----------|
| Bardo | Roko | Overall framework |
| Mori | Roko Orchestrator | Build/coding orchestration |
| Golem / Golems | Agent / Agents | Autonomous entities |
| Grimoire | Neuro / `roko-neuro` | Knowledge subsystem |
| Styx | Agent Mesh / Mesh | P2P relay + permissioned subnets |
| Clade | Collective / Mesh | Groups of cooperating agents |
| GNOS (token) | KORAI (mainnet) / DAEJI (testnet) | Tokens |
| golem.toml | roko.toml | Config file |

## Forbidden Terms

**NEVER use these in any context:**
- "Fleet" for agent groups → use "Collective" or "Mesh"
- "Golem SDK" → use "Agent SDK" or "Roko SDK"
- "Natural death" / "death protocol" / Thanatopsis
- Bloodstain, Katabasis, Necrocracy
- "Three mortality clocks"
- "Vitality gauge (Thriving → Terminal)"
- Succession / generational knowledge transfer via death
- Stochastic death / Weibull clock

## Crate Renames

### Must Rename (breaking, blocks everything)

| Current Directory | Target Directory | Impact |
|-------------------|------------------|--------|
| `crates/bardo-runtime/` | `crates/roko-runtime/` | 4 Cargo.toml, ~10 import stmts |
| `crates/bardo-primitives/` | `crates/roko-primitives/` | 6 Cargo.toml, ~18 import stmts |

### Files to Update for `bardo-runtime` → `roko-runtime`

```
Cargo.toml (workspace members)
crates/roko-cli/Cargo.toml
crates/roko-core/Cargo.toml
crates/roko-serve/Cargo.toml
apps/mirage-rs/Cargo.toml
+ all .rs files with `use bardo_runtime::`
```

### Files to Update for `bardo-primitives` → `roko-primitives`

```
Cargo.toml (workspace members)
crates/roko-core/Cargo.toml
crates/roko-compose/Cargo.toml
crates/roko-dreams/Cargo.toml
crates/roko-fs/Cargo.toml
crates/roko-learn/Cargo.toml
crates/roko-neuro/Cargo.toml
crates/roko-serve/Cargo.toml
+ all .rs files with `use bardo_primitives::`
```

## Crate Dissolution: `roko-golem`

roko-golem should be completely removed. Its subsystems are redistributed:

| Subsystem | Source | Target | Action |
|-----------|--------|--------|--------|
| Daimon | `roko-golem/daimon.rs` | `roko-daimon` (exists) | Delete placeholder |
| Dreams | `roko-golem/dreams.rs` | `roko-dreams` (exists) | Move, remove re-export |
| Grimoire | `roko-golem/grimoire.rs` | `roko-neuro` (exists) | Delete placeholder |
| Hypnagogia | `roko-golem/hypnagogia.rs` | `roko-dreams` | Move code |
| Chain Witness | `roko-golem/chain_witness.rs` | `roko-chain` | Move code |
| **Mortality** | `roko-golem/mortality.rs` | **DELETE** | Concept removed |
| **ScaffoldEngine** | `roko-golem/lib.rs` | **DELETE** | No umbrella trait |
| **GolemScaffold** | `roko-golem/lib.rs` | **DELETE** | Composition at app layer |

### Crates that depend on roko-golem (must update):

```
roko-learn:   roko-golem = { path = "../roko-golem" }
roko-dreams:  roko-golem = { path = "../roko-golem", features = ["scaffold"] }
roko-serve:   roko-golem = { path = "../roko-golem", features = ["scaffold"] }
```

## Type Rename: Signal → Engram (Tier 0D — completed)

| Current | Target | Location |
|---------|--------|----------|
| `pub struct Signal` | `pub struct Engram` | `roko-core/src/engram.rs` |
| `SignalBuilder` | `EngramBuilder` | `roko-core/src/engram.rs` |
| `signal.rs` | `engram.rs` | File rename |
| `Kind::Signal` | Keep or rename | Enum variant |
| All `use roko_core::Signal` | `use roko_core::Engram` | Completed across Rust crates/apps |

### Impact scope for Signal → Engram

The Rust workspace has been updated. Remaining textual mentions of "Signal" are
documentation / UI wording / domain-specific names and should be handled only
when they are intended to change semantically.

Historic search command:
```bash
grep -rn 'Signal' crates/ --include='*.rs' | grep -v target/ \
  | grep -v '// ' | grep -v 'signal_' \
  | cut -d: -f1 | sort -u
```

Search patterns for ripple effects:
- `Signal::` (method calls)
- `impl Signal` (impl blocks)
- `for Signal` (trait impls)
- `#[derive(...Signal...)]` (derives)
- `$crate::Signal` (macro-generated names)

## Workspace Metadata

| File | Field | Current | Target |
|------|-------|---------|--------|
| `Cargo.toml:82` | authors | `Roko <engineering@bardo.run>` | TBD (roko.dev?) |
| `Cargo.toml:83` | repository | `https://github.com/wpank/bardo` | TBD |
| `Cargo.toml:84` | homepage | `https://github.com/wpank/bardo` | TBD |
| `roko-serve/Cargo.toml` | authors | Same old | Same fix |

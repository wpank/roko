# roko-index + lang crates — Code Intelligence

> Status-quo audit · verified 2026-07-08 · git HEAD `5852c93c05` on `main` · sources: 7 roko-index src files (~4.6K LOC: graph 1400, workspace 1916, sqlite 500, symbol 215, parser 142, hdc 355, lib 47), 4 lang-crate files (~2.9K LOC: rust lib 905 + tree_sitter_parser 485, ts 938, go 673), roko-cli (main.rs, commands/util.rs, orchestrate.rs, prompt_helpers.rs, dispatch_helpers.rs), roko-mcp-code/src/lib.rs (1930 LOC), roko-primitives/src/hdc.rs, roko-graph/src/engine.rs, 12 v1 design docs (`docs/v1/15-code-intelligence/`), 6 v2 depth docs (`docs/v2-depth/22-code-intelligence/`), workspace + crate Cargo.tomls, `.roko/GAPS.md`
>
> **Re-verify (2026-07-08):** every code claim below re-checked against current source at HEAD `5852c93c05`. Line numbers current. Two headline findings confirmed at the byte level: **(a) the HDC type collision** — roko-index has NO `roko-primitives` dependency (`crates/roko-index/Cargo.toml` deps: roko-core + 3 lang crates only) and re-implements the entire 10,240-bit vector core with an **incompatible** `HdcFingerprint` struct; **(b) the language crates are real heuristic parsers, not stubs** — but tree-sitter is Rust-only and compiled by nobody.

## Summary

The index engine is **real and wired**: `WorkspaceIndex` (`crates/roko-index/src/workspace.rs:414`) parses Rust/TS/Go, builds a 5-edge-kind symbol graph, runs genuine PageRank, computes 10,240-bit HDC fingerprints, and answers keyword/structural/HDC/hybrid-RRF searches behind a `CodeIndex` trait (workspace.rs:351). It is consumed in three places: the `roko index build/rebuild/search/stats` CLI (`roko-cli/src/commands/util.rs:1251`), per-task dispatch enrichment in orchestrate.rs (code symbols injected into the 9-layer system prompt, orchestrate.rs:16136–16159, plus gate-side hints at 18543), and the roko-mcp-code MCP server (separate audit; it holds an `Arc<WorkspaceIndex>` loaded once at startup, roko-mcp-code/src/lib.rs:189).

The big divergences from design: **(1) no persistence** — the SQLite/FTS5 backend (sqlite.rs) and rkyv graph snapshots (graph.rs:190–270) are feature-gated and **no consumer enables either feature**; every CLI invocation, MCP server start, and orchestrator cache refresh re-parses the whole workspace from scratch (orchestrate.rs caches it for only 60 s, line 6208). `roko index rebuild` even deletes a `.roko/index.db` that nothing ever writes (util.rs:1273). **(2) Heuristic parsing everywhere** — tree-sitter exists only for Rust, behind an off-by-default feature no one compiles (roko-lang-rust/Cargo.toml:15); TS and Go are line-regex only. **(3) Own HDC implementation duplicating roko-primitives** (identical 10,240-bit / splitmix64 design, zero code sharing). **(4) Architecture is pre-v2 bespoke** — the v2 "cell pipeline" (Parse/Graph/Score/Fingerprint/Search/Assemble Cells, file-watch Feed, symbols-as-Signals, gate-feedback into PageRank) exists only on paper; roko-graph registers a `NoopCell` named "ScoreCell" (engine.rs:344). Both design status docs (v1 `10-current-status-and-gaps.md`, v2 `01-…-cell-pipeline.md` "Current State") are **stale** — they claim no CLI/no search/no MCP/no compose integration, all of which now exist. `.roko/GAPS.md` contains no code-intelligence entries.

## ⚠️ HDC type collision (wave-1 P0 finding — VERIFIED byte-identical)

This is the single most consequential drift in the navigation layer and it **blocks v2 cross-domain resonance** (symbols cannot resonate against Signals/Engrams/knowledge vectors because they live in a different, non-interoperable type).

**The collision, verified at HEAD `5852c93c05`:**

| Aspect | `roko-index/src/hdc.rs` | `roko-primitives/src/hdc.rs` | Match? |
|---|---|---|---|
| Bit width | 10,240 (`WORDS=160`, hdc.rs:12) | 10,240 (`HDC_BITS`, hdc.rs:8) | ✅ identical |
| Backing store | `[u64; 160]` (hdc.rs:101) | `[u64; 160]` (hdc.rs:31) | ✅ identical |
| splitmix64 add const | `0x9E37_79B9_7F4A_7C15` (hdc.rs:20) | `0x9E37_79B9_7F4A_7C15` (hdc.rs:13) | ✅ identical |
| splitmix64 mul consts | `0xBF58_476D…`, `0x94D0_49BB…` (hdc.rs:22–23) | same (hdc.rs:15–16) | ✅ identical |
| Ops | bundle / bind / hamming | bundle / bind / hamming | ✅ same algebra |
| **Public type** | `HdcFingerprint { bits: [u64;160] }` — **NOT** `Copy`, **NOT** `Serialize`, **no** rkyv | `HdcVector { bits: [u64;160] }` — `Copy`, custom `Serialize`/`Deserialize` (1280-byte packing), rkyv-gated, UUID-addressable | ❌ **structurally incompatible** |
| Crate dependency | roko-index has **zero** roko-primitives dep | — | ❌ no code sharing |

So the two implementations are **behaviourally equivalent but type-incompatible**: a `roko-index::HdcFingerprint` cannot be passed anywhere a `roko-primitives::HdcVector` is expected (and vice-versa) without a manual `words()`-to-`from_words` bit copy, and only `HdcVector` can be persisted/serialized. Every symbol/file fingerprint (`fingerprint_symbol`, `fingerprint_file`, hdc.rs:173/187) is therefore an island: it is never written to disk (no serde) and never compared against any Signal/knowledge vector. This is a textbook violation of the CLAUDE.md "NEVER reimplement what already exists" rule.

**Why it matters for v2:** the v2 architecture wants one HDC algebra shared across domains (symbols, Signals, Engrams, knowledge, daimon affect) so that "resonance" queries can cross domains. Today code-intelligence is walled off. Unifying on `roko-primitives::HdcVector` is the enabling P1 for that resonance and also unlocks fingerprint persistence (HdcVector already serializes) — see roadmap R2.

### Two HDC impls side-by-side — layout identical, API divergent (second-pass, byte-verified)
Both files re-read at HEAD 5852c93c05. The *algebra* is copy-pasted (same splitmix64 consts, same `[u64;160]`); the *type surface* is where they fork — and the fork is what makes fingerprints an un-persistable island.

| Surface | `roko-index/src/hdc.rs` | `roko-primitives/src/hdc.rs` |
|---|---|---|
| Public type | `HdcFingerprint { bits: [u64;160] }` (hdc.rs:99-102) | `HdcVector { bits: [u64;160] }` (hdc.rs:30-32) |
| Derives | `Clone, Debug, PartialEq, Eq` — **not `Copy`** (hdc.rs:99) | `Clone, Copy, Debug, Eq, PartialEq` (hdc.rs:25) |
| serde | **none** — cannot serialize | hand-written `Serialize`/`Deserialize`, 1280-byte LE pack (hdc.rs:34-81) |
| rkyv | none | `#[cfg_attr(feature="rkyv", …)]` (hdc.rs:26-29) |
| Addressability | none | UUID-seeded `random()` (hdc.rs:92-109) |
| `bind` | free fn `bind(&a,&b)` (hdc.rs:75) | method `self.bind(&other)` (hdc.rs:113) |
| `bundle` | free fn, majority-vote (hdc.rs:51) | method (same algorithm) |
| Similarity | `HdcFingerprint::similarity` normalized Hamming (hdc.rs:114) | Hamming-based on `HdcVector` |
| Domain API on top | `role_vector`(kind), `encode_name`(trigrams), `fingerprint_symbol`, `fingerprint_file` (hdc.rs:130-206) | generic vector algebra only — no code-domain layer |
| splitmix64 add const | `0x9E37_79B9_7F4A_7C15` (hdc.rs:20) | `0x9E37_79B9_7F4A_7C15` (hdc.rs:13) — **identical** |
| Crate dep on the other | **none** (roko-index Cargo.toml: roko-core + 3 lang crates) | n/a |

**Net:** an `HdcFingerprint` can only reach an `HdcVector` via a manual `words()`→`from_words` copy, and only the primitives type serializes. `WorkspaceIndex` stores `HashMap<SymbolId, hdc::HdcFingerprint>` (workspace.rs:422-423) — computed at build, held in RAM, **never written** (no serde) and **never compared to any Signal/knowledge `HdcVector`**. The island.

### Index build pipeline trace (parse → graph → rank → fingerprint)
```
WorkspaceIndex::load(root)                                    workspace.rs:435
  ├─ canonicalize(root)                                        :436  (guards MCP path-escape too)
  ├─ collect_source_files(root)                                :438  walk, ext-filter rs/ts/tsx/js/jsx/go
  └─ from_source_files_with_root(root, files)                  :439
       ├─ parse_source per file → LanguageProvider             parser.rs:28; provider by ext workspace.rs:22-24,1377
       │     RUST_PROVIDER = heuristic RustLanguageProvider    workspace.rs:22  (tree-sitter NEVER selected)
       ├─ index symbols_by_{name,id}, functions_by_name        workspace.rs:419-421
       ├─ build_graph(): 7 phases, name-match only             graph.rs:15-20,279-560
       │     Imports · Calls(regex \b\w+\s*\() · TypeRef(PascalCase) · Contains · Implements
       ├─ pagerank(): power iteration d=0.85, 30 iters         graph.rs:589  → pagerank_scores  workspace.rs:424
       │     weighted_pagerank / personalized_pagerank EXIST but 0 callers (graph.rs:641,709)
       └─ fingerprint pass                                     workspace.rs:1034,1058
             per symbol: bind(role_vector(kind), bundle(encode_name, ctx))   hdc.rs:173
             per file:   bundle(symbol fps) | seed(content)                  hdc.rs:187
             ⇒ symbol_fingerprints / file_fingerprints  (RAM only, no serde) workspace.rs:422-423
Consumers: CLI util.rs:1257/1281/1302/1366 · MCP Arc-once lib.rs:189 · dispatch 60s cache orchestrate.rs:6208
Search reachable from CLI: keyword | structural | hybrid  (hdc:None hardcoded util.rs:1338)
Search reachable from MCP: + hdc + embedding(→keyword fallback workspace.rs:583-598)
```

### The exact 3-step HDC unification
```
STEP 1  add dep         roko-index/Cargo.toml += roko-primitives = { path = "../roko-primitives" }
        verify          cargo build -p roko-index
STEP 2  newtype         roko-index/src/hdc.rs:  pub struct HdcFingerprint(roko_primitives::HdcVector);
        (keep role_vector/encode_name/fingerprint_symbol/fingerprint_file API on top;
         delete splitmix64/bundle/bind/hamming — call HdcVector methods)
        verify          grep -rn 'splitmix64' crates/roko-index/ | wc -l  → 0 ;  cargo test -p roko-index
STEP 3  persist         HdcVector now serializes ⇒ store per-symbol/file vectors in SqliteIndex;
                        enable `sqlite` feature in roko-cli + roko-mcp-code so build writes .roko/index.db
        verify          cargo run -p roko-cli -- index build && ls .roko/index.db
```
This is the single highest-value fix: it kills the "NEVER reimplement" violation, unlocks fingerprint persistence (ends the O(full-reparse) tax), and enables cross-domain resonance (R9) in one dependency edge.

## Pipeline census

| Stage | Real? | How | Evidence |
|---|---|---|---|
| Parse | ✅ (heuristic) | `parse_source` delegates to `LanguageProvider` (roko-core trait); static providers selected by extension (rs / ts,tsx,js,jsx / go) | parser.rs:28; workspace.rs:22–24, 1377–1385 |
| Tree-sitter parse | 🔌 (Rust only, never compiled) | `TreeSitterRustProvider` behind `tree-sitter` feature; no workspace member enables it; `WorkspaceIndex` hard-codes heuristic `RustLanguageProvider` | roko-lang-rust/Cargo.toml:14–20; tree_sitter_parser.rs:18; workspace.rs:22 |
| Symbols | ✅ | `SymbolId` = (file, name, kind); `SymbolRef`; `find_symbol` | symbol.rs:25, 95 |
| Graph | ✅ (heuristic edges) | 7-phase `build_graph`: Imports (import-path last-segment ↔ name), Calls (regex `\b\w+\s*\(` over body line-ranges), TypeRef (PascalCase regex), Contains (line-order), Implements ("Trait for Type" name split). Name-matching only, no type resolution | graph.rs:15–20, 279–560 |
| Rank | ✅ | Textbook PageRank (power iteration, d=0.85, 30 iters at build); `weighted_pagerank` + `personalized_pagerank` also implemented **but have zero runtime callers** (🔌) | graph.rs:589, 641, 709; workspace.rs:1018 |
| Fingerprint | ✅ (own impl) | 10,240-bit HDC: FNV-1a seed → splitmix64 vectors, trigram name encoding, bundle/bind, Hamming similarity; per-symbol + per-file fingerprints computed at index build | hdc.rs:12–206; workspace.rs:1034, 1058 |
| Search | ✅ | keyword, structural (kind/vis/glob/callers/min-pagerank), HDC, embedding (**fallback to keyword** — no embeddings computed, workspace.rs:583–598), hybrid via RRF k=60 | workspace.rs:453–634, 1435 |
| Context assembly | ✅ built / 🔌 partially used | `context_for_query`: keyword+HDC-semantic candidates → graph expansion → `CodeSlice` extraction → token budget → overlay/privacy redaction. Used by MCP `get_context` only; dispatch path injects **symbol listings, not slices** | workspace.rs:929–1009; prompt_helpers.rs:280–305 |
| Persistence | ❌ at runtime | `SqliteIndex` (WAL, FTS5, mtime-based incremental) fully coded but `sqlite` feature enabled by nobody; rkyv snapshots ditto; no `.roko/index.db` exists on disk | sqlite.rs:31–349; roko-cli/Cargo.toml:47; roko-mcp-code/Cargo.toml:20 |

## Lang parity matrix

| Capability | roko-lang-rust | roko-lang-typescript | roko-lang-go |
|---|---|---|---|
| LOC / tests | 906 + 486 ts-parser / 46 | 939 / 33 | 577 / 25 |
| Parsing | line heuristic (lib.rs:96) | line heuristic (lib.rs:179) | line heuristic (lib.rs:74) |
| Tree-sitter | 🟡 optional feature, off by default (Cargo.toml:15) | ❌ none | ❌ none |
| Imports | `use` (incl. brace expansion + alias), `mod`, `extern crate` | ES import (default/named/star/side-effect/type), CommonJS `require` | single + grouped, alias / dot / underscore |
| Symbols | fn (async/unsafe/const/extern), struct, enum, trait, impl (+`Trait for Type`), const, type, mod | function, class→Struct, interface→Trait, type, const, enum, export default | func, type struct→Struct, type interface→Trait, const, var, grouped const/var blocks |
| Visibility | `pub(...)` parsing | `export`/`declare` | capitalization (top-level col-0 only, lib.rs:215) |
| BuildSystem | `CargoBuildSystem` | `Npm`/`Pnpm`/`Yarn` | `GoBuildSystem` |
| Known blind spots | nested fns, multi-line sigs, macros | arrow fns captured as Const not Function; no methods inside classes | methods skipped when indented? (col-0 rule); no Enum kind |

## Status matrix (subsystem → tag)

| Subsystem | Tag | One-line |
|---|---|---|
| `roko index build/rebuild/search/stats` CLI | **Wired** | `WorkspaceIndex::load` per invocation (util.rs:1257/1281/1302/1366); real symbol/edge stats printed |
| WorkspaceIndex engine (parse→graph→rank→fingerprint→search) | **Wired** | Built in one shot; consumed by CLI, MCP, and dispatch enrichment |
| roko-lang-rust provider (heuristic) | **Wired** | 905 LOC real parser; `RUST_PROVIDER` hardcoded in workspace.rs:22 |
| roko-lang-typescript provider (heuristic) | **Wired** | 938 LOC real parser (ES + CommonJS imports; class→Struct, interface→Trait) |
| roko-lang-go provider (heuristic) | **Wired** | 673 LOC real parser (single/grouped imports, func/type/const/var) — **not a stub** |
| Tree-sitter (Rust) | **Stub/Partial** | 485-LOC provider exists but `tree-sitter` feature enabled by **no crate**; never compiled (Cargo grep: only roko-lang-rust declares it) |
| Tree-sitter (TS / Go) | **Missing** | no tree-sitter deps at all |
| HDC fingerprints | **Wired-but-Drifted** | Works; byte-identical duplicate of roko-primitives; no serde, never persisted |
| roko-mcp-code MCP server | **Wired** | 13 tools over one `Arc`-free `WorkspaceIndex` loaded once (lib.rs:196); exposes hdc+embedding strategies (lib.rs:236) |
| Dispatch enrichment (symbols → system prompt) | **Wired** | orchestrate.rs:16136–16159; 60 s index cache |
| SQLite/FTS5 persistence | **Stub** | fully coded (sqlite.rs) but `sqlite` feature off everywhere; no `.roko/index.db` ever written |
| rkyv graph snapshots | **Stub** | edges-only serialize behind off feature |
| CLI HDC/embedding search | **Missing** | `roko index search` hardcodes `hdc: None` (util.rs:1338); only keyword/structural/hybrid reachable |
| v2 cell pipeline / file-watch Feed / symbols-as-Signals / gate-feedback | **Missing** | paper-only; roko-graph registers `NoopCell` named "ScoreCell" |
| Dense embeddings (fastembed) | **Missing** | `EmbeddingQuery` type only; `embedding_search` silently keyword-falls-back (workspace.rs:587–594) |

## Drift ledger (code ≠ design, or code ≠ code)

1. **[P0] HDC type collision** — roko-index reimplements roko-primitives' 10,240-bit vector with an incompatible, non-serializable `HdcFingerprint`; blocks cross-domain resonance (see dedicated section). Evidence: byte-identical splitmix consts; roko-index Cargo.toml has no roko-primitives dep.
2. **[P1] Persistence dead at runtime** — `sqlite`/`rkyv` features enabled by nobody; every CLI run, MCP boot, and 60 s dispatch cache refresh re-parses the whole workspace. `roko index rebuild` deletes a `.roko/index.db` nothing writes (util.rs:1273).
3. **[P1] Tree-sitter never compiled** — Rust provider exists (tree_sitter_parser.rs) but no crate enables the feature; `WorkspaceIndex` hardcodes the heuristic `RustLanguageProvider`. So all Calls/TypeRef edges are regex, not AST.
4. **[P1] Duplicated dispatch helper** — `code_context_for_task`/`extract_task_keywords` live in dispatch_helpers.rs (used) AND prompt_helpers.rs (dead copy, zero callers).
5. **[P2] CLI cannot reach HDC/embedding/semantic search** — `hdc: None` hardcoded; MCP can, CLI can't. Capability asymmetry.
6. **[P2] `semantic_search` panics on internal invariant** — `.expect("symbol fingerprint missing")` / `"file fingerprint missing"` (workspace.rs:740/744) instead of graceful skip.
7. **[P2] `SqliteIndex::incremental_update` change-detection is a stub** — hashes the **path string** with blake3, not file content (sqlite.rs); combined with mtime it never detects content changes correctly.
8. **[P2] MCP staleness** — index loaded once at startup, no refresh/notify; long-lived servers go stale. Per-request `lookup_symbol_details` re-parses files (lib.rs:979).
9. **[P3] Stale design docs** — v1/15/10 and v2/22/01 "Current State" predate the CodeIndex trait, search API, CLI, SQLite module, MCP server, and compose wiring — all now exist. `.roko/GAPS.md` has zero code-intelligence entries.
10. **[P3] v2 architecture unbuilt** — monolithic `WorkspaceIndex` where v2 wants a Graph-of-Cells; no Feed/Bus/Signal integration; `personalized_pagerank`/`weighted_pagerank` exported but uncalled.

## Roadmap — unify HDC with roko-primitives (ordered)

- **R1 [P0, isolated]** Add `roko-primitives = { path = ... }` to roko-index Cargo.toml. Verify: `cargo build -p roko-index`.
- **R2 [P0, depends R1]** Replace `roko-index/src/hdc.rs` internals with `roko_primitives::HdcVector`; keep `HdcFingerprint` as a thin newtype wrapping `HdcVector` (preserving the symbol-specific `role_vector`/`encode_name`/`fingerprint_symbol` API on top). Delete the duplicated splitmix/bundle/bind/hamming. Verify: `grep -rn 'splitmix64' crates/roko-index/ | wc -l` → 0; `cargo test -p roko-index`.
- **R3 [P0, depends R2]** Now that fingerprints serialize (HdcVector has serde), persist them: extend SqliteIndex to store per-symbol/per-file vectors, and enable the `sqlite` feature in roko-cli + roko-mcp-code so `roko index build` writes `.roko/index.db` and loads-from-db when fresh. Verify: `cargo run -p roko-cli -- index build && ls .roko/index.db`.
- **R4 [P1, independent]** Fix `incremental_update` to blake3-hash file **content** (fallback mtime). Verify: `cargo test -p roko-index --features sqlite`.
- **R5 [P1, independent]** Enable `tree-sitter` for roko-lang-rust in roko-index; switch `RUST_PROVIDER` to `TreeSitterRustProvider` with heuristic fallback. Verify: symbol count rises on `roko index stats`.
- **R6 [P1, independent]** Delete dead `code_context_for_task`/`extract_task_keywords` copy in prompt_helpers.rs. Verify: `cargo clippy --workspace --no-deps -- -D warnings`.
- **R7 [P2]** Expose `--strategy hdc|semantic|embedding` in `roko index search` (drop the `hdc: None` hardcode); make `semantic_search` skip-not-panic. Verify: `cargo run -p roko-cli -- index search "dependency graph" --strategy hdc`.
- **R8 [P2]** MCP index refresh (notify watcher or mtime check). Verify: touch a file mid-session, `get_index_stats` reflects it.
- **R9 [P3]** Once HDC is unified (R2), wire symbol fingerprints into cross-domain resonance (Signals/knowledge query the same `HdcVector` space). This is the v2 payoff the collision currently blocks.
- **R10 [P3]** Tree-sitter TS/Go; v2 cell pipeline; symbols-as-Signals + gate-feedback into PageRank; refresh stale design docs; log gaps in `.roko/GAPS.md`.

## Current state table

| Component | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| Tree-sitter parsing | v1/15/01; v2/22/02 | `roko-lang-rust/src/tree_sitter_parser.rs` | 🔌 Rust-only, feature never enabled; TS/Go ❌ | Cargo.toml:15; no `features = ["tree-sitter"]` anywhere |
| Symbol extraction | v1/15/02 | 3 lang crates + parser.rs | ✅ (heuristic) | workspace.rs:1377 |
| Dependency graph | v1/15/03 | graph.rs `build_graph` | ✅ 5 edge kinds (design's "Imports only" is stale) | graph.rs:31–42 |
| PageRank importance | v1/15/04; v2/22/02 | graph.rs:589 | ✅ plain; 🔌 weighted/personalized unused at runtime | workspace.rs:1018 only calls `pagerank` |
| HDC fingerprints | v1/15/05; v2/22/03 | hdc.rs | ✅ works; 🕰️ duplicate of `roko-primitives::hdc` (same 10,240 bits, same splitmix64 consts); roko-index has no roko-primitives dep | hdc.rs:19 vs roko-primitives/src/hdc.rs:12 |
| Search (multi-strategy + RRF) | v2/22/04 | workspace.rs:604–634 | ✅ in-memory; FTS5/BM25 + camelCase tokenizer ❌ unreachable (feature off; default tokenizer, no identifier splitting) | sqlite.rs:103–113 |
| Context assembly | v1/15/06; v2/22/04 | workspace.rs:929 (`assemble_context`) | ✅ built; 🔌 dispatch injects symbol lists only, slices reachable only via MCP `get_context` | orchestrate.rs:16136; prompt_helpers.rs:301 |
| Dispatch enrichment wiring | v2/22/01 (feeds Compose) | `code_context_for_task` → `build_system_prompt_with_context_validated(..., code_ctx, ...)`; verify-path `code_intel_hints`; 60 s index cache | ✅ wired | orchestrate.rs:16136–16159, 18543–18546, 6208–6231, 2793 |
| CLI `roko index` | v1/15/10 Phase 1 | `IndexCmd::{Build,Rebuild,Search,Stats}` | ✅ (in-memory each run; search exposes keyword/structural/hybrid only, no HDC) | main.rs:1220–1256; util.rs:1311–1341 (`hdc: None`) |
| MCP context server | v1/15/07; v2/22/05 | `roko-mcp-code` (13 tools; embedding→HDC downgrade) | ✅ boundary: consumes `CodeIndex` API; separate audit | roko-mcp-code/src/lib.rs:189, 1601–1608 |
| Index DB scaling (SQLite) | v1/15/08; v2/22/05 | sqlite.rs | 🟡 coded + tested, feature off everywhere; incremental update is mtime-only — `blake3` hashes the **path string, not content** | sqlite.rs:244–330, 319 |
| Snapshot optimization (rkyv) | v1/15/09; v2/22/05 | graph.rs:190–270 | 🟡 graph-edges-only snapshot, feature off; no full-index snapshot (fingerprints/pagerank not serialized) | roko-index/Cargo.toml:30 |
| Cell pipeline (Parse/Graph/Score/… Cells) | v2/22/01 | none | ❌ roko-graph registers `NoopCell` placeholders named "ScoreCell"/"ComposeCell" | roko-graph/src/engine.rs:343–349 |
| File-watch Feed (auto reindex) | v2/22/01 | none | ❌ TUI `fs_watch.rs` exists but is not connected to the index | v2 doc lines 173–187 |
| Symbols-as-Signals (BLAKE3 hash, 5-axis, demurrage) | v2/22/02 | none | ❌ plain structs, no Signal/Engram integration | symbol.rs:25 |
| Feedback loops (gate verdict→rank, strategy Thompson sampling) | v2/22/01 | none | ❌ | — |
| Dense embeddings (fastembed) | v1/15/10 Tier 2 | `EmbeddingQuery` type only | ❌ silently falls back to keyword | workspace.rs:583–598 |

## V2-aligned

- `CodeIndex` trait abstracts backends (workspace.rs:351) — matches v2 open question #1's direction.
- 5-strategy search with RRF fusion exactly as specified in v2/22/04 (workspace.rs:604, 1435).
- Token-budgeted assembly with overlays + privacy redaction matches the Assemble Cell spec minus VCG (workspace.rs:929).
- Cross-language `SymbolKind` normalization table implemented as designed (v2/22/02 table ↔ lang crates).
- Uniform 10,240-bit HDC dimension matches the system-wide HDC algebra (even though the impl is duplicated).

## Old paradigm & tech debt

- 🕰️ **Monolithic `WorkspaceIndex` instead of cell pipeline** — v2's Graph-of-Cells (with TOML graph definition, Feed trigger, Bus feedback) is entirely unbuilt; the engine is a single bespoke struct built in one shot (workspace.rs:1016–1079).
- 🕰️ **HDC duplication** — roko-index/src/hdc.rs re-implements roko-primitives/src/hdc.rs (same constants `0x9E37_79B9_7F4A_7C15`, same `[u64; 160]`); violates "NEVER reimplement" rule; also duplicates `estimate_tokens`-style helpers.
- 🕰️ **`code_context_for_task` + `extract_task_keywords` duplicated verbatim** in `dispatch_helpers.rs:726` (live, used by orchestrate.rs:184) and `prompt_helpers.rs:230` (dead copy — zero callers).
- **Path-hash bug/stub**: `SqliteIndex::incremental_update` stores `blake3::hash(path_str.as_bytes())` (sqlite.rs:319) — the design's BLAKE3 *content* hashing is not implemented; change detection is mtime-only.
- **Rebuild deletes a DB nothing writes**: util.rs:1273–1277 removes `.roko/index.db`, but no code path creates it (sqlite feature off) — vestigial.
- **O(full-reparse) everywhere**: MCP server indexes once at startup and never refreshes (stale for long-lived servers, roko-mcp-code/src/lib.rs:189); orchestrator re-parses the entire workspace every 60 s of dispatch activity (orchestrate.rs:6209); `semantic_search` panics (`expect`) on internally missing fingerprints (workspace.rs:740–744).
- **Stale design docs**: v1/15/10 and v2/22/01 "Current State" sections both predate the CodeIndex trait, search API, CLI, SQLite module, MCP server, and compose wiring — all now exist.

## Not implemented

- Persistence at runtime (SQLite/FTS5, rkyv) — features never enabled; no index file in `.roko/`.
- Tree-sitter for TS/Go; tree-sitter Rust never compiled in; therefore no AST-accurate Calls edges (current Calls edges are regex).
- Cell pipeline, file-watch Feed, symbols-as-Signals, demurrage decay, gate-verdict feedback into PageRank, strategy-effectiveness learning (all of v2/22/01 §Feedback Loops).
- Dense embeddings; HDC search from the CLI (`roko index search` never sets an HDC sub-query, util.rs:1338).
- Functional integration test for `roko index` (cli_fallback.rs:144–147 only asserts the subcommands parse).
- No `.roko/GAPS.md` tracking of any of the above.

## Migration checklist

- [ ] **[P0]** Enable persistence: turn on `sqlite` feature in roko-cli + roko-mcp-code, make `roko index build` write `.roko/index.db`, and load-from-db when fresh — verify: `cargo run -p roko-cli -- index build && ls .roko/index.db && cargo run -p roko-cli -- index stats`
- [ ] **[P0]** Fix `incremental_update` to hash file **content** with BLAKE3 (sqlite.rs:319), fall back to mtime — verify: `cargo test -p roko-index --features sqlite`
- [ ] **[P1]** Deduplicate HDC: make roko-index depend on `roko-primitives::hdc` (`HdcVector`), keep `HdcFingerprint` as a thin newtype — verify: `grep -rn 'splitmix64' crates/roko-index/ | wc -l` → 0, `cargo test -p roko-index`
- [ ] **[P1]** Delete the dead `code_context_for_task`/`extract_task_keywords` copy in prompt_helpers.rs:230–360 — verify: `cargo clippy --workspace --no-deps -- -D warnings`
- [ ] **[P1]** Enable `tree-sitter` feature for roko-lang-rust in roko-index (or make it default) and switch `RUST_PROVIDER` to `TreeSitterRustProvider` with heuristic fallback — verify: `cargo run -p roko-cli -- index stats` shows higher symbol count on crates/
- [ ] **[P1]** Feed `CodeSlice` content (via `assemble_context`) into dispatch enrichment instead of bare symbol listings, budgeted — verify: run a plan task and inspect the "Relevant Code Symbols" layer in the composed prompt / episodes.jsonl
- [ ] **[P2]** Wire `personalized_pagerank` with task-symbol seeds into the dispatch search path (currently exported, uncalled) — verify: `grep -rn 'personalized_pagerank' crates/roko-cli/`
- [ ] **[P2]** Expose `--strategy hdc|semantic` in `roko index search` (util.rs:1311) — verify: `cargo run -p roko-cli -- index search "dependency graph" --strategy hdc`
- [ ] **[P2]** Add tree-sitter providers for TypeScript and Go (parity with rust) — verify: `cargo test -p roko-lang-typescript -p roko-lang-go`
- [ ] **[P2]** MCP server index refresh (mtime check or notify watcher) instead of load-once — verify: touch a file mid-session, `get_index_stats` reflects it
- [ ] **[P3]** Implement the v2 cell pipeline: wrap parse/graph/score/fingerprint/search/assemble as roko-graph Cells with a TOML pipeline + file-watch Trigger — verify: `grep -rn 'ParseCell' crates/roko-index/`
- [ ] **[P3]** Symbols-as-Signals + gate-feedback loop into PageRank weights (v2/22/01 §Feedback Loops) — verify: episodes show `Kind::CodeSymbol` signals
- [ ] **[P3]** Update stale design docs v1/15/10 + v2/22/01 "Current State" sections; log remaining gaps in `.roko/GAPS.md` — verify: `grep -in 'index' .roko/GAPS.md`

## Open questions

1. Should persistence live behind `CodeIndex` (SqliteIndex implementing the trait) or as a cache layer under `WorkspaceIndex`? SqliteIndex today implements neither the trait nor fingerprint/pagerank storage (symbols + edges only).
2. Is the 60 s dispatch-cache staleness (orchestrate.rs:6209, comment says "configurable" but it's a `const`) acceptable for large repos, or should the notify-based Feed land first?
3. `roko index rebuild` vs `build`: with no DB, they are identical — keep both subcommands? (CLAUDE.md documents only build/search/stats.)
4. Should roko-mcp-code's extra per-request file re-parsing (`lookup_symbol_details`, lib.rs:979) be replaced by `CodeSlice` from the index? (Boundary with the roko-mcp-code audit.)
5. Go provider maps `var` to `Const` and has no `Enum`; TS arrow functions land as `Const` — acceptable normalization loss, or does the graph need a `Binding` kind?

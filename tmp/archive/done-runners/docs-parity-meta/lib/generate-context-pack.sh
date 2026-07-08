#!/usr/bin/env bash
# generate-context-pack.sh — Generate the 6 shared context files for docs-parity2.
#
# Some files are static (rules, conventions, stub guidance).
# Others are auto-generated from section-map and crate scanning.

set -uo pipefail

_GEN_CTXPACK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Guard: only source dependencies if not already loaded
if [[ -z "${_SECTION_MAP_LOADED:-}" ]]; then
  source "$_GEN_CTXPACK_DIR/section-map.sh"
fi
if [[ -z "${_SCAN_CRATES_LOADED:-}" ]]; then
  source "$_GEN_CTXPACK_DIR/scan-crates.sh"
fi

: "${ROKO_ROOT:=/Users/will/dev/nunchi/roko/roko}"
: "${OUT_ROOT:=$ROKO_ROOT/tmp/docs-parity2}"

generate_context_pack() {
  local out_dir="$OUT_ROOT/context-pack"
  mkdir -p "$out_dir"

  echo "  00-DOCS-PARITY-RULES.md"
  _gen_rules > "$out_dir/00-DOCS-PARITY-RULES.md"

  echo "  01-SECTION-CRATE-MAP.md"
  _gen_section_crate_map > "$out_dir/01-SECTION-CRATE-MAP.md"

  echo "  02-WORKSPACE-TOPOLOGY.md"
  _gen_workspace_topology > "$out_dir/02-WORKSPACE-TOPOLOGY.md"

  echo "  03-EXISTING-PARITY-SUMMARY.md"
  _gen_parity_summary > "$out_dir/03-EXISTING-PARITY-SUMMARY.md"

  echo "  04-CODE-CONVENTIONS.md"
  _gen_code_conventions > "$out_dir/04-CODE-CONVENTIONS.md"

  echo "  05-PHASE2-STUB-GUIDANCE.md"
  _gen_stub_guidance > "$out_dir/05-PHASE2-STUB-GUIDANCE.md"
}

# ---------------------------------------------------------------------------
# 00 — Global rules
# ---------------------------------------------------------------------------
_gen_rules() {
  cat <<'RULES'
# Docs-Parity Runner — Common Rules (read first)

You are running as an unattended Codex batch from `tmp/docs-parity2`.
Your job: make the **code** match the **docs**. The docs are the source of truth
for what types, traits, functions, and behaviors should exist.

## Core rules

1. **No prior chat.** This prompt pack must be self-sufficient.
2. **Repository reality only.** Work from files that exist in the worktree.
   If a file, symbol, or path may have moved, verify with `rg --files` / `rg -n`
   and stop rather than guessing.
3. **Compile clean.** The verify gate per batch lists the exact `cargo` commands
   that must pass. A failure of any single command aborts the batch and triggers
   a retry.
4. **Write-scope discipline.** Stay inside the batch's write scope. If you need
   a 1-line adjacent fix for the batch to compile, make it; document it in the
   final message.
5. **Subagents authorised.** Use explorers and workers aggressively. Do not
   block on subagents when you can progress locally.
6. **Commit message format.** The runner commits as
   `docs-parity2(DPnn): <title>`. Do not commit yourself.
7. **No destructive git.** Never force-push, reset main, rm -rf outside the
   worktree. The runner handles branch/worktree lifecycle.
8. **No skipping verification.** `-D warnings` means what it says. Suppressing
   with `#[allow]` is only acceptable when the rule is genuinely wrong.
9. **Defer, do not delete.** If a batch's scope turns out too broad, finish
   the highest-leverage slice, leave a precise follow-up note, and let the
   runner move on. Do not leave uncompilable code.
10. **Naming conventions.** `neuro` (not grimoire), `Korai` (not styx/Styx),
    `fleet` (not clade). No death/mortality language. `bardo-backup/` is
    read-only.

## Batch completion bar

A batch is only complete when:

- the listed gaps are closed (types/traits/functions added)
- the batch's verify gate passes in the runner worktree
- any new files are wired into their parent module tree (`mod.rs` exports)
- new public types carry `///` doc comments
- the commit (made by the runner, not you) lands on the batch branch

## Failure behaviour

If a batch is too large:
- finish the highest-dependency work first
- leave a precise note in the final message listing what remains
- do not stop at analysis
- do not leave half-compiling code

## Context paths

Always read these files before coding:

1. `tmp/docs-parity2/context-pack/00-DOCS-PARITY-RULES.md` (this file)
2. `tmp/docs-parity2/context-pack/01-SECTION-CRATE-MAP.md`
3. `tmp/docs-parity2/context-pack/02-WORKSPACE-TOPOLOGY.md`
4. `tmp/docs-parity2/context-pack/03-EXISTING-PARITY-SUMMARY.md`
5. `tmp/docs-parity2/context-pack/04-CODE-CONVENTIONS.md`
6. `tmp/docs-parity2/context-pack/05-PHASE2-STUB-GUIDANCE.md`

## Environment invariants

- Rust toolchain >= 1.91 (`rustup update stable` required for `alloy` deps)
- `.roko/` and `tmp/` are gitignored at repo root
- Workspace root: `/Users/will/dev/nunchi/roko/roko`
RULES
}

# ---------------------------------------------------------------------------
# 01 — Section-crate map (auto-generated)
# ---------------------------------------------------------------------------
_gen_section_crate_map() {
  cat <<'HEADER'
# Section-to-Crate Map

This table maps each docs section to its target crate(s), priority, and group.

| Batch | Section | Crate(s) | Priority | Group | Dependencies |
|-------|---------|----------|----------|-------|--------------|
HEADER

  for entry in "${SECTION_REGISTRY[@]}"; do
    local num slug crates pri grp deps batch_id
    num="$(section_num "$entry")"
    slug="$(section_slug "$entry")"
    crates="$(section_crates "$entry")"
    pri="$(section_priority "$entry")"
    grp="$(section_group "$entry")"
    deps="$(section_deps "$entry")"
    batch_id="$(batch_id_for "$num")"
    printf '| %s | %s-%s | %s | %s | %s | %s |\n' \
      "$batch_id" "$num" "$slug" "$crates" "$pri" "$grp" "${deps:-none}"
  done

  cat <<'FOOTER'

## Groups

- **core** (P0): DP00-DP05 — Architecture, orchestration, agents, composition, verification, learning
- **extensions** (P1): DP06-DP07 — Neuro, conductor
- **safety-iface** (P0): DP11-DP12 — Safety, interfaces
- **infra** (P1): DP13, DP15-DP19 — Coordination, code-intel, heartbeat, lifecycle, tools, deployment
- **phase2** (P2): DP08-DP10, DP14, DP20 — Chain, daimon, dreams, identity-economy, technical-analysis

## Execution Order

1. Core foundation (DP00-DP05)
2. Safety + interfaces (DP11-DP12)
3. Extensions + infra (DP06-DP07, DP13, DP15-DP19)
4. Phase 2+ stubs (DP08-DP10, DP14, DP20)
FOOTER
}

# ---------------------------------------------------------------------------
# 02 — Workspace topology (auto-generated from crate scan)
# ---------------------------------------------------------------------------
_gen_workspace_topology() {
  cat <<'HEADER'
# Workspace Topology

All crates, their paths, and approximate sizes.

| Crate | Path | LOC | Role |
|-------|------|-----|------|
HEADER

  for dir in "$ROKO_ROOT"/crates/roko-*/; do
    [[ -d "$dir" ]] || continue
    local crate_name
    crate_name="$(basename "$dir")"
    local loc
    loc="$(crate_loc "$crate_name")"
    local role=""
    case "$crate_name" in
      roko-core) role="Kernel: Signal + 6 traits, types, config" ;;
      roko-agent) role="Agent backends, tool loop, safety, MCP" ;;
      roko-agent-server) role="Per-agent HTTP sidecar" ;;
      roko-serve) role="HTTP control plane (~200 routes)" ;;
      roko-orchestrator) role="Plan DAG, parallel executor, merge queue" ;;
      roko-gate) role="11 gates, 7-rung pipeline, adaptive thresholds" ;;
      roko-compose) role="Prompt assembly, templates, enrichment" ;;
      roko-conductor) role="Watchers, circuit breaker, diagnosis" ;;
      roko-learn) role="Episodes, playbooks, bandits, routing" ;;
      roko-cli) role="CLI binary: all subcommands, TUI" ;;
      roko-fs) role="FileSubstrate (JSONL), GC, layout" ;;
      roko-std) role="Defaults, builtin tools, mock dispatcher" ;;
      roko-runtime) role="ProcessSupervisor, event bus, cancellation" ;;
      roko-primitives) role="HDC vectors, tier routing" ;;
      roko-neuro) role="Durable knowledge store, distillation" ;;
      roko-dreams) role="Offline consolidation (Phase 2+)" ;;
      roko-daimon) role="Behavior primitives (Phase 2+)" ;;
      roko-chain) role="Chain witness primitives (Phase 2+)" ;;
      roko-index) role="Parser + graph + HDC indexing" ;;
      roko-mcp-code) role="Code-intelligence MCP server" ;;
      roko-mcp-*) role="MCP integration" ;;
      roko-lang-*) role="Language support" ;;
      *) role="—" ;;
    esac
    printf '| %s | crates/%s/src/ | %s | %s |\n' \
      "$crate_name" "$crate_name" "$loc" "$role"
  done

  cat <<'FOOTER'

## Key paths

- **Workspace root**: `/Users/will/dev/nunchi/roko/roko/`
- **All crates**: `/Users/will/dev/nunchi/roko/roko/crates/`
- **CLI source**: `crates/roko-cli/src/`
- **Orchestrator**: `crates/roko-cli/src/orchestrate.rs`
- **Agent dispatcher**: `crates/roko-agent/src/dispatcher/mod.rs`
- **System prompt builder**: `crates/roko-compose/src/system_prompt_builder.rs`

## Do not touch

- `bardo-backup/` — read-only reference material
- `.roko/` — runtime data directory (gitignored)
- `tmp/` — runner artifacts (gitignored)
- `target/` — build artifacts
FOOTER
}

# ---------------------------------------------------------------------------
# 03 — Existing parity summary (auto-generated from gap scans)
# ---------------------------------------------------------------------------
_gen_parity_summary() {
  cat <<'HEADER'
# Existing Parity Analysis Summary

Key findings from the prior `tmp/docs-parity/` audit (sections 00-12).
This is a digest — see the full analysis files for details.

HEADER

  for num in 00 01 02 03 04 05 06 07 08 09 10 11 12; do
    local parity_dir="$ROKO_ROOT/tmp/docs-parity/$num"
    if [[ ! -d "$parity_dir" ]]; then
      continue
    fi
    local gaps_file="$parity_dir/context-pack/gaps-summary.md"
    local index_file="$parity_dir/00-INDEX.md"

    printf '## Section %s\n\n' "$num"

    if [[ -f "$gaps_file" ]]; then
      # Extract the highest-value corrections section
      sed -n '/^## Highest/,/^## /p' "$gaps_file" | head -20
      echo
    elif [[ -f "$index_file" ]]; then
      head -10 "$index_file"
      echo
    else
      echo "(No analysis available)"
      echo
    fi
  done
}

# ---------------------------------------------------------------------------
# 04 — Code conventions (static)
# ---------------------------------------------------------------------------
_gen_code_conventions() {
  cat <<'CONVENTIONS'
# Roko Code Conventions

Follow these patterns when adding code to any crate.

## Doc comments

```rust
/// Short one-line summary.
///
/// Longer description if needed. Reference other types with [`TypeName`].
///
/// # Examples
///
/// ```no_run
/// let x = TypeName::new();
/// ```
pub struct TypeName { ... }
```

## Module wiring

Every new file must be declared in its parent `mod.rs`:

```rust
// In mod.rs
pub mod new_module;
```

Every public type in a submodule should be re-exported from the crate root
or the parent module's `mod.rs`:

```rust
pub use self::new_module::NewType;
```

## Error handling

- Use `thiserror::Error` for error enums
- Propagate with `?`, never `unwrap()` on hot paths
- `unwrap()` is acceptable only in tests and infallible paths (e.g., regex compilation)

## Test patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_does_the_thing() {
        // arrange
        // act
        // assert
    }
}
```

## Naming

- Types: `PascalCase`
- Functions: `snake_case`
- Constants: `SCREAMING_SNAKE`
- Crate names: `roko-*` (kebab-case)
- Module names: `snake_case`

## Imports

Prefer absolute crate paths for cross-crate imports:

```rust
use roko_core::signal::Signal;
use roko_agent::provider::ProviderRegistry;
```
CONVENTIONS
}

# ---------------------------------------------------------------------------
# 05 — Phase 2+ stub guidance (static)
# ---------------------------------------------------------------------------
_gen_stub_guidance() {
  cat <<'STUBS'
# Phase 2+ Stub Guidance

Sections 08-chain, 09-daimon, 10-dreams, and 14-identity-economy describe
functionality planned for Phase 2+. The batches for these sections create
**stubs only** — type shells with doc comments, not full implementations.

## What a stub looks like

```rust
/// Represents a witness event observed on-chain.
///
/// Phase 2+: Will track block number, transaction hash, and decoded event
/// data for on-chain verification workflows.
#[derive(Debug, Clone)]
pub struct ChainWitness {
    /// The chain ID this witness observed.
    pub chain_id: u64,
    /// Block number of the witnessed event.
    pub block_number: u64,
    /// Human-readable description of what was witnessed.
    pub description: String,
}

impl ChainWitness {
    /// Create a new chain witness record.
    pub fn new(chain_id: u64, block_number: u64, description: impl Into<String>) -> Self {
        Self {
            chain_id,
            block_number,
            description: description.into(),
        }
    }
}
```

## Rules for stubs

1. **Struct fields should be real.** Use concrete types from the doc descriptions.
   Don't use `()` or `PhantomData` unless the doc explicitly says the type is generic.

2. **Constructor methods can be real.** Simple `new()` and accessor methods are fine.

3. **Complex logic gets `todo!()`.** Methods that require external state, async I/O,
   or multi-step algorithms should have `todo!("Phase 2+: <what this does>")`.

4. **Trait impls use defaults or todo.** If a trait method has a sensible default,
   use it. Otherwise `todo!()`.

5. **No external deps.** Don't add new crate dependencies. Use types already available.

6. **`#[allow(dead_code)]` is fine.** Since stubs won't be called yet, suppress
   dead-code warnings at the module level if needed.

7. **Wire into mod.rs.** Even stubs should be reachable from the crate root.

## What NOT to stub

- Don't create empty files with just `// TODO`
- Don't stub private helper functions
- Don't stub test utilities
- Don't create integration tests for stub functionality
STUBS
}

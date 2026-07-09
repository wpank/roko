# A — Gate Foundation (Docs 00, 01)

Parity analysis of `docs/04-verification/00-gate-trait.md` and
`docs/04-verification/01-gate-implementations.md` vs the actual codebase.

---

## A.01 — Gate trait signature returns `Verdict` (not `Result<Verdict>`)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 00 §2 — `async fn verify(&self, signal: &Signal, ctx: &Context) -> Verdict`. Doc emphasizes this is the load-bearing design decision.
**Reality**: `crates/roko-core/src/traits.rs:102-108` defines the trait with `async fn verify(&self, signal: &Engram, ctx: &Context) -> Verdict` plus `fn name(&self) -> &str`. Exactly two methods, no `Result` wrapper. `#[async_trait]` applied. `Send + Sync` bounds present.
**Notes**: The doc says `Signal`; code says `Engram` — naming-only drift already flagged in the project naming map. Substance matches.

---

## A.02 — `name()` method for per-gate identification

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 00 §3 — every verdict carries the name of the gate that produced it; `category:tool` convention (e.g., `compile:cargo`, `test:cargo`).
**Reality**: Trait signature `fn name(&self) -> &str` at `crates/roko-core/src/traits.rs:107`. All concrete gates implement it; `CompileGate` and siblings default to `compile:cargo`, `test:cargo`, `clippy:cargo` naming (confirmed in `clippy_gate.rs`, `compile.rs`, `test_gate.rs`).

---

## A.03 — Verdict construction helpers (`pass`, `fail`, `with_detail`, `with_duration`, `with_error_digest`, `with_test_count`)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 00 §4 describes the fluent `Verdict::pass(name)`, `Verdict::fail(name, reason)`, with optional `detail`, `duration_ms`, `test_count`, `error_digest`.
**Reality**: All helpers present in `crates/roko-core`. Confirmed usage throughout gate impls: `Verdict::pass(&self.name).with_detail(...).with_duration(elapsed)` pattern seen in `shell.rs`, `compile.rs`, `test_gate.rs`, `clippy_gate.rs`.

---

## A.04 — Gate inventory count

**Status**: PARTIAL (MEDIUM severity)
**Doc claim**: Doc INDEX.md line 29 says "11 concrete gates". Doc 01 §1 also lists 11 gates in the header table.
**Reality**: 13 `Gate`-implementing files in `crates/roko-gate/src/`. Breakdown:

| # | Gate | File | LOC | Status |
|---|------|------|-----|--------|
| 1 | `ShellGate` | `shell.rs` | 185 | Concrete |
| 2 | `CompileGate` | `compile.rs` | 194 | Concrete |
| 3 | `ClippyGate` | `clippy_gate.rs` | 211 | Concrete |
| 4 | `TestGate` | `test_gate.rs` | 383 | Concrete |
| 5 | `SymbolGate` | `symbol_gate.rs` | 1002 | Concrete |
| 6 | `DiffGate` | `diff_gate.rs` | 357 | Concrete |
| 7 | `GeneratedTestGate` | `generated_test_gate.rs` | 820 | Concrete |
| 8 | `PropertyTestGate` | `property_test_gate.rs` | 695 | Concrete |
| 9 | `IntegrationGate` | `integration_gate.rs` | 803 | Concrete |
| 10 | `LlmJudgeGate` | `llm_judge_gate.rs` | 565 | SCAFFOLD |
| 11 | `VerifyChainGate` | `verify_chain_gate.rs` | 882 | SCAFFOLD |
| 12 | `FactCheckGate` | `fact_check.rs` | 491 | SCAFFOLD |
| 13 | `CodeExecutionGate` | `code_exec.rs` | 333 | SCAFFOLD |

Doc 01 mentions `FactCheckGate` and `CodeExecutionGate` nowhere. Both exist as Gate impls with fully declared modules in `lib.rs:21-25`.
**Fix sketch**: Update doc 01 §1 to "9 concrete + 4 scaffold = 13 total"; add rows for `FactCheckGate`, `CodeExecutionGate`.

---

## A.05 — `ShellGate` foundation for subprocess gates

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §2 — ShellGate runs arbitrary command, passes on exit 0, 5-minute default timeout, `kill_on_drop(true)`, converts timeouts/spawn errors to `Verdict::fail()`.
**Reality**: `crates/roko-gate/src/shell.rs:57-118` implements all of the above. Struct at `shell.rs:1-56` has `program`, `args`, `timeout_ms` (default 300_000), `name`. Pattern of three outcomes (timeout / spawn err / exit code) is exactly as doc describes.

---

## A.06 — `CompileGate` with `BuildSystem` dispatch

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §3 — wraps ShellGate pattern with BuildSystem awareness (Cargo/Npm/Go/Make). 10-minute default timeout. `summarize_errors()` extracts up to 3 error lines.
**Reality**: `crates/roko-gate/src/compile.rs` — 194 LOC. `summarize_errors()` at lines 134-151 matches doc snippet byte-for-byte. `BuildSystem` enum dispatch lives in `crates/roko-gate/src/payload.rs:86-189` with `program()`, `check_args()`, `test_args()`, `lint_args()` per variant.

---

## A.07 — `ClippyGate` with Cargo `--` sentinel handling

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §4 — extra args spliced before `--` so they apply to Cargo, not Clippy. 5-minute timeout.
**Reality**: `crates/roko-gate/src/clippy_gate.rs:70-93` contains the exact splicing logic shown in doc. 211 LOC total.

---

## A.08 — `TestGate` with per-build-system test count parsing

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §5 — 15-minute default timeout; `TestSelector` enum (All/Quick/Patterns); per-build-system parsers for Cargo and Go.
**Reality**: `crates/roko-gate/src/test_gate.rs:166-241` implements `parse_test_counts()` dispatched by `BuildSystem`. 383 LOC. `parse_test_counts` is re-exported from `lib.rs:55`. `TestSelector` enum present with `All`, `Quick`, `Patterns(Vec<String>)` variants. Verdict carries `test_count` via `.with_test_count(tc)` as doc claims.

---

## A.09 — `SymbolGate` — zero-subprocess Rust source scanner

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §6 — 1001 lines, no subprocess, parses Rust sources directly, produces 5 mismatch categories (MISSING/WRONG_VIS/WRONG_KIND/WRONG_PATH/AMBIGUOUS).
**Reality**: `crates/roko-gate/src/symbol_gate.rs` — **1002 LOC** (doc was one line off in both directions across sources). `extract_symbols()` single-pass scanner lives around lines 449-478 as claimed. All five mismatch categories present and tested.
**Notes**: Despite being one of the most substantial gate implementations in the crate, SymbolGate is **unreachable** from `roko plan run` (see B.04).

---

## A.10 — `DiffGate` — vacuous-implementation rejection

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §7 — rejects empty diffs, below-threshold diffs, and all-forbidden-token diffs. Forbidden tokens include `todo!()`, `unimplemented!()`, `panic!("not implemented")`, `Ok(())`.
**Reality**: `crates/roko-gate/src/diff_gate.rs:1-33` module header exactly matches doc prose. 357 LOC. `analyze_diff()` pure function + `DiffAnalysis` struct both present. Re-exported from `lib.rs:47`.
**Notes**: DiffGate is unreachable from `run_gate_rung` (see B.04). Doc labels it "N/A (pre-pipeline)" but there is no pre-pipeline call site in orchestrate.rs either.

---

## A.11 — Scaffold gates: `LlmJudgeGate`, `VerifyChainGate`, `FactCheckGate`, `CodeExecutionGate`

**Status**: SCAFFOLD (LOW severity)
**Doc claim**: Doc 01 §8 mentions `LlmJudgeGate` and `VerifyChainGate` as "auxiliary". `FactCheckGate` and `CodeExecutionGate` are not mentioned in doc 01 at all.
**Reality**:
- `llm_judge_gate.rs` — 565 LOC. Full Gate impl with judge prompt scaffolding; no production wiring.
- `verify_chain_gate.rs` — 882 LOC. Phase-2+ chain verification scaffold.
- `fact_check.rs` — 491 LOC. `FactCheckGate` + `SearchOracle` trait, no production backend.
- `code_exec.rs` — 333 LOC. `CodeExecutionGate` with `CodeExecutionBackend` trait, no production backend.

All four are referenced from `lib.rs:21-39` but have zero call sites outside their own test modules.
**Fix sketch**: Doc 01 §8 should either list all 4 scaffolds or explicitly label this set as "scaffold — not wired".

---

## A.12 — Timeout contract and `kill_on_drop`

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 00 §5.1 — every gate enforces a timeout via `tokio::time::timeout`; doc 01 §9.3 — every subprocess-spawning gate sets `kill_on_drop(true)`.
**Reality**: Verified in `shell.rs:57-118`. `CompileGate` (600,000 ms), `TestGate` (900,000 ms), `ClippyGate` (300,000 ms), `ShellGate` (300,000 ms) match the doc's §5.1 table exactly.

---

## A.13 — Error summarization helpers per gate

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §9.4 — each gate has `summarize_*` that keeps verdict `reason` concise while `detail` carries full output.
**Reality**:
- `summarize_errors()` in `compile.rs:134-151`
- `summarize_test_failures()` in `test_gate.rs:244-267`
- `summarize_lint_issues()` in `clippy_gate.rs:146-163`

All return compact strings (max 3 items joined by `; `) as doc describes.

---

## A.14 — `GatePayload` as the per-gate contract

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §9.2 — gates read `working_dir`, `extra_env`, `target_dir`, build system, test selector from a `GatePayload` in the signal body.
**Reality**: `crates/roko-gate/src/payload.rs:1-345` defines `GatePayload`, `BuildSystem`, `TestSelector`. Builder API (`in_dir`, `with_label`, `with_build_system`, `with_extra_env`) is used throughout orchestrate.rs (e.g. `orchestrate.rs:11146`). Re-exports from `lib.rs:52`.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 12 |
| PARTIAL | 1 (A.04 doc undercount) |
| SCAFFOLD | 1 (A.11 unwired scaffolds) |

The gate **foundation** layer matches its docs closely. The `Verdict` return type is real, timeouts are enforced, builder APIs work, and `GatePayload` is the actual carrier used at runtime. The main drift is the doc's gate-count header (11) versus reality (13 files, 9 concrete + 4 scaffolds). SymbolGate is the most complete gate implementation in the crate but is unreachable from orchestrate (see B.04).

## Agent Execution Notes

### A.04 — Gate Inventory Truth In Advertising

This is mostly a docs-honesty task, but it matters because later agents will otherwise assume the wrong runtime surface.

Recommended slice:

1. distinguish concrete gates from scaffold gates explicitly,
2. fix the gate-count claim wherever it still says 11,
3. do not claim runtime reachability just because a `Gate` impl exists.

Acceptance criteria:

- later agents can tell which gates are concrete,
- later agents can tell which gates are actually reachable,
- doc fixes do not quietly overstate runtime activation.

### A.11 — Scaffold Gates

Do not widen batch `04` into backend implementation for `LlmJudgeGate`, `VerifyChainGate`, `FactCheckGate`, or `CodeExecutionGate` unless a later batch explicitly owns that work.

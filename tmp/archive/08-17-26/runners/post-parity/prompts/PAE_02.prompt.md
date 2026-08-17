# PAE_02: Wire rung 3-6 gate inputs to eliminate stub verdicts

## Task
Provide real inputs to rung 3-6 gates so they produce actual verdicts instead of `stub_verdict()` with "not wired" messages.

## Runner Context
Runner PAE (Gate Pipeline Completeness), batch 2 of 4. Depends on PAE_01.

## Problem
GP-2 anti-pattern: "Stubs that silently pass." Rungs 3-6 in `rung_dispatch.rs:132-242` call `stub_verdict()` which returns `Verdict::pass()` when prerequisites are missing. This means higher rungs never actually validate — they report "passed" with a message like "no SymbolManifest wired".

## Current Stub Verdicts (VERIFIED at rung_dispatch.rs)

| Rung | Gate | Stub Message | Missing Input |
|------|------|-------------|---------------|
| 3 | symbol | "no SymbolManifest wired" (L146) | `SymbolManifest` |
| 3 | symbol | "no source roots configured" (L149) | Source root paths |
| 3 | generated_test | "generated test artifacts not wired" (L173) | Test artifact store |
| 4 | verify_chain | "no verify script wired" (L186) | Verify script path |
| 5 | fact_check | "no fact-check content" (L201) | Content to check |
| 5 | fact_check | "no fact-check oracle" (L204) | Oracle (Perplexity) |
| 6 | llm_judge | "no judge payload" (L220) | Prompt/output pair |
| 6 | llm_judge | "no judge oracle" (L223) | LLM oracle |
| 6 | integration | "no integration scenario wired" (L237) | Test scenario |

## Exact Changes

### Step 1: Wire SymbolManifest for rung 3

The code index (roko-index) can provide symbol manifests. Wire it:

```rust
// In GateService or the rung dispatch caller:
let symbol_manifest = if let Some(index) = &code_index {
    index.symbol_manifest_for_files(&task.files)
} else {
    None
};
```

Pass as gate input. If `symbol_manifest` is `Some`, rung 3 runs real validation. If `None`, it stubs (existing behavior preserved).

### Step 2: Wire source roots for rung 3

```rust
let source_roots: Vec<PathBuf> = config.project.source_roots
    .clone()
    .unwrap_or_else(|| vec![workdir.join("src"), workdir.join("crates")]);
```

### Step 3: Wire LLM oracle for rung 6

Use ModelCallService (from PAD_02) as the LLM oracle:

```rust
// Create a JudgeOracle adapter wrapping ModelCallService:
struct McsJudgeOracle {
    mcs: Arc<ModelCallService>,
    model: String,  // prefer haiku for judging (cheap + fast)
}

impl JudgeOracle for McsJudgeOracle {
    async fn judge(&self, prompt: &str, output: &str) -> Result<JudgeVerdict> {
        let response = self.mcs.call(ModelCallRequest {
            prompt: format!("Judge the following code change...\n\nPrompt: {prompt}\n\nOutput: {output}"),
            model: Some(self.model.clone()),
            ..Default::default()
        }).await?;
        parse_judge_verdict(&response.content)
    }
}
```

### Step 4: Don't wire rungs 4-5 yet (future work)

Rungs 4 (verify_chain) and 5 (fact_check) require external infrastructure:
- Rung 4 needs a verify script path (project-specific)
- Rung 5 needs a Perplexity oracle (provider config dependent)

Add TODO comments but don't wire:
```rust
// TODO(PAE_02): Rung 4 verify_chain needs project-specific verify script
// TODO(PAE_02): Rung 5 fact_check needs Perplexity oracle provider
```

## Write Scope
- `crates/roko-gate/src/rung_dispatch.rs` (accept optional inputs for rungs 3, 6)
- `crates/roko-gate/src/gate_service.rs` (wire symbol manifest and LLM oracle)
- `crates/roko-gate/src/llm_judge_gate.rs` (verify JudgeOracle trait compatibility)

## Read-Only Context
- `crates/roko-index/src/lib.rs` (symbol manifest API)
- `crates/roko-agent/src/model_call_service.rs` (for McsJudgeOracle)


## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test --workspace 2>&1 | tail -20
```
## Acceptance Criteria
- Rung 3 runs real symbol validation when index is available
- Rung 6 runs LLM judge when ModelCallService is available
- Missing inputs → existing stub behavior (backward compatible)
- No stub_verdict calls when real inputs are provided
- Rungs 4-5 documented as future work

## Do NOT
- Wire rungs 4-5 (out of scope — need project-specific config)
- Change the stub_verdict function (it's correct behavior for missing inputs)
- Force all rungs to be active (optional activation is correct)

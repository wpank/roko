# Runner 17-18 — Demo + Proof Runs

> **Give this entire file to a fresh agent.** These are the polish + acceptance phases.

---

## Context

Codebase: `/Users/will/dev/nunchi/roko/roko`. Goals:
- **17:** Polish the CLI demo output (predict/knowledge/resume/share blocks) and web demo scenarios
- **18:** Run the 12-point proof matrix + cross-cutting checks that validate the entire migration

**Read first:**

1. `tmp/workflow/implementation-plans/17-demo-completion.md`
2. `tmp/workflow/implementation-plans/18-proof-runs.md`
3. `tmp/workflow/demo/DEMO-FLOW.md` — the target CLI output format
4. `crates/roko-cli/src/run.rs` — where `roko run` output is produced
5. `crates/roko-cli/src/share.rs` — `--share` flag implementation

---

## Plan 17 — Demo CLI Track

### 17-1: Output formatter

Create `crates/roko-cli/src/output_format/mod.rs`:

```rust
pub trait RunOutputFormatter {
    fn plan(&mut self, plan: &PlanBlock);
    fn predict(&mut self, predict: &PredictBlock);
    fn knowledge(&mut self, knowledge: &KnowledgeBlock);
    fn run_started(&mut self);
    fn tool_call(&mut self, call: &ToolCallEvent);
    fn gate_verdict(&mut self, verdict: &GateVerdict);
    fn done(&mut self, summary: &DoneBlock);
    fn share_url(&mut self, url: &str);
}
```

Implement `ClackStyle` with the Clack-style blocks (◆/◇/│/└/✔).

### 17-2: Predict block

Source: `CascadeRouter` selection result + `efficiency.jsonl` regression for estimated cost/turns.

### 17-3: Knowledge block

Source: `PromptAssemblyService::assemble()` diagnostics → `knowledge_ids.len()`, source descriptions.

### 17-4: Done block with delta

`DoneBlock { actual_cost, predicted_cost, delta_pct }`. Compute after workflow completes.

### 17-5: Resume polish

Verify `Ctrl+C` saves checkpoint; `roko run --resume <id>` continues.

### 17-6: Share polish

`--share` prints both `nunchi://run/<id>` and `https://share.nunchi.dev/r/<id>`.

### 17-7: Tokyo Night theme

Create `crates/roko-cli/src/inline/themes/tokyo_night.rs`. Select via `ROKO_THEME=tokyo-night`.

### 17-8: Rehearsal script

Create `scripts/demo-rehearsal.sh`: config doctor → share doctor → pre-warm knowledge → pre-warm router → print ready message.

---

## Plan 18 — Proof Runs

Create `crates/roko-cli/tests/proof_runs.rs` gated by `#[cfg(feature = "proof")]`.

### The 12 Proofs

Each is a `#[tokio::test(flavor = "multi_thread")]`:

1. **7.1** One-task plan: executes, episode written, state persisted
2. **7.2** Diamond DAG: A→{B,C}→D→E in order; B/C overlap
3. **7.3** Gate failure → autofix → gate pass
4. **7.4** Gate exhausted → replan or halt
5. **7.5** Reviewer rejects → implementer retries with findings → approved
6. **7.6** Crash + resume → no duplicate completions
7. **7.7** 5 failures → router avoids model
8. **7.8** Run 1 episode → run 2 prompt includes knowledge
9. **7.9** Provider matrix: each configured provider produces result or classified DOA
10. **7.10** HTTP: serve + plan run + query events endpoint matches JSONL
11. **7.11** ACP: session/prompt → implement → gate → review → commit
12. **7.12** Single prompt express: implement → gate → commit

### Cross-Cutting Proofs

- **CC1** No bare `Command::new("claude")` outside adapter
- **CC2** One canonical `FeedbackEvent` enum
- **CC3** Total LOC reduced by ≥100K
- **CC4** All required new files present (persistence.rs, failure_tracker.rs, warning_store.rs, etc.)
- **CC5** All retired files absent (orchestrate.rs, daimon/, auction.rs, etc.)

### Running

```bash
cargo test --workspace --features proof -- --test-threads=1 --nocapture
```

Each proof uses a fresh `tempdir()`. No shared state. Budget provider set to `claude-haiku-4` or local Ollama for cost control.

---

## Verification

All 12 proofs + 5 cross-cutting checks must pass:

```bash
cargo test --features proof -- proof_7_ --test-threads=1
# all 12 pass

cargo test --features proof -- proof_cc --test-threads=1
# all 5 pass
```

---

## Critical Rules

1. **Proofs run against the real binary, not mocks.** The point is end-to-end validation.
2. **Each proof gets a fresh tempdir.** No global state pollution.
3. **Don't run in parallel** (`--test-threads=1`) — each uses real subprocesses.
4. **Use cheap models** (`claude-haiku-4` / local Ollama) to keep cost < $1 per full suite.
5. **Don't accept partial passes.** All 17 checks must be green.

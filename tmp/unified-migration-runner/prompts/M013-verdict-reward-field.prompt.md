# M013 — Add Verdict.reward field and verify_pre method

## Objective
Enhance the Verdict struct with a continuous `reward` field (f64, 0.0–1.0) to enable
gradient-based learning from gate results. Also add a `verify_pre` method to the Gate
trait for pre-execution verification (the spec's Verify protocol has both pre and post).

## Scope
- Crates: `roko-core`, `roko-gate`
- Files: `crates/roko-core/src/verdict.rs`, `crates/roko-core/src/traits.rs`
- Phase ref: 02-PHASE-1-KERNEL.md §1.1

## Steps
1. Find the Verdict struct:
   `grep -rn 'pub struct Verdict' crates/roko-core/src/ --include='*.rs'`

2. Add a `reward` field to Verdict:
   ```rust
   /// Continuous reward signal (0.0 = complete failure, 1.0 = perfect).
   /// Used by learning loops for gradient-based feedback.
   /// Defaults to 1.0 if passed, 0.0 if failed (but gates can set intermediate values).
   pub reward: f64,
   ```

3. Find all places that construct Verdict and add `reward`:
   ```bash
   grep -rn 'Verdict {' crates/ --include='*.rs' | grep -v target/
   grep -rn 'Verdict::new' crates/ --include='*.rs' | grep -v target/
   ```
   Set `reward: if passed { 1.0 } else { 0.0 }` as the default for existing constructors.

4. Add `verify_pre` to the Gate trait (with a default no-op implementation so existing
   gates don't break):
   ```rust
   /// Pre-execution verification. Called before the agent runs.
   /// Default: always passes.
   async fn verify_pre(&self, _input: &[Engram], _ctx: &Context) -> Result<Verdict> {
       Ok(Verdict::pass("pre-check: default pass"))
   }
   ```

5. Ensure Verdict's Serialize/Deserialize includes the reward field with a default:
   ```rust
   #[serde(default = "default_reward")]
   pub reward: f64,
   ```

## Verification
```bash
cargo check -p roko-core -p roko-gate -p roko-cli
cargo clippy -p roko-core -p roko-gate --no-deps -- -D warnings
cargo test -p roko-core --lib --no-run
```

## What NOT to do
- Do NOT change existing gate logic — just add the field with sensible defaults
- Do NOT make verify_pre mandatory (use default impl)
- Do NOT change how the gate pipeline runs — pre-check integration comes later

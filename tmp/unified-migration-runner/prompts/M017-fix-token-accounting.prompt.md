# M017 — Fix token accounting in gateway.rs

## Objective
The inference gateway uses `estimate_tokens()` (line 877-879: `text.len().div_ceil(4)`) — a rough character-based heuristic — instead of real token counts from LLM responses. Replace this heuristic with actual token counts extracted from provider responses, falling back to the heuristic only when the provider doesn't report usage.

## Scope
- Crates: `roko-serve`, `roko-agent`
- Files:
  - `crates/roko-serve/src/routes/gateway.rs` (lines ~298-302, ~538-541, ~877-879)
- Phase ref: `tmp/unified-migration/01-PHASE-0-PREP.md` §0.2
- Audit ref: `tmp/roko-trustworthy/AUDIT.md` §B1

## Steps
1. Locate all uses of the heuristic:
   ```bash
   grep -n 'estimate_tokens' crates/roko-serve/src/routes/gateway.rs
   ```

2. Check what the agent response already contains:
   ```bash
   grep -rn 'usage\|token_count\|tokens_used\|prompt_tokens\|completion_tokens' crates/roko-agent/src/ --include='*.rs' | head -20
   grep -rn 'AgentResult\|AgentResponse\|DispatchResult' crates/roko-agent/src/ --include='*.rs' | grep 'pub struct' | head -10
   ```

3. The `AgentResult` or equivalent response type from roko-agent should contain a `usage` or `token_usage` field. If it does, extract `input_tokens` and `output_tokens` from it.

4. In `inference_complete()` (around line 298-302), replace:
   ```rust
   let input_tokens = estimate_tokens(&prompt);
   let output_tokens = estimate_tokens(&content);
   ```
   with:
   ```rust
   let input_tokens = response.usage.map(|u| u.input_tokens).unwrap_or_else(|| estimate_tokens(&prompt));
   let output_tokens = response.usage.map(|u| u.output_tokens).unwrap_or_else(|| estimate_tokens(&content));
   ```
   Adjust field names based on the actual response type.

5. Apply the same fix in `batch_submit()` (around line 538-541) where batch items also use `estimate_tokens`.

6. Add a doc comment to `estimate_tokens()` marking it as a fallback:
   ```rust
   /// Fallback token count estimate when the provider doesn't report usage.
   /// ~4 characters per token for English text. Prefer real counts from LLM responses.
   ```

7. Update the existing test `estimate_tokens_rounds_up` and add a new test verifying that real token counts are preferred over the heuristic when available.

## Verification
```bash
cargo check -p roko-serve
cargo clippy -p roko-serve --no-deps -- -D warnings
cargo test -p roko-serve -- gateway
# Confirm the heuristic is now only a fallback:
grep -n 'estimate_tokens' crates/roko-serve/src/routes/gateway.rs
```

## What NOT to do
- Do NOT remove `estimate_tokens()` — it's still needed as a fallback for providers that don't report usage
- Do NOT add a tokenizer dependency (tiktoken, etc.) — that's overkill for this fix
- Do NOT change the cost computation logic in `compute_cost()` — just feed it accurate token counts
- Do NOT modify the AgentResult type — use whatever usage field it already exposes

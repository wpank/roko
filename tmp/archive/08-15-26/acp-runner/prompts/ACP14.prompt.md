# Batch ACP14 — Usage/cost bridge

## Goal

Accumulate token counts and costs from the cognitive loop and push usage_update notifications to the editor.

## Target files

- `crates/roko-acp/src/bridge_usage.rs` — Usage bridge

## Implementation details

### AcpUsageBridge struct

```rust
pub struct AcpUsageBridge {
    /// Accumulated tokens this session
    total_input_tokens: u64,
    total_output_tokens: u64,
    total_thought_tokens: u64,
    total_cached_read_tokens: u64,
    total_cached_write_tokens: u64,
    /// Accumulated cost
    total_cost_usd: f64,
    /// Context window size (for usage_update.size)
    context_window_size: u64,
}
```

### Methods

1. **`new(context_window_size: u64)`** — Initialize with zero counters

2. **`record_usage(&mut self, usage: &UsageInfo, cost: Option<f64>)`** — Add to running totals

3. **`usage_notification(&self) -> SessionUpdate`** — Build `UsageUpdate` notification
   ```rust
   SessionUpdate::UsageUpdate {
       used: self.total_tokens(),
       size: self.context_window_size,
       cost: Some(CostInfo {
           amount: self.total_cost_usd,
           currency: "USD".into(),
       }),
   }
   ```

4. **`total_tokens(&self) -> u64`** — Sum of all token types

5. **`build_usage_info(&self) -> UsageInfo`** — Build UsageInfo for SessionPromptResult

6. **`context_utilization(&self) -> f64`** — `total_tokens() / context_window_size` as percentage

7. **`should_warn_context(&self) -> Option<ContextWarning>`** — Check utilization thresholds:
   - <75%: None
   - 75–90%: Some(SuggestManagement)
   - 90–95%: Some(RecommendNewSession)
   - >95%: Some(WarnNextMayFail)

### ContextWarning enum

```rust
pub enum ContextWarning {
    SuggestManagement,
    RecommendNewSession,
    WarnNextMayFail,
}
```

## Verification

```bash
cargo check -p roko-acp
cargo clippy -p roko-acp --no-deps -- -D warnings
```

## Done when

- Usage tracking accumulates correctly
- Usage notifications have correct format
- Context window warnings at correct thresholds
- Cost tracking in USD

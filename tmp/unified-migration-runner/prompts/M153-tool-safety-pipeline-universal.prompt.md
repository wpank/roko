# M153 — Wire Universal Tool Safety Pipeline

## Objective
Wire the safety hook Pipeline universally so that ALL tool execution paths route through the same safety checks. Currently, the safety hooks (PolicyCage, AllowlistGuard, SpendingLimiter, RateLimiter, ResultFilter) exist in `roko-agent/src/safety/` but some execution paths — particularly subprocess/specialty branches in orchestrate.rs — bypass them. Create a single `execute_tool_safe()` entry point that is the ONLY path to tool execution, eliminating the critical integration gap.

## Scope
- Crates: `roko-agent`, `roko-cli`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/mod.rs` (unified entry point)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/hooks.rs` (pipeline assembly)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (wire all paths)
- Depth doc: `tmp/unified-depth/13-builtin-catalog/16-critical-integration-gap.md`

## Steps
1. Read existing safety hooks and their current wiring:
   ```bash
   grep -n 'pub struct\|pub trait\|SafetyHook\|impl SafetyHook' /Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/mod.rs | head -15
   grep -n 'pub struct\|SafetyHook' /Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/hooks.rs | head -15
   ```

2. Find all tool execution paths in orchestrate.rs:
   ```bash
   grep -n 'execute\|dispatch.*tool\|tool.*call\|subprocess\|Command::new\|run_tool' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs | head -20
   ```

3. Read existing safety pipeline components:
   ```bash
   grep -n 'AllowlistGuard\|SpendingLimiter\|RateLimiter\|ResultFilter\|PolicyCage' /Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/ -r --include='*.rs' | grep 'pub struct' | head -10
   ```

4. Create `SafetyPipeline` struct that chains all hooks:
   ```rust
   /// Universal safety pipeline for tool execution.
   ///
   /// ALL tool calls MUST route through this pipeline. No exceptions.
   /// Order: PolicyCage → AllowlistGuard → SpendingLimiter → RateLimiter → [execute] → ResultFilter
   pub struct SafetyPipeline {
       policy_cage: PolicyCage,
       allowlist: AllowlistGuard,
       spending_limiter: SpendingLimiter,
       rate_limiter: RateLimiter,
       result_filter: ResultFilter,
   }
   ```

5. Implement `execute_tool_safe()`:
   ```rust
   impl SafetyPipeline {
       /// Execute a tool call through the full safety pipeline.
       ///
       /// This is the ONLY sanctioned path to tool execution.
       /// Returns `Err(SafetyDenied)` if any pre-check fails.
       pub async fn execute_tool_safe(
           &self,
           tool_name: &str,
           args: &serde_json::Value,
           context: &SafetyContext,
           executor: &dyn ToolExecutor,
       ) -> Result<ToolResult, SafetyError> {
           // Pre-execution checks (in order)
           self.policy_cage.check(tool_name, args, context)?;
           self.allowlist.check(tool_name, args, context)?;
           self.spending_limiter.check(tool_name, args, context)?;
           self.rate_limiter.check(tool_name, args, context)?;

           // Execute
           let result = executor.execute(tool_name, args).await?;

           // Post-execution filter
           let filtered = self.result_filter.filter(tool_name, result, context)?;
           Ok(filtered)
       }
   }
   ```

6. Find and replace all direct tool execution paths in orchestrate.rs:
   ```bash
   grep -n 'tool.*exec\|dispatch.*tool\|run_command\|subprocess' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs | head -15
   ```
   Route each through `safety_pipeline.execute_tool_safe()`.

7. Add builder pattern for SafetyPipeline configuration:
   ```rust
   impl SafetyPipeline {
       pub fn builder() -> SafetyPipelineBuilder { ... }
       pub fn permissive() -> Self { ... } // For testing
       pub fn from_config(config: &SafetyConfig) -> Self { ... }
   }
   ```

8. Write tests:
   - Pipeline denies tool not in allowlist
   - Pipeline denies when spending limit exceeded
   - Pipeline denies when rate limit exceeded
   - ResultFilter redacts sensitive output
   - `execute_tool_safe` calls executor only if all pre-checks pass

## Verification
```bash
cargo check -p roko-agent
cargo clippy -p roko-agent --no-deps -- -D warnings
cargo test -p roko-agent -- safety
cargo check -p roko-cli
```

## What NOT to do
- Do NOT remove individual safety hook files — the pipeline composes them
- Do NOT add new safety checks in this batch — only wire existing ones universally
- Do NOT make the pipeline optional — it must be the ONLY path to execution
- Do NOT modify tool implementations — only the dispatch path changes
- Do NOT break the existing SafetyHook trait interface

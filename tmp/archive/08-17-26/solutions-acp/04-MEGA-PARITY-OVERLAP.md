# ACP ↔ mega-parity Overlap Analysis

## mega-parity Runner Overview

The mega-parity runner has 123 batches across 7 sub-runners. Here's what overlaps with ACP needs:

## Direct Overlaps (ACP benefits for free)

### Runner 2: Execution Contract
- **What it does:** Ensures all dispatch paths emit episodes, track costs, report outcomes
- **ACP benefit:** If it creates a shared `FeedbackService` or standardizes episode emission,
  ACP can call the same interface
- **Action needed:** After mega-parity runs, check if episode emission is now a reusable service
  or still inline in orchestrate.rs. If inline, extract.

### Runner 3: Model Selection
- **What it does:** Wires CascadeRouter properly, ensures learning from all dispatch outcomes
- **ACP benefit:** If CascadeRouter becomes a standalone callable service, ACP can query it
  for model suggestions and report outcomes
- **Action needed:** After mega-parity, add ACP calls to CascadeRouter

### Runner 4: Safety
- **What it does:** Enforces AgentContract, pre/post checks across all dispatch paths
- **ACP benefit:** If safety becomes a gate that any runtime can invoke, ACP wires in
- **Action needed:** After mega-parity, call safety check before ACP dispatch

### Runner 5: Dispatch
- **What it does:** Standardizes how agents are dispatched (through roko-agent properly)
- **ACP benefit:** ACP's raw `Command::new("claude")` calls should go through the standard dispatcher
- **Action needed:** This is the KEY overlap. If mega-parity standardizes dispatch into a
  callable service, ACP switches from raw subprocess to the service.

## Indirect Overlaps

### Runner 6: Projection
- Likely about progress/cost estimation
- ACP's dead `total_cost_usd` field could benefit from whatever cost tracking is standardized

### Runner 7: Mori Polish
- UX polish and behavior matching
- May touch ACP if it covers editor integration behaviors

## What mega-parity DOES NOT Cover (ACP-specific)

These are ACP-only concerns that need their own batches:

| Gap | Why mega-parity won't cover it |
|-----|-------------------------------|
| Static system prompts → 9-layer builder | ACP-specific wiring (which mode/role params to pass) |
| MCP tool routing in pipeline | ACP's subprocess model is unique |
| Session concurrency (RwLock) | Only affects ACP's SessionManager |
| Conversation history accumulation | ACP's multi-turn model is different from orchestrate.rs |
| File change notifications | ACP protocol-specific (session/update) |
| Missing slash commands (4) | Trivial, ACP-only |
| Pipeline provider fallback | ACP runner.rs uses raw CLI, needs roko-agent |
| Budget per-session | ACP session config → cost enforcement |

## Recommended Strategy

### Phase 1: Let mega-parity run first
It will standardize shared services (feedback, dispatch, safety, routing).
ACP then wires into those services.

### Phase 2: Bundle ACP integration batches INTO mega-parity
Add ~8-10 batches to mega-parity that wire ACP to the shared services:

| Batch | What | Deps |
|-------|------|------|
| ACP-W01 | Add roko-compose dep, replace static prompts with builder | Runner 5 done |
| ACP-W02 | Add roko-learn dep, emit episodes from bridge_events | Runner 2 done |
| ACP-W03 | Add CascadeRouter calls (suggest + record) | Runner 3 done |
| ACP-W04 | Wire safety contract check before dispatch | Runner 4 done |
| ACP-W05 | Route pipeline agents through roko-agent dispatcher | Runner 5 done |
| ACP-W06 | Wire cost tracking (accumulate from provider responses) | Runner 6 done |
| ACP-W07 | Add session concurrency (Arc<RwLock>) | Independent |
| ACP-W08 | File change notifications via git diff | Independent |
| ACP-W09 | Conversation history accumulation + context injection | Independent |
| ACP-W10 | Missing slash commands + integration tests | Independent |

### Phase 3: Verify integration
- Run `roko acp` in Zed
- Execute a Standard workflow
- Verify episodes appear in `.roko/episodes.jsonl`
- Verify CascadeRouter updates in `.roko/learn/cascade-router.json`
- Verify system prompt includes 9 layers (not static string)

## Key Insight

**The dependency direction is clear:**
1. mega-parity standardizes shared services (Runners 2-5)
2. ACP batches wire into those services (ACP-W01 through ACP-W10)
3. ACP-W01–W06 MUST run after their corresponding mega-parity runners
4. ACP-W07–W10 are independent and can run in parallel with anything

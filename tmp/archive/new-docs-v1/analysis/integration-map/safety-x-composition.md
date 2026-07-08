---
title: "Safety × Composition"
section: analysis
subsection: integration-map
id: im-safety-x-composition
source: 24-cross-section-integration-map.md (§6.1 M13, §3.3)
missing-integration: M13
tier: 3
tags: [safety, composition, constraints, capability-tokens, context-injection, safe-behavior-prompting]
---

# Safety × Composition

**Direction**: 11-Safety → 03-Composition (active safety constraints in system prompt)  
**Status**: **Missing (M13)** — Tier 3, ~80 LOC. SafetyLayer and ToolDispatcher are built but never invoked from `orchestrate.rs`.  
**Interface**: `roko-orchestrator::safety` capability context → `roko-compose::SystemPromptBuilder`

## What Flows

The safety subsystem knows what capabilities a given agent has (capability tokens), what actions are restricted (BashPolicy, GitPolicy, NetworkPolicy, PathPolicy), and what was recently scrubbed (ScrubPolicy). This information should be reflected in the system prompt so agents understand their constraints before attempting prohibited actions.

| Signal | From | To | Status |
|---|---|---|---|
| Active capability token set | `SafetyLayer::capability_tokens` | SystemPromptBuilder "safety constraints" section | **Missing** (M13) |
| Active policies (Bash/Git/Network/Path restrictions) | `SafetyLayer` config | System prompt warning section | **Missing** |
| Recent scrub actions | `ScrubPolicy::recent_actions` | System prompt context | **Missing** |
| `Kind::Warning` safety events | Safety audit log | `NeuroStore` → composition via M5 | **Indirect** (via M5) |

## Wiring Recipe

```rust
// In SystemPromptBuilder::build(), inject safety context:
let safety_context = safety_layer.current_context();

if !safety_context.restricted_operations.is_empty() {
    builder.add_section("safety_constraints", Section {
        content: render_safety_constraints(&safety_context),
        priority: Priority::Critical,  // Highest priority — always included
        max_tokens: 500,
    });
}

fn render_safety_constraints(ctx: &SafetyContext) -> String {
    let restrictions: Vec<String> = ctx.restricted_operations.iter()
        .map(|op| format!("- Cannot {op}"))
        .collect();
    format!(
        "ACTIVE CONSTRAINTS:\n{}\n\nCapabilities: {:?}",
        restrictions.join("\n"),
        ctx.capability_tokens
    )
}
```

Estimated LOC: ~80.

**Critical context**: This integration addresses a wiring gap, not a code gap. The safety layer (6 guards: BashPolicy, GitPolicy, NetworkPolicy, PathPolicy, ScrubPolicy, RateLimiter) is fully built and tested but never invoked from `orchestrate.rs`. M13 adds the composition-side reflection of safety constraints; the parallel gap is wiring ToolDispatcher into the execution path (Readiness Audit G1, G13).

## Invariants of the Interaction

1. Safety constraints section is always included when there are active restrictions — never omitted due to token budget.
2. Safety context is read-only from the composition perspective.
3. The rendered constraints match the actual runtime capability tokens — no stale or incorrect constraint declarations.
4. Capability tokens are session-scoped — they are re-read fresh each prompt build.

## Failure Modes

| Failure | Consequence | Detection |
|---|---|---|
| SafetyLayer not initialized | No constraint injection; agents unaware of restrictions | Log warn; safety section omitted (not silently misconfigured) |
| Stale capability tokens | Prompt declares wrong constraints | Invalidate tokens on config change |
| Agent ignores injected constraints | Safety guards blocked at execution (correct behavior) | Track policy_violation rate; alert on sustained violations |

## Open Questions

1. Should the capability token list be rendered verbosely (each token described) or just categorically ("Can: bash commands, git operations")?
2. Should scrub events (recently sanitized data) be listed to help agents avoid referencing scrubbed content?
3. How does this interact with M15 (AntiKnowledge)? Safety constraints are rule-based; AntiKnowledge is empirical — both belong in the prompt but are distinct.

## Cross-References

- Execution gap: [agents-x-verification.md](./agents-x-verification.md) — Wired connection; safety layer should also gate execution (G1, G13)
- Anti-knowledge complement: [anti-knowledge-x-composition.md](./anti-knowledge-x-composition.md) — M15 (empirical "do not" vs. policy "cannot")
- Readiness audit: [RA-11: Safety](../readiness-audit/subsystem-safety.md), [RA-03: Composition](../readiness-audit/subsystem-composition.md)
- Critical audit gap G1 and G13 must be resolved alongside M13

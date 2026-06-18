# Post-Parity Review Vetoes

## Rejection Criteria

A batch is REJECTED if any of the following are true:

1. **Creates a new reqwest::Client.** All HTTP goes through the shared client from PA.
2. **Adds a new dispatch path.** Chat dispatch has ONE path through the adapter layer.
3. **Stores session state outside ChatAgentSession.** One session struct, one owner.
4. **Slash command confirms without applying.** If it says "set", it must be set.
5. **Sets dangerously_skip_permissions to true** without explicit user flag.
6. **Touches orchestrate.rs** beyond PF deprecation attr or PG efficiency drain.
7. **Creates new public API surface** that isn't called from the runtime.
8. **Adds TODO/FIXME comments** instead of doing the work.
9. **Changes function signatures** of public traits without updating all call sites.
10. **Introduces new dependencies** that aren't strictly necessary.

## Batch Size Rules

| Rule | Limit | Remediation |
|---|---|---|
| Modified files per batch | ≤ 5 | Split into sub-batches |
| Net new lines per batch | ≤ 300 | Split into sub-batches |
| New public functions | ≤ 3 | Check if existing function can be extended |
| New structs/enums | ≤ 2 | Check if existing type can be reused |

## Required Proof Shape

Each batch prompt must include:
- Exact file:line references for code being changed
- Before/after code snippets showing the wiring
- Write scope (files to modify)
- Read-only context (files to reference)
- Acceptance criteria (observable behavior change)

Negative proof — these are NOT acceptable:
- "Code compiles" (we don't compile in batch runners)
- "Tests pass" (validation is separate)
- "Looks correct" (show the wiring)
- "Similar to existing pattern" (cite the specific pattern)

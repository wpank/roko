# Agent Failure Patterns

Reusable lessons for future Codex/Claude runner batches. This is a compact prompt
and review companion to `ANTI-PATTERNS-V2.md`.

## Core Lesson

Agents did not mostly fail by writing obviously bad code. They failed by making
locally reasonable changes that preserved or recreated the broken system shape:
duplicate dispatch paths, optional core services, fake success states, and
"wired" claims without product-path proof.

Use this doc when preparing runner prompts, reviewing batch output, or designing
fitness checks.

## Repeated Agent Habits

### 1. Local surface patch instead of shared-layer fix

**What agents did:** Fixed ACP, chat, serve, or demo symptoms inside that surface.

**Why it was wrong:** The same bug class stayed alive in every other surface.

**Prompt guardrail:** "Before editing, name the owning layer. If this capability
belongs in provider/gate/prompt/config/runtime, change that layer first."

**Review question:** Did the patch delete a duplicate path or only add another one?

---

### 2. "Built" treated as "wired"

**What agents did:** Added structs, services, loggers, adapters, or tests, then
reported completion.

**Why it was wrong:** Live entry points never called the new code.

**Prompt guardrail:** "A feature is not wired until a live entry point exercises it
end to end and the old bypass path is removed or blocked."

**Review question:** Which command or product surface proves this path now runs?

---

### 3. New abstraction without retiring the old path

**What agents did:** Added `ModelCallService`, `WorkflowEngine`, `GateService`,
`FeedbackService`, or `PromptAssemblyService` while keeping legacy fallbacks alive.

**Why it was wrong:** The codebase gained a better path but did not lose the bad
path, so later work kept landing in both.

**Prompt guardrail:** "Every replacement must include a retirement plan: delete,
feature-gate, or hard-error on the old path. No silent fallback to legacy."

**Review question:** Can production still reach the replaced path?

---

### 4. Optional core services

**What agents did:** Used `Option`, default no-op sinks, or absent config to avoid
threading feedback, safety, budget, gateway events, or MCP through constructors.

**Why it was wrong:** Production silently ran without core product behavior.

**Prompt guardrail:** "Core services are mandatory in production. Tests may use
explicit no-op implementations named `Noop...`, but absence must not mean no-op."

**Review question:** What happens if the service is not supplied in production?

---

### 5. Fake success states

**What agents did:** Encoded no-op, skipped, unsupported, or failed states inside
success variants or strings.

**Examples:** `CommitDone { hash: "noop" }`, skipped gates as failed/pass-like
events, provider error text followed by normal completion, placeholder transcripts.

**Prompt guardrail:** "Do not encode new states in strings, booleans, or sentinel
values. Add a typed enum variant."

**Review question:** Does any caller need to special-case a string to know truth?

---

### 6. String/debug output used as a contract

**What agents did:** Parsed terminal output, debug strings, rendered JSON, or
human-readable gate text to make decisions.

**Why it was wrong:** Display text is not stable. It also hides missing typed data.

**Prompt guardrail:** "If code branches on rendered text, stop and add a typed
event/result field."

**Review question:** Is this logic matching a schema or a display string?

---

### 7. Unknown collapsed to zero

**What agents did:** Used `unwrap_or(0)`, `Usage::default()`, empty strings, or
default contexts when provider/runtime data was missing.

**Why it was wrong:** Learning and reporting treated missing data as real cheap,
fast, zero-token behavior.

**Prompt guardrail:** "Unknown stays optional until display. Never normalize missing
telemetry to zero at collection time."

**Review question:** Can the downstream learner distinguish missing from zero?

---

### 8. Markdown rules trusted as enforcement

**What agents did:** Read context-pack rules and still violated them because no test,
lint, sandbox, or ownership check enforced them.

**Why it was wrong:** Long-running parallel agents drift. Instructions are not a
control plane.

**Prompt guardrail:** "Every critical rule must become a check in the same batch."

**Review question:** What command fails if this rule is violated again?

---

### 9. Status docs updated beyond runtime reality

**What agents did:** Marked anti-patterns "resolved" when a replacement abstraction
existed, not when all product paths used it.

**Why it was wrong:** Future agents trusted stale status and stopped looking for
bypasses.

**Prompt guardrail:** "Use coverage statuses: Built, WiredInOnePath,
LiveInAllProductPaths, OldPathRetired, ProvenByE2E."

**Review question:** Does the status describe intent or observed runtime coverage?

---

### 10. Avoiding deletion because it felt risky

**What agents did:** Left old code, duplicate helpers, and compatibility paths in
place to reduce immediate risk.

**Why it was wrong:** The system kept all the old ambiguity plus the new code.

**Prompt guardrail:** "If you cannot delete the old path, add a failing guard,
feature flag, or issue that blocks claiming convergence."

**Review question:** What prevents future code from using the old path?

## Reusable Prompt Block

Paste this into future runner prompts when the task touches architecture:

```text
Before editing:
1. Name the owning layer for the behavior you are changing.
2. Find existing implementations of the same behavior.
3. State which old path will be deleted, blocked, or migrated.

While editing:
- Do not add provider/gate/prompt/config/runtime logic to a surface crate if a
  shared layer should own it.
- Do not encode state in strings, booleans, empty fields, or sentinel values.
- Do not make core services optional in production.
- Do not collapse unknown telemetry to zero.
- Do not parse terminal/debug/display text as a contract.

Before claiming done:
1. Show the live entry point that exercises the new path.
2. Show the check that prevents the old anti-pattern from returning.
3. If old code remains reachable, mark the work as partial, not complete.
```

## Review Checklist

Use this to review agent diffs:

| Check | Pass condition |
|---|---|
| Owner named | The diff changes the shared owner, not only the symptom surface |
| Duplicate removed | At least one old implementation is deleted, blocked, or made unreachable |
| Product path proven | A live command/path exercises the behavior |
| Typed contract | New states use enums/structs, not strings/booleans/sentinels |
| Failure explicit | Unsupported/missing config fails loudly or returns a typed status |
| Services mandatory | Feedback/safety/budget/config are required in production constructors |
| Unknown preserved | Missing usage/cost/context remains optional |
| Rule executable | A test/lint/grep/sandbox catches recurrence |
| Docs honest | Status uses coverage language, not vague "resolved" |

## High-Value Fitness Checks To Add

These checks would have caught many regressions:

```bash
# No raw provider HTTP outside provider adapters.
rg 'reqwest::Client::new\(\)' crates/ --type rust | grep -v test | grep -v 'roko-agent/src/provider'

# No shared dangerous permission bypass.
rg 'dangerously_skip_permissions\\s*[:=]\\s*true' crates/ roko.toml --type-add 'toml:*.toml' | grep -v test

# No success-noop sentinel.
rg '"noop"|success_noop|CommitDone' crates/ --type rust

# No path-based shared modules.
rg '#\\[path\\s*=' crates/ --type rust

# No obvious unknown-to-zero telemetry conversions.
rg 'unwrap_or\\(0\\)|Usage::default\\(\\)|cost_usd: 0\\.0|input_tokens: 0|output_tokens: 0' crates/ --type rust

# No production let-underscore swallowing without review.
rg 'let _ =' crates/ --type rust | grep -v test
```

## Definition Of Done For Redesign Work

A redesign is done only when all five are true:

1. One owner: there is one canonical module/service for the behavior.
2. One path: production entry points route through that owner.
3. Typed truth: states and outcomes are represented by types, not strings.
4. Old path retired: bypasses are deleted, hard-blocked, or explicitly legacy-only.
5. Recurrence check: CI or a local fitness command catches the anti-pattern.

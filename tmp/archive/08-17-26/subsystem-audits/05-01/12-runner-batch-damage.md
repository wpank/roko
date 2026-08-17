# 12 — Runner Batch Damage Assessment

Analysis of how the 5 runner batches (661 total) affected code quality.

## Runner Overview

| Runner | Batches | Agents | Period | Goal |
|--------|---------|--------|--------|------|
| arch | 16 | ~16 | Apr 28 | Create service trait abstractions |
| converge | 87 | ~40 | Apr 28 | Wire services, retire legacy |
| converge-followup | 33 | ~20 | Apr 28 | Fix contract issues from converge |
| mega-parity | 195 | ~40 | Apr 29 | Complete mori parity checklist |
| post-parity | 330 | ~20 | Apr 30 | Product maturity + GTM |

**Total: 661 batches, each producing a branch + cherry-pick. ~40 concurrent codex agents at peak.**

---

## Systemic Problems From Batch Execution

### 1. Each batch optimized locally, nobody optimized globally

Each codex agent received a focused prompt like "wire CascadeRouter into dispatch" or "add daimon modulation to agent calls." The agent would:
1. Find the target function (usually `dispatch_agent_with` in orchestrate.rs)
2. Add 50-200 lines of new logic
3. Pass compile + test
4. Create a branch

Nobody's prompt said "refactor this 2000-line function into smaller pieces." The anti-pattern checks looked for:
- No trait duplication
- No dead imports
- No new `unwrap()` in non-test code

But NOT for:
- Function length > N lines
- Parameter count > N
- Code duplication within a file
- Responsibility sprawl

### 2. Cherry-pick merging created accidental dead code

When batches from the same wave modified the same function, cherry-picks sometimes created:
- Duplicate variable bindings (e.g., `_history_context` computed but unused after another batch changed the dispatch path)
- Overlapping match arms where one is dead
- Config fields added by one batch but only read by a different batch that landed later

### 3. "Wired" claims inflated

The runner results marked tasks as "done" when code compiled and tests passed. CLAUDE.md was updated to say "Wired" for each component. But "compiles and tests pass" != "works end-to-end":

| Component | Claimed | Actual |
|-----------|---------|--------|
| CascadeRouter | "Wired" | Returns default slug because RoutingContext never populated |
| Safety contracts | "Wired" | Falls back to permissive() on every role |
| Permissions | "Wired" | Hardcoded `dangerously_skip_permissions = true` in 8 sites |
| LLM judge gate | "Wired" | `StubJudgeGate` always fails; skipped at runtime |
| Dream consolidation | "Wired" | Triggers written, no consumer exists |
| Efficiency feedback | "Wired" | Events recorded with empty model/provider fields |
| Playbook store | "Wired" | Queried but results not fed into prompts |

### 4. Duct-tape patterns from conflict resolution

When cherry-picks conflicted, the merge resolution often chose the pragmatic fix:
- Add `#[allow(dead_code)]` instead of removing unused code
- Rename to `_unused_var` instead of fixing the caller
- Add `unwrap_or_default()` instead of propagating errors
- Use `.ok()` to silence Result warnings instead of handling them

### 5. Test coverage doesn't validate integration

Each batch added unit tests for its specific change. But the tests use mocks:
- `HallucinationDetector::permissive()` in all safety tests
- `AgentContract::permissive()` as test default
- Stub providers that return canned responses
- In-memory stores instead of filesystem

No integration test validates the full path: prompt → model selection → dispatch → gate → episode → feedback. The individual pieces pass, but the pipeline has gaps.

---

## Damage Inventory

### Files most bloated by batch accumulation

| File | Lines | Functions | Functions >300L |
|------|-------|-----------|-----------------|
| `orchestrate.rs` | 22,635 | 138 | 14 |
| `bridge_events.rs` | ~3,300 | ~40 | 3 |
| `session.rs` (ACP) | ~1,100 | ~30 | 2 |
| `runner/types.rs` | ~1,500 | ~25 | 4 |

### Most-modified functions across batches

| Function | Modified by | Current size |
|----------|-----------|-------------|
| `dispatch_agent_with` | All 5 runners | 2,059 lines |
| `build_context_assembler_sections` | arch, converge, mega-parity | 736 lines |
| `attempt_replan` | converge, mega-parity | 735 lines |
| `dispatch_action` | converge, post-parity | 699 lines |
| `handle_implementing_single` | converge, mega-parity | 431 lines |

---

## What Should Have Been Done Differently

### Before running 661 batches:
1. **Define module boundaries** — orchestrate.rs should have been split into `dispatch/`, `context/`, `feedback/`, `gates/` modules first
2. **Set code quality gates** — function length limits, parameter count limits, duplication detection
3. **Design the data flow** — map which data flows from recording → consumption before implementing either end
4. **Use integration tests** — require each batch to prove its change works through the full pipeline, not just compiles

### After the damage:
1. **Extract dispatch** — the 2059-line function needs to become 4-5 functions in a `dispatch/` module
2. **Wire feedback loops** — close the "record → use" gaps for efficiency, playbooks, routing
3. **Flip safety defaults** — PE_02 needs to happen before any production use
4. **Clean dead code** — remove the 800 LOC of unexported learning modules, dead stream types, unused variables
5. **Add architectural fitness functions** — CI checks for function length, file length, parameter count

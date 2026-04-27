# Phase 5: Architecture Redesign Pass

You are doing a fresh architecture review of Roko, informed by the complete converged spec and synthesis. Your job is to identify what should CHANGE — not just document what is, but propose what should be different.

## Context

All previous phases have produced:
- Per-topic converged docs (Phase 2)
- Cross-topic synthesis with synergies, redundancies, and gaps (Phase 3)
- Dogfooded PRDs and task files (Phase 4)

Now step back and think architecturally. With everything visible in one place:
- What's over-engineered?
- What's under-engineered?
- What new capabilities emerge from combining subsystems differently?
- What should be cut entirely?
- What's the simplest version that captures the core value?

## Sources

Read:
- All files in `/Users/will/dev/nunchi/roko/roko/tmp/doc-convergence/output/`
- `/Users/will/dev/nunchi/roko/roko/tmp/doc-convergence/output/00-SYNTHESIS.md`
- `/Users/will/dev/nunchi/roko/roko/docs/v2/00-INDEX.md` (the current v2 "one rule" and design principles)
- `/Users/will/dev/nunchi/roko/roko/CLAUDE.md` (what's actually shipping today)

## Output

Write to: `/Users/will/dev/nunchi/roko/roko/tmp/doc-convergence/output/00-REDESIGN.md`

### Structure:

```markdown
# Architecture Redesign — v3 Proposals

Generated: {date}

## 0. Design Audit

### What the architecture gets RIGHT
[Things that should be preserved. The genuine insights.]

### What the architecture gets WRONG
[Over-engineering, unnecessary complexity, concepts that don't earn their keep.]

### The gap between spec and reality
[v2 describes a system with 28 subsystems. The running code has maybe 8.
Is the spec aspirational-good or aspirational-delusional?
Which of the 20 unimplemented subsystems are actually needed?]

## 1. Proposed Changes

For each proposed change:

### Change N: {title}

**What**: [What to change]
**Why**: [Why this improves things — be specific about the problem it solves]
**Impact**: [Which topics/crates are affected]
**Effort**: [Small/Medium/Large]
**Risk**: [What could go wrong]
**Alternative**: [What you considered instead]

Categories to evaluate:

#### Cuts (what to remove or defer)
[Subsystems or features that add complexity without proportional value.
Be aggressive — what's the minimal viable architecture?]

#### Simplifications (what to make simpler)
[Where can complexity be reduced without losing capability?
Are there abstractions that don't justify themselves?]

#### New features (what emerges from the synthesis)
[Capabilities that become possible when you see all topics together.
Only propose features that are genuinely new, not rehashes of existing spec.]

#### Architectural changes (structural improvements)
[Changes to how crates/modules/traits are organized.
Merges, splits, new boundaries.]

#### Vocabulary changes (naming improvements)
[Terms that should change based on the convergence experience.
Is the Signal/Cell/Graph vocabulary actually helping?]

## 2. The Minimal Viable Architecture

[If you had to ship Roko with the fewest possible subsystems while
preserving the core value proposition ("agents that build themselves"),
what would you keep?]

### Must-have (self-hosting requires it)
| Subsystem | Why | Current Status |
|---|---|---|
| ... | ... | ... |

### Should-have (significantly better with it)
| Subsystem | Why | Current Status |
|---|---|---|
| ... | ... | ... |

### Nice-to-have (can add later)
| Subsystem | Why | Current Status |
|---|---|---|
| ... | ... | ... |

### Cut (remove from spec entirely)
| Subsystem | Why Cut | What Replaces It |
|---|---|---|
| ... | ... | ... |

## 3. New Capabilities

[Features or capabilities that emerge from the convergence that nobody
has documented before. Be specific about HOW they emerge — which
subsystems combine to create them?]

### Capability N: {title}
**Emerges from**: [which topics/subsystems combining]
**What it enables**: [concrete use case]
**Implementation sketch**: [high-level approach]

## 4. Revised Crate Map

[Based on all proposed changes, what should the crate structure look like?
Which crates merge? Which split? Which are new? Which are deleted?]

```
roko-core      →  [what changes]
roko-agent     →  [what changes]
roko-compose   →  [what changes]
...
```

## 5. Decision Points

[Things that require Will's input before proceeding.
Each should have 2-3 concrete options with clear tradeoffs.]

### Decision N: {title}
**Context**: [why this needs a decision]
**Option A**: ... [tradeoff]
**Option B**: ... [tradeoff]
**Recommendation**: [if you have one, say it]
```

## Instructions

1. Read everything from Phase 2 and Phase 3 first
2. Think like a principal engineer doing a design review, not a documentarian
3. Be honest about over-engineering — this codebase has 18 crates and ~177K LOC but many are barely used
4. Propose cuts aggressively — it's easier to add back than to maintain unused code
5. New features should be genuinely emergent, not "it would be nice to have X"
6. The minimal viable architecture section should be ruthlessly honest
7. Every proposal should have a clear "why" tied to a real problem, not theoretical cleanliness

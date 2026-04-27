# Phase 3: Cross-Topic Synthesis

You are reading all 28+ converged topic documents and producing a synthesis that identifies patterns, synergies, redundancies, and a revised roadmap.

## Context

Phase 2 produced one converged document per topic in `/Users/will/dev/nunchi/roko/roko/tmp/doc-convergence/output/`. Each document has: Spec, Status, Plan, and Discoveries sections.

Your job is to read ALL of them and think across topics.

## Sources

Read every `*.md` file in `/Users/will/dev/nunchi/roko/roko/tmp/doc-convergence/output/` EXCEPT files starting with `00-`.

Also read:
- `/Users/will/dev/nunchi/roko/roko/tmp/doc-convergence/status/MATRIX.md` — the topic matrix from Phase 1
- `/Users/will/dev/nunchi/roko/roko/CLAUDE.md` — current project state and priorities

## Output

Write to: `/Users/will/dev/nunchi/roko/roko/tmp/doc-convergence/output/00-SYNTHESIS.md`

### Structure:

```markdown
# Cross-Topic Synthesis

Generated: {date}

## 1. Architecture Coherence

### Does the converged spec tell a coherent story?
[After reading all 28 topics together, is the "everything is a Graph of Cells"
model consistently applied? Where does it break down? Where is it most elegant?]

### Vocabulary consistency
[Are there remaining inconsistencies across topics? Terms used differently?]

### Pattern coverage
[The 4 universal patterns (Pipeline, Loop, Functor, Space) — which topics
use them well? Which topics describe systems that SHOULD use them but don't?]

## 2. Implementation Reality

### Overall status snapshot
| Status | Topics | Percentage |
|---|---|---|
| DONE | [list] | NN% |
| PARTIAL | [list] | NN% |
| NOT STARTED | [list] | NN% |
| N/A | [list] | NN% |

### The actual critical path
[Based on real code status across ALL topics, what is the true shortest path
to self-hosting? Which topics block other topics?]

### Dead code audit
[Code that exists across topics but is never called. Candidates for removal.]

### Duplicate implementations
[Same functionality implemented in multiple crates. Which should be canonical?]

## 3. Synergies Discovered

[Things visible ONLY when you see all topics together. Examples:]

### Cross-topic connections nobody documented
[Topic X's output is Topic Y's input, but neither doc mentions it]

### Features that emerge from combining subsystems
[If you wire X + Y + Z together, you get capability W for free]

### Shared infrastructure opportunities
[Multiple topics build similar things independently — could share]

## 4. Redundancies and Cuts

### Subsystems that overlap
[Where do topics duplicate each other's work?]

### Features that should be cut
[Things that are specced but add complexity without proportional value]

### Simplification opportunities
[Where can the architecture be made simpler without losing capability?]

## 5. Missing Pieces

### Gaps no doc covers
[Things that SHOULD exist based on the architecture but no topic mentions]

### Integration gaps
[Topics that should connect but don't specify how]

### Operational gaps
[Things needed to actually run this in production that nobody specced]

## 6. Revised Roadmap

### Priority reordering
[Based on the synthesis, should the implementation order change?
Consider: what unblocks the most other things? What has the highest value
per effort? What's the new critical path?]

### Phase 1: Self-hosting bootstrap (must-do)
[Tasks across topics needed to reach full self-hosting]

### Phase 2: Core intelligence (high-value)
[Tasks that make agents actually smart: learning, memory, routing]

### Phase 3: Economy and chain (market-facing)
[ISFR, registries, marketplace, arenas]

### Phase 4: Polish and scale
[UX, deployment, multi-chain, extensibility]

### Dependency graph
[Which phases/tasks block which others? Mermaid DAG if helpful]

## 7. Design Questions for Human Decision

[Questions that emerged from the synthesis that require Will's input.
Each should have clear options with tradeoffs.]

1. **Question**: ...
   - Option A: ... (tradeoff: ...)
   - Option B: ... (tradeoff: ...)

## 8. Topic Interaction Matrix

[A matrix showing which topics interact with which, and how.
Mark: produces→consumes, shares-types, extends, blocks, conflicts]
```

## Instructions

1. Read EVERY converged topic doc — do not skip any
2. Pay special attention to each topic's "Discoveries" section — that's where per-topic agents flagged cross-cutting concerns
3. Think about the system as a whole, not just individual topics
4. Be honest about what should be cut — complexity without value is harmful
5. The revised roadmap should be based on actual code status, not aspirational design
6. Design questions should be genuine — things where the synthesis reveals a real decision point

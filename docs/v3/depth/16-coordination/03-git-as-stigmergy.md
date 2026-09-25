# Depth: Git as Stigmergy

> Parent: [16-COORDINATION](../../16-COORDINATION.md) -- Section 1

---

## The Repository as a Stigmergic Environment

A Git repository is the most natural stigmergic environment for software
development agents. It satisfies all three conditions for stigmergy identified
by Grasse (1959) and formalized by Theraulaz & Bonabeau (1999):

| Condition | Git Implementation |
|-----------|-------------------|
| **Shared environment** | The repository (working tree + object store + refs) accessible to all agents |
| **Persistent modifications** | Commits persist indefinitely; branches create named trails |
| **Stimulus-response coupling** | Diff output, test results, linting warnings, and merge conflicts trigger agent actions |

Every agent working in a Roko-managed repository reads from and writes to this
shared environment. The repository state itself is the coordination medium.

---

## Sematectonic Stigmergy in Git

Structure-based coordination is pervasive in Git repositories. Code structure
guides agent behavior without explicit signaling.

### Code Structure as Signal

| Structural Feature | Information Conveyed | Agent Response |
|-------------------|---------------------|----------------|
| Module with no tests | "Testing gap" | Testing agent writes tests |
| Function with `TODO` comment | "Incomplete work" | Coding agent completes |
| Trait with one implementor | "Premature abstraction?" | Refactoring agent simplifies |
| `pub` function, no doc comment | "Missing documentation" | Documentation agent adds rustdoc |
| `unsafe` block without `SAFETY:` | "Unjustified unsafety" | Safety agent adds justification |
| Module with many imports | "High coupling" | Refactoring agent reduces deps |

Each structural feature is a "pheromone" that attracts specific agent
behaviors. The code itself recruits the right kind of work.

### File System Layout as Trail

```
crates/
  roko-core/          <- "kernel code lives here"
    src/
      lib.rs          <- "start reading here"
      engram.rs       <- "the Signal type"
      traits.rs       <- "the 12 kernel traits"
    Cargo.toml        <- "these are the dependencies"
  roko-agent/         <- "agent dispatch lives here"
  roko-gate/          <- "verification lives here"
```

An agent exploring the codebase follows these structural cues to orient
itself -- the same way an ant follows tunnel geometry to navigate the nest.

### Cargo.toml as Dependency Signal

- A crate depending on `roko-core` signals "this crate uses the kernel API."
- `[dev-dependencies]` on `proptest` signals "property-based testing is
  expected here."
- `#![forbid(unsafe_code)]` signals "no unsafe code -- find safe alternatives."
- Feature flags signal optional capabilities requiring conditional compilation.

---

## Marker-Based Stigmergy in Git

### Commit Messages as Trail Pheromones

Each commit message is a deliberate deposit of information:

```
feat(roko-agent): Wire CascadeRouter into dispatch pipeline

Connect the LinUCB-based model router to the agent dispatch path.
Models are selected per-request based on task complexity and
historical performance. Fallback to config default on cold start.
```

This functions as a trail pheromone:

1. **Type prefix** (`feat`) -> signals change type
2. **Scope** (`roko-agent`) -> signals which subsystem was modified
3. **Description** -> encodes intent and reasoning
4. **References** -> links to academic basis for the decision

### Branch Names as Scoped Pheromones

| Branch Pattern | Signal | Scope |
|---------------|--------|-------|
| `feat/wire-cascade-router` | "New feature in development" | Feature scope |
| `fix/gate-threshold-overflow` | "Bug being fixed" | Bug scope |
| `refactor/agent-dispatch-cleanup` | "Structural improvement" | Refactoring scope |
| `agent/task-42-implement-scorer` | "Agent working on task" | Task scope |

### CI/CD Status as Environmental Feedback

| CI Signal | Pheromone Equivalent | Intensity |
|----------|---------------------|-----------|
| All tests pass | `Opportunity` -- "safe to build on" | High |
| Clippy warnings | `Pattern` -- "code quality issue" | Medium |
| Test failure | `Threat` -- "regression detected" | High |
| Coverage decrease | `Anomaly` -- "testing gap widened" | Medium |
| Build failure | `Threat` -- "broken build" | Very High |

In Roko's gate pipeline, CI results are explicitly converted into scored
Signals that enter the stigmergic loop.

### Git Blame as Historical Pheromone Map

`git blame` provides a historical pheromone map:

- Recently modified lines have "fresh" pheromone (recent activity signal).
- Lines untouched for months have "decayed" pheromone (stable code).
- Lines modified by many agents have "mixed" pheromone (contested code).

This identifies **hot spots** (frequent modification), **cold spots**
(stable code), and **contested spots** (poorly abstracted, attracts repeated
changes).

---

## Multi-Agent Worktree Model

Roko's multi-agent coordination uses Git worktrees as isolated stigmergic
environments.

### Worktree as Private Pheromone Field

Each agent's worktree is a `Local(SubstrateId)` scope pheromone field. Changes
are visible only within the worktree until committed. This creates a natural
privacy boundary: work-in-progress modifications are "local pheromones" that
do not affect other agents.

### Merge Conflicts as Coordination Signals

```
<<<<<<< HEAD
fn process_signal(signal: &Signal) -> Score {
    self.scorer.score(signal)
=======
fn process_signal(signal: &Signal) -> Result<Score> {
    self.scorer.score(signal).map_err(|e| ScoringError::from(e))
>>>>>>> agent-42-add-error-handling
```

This conflict encodes information: two agents made different design decisions.
The resolution itself becomes a new structural signal guiding future agents.

### The Base + Overlay Pattern

For multi-agent code indexing, Roko uses base + overlay:

- **Base index**: Read-only snapshot at the branch point. Shared by all agents
  on the same plan.
- **Overlay index**: Per-agent additions from uncommitted changes. Invisible
  to other agents.

The base index is the shared environment (all agents read from it), and each
agent's overlay is a local modification that becomes visible only when committed.

### Declared Contracts

Agents working on related tasks can declare contracts -- explicit promises
about what their branch will provide when merged:

```toml
[contracts]
provides = ["trait ScorerV2", "fn score_with_context"]
requires = ["trait Substrate", "struct Signal"]
```

These function as explicit pheromone deposits: they signal what affordances
will become available after merge.

---

## Pheromone Traces in the Codebase

Roko coding agents deposit explicit stigmergic markers during development:

| Trace Type | Mechanism | Half-Life |
|-----------|-----------|-----------|
| Test coverage | Tests guide confidence | Permanent (sematectonic) |
| Documentation | Doc comments signal intent | Permanent (sematectonic) |
| Type signatures | Types constrain usage | Permanent (sematectonic) |
| Error types | Variants document failure modes | Permanent (sematectonic) |
| Commit messages | Trail markers in history | Permanent (marker) |
| Pheromone Signals | Explicit typed signals in Substrate | Configurable (marker) |

### The Coding Agent's Stigmergic Behavior

1. **Sense**: Read repository state (code, tests, docs, CI status).
2. **Act**: Modify code, write tests, add documentation.
3. **Deposit**: Commit with descriptive messages; optionally deposit explicit
   pheromone Signals.
4. **Signal**: Push the branch, triggering CI (environmental feedback).

The next agent to work in the same area encounters all traces and is guided
by them -- without any direct communication between agents.

---

## Stigmergic Workflow Example

```
Timeline:
---------------------------------------------------------------

T=0   Agent A reads failing test in CI (SENSES Threat pheromone)
T=1   Agent A investigates -> finds bug in scorer.rs
T=2   Agent A creates branch fix/scorer-nan-handling
T=3   Agent A commits fix:
      "fix(gate): Handle NaN scores in threshold comparison"
      (DEPOSITS marker-based pheromone: commit message)
T=4   Agent A pushes -> CI passes
      (ENVIRONMENTAL FEEDBACK: Threat removed, Opportunity deposited)

T=5   Agent B reads repository for model routing task
T=6   Agent B encounters Agent A's fix in recent commits
      (SENSES marker: "NaN handling added to scorer")
T=7   Agent B realizes its routing code should also handle NaN
T=8   Agent B adds NaN handling to cascade router
      (STIGMERGIC RESPONSE: A's trace guided B's work)

T=9   Agent C reviews merged code for documentation task
T=10  Agent C sees both NaN-handling implementations
      (SENSES sematectonic signal: pattern of NaN handling)
T=11  Agent C documents the NaN-handling convention
      (DEPOSITS sematectonic pheromone: documentation)
```

No agent communicated directly with any other. The repository was the sole
coordination medium.

---

## Git as Roko's Primary Substrate

| Substrate Operation | Git Implementation |
|--------------------|-------------------|
| `store(signal)` | `git add` + `git commit` (deposit a code modification) |
| `query(filter)` | `git log`, `git diff`, `git blame` (sense the environment) |
| `get(hash)` | `git show <hash>` (retrieve a specific modification) |
| `gc()` | `git gc` (compact the object store) |

The content-addressing property of Git (SHA-based hashes) aligns with
Signal's content-addressing property (`hash: [u8; 32]`). Both systems provide
tamper-evident, immutable records of modifications.

---

## References

- [Bolici et al. 2009] Scalability in OSS via stigmergy, *AMCIS*
- [Elliott 2006] Stigmergic Collaboration, University of Melbourne
- [Fowler 1999] *Refactoring*, Addison-Wesley
- [Grasse 1959] Termite mound stigmergy, *Insectes Sociaux*
- [Odling-Smee, Laland & Feldman 2003] *Niche Construction*, Princeton
- [Pirolli & Card 1999] Information Foraging, *Psychological Review*
- [Theraulaz & Bonabeau 1999] History of Stigmergy, *Artificial Life*

# Gap Inventory — 02 Agents

Concise gap list for agents working on agent parity batches.

## Focus Now

These are the gaps batch `02` should actively try to close:

### 1. Shared Response Surface Is Split — HIGH

- `ChatResponse` and `ResponseMetadata` are duplicated,
- the canonical home is unclear,
- broader crates want these types without depending on all of `roko-agent`.

### 2. The Best Agent Path Is Not The Main Agent Path — HIGH

- `run.rs` uses dispatcher + safety + scoped creation,
- `orchestrate.rs` mostly does not,
- so plan execution bypasses the strongest agent runtime protections.

### 3. Temperament Is Not A Real Runtime Contract — HIGH

- docs describe it,
- `AgentIdentity` stores only a string,
- there is no typed config or stable propagation path.

### 4. Creation-Site Consolidation Is Close But Incomplete — MEDIUM

- most paths are migrated,
- a few research entrypoints still call `create_agent_for_model` directly,
- which weakens scoped safety guarantees.

### 5. Tool Count Limits Exist Only On Paper — MEDIUM

- model profiles carry tool-cap hints,
- runtime does not enforce them,
- smaller models can still receive oversized tool sets.

## Defer From Batch 02

These are valid findings, but they should usually be documented and handed off:

- pool activation in the orchestrator -> `01`
- gate strictness by temperament -> `04`
- adaptive reward/routing economics -> `05`
- domain/plugin scaffold generation -> `03`
- concrete feedback collectors -> `05`
- supervision-tree runtime recovery wiring -> `01`
- Darwin-Godel / shared memory systems -> post-parity roadmap

## Working Rule

If an agent task requires:

- executor-state redesign,
- learning-policy redesign,
- or full verification-policy semantics,

then batch `02` should normally implement the smallest agent-layer foundation and defer the rest.

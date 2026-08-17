# PAG_02: Wire neuro store queries from all active dispatch paths

## Task
Ensure all active dispatch paths (roko run, plan run, ACP) query the neuro knowledge store for task context, not just orchestrate.rs (dead code).

## Runner Context
Runner PAG (Cognitive Cleanup), batch 2 of 3. Depends on PAF_01 (lifecycle manager available).

## Problem
CL-2 anti-pattern: "Knowledge built, never consulted." The neuro store has 7 verified query call sites, but most are in `orchestrate.rs` (dead code) or `knowledge_helpers.rs` (utility functions that need to be called from active paths). The v2 runner's event loop and ACP runner don't query the store before dispatch.

## Current Query Sites (VERIFIED)

**Active paths querying store** — These work:
- `knowledge_helpers.rs:132` — anti-knowledge queries
- `knowledge_helpers.rs:157` — playbook queries
- `knowledge_helpers.rs:284,309` — gate history queries
- `knowledge_helpers.rs:540` — task context queries
- `neuro/context.rs:746` — ContextPackBuilder queries
- `neuro/admission.rs:704` — duplicate check queries

**Dead code querying store** — These DON'T run:
- `orchestrate.rs:18125` — playbook retrieval (behind legacy-orchestrate feature)

**Paths that SHOULD query but DON'T**:
- v2 runner event_loop.rs — dispatches tasks without knowledge context
- ACP runner.rs — dispatches phases without knowledge context

## Exact Changes

### Step 1: Add knowledge query helper to v2 runner dispatch

In `event_loop.rs`, before dispatching each task:

```rust
use crate::knowledge_helpers;

// Query knowledge store for task context
let knowledge_context = if let Some(store) = &knowledge_store {
    let task_text = format!("{}: {}", task.title, task.description.as_deref().unwrap_or(""));

    let mut context_parts = Vec::new();

    // Anti-knowledge (things that DON'T work)
    if let Ok(anti) = knowledge_helpers::query_anti_knowledge(store, &task_text, 3) {
        if !anti.is_empty() {
            context_parts.push(format!("## Known Anti-Patterns\n{}", anti.join("\n")));
        }
    }

    // Relevant playbooks
    if let Ok(playbooks) = knowledge_helpers::query_playbooks(store, &task_text, 2) {
        if !playbooks.is_empty() {
            context_parts.push(format!("## Relevant Playbooks\n{}", playbooks.join("\n")));
        }
    }

    // Task-specific knowledge
    if let Ok(knowledge) = knowledge_helpers::query_task_context(store, &task_text, 5) {
        if !knowledge.is_empty() {
            context_parts.push(format!("## Relevant Knowledge\n{}", knowledge.join("\n")));
        }
    }

    if context_parts.is_empty() {
        None
    } else {
        Some(context_parts.join("\n\n"))
    }
} else {
    None
};
```

### Step 2: Inject knowledge into system prompt

Pass the knowledge context into the SystemPromptBuilder (via PK_03's `with_domain()` or a dedicated layer):

```rust
if let Some(knowledge) = &knowledge_context {
    system_prompt_builder = system_prompt_builder
        .with_domain(knowledge.clone());
}
```

### Step 3: Add knowledge query to ACP runner

In ACP `runner.rs`, before each phase:

```rust
let knowledge_context = query_knowledge_for_phase(
    &knowledge_store,
    &phase.name,
    &task_description,
).await;

// Prepend knowledge context to phase prompt
let enriched_prompt = if let Some(ctx) = knowledge_context {
    format!("{ctx}\n\n{prompt}")
} else {
    prompt.to_string()
};
```

## Write Scope
- `crates/roko-cli/src/runner/event_loop.rs` (add knowledge queries before dispatch)
- `crates/roko-acp/src/runner.rs` (add knowledge queries before phases)

## Read-Only Context
- `crates/roko-cli/src/knowledge_helpers.rs` (query_anti_knowledge, query_playbooks, query_task_context)
- `crates/roko-neuro/src/lib.rs` (KnowledgeStore query API)


## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test --workspace 2>&1 | tail -20
```
## Acceptance Criteria
- v2 runner queries knowledge store before every task dispatch
- ACP runner queries knowledge store before every phase
- Anti-knowledge, playbooks, and task context all queried
- Missing knowledge store → no injection (no crash)
- Knowledge limited to ~2000 tokens to avoid prompt bloat

## Do NOT
- Change the knowledge_helpers.rs query functions
- Add new query types
- Query on every tool call (too expensive — once per task dispatch)

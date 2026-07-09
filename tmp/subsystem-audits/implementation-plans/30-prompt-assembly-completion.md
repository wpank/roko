# 30 — Prompt Assembly Completion

The 9-layer `SystemPromptBuilder` exists but only one of six entry
points uses the full builder. HDC similarity is disabled. The playbook
layer is empty (T4-32 wires it). The role-specific prompt content has
hardcoded fragments.

Source: subsystem-audits/prompt-assembly/AUDIT.md, doc 35 § Dispatch
(prompt rendering preserves role boundaries).

---

## Today's State

- 9 layers: identity, capability, role, task, context (HDC), playbooks,
  scratch, system, hint.
- One entry point (orchestrate.rs's main dispatch path) uses the full
  builder.
- Other surfaces (chat REPL, ACP, agent registration test, research
  enhancement, conductor model spawn) inline prompt strings or use
  partial builders.
- HDC similarity step disabled (constant false in some flag).
- Playbook layer doesn't query the playbook store (T4-32).
- VCG auction (a re-ranking mechanism for prompt elements) was
  identified as overengineered.

---

## Anti-Patterns

1. **No inline prompt string in dispatch.** Every model call goes
   through `SystemPromptBuilder`.
2. **No "fallback to inline prompt if builder fails."** Builder failures
   are typed errors.
3. **No layer that always inserts content.** Each layer's content is
   conditional; empty layers produce empty output.
4. **No copy-paste of prompt fragments between crates.** Reusable
   fragments live in `roko-prompt`'s text registry.

---

## Plan

### [ ] PA-1: Audit prompt construction sites

```bash
rg 'SystemPromptBuilder|system_prompt =|format!\(.*system' crates/ -g '*.rs'
```

For each match:

- Is it using the full builder? Mark ✓.
- Is it inlining a prompt string? Mark ✗ — needs migration.

Build a table:

| Site | Uses builder? | Migration plan |
|---|---|---|
| `orchestrate.rs::dispatch_agent_with` | ✓ | none |
| `chat_inline.rs::build_initial_prompt` | partial | finish migration |
| `acp::session::build_prompt` | unknown | audit |
| `roko-cli::commands::research::build_prompt` | inline | migrate |
| `roko-cli::commands::conductor::spawn` | inline | migrate |

### [ ] PA-2: Migrate chat REPL prompt to full builder

**File**: `crates/roko-cli/src/chat_inline.rs`

The chat REPL builds an initial system prompt that introduces the
assistant. Today this is inline. Migrate to the builder so the user's
custom config (identity overrides, role hints, playbook results) flows
through.

```rust
let prompt = SystemPromptBuilder::new()
    .identity(&self.config.agent.identity)
    .capability(&self.config.agent.capability)
    .role(&Role::Assistant)
    .scratch(&self.session.scratch)
    .build();
self.session.system_prompt = prompt;
```

**Estimated effort**: 3-4 hours.

### [ ] PA-3: Re-enable HDC similarity (with kill switch)

**File**: `crates/roko-prompt/src/context_layer.rs` (or wherever the HDC
step lives).

The HDC similarity layer retrieves similar past tasks and includes them
as context. It was disabled because of latency or quality concerns.
Re-enable with a kill switch:

```rust
pub fn build_context_layer(req: &ContextLayerReq) -> Option<String> {
    if !req.config.prompt.hdc_similarity_enabled {
        return None;
    }
    let similar = req.codeintel.find_similar(req.task_fingerprint, 3)?;
    if similar.is_empty() { return None; }
    let mut s = String::from("## Relevant Prior Work\n");
    for entry in similar {
        s.push_str(&format!("- {}: {}\n", entry.title, entry.summary));
    }
    Some(s)
}
```

Default `hdc_similarity_enabled = false` until quality measurements
justify enabling. T4-32-style measurement comes from the cascade router.

### [ ] PA-4: Wire playbook layer (depends on T4-32)

See plan 14 § T4-32. After T4-32 lands, the playbook layer reads from
the playbook store. Verify the `SystemPromptBuilder` integration.

### [ ] PA-5: Drop VCG auction overengineering

The audit calls VCG auction overengineered. If the codebase has a
`vcg_auction` module / type in `roko-prompt`:

```bash
rg 'vcg_auction|VcgAuction' crates/roko-prompt/
```

If unused, delete (T2-style subtraction). If used in only one site,
inline the simpler logic and remove the abstraction.

### [ ] PA-6: Centralize role-specific prompt fragments

Today, `Role::Implementer`, `Role::Reviewer`, etc. have hardcoded
prompt fragments scattered across files. Centralize:

```rust
// crates/roko-prompt/src/role_text.rs

pub fn role_prompt(role: &Role) -> &'static str {
    match role {
        Role::Implementer => include_str!("../assets/role/implementer.md"),
        Role::Reviewer => include_str!("../assets/role/reviewer.md"),
        Role::Researcher => include_str!("../assets/role/researcher.md"),
        // ...
    }
}
```

The role layer reads from this registry; per-agent overrides come from
the config.

---

## Combined Verification

```bash
cargo test -p roko-prompt --lib

# Inline prompt strings in dispatch sites: 0
rg 'system_prompt = format!\(' crates/roko-cli/src/   # 0 matches
rg 'system: format!\(' crates/roko-cli/src/           # 0 matches

# All entry points use the builder
rg 'SystemPromptBuilder::new' crates/ -g '*.rs' | wc -l   # >= 5
```

---

## Status

- [ ] PA-1 — Audit prompt construction sites
- [ ] PA-2 — Migrate chat REPL to full builder
- [ ] PA-3 — Re-enable HDC similarity with kill switch
- [ ] PA-4 — Wire playbook layer (after T4-32)
- [ ] PA-5 — Drop VCG auction
- [ ] PA-6 — Centralize role fragments

**Estimated effort**: 12-20 hours.

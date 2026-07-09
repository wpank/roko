# S-prompt-2: Migrate chat REPL initial prompt to full SystemPromptBuilder

## Task
Replace the chat REPL's inline-string initial system prompt with a full `SystemPromptBuilder` chain.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-prompt-1 (audit table). Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/30-prompt-assembly-completion.md` § PA-2.

## Read first

```bash
rg 'fn build_initial_prompt|fn initial_system_prompt|let system_prompt = ' crates/roko-cli/src/chat_inline.rs -n
rg 'SystemPromptBuilder' crates/roko-prompt/src/lib.rs
```

## Exact changes

`crates/roko-cli/src/chat_inline.rs`:

Replace the inline construction with:

```rust
use roko_prompt::SystemPromptBuilder;

fn build_initial_prompt(&self) -> String {
    SystemPromptBuilder::new()
        .identity(&self.config.config().agent.identity)
        .capability(&self.config.config().agent.capability)
        .role(&Role::Assistant)        // chat REPL is the assistant role
        .scratch(&self.session.scratch)
        // Add context / playbook layers when they have content
        .build()
}
```

Use `ValidatedConfig::config()` (post S-config-3) or whatever current accessor is.

If `Role::Assistant` doesn't exist as a variant, add it (or use the closest existing variant).

If the `identity` / `capability` config fields don't exist, this batch is **blocked** on those being added; log and stop.

## Tests

```rust
#[test]
fn chat_initial_prompt_uses_builder_layers() {
    let session = test_chat_session();
    let prompt = session.build_initial_prompt();
    assert!(prompt.contains("Assistant"));    // role layer rendered
    assert!(!prompt.contains("format!"));     // not inline
}
```

## Write Scope
- `crates/roko-cli/src/chat_inline.rs`

## Read-Only Context
- `crates/roko-prompt/src/lib.rs`

## Verify

```bash
rg 'SystemPromptBuilder::new' crates/roko-cli/src/chat_inline.rs
# Expect: at least 1 hit

rg 'fn build_initial_prompt' crates/roko-cli/src/chat_inline.rs
# Verify the body uses the builder
```

## Do NOT

- Do NOT bundle with other S-prompt batches.
- Do NOT change `SystemPromptBuilder` API.
- Do NOT keep the inline-string fallback.
- Do NOT change the chat REPL's other behavior (slash commands, history).

# S-prompt-4: Centralize role prompt fragments

## Task
Move hardcoded per-role prompt fragments (currently scattered across files) into `crates/roko-prompt/src/role_text.rs` + per-role markdown asset files.

## Runner Context
Runner audit-2026-05-01, group S. No dependencies. Wave 1.

## Source plan
`tmp/subsystem-audits/implementation-plans/30-prompt-assembly-completion.md` § PA-6.

## Read first

```bash
rg 'Role::Implementer|Role::Reviewer|Role::Researcher|"You are an? (implementer|reviewer|researcher)"' crates/ -g '*.rs' -n
```

Identify the per-role string fragments currently inline.

## Exact changes

### 1. Create asset files

```
crates/roko-prompt/assets/role/
├── implementer.md
├── reviewer.md
├── researcher.md
├── conductor.md
└── ... (one per Role variant)
```

Copy the existing inline strings into the markdown files verbatim.

### 2. `crates/roko-prompt/src/role_text.rs`

```rust
use crate::Role;

pub fn role_prompt(role: &Role) -> &'static str {
    match role {
        Role::Implementer => include_str!("../assets/role/implementer.md"),
        Role::Reviewer => include_str!("../assets/role/reviewer.md"),
        Role::Researcher => include_str!("../assets/role/researcher.md"),
        Role::Conductor => include_str!("../assets/role/conductor.md"),
        Role::Assistant => include_str!("../assets/role/assistant.md"),
        // ... add a variant for each Role
    }
}
```

### 3. Update `SystemPromptBuilder::role(...)`

```rust
impl<'a> SystemPromptBuilder<'a> {
    pub fn role(mut self, role: &Role) -> Self {
        self.role_section = Some(role_text::role_prompt(role).to_string());
        self
    }
}
```

### 4. Replace inline call sites

Wherever the codebase had a hardcoded role string, replace with `role_text::role_prompt(&role)` or — better — let the builder handle it via `.role(&Role::Implementer)`.

### 5. Allow per-agent override

The `[agent]` config can override the role prompt:

```rust
pub fn role_prompt_with_override(role: &Role, override_text: Option<&str>) -> String {
    override_text.map(String::from).unwrap_or_else(|| role_prompt(role).to_string())
}
```

### 6. Tests

```rust
#[test]
fn role_prompt_implementer_includes_canonical_phrase() {
    let s = role_prompt(&Role::Implementer);
    assert!(!s.is_empty());
    assert!(s.lines().count() > 0);
}

#[test]
fn override_takes_precedence_over_canonical() {
    let s = role_prompt_with_override(&Role::Implementer, Some("custom impl prompt"));
    assert_eq!(s, "custom impl prompt");
}
```

## Write Scope
- `crates/roko-prompt/src/role_text.rs` (new)
- `crates/roko-prompt/src/lib.rs` (re-export + builder change)
- `crates/roko-prompt/assets/role/*.md` (new)
- (Sites that previously inlined role text)

## Verify

```bash
ls crates/roko-prompt/assets/role/

rg 'role_prompt|role_text::' crates/roko-prompt/src/
# Expect: at least 3 hits

# Inline role strings gone (or only in tests)
rg '"You are an? (implementer|reviewer|researcher)"' crates/ -g '*.rs'
# Expect: 0 hits in non-test code
```

## Do NOT

- Do NOT bundle with other S-prompt batches.
- Do NOT add `Role` variants in this batch.
- Do NOT change runtime role-selection logic.
- Do NOT make role files configurable from `roko.toml` (separate concern).

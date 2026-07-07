# Prompt Layer Ordering

> The SystemPromptBuilder always assembles the 7 layers in the correct order. Higher-priority layers appear before lower-priority layers in the assembled prompt.

**Crate**: `roko-compose`
**Test type**: Unit test
**Enforcement**: `SystemPromptBuilder::build`
**Last reviewed**: 2026-04-19

---

## Statement

The assembled system prompt always has layers in the order:
1. Role and persona
2. Cognitive state
3. Context and memory
4. Task specification
5. Tool availability
6. Output format
7. Safety and constraints

For U-shape placement: most-important context appears at positions 1-2 and 6-7 (not 3-5).

---

## Test

```rust
#[test]
fn prompt_layers_in_correct_order() {
    let prompt = SystemPromptBuilder::default()
        .with_role(Role::Implementer)
        .with_task("implement feature X")
        .build();

    let content = prompt.to_string();

    // Layer markers must appear in order
    let role_pos = content.find("[ROLE]").unwrap();
    let task_pos = content.find("[TASK]").unwrap();
    let safety_pos = content.find("[SAFETY]").unwrap();

    assert!(role_pos < task_pos, "Role must precede task");
    assert!(task_pos < safety_pos, "Task must precede safety constraints");
}
```

---

## See also

- [../by-subsystem/subsystem-compose.md](../by-subsystem/subsystem-compose.md)

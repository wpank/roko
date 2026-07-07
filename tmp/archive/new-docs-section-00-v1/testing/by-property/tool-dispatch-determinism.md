# Tool Dispatch Determinism

> Given the same task context and tool registry, the tool selector always chooses the same tool.

**Crate**: `roko-std`
**Test type**: Unit test
**Enforcement**: `ToolSelector::select`
**Last reviewed**: 2026-04-19

---

## Statement

For all task contexts T and tool registries R:
`ToolSelector::select(T, R) == ToolSelector::select(T, R)` (evaluated twice)

---

## Why It Matters

Non-deterministic tool selection would make agent runs irreproducible. The learning subsystem relies on tool choice determinism to correctly attribute outcomes.

---

## See also

- [../by-subsystem/subsystem-std.md](../by-subsystem/subsystem-std.md)

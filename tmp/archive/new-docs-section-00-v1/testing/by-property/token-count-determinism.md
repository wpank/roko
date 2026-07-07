# Token Count Determinism

> Counting the tokens in a prompt always returns the same count for the same prompt string.

**Crate**: `roko-agent`
**Test type**: Unit test
**Enforcement**: `TokenCounter::count`
**Last reviewed**: 2026-04-19

---

## Statement

For all prompt strings P:
`TokenCounter::count(P) == TokenCounter::count(P)` (evaluated twice)

---

## Why It Matters

Token counting is used by the SystemPromptBuilder for budget management and by the CascadeRouter for cost estimation. Non-deterministic token counts would cause prompt assembly to produce different sizes for the same prompt, breaking the token budget guarantee.

---

## See also

- [../by-subsystem/subsystem-agent.md](../by-subsystem/subsystem-agent.md)
- [../by-subsystem/subsystem-compose.md](../by-subsystem/subsystem-compose.md)

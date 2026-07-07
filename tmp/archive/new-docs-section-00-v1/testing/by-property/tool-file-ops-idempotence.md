# Tool File Operations Idempotence

> `delete_file` on a non-existent file returns `Ok(())`. `write_file` on an existing path overwrites without error.

**Crate**: `roko-std`
**Test type**: Unit test
**Enforcement**: `DeleteFile::run`, `WriteFile::run`
**Last reviewed**: 2026-04-19

---

## Statement

1. `delete_file(path)` where `path` does not exist returns `Ok(())` (idempotent delete).
2. `write_file(path, content)` where `path` already exists overwrites the content (idempotent write).

---

## Why It Matters

Agents may retry failed tool calls. An agent that retried `delete_file` should not get an error on the second attempt.

---

## See also

- [../by-subsystem/subsystem-std.md](../by-subsystem/subsystem-std.md)

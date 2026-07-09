# roko-fs — Test Coverage

> 37 tests for the JSONL FileSubstrate: write, read, GC, and content-addressing correctness.

**Status**: Shipping
**Crate**: `roko-fs`
**Section**: 04 — Storage (implements `Substrate` trait)
**Last reviewed**: 2026-04-19

---

## Test Count: 37

Source: implementation status audit, 2026-04-17.

| Module | Approx. tests | Focus |
|---|---|---|
| `file_substrate` | ~20 | Write, read, list, GC operations |
| `serialization` | ~10 | JSONL format, encoding correctness |
| `garbage_collector` | ~7 | GC correctness, living engram preservation |

---

## Key Test Focus Areas

### Write/Read

- `write` then `read` by content hash returns the original Engram (round-trip).
- `write` of an already-existing Engram is idempotent (no duplicates in the file).
- `read` of a non-existent content hash returns `None`.
- Large Engram (> 64KB body) is written and read correctly.
- Engrams with all body variants are written and read correctly.

### List / Query

- `list_all` returns all written Engrams.
- `list_by_kind` filters correctly.
- `list_by_score_range` returns only Engrams within the score range.
- Empty substrate returns empty list.

### Garbage Collection

- GC does not delete Engrams whose decay has not reached zero.
- GC deletes Engrams whose decay has reached zero.
- GC preserves all living Engrams exactly (no false positives).
- After GC, the substrate is consistent: no dangling references.
- GC is idempotent: running it twice in a row with no new writes changes nothing.

Key property: [../by-property/substrate-gc-preserves-living.md](../by-property/substrate-gc-preserves-living.md).

---

## Property Tests

| Property | Test name |
|---|---|
| Write idempotence | `substrate_write_idempotent` |
| Read-after-write | `substrate_read_after_write_consistent` |
| GC preserves living Engrams | `gc_never_deletes_living_engram` |
| JSONL encoding round-trip | `jsonl_encoding_roundtrip` |

---

## Known Gaps

- No concurrent write tests: `roko-fs` is not designed for concurrent access, but this is undocumented.
- No test for substrate files > 100MB.
- No test for filesystem full (ENOSPC) error handling.

## See also

- [../by-property/substrate-idempotence.md](../by-property/substrate-idempotence.md)
- [../by-property/substrate-read-after-write.md](../by-property/substrate-read-after-write.md)
- [../by-property/substrate-gc-preserves-living.md](../by-property/substrate-gc-preserves-living.md)

# Index.db Storage and Scaling

> **Parent:** [34-CODE-INTELLIGENCE](../../34-CODE-INTELLIGENCE.md) Section 12

---

## Storage Design Philosophy

### Why SQLite

SQLite is the right choice for a developer-side code intelligence database:

1. **Zero administration** -- No server process, no configuration, no
   network. The index is a single file at `.roko/index.db`.
2. **ACID guarantees** -- Concurrent reads with serialized writes via WAL
   mode. No corruption from crashes during re-indexing.
3. **FTS5** -- Built-in full-text search with BM25 ranking and custom
   tokenizers for camelCase and snake_case.
4. **Performance** -- Read throughput exceeds code intelligence needs by
   orders of magnitude.
5. **Embeddable** -- Links directly into the Rust binary via `rusqlite`.
   No dynamic dependencies.

### File location

```
.roko/index.db
```

Placed alongside other Roko state files in the project's `.roko/` directory.
Excluded from version control.

---

## Database Schema (v4)

The `SqliteIndex` in `crates/roko-index/src/sqlite.rs` (1,502 lines, 22
tests) implements the persistent storage layer behind the `sqlite` feature
flag.

### Core tables

```sql
-- Files tracked by the index
CREATE TABLE files (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    language TEXT NOT NULL,
    content_hash BLOB NOT NULL,    -- BLAKE3 hash of file content
    size_bytes INTEGER NOT NULL,
    last_indexed INTEGER NOT NULL,
    symbol_count INTEGER NOT NULL DEFAULT 0
);

-- Symbol definitions
CREATE TABLE symbols (
    id INTEGER PRIMARY KEY,
    file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    visibility TEXT NOT NULL,
    line INTEGER NOT NULL,
    column INTEGER NOT NULL DEFAULT 0,
    end_line INTEGER,
    signature TEXT,
    doc_comment TEXT,
    content_hash BLOB,
    UNIQUE(file_id, name, kind)
);

-- Full-text search over symbol names
CREATE VIRTUAL TABLE symbols_fts USING fts5(
    name, doc_comment,
    content='symbols', content_rowid='id',
    tokenize='unicode61 remove_diacritics 2'
);

-- Dependency edges
CREATE TABLE edges (
    id INTEGER PRIMARY KEY,
    from_symbol_id INTEGER NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
    to_symbol_id INTEGER NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    weight REAL NOT NULL DEFAULT 1.0,
    UNIQUE(from_symbol_id, to_symbol_id, kind)
);

-- Cached PageRank scores
CREATE TABLE pagerank (
    symbol_id INTEGER PRIMARY KEY REFERENCES symbols(id) ON DELETE CASCADE,
    score REAL NOT NULL,
    computed_at INTEGER NOT NULL
);

-- Schema versioning and metadata
CREATE TABLE meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

### Key design decisions

- WAL mode enabled at creation for concurrent reader/single writer access
- Foreign keys with CASCADE deletes: removing a file removes its symbols,
  edges, and fingerprints
- BLAKE3 content hashing for accurate change detection
- FTS5 search tokenizes both camelCase and snake_case identifiers
- Schema version in `meta` table for migration support

---

## Incremental Update Strategy

### Content-based change detection via BLAKE3

The incremental update algorithm uses BLAKE3 content hashing to detect
actual changes:

```
1. Enumerate workspace files by extension
2. For each file:
   a. Compute BLAKE3(content)
   b. Look up in files table by path
   c. Hash matches --> skip (no change)
   d. Hash differs --> re-parse, update symbols, edges, fingerprints
   e. New file --> insert everything fresh
3. Delete removed files (present in DB but not on disk)
4. Recompute PageRank if graph changed
5. Update meta.last_indexed
```

This is strictly more accurate than timestamp-based detection:

| Scenario | Timestamp | BLAKE3 |
|----------|-----------|--------|
| `git checkout` (new timestamps, same content) | Re-indexes (wrong) | Skips (correct) |
| IDE write-then-rename (stale timestamp) | Skips (wrong) | Re-indexes (correct) |
| Same content across branches | Re-indexes (wrong) | Skips (correct) |

### Batch operations

The update runs within a single SQLite transaction for atomicity. If
anything fails, the entire update rolls back cleanly.

### Expected performance

| Operation | 100 files | 1K files | 10K files | 100K files |
|-----------|----------|---------|----------|-----------|
| Full index build | 50ms | 500ms | 5s | 50s |
| Incremental (1 file) | 5ms | 5ms | 5ms | 5ms |
| Incremental (10 files) | 50ms | 50ms | 50ms | 50ms |
| BLAKE3 hashing (all) | 10ms | 100ms | 1s | 10s |
| PageRank recomputation | <1ms | 1ms | 10ms | 100ms |
| FTS5 query | <1ms | <1ms | 1ms | 5ms |

The critical insight: incremental update time is proportional to changed
files, not total files. A typical commit changes 1--5 files, so
re-indexing takes < 25ms regardless of workspace size.

---

## Scaling Characteristics

### Storage requirements

| Metric | Per symbol | 5K symbols | 50K symbols | 500K symbols |
|--------|-----------|-----------|------------|-------------|
| Symbol record | ~200 bytes | 1 MB | 10 MB | 100 MB |
| HDC fingerprint | 1,280 bytes | 6.25 MB | 62.5 MB | 625 MB |
| Edges (avg 3/sym) | ~50 bytes | 750 KB | 7.5 MB | 75 MB |
| FTS5 index | ~100 bytes | 500 KB | 5 MB | 50 MB |
| **Total** | | **~9 MB** | **~85 MB** | **~850 MB** |

### Query performance

| Query type | Expected latency |
|-----------|-----------------|
| Symbol by name (indexed) | < 0.1ms |
| Symbol by kind (indexed) | < 1ms |
| FTS5 search | < 5ms |
| Forward/reverse edge lookup | < 0.1ms |
| Fingerprint scan (5K) | ~0.25ms |
| Fingerprint scan (50K) | ~2.5ms |

---

## FTS5 Search Design

### Tokenization for code

Code identifiers follow different conventions than natural language:

| Convention | Example | Tokens |
|-----------|---------|--------|
| snake_case | `process_input` | `process`, `input` |
| camelCase | `processInput` | `process`, `input` |
| PascalCase | `ProcessInput` | `process`, `input` |
| SCREAMING_SNAKE | `MAX_BUFFER_SIZE` | `max`, `buffer`, `size` |

The `unicode61` tokenizer handles snake_case via underscore separators.
CamelCase splitting uses a pre-processing step that splits identifiers
before insertion.

### Query examples

```sql
-- Prefix search
SELECT * FROM symbols_fts WHERE symbols_fts MATCH 'process*';

-- Boolean AND
SELECT * FROM symbols_fts WHERE symbols_fts MATCH 'graph AND build';

-- Negation
SELECT * FROM symbols_fts WHERE symbols_fts MATCH 'graph NOT test';

-- Column-specific
SELECT * FROM symbols_fts WHERE symbols_fts MATCH 'name:process';
```

FTS5 uses BM25 ranking by default. The `rank` column gives relevance
scores.

---

## WAL Mode Configuration

```rust
fn configure_database(db: &Connection) -> Result<()> {
    db.pragma_update(None, "journal_mode", "WAL")?;
    db.pragma_update(None, "synchronous", "NORMAL")?;
    db.pragma_update(None, "cache_size", "-65536")?;  // 64MB
    db.pragma_update(None, "foreign_keys", "ON")?;
    db.pragma_update(None, "busy_timeout", "5000")?;
    Ok(())
}
```

WAL mode trade-offs:
- Reads never block writes, writes never block reads
- The `-wal` and `-shm` files must be on the same filesystem
- Checkpoint runs automatically when WAL exceeds 1000 pages (~4MB)

---

## Schema Migration System

The `meta` table tracks schema version. On startup, pending migrations
run:

```rust
const CURRENT_SCHEMA_VERSION: i64 = 4;

static MIGRATIONS: &[(i64, &str)] = &[
    (2, include_str!("migrations/002_add_embeddings.sql")),
    (3, include_str!("migrations/003_add_edge_weight.sql")),
    (4, include_str!("migrations/004_add_file_language_index.sql")),
];
```

Migration rules:
- Migrations are additive (never drop columns)
- Each migration uses `IF NOT EXISTS` where possible
- On incompatible schema changes, drop and rebuild (index is a cache)

---

## Feature-Flag Architecture

```toml
[features]
default = []
sqlite = ["dep:rusqlite"]
```

Without the feature enabled, `roko-index` works entirely in-memory. The
`CodeIndex` trait abstracts over both backends. The CLI enables `sqlite`
and uses `SqliteIndex` for persistence.

---

## CLI Commands

| Command | What it does |
|---------|-------------|
| `roko index build [--path DIR]` | Build or incrementally update the index |
| `roko index rebuild [--path DIR]` | Drop and rebuild from scratch |
| `roko index search QUERY [--kind K] [--strategy S]` | Search the index |
| `roko index stats [--path DIR]` | Print index statistics |
| `roko run-index repair [--apply]` | Inspect or rebuild derived indexes |

---

## Verified Behaviors (22 tests)

- Schema creation and migration
- File insert/update with BLAKE3 hashing
- Symbol upsert and query
- Edge upsert and query
- PageRank persistence
- Incremental update (add/update/skip unchanged)
- WAL mode configuration
- FTS5 search results ranked by BM25

---

## Academic Foundations

- **SQLite**: Hipp (2000). Zero-configuration embeddable database with
  ACID guarantees.
- **BLAKE3**: O'Connor et al. (2020). Content hashing 3--5x faster than
  SHA-256. Tree-hashable for parallel computation.
- **FTS5**: SQLite Extension. Full-text search with BM25 ranking and
  custom tokenizers.
- **Salsa**: rust-analyzer team (2019). Incremental computation framework
  (planned `salsa-memo` feature).

---

## Cross-References

- See [snapshot-optimization.md](./snapshot-optimization.md) for rkyv
  zero-copy snapshots as a complement to SQLite
- See [hdc-fingerprints.md](./hdc-fingerprints.md) for fingerprint storage
  requirements
- See [context-assembly-from-code.md](./context-assembly-from-code.md) for
  search queries that hit the database
- See [mcp-context-server.md](./mcp-context-server.md) for the server that
  queries the index

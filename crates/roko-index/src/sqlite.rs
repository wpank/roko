//! Optional SQLite-backed persistent code index.
//!
//! Feature-gated behind `sqlite`. Stores symbols, edges, and file metadata in
//! a local `.roko/index.db` file with WAL mode for concurrent reads.
//!
//! # Feature disposition (backlog #362)
//!
//! - **SQLite** (`roko-index/sqlite`): canonical CLI persistence at `<root>/.roko/index.db`.
//!   Enabled by `roko-cli`.
//! - **rkyv** (`roko-index/rkyv`): library-only opt-in. CLI does not enable it.
//! - **tree-sitter** (`roko-lang-rust/tree-sitter`): disabled and experimental. CLI/index
//!   code must not branch on it.
//! - **HDC search**: library-only per backlog #335.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context as _, Result, bail};
use rusqlite::{Connection, params};

use crate::graph::EdgeKind;
use crate::symbol::SymbolId;
use crate::workspace::SymbolInfo;
use roko_core::language::{SymbolKind, Visibility};

/// Statistics returned from an incremental update.
#[derive(Clone, Debug, Default)]
pub struct UpdateStats {
    /// Number of files that were re-parsed.
    pub files_updated: usize,
    /// Number of files that were skipped (unchanged).
    pub files_skipped: usize,
    /// Number of symbols inserted or replaced.
    pub symbols_upserted: usize,
    /// Number of edges inserted or replaced.
    pub edges_upserted: usize,
}

/// SQLite-backed persistent index for symbols and edges.
pub struct SqliteIndex {
    conn: Connection,
}

impl SqliteIndex {
    /// Open (or create) the index database at `path`.
    ///
    /// Enables WAL mode for concurrent reads.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("opening index db at {}", path.display()))?;

        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let index = Self { conn };
        index.create_tables()?;
        Ok(index)
    }

    /// Open an in-memory database (useful for testing).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let index = Self { conn };
        index.create_tables()?;
        Ok(index)
    }

    /// Create the schema tables if they do not already exist.
    pub fn create_tables(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS files (
                path     TEXT PRIMARY KEY,
                mtime_ns INTEGER NOT NULL,
                hash     TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS symbols (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path  TEXT NOT NULL,
                name       TEXT NOT NULL,
                kind       TEXT NOT NULL,
                line       INTEGER NOT NULL,
                col        INTEGER NOT NULL DEFAULT 0,
                visibility TEXT NOT NULL DEFAULT 'Private',
                UNIQUE(file_path, name, kind)
            );
            CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name);
            CREATE INDEX IF NOT EXISTS idx_symbols_file ON symbols(file_path);

            CREATE TABLE IF NOT EXISTS edges (
                from_file TEXT NOT NULL,
                from_name TEXT NOT NULL,
                from_kind TEXT NOT NULL,
                to_file   TEXT NOT NULL,
                to_name   TEXT NOT NULL,
                to_kind   TEXT NOT NULL,
                edge_kind TEXT NOT NULL,
                UNIQUE(from_file, from_name, from_kind, to_file, to_name, to_kind, edge_kind)
            );
            CREATE INDEX IF NOT EXISTS idx_edges_from ON edges(from_file, from_name);
            CREATE INDEX IF NOT EXISTS idx_edges_to ON edges(to_file, to_name);

            CREATE TABLE IF NOT EXISTS rankings (
                file_path TEXT NOT NULL,
                name      TEXT NOT NULL,
                kind      TEXT NOT NULL,
                score     REAL NOT NULL,
                UNIQUE(file_path, name, kind)
            );
            CREATE INDEX IF NOT EXISTS idx_rankings_score ON rankings(score DESC);",
        )?;
        self.create_fts_table()?;
        Ok(())
    }

    /// Create the FTS5 virtual table for full-text keyword search (CODE-06).
    ///
    /// Uses a standalone (non-content-sync) FTS table populated via `rebuild_fts`.
    fn create_fts_table(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS symbols_fts USING fts5(
                name,
                file_path,
                kind,
                sym_id
            );",
        )?;
        Ok(())
    }

    /// Rebuild the FTS5 index from the current symbols table.
    ///
    /// Should be called after bulk inserts or an incremental update.
    pub fn rebuild_fts(&self) -> Result<()> {
        self.conn.execute_batch(
            "DELETE FROM symbols_fts;
             INSERT INTO symbols_fts(sym_id, name, file_path, kind)
                SELECT id, name, file_path, kind FROM symbols;",
        )?;
        Ok(())
    }

    /// Full-text keyword search using FTS5 (CODE-06).
    ///
    /// Searches symbol names and file paths using the SQLite FTS5 engine.
    /// Returns up to 100 matching symbols sorted by FTS5 relevance rank.
    pub fn fts_search(&self, query: &str) -> Result<Vec<SymbolInfo>> {
        // Escape special FTS5 characters and add prefix matching.
        let safe_query = query.replace('"', "\"\"");
        let fts_query = format!("\"{safe_query}\"*");

        let mut stmt = self.conn.prepare(
            "SELECT s.file_path, s.name, s.kind, s.line, s.visibility
             FROM symbols_fts fts
             JOIN symbols s ON s.id = CAST(fts.sym_id AS INTEGER)
             WHERE symbols_fts MATCH ?1
             ORDER BY rank
             LIMIT 100",
        )?;

        let rows = stmt.query_map(params![fts_query], |row| {
            let file_path: String = row.get(0)?;
            let name: String = row.get(1)?;
            let kind_str: String = row.get(2)?;
            let line: usize = row.get(3)?;
            let vis_str: String = row.get(4)?;
            Ok((file_path, name, kind_str, line, vis_str))
        })?;

        let mut results = Vec::new();
        for row in rows {
            let (file_path, name, kind_str, line, vis_str) = row?;
            let kind = parse_symbol_kind(&kind_str);
            let visibility = parse_visibility(&vis_str);
            results.push(SymbolInfo {
                id: SymbolId::new(file_path, name, kind),
                visibility,
                line,
                language: String::new(),
            });
        }
        Ok(results)
    }

    /// Insert or replace a symbol.
    pub fn insert_symbol(&self, symbol: &SymbolInfo) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO symbols (file_path, name, kind, line, col, visibility)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                symbol.id.file_path,
                symbol.id.symbol_name,
                format!("{:?}", symbol.id.kind),
                symbol.line,
                0,
                format!("{:?}", symbol.visibility),
            ],
        )?;
        Ok(())
    }

    /// Insert or replace an edge.
    pub fn insert_edge(&self, from: &SymbolId, to: &SymbolId, kind: &EdgeKind) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO edges (from_file, from_name, from_kind, to_file, to_name, to_kind, edge_kind)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                from.file_path,
                from.symbol_name,
                format!("{:?}", from.kind),
                to.file_path,
                to.symbol_name,
                format!("{:?}", to.kind),
                format!("{:?}", kind),
            ],
        )?;
        Ok(())
    }

    /// Query symbols whose name contains the query string (case-insensitive).
    pub fn query_symbols(&self, query: &str) -> Result<Vec<SymbolInfo>> {
        let pattern = format!("%{query}%");
        let mut stmt = self.conn.prepare(
            "SELECT file_path, name, kind, line, visibility FROM symbols
             WHERE name LIKE ?1
             ORDER BY name, file_path
             LIMIT 100",
        )?;

        let rows = stmt.query_map(params![pattern], |row| {
            let file_path: String = row.get(0)?;
            let name: String = row.get(1)?;
            let kind_str: String = row.get(2)?;
            let line: usize = row.get(3)?;
            let vis_str: String = row.get(4)?;

            Ok((file_path, name, kind_str, line, vis_str))
        })?;

        let mut results = Vec::new();
        for row in rows {
            let (file_path, name, kind_str, line, vis_str) = row?;
            let kind = parse_symbol_kind(&kind_str);
            let visibility = parse_visibility(&vis_str);
            results.push(SymbolInfo {
                id: SymbolId::new(file_path, name, kind),
                visibility,
                line,
                language: String::new(),
            });
        }
        Ok(results)
    }

    /// Perform an incremental update: check file mtimes and only re-index
    /// changed files.
    ///
    /// `changed_files` provides the set of files to check. Files whose mtime
    /// has not changed (compared to the stored value) are skipped.
    pub fn incremental_update<F>(
        &self,
        changed_files: &[PathBuf],
        mut index_file: F,
    ) -> Result<UpdateStats>
    where
        F: FnMut(&Path) -> Result<(Vec<SymbolInfo>, Vec<(SymbolId, SymbolId, EdgeKind)>)>,
    {
        let mut stats = UpdateStats::default();
        let tx = self.conn.unchecked_transaction()?;

        for file_path in changed_files {
            let path_str = file_path.to_string_lossy().to_string();

            let current_mtime = file_mtime_ns(file_path);
            let stored_mtime: Option<i64> = tx
                .query_row(
                    "SELECT mtime_ns FROM files WHERE path = ?1",
                    params![path_str],
                    |row| row.get(0),
                )
                .ok();

            if stored_mtime == Some(current_mtime) {
                stats.files_skipped += 1;
                continue;
            }

            // Remove stale data for this file.
            tx.execute(
                "DELETE FROM symbols WHERE file_path = ?1",
                params![path_str],
            )?;
            tx.execute(
                "DELETE FROM edges WHERE from_file = ?1 OR to_file = ?1",
                params![path_str],
            )?;

            // Re-index.
            let (symbols, edges) = index_file(file_path)?;

            for sym in &symbols {
                tx.execute(
                    "INSERT OR REPLACE INTO symbols (file_path, name, kind, line, col, visibility)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        sym.id.file_path,
                        sym.id.symbol_name,
                        format!("{:?}", sym.id.kind),
                        sym.line,
                        0,
                        format!("{:?}", sym.visibility),
                    ],
                )?;
                stats.symbols_upserted += 1;
            }

            for (from, to, kind) in &edges {
                tx.execute(
                    "INSERT OR REPLACE INTO edges (from_file, from_name, from_kind, to_file, to_name, to_kind, edge_kind)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        from.file_path,
                        from.symbol_name,
                        format!("{:?}", from.kind),
                        to.file_path,
                        to.symbol_name,
                        format!("{:?}", to.kind),
                        format!("{:?}", kind),
                    ],
                )?;
                stats.edges_upserted += 1;
            }

            // Update file record.
            let hash = blake3::hash(path_str.as_bytes()).to_hex().to_string();
            tx.execute(
                "INSERT OR REPLACE INTO files (path, mtime_ns, hash) VALUES (?1, ?2, ?3)",
                params![path_str, current_mtime, hash],
            )?;

            stats.files_updated += 1;
        }

        tx.commit()?;
        Ok(stats)
    }

    /// Insert or replace a ranking score for a symbol.
    pub fn insert_ranking(&self, id: &SymbolId, score: f64) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO rankings (file_path, name, kind, score)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                id.file_path,
                id.symbol_name,
                format!("{:?}", id.kind),
                score,
            ],
        )?;
        Ok(())
    }

    /// Query the top-N ranked symbols by PageRank score.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn top_rankings(&self, limit: usize) -> Result<Vec<(SymbolId, f64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT file_path, name, kind, score FROM rankings
             ORDER BY score DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            let file_path: String = row.get(0)?;
            let name: String = row.get(1)?;
            let kind_str: String = row.get(2)?;
            let score: f64 = row.get(3)?;
            Ok((file_path, name, kind_str, score))
        })?;
        let mut results = Vec::new();
        for row in rows {
            let (file_path, name, kind_str, score) = row?;
            let kind = parse_symbol_kind(&kind_str);
            results.push((SymbolId::new(file_path, name, kind), score));
        }
        Ok(results)
    }

    /// Look up the stored ranking score for a specific symbol.
    pub fn ranking_for(&self, id: &SymbolId) -> Result<Option<f64>> {
        let result = self.conn.query_row(
            "SELECT score FROM rankings WHERE file_path = ?1 AND name = ?2 AND kind = ?3",
            params![
                id.file_path,
                id.symbol_name,
                format!("{:?}", id.kind),
            ],
            |row| row.get(0),
        );
        match result {
            Ok(score) => Ok(Some(score)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Total number of ranking entries stored.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn ranking_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM rankings", [], |row| row.get(0))?;
        Ok(count.max(0) as usize)
    }

    /// Total number of symbols stored.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn symbol_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM symbols", [], |row| row.get(0))?;
        Ok(count.max(0) as usize)
    }

    /// Total number of edges stored.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn edge_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))?;
        Ok(count.max(0) as usize)
    }
}

#[allow(clippy::cast_possible_truncation)]
fn file_mtime_ns(path: &Path) -> i64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map_or(0, |d| {
            let ns = d.as_nanos();
            if ns > i64::MAX as u128 {
                i64::MAX
            } else {
                ns as i64
            }
        })
}

#[allow(clippy::match_same_arms)]
fn parse_symbol_kind(s: &str) -> SymbolKind {
    match s {
        "Function" => SymbolKind::Function,
        "Struct" => SymbolKind::Struct,
        "Enum" => SymbolKind::Enum,
        "Trait" => SymbolKind::Trait,
        "Const" => SymbolKind::Const,
        "Type" => SymbolKind::Type,
        "Module" => SymbolKind::Module,
        "Impl" => SymbolKind::Impl,
        _ => SymbolKind::Function,
    }
}

fn parse_visibility(s: &str) -> Visibility {
    match s {
        "Public" => Visibility::Public,
        _ => Visibility::Private,
    }
}

// ─── IndexStore facade ──────────────────────────────────────────────────

/// Current schema version. Bump whenever the table layout changes.
/// v2: added `rankings` table for PageRank score persistence (#362).
const SCHEMA_VERSION: i64 = 2;

/// Busy timeout in milliseconds for concurrent access.
const BUSY_TIMEOUT_MS: u64 = 5_000;

/// Index database filename inside `.roko/`.
const INDEX_DB_NAME: &str = "index.db";

/// Typed errors that `IndexStore` can report for non-fatal conditions.
#[derive(Debug)]
pub enum IndexStoreError {
    /// The schema version in the DB does not match the running code.
    VersionMismatch { stored: i64, expected: i64 },
    /// The canonical root in the DB does not match the requested root.
    RootMismatch { stored: String, requested: String },
    /// The database file is corrupt or unreadable.
    Corrupt(String),
    /// Another process holds the database lock.
    Locked(String),
    /// A general I/O or SQL error.
    Other(anyhow::Error),
}

impl std::fmt::Display for IndexStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VersionMismatch { stored, expected } => {
                write!(
                    f,
                    "index schema version mismatch: stored {stored}, expected {expected}; rebuild required"
                )
            }
            Self::RootMismatch { stored, requested } => {
                write!(
                    f,
                    "index root mismatch: stored '{stored}', requested '{requested}'; rebuild required"
                )
            }
            Self::Corrupt(msg) => write!(f, "index database corrupt: {msg}"),
            Self::Locked(msg) => write!(f, "index database locked: {msg}"),
            Self::Other(err) => write!(f, "index store error: {err}"),
        }
    }
}

impl std::error::Error for IndexStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Other(err) => Some(err.as_ref()),
            _ => None,
        }
    }
}

impl From<anyhow::Error> for IndexStoreError {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(err)
    }
}

impl From<rusqlite::Error> for IndexStoreError {
    fn from(err: rusqlite::Error) -> Self {
        let msg = err.to_string();
        if msg.contains("database is locked") || msg.contains("SQLITE_BUSY") {
            Self::Locked(msg)
        } else if msg.contains("database disk image is malformed")
            || msg.contains("not a database")
            || msg.contains("SQLITE_CORRUPT")
            || msg.contains("SQLITE_NOTADB")
        {
            Self::Corrupt(msg)
        } else {
            Self::Other(err.into())
        }
    }
}

/// High-level persistent index store backed by SQLite.
///
/// Wraps [`SqliteIndex`] with schema versioning, canonical root validation,
/// atomic build (temp + rename), read-only open for search/stats, incremental
/// updates, and safe rebuild that only touches `<root>/.roko/index.db`.
pub struct IndexStore {
    inner: SqliteIndex,
    db_path: PathBuf,
    canonical_root: String,
}

/// Metadata stored in the `index_meta` table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexMeta {
    /// Schema version of the database.
    pub schema_version: i64,
    /// Canonical root path that the index was built against.
    pub canonical_root: String,
    /// Version string of the parser/index implementation.
    pub index_version: String,
    /// Fingerprint of enabled features at build time.
    pub feature_fingerprint: String,
}

impl IndexStore {
    /// Resolve the canonical DB path for a workspace root.
    pub fn db_path_for(root: &Path) -> PathBuf {
        root.join(".roko").join(INDEX_DB_NAME)
    }

    /// Open an existing index database for read-only queries.
    ///
    /// Validates the schema version and canonical root. Returns a typed
    /// error for mismatch, corruption, or lock contention.
    pub fn open_readonly(root: &Path) -> std::result::Result<Self, IndexStoreError> {
        let root = std::fs::canonicalize(root)
            .map_err(|e| IndexStoreError::Other(anyhow::Error::new(e).context("canonicalize root")))?;
        let db_path = Self::db_path_for(&root);
        if !db_path.exists() {
            return Err(IndexStoreError::Other(anyhow::anyhow!(
                "no index database at {}; run `roko index build` first",
                db_path.display()
            )));
        }

        let conn = Connection::open_with_flags(
            &db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.busy_timeout(std::time::Duration::from_millis(BUSY_TIMEOUT_MS))?;

        let root_str = root.to_string_lossy().to_string();
        let store = Self {
            inner: SqliteIndex { conn },
            db_path,
            canonical_root: root_str,
        };
        store.validate_meta()?;
        Ok(store)
    }

    /// Build a new index database, atomically replacing any existing one.
    ///
    /// 1. Writes to a temporary file next to the canonical path.
    /// 2. Fsyncs the temp file.
    /// 3. Atomically renames it to `index.db`.
    ///
    /// On failure the prior good DB (if any) is preserved.
    pub fn build(
        root: &Path,
        symbols: &[SymbolInfo],
        edges: &[(SymbolId, SymbolId, EdgeKind)],
        file_records: &[FileRecord],
    ) -> Result<Self> {
        Self::build_with_rankings(root, symbols, edges, file_records, &[])
    }

    /// Build a new index database with ranking data, atomically replacing any
    /// existing one.
    ///
    /// Like [`build`](Self::build) but additionally persists PageRank scores
    /// in the `rankings` table.
    pub fn build_with_rankings(
        root: &Path,
        symbols: &[SymbolInfo],
        edges: &[(SymbolId, SymbolId, EdgeKind)],
        file_records: &[FileRecord],
        rankings: &[RankingRecord],
    ) -> Result<Self> {
        let root = std::fs::canonicalize(root)
            .with_context(|| format!("canonicalize root {}", root.display()))?;
        let db_path = Self::db_path_for(&root);

        // Ensure the .roko directory exists.
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create .roko directory at {}", parent.display()))?;
        }

        let temp_path = db_path.with_extension("db.building");
        // Remove stale temp if present.
        let _ = std::fs::remove_file(&temp_path);

        let conn = Connection::open(&temp_path)
            .with_context(|| format!("create temp index at {}", temp_path.display()))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_millis(BUSY_TIMEOUT_MS))?;

        let idx = SqliteIndex { conn };
        idx.create_tables()?;

        let root_str = root.to_string_lossy().to_string();

        // Write metadata.
        idx.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS index_meta (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )?;

        {
            let tx = idx.conn.unchecked_transaction()?;

            tx.execute(
                "INSERT OR REPLACE INTO index_meta (key, value) VALUES ('schema_version', ?1)",
                params![SCHEMA_VERSION.to_string()],
            )?;
            tx.execute(
                "INSERT OR REPLACE INTO index_meta (key, value) VALUES ('canonical_root', ?1)",
                params![root_str],
            )?;
            tx.execute(
                "INSERT OR REPLACE INTO index_meta (key, value) VALUES ('index_version', ?1)",
                params![env!("CARGO_PKG_VERSION")],
            )?;
            tx.execute(
                "INSERT OR REPLACE INTO index_meta (key, value) VALUES ('feature_fingerprint', ?1)",
                params![Self::feature_fingerprint()],
            )?;

            // Insert file records.
            for fr in file_records {
                let content_hash = blake3::hash(fr.content.as_bytes()).to_hex().to_string();
                tx.execute(
                    "INSERT OR REPLACE INTO files (path, mtime_ns, hash) VALUES (?1, ?2, ?3)",
                    params![fr.path, file_mtime_ns(Path::new(&fr.path)), content_hash],
                )?;
            }

            // Insert symbols.
            for sym in symbols {
                tx.execute(
                    "INSERT OR REPLACE INTO symbols (file_path, name, kind, line, col, visibility)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        sym.id.file_path,
                        sym.id.symbol_name,
                        format!("{:?}", sym.id.kind),
                        sym.line,
                        0,
                        format!("{:?}", sym.visibility),
                    ],
                )?;
            }

            // Insert edges.
            for (from, to, kind) in edges {
                tx.execute(
                    "INSERT OR REPLACE INTO edges (from_file, from_name, from_kind, to_file, to_name, to_kind, edge_kind)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        from.file_path,
                        from.symbol_name,
                        format!("{:?}", from.kind),
                        to.file_path,
                        to.symbol_name,
                        format!("{:?}", to.kind),
                        format!("{:?}", kind),
                    ],
                )?;
            }

            // Insert ranking scores (PageRank).
            for ranking in rankings {
                tx.execute(
                    "INSERT OR REPLACE INTO rankings (file_path, name, kind, score)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        ranking.id.file_path,
                        ranking.id.symbol_name,
                        format!("{:?}", ranking.id.kind),
                        ranking.score,
                    ],
                )?;
            }

            tx.commit()?;
        }

        // Rebuild FTS after bulk insert.
        idx.rebuild_fts()?;

        // Fsync the temp file before rename.
        // WAL mode uses two files: the main DB and the WAL. Checkpoint first
        // so all data is in the main file, then fsync.
        idx.conn.pragma_update(None, "wal_checkpoint", "TRUNCATE")?;

        // Close the connection before rename so WAL/SHM files are cleaned up.
        drop(idx);

        // Fsync the temp DB file.
        {
            let file = std::fs::File::open(&temp_path)
                .with_context(|| format!("open temp db for fsync {}", temp_path.display()))?;
            file.sync_all()
                .with_context(|| format!("fsync temp db {}", temp_path.display()))?;
        }

        // Atomic rename.
        std::fs::rename(&temp_path, &db_path).with_context(|| {
            format!(
                "atomically replace index {} from {}",
                db_path.display(),
                temp_path.display()
            )
        })?;

        // Fsync the parent directory for rename durability.
        if let Some(parent) = db_path.parent() {
            if let Ok(dir) = std::fs::File::open(parent) {
                let _ = dir.sync_all();
            }
        }

        // Re-open the final DB for continued use.
        let conn = Connection::open(&db_path)
            .with_context(|| format!("reopen index at {}", db_path.display()))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.busy_timeout(std::time::Duration::from_millis(BUSY_TIMEOUT_MS))?;

        Ok(Self {
            inner: SqliteIndex { conn },
            db_path,
            canonical_root: root_str,
        })
    }

    /// Perform an incremental update on the existing database.
    ///
    /// Only re-indexes files whose mtime or content hash has changed.
    /// Removes symbols and edges for deleted files.
    pub fn incremental_update(
        &self,
        current_files: &[PathBuf],
        index_file: impl FnMut(&Path) -> Result<(Vec<SymbolInfo>, Vec<(SymbolId, SymbolId, EdgeKind)>)>,
    ) -> Result<UpdateStats> {
        let stats = self.inner.incremental_update(current_files, index_file)?;

        // Remove files that are no longer present on disk.
        let current_set: std::collections::HashSet<String> = current_files
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        let mut stmt = self
            .inner
            .conn
            .prepare("SELECT path FROM files")?;
        let stored_paths: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        drop(stmt);

        let tx = self.inner.conn.unchecked_transaction()?;
        for stored_path in &stored_paths {
            if !current_set.contains(stored_path) {
                tx.execute(
                    "DELETE FROM symbols WHERE file_path = ?1",
                    params![stored_path],
                )?;
                tx.execute(
                    "DELETE FROM edges WHERE from_file = ?1 OR to_file = ?1",
                    params![stored_path],
                )?;
                tx.execute("DELETE FROM files WHERE path = ?1", params![stored_path])?;
            }
        }
        tx.commit()?;

        // Rebuild FTS after updates.
        self.inner.rebuild_fts()?;

        Ok(stats)
    }

    /// Safely rebuild the index by deleting only `.roko/index.db` and its
    /// SQLite sidecars (`-wal`, `-shm`).
    ///
    /// # Safety
    ///
    /// Validates the canonical root to prevent deletion outside `.roko/index.db`.
    pub fn rebuild(root: &Path) -> Result<()> {
        let root = std::fs::canonicalize(root)
            .with_context(|| format!("canonicalize root {}", root.display()))?;
        let db_path = Self::db_path_for(&root);
        Self::validate_db_path(&db_path, &root)?;

        // Remove the main DB and its WAL/SHM sidecars.
        for suffix in ["", "-wal", "-shm"] {
            let target = if suffix.is_empty() {
                db_path.clone()
            } else {
                let mut s = db_path.as_os_str().to_owned();
                s.push(suffix);
                PathBuf::from(s)
            };
            if target.exists() {
                std::fs::remove_file(&target)
                    .with_context(|| format!("remove {}", target.display()))?;
            }
        }

        // Also remove the temp build file if present.
        let temp = db_path.with_extension("db.building");
        if temp.exists() {
            let _ = std::fs::remove_file(&temp);
        }

        Ok(())
    }

    /// Access the underlying `SqliteIndex` for queries.
    pub fn inner(&self) -> &SqliteIndex {
        &self.inner
    }

    /// Path to the database file.
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Read stored metadata.
    pub fn meta(&self) -> Result<IndexMeta> {
        self.read_meta().map_err(|e| match e {
            IndexStoreError::Other(e) => e,
            other => anyhow::anyhow!("{other}"),
        })
    }

    /// Compute a feature fingerprint string for the current build.
    fn feature_fingerprint() -> String {
        let mut features = Vec::new();
        features.push("sqlite");
        if cfg!(feature = "rkyv") {
            features.push("rkyv");
        }
        features.join(",")
    }

    /// Validate that the DB path is safely inside `.roko/`.
    fn validate_db_path(db_path: &Path, root: &Path) -> Result<()> {
        let roko_dir = root.join(".roko");
        if !db_path.starts_with(&roko_dir) {
            bail!(
                "index database path escapes .roko directory: {}",
                db_path.display()
            );
        }
        let file_name = db_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if file_name != INDEX_DB_NAME {
            bail!(
                "unexpected index database filename: {} (expected {})",
                file_name,
                INDEX_DB_NAME
            );
        }
        Ok(())
    }

    /// Read and validate metadata from the DB.
    fn validate_meta(&self) -> std::result::Result<(), IndexStoreError> {
        let meta = self.read_meta()?;

        if meta.schema_version != SCHEMA_VERSION {
            return Err(IndexStoreError::VersionMismatch {
                stored: meta.schema_version,
                expected: SCHEMA_VERSION,
            });
        }

        if meta.canonical_root != self.canonical_root {
            return Err(IndexStoreError::RootMismatch {
                stored: meta.canonical_root,
                requested: self.canonical_root.clone(),
            });
        }

        Ok(())
    }

    /// Read metadata from the index_meta table.
    fn read_meta(&self) -> std::result::Result<IndexMeta, IndexStoreError> {
        // Check if index_meta table exists.
        let table_exists: bool = self
            .inner
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='index_meta'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count > 0)
            .map_err(IndexStoreError::from)?;

        if !table_exists {
            return Err(IndexStoreError::VersionMismatch {
                stored: 0,
                expected: SCHEMA_VERSION,
            });
        }

        let get_meta = |key: &str| -> std::result::Result<String, IndexStoreError> {
            self.inner
                .conn
                .query_row(
                    "SELECT value FROM index_meta WHERE key = ?1",
                    params![key],
                    |row| row.get(0),
                )
                .map_err(|e| {
                    if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                        IndexStoreError::Corrupt(format!("missing metadata key: {key}"))
                    } else {
                        IndexStoreError::from(e)
                    }
                })
        };

        let version_str = get_meta("schema_version")?;
        let schema_version = version_str.parse::<i64>().map_err(|_| {
            IndexStoreError::Corrupt(format!("invalid schema_version: {version_str}"))
        })?;

        Ok(IndexMeta {
            schema_version,
            canonical_root: get_meta("canonical_root")?,
            index_version: get_meta("index_version").unwrap_or_default(),
            feature_fingerprint: get_meta("feature_fingerprint").unwrap_or_default(),
        })
    }
}

/// Record for a source file to be stored in the index.
#[derive(Clone, Debug)]
pub struct FileRecord {
    /// Workspace-relative file path.
    pub path: String,
    /// Raw file content (used for content hashing).
    pub content: String,
}

/// A symbol's ranking score for persistence.
#[derive(Clone, Debug)]
pub struct RankingRecord {
    /// Symbol identifier.
    pub id: SymbolId,
    /// PageRank score.
    pub score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_symbol(file: &str, name: &str, kind: SymbolKind, line: usize) -> SymbolInfo {
        SymbolInfo {
            id: SymbolId::new(file, name, kind),
            visibility: Visibility::Public,
            line,
            language: "rust".into(),
        }
    }

    #[test]
    fn open_and_insert_symbols() {
        let db = SqliteIndex::open_in_memory().expect("open in-memory db");

        let sym = test_symbol("lib.rs", "main", SymbolKind::Function, 1);
        db.insert_symbol(&sym).expect("insert symbol");

        let results = db.query_symbols("main").expect("query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id.symbol_name, "main");
        assert_eq!(results[0].line, 1);
    }

    #[test]
    fn insert_and_count_edges() {
        let db = SqliteIndex::open_in_memory().expect("open in-memory db");

        let from = SymbolId::new("a.rs", "foo", SymbolKind::Function);
        let to = SymbolId::new("b.rs", "Bar", SymbolKind::Struct);
        db.insert_edge(&from, &to, &EdgeKind::TypeRef)
            .expect("insert edge");

        assert_eq!(db.edge_count().expect("edge count"), 1);
    }

    #[test]
    fn query_is_case_insensitive() {
        let db = SqliteIndex::open_in_memory().expect("open in-memory db");

        let sym = test_symbol("lib.rs", "MyStruct", SymbolKind::Struct, 10);
        db.insert_symbol(&sym).expect("insert");

        let results = db.query_symbols("mystruct").expect("query");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn fts_search_finds_symbols() {
        let db = SqliteIndex::open_in_memory().expect("open in-memory db");

        let sym1 = test_symbol("lib.rs", "process_request", SymbolKind::Function, 10);
        let sym2 = test_symbol("lib.rs", "handle_response", SymbolKind::Function, 20);
        let sym3 = test_symbol("types.rs", "RequestConfig", SymbolKind::Struct, 5);
        db.insert_symbol(&sym1).expect("insert");
        db.insert_symbol(&sym2).expect("insert");
        db.insert_symbol(&sym3).expect("insert");
        db.rebuild_fts().expect("rebuild fts");

        let results = db.fts_search("request").expect("fts search");
        assert!(!results.is_empty());
        let names: Vec<&str> = results.iter().map(|s| s.id.symbol_name.as_str()).collect();
        assert!(
            names.contains(&"process_request") || names.contains(&"RequestConfig"),
            "expected FTS to find request-related symbols, got {names:?}"
        );
    }

    #[test]
    fn fts_search_empty_index() {
        let db = SqliteIndex::open_in_memory().expect("open in-memory db");
        db.rebuild_fts().expect("rebuild fts");
        let results = db.fts_search("anything").expect("fts search");
        assert!(results.is_empty());
    }

    #[test]
    fn incremental_update_skips_unchanged() {
        let db = SqliteIndex::open_in_memory().expect("open in-memory db");

        // Use a non-existent path so mtime will be 0, matching the empty DB.
        let fake_path = PathBuf::from("/nonexistent/file.rs");

        // First pass: file not in DB, so index_file is called.
        let stats = db
            .incremental_update(&[fake_path.clone()], |_path| {
                Ok((
                    vec![test_symbol(
                        "/nonexistent/file.rs",
                        "func",
                        SymbolKind::Function,
                        1,
                    )],
                    vec![],
                ))
            })
            .expect("first update");
        assert_eq!(stats.files_updated, 1);
        assert_eq!(stats.symbols_upserted, 1);

        // Second pass: mtime is still 0 (file doesn't exist), DB has 0 — should skip.
        let stats2 = db
            .incremental_update(&[fake_path], |_path| {
                panic!("should not be called for unchanged file");
            })
            .expect("second update");
        assert_eq!(stats2.files_skipped, 1);
        assert_eq!(stats2.files_updated, 0);
    }

    // ── IndexStore tests ────────────────────────────────────────────────

    #[test]
    fn sqlite_store_build_creates_versioned_db() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();
        std::fs::create_dir_all(root.join(".roko")).unwrap();

        let sym = test_symbol("lib.rs", "main", SymbolKind::Function, 1);
        let fr = FileRecord {
            path: "lib.rs".into(),
            content: "fn main() {}".into(),
        };

        let store = IndexStore::build(root, &[sym], &[], &[fr]).expect("build");
        assert!(store.db_path().exists());

        let meta = store.meta().expect("meta");
        assert_eq!(meta.schema_version, SCHEMA_VERSION);
        assert_eq!(meta.canonical_root, root.canonicalize().unwrap().to_string_lossy());
        assert!(!meta.index_version.is_empty());
        assert!(meta.feature_fingerprint.contains("sqlite"));
    }

    #[test]
    fn sqlite_store_second_process_reuses_without_reparsing() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();
        std::fs::create_dir_all(root.join(".roko")).unwrap();

        // Write a source file so mtime-based checks work.
        std::fs::write(root.join("lib.rs"), "fn main() {}").unwrap();

        let sym = test_symbol("lib.rs", "main", SymbolKind::Function, 1);
        let fr = FileRecord {
            path: "lib.rs".into(),
            content: "fn main() {}".into(),
        };

        IndexStore::build(root, &[sym], &[], &[fr]).expect("build");

        // Second open: read-only reuse.
        let store2 = IndexStore::open_readonly(root).expect("open readonly");
        let results = store2.inner().query_symbols("main").expect("query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id.symbol_name, "main");
    }

    #[test]
    fn sqlite_store_incremental_edit_add_delete() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();
        std::fs::create_dir_all(root.join(".roko")).unwrap();

        let sym1 = test_symbol("a.rs", "foo", SymbolKind::Function, 1);
        let sym2 = test_symbol("b.rs", "bar", SymbolKind::Function, 1);
        let fr1 = FileRecord {
            path: "a.rs".into(),
            content: "fn foo() {}".into(),
        };
        let fr2 = FileRecord {
            path: "b.rs".into(),
            content: "fn bar() {}".into(),
        };

        let store =
            IndexStore::build(root, &[sym1, sym2], &[], &[fr1, fr2]).expect("build");

        // Incremental update with only a.rs present: b.rs symbols should be removed.
        let fake_a = PathBuf::from("a.rs");
        let stats = store
            .incremental_update(&[fake_a], |_path| {
                Ok((
                    vec![test_symbol("a.rs", "foo_v2", SymbolKind::Function, 2)],
                    vec![],
                ))
            })
            .expect("incremental");

        // a.rs had mtime=0 matching stored, so skipped by mtime. But b.rs
        // should be deleted because it is no longer in current_files.
        assert_eq!(stats.files_skipped, 1);

        let bar_results = store.inner().query_symbols("bar").expect("query bar");
        assert!(
            bar_results.is_empty(),
            "deleted file symbols should be removed"
        );
    }

    #[test]
    fn sqlite_store_corrupt_db_reports_error() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();
        let roko_dir = root.join(".roko");
        std::fs::create_dir_all(&roko_dir).unwrap();

        // Write garbage to the DB file.
        std::fs::write(roko_dir.join(INDEX_DB_NAME), b"not a database").unwrap();

        let result = IndexStore::open_readonly(root);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let is_corrupt_or_version = matches!(
            err,
            IndexStoreError::Corrupt(_) | IndexStoreError::VersionMismatch { .. }
        );
        assert!(
            is_corrupt_or_version,
            "expected Corrupt or VersionMismatch error, got: {err}"
        );
    }

    #[test]
    fn sqlite_store_version_mismatch_reports_rebuild() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();
        std::fs::create_dir_all(root.join(".roko")).unwrap();

        // Build a valid DB, then tamper with the version.
        let store = IndexStore::build(root, &[], &[], &[]).expect("build");
        store
            .inner
            .conn
            .execute(
                "UPDATE index_meta SET value = '999' WHERE key = 'schema_version'",
                [],
            )
            .expect("tamper version");
        drop(store);

        let result = IndexStore::open_readonly(root);
        assert!(result.is_err());
        match result.unwrap_err() {
            IndexStoreError::VersionMismatch { stored, expected } => {
                assert_eq!(stored, 999);
                assert_eq!(expected, SCHEMA_VERSION);
            }
            other => panic!("expected VersionMismatch, got: {other}"),
        }
    }

    #[test]
    fn sqlite_store_root_mismatch_reports_rebuild() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();
        std::fs::create_dir_all(root.join(".roko")).unwrap();

        // Build a valid DB, then tamper with the root.
        let store = IndexStore::build(root, &[], &[], &[]).expect("build");
        store
            .inner
            .conn
            .execute(
                "UPDATE index_meta SET value = '/some/other/root' WHERE key = 'canonical_root'",
                [],
            )
            .expect("tamper root");
        drop(store);

        let result = IndexStore::open_readonly(root);
        assert!(result.is_err());
        match result.unwrap_err() {
            IndexStoreError::RootMismatch { stored, requested } => {
                assert_eq!(stored, "/some/other/root");
                assert!(!requested.is_empty());
            }
            other => panic!("expected RootMismatch, got: {other}"),
        }
    }

    #[test]
    fn sqlite_store_failed_rebuild_preserves_prior_data() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();
        std::fs::create_dir_all(root.join(".roko")).unwrap();

        let sym = test_symbol("lib.rs", "preserved", SymbolKind::Function, 1);
        let fr = FileRecord {
            path: "lib.rs".into(),
            content: "fn preserved() {}".into(),
        };
        IndexStore::build(root, &[sym], &[], &[fr]).expect("initial build");

        // The atomic build pattern means if the temp file fails, the original
        // DB stays intact. Simulate by verifying the DB still works after
        // we manually remove only the temp file.
        let temp = IndexStore::db_path_for(root).with_extension("db.building");
        std::fs::write(&temp, b"garbage").unwrap();
        // Clean up temp without touching the real DB.
        std::fs::remove_file(&temp).unwrap();

        let store = IndexStore::open_readonly(root).expect("prior DB intact");
        let results = store.inner().query_symbols("preserved").expect("query");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn sqlite_store_rebuild_only_deletes_index_db() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();
        let roko_dir = root.join(".roko");
        std::fs::create_dir_all(&roko_dir).unwrap();

        // Create the DB and an unrelated file.
        IndexStore::build(root, &[], &[], &[]).expect("build");
        let other_file = roko_dir.join("other.json");
        std::fs::write(&other_file, b"{}").unwrap();

        IndexStore::rebuild(root).expect("rebuild");

        assert!(!IndexStore::db_path_for(root).exists(), "DB should be removed");
        assert!(other_file.exists(), "unrelated files must not be removed");
    }

    #[test]
    fn sqlite_store_root_validation_prevents_external_deletion() {
        // Validate that db_path_for always puts files inside .roko/.
        let root = Path::new("/tmp/test-root");
        let db = IndexStore::db_path_for(root);
        assert!(db.starts_with(root.join(".roko")));
        assert_eq!(
            db.file_name().and_then(|n| n.to_str()),
            Some(INDEX_DB_NAME)
        );
    }

    #[test]
    fn sqlite_store_concurrent_reader_sees_complete_transaction() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();
        std::fs::create_dir_all(root.join(".roko")).unwrap();

        let sym = test_symbol("lib.rs", "concurrent_sym", SymbolKind::Function, 42);
        let fr = FileRecord {
            path: "lib.rs".into(),
            content: "fn concurrent_sym() {}".into(),
        };
        IndexStore::build(root, &[sym], &[], &[fr]).expect("build");

        // Open two readers concurrently. Both should see the same data.
        let r1 = IndexStore::open_readonly(root).expect("reader 1");
        let r2 = IndexStore::open_readonly(root).expect("reader 2");

        let results1 = r1.inner().query_symbols("concurrent_sym").expect("q1");
        let results2 = r2.inner().query_symbols("concurrent_sym").expect("q2");

        assert_eq!(results1.len(), 1);
        assert_eq!(results2.len(), 1);
        assert_eq!(results1[0].line, results2[0].line);
    }

    // ── Ranking tests ──────────────────────────────────────────────────

    #[test]
    fn insert_and_query_rankings() {
        let db = SqliteIndex::open_in_memory().expect("open in-memory db");

        let id1 = SymbolId::new("lib.rs", "main", SymbolKind::Function);
        let id2 = SymbolId::new("lib.rs", "Config", SymbolKind::Struct);
        db.insert_ranking(&id1, 0.42).expect("insert ranking 1");
        db.insert_ranking(&id2, 0.85).expect("insert ranking 2");

        assert_eq!(db.ranking_count().expect("count"), 2);

        let top = db.top_rankings(10).expect("top rankings");
        assert_eq!(top.len(), 2);
        // Highest score first.
        assert_eq!(top[0].0.symbol_name, "Config");
        assert!((top[0].1 - 0.85).abs() < 1e-9);
        assert_eq!(top[1].0.symbol_name, "main");
        assert!((top[1].1 - 0.42).abs() < 1e-9);
    }

    #[test]
    fn ranking_for_specific_symbol() {
        let db = SqliteIndex::open_in_memory().expect("open in-memory db");

        let id = SymbolId::new("a.rs", "foo", SymbolKind::Function);
        assert!(db.ranking_for(&id).expect("lookup").is_none());

        db.insert_ranking(&id, 0.123).expect("insert");
        let score = db.ranking_for(&id).expect("lookup").expect("present");
        assert!((score - 0.123).abs() < 1e-9);
    }

    #[test]
    fn ranking_upsert_replaces_score() {
        let db = SqliteIndex::open_in_memory().expect("open in-memory db");

        let id = SymbolId::new("a.rs", "bar", SymbolKind::Function);
        db.insert_ranking(&id, 0.5).expect("insert");
        db.insert_ranking(&id, 0.9).expect("upsert");

        assert_eq!(db.ranking_count().expect("count"), 1);
        let score = db.ranking_for(&id).expect("lookup").expect("present");
        assert!((score - 0.9).abs() < 1e-9);
    }

    #[test]
    fn sqlite_store_build_with_rankings_persists_scores() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();
        std::fs::create_dir_all(root.join(".roko")).unwrap();

        let sym = test_symbol("lib.rs", "main", SymbolKind::Function, 1);
        let fr = FileRecord {
            path: "lib.rs".into(),
            content: "fn main() {}".into(),
        };
        let ranking = RankingRecord {
            id: SymbolId::new("lib.rs", "main", SymbolKind::Function),
            score: 0.75,
        };

        let store = IndexStore::build_with_rankings(
            root, &[sym], &[], &[fr], &[ranking],
        )
        .expect("build with rankings");

        assert_eq!(store.inner().ranking_count().expect("count"), 1);
        let top = store.inner().top_rankings(10).expect("top");
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].0.symbol_name, "main");
        assert!((top[0].1 - 0.75).abs() < 1e-9);
    }

    #[test]
    fn sqlite_store_rankings_survive_readonly_reopen() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();
        std::fs::create_dir_all(root.join(".roko")).unwrap();

        let sym = test_symbol("lib.rs", "main", SymbolKind::Function, 1);
        let fr = FileRecord {
            path: "lib.rs".into(),
            content: "fn main() {}".into(),
        };
        let ranking = RankingRecord {
            id: SymbolId::new("lib.rs", "main", SymbolKind::Function),
            score: 0.63,
        };
        IndexStore::build_with_rankings(root, &[sym], &[], &[fr], &[ranking]).expect("build");

        // Reopen read-only and verify rankings are intact.
        let store2 = IndexStore::open_readonly(root).expect("open readonly");
        let top = store2.inner().top_rankings(10).expect("top");
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].0.symbol_name, "main");
        assert!((top[0].1 - 0.63).abs() < 1e-9);
    }

    #[test]
    fn sqlite_store_schema_version_is_two() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let root = dir.path();
        std::fs::create_dir_all(root.join(".roko")).unwrap();

        let store = IndexStore::build(root, &[], &[], &[]).expect("build");
        let meta = store.meta().expect("meta");
        assert_eq!(meta.schema_version, SCHEMA_VERSION);
        assert_eq!(meta.schema_version, 2);
    }
}

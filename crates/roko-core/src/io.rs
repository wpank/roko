//! Atomic file I/O utilities.
//!
//! All state-critical writes should use [`atomic_write`] or [`atomic_write_async`]
//! to prevent partial / corrupted files on crash.
//!
//! JSONL appenders write each row with [`append_jsonl`], [`append_jsonl_line`]
//! or [`write_jsonl_line`]: the row and its newline in one write, so rows
//! appended at once never interleave.

use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

use serde::Serialize;

/// Atomically write `data` to `path` by writing to a `.tmp` sibling, then
/// renaming.  Safe when `path` and the temp file are on the same filesystem
/// (rename is atomic on POSIX / NTFS).
///
/// Parent directories are created if they don't exist.
pub fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = tmp_path(path);
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        // Clean up the temp file on rename failure.
        let _ = std::fs::remove_file(&tmp);
    })
}

/// Async variant of [`atomic_write`].
pub async fn atomic_write_async(path: &Path, data: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let tmp = tmp_path(path);
    tokio::fs::write(&tmp, data).await?;
    if let Err(e) = tokio::fs::rename(&tmp, path).await {
        // Clean up the temp file on rename failure.
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(e);
    }
    Ok(())
}

/// Atomically write string data to `path`.
pub fn atomic_write_str(path: &Path, data: &str) -> io::Result<()> {
    atomic_write(path, data.as_bytes())
}

/// Async variant of [`atomic_write_str`].
pub async fn atomic_write_str_async(path: &Path, data: &str) -> io::Result<()> {
    atomic_write_async(path, data.as_bytes()).await
}

/// Read a file, returning `Ok(None)` for `NotFound` instead of `Err`.
///
/// This eliminates the TOCTOU race in the common pattern:
/// ```ignore
/// if path.exists() {
///     let s = fs::read_to_string(&path)?;
///     // ...
/// }
/// ```
pub fn read_optional(path: &Path) -> io::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Async variant of [`read_optional`].
pub async fn read_optional_async(path: &Path) -> io::Result<Option<String>> {
    match tokio::fs::read_to_string(path).await {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Read bytes from a file, returning `Ok(None)` for `NotFound`.
pub fn read_optional_bytes(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Write `line`, one JSONL row without its newline, to `writer` together with
/// the newline in a single `write_all`.
///
/// On a file opened for appending, one write keeps the row whole while other
/// writers, in this process or another, append to the same file. `writeln!`
/// on a `File` writes the newline in a second call, so rows appended at once
/// can interleave, and their readers drop them as malformed (bug-8417d9).
pub fn write_jsonl_line(writer: &mut impl Write, line: &str) -> io::Result<()> {
    let mut row = String::with_capacity(line.len() + 1);
    row.push_str(line);
    row.push('\n');
    writer.write_all(row.as_bytes())
}

/// Append `line`, one JSONL row without its newline, to the file at `path`
/// in one write ([`write_jsonl_line`]). The file and its parent directories
/// are created if they don't exist.
pub fn append_jsonl_line(path: &Path, line: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    write_jsonl_line(&mut file, line)
}

/// Serialize `value` and append it to the file at `path` as one JSONL row
/// ([`append_jsonl_line`]).
///
/// # Errors
///
/// Returns an [`io::ErrorKind::InvalidData`] error if `value` cannot be
/// serialized, and any error creating, opening or writing the file.
pub fn append_jsonl<T: Serialize + ?Sized>(path: &Path, value: &T) -> io::Result<()> {
    let line = serde_json::to_string(value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    append_jsonl_line(path, &line)
}

/// Process-global monotonic counter to prevent tmp-file collisions between
/// concurrent writers targeting the same destination path.
static TMP_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Compute a unique temporary file path for atomic writes.
///
/// Format: `<path>.tmp.<pid>.<counter>` — unique per-process and per-call,
/// so concurrent writers to the same destination never collide.
fn tmp_path(path: &Path) -> std::path::PathBuf {
    let pid = std::process::id();
    let seq = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(".tmp.{pid}.{seq}"));
    std::path::PathBuf::from(tmp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn atomic_write_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        atomic_write(&path, b"hello").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn atomic_write_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b/c/test.txt");
        atomic_write(&path, b"nested").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "nested");
    }

    #[test]
    fn atomic_write_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        atomic_write(&path, b"first").unwrap();
        atomic_write(&path, b"second").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "second");
    }

    #[test]
    fn atomic_write_no_tmp_file_remains() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        atomic_write(&path, b"data").unwrap();
        // No .tmp.* siblings should remain after a successful write.
        let siblings: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("test.txt.tmp."))
            .collect();
        assert!(siblings.is_empty(), "temp file should be cleaned up");
    }

    #[test]
    fn read_optional_returns_none_for_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.txt");
        assert_eq!(read_optional(&path).unwrap(), None);
    }

    #[test]
    fn read_optional_returns_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "content").unwrap();
        assert_eq!(read_optional(&path).unwrap(), Some("content".to_owned()));
    }

    #[test]
    fn read_optional_bytes_returns_none_for_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.bin");
        assert_eq!(read_optional_bytes(&path).unwrap(), None);
    }

    #[test]
    fn atomic_write_str_works() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        atomic_write_str(&path, "string data").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "string data");
    }

    #[tokio::test]
    async fn atomic_write_async_works() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("async_test.txt");
        atomic_write_async(&path, b"async hello").await.unwrap();
        assert_eq!(
            tokio::fs::read_to_string(&path).await.unwrap(),
            "async hello"
        );
    }

    #[tokio::test]
    async fn read_optional_async_returns_none_for_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.txt");
        assert_eq!(read_optional_async(&path).await.unwrap(), None);
    }

    /// A writer that records each `write` call it gets.
    struct Writes(Vec<Vec<u8>>);

    impl Write for Writes {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.push(buf.to_vec());
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_jsonl_row_and_its_newline_are_one_write() {
        let mut writes = Writes(Vec::new());
        write_jsonl_line(&mut writes, r#"{"id":1}"#).unwrap();
        assert_eq!(writes.0, [b"{\"id\":1}\n".to_vec()]);
    }

    /// bug-8417d9: rows that threads append to one file at once each land
    /// whole, on a line of their own.
    #[test]
    fn concurrent_jsonl_appends_never_interleave() {
        const WRITERS: u64 = 16;
        const ROWS: u64 = 50;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("learn/rows.jsonl");
        let start = std::sync::Arc::new(std::sync::Barrier::new(WRITERS as usize));
        let writers: Vec<_> = (0..WRITERS)
            .map(|writer| {
                let (path, start) = (path.clone(), std::sync::Arc::clone(&start));
                std::thread::spawn(move || {
                    start.wait();
                    for row in 0..ROWS {
                        let value = serde_json::json!({
                            "writer": writer,
                            "row": row,
                            "pad": "x".repeat(512),
                        });
                        append_jsonl(&path, &value).unwrap();
                    }
                })
            })
            .collect();
        for writer in writers {
            writer.join().unwrap();
        }

        let mut rows: Vec<(u64, u64)> = fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(|line| {
                let row: serde_json::Value = serde_json::from_str(line)
                    .unwrap_or_else(|error| panic!("a whole row, not {line:?}: {error}"));
                (
                    row["writer"].as_u64().unwrap(),
                    row["row"].as_u64().unwrap(),
                )
            })
            .collect();
        rows.sort_unstable();
        let expected: Vec<(u64, u64)> = (0..WRITERS)
            .flat_map(|writer| (0..ROWS).map(move |row| (writer, row)))
            .collect();
        assert_eq!(rows, expected);
    }

    #[tokio::test]
    async fn read_optional_async_returns_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        tokio::fs::write(&path, "async content").await.unwrap();
        assert_eq!(
            read_optional_async(&path).await.unwrap(),
            Some("async content".to_owned())
        );
    }
}

//! Explicit, bounded repair for historical derived per-run event indexes.
//!
//! This command is intentionally disconnected from server startup and HTTP
//! request handling. Global JSONL generations remain authoritative; indexes
//! are disposable read projections rebuilt only after an explicit `--apply`.

use crate::{Cli, EXIT_FAILURE, EXIT_SUCCESS, RunIndexCmd, resolve_workdir};
use anyhow::{Context as _, Result, bail};
use fs2::FileExt as _;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write as _};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_JSONL_RECORD_BYTES: usize = 256 * 1024;
const MAX_OPEN_STAGING_WRITERS: usize = 32;
const MAX_DISCOVERY_ENTRIES: usize = 8_192;

#[derive(Debug, Serialize)]
struct RepairReport {
    workdir: PathBuf,
    dry_run: bool,
    status: &'static str,
    applied: bool,
    truncated: bool,
    truncation_reason: Option<&'static str>,
    max_bytes: u64,
    max_records: u64,
    max_indexes: usize,
    deadline_secs: u64,
    bytes_scanned: u64,
    records_seen: u64,
    records_indexed: u64,
    malformed_records: u64,
    partial_records: u64,
    oversized_records: u64,
    missing_run_ids: u64,
    invalid_run_ids: u64,
    cross_run_records: u64,
    distinct_indexes: usize,
    files_replaced: usize,
    sources: Vec<SourceReport>,
}

#[derive(Debug, Serialize)]
struct SourceReport {
    kind: &'static str,
    path: PathBuf,
    bytes_scanned: u64,
    records_seen: u64,
}

struct RepairLimits {
    max_bytes: u64,
    max_records: u64,
    max_indexes: usize,
    deadline: Instant,
}

#[derive(Default)]
struct ScanState {
    bytes_scanned: u64,
    records_seen: u64,
    records_indexed: u64,
    malformed_records: u64,
    partial_records: u64,
    oversized_records: u64,
    missing_run_ids: u64,
    invalid_run_ids: u64,
    cross_run_records: u64,
    truncated: bool,
    truncation_reason: Option<&'static str>,
    sources: Vec<SourceReport>,
}

pub(crate) async fn cmd_run_index(cli: &Cli, cmd: RunIndexCmd) -> Result<i32> {
    let RunIndexCmd::Repair {
        apply,
        workdir,
        max_bytes,
        max_records,
        max_indexes,
        deadline_secs,
    } = cmd;
    if max_bytes == 0 {
        bail!("--max-bytes must be greater than zero");
    }
    if max_records == 0 {
        bail!("--max-records must be greater than zero");
    }
    if max_indexes == 0 {
        bail!("--max-indexes must be greater than zero");
    }
    if deadline_secs == 0 {
        bail!("--deadline-secs must be greater than zero");
    }

    let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
    let report = tokio::task::spawn_blocking(move || {
        repair_indexes(
            &workdir,
            apply,
            max_bytes,
            max_records,
            max_indexes,
            deadline_secs,
        )
    })
    .await
    .context("run-index repair worker panicked")??;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_report(&report);
    }
    Ok(if report.truncated || (apply && !report.applied) {
        EXIT_FAILURE
    } else {
        EXIT_SUCCESS
    })
}

fn repair_indexes(
    workdir: &Path,
    apply: bool,
    max_bytes: u64,
    max_records: u64,
    max_indexes: usize,
    deadline_secs: u64,
) -> Result<RepairReport> {
    let workdir = workdir
        .canonicalize()
        .with_context(|| format!("canonicalize workspace {}", workdir.display()))?;
    let roko_dir = workdir.join(".roko");
    reject_symlink(&roko_dir, "workspace state directory")?;
    let roko_canonical = roko_dir
        .canonicalize()
        .with_context(|| format!("canonicalize {}", roko_dir.display()))?;
    if !roko_canonical.starts_with(&workdir) {
        bail!("workspace state directory escapes the selected workspace");
    }

    let deadline = Instant::now()
        .checked_add(Duration::from_secs(deadline_secs))
        .context("--deadline-secs is too large")?;

    // Holding these locks makes the scan/replacement a genuinely offline
    // operation. Dry runs only inspect already-existing locks and never create
    // maintenance state; apply creates and holds the full lock set.
    let _locks = RepairLocks::acquire(&roko_canonical, apply)?;
    let runner_live = roko_canonical.join("events.jsonl");
    let runtime_live = roko_canonical.join("runtime-events.jsonl");
    let mut scan = ScanState::default();
    let mut inspected_entries = 0usize;
    let mut inputs = Vec::new();
    for (live, kind) in [(&runner_live, "runner"), (&runtime_live, "runtime")] {
        let (discovered, truncation) =
            discover_generations(live, kind, deadline, &mut inspected_entries)?;
        inputs.extend(discovered);
        if let Some(reason) = truncation {
            mark_truncated(&mut scan, reason);
            break;
        }
    }
    let limits = RepairLimits {
        max_bytes,
        max_records,
        max_indexes,
        deadline,
    };
    let mut planned = BTreeSet::new();
    let mut staging = apply.then(|| StagingSet::new(roko_canonical.clone()));

    for input in &inputs {
        if scan.truncated {
            break;
        }
        scan_source(
            input,
            &roko_canonical,
            &limits,
            &mut scan,
            &mut planned,
            staging.as_mut(),
        )?;
    }

    let mut files_replaced = 0;
    let applied = if apply && !scan.truncated {
        let staging = staging
            .as_mut()
            .context("staging is unavailable for apply")?;
        if staging.prepare_commit(deadline)? {
            files_replaced = staging.commit_prepared()?;
            true
        } else {
            mark_truncated(&mut scan, "deadline");
            false
        }
    } else {
        false
    };
    let status = match (apply, applied, scan.truncated) {
        (_, _, true) => "incomplete_not_applied",
        (false, _, false) => "ready_dry_run",
        (true, true, false) => "repaired",
        (true, false, false) => "not_applied",
    };

    Ok(RepairReport {
        workdir,
        dry_run: !apply,
        status,
        applied,
        truncated: scan.truncated,
        truncation_reason: scan.truncation_reason,
        max_bytes,
        max_records,
        max_indexes,
        deadline_secs,
        bytes_scanned: scan.bytes_scanned,
        records_seen: scan.records_seen,
        records_indexed: scan.records_indexed,
        malformed_records: scan.malformed_records,
        partial_records: scan.partial_records,
        oversized_records: scan.oversized_records,
        missing_run_ids: scan.missing_run_ids,
        invalid_run_ids: scan.invalid_run_ids,
        cross_run_records: scan.cross_run_records,
        distinct_indexes: planned.len(),
        files_replaced,
        sources: scan.sources,
    })
}

#[derive(Debug)]
struct InputGeneration {
    kind: &'static str,
    live_path: PathBuf,
    path: PathBuf,
}

fn discover_generations(
    live_path: &Path,
    kind: &'static str,
    deadline: Instant,
    inspected_entries: &mut usize,
) -> Result<(Vec<InputGeneration>, Option<&'static str>)> {
    let Some(parent) = live_path.parent() else {
        bail!("global log has no parent: {}", live_path.display());
    };
    let stem = live_path
        .file_stem()
        .and_then(|value| value.to_str())
        .context("global log has no UTF-8 stem")?;
    let mut paths = Vec::new();
    let mut truncation = None;
    for entry in std::fs::read_dir(parent)
        .with_context(|| format!("read global log directory {}", parent.display()))?
    {
        if Instant::now() >= deadline {
            truncation = Some("deadline");
            break;
        }
        if *inspected_entries >= MAX_DISCOVERY_ENTRIES {
            truncation = Some("discovery_entries");
            break;
        }
        *inspected_entries += 1;
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let prefix = format!("{stem}.");
        let Some(middle) = name
            .strip_prefix(&prefix)
            .and_then(|value| value.strip_suffix(".jsonl"))
        else {
            continue;
        };
        if timestamp_like(middle) {
            paths.push(entry.path());
        }
    }
    paths.sort();
    if live_path.exists() {
        paths.push(live_path.to_path_buf());
    }
    let inputs = paths
        .into_iter()
        .map(|path| {
            reject_regular_file_inside(&path, parent)?;
            Ok(InputGeneration {
                kind,
                live_path: live_path.to_path_buf(),
                path,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok((inputs, truncation))
}

fn timestamp_like(value: &str) -> bool {
    (15..=40).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'T' | b'Z' | b'-' | b'_' | b'.'))
}

fn scan_source(
    input: &InputGeneration,
    roko_dir: &Path,
    limits: &RepairLimits,
    scan: &mut ScanState,
    planned: &mut BTreeSet<PathBuf>,
    mut staging: Option<&mut StagingSet>,
) -> Result<()> {
    let file = File::open(&input.path)
        .with_context(|| format!("open global event log {}", input.path.display()))?;
    let mut reader = BufReader::new(file);
    let source_start_bytes = scan.bytes_scanned;
    let source_start_records = scan.records_seen;

    loop {
        // Probe EOF before enforcing aggregate limits so a source whose exact
        // size/count equals a configured cap is reported complete rather than
        // spuriously truncated.
        if reader.fill_buf()?.is_empty() {
            break;
        }
        if Instant::now() >= limits.deadline {
            mark_truncated(scan, "deadline");
            break;
        }
        if scan.bytes_scanned >= limits.max_bytes {
            mark_truncated(scan, "max_bytes");
            break;
        }
        if scan.records_seen >= limits.max_records {
            mark_truncated(scan, "max_records");
            break;
        }
        let remaining = limits.max_bytes - scan.bytes_scanned;
        match read_bounded_line(&mut reader, remaining, limits.deadline)? {
            LineRead::Eof => break,
            LineRead::BudgetExhausted(bytes) => {
                scan.bytes_scanned = scan.bytes_scanned.saturating_add(bytes);
                mark_truncated(scan, "max_bytes");
                break;
            }
            LineRead::Deadline(bytes) => {
                scan.bytes_scanned = scan.bytes_scanned.saturating_add(bytes);
                mark_truncated(scan, "deadline");
                break;
            }
            LineRead::Partial(bytes, raw) => {
                scan.bytes_scanned = scan.bytes_scanned.saturating_add(bytes);
                scan.records_seen += 1;
                scan.partial_records += 1;
                scan.malformed_records += 1;
                drop(raw);
                break;
            }
            LineRead::Oversized(bytes) => {
                scan.bytes_scanned = scan.bytes_scanned.saturating_add(bytes);
                scan.records_seen += 1;
                scan.oversized_records += 1;
            }
            LineRead::Complete(bytes, raw) => {
                scan.bytes_scanned = scan.bytes_scanned.saturating_add(bytes);
                scan.records_seen += 1;
                let value: Value = match serde_json::from_slice(trim_newline(&raw)) {
                    Ok(value) => value,
                    Err(_) => {
                        scan.malformed_records += 1;
                        continue;
                    }
                };
                let run_id = match validated_run_id(&value) {
                    Ok(run_id) => run_id,
                    Err(RecordRejection::Missing) => {
                        scan.missing_run_ids += 1;
                        continue;
                    }
                    Err(RecordRejection::Invalid) => {
                        scan.invalid_run_ids += 1;
                        continue;
                    }
                    Err(RecordRejection::CrossRun) => {
                        scan.cross_run_records += 1;
                        continue;
                    }
                };
                let final_path = roko_fs::run_index::run_index_path(&input.live_path, run_id)
                    .map_err(anyhow::Error::msg)?;
                ensure_index_path(&final_path, roko_dir)?;
                if !planned.contains(&final_path) && planned.len() >= limits.max_indexes {
                    mark_truncated(scan, "max_indexes");
                    break;
                }
                planned.insert(final_path.clone());
                if let Some(staging) = staging.as_deref_mut() {
                    staging.append(final_path, &raw)?;
                }
                scan.records_indexed += 1;
            }
        }
    }

    scan.sources.push(SourceReport {
        kind: input.kind,
        path: input.path.clone(),
        bytes_scanned: scan.bytes_scanned - source_start_bytes,
        records_seen: scan.records_seen - source_start_records,
    });
    Ok(())
}

enum RecordRejection {
    Missing,
    Invalid,
    CrossRun,
}

fn validated_run_id(value: &Value) -> std::result::Result<&str, RecordRejection> {
    let Some(run_id) = value.get("run_id").and_then(Value::as_str) else {
        return Err(RecordRejection::Missing);
    };
    if roko_fs::run_index::validate_scoped_id(run_id).is_err() {
        return Err(RecordRejection::Invalid);
    }
    // Runtime envelopes carry the same ownership in `payload.data.run_id`.
    // A legacy nested `event.run_id` is also checked when present. Arbitrary
    // user payloads are deliberately not recursively searched.
    for pointer in ["/payload/data/run_id", "/event/run_id"] {
        if let Some(nested) = value.pointer(pointer) {
            let Some(nested) = nested.as_str() else {
                return Err(RecordRejection::Invalid);
            };
            if roko_fs::run_index::validate_scoped_id(nested).is_err() {
                return Err(RecordRejection::Invalid);
            }
            if nested != run_id {
                return Err(RecordRejection::CrossRun);
            }
        }
    }
    Ok(run_id)
}

enum LineRead {
    Eof,
    Complete(u64, Vec<u8>),
    Partial(u64, Vec<u8>),
    Oversized(u64),
    BudgetExhausted(u64),
    Deadline(u64),
}

fn read_bounded_line(
    reader: &mut impl BufRead,
    mut budget: u64,
    deadline: Instant,
) -> std::io::Result<LineRead> {
    let mut raw = Vec::new();
    let mut consumed = 0_u64;
    let mut oversized = false;
    loop {
        if Instant::now() >= deadline {
            return Ok(LineRead::Deadline(consumed));
        }
        if budget == 0 {
            return Ok(LineRead::BudgetExhausted(consumed));
        }
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(if consumed == 0 {
                LineRead::Eof
            } else if oversized {
                LineRead::Oversized(consumed)
            } else {
                LineRead::Partial(consumed, raw)
            });
        }
        let through_newline = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |position| position + 1);
        let take = through_newline.min(budget.min(usize::MAX as u64) as usize);
        if !oversized {
            if raw.len().saturating_add(take) <= MAX_JSONL_RECORD_BYTES {
                raw.extend_from_slice(&available[..take]);
            } else {
                raw.clear();
                oversized = true;
            }
        }
        let found_newline = available[..take].last() == Some(&b'\n');
        reader.consume(take);
        let taken = take as u64;
        budget -= taken;
        consumed += taken;
        if found_newline {
            return Ok(if oversized {
                LineRead::Oversized(consumed)
            } else {
                LineRead::Complete(consumed, raw)
            });
        }
    }
}

fn trim_newline(raw: &[u8]) -> &[u8] {
    let raw = raw.strip_suffix(b"\n").unwrap_or(raw);
    raw.strip_suffix(b"\r").unwrap_or(raw)
}

fn mark_truncated(scan: &mut ScanState, reason: &'static str) {
    scan.truncated = true;
    scan.truncation_reason = Some(reason);
}

struct StagedFile {
    temp_path: PathBuf,
    final_path: PathBuf,
    writer: Option<BufWriter<File>>,
}

struct StagingSet {
    roko_dir: PathBuf,
    files: BTreeMap<PathBuf, StagedFile>,
    open_writers: usize,
    committed: bool,
    nonce: u128,
}

impl StagingSet {
    fn new(roko_dir: PathBuf) -> Self {
        Self {
            roko_dir,
            files: BTreeMap::new(),
            open_writers: 0,
            committed: false,
            nonce: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
        }
    }

    fn append(&mut self, final_path: PathBuf, raw: &[u8]) -> Result<()> {
        if !self.files.contains_key(&final_path) {
            self.create_staged_file(final_path.clone())?;
        }
        if self
            .files
            .get(&final_path)
            .is_some_and(|staged| staged.writer.is_none())
        {
            self.open_writer(&final_path)?;
        }
        let staged = self.files.get_mut(&final_path).expect("staged file exists");
        staged
            .writer
            .as_mut()
            .expect("writer is open")
            .write_all(raw)
            .with_context(|| format!("stage run index {}", final_path.display()))?;
        Ok(())
    }

    fn create_staged_file(&mut self, final_path: PathBuf) -> Result<()> {
        ensure_index_path(&final_path, &self.roko_dir)?;
        let parent = final_path.parent().context("run index has no parent")?;
        if parent.exists() {
            reject_symlink(parent, "run index directory")?;
        } else {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create run index directory {}", parent.display()))?;
        }
        ensure_directory_inside(parent, &self.roko_dir)?;
        if final_path.exists() {
            reject_regular_file_inside(&final_path, parent)?;
        }
        let name = final_path
            .file_name()
            .and_then(|value| value.to_str())
            .context("run index has no UTF-8 filename")?;
        let temp_path = parent.join(format!(
            ".{name}.repair-{}-{}.tmp",
            std::process::id(),
            self.nonce
        ));
        reject_absent(&temp_path, "repair staging file")?;
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)
            .with_context(|| format!("create staging file {}", temp_path.display()))?;
        self.files.insert(
            final_path.clone(),
            StagedFile {
                temp_path,
                final_path,
                writer: None,
            },
        );
        Ok(())
    }

    fn open_writer(&mut self, final_path: &Path) -> Result<()> {
        if self.open_writers >= MAX_OPEN_STAGING_WRITERS {
            let close_path = self
                .files
                .iter()
                .find_map(|(path, staged)| staged.writer.is_some().then(|| path.clone()));
            if let Some(close_path) = close_path {
                self.close_writer(&close_path)?;
            }
        }
        let staged = self
            .files
            .get_mut(final_path)
            .context("missing staging file")?;
        let file = OpenOptions::new()
            .append(true)
            .open(&staged.temp_path)
            .with_context(|| format!("open staging file {}", staged.temp_path.display()))?;
        staged.writer = Some(BufWriter::with_capacity(64 * 1024, file));
        self.open_writers += 1;
        Ok(())
    }

    fn close_writer(&mut self, final_path: &Path) -> Result<()> {
        let staged = self
            .files
            .get_mut(final_path)
            .context("missing staging file")?;
        if let Some(mut writer) = staged.writer.take() {
            writer.flush()?;
            self.open_writers = self.open_writers.saturating_sub(1);
        }
        Ok(())
    }

    fn prepare_commit(&mut self, deadline: Instant) -> Result<bool> {
        let paths = self.files.keys().cloned().collect::<Vec<_>>();
        for path in &paths {
            if Instant::now() >= deadline {
                return Ok(false);
            }
            self.close_writer(path)?;
        }
        for staged in self.files.values() {
            if Instant::now() >= deadline {
                return Ok(false);
            }
            File::open(&staged.temp_path)?.sync_all()?;
            ensure_index_path(&staged.final_path, &self.roko_dir)?;
            if staged.final_path.exists() {
                reject_regular_file_inside(
                    &staged.final_path,
                    staged.final_path.parent().context("run index parent")?,
                )?;
            }
        }
        Ok(Instant::now() < deadline)
    }

    fn commit_prepared(&mut self) -> Result<usize> {
        for staged in self.files.values() {
            std::fs::rename(&staged.temp_path, &staged.final_path).with_context(|| {
                format!(
                    "atomically replace {} from {}",
                    staged.final_path.display(),
                    staged.temp_path.display()
                )
            })?;
        }
        let mut parents = BTreeSet::new();
        for staged in self.files.values() {
            if let Some(parent) = staged.final_path.parent() {
                parents.insert(parent);
            }
        }
        for parent in parents {
            // The file replacement itself is already atomic. Directory fsync
            // is best effort because some supported filesystems reject fsync
            // on directory descriptors even after a successful rename.
            if let Ok(directory) = File::open(parent) {
                let _ = directory.sync_all();
            }
        }
        self.committed = true;
        Ok(self.files.len())
    }
}

impl Drop for StagingSet {
    fn drop(&mut self) {
        if !self.committed {
            for staged in self.files.values_mut() {
                staged.writer.take();
                let _ = std::fs::remove_file(&staged.temp_path);
            }
        }
    }
}

struct RepairLocks {
    files: Vec<File>,
}

impl RepairLocks {
    fn acquire(roko_dir: &Path, apply: bool) -> Result<Self> {
        let runtime = roko_dir.join("runtime");
        if apply {
            if runtime.exists() {
                reject_symlink(&runtime, "runtime lock directory")?;
            } else {
                std::fs::create_dir_all(&runtime)?;
            }
            ensure_directory_inside(&runtime, roko_dir)?;
        }
        let mut paths = vec![
            runtime.join("cache-gc.lock"),
            runtime.join("roko.lock"),
            runtime.join("run-index-repair.lock"),
            roko_dir.join("events.jsonl.lock"),
            roko_dir.join("runtime-events.jsonl.lock"),
        ];
        paths.sort();
        let mut files = Vec::new();
        for path in paths {
            if !apply && !path.exists() {
                continue;
            }
            reject_symlink_if_present(&path, "maintenance lock")?;
            let file = OpenOptions::new()
                .create(apply)
                .read(true)
                .write(true)
                .open(&path)
                .with_context(|| format!("open maintenance lock {}", path.display()))?;
            file.try_lock_exclusive().map_err(|error| {
                anyhow::anyhow!(
                    "offline run-index repair refused: active writer or GC lock at {}: {error}",
                    path.display()
                )
            })?;
            files.push(file);
        }
        Ok(Self { files })
    }
}

impl Drop for RepairLocks {
    fn drop(&mut self) {
        for file in &self.files {
            let _ = file.unlock();
        }
    }
}

fn ensure_index_path(path: &Path, roko_dir: &Path) -> Result<()> {
    let parent = path.parent().context("run index has no parent")?;
    if !parent.starts_with(roko_dir) {
        bail!(
            "derived run index escapes workspace state: {}",
            path.display()
        );
    }
    reject_symlink_if_present(parent, "run index directory")?;
    reject_symlink_if_present(path, "run index file")?;
    Ok(())
}

fn reject_regular_file_inside(path: &Path, expected_parent: &Path) -> Result<()> {
    reject_symlink(path, "JSONL file")?;
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_file() {
        bail!("expected regular file: {}", path.display());
    }
    let canonical = path.canonicalize()?;
    let parent = expected_parent.canonicalize()?;
    if canonical.parent() != Some(parent.as_path()) {
        bail!("file escapes expected directory: {}", path.display());
    }
    Ok(())
}

fn ensure_directory_inside(path: &Path, root: &Path) -> Result<()> {
    reject_symlink(path, "directory")?;
    let canonical = path.canonicalize()?;
    if !canonical.starts_with(root) {
        bail!("directory escapes workspace state: {}", path.display());
    }
    Ok(())
}

fn reject_symlink(path: &Path, label: &str) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("inspect {label} {}", path.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("{label} must not be a symlink: {}", path.display());
    }
    Ok(())
}

fn reject_symlink_if_present(path: &Path, label: &str) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!("{label} must not be a symlink: {}", path.display())
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn reject_absent(path: &Path, label: &str) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => bail!("{label} already exists: {}", path.display()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn print_report(report: &RepairReport) {
    println!("run-index repair: {}", report.workdir.display());
    println!("  status:             {}", report.status);
    println!(
        "  bytes scanned:      {} / {}",
        report.bytes_scanned, report.max_bytes
    );
    println!(
        "  records inspected:  {} / {}",
        report.records_seen, report.max_records
    );
    println!("  accepted records:   {}", report.records_indexed);
    println!(
        "  distinct indexes:   {} / {}",
        report.distinct_indexes, report.max_indexes
    );
    println!(
        "  rejected records:   {} malformed, {} partial, {} oversized, {} missing id, {} invalid id, {} cross-run",
        report.malformed_records,
        report.partial_records,
        report.oversized_records,
        report.missing_run_ids,
        report.invalid_run_ids,
        report.cross_run_records
    );
    if report.truncated {
        println!(
            "incomplete: {} limit reached; no index was replaced",
            report.truncation_reason.unwrap_or("unknown")
        );
    } else if report.dry_run {
        println!("dry run: nothing changed; rerun with `--apply` to replace these indexes");
    } else {
        println!(
            "repaired: {} index files atomically replaced",
            report.files_replaced
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_envelope_ownership_and_rejects_cross_run_records() {
        let valid = serde_json::json!({
            "run_id": "run-1",
            "payload": {"kind": "workflow_started", "data": {"run_id": "run-1"}}
        });
        assert_eq!(validated_run_id(&valid).ok(), Some("run-1"));
        let cross_run = serde_json::json!({
            "run_id": "run-1",
            "payload": {"kind": "workflow_started", "data": {"run_id": "run-2"}}
        });
        assert!(matches!(
            validated_run_id(&cross_run),
            Err(RecordRejection::CrossRun)
        ));
    }

    #[test]
    fn truncated_apply_discards_staging_without_replacing_indexes() {
        let workspace = tempfile::tempdir().unwrap();
        let roko = workspace.path().join(".roko");
        std::fs::create_dir_all(roko.join("runtime")).unwrap();
        std::fs::write(
            roko.join("events.jsonl"),
            b"{\"type\":\"run.started\",\"run_id\":\"old-run\"}\n",
        )
        .unwrap();
        let report = repair_indexes(workspace.path(), true, 8, 100, 100, 10).unwrap();
        assert!(report.truncated);
        assert!(!report.applied);
        let index =
            roko_fs::run_index::run_index_path(&roko.join("events.jsonl"), "old-run").unwrap();
        assert!(!index.exists());
    }

    #[test]
    fn apply_rebuilds_hashed_indexes_and_rejects_bad_records() {
        let workspace = tempfile::tempdir().unwrap();
        let roko = workspace.path().join(".roko");
        std::fs::create_dir_all(roko.join("runtime")).unwrap();
        std::fs::write(
            roko.join("events.jsonl"),
            concat!(
                "{\"type\":\"run.started\",\"run_id\":\"run-1\"}\n",
                "{not-json}\n",
                "{\"run_id\":\"../escape\"}\n"
            ),
        )
        .unwrap();
        let report = repair_indexes(workspace.path(), true, 1_000_000, 100, 100, 10).unwrap();
        assert!(report.applied);
        assert_eq!(report.records_indexed, 1);
        assert_eq!(report.malformed_records, 1);
        assert_eq!(report.invalid_run_ids, 1);
        let index =
            roko_fs::run_index::run_index_path(&roko.join("events.jsonl"), "run-1").unwrap();
        assert!(index.exists());
        assert!(!index.to_string_lossy().contains("run-1"));
    }

    #[test]
    fn distinct_index_limit_fails_closed_without_replacement() {
        let workspace = tempfile::tempdir().unwrap();
        let roko = workspace.path().join(".roko");
        std::fs::create_dir_all(roko.join("runtime")).unwrap();
        std::fs::write(
            roko.join("events.jsonl"),
            b"{\"run_id\":\"run-1\"}\n{\"run_id\":\"run-2\"}\n",
        )
        .unwrap();

        let report = repair_indexes(workspace.path(), true, 1_000_000, 100, 1, 10).unwrap();

        assert!(report.truncated);
        assert_eq!(report.truncation_reason, Some("max_indexes"));
        assert!(!report.applied);
        assert_eq!(report.distinct_indexes, 1);
    }

    #[test]
    fn active_workspace_lock_refuses_apply() {
        let workspace = tempfile::tempdir().unwrap();
        let runtime = workspace.path().join(".roko/runtime");
        std::fs::create_dir_all(&runtime).unwrap();
        let lock = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(runtime.join("roko.lock"))
            .unwrap();
        lock.lock_exclusive().unwrap();
        let error = repair_indexes(workspace.path(), true, 1_000, 100, 100, 10)
            .expect_err("active writer lock must refuse repair");
        assert!(error.to_string().contains("active writer or GC lock"));
        lock.unlock().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_index_directory_is_rejected() {
        use std::os::unix::fs::symlink;

        let workspace = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let roko = workspace.path().join(".roko");
        std::fs::create_dir_all(roko.join("runtime")).unwrap();
        std::fs::write(
            roko.join("events.jsonl"),
            b"{\"type\":\"run.started\",\"run_id\":\"run-1\"}\n",
        )
        .unwrap();
        symlink(outside.path(), roko.join("events-by-run")).unwrap();
        let error = repair_indexes(workspace.path(), true, 1_000, 100, 100, 10)
            .expect_err("symlinked output must fail closed");
        assert!(error.to_string().contains("symlink"));
        assert!(std::fs::read_dir(outside.path()).unwrap().next().is_none());
    }
}

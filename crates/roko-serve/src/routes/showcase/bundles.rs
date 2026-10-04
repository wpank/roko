//! The showcase bundle loader (S10 §4.5): every bundle under `[showcase] bundle_root` is checked
//! before any of it is served.
//!
//! A bundle is `rejected` with a reason when its `SHA256SUMS` does not match its files exactly
//! (`integrity`), its manifest is not a `showcase-bundle/1` replay (`manifest`, `kind`), any
//! JSON in it says `simulated` other than `false` (`simulated`), a metric has no `run_ids`
//! (`no_run_ids`), a metric counts a run that ended in `infra_error` or `leak_suspected`
//! (`infra_error_counted`), or a view it names is missing (`views`). The routes serve nothing
//! from a rejected bundle; they answer `409 bundle_rejected`. Handlers never read
//! `.roko/bench/runs/*`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, PoisonError, RwLock};

use serde_json::Value;
use sha2::{Digest, Sha256};

/// `bundle.json`'s schema.
pub const BUNDLE_SCHEMA: &str = "showcase-bundle/1";

/// The views of the contract (S10 §5.2) a bundle may carry, by id.
pub const VIEW_IDS: [&str; 9] = [
    "overview",
    "p1-head-to-head",
    "p1-routing",
    "p1-specs",
    "m1-essential-variables",
    "m1-episodes",
    "m3-calibration",
    "m4-audits",
    "bench-curves",
];

/// The checksum file every bundle carries.
const SUMS_FILE: &str = "SHA256SUMS";

/// Run statuses that are excluded from results, so no metric may count them.
const EXCLUDED_RUN_STATUSES: [&str; 2] = ["infra_error", "leak_suspected"];

/// Whether a bundle may be served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleStatus {
    /// Every rule holds.
    Verified,
    /// A rule failed; holds its reason.
    Rejected(String),
}

/// One bundle as the loader found it.
#[derive(Debug, Clone)]
pub struct LoadedBundle {
    /// The bundle's directory name.
    pub id: String,
    pub dir: PathBuf,
    pub status: BundleStatus,
    pub title: String,
    pub created_at: String,
    pub featured: bool,
    pub harness_commit: Option<String>,
    /// The views the manifest names.
    pub views: Vec<String>,
    /// The SHA-256 of each file, by its path in the bundle, as `SHA256SUMS` lists it.
    pub sums: BTreeMap<String, String>,
}

impl LoadedBundle {
    /// The status word of the manifest: `verified` or `rejected`.
    pub fn status_word(&self) -> &'static str {
        match self.status {
            BundleStatus::Verified => "verified",
            BundleStatus::Rejected(_) => "rejected",
        }
    }
}

/// Every bundle under one root, as last loaded, sorted by id.
#[derive(Debug, Default)]
pub struct Catalog {
    pub bundles: Vec<LoadedBundle>,
}

impl Catalog {
    /// Load and check every bundle directory under `root`; a missing root holds none.
    pub fn load(root: &Path) -> Self {
        let Ok(entries) = std::fs::read_dir(root) else {
            return Self::default();
        };
        let mut dirs: Vec<(String, PathBuf)> = entries
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
            .filter_map(|entry| Some((entry.file_name().to_str()?.to_string(), entry.path())))
            .filter(|(id, _)| !id.starts_with('.'))
            .collect();
        dirs.sort();
        let bundles = dirs
            .into_iter()
            .map(|(id, dir)| load_bundle(id, dir))
            .collect();
        Self { bundles }
    }

    /// The bundle called `id`.
    pub fn get(&self, id: &str) -> Option<&LoadedBundle> {
        self.bundles.iter().find(|bundle| bundle.id == id)
    }

    /// The bundle the showcase opens on: the newest verified one marked `featured`, else the
    /// newest verified one.
    pub fn featured(&self) -> Option<&LoadedBundle> {
        let verified = || {
            self.bundles
                .iter()
                .filter(|bundle| bundle.status == BundleStatus::Verified)
        };
        verified()
            .filter(|bundle| bundle.featured)
            .max_by(|a, b| a.created_at.cmp(&b.created_at))
            .or_else(|| verified().max_by(|a, b| a.created_at.cmp(&b.created_at)))
    }
}

/// The catalogue the routes read: loaded on first use, and again on an admin reload or when
/// `bundle_root` changes.
#[derive(Debug, Default)]
pub struct BundleCache {
    loaded: RwLock<Option<(PathBuf, Arc<Catalog>)>>,
}

impl BundleCache {
    /// The catalogue of `root`, loading it when it is not cached.
    pub fn catalog(&self, root: &Path) -> Arc<Catalog> {
        let cached = self.loaded.read().unwrap_or_else(PoisonError::into_inner);
        if let Some((cached_root, catalog)) = cached.as_ref()
            && cached_root == root
        {
            return Arc::clone(catalog);
        }
        drop(cached);
        self.reload(root)
    }

    /// Load `root` again, replacing the cached catalogue.
    pub fn reload(&self, root: &Path) -> Arc<Catalog> {
        let catalog = Arc::new(Catalog::load(root));
        *self.loaded.write().unwrap_or_else(PoisonError::into_inner) =
            Some((root.to_path_buf(), Arc::clone(&catalog)));
        catalog
    }
}

/// The hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            use std::fmt::Write as _;
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

/// Read and check the bundle in `dir`.
fn load_bundle(id: String, dir: PathBuf) -> LoadedBundle {
    let mut bundle = LoadedBundle {
        id,
        dir,
        status: BundleStatus::Verified,
        title: String::new(),
        created_at: String::new(),
        featured: false,
        harness_commit: None,
        views: Vec::new(),
        sums: BTreeMap::new(),
    };
    if let Err(reason) = check_bundle(&mut bundle) {
        bundle.status = BundleStatus::Rejected(reason.to_string());
    }
    bundle
}

/// Apply every rule to `bundle`, filling in what its manifest says on the way.
fn check_bundle(bundle: &mut LoadedBundle) -> Result<(), &'static str> {
    bundle.sums = read_sums(&bundle.dir).ok_or("integrity")?;
    check_files(&bundle.dir, &bundle.sums)?;
    let manifest = read_json(&bundle.dir.join("bundle.json")).ok_or("manifest")?;
    describe(bundle, &manifest);
    if manifest.get("schema").and_then(Value::as_str) != Some(BUNDLE_SCHEMA) {
        return Err("manifest");
    }
    if manifest.get("kind").and_then(Value::as_str) != Some("replay") {
        return Err("kind");
    }
    if manifest.get("simulated") != Some(&Value::Bool(false)) {
        return Err("simulated");
    }
    check_documents(&bundle.dir, &bundle.sums)?;
    check_metrics(&bundle.dir)?;
    let views_present = bundle.views.iter().all(|view| {
        VIEW_IDS.contains(&view.as_str()) && bundle.sums.contains_key(&format!("views/{view}.json"))
    });
    if views_present { Ok(()) } else { Err("views") }
}

/// What the manifest says about the bundle, for the manifest route.
fn describe(bundle: &mut LoadedBundle, manifest: &Value) {
    let text = |key: &str| {
        manifest
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    bundle.title = text("title").unwrap_or_else(|| bundle.id.clone());
    bundle.created_at = text("created_at").unwrap_or_default();
    bundle.featured = manifest.get("featured") == Some(&Value::Bool(true));
    bundle.harness_commit = text("harness_commit");
    bundle.views = manifest
        .get("views")
        .and_then(Value::as_array)
        .map(|views| {
            views
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
}

/// `SHA256SUMS` as path → digest, or `None` when a line is malformed or names a path that leaves
/// the bundle.
fn read_sums(dir: &Path) -> Option<BTreeMap<String, String>> {
    let text = std::fs::read_to_string(dir.join(SUMS_FILE)).ok()?;
    let mut sums = BTreeMap::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let (digest, path) = line.split_at_checked(64)?;
        let path = path.strip_prefix("  ").or_else(|| path.strip_prefix(" *"))?;
        let hex = digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
        if !hex || !is_bundle_path(path) {
            return None;
        }
        sums.insert(path.to_string(), digest.to_string());
    }
    Some(sums)
}

/// Whether `path` is a plain relative path inside a bundle.
fn is_bundle_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

/// Every listed file matches its digest, and every file is listed; no symbolic links.
fn check_files(dir: &Path, sums: &BTreeMap<String, String>) -> Result<(), &'static str> {
    for (path, digest) in sums {
        let bytes = std::fs::read(dir.join(path)).map_err(|_| "integrity")?;
        if sha256_hex(&bytes) != *digest {
            return Err("integrity");
        }
    }
    let mut present = BTreeSet::new();
    collect_files(dir, dir, &mut present)?;
    present.remove(SUMS_FILE);
    if present.iter().all(|path| sums.contains_key(path)) {
        Ok(())
    } else {
        Err("integrity")
    }
}

/// The paths of every file under `dir`, relative to `root`.
fn collect_files(
    root: &Path,
    dir: &Path,
    files: &mut BTreeSet<String>,
) -> Result<(), &'static str> {
    for entry in std::fs::read_dir(dir).map_err(|_| "integrity")? {
        let entry = entry.map_err(|_| "integrity")?;
        let kind = entry.file_type().map_err(|_| "integrity")?;
        let path = entry.path();
        if kind.is_symlink() {
            return Err("integrity");
        }
        if kind.is_dir() {
            collect_files(root, &path, files)?;
        } else {
            let relative = path.strip_prefix(root).map_err(|_| "integrity")?;
            let relative = relative.to_str().ok_or("integrity")?.replace('\\', "/");
            files.insert(relative);
        }
    }
    Ok(())
}

/// Every `.json` file and `.jsonl` line parses, and none says `simulated` other than `false`.
fn check_documents(dir: &Path, sums: &BTreeMap<String, String>) -> Result<(), &'static str> {
    for path in sums.keys() {
        let extension = Path::new(path).extension().and_then(|ext| ext.to_str());
        let documents = match extension {
            Some("json") => vec![read_json(&dir.join(path)).ok_or("manifest")?],
            Some("jsonl") => read_jsonl(&dir.join(path)).ok_or("manifest")?,
            _ => continue,
        };
        if documents.iter().any(says_simulated) {
            return Err("simulated");
        }
    }
    Ok(())
}

/// Whether `value` holds a `simulated` key whose value is anything but `false`.
fn says_simulated(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, item)| {
            (key == "simulated" && *item != Value::Bool(false)) || says_simulated(item)
        }),
        Value::Array(items) => items.iter().any(says_simulated),
        _ => false,
    }
}

/// Every metric lists the runs it counts, and none counts a run excluded from results.
fn check_metrics(dir: &Path) -> Result<(), &'static str> {
    let metrics = read_jsonl(&dir.join("data/metrics.jsonl")).ok_or("manifest")?;
    let records = read_jsonl(&dir.join("data/records.jsonl")).unwrap_or_default();
    let excluded: BTreeSet<&str> = records
        .iter()
        .filter(|record| {
            let status = record.pointer("/execution/status").and_then(Value::as_str);
            status.is_some_and(|status| EXCLUDED_RUN_STATUSES.contains(&status))
        })
        .filter_map(|record| record.get("run_id").and_then(Value::as_str))
        .collect();
    for metric in &metrics {
        let run_ids = metric.get("run_ids").and_then(Value::as_array);
        let Some(run_ids) = run_ids.filter(|run_ids| !run_ids.is_empty()) else {
            return Err("no_run_ids");
        };
        let counts_excluded = run_ids
            .iter()
            .filter_map(Value::as_str)
            .any(|run_id| excluded.contains(run_id));
        if counts_excluded {
            return Err("infra_error_counted");
        }
    }
    Ok(())
}

fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// The rows of a JSON Lines file, or `None` when it is missing or a line does not parse.
fn read_jsonl(path: &Path) -> Option<Vec<Value>> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).ok())
        .collect()
}

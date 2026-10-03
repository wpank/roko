//! Dated price snapshots, and API-equivalent costs priced from them (S04 §4.8, S08 §5.6).
//!
//! A snapshot is a TOML file, `config/prices/<date>.toml`, with the schema
//! `roko.price_snapshot/1` and the id `prices-<date>`: one row of rates per model, in USD per
//! 1M tokens. Snapshots are immutable: a new rate means a new dated file with a new id. The
//! ViabilityBench ledger (`benchmarks/viabilitybench/driver/ledger.py`) reads the same files and
//! prices usage the same way.
//!
//! Which snapshot a run prices from is decision 2113 ([`PriceSnapshot::for_workspace`]): the one
//! `[pricing] snapshot` names in roko.toml, else the newest file in the workspace's
//! `config/prices/`, else the copy built into the binary ([`BUILTIN_SNAPSHOT_ID`]) for a
//! workspace that has none.
//!
//! [`PriceSnapshot::price`] returns `None` for a model the snapshot has no row for: its cost is
//! unknown, never another model's rate. roko-core cannot name roko-learn's `CostSource`, so
//! callers map a priced usage to it.

use std::collections::{HashMap, HashSet};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::config::model_registry::is_snapshot_of;

/// The schema every snapshot file declares.
pub const PRICE_SNAPSHOT_SCHEMA: &str = "roko.price_snapshot/1";

/// Where a workspace keeps its snapshots: the id `prices-<date>` names
/// `config/prices/<date>.toml`.
pub const PRICES_DIR: &str = "config/prices";

/// The id of the snapshot built into the binary: the newest file in this repository's
/// `config/prices/` when the binary was built.
pub const BUILTIN_SNAPSHOT_ID: &str = "prices-2026-09-28";

/// The built-in snapshot's bytes, exactly as the file has them.
const BUILTIN_SNAPSHOT_TOML: &str = include_str!("../../../config/prices/2026-09-28.toml");

/// The prefix of every snapshot id.
const ID_PREFIX: &str = "prices-";

/// Snapshot rates are per this many tokens.
const PER_TOKENS: f64 = 1_000_000.0;

// ---- [pricing] -----------------------------------------------------------

/// `[pricing]` in roko.toml: the dated price snapshot behind API-equivalent costs.
///
/// ```toml
/// [pricing]
/// snapshot = "prices-2026-09-28"   # config/prices/2026-09-28.toml
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PricingConfig {
    /// Snapshot id, `prices-<date>`.
    ///
    /// Empty (the default) means unset: a run prices from the newest snapshot in
    /// `config/prices/`, else the built-in copy. It is a string rather than an `Option` so the
    /// key always serializes; the config loader drops any key that the serialized default
    /// config lacks.
    #[serde(default)]
    pub snapshot: String,
}

impl PricingConfig {
    /// The configured snapshot id, or `None` when unset.
    #[must_use]
    pub fn snapshot_id(&self) -> Option<&str> {
        let id = self.snapshot.trim();
        (!id.is_empty()).then_some(id)
    }
}

// ---- snapshots -------------------------------------------------------------

/// Token usage in the five priced classes, which are disjoint (S01 §4.4): no token is in two
/// of them. `input` is uncached input only; OpenAI-style counts include cached tokens, so
/// subtract them first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TokenCounts {
    /// Uncached input.
    pub input: u64,
    /// Input read from the prompt cache.
    pub cache_read: u64,
    /// Input written to the prompt cache with a 5-minute TTL.
    pub cache_write_5m: u64,
    /// Input written to the prompt cache with a 1-hour TTL.
    pub cache_write_1h: u64,
    /// Output.
    pub output: u64,
    /// Reasoning tokens. Not a sixth class unless the row's `reasoning_in_output` is false:
    /// otherwise they are already inside `output` and are not priced again.
    pub reasoning: u64,
}

/// What a usage costs at one snapshot row's rates, in USD.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PricedUsage {
    /// Each class at its own rate: `AttemptCost.api_equiv_usd`.
    pub api_equiv_usd: f64,
    /// The same tokens with cache reads priced as uncached input:
    /// `AttemptCost.without_cache_usd`.
    pub without_cache_usd: f64,
}

/// One model's rates in a snapshot, in USD per 1M tokens. Every column is required; only
/// `note` may be left out.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PriceRow {
    /// The model name the provider reports.
    pub slug: String,
    /// The provider that bills the model.
    pub provider: String,
    /// Uncached input.
    pub input: f64,
    /// Input read from the prompt cache; equals `input` when the provider publishes no read
    /// discount.
    pub cache_read: f64,
    /// Input written to the prompt cache with a 5-minute TTL; equals `input` when the provider
    /// publishes no write premium.
    pub cache_write_5m: f64,
    /// Input written to the prompt cache with a 1-hour TTL; equals `input` when the provider
    /// publishes no write premium.
    pub cache_write_1h: f64,
    /// Output.
    pub output: f64,
    /// `true`: reasoning tokens are already inside the output tokens. `false`: they are billed
    /// on top of them, at the output rate.
    pub reasoning_in_output: bool,
    /// The price page the rates come from.
    pub source_url: String,
    /// How and when the rates were checked.
    pub verified: String,
    /// Anything else about the row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl PriceRow {
    /// What `tokens` cost at this row's rates.
    #[must_use]
    pub fn price(&self, tokens: &TokenCounts) -> PricedUsage {
        let reasoning = if self.reasoning_in_output {
            0
        } else {
            tokens.reasoning
        };
        let paid = (tokens.output as f64 + reasoning as f64) * self.output
            + tokens.cache_write_5m as f64 * self.cache_write_5m
            + tokens.cache_write_1h as f64 * self.cache_write_1h;
        let input = tokens.input as f64;
        let cache_read = tokens.cache_read as f64;
        PricedUsage {
            api_equiv_usd: (input * self.input + cache_read * self.cache_read + paid) / PER_TOKENS,
            without_cache_usd: ((input + cache_read) * self.input + paid) / PER_TOKENS,
        }
    }

    /// The five rate columns, by name.
    fn rates(&self) -> [(&'static str, f64); 5] {
        [
            ("input", self.input),
            ("cache_read", self.cache_read),
            ("cache_write_5m", self.cache_write_5m),
            ("cache_write_1h", self.cache_write_1h),
            ("output", self.output),
        ]
    }

    /// The rule this row breaks, or `None`. `slugs` collects the slugs of the rows before it.
    fn problem(&self, slugs: &mut HashSet<String>) -> Option<String> {
        if self.slug.trim().is_empty() {
            return Some("the slug is empty".to_string());
        }
        if !slugs.insert(self.slug.to_ascii_lowercase()) {
            return Some("duplicate slug".to_string());
        }
        if self.source_url.trim().is_empty() {
            return Some("source_url is empty".to_string());
        }
        let (column, rate) = self
            .rates()
            .into_iter()
            .find(|(_, rate)| !(rate.is_finite() && *rate > 0.0))?;
        Some(format!(
            "{column} = {rate}: a rate must be a finite number above 0"
        ))
    }
}

/// A validated price snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct PriceSnapshot {
    id: String,
    fetched_at: String,
    rows: Vec<PriceRow>,
}

impl PriceSnapshot {
    /// Read and validate a snapshot file.
    pub fn load(path: &Path) -> Result<Self, PriceSnapshotError> {
        let text = std::fs::read_to_string(path).map_err(|source| PriceSnapshotError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_toml(&text, &path.display().to_string())
    }

    /// Parse and validate snapshot text; `origin` names it in errors.
    pub fn from_toml(text: &str, origin: &str) -> Result<Self, PriceSnapshotError> {
        let file: SnapshotFile =
            toml::from_str(text).map_err(|source| PriceSnapshotError::Parse {
                origin: origin.to_string(),
                source,
            })?;
        file.validate(origin)
    }

    /// The copy of [`BUILTIN_SNAPSHOT_ID`] built into the binary.
    pub fn builtin() -> Result<Self, PriceSnapshotError> {
        let origin = format!("built-in {BUILTIN_SNAPSHOT_ID}");
        Self::from_toml(BUILTIN_SNAPSHOT_TOML, &origin)
    }

    /// Read the snapshot `id` names: `prices-<date>` is
    /// `<workspace_root>/config/prices/<date>.toml`, and the file must carry that id. The
    /// built-in id resolves to the built-in copy when the workspace lacks its file; any other
    /// id without a file is an error.
    pub fn resolve(id: &str, workspace_root: &Path) -> Result<Self, PriceSnapshotError> {
        let date = snapshot_date(id).ok_or_else(|| PriceSnapshotError::BadId(id.to_string()))?;
        let path = snapshot_path(workspace_root, date);
        let snapshot = match std::fs::read_to_string(&path) {
            Ok(text) => Self::from_toml(&text, &path.display().to_string())?,
            Err(err) if err.kind() == ErrorKind::NotFound && id == BUILTIN_SNAPSHOT_ID => {
                return Self::builtin();
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {
                return Err(PriceSnapshotError::NotFound {
                    id: id.to_string(),
                    path,
                });
            }
            Err(source) => return Err(PriceSnapshotError::Read { path, source }),
        };
        if snapshot.id != id {
            return Err(PriceSnapshotError::Invalid {
                origin: path.display().to_string(),
                reason: format!("its id is {:?}, not {id:?}", snapshot.id),
            });
        }
        Ok(snapshot)
    }

    /// The newest snapshot in `<workspace_root>/config/prices/`, or `None` when the workspace
    /// has none. Only a file named `<YYYY-MM-DD>.toml` is a snapshot.
    pub fn newest(workspace_root: &Path) -> Result<Option<Self>, PriceSnapshotError> {
        let dir = workspace_root.join(PRICES_DIR);
        match newest_date(&dir)? {
            Some(date) => Self::resolve(&format!("{ID_PREFIX}{date}"), workspace_root).map(Some),
            None => Ok(None),
        }
    }

    /// The snapshot a run in `workspace_root` prices from (decision 2113): the one
    /// `[pricing] snapshot` names, else the newest file in `config/prices/`, else the built-in
    /// copy. A named snapshot that cannot be read is an error, never another snapshot.
    pub fn for_workspace(
        pricing: &PricingConfig,
        workspace_root: &Path,
    ) -> Result<Self, PriceSnapshotError> {
        if let Some(id) = pricing.snapshot_id() {
            return Self::resolve(id, workspace_root);
        }
        match Self::newest(workspace_root)? {
            Some(snapshot) => Ok(snapshot),
            None => Self::builtin(),
        }
    }

    /// [`Self::for_workspace`], loaded once per process for each workspace and configured id,
    /// and shared by every caller (backlog 2114, 6105). `None`, with a warning, when the
    /// snapshot cannot be read.
    pub fn shared(pricing: &PricingConfig, workspace_root: &Path) -> Option<Arc<Self>> {
        type Loaded = HashMap<(PathBuf, String), Option<Arc<PriceSnapshot>>>;
        static LOADED: LazyLock<Mutex<Loaded>> = LazyLock::new(Mutex::default);
        let key = (
            workspace_root.to_path_buf(),
            pricing.snapshot_id().unwrap_or_default().to_string(),
        );
        LOADED
            .lock()
            .entry(key)
            .or_insert_with(
                || match Self::for_workspace(pricing, workspace_root) {
                    Ok(snapshot) => Some(Arc::new(snapshot)),
                    Err(error) => {
                        tracing::warn!(
                            workspace = %workspace_root.display(),
                            %error,
                            "no price snapshot: the costs it would price stay unknown"
                        );
                        None
                    }
                },
            )
            .clone()
    }

    /// The snapshot id, `prices-<date>`: what `AttemptCost.price_snapshot_id` records.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The date the rates were read, `YYYY-MM-DD`.
    #[must_use]
    pub fn fetched_at(&self) -> &str {
        &self.fetched_at
    }

    /// Every row, in file order.
    #[must_use]
    pub fn rows(&self) -> &[PriceRow] {
        &self.rows
    }

    /// The row for `slug`, the model name the provider reports: its own row (ASCII case
    /// ignored), else the longest slug that `slug` is a dated or versioned snapshot of
    /// ([`is_snapshot_of`]), as [`builtin_pricing`] matches. `None` for any other model.
    ///
    /// [`builtin_pricing`]: crate::config::model_registry::builtin_pricing
    #[must_use]
    pub fn row(&self, slug: &str) -> Option<&PriceRow> {
        let slug = slug.to_ascii_lowercase();
        if slug.is_empty() {
            return None;
        }
        self.rows
            .iter()
            .find(|row| row.slug.eq_ignore_ascii_case(&slug))
            .or_else(|| {
                self.rows
                    .iter()
                    .filter(|row| is_snapshot_of(&slug, &row.slug.to_ascii_lowercase()))
                    .max_by_key(|row| row.slug.len())
            })
    }

    /// What `tokens` of model `slug` cost at this snapshot's rates. `None` when the snapshot
    /// has no row for `slug`: the cost is unknown, never another model's rate.
    #[must_use]
    pub fn price(&self, slug: &str, tokens: &TokenCounts) -> Option<PricedUsage> {
        self.row(slug).map(|row| row.price(tokens))
    }
}

/// Why a price snapshot could not be read.
#[derive(Debug, thiserror::Error)]
pub enum PriceSnapshotError {
    /// The id is not `prices-YYYY-MM-DD`.
    #[error("not a price snapshot id: {0:?} (expected prices-YYYY-MM-DD)")]
    BadId(String),
    /// The workspace has no file for the id, and the binary has no copy of it.
    #[error("price snapshot {id} not found: {path} does not exist")]
    NotFound {
        /// The snapshot id.
        id: String,
        /// The file the id names.
        path: PathBuf,
    },
    /// A snapshot file or the prices directory could not be read.
    #[error("read {path}: {source}")]
    Read {
        /// The file or directory.
        path: PathBuf,
        /// The I/O error.
        source: std::io::Error,
    },
    /// The text is not TOML of a snapshot's shape: a column is missing or unknown, or a value
    /// has the wrong type.
    #[error("parse {origin}: {source}")]
    Parse {
        /// The file, or the built-in copy.
        origin: String,
        /// The TOML error.
        source: toml::de::Error,
    },
    /// The snapshot parsed but breaks a rule of `roko.price_snapshot/1`.
    #[error("{origin}: {reason}")]
    Invalid {
        /// The file, or the built-in copy.
        origin: String,
        /// The rule it breaks.
        reason: String,
    },
}

/// A snapshot file as written (S08 §5.6). Parsing rejects a missing or unknown column.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotFile {
    schema_version: String,
    id: String,
    fetched_at: String,
    currency: String,
    unit: String,
    model: Vec<PriceRow>,
}

impl SnapshotFile {
    /// Check the rules of `roko.price_snapshot/1` that parsing does not.
    fn validate(self, origin: &str) -> Result<PriceSnapshot, PriceSnapshotError> {
        let invalid = |reason: String| PriceSnapshotError::Invalid {
            origin: origin.to_string(),
            reason,
        };
        if self.schema_version != PRICE_SNAPSHOT_SCHEMA {
            return Err(invalid(format!(
                "schema_version is {:?}, not {PRICE_SNAPSHOT_SCHEMA:?}",
                self.schema_version
            )));
        }
        if snapshot_date(&self.id).is_none() {
            return Err(invalid(format!(
                "id {:?} is not prices-YYYY-MM-DD",
                self.id
            )));
        }
        if self.currency != "USD" || self.unit != "per_1M_tokens" {
            return Err(invalid(format!(
                "rates must be in USD per_1M_tokens, not {} {}",
                self.currency, self.unit
            )));
        }
        if self.model.is_empty() {
            return Err(invalid("it has no model rows".to_string()));
        }
        let mut slugs = HashSet::new();
        for (index, row) in self.model.iter().enumerate() {
            if let Some(reason) = row.problem(&mut slugs) {
                return Err(invalid(format!("model[{index}] {:?}: {reason}", row.slug)));
            }
        }
        Ok(PriceSnapshot {
            id: self.id,
            fetched_at: self.fetched_at,
            rows: self.model,
        })
    }
}

/// The date in a snapshot id, `prices-<YYYY-MM-DD>`, or `None` for any other text.
fn snapshot_date(id: &str) -> Option<&str> {
    let date = id.strip_prefix(ID_PREFIX)?;
    is_iso_date(date).then_some(date)
}

/// Whether `text` has the form `YYYY-MM-DD`.
fn is_iso_date(text: &str) -> bool {
    text.len() == 10 && text.bytes().enumerate().all(is_date_byte)
}

/// Whether `byte` may stand at `index` of a `YYYY-MM-DD` date.
fn is_date_byte((index, byte): (usize, u8)) -> bool {
    if index == 4 || index == 7 {
        byte == b'-'
    } else {
        byte.is_ascii_digit()
    }
}

/// The file that holds the snapshot of `date`.
fn snapshot_path(workspace_root: &Path, date: &str) -> PathBuf {
    let file = format!("{date}.toml");
    workspace_root.join(PRICES_DIR).join(file)
}

/// The newest date with a `<date>.toml` in `dir`, or `None` when it has none or is missing.
fn newest_date(dir: &Path) -> Result<Option<String>, PriceSnapshotError> {
    let read_error = |source| PriceSnapshotError::Read {
        path: dir.to_path_buf(),
        source,
    };
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(read_error(err)),
    };
    let mut newest: Option<String> = None;
    for entry in entries {
        let name = entry.map_err(read_error)?.file_name();
        let Some(date) = name.to_str().and_then(|name| name.strip_suffix(".toml")) else {
            continue;
        };
        if is_iso_date(date) && newest.as_deref().is_none_or(|current| date > current) {
            newest = Some(date.to_string());
        }
    }
    Ok(newest)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::config::loader::{LoadOptions, load_config_file};
    use crate::config::schema::RokoConfig;

    const URL: &str = "https://example.com/pricing";

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn real_snapshot() -> PriceSnapshot {
        let path = workspace_root().join("config/prices/2026-09-28.toml");
        PriceSnapshot::load(&path).expect("load config/prices/2026-09-28.toml")
    }

    fn providers(snapshot: &PriceSnapshot) -> BTreeSet<&str> {
        snapshot
            .rows()
            .iter()
            .map(|row| row.provider.as_str())
            .collect()
    }

    /// One `[[model]]` row, with rates input 1, cache read 0.5, 5-minute cache write 1.25,
    /// 1-hour cache write 2 and output 4.
    fn row(slug: &str, reasoning_in_output: bool, source_url: &str) -> String {
        format!(
            "[[model]]\nslug = \"{slug}\"\nprovider = \"test\"\ninput = 1.0\n\
             cache_read = 0.5\ncache_write_5m = 1.25\ncache_write_1h = 2.0\noutput = 4.0\n\
             reasoning_in_output = {reasoning_in_output}\nsource_url = \"{source_url}\"\n\
             verified = \"test\"\n"
        )
    }

    /// A snapshot file with `id` and `rows`.
    fn snapshot_text(id: &str, rows: &str) -> String {
        format!(
            "schema_version = \"roko.price_snapshot/1\"\nid = \"{id}\"\n\
             fetched_at = \"2030-01-01\"\ncurrency = \"USD\"\nunit = \"per_1M_tokens\"\n{rows}"
        )
    }

    /// Write `config/prices/<date>.toml` under `root`.
    fn write_snapshot(root: &Path, date: &str, text: &str) {
        let dir = root.join(PRICES_DIR);
        std::fs::create_dir_all(&dir).expect("create config/prices");
        std::fs::write(dir.join(format!("{date}.toml")), text).expect("write a snapshot");
    }

    #[test]
    fn pricing_snapshot_real_file_parses() {
        let snapshot = real_snapshot();

        assert_eq!(snapshot.id(), "prices-2026-09-28");
        assert_eq!(snapshot.fetched_at(), "2026-09-28");
        assert_eq!(snapshot.rows().len(), 10);
        assert_eq!(
            providers(&snapshot),
            BTreeSet::from(["anthropic", "cerebras", "moonshot", "openai", "zai"])
        );
        for row in snapshot.rows() {
            assert!(row.source_url.starts_with("https://"), "{}", row.slug);
        }
    }

    /// One row per provider, priced by hand: tokens times USD per 1M tokens.
    #[test]
    fn pricing_snapshot_reprices_one_row_per_provider() {
        let snapshot = real_snapshot();
        let cases = [
            // cerebras, S01 §5.5's attempt: 38,211 × 0.35 + 2,904 × 0.75 = 15,551.85.
            (
                "gpt-oss-120b",
                TokenCounts {
                    input: 38_211,
                    output: 2_904,
                    ..TokenCounts::default()
                },
                0.01555185,
                0.01555185,
            ),
            // zai: 100,000 × 0.60 + 50,000 × 0.11 + 10,000 × 2.20 = 87,500; with cache reads
            // priced as input, 150,000 × 0.60 + 22,000 = 112,000.
            (
                "glm-4.7",
                TokenCounts {
                    input: 100_000,
                    cache_read: 50_000,
                    output: 10_000,
                    ..TokenCounts::default()
                },
                0.0875,
                0.112,
            ),
            // moonshot, reasoning inside output: 20,000 × 0.95 + 80,000 × 0.16 + 5,000 × 4.00
            // = 51,800; 100,000 × 0.95 + 20,000 = 115,000.
            (
                "kimi-k2.6",
                TokenCounts {
                    input: 20_000,
                    cache_read: 80_000,
                    output: 5_000,
                    reasoning: 1_200,
                    ..TokenCounts::default()
                },
                0.0518,
                0.115,
            ),
            // openai, reasoning inside output: 12,345 × 0.75 + 6,789 × 0.075 + 2,000 × 4.50
            // = 18,767.925; 19,134 × 0.75 + 9,000 = 23,350.5.
            (
                "gpt-5.4-mini",
                TokenCounts {
                    input: 12_345,
                    cache_read: 6_789,
                    output: 2_000,
                    reasoning: 1_500,
                    ..TokenCounts::default()
                },
                0.018767925,
                0.0233505,
            ),
            // anthropic, both cache-write TTLs: 3,000 × 4 + 200,000 × 0.20 + 10,000 × 5
            // + 4,000 × 8 + 8,000 × 20 = 294,000; 203,000 × 4 + 242,000 = 1,054,000.
            (
                "claude-opus-5-5",
                TokenCounts {
                    input: 3_000,
                    cache_read: 200_000,
                    cache_write_5m: 10_000,
                    cache_write_1h: 4_000,
                    output: 8_000,
                    ..TokenCounts::default()
                },
                0.294,
                1.054,
            ),
        ];

        let mut priced_providers = BTreeSet::new();
        for (slug, tokens, api_equiv_usd, without_cache_usd) in cases {
            let Some(priced) = snapshot.price(slug, &tokens) else {
                panic!("{slug} has no price");
            };
            assert!(
                (priced.api_equiv_usd - api_equiv_usd).abs() < 1e-9,
                "{slug}: api_equiv_usd {} != {api_equiv_usd}",
                priced.api_equiv_usd
            );
            assert!(
                (priced.without_cache_usd - without_cache_usd).abs() < 1e-9,
                "{slug}: without_cache_usd {} != {without_cache_usd}",
                priced.without_cache_usd
            );
            priced_providers.extend(snapshot.row(slug).map(|row| row.provider.as_str()));
        }
        assert_eq!(priced_providers, providers(&snapshot));
    }

    #[test]
    fn pricing_snapshot_unknown_slug_has_no_price() {
        let snapshot = real_snapshot();
        let tokens = TokenCounts {
            input: 1_000,
            output: 1_000,
            ..TokenCounts::default()
        };

        // claude-sonnet-4-6, the ladder's top rung, has no row (decision 2113): its cost is
        // unknown until a new dated snapshot lists it, never another model's rate.
        for slug in ["claude-sonnet-4-6", "gpt-5.4-nano", "gpt-oss-20b", ""] {
            assert_eq!(snapshot.price(slug, &tokens), None, "{slug}");
        }
        // A dated name of a listed model takes that model's row, in any case.
        for (slug, expected) in [
            ("gpt-5.4-2026-03-05", "gpt-5.4"),
            ("claude-haiku-4-5-20251001", "claude-haiku-4-5"),
            ("GPT-5.4-Mini", "gpt-5.4-mini"),
        ] {
            let found = snapshot.row(slug).map(|row| row.slug.as_str());
            assert_eq!(found, Some(expected), "{slug}");
        }
    }

    #[test]
    fn pricing_snapshot_adds_reasoning_only_outside_output() {
        let rows = row("outside", false, URL) + &row("inside", true, URL);
        let text = snapshot_text("prices-2030-01-01", &rows);
        let snapshot = PriceSnapshot::from_toml(&text, "test").expect("parse two rows");
        let tokens = TokenCounts {
            input: 1_000,
            output: 500,
            reasoning: 250,
            ..TokenCounts::default()
        };

        // 1,000 × 1 + (500 + 250) × 4 = 4,000, against 1,000 × 1 + 500 × 4 = 3,000.
        let outside = snapshot.price("outside", &tokens).expect("priced");
        let inside = snapshot.price("inside", &tokens).expect("priced");
        assert!((outside.api_equiv_usd - 0.004).abs() < 1e-12);
        assert!((inside.api_equiv_usd - 0.003).abs() < 1e-12);
    }

    #[test]
    fn pricing_snapshot_rejects_a_duplicate_slug() {
        for second in ["m-1", "M-1"] {
            let rows = row("m-1", true, URL) + &row(second, true, URL);
            let text = snapshot_text("prices-2030-01-01", &rows);

            let err = PriceSnapshot::from_toml(&text, "test").expect_err("a slug appears once");

            assert!(matches!(err, PriceSnapshotError::Invalid { .. }), "{err}");
            assert!(err.to_string().contains("duplicate slug"), "{err}");
        }
    }

    #[test]
    fn pricing_snapshot_rejects_a_missing_source_url() {
        // A row without the column fails to parse; a blank one fails validation.
        let absent = row("m-1", true, URL).replace(&format!("source_url = \"{URL}\"\n"), "");
        let blank = row("m-1", true, " ");
        for rows in [absent, blank] {
            let text = snapshot_text("prices-2030-01-01", &rows);

            let err = PriceSnapshot::from_toml(&text, "test").expect_err("a row names its page");

            assert!(err.to_string().contains("source_url"), "{err}");
        }
    }

    #[test]
    fn pricing_snapshot_rejects_what_the_schema_forbids() {
        let good = snapshot_text("prices-2030-01-01", &row("m-1", true, URL));
        assert!(PriceSnapshot::from_toml(&good, "test").is_ok());
        // (what the error names, text in the good snapshot, its replacement)
        let cases = [
            ("schema_version", "_snapshot/1", "_snapshot/2"),
            ("cache_write_1h", "cache_write_1h = 2.0\n", ""),
            ("unknown field", "input = 1.0", "inputs = 1.0"),
            ("output = 0", "output = 4.0", "output = 0.0"),
            ("per_1K_tokens", "per_1M_tokens", "per_1K_tokens"),
            ("prices-YYYY-MM-DD", "prices-2030-01-01", "2030-01-01"),
        ];

        for (needle, from, to) in cases {
            let text = good.replace(from, to);
            let err = PriceSnapshot::from_toml(&text, "test").expect_err(needle);
            assert!(err.to_string().contains(needle), "{needle}: {err}");
        }
        let empty = snapshot_text("prices-2030-01-01", "model = []\n");
        let err = PriceSnapshot::from_toml(&empty, "test").expect_err("no rows");
        assert!(err.to_string().contains("no model rows"), "{err}");
    }

    #[test]
    fn pricing_snapshot_id_resolves_to_the_file() {
        let resolved =
            PriceSnapshot::resolve("prices-2026-09-28", &workspace_root()).expect("resolve");
        assert_eq!(resolved, real_snapshot());

        let dir = tempfile::tempdir().expect("tempdir");
        let text = snapshot_text("prices-2030-01-01", &row("m-1", true, URL));
        write_snapshot(dir.path(), "2030-01-01", &text);
        let resolved = PriceSnapshot::resolve("prices-2030-01-01", dir.path()).expect("resolve");
        assert_eq!(resolved.id(), "prices-2030-01-01");
        assert_eq!(resolved.rows()[0].slug, "m-1");

        // A file carries the id its name gives it: a copy that kept the old id is rejected.
        write_snapshot(dir.path(), "2030-02-02", &text);
        let err = PriceSnapshot::resolve("prices-2030-02-02", dir.path()).expect_err("old id");
        assert!(matches!(err, PriceSnapshotError::Invalid { .. }), "{err}");

        for id in ["2030-01-01", "prices-2030-1-1", "prices-latest"] {
            let err = PriceSnapshot::resolve(id, dir.path()).expect_err(id);
            assert!(matches!(err, PriceSnapshotError::BadId(_)), "{err}");
        }
        // An id without a file is an error, except the built-in one.
        let err = PriceSnapshot::resolve("prices-2030-03-03", dir.path()).expect_err("no file");
        assert!(matches!(err, PriceSnapshotError::NotFound { .. }), "{err}");
        let builtin = PriceSnapshot::resolve(BUILTIN_SNAPSHOT_ID, dir.path()).expect("built-in");
        assert_eq!(builtin.id(), BUILTIN_SNAPSHOT_ID);
    }

    /// Decision 2113: the snapshot roko.toml names, else the newest in `config/prices/`, else
    /// the built-in copy.
    #[test]
    fn pricing_snapshot_run_uses_the_named_then_newest_then_builtin() {
        let dir = tempfile::tempdir().expect("tempdir");
        let unset = PricingConfig::default();
        let builtin = PriceSnapshot::for_workspace(&unset, dir.path()).expect("built-in");
        assert_eq!(builtin.id(), BUILTIN_SNAPSHOT_ID);

        for date in ["2030-01-01", "2030-06-01"] {
            let text = snapshot_text(&format!("prices-{date}"), &row("m-1", true, URL));
            write_snapshot(dir.path(), date, &text);
        }
        // Neither a probe record nor an undated TOML file is a snapshot.
        let prices = dir.path().join(PRICES_DIR);
        std::fs::write(prices.join("2031-01-01.probes.md"), "# probes\n").expect("write");
        std::fs::write(prices.join("notes.toml"), "note = \"no snapshot\"\n").expect("write");
        let newest = PriceSnapshot::for_workspace(&unset, dir.path()).expect("newest");
        assert_eq!(newest.id(), "prices-2030-06-01");

        let named = PricingConfig {
            snapshot: "prices-2030-01-01".to_string(),
        };
        let snapshot = PriceSnapshot::for_workspace(&named, dir.path()).expect("named");
        assert_eq!(snapshot.id(), "prices-2030-01-01");
        // A named snapshot the workspace lacks is an error, never another snapshot.
        let missing = PricingConfig {
            snapshot: "prices-2029-01-01".to_string(),
        };
        let err = PriceSnapshot::for_workspace(&missing, dir.path()).expect_err("missing");
        assert!(matches!(err, PriceSnapshotError::NotFound { .. }), "{err}");
    }

    /// The binary carries a copy of the newest snapshot (decision 2113), so a newer file in
    /// `config/prices/` needs a new copy.
    #[test]
    fn pricing_snapshot_builtin_copy_is_the_newest_file() {
        let root = workspace_root();
        let newest = PriceSnapshot::newest(&root)
            .expect("read config/prices")
            .expect("config/prices has a snapshot");
        assert_eq!(
            newest.id(),
            BUILTIN_SNAPSHOT_ID,
            "point BUILTIN_SNAPSHOT_ID and BUILTIN_SNAPSHOT_TOML at the newest snapshot"
        );

        let date = snapshot_date(BUILTIN_SNAPSHOT_ID).expect("the built-in id has a date");
        let file = std::fs::read_to_string(snapshot_path(&root, date)).expect("read the file");
        assert_eq!(BUILTIN_SNAPSHOT_TOML, file);
        let builtin = PriceSnapshot::builtin().expect("parse the built-in copy");
        assert_eq!(builtin, newest);
    }

    #[test]
    fn pricing_snapshot_config_key_is_optional() {
        let unset = RokoConfig::from_toml("").expect("parse an empty config");
        assert_eq!(unset.pricing, PricingConfig::default());
        assert_eq!(unset.pricing.snapshot_id(), None);

        let blank = RokoConfig::from_toml("[pricing]\nsnapshot = \" \"\n").expect("parse");
        assert_eq!(blank.pricing.snapshot_id(), None);

        let named = RokoConfig::from_toml("[pricing]\nsnapshot = \" prices-2026-09-28 \"\n")
            .expect("parse [pricing]");
        assert_eq!(named.pricing.snapshot_id(), Some("prices-2026-09-28"));

        assert!(RokoConfig::from_toml("[pricing]\nsnapshots = \"prices-2026-09-28\"\n").is_err());
    }

    /// The loader strips every key that the serialized default config lacks, so the key has
    /// to survive a load from disk.
    #[test]
    fn pricing_snapshot_config_key_survives_the_config_loader() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("roko.toml");
        std::fs::write(&path, "[pricing]\nsnapshot = \"prices-2026-09-28\"\n")
            .expect("write roko.toml");
        let opts = LoadOptions {
            merge_global: false,
            apply_env_overrides: false,
            apply_hierarchical_env: false,
            strict_validation: false,
        };

        let config = load_config_file(&path, &opts).expect("load roko.toml");

        assert_eq!(config.pricing.snapshot_id(), Some("prices-2026-09-28"));
        // The workspace has no config/prices/, so the built-in id resolves to the copy.
        let snapshot =
            PriceSnapshot::for_workspace(&config.pricing, dir.path()).expect("resolve the id");
        assert_eq!(snapshot.id(), "prices-2026-09-28");
    }
}

//! Disturbances for M1's experiments and demo (S06 §4.9): the
//! [`DisturbanceSpec`], the between-run overlays that apply some kinds to a
//! config copy, and the hidden ground truth (A-DIST, `roko.disturbance/1`,
//! S01 v1.3 §5.10).
//!
//! - The kinds are `disturb.py`'s six (`KINDS` in
//!   `benchmarks/viabilitybench/driver/disturb.py`, which a test reads so
//!   the lists cannot drift) plus the optional `price_shock`. A spec file
//!   has `disturb.py`'s format: `schema_version = "vb.disturbance/1"` and
//!   `[[disturbance]]` tables.
//! - [`overlay`] applies `budget_cut` (the per-task budget caps scaled) and
//!   `model_swap` (a `[routing.ladder]` rung's model replaced, the tier
//!   alias) to a copy of the config, and points `price_shock`'s run at its
//!   shocked price snapshot, which [`shocked_prices`] writes. The other
//!   kinds are applied by the benchmark driver alone (the fault proxy, the
//!   task generator, the visible-verify wrapper).
//! - The ground truth goes to the run's `disturbances.jsonl`, one row when a
//!   disturbance starts and one when it ends ([`GroundTruthWriter`]), and to
//!   the manifest's `experiment.disturbance_spec` ([`manifest_value`]). The
//!   controller never reads either: a test in roko-learn holds its module to
//!   that.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::RokoConfig;
use crate::pricing_snapshot::PriceSnapshot;

/// `schema_version` of a ground-truth row.
pub const DISTURBANCE_SCHEMA: &str = "roko.disturbance/1";
/// The run file of the ground truth, in `.roko/runs/<run_id>/`.
pub const DISTURBANCES_FILE: &str = "disturbances.jsonl";
/// `schema_version` of a spec file, `disturb.py`'s `SCHEMA`.
pub const SPEC_SCHEMA: &str = "vb.disturbance/1";

/// A canonical disturbance (S06 §4.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisturbanceKind {
    /// An HTTP fault on one provider, or every provider slowed.
    ProviderFault,
    /// A tier's model replaced by a weaker one.
    ModelSwap,
    /// The task sampler switches to the hard pool.
    HarderMix,
    /// The per-task budget cut.
    BudgetCut,
    /// The repository's conventions change under the tasks.
    ConventionFlip,
    /// The visible verify step fails at random.
    FlakyVerify,
    /// One model's prices multiplied (optional, outside H6).
    PriceShock,
}

impl DisturbanceKind {
    /// `disturb.py`'s `KINDS`, in its order.
    pub const DISTURB_PY: [Self; 6] = [
        Self::ProviderFault,
        Self::ModelSwap,
        Self::HarderMix,
        Self::BudgetCut,
        Self::ConventionFlip,
        Self::FlakyVerify,
    ];

    /// Every kind: `disturb.py`'s, then `price_shock`.
    pub const ALL: [Self; 7] = [
        Self::ProviderFault,
        Self::ModelSwap,
        Self::HarderMix,
        Self::BudgetCut,
        Self::ConventionFlip,
        Self::FlakyVerify,
        Self::PriceShock,
    ];

    /// The kind's name, as specs and rows write it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ProviderFault => "provider_fault",
            Self::ModelSwap => "model_swap",
            Self::HarderMix => "harder_mix",
            Self::BudgetCut => "budget_cut",
            Self::ConventionFlip => "convention_flip",
            Self::FlakyVerify => "flaky_verify",
            Self::PriceShock => "price_shock",
        }
    }

    /// The kind called `name`.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.name() == name)
    }

    /// Whether [`overlay`] applies the kind between runs; the benchmark
    /// driver applies the others.
    #[must_use]
    pub const fn has_overlay(self) -> bool {
        matches!(self, Self::BudgetCut | Self::ModelSwap | Self::PriceShock)
    }

    /// Where the injection shows (S01 v1.3 §5.10's `ground_truth`).
    #[must_use]
    pub const fn ground_truth(self) -> &'static str {
        match self {
            Self::ProviderFault | Self::ModelSwap => "proxy:fault_injected",
            Self::HarderMix => "stream_position",
            Self::BudgetCut => "policy_version, config_hash",
            Self::ConventionFlip => "latent_version",
            Self::FlakyVerify => "visible.flake_injected",
            Self::PriceShock => "price_snapshot_id",
        }
    }
}

/// Why a spec or an overlay cannot be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DisturbanceError {
    /// The spec file is not `disturb.py`'s format.
    #[error("not a {SPEC_SCHEMA} file: {0}")]
    File(String),
    /// A spec's positions or parameters cannot be right.
    #[error("{kind}: {problem}")]
    Invalid {
        /// The kind.
        kind: &'static str,
        /// What is wrong.
        problem: String,
    },
    /// Only the benchmark driver applies the kind.
    #[error("{0} has no between-run overlay: the benchmark driver applies it")]
    NoOverlay(&'static str),
}

/// One disturbance: its kind and parameters, the stream positions it covers
/// (from 1; `end_at` none means to the end), and its seed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisturbanceSpec {
    /// The kind.
    pub kind: DisturbanceKind,
    /// The kind's parameters: `provider_fault {fault, rate, provider}`,
    /// `model_swap {tier, to}`, `flaky_verify {p}`, `budget_cut {factor}`,
    /// `price_shock {model, k, snapshot}`.
    #[serde(default)]
    pub params: BTreeMap<String, Value>,
    /// The first position it covers.
    #[serde(default = "first_position")]
    pub start_at: u64,
    /// The last position it covers; `None` runs to the end.
    #[serde(default)]
    pub end_at: Option<u64>,
    /// Its seed.
    #[serde(default)]
    pub seed: u64,
}

const fn first_position() -> u64 {
    1
}

/// A spec file: `disturb.py`'s format.
#[derive(Deserialize)]
struct SpecFile {
    schema_version: String,
    disturbance: Vec<DisturbanceSpec>,
}

impl DisturbanceSpec {
    /// Parse and check a spec file.
    ///
    /// # Errors
    ///
    /// [`DisturbanceError::File`] when the text is not a spec file, and the
    /// first spec's [`Self::check`] error.
    pub fn parse_file(text: &str) -> Result<Vec<Self>, DisturbanceError> {
        let file: SpecFile =
            toml::from_str(text).map_err(|error| DisturbanceError::File(error.to_string()))?;
        if file.schema_version != SPEC_SCHEMA || file.disturbance.is_empty() {
            return Err(DisturbanceError::File(
                "it needs schema_version and one or more [[disturbance]] tables".to_string(),
            ));
        }
        for spec in &file.disturbance {
            spec.check()?;
        }
        Ok(file.disturbance)
    }

    /// Whether the spec covers stream position `position`.
    #[must_use]
    pub fn covers(&self, position: u64) -> bool {
        self.start_at <= position && self.end_at.is_none_or(|end| position <= end)
    }

    /// Check the positions and the parameters the kind needs.
    ///
    /// # Errors
    ///
    /// [`DisturbanceError::Invalid`] with what is wrong.
    pub fn check(&self) -> Result<(), DisturbanceError> {
        let invalid = |problem: &str| DisturbanceError::Invalid {
            kind: self.kind.name(),
            problem: problem.to_string(),
        };
        if self.start_at == 0 || self.end_at.is_some_and(|end| end < self.start_at) {
            return Err(invalid("start_at is a position from 1, and end_at none before it"));
        }
        match self.kind {
            DisturbanceKind::BudgetCut => {
                let factor = self.number("factor").unwrap_or(0.5);
                if !(factor > 0.0 && factor <= 1.0) {
                    return Err(invalid("factor must be in (0, 1]"));
                }
            }
            DisturbanceKind::ModelSwap if self.text("to").is_none() => {
                return Err(invalid("needs `to`, the model that replaces the tier's"));
            }
            DisturbanceKind::FlakyVerify => {
                let p = self.number("p").unwrap_or(0.25);
                if !(0.0..=1.0).contains(&p) {
                    return Err(invalid("p must be in [0, 1]"));
                }
            }
            DisturbanceKind::PriceShock => {
                let k = self.number("k").unwrap_or(0.0);
                if self.text("model").is_none() || k <= 0.0 {
                    return Err(invalid("needs `model` and a multiplier `k` above 0"));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn number(&self, key: &str) -> Option<f64> {
        self.params.get(key).and_then(Value::as_f64)
    }

    fn text(&self, key: &str) -> Option<&str> {
        self.params.get(key).and_then(Value::as_str)
    }
}

/// `spec` applied between runs to a copy of `config`: `budget_cut` scales
/// `budget.max_task_usd` and `budget.max_task_retry_usd` by `factor`;
/// `model_swap` replaces the model of the `[routing.ladder]` rung `tier`
/// (every rung when `tier` is absent) with `to`; `price_shock` points
/// `[pricing] snapshot` at `snapshot`, the shocked table [`shocked_prices`]
/// writes.
///
/// # Errors
///
/// [`DisturbanceError::NoOverlay`] for a kind only the benchmark driver
/// applies, and [`DisturbanceError::Invalid`] for a spec the config cannot
/// take: an unlimited task budget, a rung the ladder lacks, or no snapshot
/// id.
pub fn overlay(
    config: &RokoConfig,
    spec: &DisturbanceSpec,
) -> Result<RokoConfig, DisturbanceError> {
    spec.check()?;
    let invalid = |problem: String| DisturbanceError::Invalid {
        kind: spec.kind.name(),
        problem,
    };
    let mut changed = config.clone();
    match spec.kind {
        DisturbanceKind::BudgetCut => {
            let factor = spec.number("factor").unwrap_or(0.5) as f32;
            if changed.budget.max_task_usd <= 0.0 {
                return Err(invalid(
                    "budget.max_task_usd is unlimited (0), so there is nothing to cut".to_string(),
                ));
            }
            changed.budget.max_task_usd *= factor;
            changed.budget.max_task_retry_usd *= factor;
        }
        DisturbanceKind::ModelSwap => {
            let to = spec.text("to").unwrap_or_default().to_string();
            let tier = spec.text("tier");
            let mut swapped = 0;
            for rung in &mut changed.routing.ladder.rungs {
                if tier.is_none_or(|name| rung.name == name) {
                    rung.model.clone_from(&to);
                    swapped += 1;
                }
            }
            if swapped == 0 {
                return Err(invalid(format!(
                    "the ladder has no rung {}",
                    tier.unwrap_or_default()
                )));
            }
        }
        DisturbanceKind::PriceShock => {
            let Some(snapshot) = spec.text("snapshot") else {
                return Err(invalid("needs `snapshot`, the shocked table's id".to_string()));
            };
            changed.pricing.snapshot = snapshot.to_string();
        }
        kind => return Err(DisturbanceError::NoOverlay(kind.name())),
    }
    Ok(changed)
}

/// The price table of a `price_shock`: `snapshot_toml` with the rates of
/// the spec's `model` multiplied by its `k`, under the id `id`
/// (`prices-YYYY-MM-DD`, the file the run's workspace keeps it in).
///
/// # Errors
///
/// [`DisturbanceError::Invalid`] when the spec is not a price shock, the
/// table lacks the model, or the result is not a valid snapshot.
pub fn shocked_prices(
    snapshot_toml: &str,
    spec: &DisturbanceSpec,
    id: &str,
) -> Result<String, DisturbanceError> {
    let invalid = |problem: String| DisturbanceError::Invalid {
        kind: DisturbanceKind::PriceShock.name(),
        problem,
    };
    if spec.kind != DisturbanceKind::PriceShock {
        return Err(invalid(format!("{} is not a price shock", spec.kind.name())));
    }
    spec.check()?;
    let model = spec.text("model").unwrap_or_default();
    let k = spec.number("k").unwrap_or(1.0);
    let mut table: toml::Table =
        toml::from_str(snapshot_toml).map_err(|error| invalid(error.to_string()))?;
    table.insert("id".to_string(), toml::Value::String(id.to_string()));
    let rows = table
        .get_mut("model")
        .and_then(toml::Value::as_array_mut)
        .ok_or_else(|| invalid("the table has no [[model]] rows".to_string()))?;
    let mut found = false;
    for row in rows.iter_mut().filter_map(toml::Value::as_table_mut) {
        if row.get("slug").and_then(toml::Value::as_str) != Some(model) {
            continue;
        }
        found = true;
        for rate in ["input", "cache_read", "cache_write_5m", "cache_write_1h", "output"] {
            let value = row.get(rate).and_then(|value| {
                value
                    .as_float()
                    .or_else(|| value.as_integer().map(|whole| whole as f64))
            });
            if let Some(value) = value {
                row.insert(rate.to_string(), toml::Value::Float(value * k));
            }
        }
    }
    if !found {
        return Err(invalid(format!("the table has no model {model}")));
    }
    let text = toml::to_string(&table).map_err(|error| invalid(error.to_string()))?;
    PriceSnapshot::from_toml(&text, "price_shock").map_err(|error| invalid(error.to_string()))?;
    Ok(text)
}

/// The manifest's `experiment.disturbance_spec`: the run's specs.
#[must_use]
pub fn manifest_value(specs: &[DisturbanceSpec]) -> Value {
    serde_json::to_value(specs).unwrap_or(Value::Null)
}

/// A ground-truth row's event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DisturbanceEvent {
    /// The disturbance started.
    #[serde(rename = "disturbance.inject")]
    Inject,
    /// It ended.
    #[serde(rename = "disturbance.end")]
    End,
}

/// Where a disturbance came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// A spec file given to the run or the benchmark driver.
    SpecFile,
    /// The demo's admin route.
    AdminRoute,
}

/// One A-DIST row of a run's `disturbances.jsonl`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisturbanceRecord {
    /// [`DISTURBANCE_SCHEMA`].
    pub schema_version: String,
    /// The id readers dedupe on: `b3:` of schema, run, event, id and seq.
    pub record_id: String,
    /// The run's sequence number of the row.
    pub seq: u64,
    /// ISO-8601 UTC time of the row.
    pub ts: String,
    /// `disturbance.inject` or `disturbance.end`.
    pub kind: DisturbanceEvent,
    /// The disturbance's id in the run, such as `dist-3`.
    pub id: String,
    /// The disturbance's kind.
    #[serde(rename = "type")]
    pub disturbance: DisturbanceKind,
    /// Its parameters.
    pub params: BTreeMap<String, Value>,
    /// The spec's `start_at`.
    pub start_resolution: u64,
    /// The spec's `end_at`; `null` while open-ended.
    pub end_resolution: Option<u64>,
    /// The spec's seed.
    pub seed: u64,
    /// Where it came from.
    pub origin: Origin,
    /// Where the injection shows.
    pub ground_truth: String,
}

/// Appends a run's ground truth to its `disturbances.jsonl`.
#[derive(Debug, Clone)]
pub struct GroundTruthWriter {
    path: PathBuf,
    run_id: String,
    seq: u64,
}

impl GroundTruthWriter {
    /// The writer of the run `run_id` whose directory is `run_dir`.
    #[must_use]
    pub fn new(run_dir: &Path, run_id: &str) -> Self {
        Self {
            path: run_dir.join(DISTURBANCES_FILE),
            run_id: run_id.to_string(),
            seq: 0,
        }
    }

    /// The file the writer appends to.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append the row of `event` for the disturbance `id`, as `spec` and
    /// `origin` describe it, at `ts`, and return it.
    ///
    /// # Errors
    ///
    /// I/O errors creating the directory or appending the row.
    pub fn record(
        &mut self,
        event: DisturbanceEvent,
        id: &str,
        spec: &DisturbanceSpec,
        origin: Origin,
        ts: &str,
    ) -> std::io::Result<DisturbanceRecord> {
        self.seq += 1;
        let event_name = match event {
            DisturbanceEvent::Inject => "disturbance.inject",
            DisturbanceEvent::End => "disturbance.end",
        };
        let parts = [
            DISTURBANCE_SCHEMA,
            self.run_id.as_str(),
            event_name,
            id,
            &self.seq.to_string(),
        ];
        let record = DisturbanceRecord {
            schema_version: DISTURBANCE_SCHEMA.to_string(),
            record_id: format!("b3:{}", blake3::hash(parts.join("|").as_bytes()).to_hex()),
            seq: self.seq,
            ts: ts.to_string(),
            kind: event,
            id: id.to_string(),
            disturbance: spec.kind,
            params: spec.params.clone(),
            start_resolution: spec.start_at,
            end_resolution: spec.end_at,
            seed: spec.seed,
            origin,
            ground_truth: spec.kind.ground_truth().to_string(),
        };
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut line = serde_json::to_string(&record).map_err(std::io::Error::other)?;
        line.push('\n');
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?
            .write_all(line.as_bytes())?;
        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `disturb.py`, whose `KINDS` the Rust list must match.
    const DISTURB_PY: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../benchmarks/viabilitybench/driver/disturb.py"
    );

    #[test]
    fn disturbance_kinds_match_benchmark_driver() {
        let source = std::fs::read_to_string(DISTURB_PY).expect("read disturb.py");
        let line = source
            .lines()
            .find(|line| line.starts_with("KINDS = ("))
            .expect("disturb.py defines KINDS");
        let kinds: Vec<&str> = line
            .split('"')
            .skip(1)
            .step_by(2)
            .collect();
        let ours: Vec<&str> = DisturbanceKind::DISTURB_PY
            .iter()
            .map(|kind| kind.name())
            .collect();
        assert_eq!(kinds, ours, "{line}");
        for kind in DisturbanceKind::ALL {
            assert_eq!(DisturbanceKind::parse(kind.name()), Some(kind));
        }

        // A spec file in disturb.py's format parses and checks.
        let specs = DisturbanceSpec::parse_file(
            "schema_version = \"vb.disturbance/1\"\n\n[[disturbance]]\nkind = \"budget_cut\"\n\
             start_at = 20\nseed = 7\nparams = { factor = 0.5 }\n\n[[disturbance]]\n\
             kind = \"model_swap\"\nstart_at = 30\nend_at = 60\nparams = { tier = \"mid\", \
             to = \"weak\" }\n",
        )
        .expect("a spec file");
        assert_eq!(specs.len(), 2);
        assert!(specs[1].covers(30) && specs[1].covers(60) && !specs[1].covers(61));
        assert!(DisturbanceSpec::parse_file("schema_version = \"vb.disturbance/1\"\n").is_err());
        let bad = "schema_version = \"vb.disturbance/1\"\n[[disturbance]]\nkind = \"meteor\"\n";
        assert!(DisturbanceSpec::parse_file(bad).is_err());

        // Overlays: budget_cut halves the task caps; model_swap swaps one
        // rung's model; the driver's kinds have none.
        let mut config = RokoConfig::default();
        config.budget.max_task_usd = 2.0;
        let cut = overlay(&config, &specs[0]).expect("a budget cut");
        assert!((cut.budget.max_task_usd - 1.0).abs() < 1e-6);
        assert!(
            (cut.budget.max_task_retry_usd - config.budget.max_task_retry_usd * 0.5).abs() < 1e-6
        );
        let swapped = overlay(&config, &specs[1]).expect("a model swap");
        let mid = swapped
            .routing
            .ladder
            .rungs
            .iter()
            .find(|rung| rung.name == "mid")
            .expect("a mid rung");
        assert_eq!(mid.model, "weak");
        let others_kept = swapped
            .routing
            .ladder
            .rungs
            .iter()
            .zip(&config.routing.ladder.rungs)
            .all(|(after, before)| after.name == "mid" || after.model == before.model);
        assert!(others_kept);
        let flaky = DisturbanceSpec {
            kind: DisturbanceKind::FlakyVerify,
            params: BTreeMap::new(),
            start_at: 1,
            end_at: None,
            seed: 0,
        };
        assert!(matches!(
            overlay(&config, &flaky),
            Err(DisturbanceError::NoOverlay("flaky_verify"))
        ));
        config.budget.max_task_usd = 0.0;
        assert!(overlay(&config, &specs[0]).is_err(), "an unlimited budget");

        // A price shock writes a valid snapshot with one model's rates × k.
        let base = PriceSnapshot::builtin().expect("the built-in snapshot");
        let model = base.rows()[0].slug.clone();
        let builtin_text = include_str!("../../../config/prices/2026-09-28.toml");
        let shock = DisturbanceSpec {
            kind: DisturbanceKind::PriceShock,
            params: BTreeMap::from([
                ("model".to_string(), Value::from(model.as_str())),
                ("k".to_string(), Value::from(3.0)),
                ("snapshot".to_string(), Value::from("prices-2026-10-03")),
            ]),
            start_at: 1,
            end_at: None,
            seed: 0,
        };
        let text = shocked_prices(builtin_text, &shock, "prices-2026-10-03").expect("shocked");
        let shocked = PriceSnapshot::from_toml(&text, "test").expect("a valid snapshot");
        assert_eq!(shocked.id(), "prices-2026-10-03");
        let before = base.row(&model).expect("the model's row");
        let after = shocked.row(&model).expect("the shocked row");
        assert!((after.output - 3.0 * before.output).abs() < 1e-9);
        let pointed = overlay(&RokoConfig::default(), &shock).expect("a price shock");
        assert_eq!(pointed.pricing.snapshot, "prices-2026-10-03");
    }

    #[test]
    fn ground_truth_rows_round_trip_s01_example() {
        // S01 v1.3 §5.10's A-DIST example, with the envelope it leaves out.
        let example = r#"{"schema_version":"roko.disturbance/1","record_id":"b3:9e1a","seq":1,"ts":"2026-10-02T14:03:21.950Z","kind":"disturbance.inject","id":"dist-3","type":"model_swap","params":{"tier":"mid","to":"weak"},"start_resolution":20,"end_resolution":null,"seed":7,"origin":"spec_file","ground_truth":"proxy:fault_injected"}"#;
        let row: Value = serde_json::from_str(example).expect("the example parses");
        let record: DisturbanceRecord =
            serde_json::from_value(row.clone()).expect("a ground-truth row");
        assert_eq!(serde_json::to_value(&record).expect("serialize"), row);

        // The writer appends inject and end rows to the run's file, and the
        // manifest carries the spec list.
        let dir = std::env::temp_dir().join(format!("roko-dist-{}", std::process::id()));
        let mut writer = GroundTruthWriter::new(&dir, "gr-test");
        let spec = DisturbanceSpec {
            kind: DisturbanceKind::ModelSwap,
            params: record.params.clone(),
            start_at: 20,
            end_at: None,
            seed: 7,
        };
        let ts = "2026-10-02T14:03:21.950Z";
        let inject = writer
            .record(DisturbanceEvent::Inject, "dist-3", &spec, Origin::SpecFile, ts)
            .expect("inject row");
        let end = writer
            .record(DisturbanceEvent::End, "dist-3", &spec, Origin::SpecFile, ts)
            .expect("end row");
        assert_eq!((inject.seq, end.seq), (1, 2));
        assert_ne!(inject.record_id, end.record_id);
        assert_eq!(inject.ground_truth, "proxy:fault_injected");
        let text = std::fs::read_to_string(writer.path()).expect("read the file");
        assert_eq!(text.lines().count(), 2);
        assert!(writer.path().ends_with(DISTURBANCES_FILE));
        std::fs::remove_dir_all(&dir).expect("clean up");
        let manifest = manifest_value(&[spec]);
        assert_eq!(manifest[0]["kind"], "model_swap");
        assert_eq!(manifest[0]["start_at"], 20);
    }
}

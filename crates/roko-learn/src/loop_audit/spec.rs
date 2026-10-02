//! The loop contract and the loop registry (S03 §4.2, §4.6).
//!
//! Every learning loop registers a [`LoopSpec`]: the executed decision its
//! learned state should reach, the layer that randomizes that decision, when
//! the loop has an opportunity, the default policy π⁰ it is compared with,
//! and the receipt that proves the learned artifact reached the executed
//! request. Its static findings are the code-review facts the census prints
//! as `declared` evidence, each with a `path::symbol` pointer and the commit
//! it was checked at. A loop without a contract is reported `UNREGISTERED`.
//!
//! [`Registry::embedded`] parses the registry compiled into the crate,
//! `loops.toml`. [`Registry::load`] merges a workspace's
//! `.roko/learn/loop-registry.toml` into it by loop id: an override entry
//! replaces the keys it sets, and an entry with a new id adds a loop. Every
//! registry is validated before use, and a broken override fails the load.
//!
//! [`ReasonCode`] is S03's one closed reason enum: the census, `loop.health`
//! and `loop.transition` use nothing else. When several codes apply, the
//! cheapest check wins ([`ReasonCode::cheapest`]).

use std::collections::HashSet;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::telemetry::AssignmentUnit;

/// Schema id of a registry file, after S01 §9.4's `roko.<name>/<major>`.
pub const REGISTRY_SCHEMA: &str = "roko.loop_registry/1";
/// A workspace's registry override, relative to the workspace root.
pub const REGISTRY_OVERRIDE_PATH: &str = ".roko/learn/loop-registry.toml";
/// The outcome β is measured on unless a loop names another: S03 §4.4's U,
/// a verified pass less M4's inverse-propensity-weighted false greens.
pub const DEFAULT_OUTCOME: &str = "U";
/// The strata β is reported over unless a loop names others.
pub const DEFAULT_STRATA: [&str; 3] = ["task_type", "role", "tier"];

/// The registry compiled into the crate.
const EMBEDDED_REGISTRY: &str = include_str!("loops.toml");
/// Names the embedded registry in errors.
const EMBEDDED_ORIGIN: &str = "the embedded loop registry";
/// The registry's array of tables, `[[loop]]`.
const LOOPS_KEY: &str = "loop";

/// A loop's registry id: `L-` and a name, e.g. `L-route` or `L-dream-bias`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LoopId(String);

impl LoopId {
    /// Wrap `id`. Registry validation checks its form.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The id as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The name after `L-` in snake case, which names a nested loop's
    /// layer: `L-dream-bias` gives `dream_bias`.
    #[must_use]
    pub fn slug(&self) -> String {
        let name = self.0.strip_prefix("L-").unwrap_or(self.0.as_str());
        name.to_ascii_lowercase().replace('-', "_")
    }

    /// Whether the id is `L-` followed by ASCII letters, digits and dashes.
    fn is_well_formed(&self) -> bool {
        let name = self.0.strip_prefix("L-").unwrap_or_default();
        !name.is_empty() && name.bytes().all(is_id_byte)
    }
}

impl fmt::Display for LoopId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// An assignment layer: one per decision point (S03 §4.3), e.g. `route`,
/// `knowledge` or `placebo`. A nested loop decides on its parent's layer and
/// draws on `<layer>.<loop slug>` (`route.dream_bias`), only inside the
/// parent's learned arm.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Layer(String);

impl Layer {
    /// Wrap `name`. Registry validation checks its form.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// The layer name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The layer `loop_id` draws on when it is nested in a loop on this
    /// layer: `route` and `L-dream-bias` give `route.dream_bias`.
    #[must_use]
    pub fn nested(&self, loop_id: &LoopId) -> Self {
        Self(format!("{}.{}", self.0, loop_id.slug()))
    }

    /// Whether the name is dot-separated parts of lowercase ASCII letters,
    /// digits and underscores.
    fn is_well_formed(&self) -> bool {
        self.0.split('.').all(is_layer_part)
    }
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Where a loop stands in the registry, outside the four audit states (S03
/// §4.6).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
    /// Randomized on its layer and audited.
    #[default]
    Active,
    /// Registered and measured, never randomized or enforced (L-gate-thr per
    /// S02 decision 1; L-err until a withhold arm is approved).
    ObserveOnly,
    /// S02 deleted the loop's code. Its history stays readable, the census
    /// prints the deleting task, and the loop makes no transitions and leaves
    /// K.
    Retired {
        /// The task that deleted it, e.g. `S02.P1-8`.
        by: String,
    },
}

impl Lifecycle {
    /// Whether the loop is randomized on its layer: only active loops are.
    #[must_use]
    pub fn is_randomized(&self) -> bool {
        matches!(self, Self::Active)
    }
}

/// The audit state machine's four states (S03 §4.6). A state carries a
/// [`ReasonCode`] outside this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditState {
    /// New or re-admitted, at the probation holdout rate.
    #[default]
    Probation,
    /// Proven benefit: the learned policy runs, at the live holdout rate.
    Live,
    /// Dormant, inert or null; the reason says which.
    Flagged,
    /// Harmful: π⁰ runs when enforced, and learning continues in shadow.
    Demoted,
}

/// S03 §4.6's one closed reason enum, in precedence order: when several codes
/// apply, the first one wins, because its check is the cheapest. A
/// write-only state with no Graph opportunity is therefore
/// `dormant:no_opportunity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReasonCode {
    /// The arm is drawn after the decision, so it only labels the decision.
    #[serde(rename = "dormant:label_only")]
    LabelOnly,
    /// The state's decision-relevant variance is 0.
    #[serde(rename = "dormant:degenerate")]
    Degenerate,
    /// No opportunity arises on the executed path.
    #[serde(rename = "dormant:no_opportunity")]
    NoOpportunity,
    /// The writer never updates the state on settled opportunities.
    #[serde(rename = "dormant:no_learning")]
    NoLearning,
    /// The state is updated, but no executed decision loads it.
    #[serde(rename = "dormant:write_only")]
    WriteOnly,
    /// The reader runs but returns empty state.
    #[serde(rename = "dormant:cut")]
    Cut,
    /// The reader runs on an old state version.
    #[serde(rename = "dormant:stale")]
    Stale,
    /// The executed decision is not the learned one, under the learned label.
    #[serde(rename = "dormant:mask")]
    Mask,
    /// The exposure receipt is missing.
    #[serde(rename = "dormant:unlogged")]
    Unlogged,
    /// Exposed, but no net influence over the A/A floor.
    #[serde(rename = "inert")]
    Inert,
    /// Exposed and influential, but the benefit is narrowly null.
    #[serde(rename = "null")]
    Null,
    /// The benefit is harmful.
    #[serde(rename = "harm")]
    Harm,
}

impl ReasonCode {
    /// Every code, in precedence order.
    pub const ALL: [Self; 12] = [
        Self::LabelOnly,
        Self::Degenerate,
        Self::NoOpportunity,
        Self::NoLearning,
        Self::WriteOnly,
        Self::Cut,
        Self::Stale,
        Self::Mask,
        Self::Unlogged,
        Self::Inert,
        Self::Null,
        Self::Harm,
    ];

    /// The code's rank: 0 is checked first and wins over every later code.
    #[must_use]
    pub const fn precedence(self) -> u8 {
        self as u8
    }

    /// The serialized form, e.g. `dormant:write_only`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LabelOnly => "dormant:label_only",
            Self::Degenerate => "dormant:degenerate",
            Self::NoOpportunity => "dormant:no_opportunity",
            Self::NoLearning => "dormant:no_learning",
            Self::WriteOnly => "dormant:write_only",
            Self::Cut => "dormant:cut",
            Self::Stale => "dormant:stale",
            Self::Mask => "dormant:mask",
            Self::Unlogged => "dormant:unlogged",
            Self::Inert => "inert",
            Self::Null => "null",
            Self::Harm => "harm",
        }
    }

    /// Whether the code says the learned state does not reach the executed
    /// decision (`dormant:*`).
    #[must_use]
    pub const fn is_dormant(self) -> bool {
        self.precedence() < Self::Inert.precedence()
    }

    /// The code that wins among `codes`: the first in precedence order.
    #[must_use]
    pub fn cheapest(codes: impl IntoIterator<Item = Self>) -> Option<Self> {
        codes.into_iter().min_by_key(|code| code.precedence())
    }
}

impl fmt::Display for ReasonCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A closed list of qualifiers, printed after the reason and never in its
/// place (S03 §4.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Qualifier {
    /// The loop's own experiment or accounting is invalid, e.g.
    /// `HoldoutExperiment`'s fixed per-task arm and zero cost.
    Misspecified,
    /// The evidence predates the typed verdict, so β excludes it.
    PreInstrumentation,
    /// A declared finding's `verified_at` is not the harness's commit.
    DeclaredStale,
}

impl Qualifier {
    /// The serialized form, e.g. `declared_stale`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Misspecified => "misspecified",
            Self::PreInstrumentation => "pre_instrumentation",
            Self::DeclaredStale => "declared_stale",
        }
    }
}

impl fmt::Display for Qualifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What proves that a learned artifact reached the executed request (S03
/// §4.2, registration rule 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptKind {
    /// The model the provider call record names, not the routed slug.
    ExecutedModel,
    /// A hash of the rendered artifact inside the request body.
    ArtifactHashInRequest,
    /// The ids of the sections the request included.
    IncludedIds,
    /// The threshold in force when the decision was made.
    ThresholdUsed,
    /// The params digest or policy version in the decision record.
    ParamsDigest,
    /// The prediction id the decision consumed.
    PredictionConsumed,
    /// An audit-derived penalty id in a later decision's learned state.
    AuditPenaltyApplied,
    /// The placebo's: there is nothing to prove.
    Sham,
}

/// A code-review fact the census prints as `declared` evidence (S03 §9.11).
/// `verified_at` rots by design: the census qualifies a finding checked at
/// another commit than the harness's as `declared_stale`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticFinding {
    /// The fact, in a sentence.
    pub claim: String,
    /// `path::symbol`: a workspace-relative file and the symbol the claim is
    /// about, e.g.
    /// `crates/roko-cli/src/dispatch/model_routing.rs::ModelRouter::route`.
    pub pointer: String,
    /// The commit the claim was checked at.
    pub verified_at: String,
    /// The reason code the fact is evidence for, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<ReasonCode>,
    /// The qualifier the fact is evidence for, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qualifier: Option<Qualifier>,
}

impl StaticFinding {
    /// The pointer's file and symbol, split at the first `::` after the last
    /// `/`. `None` when either part is missing.
    #[must_use]
    pub fn pointer_parts(&self) -> Option<(&str, &str)> {
        let file_start = self.pointer.rfind('/').map_or(0, |slash| slash + 1);
        let split = file_start + self.pointer[file_start..].find("::")?;
        let (file, symbol) = (&self.pointer[..split], &self.pointer[split + 2..]);
        (!file.is_empty() && !symbol.is_empty()).then_some((file, symbol))
    }

    /// The first rule the finding breaks, if any.
    fn check(&self) -> Result<(), String> {
        if self.claim.trim().is_empty() {
            return Err(format!("the finding at `{}` has no claim", self.pointer));
        }
        if self.pointer_parts().is_none() {
            return Err(format!(
                "finding pointer `{}` is not `path::symbol`",
                self.pointer
            ));
        }
        if !is_commit(&self.verified_at) {
            return Err(format!(
                "the finding at `{}` has no commit sha in verified_at",
                self.pointer
            ));
        }
        Ok(())
    }
}

/// One loop's contract (S03 §4.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoopSpec {
    /// Registry id, e.g. `L-route`.
    pub id: LoopId,
    /// Contract version, bumped when the contract changes.
    #[serde(default = "default_version")]
    pub version: u32,
    /// What the loop is, in a few words.
    #[serde(default)]
    pub name: String,
    /// The executed decision its learned state should reach, as a code path
    /// (`dispatch::model_routing::ModelRouter::route`).
    #[serde(default)]
    pub decision_point: String,
    /// The layer of the decision point. A nested loop names its parent's.
    pub layer: Layer,
    /// The parent loop, for a loop drawn only inside the parent's learned
    /// arm (L-dream-bias and L-linucb inside L-route).
    #[serde(default)]
    pub nested_in: Option<LoopId>,
    /// What the layer randomizes: the task chain unless stated.
    #[serde(default = "default_unit")]
    pub unit: AssignmentUnit,
    /// The opportunity predicate, over the pre-treatment context.
    #[serde(default)]
    pub opportunity: String,
    /// The default policy π⁰, computed on both arms.
    #[serde(default)]
    pub default_policy: String,
    /// What proves the learned artifact reached the executed request.
    pub receipt: ReceiptKind,
    /// The outcome β is measured on; [`DEFAULT_OUTCOME`] unless stated.
    #[serde(default = "default_outcome")]
    pub outcome: String,
    /// The strata β is reported over; [`DEFAULT_STRATA`] unless stated.
    #[serde(default = "default_strata")]
    pub strata: Vec<String>,
    /// Whether a demotion changes the executed policy. Off until C2 and C4
    /// pass and 14 days have elapsed (S03 §4.6 guard 6, D10).
    #[serde(default)]
    pub enforce: bool,
    /// Audited but never demoted (S03 §4.6 guard 6).
    #[serde(default)]
    pub exempt: bool,
    /// Where the loop stands in the registry.
    #[serde(default)]
    pub lifecycle: Lifecycle,
    /// A fixed holdout rate that replaces the state schedule: L-M1's 0.10,
    /// the placebo's 0.5.
    #[serde(default)]
    pub fixed_holdout: Option<f64>,
    /// The code-review facts the census prints as `declared` evidence.
    #[serde(default)]
    pub static_findings: Vec<StaticFinding>,
    /// Anything else a reader needs: pending retirements, decisions.
    #[serde(default)]
    pub notes: Vec<String>,
}

impl LoopSpec {
    /// The layer this loop's arm is drawn on: its decision layer, or
    /// `<layer>.<slug>` when it is nested in a parent's learned arm.
    #[must_use]
    pub fn assignment_layer(&self) -> Layer {
        if self.nested_in.is_some() {
            self.layer.nested(&self.id)
        } else {
            self.layer.clone()
        }
    }

    /// The first registration rule this entry breaks on its own, if any. The
    /// registry checks the rules that span entries.
    fn check(&self) -> Result<(), String> {
        if !self.id.is_well_formed() {
            return Err("the id is not `L-` and ASCII letters, digits or dashes".to_string());
        }
        let required = [
            ("name", &self.name),
            ("decision_point", &self.decision_point),
            ("opportunity", &self.opportunity),
            ("default_policy (π⁰)", &self.default_policy),
            ("outcome", &self.outcome),
        ];
        if let Some((field, _)) = required.iter().find(|(_, value)| value.trim().is_empty()) {
            return Err(format!("{field} is empty"));
        }
        if !self.layer.is_well_formed() {
            return Err(format!(
                "layer `{}` is not a dotted lowercase name",
                self.layer
            ));
        }
        if self.fixed_holdout.is_some_and(|h| !(h > 0.0 && h < 1.0)) {
            return Err("fixed_holdout is not strictly between 0 and 1".to_string());
        }
        if matches!(&self.lifecycle, Lifecycle::Retired { by } if by.trim().is_empty()) {
            return Err("a retired loop does not name the task that retired it".to_string());
        }
        self.static_findings
            .iter()
            .try_for_each(StaticFinding::check)
    }
}

/// The registered loops: the embedded registry, optionally merged with a
/// workspace override. Every loop in it passed validation.
#[derive(Debug, Clone, PartialEq)]
pub struct Registry {
    loops: Vec<LoopSpec>,
}

impl Registry {
    /// Parse and validate a registry file: an optional `schema` and its
    /// `[[loop]]` entries.
    pub fn from_toml_str(text: &str) -> Result<Self, RegistryError> {
        Self::parse(text, "loop registry")
    }

    /// The registry compiled into the crate (`loops.toml`).
    pub fn embedded() -> Result<Self, RegistryError> {
        Self::parse(EMBEDDED_REGISTRY, EMBEDDED_ORIGIN)
    }

    /// The embedded registry, merged by loop id with the workspace override
    /// at [`REGISTRY_OVERRIDE_PATH`] under `workdir` when it exists: an
    /// override entry replaces the keys it sets, and one with a new id is
    /// appended. An unreadable or invalid override is an error.
    pub fn load(workdir: &Path) -> Result<Self, RegistryError> {
        let path = workdir.join(REGISTRY_OVERRIDE_PATH);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Self::embedded(),
            Err(source) => return Err(RegistryError::Io { path, source }),
        };
        let origin = path.display().to_string();
        let overrides = toml::from_str(&text).map_err(|source| parse_error(&origin, source))?;
        let mut merged = toml::from_str(EMBEDDED_REGISTRY)
            .map_err(|source| parse_error(EMBEDDED_ORIGIN, source))?;
        merge_by_loop_id(&mut merged, overrides, &origin)?;
        let file = toml::Value::Table(merged)
            .try_into()
            .map_err(|source| parse_error(&origin, source))?;
        Self::validated(file, &origin)
    }

    /// Every registered loop, in registry order.
    #[must_use]
    pub fn loops(&self) -> &[LoopSpec] {
        &self.loops
    }

    /// The loop registered as `id`.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&LoopSpec> {
        self.loops.iter().find(|spec| spec.id.as_str() == id)
    }

    fn parse(text: &str, origin: &str) -> Result<Self, RegistryError> {
        let file = toml::from_str(text).map_err(|source| parse_error(origin, source))?;
        Self::validated(file, origin)
    }

    /// Check the schema, every entry, unique ids and each nested loop's
    /// parent.
    fn validated(file: RegistryFile, origin: &str) -> Result<Self, RegistryError> {
        if let Some(schema) = file.schema.as_deref() {
            check_schema(schema, origin)?;
        }
        let mut seen = HashSet::new();
        for spec in &file.loops {
            spec.check().map_err(|problem| invalid(&spec.id, problem))?;
            if !seen.insert(&spec.id) {
                return Err(invalid(&spec.id, "the id is registered twice"));
            }
            if let Some(parent) = &spec.nested_in {
                check_parent(spec, parent, &file.loops)
                    .map_err(|problem| invalid(&spec.id, problem))?;
            }
        }
        Ok(Self { loops: file.loops })
    }
}

/// Why a registry could not be loaded.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    /// Reading the workspace override failed.
    #[error("read {path}: {source}")]
    Io {
        /// The override file.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },
    /// The text is not TOML of the registry's shape.
    #[error("parse {origin}: {source}")]
    Parse {
        /// The registry text's origin.
        origin: String,
        /// Underlying parse error.
        source: toml::de::Error,
    },
    /// The file breaks a rule of its own: its schema, or the override's
    /// layout.
    #[error("{origin}: {problem}")]
    File {
        /// The registry text's origin.
        origin: String,
        /// The rule it breaks.
        problem: String,
    },
    /// An entry breaks a registration rule.
    #[error("loop {loop_id}: {problem}")]
    Invalid {
        /// The entry's id, as written.
        loop_id: String,
        /// The rule it breaks.
        problem: String,
    },
}

/// A registry file: `schema` and the `[[loop]]` entries.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistryFile {
    #[serde(default)]
    schema: Option<String>,
    #[serde(default, rename = "loop")]
    loops: Vec<LoopSpec>,
}

/// Merge the loops of `overrides` into `base` by `id`: an entry whose id is
/// registered replaces the keys it sets, and an entry with a new id is
/// appended.
fn merge_by_loop_id(
    base: &mut toml::Table,
    mut overrides: toml::Table,
    origin: &str,
) -> Result<(), RegistryError> {
    let file_error = |problem: &str| RegistryError::File {
        origin: origin.to_string(),
        problem: problem.to_string(),
    };
    if let Some(schema) = overrides.remove("schema") {
        check_schema(schema.as_str().unwrap_or_default(), origin)?;
    }
    let entries = match overrides.remove(LOOPS_KEY) {
        None => Vec::new(),
        Some(toml::Value::Array(entries)) => entries,
        Some(_) => return Err(file_error("`loop` is not an array of tables")),
    };
    if let Some(key) = overrides.keys().next() {
        return Err(file_error(format!("unknown key `{key}`").as_str()));
    }
    let loops = base
        .entry(LOOPS_KEY)
        .or_insert_with(|| toml::Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or_else(|| file_error("`loop` is not an array of tables"))?;
    for entry in entries {
        let toml::Value::Table(entry) = entry else {
            return Err(file_error("a `loop` entry is not a table"));
        };
        let Some(id) = loop_id_of(&entry).map(str::to_string) else {
            return Err(file_error("a `loop` entry has no `id`"));
        };
        match loops.iter_mut().find_map(|item| table_with_id(item, &id)) {
            Some(existing) => existing.extend(entry),
            None => loops.push(toml::Value::Table(entry)),
        }
    }
    Ok(())
}

/// The `id` of a `[[loop]]` entry.
fn loop_id_of(entry: &toml::Table) -> Option<&str> {
    entry.get("id").and_then(toml::Value::as_str)
}

/// `value` as a mutable table, when it is the entry of loop `id`.
fn table_with_id<'a>(value: &'a mut toml::Value, id: &str) -> Option<&'a mut toml::Table> {
    value
        .as_table_mut()
        .filter(|table| loop_id_of(table) == Some(id))
}

/// A nested loop's parent is registered, is not nested itself, and decides
/// on the same layer.
fn check_parent(spec: &LoopSpec, parent: &LoopId, loops: &[LoopSpec]) -> Result<(), String> {
    if *parent == spec.id {
        return Err("the loop is nested in itself".to_string());
    }
    let Some(parent_spec) = loops.iter().find(|other| other.id == *parent) else {
        return Err(format!("nested in {parent}, which is not registered"));
    };
    if parent_spec.nested_in.is_some() {
        return Err(format!("nested in {parent}, which is nested itself"));
    }
    if parent_spec.layer != spec.layer {
        return Err(format!(
            "nested in {parent}, which decides on layer `{}`, but declares layer `{}`",
            parent_spec.layer, spec.layer
        ));
    }
    Ok(())
}

fn check_schema(schema: &str, origin: &str) -> Result<(), RegistryError> {
    if schema == REGISTRY_SCHEMA {
        return Ok(());
    }
    Err(RegistryError::File {
        origin: origin.to_string(),
        problem: format!("schema `{schema}` is not `{REGISTRY_SCHEMA}`"),
    })
}

fn parse_error(origin: &str, source: toml::de::Error) -> RegistryError {
    RegistryError::Parse {
        origin: origin.to_string(),
        source,
    }
}

fn invalid(loop_id: &LoopId, problem: impl Into<String>) -> RegistryError {
    RegistryError::Invalid {
        loop_id: loop_id.to_string(),
        problem: problem.into(),
    }
}

/// A loop id's name: ASCII letters, digits and dashes.
const fn is_id_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-'
}

/// One dot-separated part of a layer name: lowercase ASCII letters, digits
/// and underscores.
fn is_layer_part(part: &str) -> bool {
    !part.is_empty() && part.bytes().all(is_layer_byte)
}

const fn is_layer_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
}

/// Whether `sha` is a commit id: 7 to 40 hex digits.
fn is_commit(sha: &str) -> bool {
    (7..=40).contains(&sha.len()) && sha.bytes().all(|byte| byte.is_ascii_hexdigit())
}

const fn default_version() -> u32 {
    1
}

const fn default_unit() -> AssignmentUnit {
    AssignmentUnit::Chain
}

fn default_outcome() -> String {
    DEFAULT_OUTCOME.to_string()
}

fn default_strata() -> Vec<String> {
    DEFAULT_STRATA.map(String::from).into()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        Lifecycle, Qualifier, REGISTRY_OVERRIDE_PATH, ReasonCode, ReceiptKind, Registry,
        RegistryError, StaticFinding,
    };

    /// One valid loop: the base the rejection tests break.
    const VALID: &str = r#"
schema = "roko.loop_registry/1"

[[loop]]
id = "L-test"
name = "test loop"
decision_point = "dispatch::test::decide"
layer = "route"
opportunity = "every dispatch"
default_policy = "default_slug"
receipt = "executed_model"

[[loop.static_findings]]
claim = "The contract type exists."
pointer = "crates/roko-learn/src/loop_audit/spec.rs::LoopSpec"
verified_at = "976220c3e"
"#;

    /// A loop nested in `L-test`, on its layer.
    const NESTED: &str = r#"
[[loop]]
id = "L-child"
name = "nested test loop"
decision_point = "dispatch::test::child"
layer = "route"
nested_in = "L-test"
opportunity = "the parent's learned arm"
default_policy = "the pick without the child"
receipt = "executed_model"
"#;

    /// The loop `text` names as invalid, and the rule it breaks.
    fn rejection(text: &str) -> (String, String) {
        match Registry::from_toml_str(text) {
            Err(RegistryError::Invalid { loop_id, problem }) => (loop_id, problem),
            other => panic!("expected an invalid loop, got {other:?}"),
        }
    }

    fn ids(registry: &Registry) -> Vec<&str> {
        registry
            .loops()
            .iter()
            .map(|spec| spec.id.as_str())
            .collect()
    }

    /// Whether `text` holds `word` between non-identifier characters.
    fn contains_word(text: &str, word: &str) -> bool {
        let is_ident = |c: char| c.is_alphanumeric() || c == '_';
        text.match_indices(word).any(|(at, _)| {
            let before = text[..at].chars().next_back();
            let after = text[at + word.len()..].chars().next();
            !before.is_some_and(is_ident) && !after.is_some_and(is_ident)
        })
    }

    #[test]
    fn registry_rejects_loop_without_default_policy() {
        let registry = Registry::from_toml_str(VALID).expect("the base loop is valid");
        let spec = registry.get("L-test").expect("L-test is registered");
        assert_eq!(spec.default_policy, "default_slug");
        assert_eq!((spec.version, spec.outcome.as_str()), (1, "U"));
        assert_eq!(spec.strata, ["task_type", "role", "tier"]);
        assert!(!spec.enforce && !spec.exempt && spec.lifecycle.is_randomized());

        // π⁰ missing or blank: either way the loop is rejected, by name.
        let missing = VALID.replace("default_policy = \"default_slug\"\n", "");
        let blank = VALID.replace("\"default_slug\"", "\"  \"");
        for text in [missing, blank] {
            let (loop_id, problem) = rejection(&text);
            assert_eq!(loop_id, "L-test");
            assert!(problem.contains("default_policy"), "{problem}");
        }

        // The other rules an entry checks on its own, one edit each.
        let edits = [
            ("\"every dispatch\"", "\"\"", "opportunity"),
            ("\"route\"", "\"\"", "layer"),
            ("\"route\"", "\"Route\"", "layer"),
            ("\"L-test\"", "\"test\"", "id"),
            ("\"976220c3e\"", "\"HEAD\"", "verified_at"),
            ("spec.rs::LoopSpec", "spec.rs", "path::symbol"),
            ("receipt", "fixed_holdout = 1.5\nreceipt", "fixed_holdout"),
        ];
        for (from, to, needle) in edits {
            let (_, problem) = rejection(&VALID.replace(from, to));
            assert!(problem.contains(needle), "{needle}: {problem}");
        }

        // A nested loop needs a registered parent on its own layer, and an id
        // is registered once.
        let edits = [
            ("nested_in = \"L-test\"", "nested_in = \"L-gone\"", "L-gone"),
            ("\"route\"", "\"knowledge\"", "layer"),
            ("\"L-child\"", "\"L-test\"", "twice"),
        ];
        for (from, to, needle) in edits {
            let text = format!("{VALID}{}", NESTED.replace(from, to));
            let (_, problem) = rejection(&text);
            assert!(problem.contains(needle), "{needle}: {problem}");
        }

        // The receipt and the schema are part of the file's shape.
        let no_receipt = VALID.replace("receipt = \"executed_model\"\n", "");
        assert!(matches!(
            Registry::from_toml_str(&no_receipt),
            Err(RegistryError::Parse { .. })
        ));
        let schema_2 = VALID.replace("roko.loop_registry/1", "roko.loop_registry/2");
        assert!(matches!(
            Registry::from_toml_str(&schema_2),
            Err(RegistryError::File { .. })
        ));

        // A nested loop decides on its parent's layer and draws on its own.
        let registry = Registry::from_toml_str(&format!("{VALID}{NESTED}")).expect("nested");
        let child = registry.get("L-child").expect("L-child is registered");
        assert_eq!(child.assignment_layer().as_str(), "route.child");
        let parent = registry.get("L-test").expect("L-test is registered");
        assert_eq!(parent.assignment_layer().as_str(), "route");
    }

    #[test]
    fn reason_code_precedence_picks_cheapest_first() {
        use ReasonCode::{
            Cut, Degenerate, Harm, Inert, LabelOnly, Mask, NoOpportunity, Null, Unlogged, WriteOnly,
        };

        // S03 §4.6's closed enum, cheapest check first.
        let serialized: Vec<String> = ReasonCode::ALL.iter().map(ToString::to_string).collect();
        assert_eq!(
            serialized,
            [
                "dormant:label_only",
                "dormant:degenerate",
                "dormant:no_opportunity",
                "dormant:no_learning",
                "dormant:write_only",
                "dormant:cut",
                "dormant:stale",
                "dormant:mask",
                "dormant:unlogged",
                "inert",
                "null",
                "harm",
            ]
        );
        for (rank, code) in ReasonCode::ALL.into_iter().enumerate() {
            assert_eq!(usize::from(code.precedence()), rank);
            assert_eq!(code.is_dormant(), rank < 9, "{code}");
            let json = serde_json::to_value(code).expect("serialize reason");
            assert_eq!(json, code.as_str());
            let back: ReasonCode = serde_json::from_value(json).expect("parse reason");
            assert_eq!(back, code);
        }

        // A write-only state with no Graph opportunity is no_opportunity.
        let winner = |codes: &[ReasonCode]| ReasonCode::cheapest(codes.iter().copied());
        assert_eq!(winner(&[WriteOnly, NoOpportunity]), Some(NoOpportunity));
        assert_eq!(winner(&[Unlogged, Mask]), Some(Mask));
        assert_eq!(winner(&[Harm, Inert, Cut]), Some(Cut));
        assert_eq!(winner(&[Harm, Null]), Some(Null));
        assert_eq!(winner(&[Degenerate, LabelOnly]), Some(LabelOnly));
        assert_eq!(winner(&[]), None);

        let qualifiers = [
            (Qualifier::Misspecified, "misspecified"),
            (Qualifier::PreInstrumentation, "pre_instrumentation"),
            (Qualifier::DeclaredStale, "declared_stale"),
        ];
        for (qualifier, name) in qualifiers {
            assert_eq!(qualifier.to_string(), name);
            assert_eq!(serde_json::to_value(qualifier).expect("qualifier"), name);
        }

        // A finding names the code it is evidence for by its string.
        const FINDING: &str = r#"
claim = "c"
pointer = "a/b.rs::f"
verified_at = "abcdef1"
reason = "dormant:write_only"
qualifier = "misspecified"
"#;
        let finding: StaticFinding = toml::from_str(FINDING).expect("parse finding");
        assert_eq!(finding.reason, Some(WriteOnly));
        assert_eq!(finding.qualifier, Some(Qualifier::Misspecified));
        assert_eq!(finding.pointer_parts(), Some(("a/b.rs", "f")));
    }

    #[test]
    fn override_merges_by_loop_id() {
        let workdir = tempfile::tempdir().expect("tempdir");
        let embedded = Registry::embedded().expect("embedded registry");
        let loaded = Registry::load(workdir.path()).expect("load without an override");
        assert_eq!(loaded, embedded);

        let path = workdir.path().join(REGISTRY_OVERRIDE_PATH);
        std::fs::create_dir_all(path.parent().expect("override dir")).expect("create dir");
        let overrides = r#"
schema = "roko.loop_registry/1"

[[loop]]
id = "L-route"
enforce = true
opportunity = "no pin and at least three eligible models"

[[loop]]
id = "L-custom"
name = "workspace loop"
decision_point = "custom::decide"
layer = "custom"
opportunity = "every dispatch"
default_policy = "no custom section"
receipt = "artifact_hash_in_request"
"#;
        std::fs::write(&path, overrides).expect("write override");
        let merged = Registry::load(workdir.path()).expect("load with an override");

        // The override replaces only the keys it sets.
        let route = merged.get("L-route").expect("L-route");
        let before = embedded.get("L-route").expect("embedded L-route");
        assert!(route.enforce && !before.enforce);
        assert_eq!(
            route.opportunity,
            "no pin and at least three eligible models"
        );
        assert_eq!(route.default_policy, before.default_policy);
        assert_eq!(route.layer, before.layer);
        assert_eq!(route.static_findings, before.static_findings);

        // Every embedded loop keeps its place, and a new id is appended.
        let (merged_ids, embedded_ids) = (ids(&merged), ids(&embedded));
        assert_eq!(merged_ids[..embedded_ids.len()], embedded_ids[..]);
        assert_eq!(merged_ids[embedded_ids.len()..], ["L-custom"]);
        let custom = merged.get("L-custom").expect("L-custom");
        assert_eq!(custom.layer.as_str(), "custom");

        // A broken override fails the load rather than being skipped.
        std::fs::write(&path, "[[loop]]\nid = \"L-route\"\nlayer = \"\"\n").expect("write");
        match Registry::load(workdir.path()) {
            Err(RegistryError::Invalid { loop_id, .. }) => assert_eq!(loop_id, "L-route"),
            other => panic!("expected L-route to be rejected, got {other:?}"),
        }
        std::fs::write(&path, "[[loop]]\nname = \"no id\"\n").expect("write");
        assert!(matches!(
            Registry::load(workdir.path()),
            Err(RegistryError::File { .. })
        ));
    }

    #[test]
    fn embedded_registry_declares_every_loop() {
        // S03 v1.1 §6 T1's loops, plus the retry-budget reader of the gate
        // thresholds (5103 option a).
        const EXPECTED: [&str; 21] = [
            "L-route",
            "L-dream-bias",
            "L-linucb",
            "L-model-exp",
            "L-routing-log",
            "L-know",
            "L-rag11",
            "L-play",
            "L-hdc",
            "L-err",
            "L-dream",
            "L-prompt-exp",
            "L-bid",
            "L-sec",
            "L-gate-thr",
            "L-retry-budget",
            "L-holdout",
            "L-M1",
            "L-M3",
            "L-M4",
            "L-placebo",
        ];
        let registry = Registry::embedded().expect("the embedded registry is valid");
        assert_eq!(ids(&registry), EXPECTED);
        let spec = |id: &str| registry.get(id).expect("registered loop");

        // Observe-only loops are measured but never randomized; the legacy
        // holdout and the retrieval A/A test are retired (4101, 4105), and
        // nothing enforces before C2, C4 and 14 days (D10).
        let observe_only: Vec<&str> = ids(&registry)
            .into_iter()
            .filter(|id| spec(id).lifecycle == Lifecycle::ObserveOnly)
            .collect();
        assert_eq!(
            observe_only,
            ["L-routing-log", "L-err", "L-gate-thr", "L-retry-budget"]
        );
        for loop_spec in registry.loops() {
            let id = loop_spec.id.as_str();
            let retired = matches!(loop_spec.lifecycle, Lifecycle::Retired { .. });
            assert_eq!(retired, matches!(id, "L-holdout" | "L-rag11"), "{id}");
            assert!(!loop_spec.enforce && !loop_spec.exempt, "{id}");
        }

        // L-M1's static 10% and the placebo's 0.5 replace the schedule.
        let fixed: Vec<(&str, f64)> = registry
            .loops()
            .iter()
            .filter_map(|loop_spec| Some((loop_spec.id.as_str(), loop_spec.fixed_holdout?)))
            .collect();
        assert_eq!(fixed, [("L-M1", 0.10), ("L-placebo", 0.5)]);

        // Nested loops draw only inside their parent's learned arm, and L-M3
        // rotates with L-route on the route layer.
        let nested: Vec<(&str, &str)> = registry
            .loops()
            .iter()
            .filter_map(|loop_spec| {
                let parent = loop_spec.nested_in.as_ref()?;
                Some((loop_spec.id.as_str(), parent.as_str()))
            })
            .collect();
        let expected = [
            ("L-dream-bias", "L-route"),
            ("L-linucb", "L-route"),
            ("L-rag11", "L-know"),
        ];
        assert_eq!(nested, expected);
        assert_eq!(spec("L-M3").layer, spec("L-route").layer);
        assert_eq!(
            spec("L-dream-bias").assignment_layer().as_str(),
            "route.dream_bias"
        );

        // Every receipt kind proves at least one loop's exposure.
        let receipts = [
            ReceiptKind::ExecutedModel,
            ReceiptKind::ArtifactHashInRequest,
            ReceiptKind::IncludedIds,
            ReceiptKind::ThresholdUsed,
            ReceiptKind::ParamsDigest,
            ReceiptKind::PredictionConsumed,
            ReceiptKind::AuditPenaltyApplied,
            ReceiptKind::Sham,
        ];
        for receipt in receipts {
            let used = registry
                .loops()
                .iter()
                .any(|loop_spec| loop_spec.receipt == receipt);
            assert!(used, "no loop proves exposure with {receipt:?}");
        }

        // Each finding points at a file under the workspace root that holds
        // its symbol, checked at a full commit sha.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for loop_spec in registry.loops() {
            let id = loop_spec.id.as_str();
            for finding in &loop_spec.static_findings {
                let (file, symbol) = finding.pointer_parts().expect("validated pointer");
                let text = std::fs::read_to_string(root.join(file))
                    .unwrap_or_else(|error| panic!("{id}: {file}: {error}"));
                for segment in symbol.split("::") {
                    assert!(
                        contains_word(&text, segment),
                        "{id}: `{segment}` is not in {file}"
                    );
                }
                assert_eq!(finding.verified_at.len(), 40, "{id}: {}", finding.pointer);
            }
        }

        // The facts 5103 re-verified at HEAD each appear with their pointer.
        let facts = [
            ("L-route", "model_routing.rs::ModelRouter::route"),
            ("L-route", "::ladder_choice"),
            ("L-dream-bias", "::cascade_pick"),
            ("L-prompt-exp", "prompt_experiment.rs::context"),
            ("L-prompt-exp", "::check_conclusion"),
            ("L-prompt-exp", "::RETRIEVAL_STRATEGY_EXPERIMENT_ID"),
            ("L-gate-thr", "retry_budget.rs::AdaptiveThresholds"),
            ("L-rag11", "retrieval_outcome.rs::STRATEGY_KEYWORD"),
            ("L-holdout", "holdout.rs::HoldoutExperiment::should_update_learning"),
            ("L-bid", "::load_attention_bidders"),
            ("L-bid", "::ATTENTION_BIDDERS_FILENAME"),
            ("L-err", "::ErrorPatternSink"),
        ];
        for (id, pointer) in facts {
            let findings = &spec(id).static_findings;
            let found = findings
                .iter()
                .any(|finding| finding.pointer.ends_with(pointer));
            assert!(found, "{id} has no finding at {pointer}");
        }
    }
}

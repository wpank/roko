//! The universal Signal type (defined as `Engram` for backward compatibility).
//!
//! A Signal is every event, every piece of data, every agent output, every
//! gate verdict in the Roko system. Signals are:
//!
//! - **Addressable** — content-hashed via BLAKE3
//! - **Decaying** — every signal has a decay function; weight fades over time
//! - **Scored** — multi-dimensional confidence/novelty/utility/reputation
//! - **Traced** — lineage tracks which signals this derived from
//! - **Composable** — signals combine into new signals via [`Compose`]s

use crate::{Attestation, Body, ContentHash, Decay, EmotionalTag, Kind, Provenance, Pulse, Score};
use roko_primitives::HdcVector;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ─── SignalStatus ─────────────────────────────────────────────────────────────

/// Lifecycle tier for a durable Signal.
///
/// Graduation is **monotonic** — signals can only move forward through tiers,
/// never backward. Each tier carries a different retention guarantee:
///
/// - [`Transient`](SignalStatus::Transient): may be pruned aggressively (minutes)
/// - [`Working`](SignalStatus::Working): retained during active task scope
/// - [`Consolidated`](SignalStatus::Consolidated): survives across sessions, feeds learning
/// - [`Persistent`](SignalStatus::Persistent): permanent archive, never auto-pruned
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalStatus {
    /// Default tier — subject to aggressive pruning.
    Transient,
    /// Retained for the duration of the active task scope.
    Working,
    /// Survives session boundaries; eligible to feed learning subsystems.
    Consolidated,
    /// Permanent archive; never auto-pruned.
    Persistent,
}

impl Default for SignalStatus {
    fn default() -> Self {
        Self::Transient
    }
}

impl SignalStatus {
    /// Returns `true` for tiers that survive beyond a single session.
    #[must_use]
    pub fn is_durable(self) -> bool {
        matches!(self, Self::Consolidated | Self::Persistent)
    }
}

// ─── GraduationError ─────────────────────────────────────────────────────────

/// Errors that can occur when attempting a Signal graduation transition.
#[derive(Clone, Debug, PartialEq)]
pub enum GraduationError {
    /// The requested transition is not a valid forward step.
    InvalidTransition {
        /// Current status before the attempted transition.
        from: SignalStatus,
        /// Requested target status.
        to: SignalStatus,
    },
    /// The signal's effective score is below the required threshold.
    ScoreTooLow {
        /// Minimum required effective score.
        required: f32,
        /// Actual effective score at transition time.
        actual: f32,
    },
    /// The signal has not been alive long enough to graduate.
    InsufficientAge {
        /// Required minimum age in seconds.
        required_secs: u64,
        /// Actual age at transition time in seconds.
        actual_secs: u64,
    },
    /// The signal has not been accessed enough times to graduate.
    InsufficientAccesses {
        /// Required minimum access count.
        required: u32,
        /// Actual access count at transition time.
        actual: u32,
    },
}

impl std::fmt::Display for GraduationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid graduation transition: {from:?} → {to:?}")
            }
            Self::ScoreTooLow { required, actual } => {
                write!(
                    f,
                    "score too low for graduation: required {required:.3}, got {actual:.3}"
                )
            }
            Self::InsufficientAge {
                required_secs,
                actual_secs,
            } => {
                write!(
                    f,
                    "insufficient age: required {required_secs}s, got {actual_secs}s"
                )
            }
            Self::InsufficientAccesses { required, actual } => {
                write!(
                    f,
                    "insufficient accesses: required {required}, got {actual}"
                )
            }
        }
    }
}

impl std::error::Error for GraduationError {}

/// Encoder version tag for the text-v1 HDC fingerprint encoder.
///
/// Text and JSON bodies are fingerprinted using [`HdcVector::from_seed`] with
/// this version recorded so future re-encoders can invalidate stale fingerprints.
pub const ENCODER_VERSION_TEXT_V1: u32 = 1;

/// HDC fingerprint metadata stored alongside a Signal.
///
/// The vector provides semantic similarity lookup, while `encoder_version`
/// records which deterministic encoder produced it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HdcFingerprint {
    /// The semantic fingerprint vector for this signal.
    pub vector: HdcVector,
    /// Monotonic version of the encoder used to derive `vector`.
    pub encoder_version: u32,
}

impl HdcFingerprint {
    /// Construct fingerprint metadata from a vector and encoder version.
    #[must_use]
    pub const fn new(vector: HdcVector, encoder_version: u32) -> Self {
        Self {
            vector,
            encoder_version,
        }
    }
}

/// The universal datum of the Roko system.
///
/// See [crate-level docs](crate) for the architectural role of Signal.
/// `Engram` is the canonical struct name; `Signal` is the preferred type alias.
///
/// # Identity
///
/// A signal's identity is its [`ContentHash`], computed from its kind, body,
/// author, and tags (see [`Engram::content_hash`]). Score and decay are
/// **excluded** from the hash — they can change without changing identity.
///
/// # Construction
///
/// Use [`Engram::builder`] (or equivalently `Signal::builder`) for ergonomic construction:
///
/// ```
/// use roko_core::{Body, Signal, Kind};
///
/// let s = Signal::builder(Kind::Task)
///     .body(Body::text("implement login"))
///     .tag("priority", "high")
///     .build();
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Engram {
    /// Content-addressed identity (computed from kind + body + author + tags).
    pub id: ContentHash,
    /// HDC fingerprint plus encoder metadata used for similarity and clustering.
    ///
    /// This remains optional so callers can construct engrams before a
    /// substrate has finalized fingerprinting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<HdcFingerprint>,
    /// What kind of engram this is.
    pub kind: Kind,
    /// The engram's payload.
    pub body: Body,
    /// Unix milliseconds when this engram was first emitted.
    pub created_at_ms: i64,
    /// How this engram's weight decays over time.
    pub decay: Decay,
    /// Producer attribution and trust.
    pub provenance: Provenance,
    /// Quality score at emission time (may be recomputed by scorers).
    pub score: Score,
    /// `ContentHash`es of engrams this derived from (forms a DAG for auditing
    /// and autocatalytic metrics).
    pub lineage: Vec<ContentHash>,
    /// Arbitrary string metadata (ordered for stable hashing).
    pub tags: BTreeMap<String, String>,
    /// Optional cryptographic proof of origin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attestation: Option<Attestation>,
    /// Optional emotional metadata associated with this engram.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emotional_tag: Option<EmotionalTag>,
    /// Demurrage balance in [0.0, 1.0]. Decays over time; refreshed on access.
    #[serde(default = "default_balance")]
    pub balance: f64,
    /// Lifecycle tier — tracks the graduation state of this signal.
    #[serde(default)]
    pub status: SignalStatus,
    /// Number of times this engram has been accessed (used for graduation checks).
    #[serde(default)]
    pub access_count: u32,
    /// Cumulative demurrage paid over this signal's lifetime (monotonically increasing).
    ///
    /// Tracks the total balance lost to demurrage ticks. Never decreases — even if
    /// novelty gains partially offset decay, this field only grows.
    #[serde(default)]
    pub demurrage_paid: f64,
}

/// Forward-compatible alias: `Signal` is the preferred name for [`Engram`].
pub type Signal = Engram;

impl Engram {
    /// Begin building a signal.
    #[must_use]
    pub fn builder(kind: Kind) -> EngramBuilder {
        EngramBuilder::new(kind)
    }

    /// Compute the content hash of this signal's identity fields.
    ///
    /// The hash covers: kind, body, author, taint, lineage, and tags.
    /// It does NOT cover: score, decay, timestamp, attestation, or emotional
    /// metadata — these can change without changing what the signal fundamentally is.
    #[must_use]
    pub fn content_hash(&self) -> ContentHash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.kind.identity_key().as_bytes());
        hasher.update(b"|");
        hasher.update(&self.body.canonical_bytes());
        hasher.update(b"|");
        hasher.update(self.provenance.author.as_bytes());
        hasher.update(b"|");
        hasher.update(&[u8::from(self.provenance.is_tainted())]);
        hasher.update(b"|");
        for h in &self.lineage {
            hasher.update(&h.0);
        }
        hasher.update(b"|");
        for (k, v) in &self.tags {
            hasher.update(k.as_bytes());
            hasher.update(b"=");
            hasher.update(v.as_bytes());
            hasher.update(b";");
        }
        ContentHash(*hasher.finalize().as_bytes())
    }

    /// The effective weight of this signal at the given current time.
    /// Combines score × decay.
    #[must_use]
    pub fn weight_at(&self, now_ms: i64) -> f32 {
        let age = now_ms - self.created_at_ms;
        self.score.effective() * self.decay.apply(age)
    }

    /// Age of this signal in milliseconds relative to a reference time.
    #[must_use]
    pub fn age_ms(&self, now_ms: i64) -> i64 {
        (now_ms - self.created_at_ms).max(0)
    }

    /// Reset the demurrage balance to full (1.0), as if freshly accessed.
    ///
    /// Also increments `access_count`, which is tracked for graduation checks.
    pub fn touch(&mut self) {
        self.balance = 1.0;
        self.access_count = self.access_count.saturating_add(1);
    }

    // ─── Graduation transitions ────────────────────────────────────────────

    /// Attempt to promote this signal from `Transient` to `Working`.
    ///
    /// Requires the signal's effective score to be >= `min_score`.
    ///
    /// # Errors
    ///
    /// Returns [`GraduationError::InvalidTransition`] if the current status is
    /// not `Transient`, or [`GraduationError::ScoreTooLow`] if the score is
    /// below the threshold.
    pub fn promote_to_working(&mut self, min_score: f32) -> Result<(), GraduationError> {
        if self.status != SignalStatus::Transient {
            return Err(GraduationError::InvalidTransition {
                from: self.status,
                to: SignalStatus::Working,
            });
        }
        let actual = self.score.effective();
        if actual < min_score {
            return Err(GraduationError::ScoreTooLow {
                required: min_score,
                actual,
            });
        }
        self.status = SignalStatus::Working;
        Ok(())
    }

    /// Attempt to promote this signal from `Working` to `Consolidated`.
    ///
    /// Requires the current status to be `Working`. Typically called after
    /// a gate pass, but the precondition check is the caller's responsibility.
    ///
    /// # Errors
    ///
    /// Returns [`GraduationError::InvalidTransition`] if the current status is
    /// not `Working`.
    pub fn promote_to_consolidated(&mut self) -> Result<(), GraduationError> {
        if self.status != SignalStatus::Working {
            return Err(GraduationError::InvalidTransition {
                from: self.status,
                to: SignalStatus::Consolidated,
            });
        }
        self.status = SignalStatus::Consolidated;
        Ok(())
    }

    /// Attempt to promote this signal from `Consolidated` to `Persistent`.
    ///
    /// Requires the current status to be `Consolidated`, the signal to have
    /// been alive for at least `min_age_secs`, and accessed at least
    /// `min_accesses` times.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`GraduationError`] variant if any precondition
    /// is not met. Preconditions are checked in order: status, age, accesses.
    pub fn promote_to_persistent(
        &mut self,
        min_age_secs: u64,
        min_accesses: u32,
    ) -> Result<(), GraduationError> {
        if self.status != SignalStatus::Consolidated {
            return Err(GraduationError::InvalidTransition {
                from: self.status,
                to: SignalStatus::Persistent,
            });
        }
        let now_ms = current_time_ms();
        let age_ms = (now_ms - self.created_at_ms).max(0) as u64;
        let actual_secs = age_ms / 1000;
        if actual_secs < min_age_secs {
            return Err(GraduationError::InsufficientAge {
                required_secs: min_age_secs,
                actual_secs,
            });
        }
        if self.access_count < min_accesses {
            return Err(GraduationError::InsufficientAccesses {
                required: min_accesses,
                actual: self.access_count,
            });
        }
        self.status = SignalStatus::Persistent;
        Ok(())
    }

    // ─── HDC fingerprinting ────────────────────────────────────────────────

    /// Compute and set an HDC fingerprint from this signal's body content.
    ///
    /// - [`Body::Text`] and [`Body::Json`] are encoded via [`HdcVector::from_seed`]
    ///   using the body's canonical byte representation as the seed.
    /// - [`Body::Empty`] produces a zero vector (marker signals have no content).
    /// - [`Body::Bytes`] is skipped — binary data requires specialized encoding.
    ///
    /// The encoder version is set to [`ENCODER_VERSION_TEXT_V1`].
    /// This method always overwrites any existing fingerprint.
    pub fn compute_fingerprint(&mut self) {
        let vector = match &self.body {
            Body::Text(s) => HdcVector::from_seed(s.as_bytes()),
            Body::Json(v) => HdcVector::from_seed(v.to_string().as_bytes()),
            Body::Empty => HdcVector::zeros(),
            Body::Bytes(_) => return, // binary data: skip
        };
        self.fingerprint = Some(HdcFingerprint::new(vector, ENCODER_VERSION_TEXT_V1));
    }

    /// Ensure this signal has an HDC fingerprint, computing one if absent.
    ///
    /// Idempotent — if `self.fingerprint` is already `Some`, this is a no-op.
    /// For [`Body::Bytes`], this is always a no-op (binary bodies are not fingerprinted).
    pub fn ensure_fingerprint(&mut self) {
        if self.fingerprint.is_none() {
            self.compute_fingerprint();
        }
    }

    // ─── Pulse projection ──────────────────────────────────────────────────

    /// Project this durable Signal into an ephemeral [`Pulse`] for Bus broadcast.
    ///
    /// This is the inverse of [`Pulse::graduate`] — it creates a lossy projection
    /// of a Signal back into the ephemeral transport layer. The projection intentionally
    /// drops durable-only metadata: score, balance, decay, fingerprint, attestation,
    /// and the full lineage DAG. Only the essential content (kind, body, tags) and a
    /// back-reference (`lineage_hint`) cross over.
    ///
    /// The originating Signal's id is preserved as the Pulse's `lineage_hint`, enabling
    /// consumers to trace a received Pulse back to its Signal if needed.
    ///
    /// # Arguments
    ///
    /// * `topic` - The Bus routing topic for the emitted Pulse
    /// * `seq` - The monotonic sequence number assigned by the Bus
    #[must_use]
    pub fn to_pulse(&self, topic: crate::pulse::Topic, seq: u64) -> Pulse {
        let mut builder = Pulse::builder(seq, topic, self.kind.clone())
            .body(self.body.clone())
            .lineage_hint(self.id)
            .tag("signal_author", self.provenance.author.clone());
        // Copy all signal tags onto the pulse for downstream filtering.
        for (key, value) in &self.tags {
            builder = builder.tag(key.clone(), value.clone());
        }
        builder.build()
    }

    /// Get a tag value by key.
    #[must_use]
    pub fn tag(&self, key: &str) -> Option<&str> {
        self.tags.get(key).map(String::as_str)
    }

    /// Check if this signal's kind matches the given kind.
    #[must_use]
    pub fn is(&self, kind: &Kind) -> bool {
        &self.kind == kind
    }

    /// Emit a derived signal — new kind/body, but tracks this signal as lineage.
    /// Useful when a gate/composer/policy produces a new signal from an input.
    pub fn derive(&self, kind: Kind, body: Body) -> EngramBuilder {
        EngramBuilder::new(kind)
            .body(body)
            .lineage([self.id])
            .provenance(Provenance::agent("derived"))
    }

    /// Emit a derived gate verdict signal with explicit verdict defaults.
    ///
    /// Unlike [`Engram::derive`], this preserves the parent's visible tag set,
    /// carries forward the full known lineage chain, and applies the
    /// [`Decay::GATE_VERDICT`] contract.
    pub fn derive_verdict(&self, body: Body) -> EngramBuilder {
        let mut builder = EngramBuilder::new(Kind::GateVerdict)
            .body(body)
            .decay(Decay::GATE_VERDICT)
            .lineage(self.derived_lineage())
            .provenance(Provenance::agent("derived"));

        for (key, value) in &self.tags {
            builder = builder.tag(key.clone(), value.clone());
        }

        builder
    }

    /// Promote a single [`Pulse`] to a synthetic [`Engram`].
    ///
    /// The resulting signal carries the pulse's kind, body, tags, and timestamp.
    /// Provenance is marked `"pulse_promotion"` and decay is `None`.
    #[must_use]
    pub fn from_pulse_synthetic(p: &Pulse) -> Self {
        let mut builder = EngramBuilder::new(p.kind.clone())
            .body(p.body.clone())
            .created_at_ms(p.created_at_ms)
            .provenance(Provenance::agent("pulse_promotion"));
        for (k, v) in &p.tags {
            builder = builder.tag(k.clone(), v.clone());
        }
        builder.build()
    }

    /// Combine multiple [`Pulse`]s into a single summary [`Engram`].
    ///
    /// Uses the first pulse's kind, concatenates text bodies (or collects
    /// JSON bodies into an array), and merges all tags. Useful for gate
    /// defaults that need to persist a batch of ephemeral events.
    #[must_use]
    pub fn from_pulses(pulses: &[Pulse]) -> Self {
        if pulses.is_empty() {
            return EngramBuilder::new(Kind::Episode)
                .provenance(Provenance::agent("pulse_batch"))
                .build();
        }

        let kind = pulses[0].kind.clone();
        let body = if pulses.len() == 1 {
            pulses[0].body.clone()
        } else {
            // Concatenate text bodies, or collect as JSON array.
            let texts: Vec<&str> = pulses
                .iter()
                .filter_map(|p| {
                    if let Body::Text(s) = &p.body {
                        Some(s.as_str())
                    } else {
                        None
                    }
                })
                .collect();
            if texts.len() == pulses.len() {
                Body::text(texts.join("\n"))
            } else {
                let values: Vec<serde_json::Value> = pulses
                    .iter()
                    .map(|p| serde_json::to_value(&p.body).unwrap_or_default())
                    .collect();
                Body::Json(serde_json::Value::Array(values))
            }
        };

        // Merge tags from all pulses (later values win on key collision).
        let mut tags = BTreeMap::new();
        for p in pulses {
            for (k, v) in &p.tags {
                tags.insert(k.clone(), v.clone());
            }
        }

        let earliest = pulses.iter().map(|p| p.created_at_ms).min().unwrap_or(0);

        let mut builder = EngramBuilder::new(kind)
            .body(body)
            .created_at_ms(earliest)
            .provenance(Provenance::agent("pulse_batch"));
        for (k, v) in tags {
            builder = builder.tag(k, v);
        }
        builder.build()
    }

    /// Bind this signal to another in HDC space when both fingerprints exist.
    #[must_use]
    pub fn bind(&self, other: &Engram) -> Option<HdcVector> {
        Some(self.fingerprint?.vector.bind(&other.fingerprint?.vector))
    }

    /// Bundle the fingerprints of several signals into one consensus vector.
    #[must_use]
    pub fn bundle(engrams: &[Engram]) -> Option<HdcVector> {
        let mut vectors = Vec::with_capacity(engrams.len());
        for engram in engrams {
            vectors.push(engram.fingerprint?.vector);
        }
        let refs = vectors.iter().collect::<Vec<_>>();
        Some(HdcVector::bundle(&refs))
    }

    /// Permute this signal's fingerprint into a positional binding slot.
    #[must_use]
    pub fn at_position(&self, position: usize) -> Option<HdcVector> {
        Some(self.fingerprint?.vector.permute(position))
    }

    fn derived_lineage(&self) -> Vec<ContentHash> {
        let mut lineage = Vec::with_capacity(self.lineage.len() + 1);
        for hash in self.lineage.iter().copied().chain(std::iter::once(self.id)) {
            if !lineage.contains(&hash) {
                lineage.push(hash);
            }
        }
        lineage
    }
}

// ─── Builder ───────────────────────────────────────────────────────────────

/// Ergonomic builder for Signals.
///
/// Fills in sensible defaults: current time, neutral score, no decay, trusted
/// roko provenance, empty lineage and tags.
pub struct EngramBuilder {
    kind: Kind,
    body: Body,
    created_at_ms: Option<i64>,
    decay: Decay,
    provenance: Provenance,
    score: Score,
    lineage: Vec<ContentHash>,
    tags: BTreeMap<String, String>,
    fingerprint: Option<HdcFingerprint>,
    attestation: Option<Attestation>,
    emotional_tag: Option<EmotionalTag>,
    balance: f64,
    status: SignalStatus,
    access_count: u32,
}

/// Forward-compatible alias: `SignalBuilder` is the preferred name for [`EngramBuilder`].
pub type SignalBuilder = EngramBuilder;

impl EngramBuilder {
    /// Start building a signal of the given kind.
    #[must_use]
    pub fn new(kind: Kind) -> Self {
        Self {
            kind,
            body: Body::empty(),
            created_at_ms: None,
            decay: Decay::None,
            provenance: Provenance::default(),
            score: Score::NEUTRAL,
            lineage: Vec::new(),
            tags: BTreeMap::new(),
            fingerprint: None,
            attestation: None,
            emotional_tag: None,
            balance: 1.0,
            status: SignalStatus::Transient,
            access_count: 0,
        }
    }

    /// Set the signal's body (payload).
    #[must_use]
    pub fn body(mut self, body: Body) -> Self {
        self.body = body;
        self
    }

    /// Set the signal's decay function.
    #[must_use]
    pub const fn decay(mut self, decay: Decay) -> Self {
        self.decay = decay;
        self
    }

    /// Set the signal's provenance (author + trust).
    #[must_use]
    pub fn provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = provenance;
        self
    }

    /// Set the signal's score.
    #[must_use]
    pub const fn score(mut self, score: Score) -> Self {
        self.score = score;
        self
    }

    /// Pin the signal's creation time (mostly useful for tests).
    #[must_use]
    pub const fn created_at_ms(mut self, t: i64) -> Self {
        self.created_at_ms = Some(t);
        self
    }

    /// Add content-hashes of parent signals to the lineage chain.
    #[must_use]
    pub fn lineage(mut self, hashes: impl IntoIterator<Item = ContentHash>) -> Self {
        self.lineage.extend(hashes);
        self
    }

    /// Set a string tag for filtering and indexing.
    #[must_use]
    pub fn tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    /// Stage fingerprint metadata for this signal.
    ///
    /// Most callers leave this unset and allow `Store::put()` to populate
    /// it using the active encoder registry.
    #[must_use]
    pub fn fingerprint(mut self, fingerprint: HdcFingerprint) -> Self {
        self.fingerprint = Some(fingerprint);
        self
    }

    /// Attach a cryptographic proof of origin.
    #[must_use]
    pub fn attestation(mut self, attestation: Attestation) -> Self {
        self.attestation = Some(attestation);
        self
    }

    /// Attach optional emotional metadata.
    #[must_use]
    pub fn emotional_tag(mut self, emotional_tag: EmotionalTag) -> Self {
        self.emotional_tag = Some(emotional_tag);
        self
    }

    /// Set the initial demurrage balance (defaults to 1.0).
    #[must_use]
    pub fn balance(mut self, balance: f64) -> Self {
        self.balance = balance;
        self
    }

    /// Set the initial graduation status (defaults to `Transient`).
    #[must_use]
    pub fn status(mut self, status: SignalStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the initial access count (defaults to 0).
    #[must_use]
    pub fn access_count(mut self, count: u32) -> Self {
        self.access_count = count;
        self
    }

    /// Finalize the signal, computing its content hash.
    #[must_use]
    pub fn build(self) -> Engram {
        let created_at_ms = self.created_at_ms.unwrap_or_else(current_time_ms);
        let mut engram = Engram {
            id: ContentHash([0; 32]), // placeholder
            fingerprint: self.fingerprint,
            kind: self.kind,
            body: self.body,
            created_at_ms,
            decay: self.decay,
            provenance: self.provenance,
            score: self.score,
            lineage: self.lineage,
            tags: self.tags,
            attestation: self.attestation,
            emotional_tag: self.emotional_tag,
            balance: self.balance,
            status: self.status,
            access_count: self.access_count,
            demurrage_paid: 0.0,
        };
        engram.id = engram.content_hash();
        engram
    }
}

/// Default demurrage balance for new or deserialized signals.
fn default_balance() -> f64 {
    1.0
}

/// Current Unix time in milliseconds.
fn current_time_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_defaults() {
        let s = Engram::builder(Kind::Task).build();
        assert_eq!(s.kind, Kind::Task);
        assert_eq!(s.body, Body::Empty);
        assert_eq!(s.decay, Decay::None);
        assert!(s.lineage.is_empty());
        assert!(s.tags.is_empty());
        assert!(s.fingerprint.is_none());
        assert!(s.attestation.is_none());
        assert!(s.emotional_tag.is_none());
        assert!((s.balance - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn content_hash_is_deterministic() {
        let a = Engram::builder(Kind::Task)
            .body(Body::text("same"))
            .created_at_ms(0)
            .build();
        let b = Engram::builder(Kind::Task)
            .body(Body::text("same"))
            .created_at_ms(0)
            .build();
        assert_eq!(a.id, b.id);
    }

    #[test]
    fn content_hash_ignores_score_and_decay() {
        // Two engrams with same identity fields but different score/decay
        // should have the SAME id (score/decay don't affect identity).
        let a = Engram::builder(Kind::Task)
            .body(Body::text("same"))
            .created_at_ms(0)
            .score(Score::new(0.1, 0.0, 0.0, 1.0))
            .decay(Decay::None)
            .build();
        let b = Engram::builder(Kind::Task)
            .body(Body::text("same"))
            .created_at_ms(0)
            .score(Score::new(0.9, 1.0, 5.0, 2.0))
            .decay(Decay::HalfLife { half_life_ms: 1000 })
            .build();
        assert_eq!(a.id, b.id);
    }

    #[test]
    fn content_hash_includes_body() {
        let a = Engram::builder(Kind::Task)
            .body(Body::text("a"))
            .created_at_ms(0)
            .build();
        let b = Engram::builder(Kind::Task)
            .body(Body::text("b"))
            .created_at_ms(0)
            .build();
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn content_hash_includes_tags() {
        let a = Engram::builder(Kind::Task)
            .body(Body::text("x"))
            .created_at_ms(0)
            .tag("priority", "high")
            .build();
        let b = Engram::builder(Kind::Task)
            .body(Body::text("x"))
            .created_at_ms(0)
            .tag("priority", "low")
            .build();
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn tag_order_does_not_affect_hash() {
        // BTreeMap stores keys in sorted order, so insertion order is irrelevant.
        let a = Engram::builder(Kind::Task)
            .body(Body::text("x"))
            .created_at_ms(0)
            .tag("a", "1")
            .tag("b", "2")
            .build();
        let b = Engram::builder(Kind::Task)
            .body(Body::text("x"))
            .created_at_ms(0)
            .tag("b", "2")
            .tag("a", "1")
            .build();
        assert_eq!(a.id, b.id);
    }

    #[test]
    fn weight_at_combines_score_and_decay() {
        let s = Engram::builder(Kind::Pheromone)
            .decay(Decay::HalfLife { half_life_ms: 1000 })
            .score(Score::new(1.0, 0.0, 0.0, 1.0)) // effective = 1.0
            .created_at_ms(0)
            .build();
        assert!((s.weight_at(0) - 1.0).abs() < 1e-6);
        assert!((s.weight_at(1000) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn derive_tracks_lineage() {
        let parent = Engram::builder(Kind::Task)
            .body(Body::text("parent"))
            .created_at_ms(0)
            .build();
        let child = parent.derive(Kind::GateVerdict, Body::text("pass")).build();
        assert_eq!(child.lineage, vec![parent.id]);
        assert_eq!(child.kind, Kind::GateVerdict);
    }

    #[test]
    fn derive_verdict_preserves_lineage_tags_and_decay() {
        let ancestor = Engram::builder(Kind::Prompt)
            .body(Body::text("ancestor"))
            .created_at_ms(0)
            .build();
        let parent = Engram::builder(Kind::Task)
            .body(Body::text("parent"))
            .created_at_ms(1)
            .lineage([ancestor.id])
            .tag("plan_id", "plan-42")
            .tag("gate", "compile")
            .build();

        let child = parent
            .derive_verdict(Body::text("pass"))
            .tag("passed", "true")
            .tag("gate", "test")
            .build();

        assert_eq!(child.kind, Kind::GateVerdict);
        assert_eq!(child.decay, Decay::GATE_VERDICT);
        assert_eq!(child.lineage, vec![ancestor.id, parent.id]);
        assert_eq!(child.tag("plan_id"), Some("plan-42"));
        assert_eq!(child.tag("passed"), Some("true"));
        assert_eq!(child.tag("gate"), Some("test"));
    }

    #[test]
    fn content_hash_ignores_attestation() {
        let base = Engram::builder(Kind::Task)
            .body(Body::text("same"))
            .created_at_ms(0)
            .build();
        let attested = Engram::builder(Kind::Task)
            .body(Body::text("same"))
            .created_at_ms(0)
            .attestation(Attestation {
                signature: crate::attestation::Ed25519Signature([7; 64]),
                public_key: crate::attestation::PublicKey([3; 32]),
                chain_attestation: Some(crate::attestation::ChainAttestation {
                    chain_id: 42,
                    tx_hash: [9; 32],
                    block_number: 99,
                }),
            })
            .build();
        assert_eq!(base.id, attested.id);
    }

    #[test]
    fn content_hash_ignores_emotional_tag() {
        let base = Engram::builder(Kind::Task)
            .body(Body::text("same"))
            .created_at_ms(0)
            .build();
        let tagged = Engram::builder(Kind::Task)
            .body(Body::text("same"))
            .created_at_ms(0)
            .emotional_tag(EmotionalTag::new(
                crate::PadVector::new(-0.2, 0.4, -0.1),
                0.6,
                "gate_failure",
                crate::PadVector::new(-0.2, 0.4, -0.1),
            ))
            .build();
        assert_eq!(base.id, tagged.id);
    }

    #[test]
    fn content_hash_ignores_fingerprint() {
        let base = Engram::builder(Kind::Task)
            .body(Body::text("same"))
            .created_at_ms(0)
            .build();
        let fingerprinted = Engram::builder(Kind::Task)
            .body(Body::text("same"))
            .created_at_ms(0)
            .fingerprint(HdcFingerprint::new(HdcVector::from_seed(b"same"), 3))
            .build();
        assert_eq!(base.id, fingerprinted.id);
    }

    #[test]
    fn serde_roundtrip() {
        let s = Engram::builder(Kind::Episode)
            .body(Body::text("an episode happened"))
            .decay(Decay::HalfLife {
                half_life_ms: 60_000,
            })
            .tag("run", "42")
            .fingerprint(HdcFingerprint::new(HdcVector::from_seed(b"episode"), 7))
            .build();
        let json = serde_json::to_string(&s).unwrap();
        let parsed: Engram = serde_json::from_str(&json).unwrap();
        assert_eq!(s, parsed);
    }

    #[test]
    fn tag_accessor() {
        let s = Engram::builder(Kind::Task).tag("foo", "bar").build();
        assert_eq!(s.tag("foo"), Some("bar"));
        assert_eq!(s.tag("missing"), None);
    }

    #[test]
    fn is_matches_kind() {
        let s = Engram::builder(Kind::GateVerdict).build();
        assert!(s.is(&Kind::GateVerdict));
        assert!(!s.is(&Kind::Task));
    }

    #[test]
    fn compound_kind_hash_distinguishes_components() {
        let a = Engram::builder(Kind::Compound(vec![Kind::Task, Kind::Prompt]))
            .body(Body::text("same"))
            .created_at_ms(0)
            .build();
        let b = Engram::builder(Kind::Compound(vec![Kind::Task, Kind::PromptSection]))
            .body(Body::text("same"))
            .created_at_ms(0)
            .build();
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn hdc_helpers_use_staged_fingerprints() {
        let left = Engram::builder(Kind::Task)
            .fingerprint(HdcFingerprint::new(HdcVector::from_seed(b"left"), 1))
            .build();
        let right = Engram::builder(Kind::Prompt)
            .fingerprint(HdcFingerprint::new(HdcVector::from_seed(b"right"), 1))
            .build();

        assert_eq!(
            left.bind(&right),
            Some(HdcVector::from_seed(b"left").bind(&HdcVector::from_seed(b"right")))
        );
        assert_eq!(
            Engram::bundle(&[left.clone(), right.clone()]),
            Some(HdcVector::bundle(&[
                &HdcVector::from_seed(b"left"),
                &HdcVector::from_seed(b"right"),
            ]))
        );
        assert_eq!(
            left.at_position(13),
            Some(HdcVector::from_seed(b"left").permute(13))
        );
    }

    #[test]
    fn builder_balance_defaults_to_one() {
        let s = Engram::builder(Kind::Task).build();
        assert!((s.balance - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn builder_balance_sets_custom_value() {
        let s = Engram::builder(Kind::Task).balance(0.42).build();
        assert!((s.balance - 0.42).abs() < f64::EPSILON);
    }

    #[test]
    fn content_hash_ignores_balance() {
        let a = Engram::builder(Kind::Task)
            .body(Body::text("same"))
            .created_at_ms(0)
            .balance(1.0)
            .build();
        let b = Engram::builder(Kind::Task)
            .body(Body::text("same"))
            .created_at_ms(0)
            .balance(0.3)
            .build();
        assert_eq!(a.id, b.id);
    }

    #[test]
    fn serde_defaults_missing_balance_to_one() {
        // Serialize an engram, strip the balance field, then deserialize.
        let s = Engram::builder(Kind::Task)
            .body(Body::text("hello"))
            .created_at_ms(0)
            .build();
        let mut json: serde_json::Value = serde_json::to_value(&s).unwrap();
        json.as_object_mut().unwrap().remove("balance");
        let parsed: Engram = serde_json::from_value(json).unwrap();
        assert!((parsed.balance - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn touch_resets_balance_to_one() {
        let mut s = Engram::builder(Kind::Task).balance(0.1).build();
        assert!((s.balance - 0.1).abs() < f64::EPSILON);
        s.touch();
        assert!((s.balance - 1.0).abs() < f64::EPSILON);
    }

    // ─── Additional coverage ────────────────────────────────────────────────

    #[test]
    fn builder_sets_all_fields() {
        let fp = HdcFingerprint::new(HdcVector::from_seed(b"test"), 5);
        let att = Attestation {
            signature: crate::attestation::Ed25519Signature([1; 64]),
            public_key: crate::attestation::PublicKey([2; 32]),
            chain_attestation: None,
        };
        let emo = EmotionalTag::new(
            crate::PadVector::new(0.3, -0.5, 0.1),
            0.8,
            "curiosity",
            crate::PadVector::new(0.3, -0.5, 0.1),
        );
        let prov = Provenance::agent("builder-test");
        let score = Score::new(0.9, 0.4, 2.0, 1.5);
        let parent_hash = ContentHash([42; 32]);

        let e = Engram::builder(Kind::Episode)
            .body(Body::text("payload"))
            .decay(Decay::Ttl { ttl_ms: 5000 })
            .provenance(prov.clone())
            .score(score)
            .created_at_ms(12345)
            .lineage([parent_hash])
            .tag("env", "test")
            .tag("run_id", "7")
            .fingerprint(fp)
            .attestation(att.clone())
            .emotional_tag(emo.clone())
            .balance(0.75)
            .build();

        assert_eq!(e.kind, Kind::Episode);
        assert_eq!(e.body, Body::text("payload"));
        assert_eq!(e.decay, Decay::Ttl { ttl_ms: 5000 });
        assert_eq!(e.provenance.author, "builder-test");
        assert_eq!(e.score, score);
        assert_eq!(e.created_at_ms, 12345);
        assert_eq!(e.lineage, vec![parent_hash]);
        assert_eq!(e.tag("env"), Some("test"));
        assert_eq!(e.tag("run_id"), Some("7"));
        assert_eq!(e.fingerprint, Some(fp));
        assert_eq!(e.attestation, Some(att));
        assert_eq!(e.emotional_tag, Some(emo));
        assert!((e.balance - 0.75).abs() < f64::EPSILON);
        // id should be the computed content hash, not the placeholder
        assert_ne!(e.id, ContentHash([0; 32]));
        assert_eq!(e.id, e.content_hash());
    }

    #[test]
    fn content_hash_deterministic_complex() {
        // Two engrams with identical identity fields (kind, body, provenance,
        // lineage, tags) but different non-identity fields (score, decay,
        // timestamp, attestation, emotional_tag, fingerprint, balance) must
        // produce the same content hash.
        let parent = ContentHash([99; 32]);
        let build = |score, decay, ts, balance| {
            Engram::builder(Kind::Prompt)
                .body(Body::text("complex payload"))
                .provenance(Provenance::agent("author-x"))
                .lineage([parent])
                .tag("k1", "v1")
                .tag("k2", "v2")
                .score(score)
                .decay(decay)
                .created_at_ms(ts)
                .balance(balance)
                .build()
        };

        let a = build(Score::new(0.1, 0.0, 0.0, 1.0), Decay::None, 100, 1.0);
        let b = build(
            Score::new(0.9, 1.0, 5.0, 3.0),
            Decay::HalfLife { half_life_ms: 999 },
            200,
            0.5,
        );
        assert_eq!(a.id, b.id);
    }

    #[test]
    fn content_hash_differs_on_kind() {
        let a = Engram::builder(Kind::Task)
            .body(Body::text("same"))
            .created_at_ms(0)
            .build();
        let b = Engram::builder(Kind::Prompt)
            .body(Body::text("same"))
            .created_at_ms(0)
            .build();
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn content_hash_differs_on_provenance_author() {
        let a = Engram::builder(Kind::Task)
            .body(Body::text("same"))
            .provenance(Provenance::agent("alice"))
            .created_at_ms(0)
            .build();
        let b = Engram::builder(Kind::Task)
            .body(Body::text("same"))
            .provenance(Provenance::agent("bob"))
            .created_at_ms(0)
            .build();
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn content_hash_differs_on_lineage() {
        let a = Engram::builder(Kind::Task)
            .body(Body::text("same"))
            .lineage([ContentHash([1; 32])])
            .created_at_ms(0)
            .build();
        let b = Engram::builder(Kind::Task)
            .body(Body::text("same"))
            .lineage([ContentHash([2; 32])])
            .created_at_ms(0)
            .build();
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn weight_at_creation_time_equals_effective_score() {
        let score = Score::new(0.8, 0.2, 1.0, 1.5);
        let e = Engram::builder(Kind::Task)
            .score(score)
            .decay(Decay::HalfLife { half_life_ms: 1000 })
            .created_at_ms(1000)
            .build();
        // At creation time (age = 0), decay multiplier = 1.0
        let w = e.weight_at(1000);
        assert!((w - score.effective()).abs() < 1e-6);
    }

    #[test]
    fn weight_at_with_no_decay() {
        let score = Score::new(0.6, 0.0, 0.0, 1.0);
        let e = Engram::builder(Kind::Task)
            .score(score)
            .decay(Decay::None)
            .created_at_ms(0)
            .build();
        // Decay::None always returns 1.0, so weight = score.effective() at any time.
        assert!((e.weight_at(0) - score.effective()).abs() < 1e-6);
        assert!((e.weight_at(1_000_000) - score.effective()).abs() < 1e-6);
    }

    #[test]
    fn weight_at_decays_over_two_half_lives() {
        let score = Score::new(1.0, 0.0, 0.0, 1.0); // effective = 1.0
        let e = Engram::builder(Kind::Task)
            .score(score)
            .decay(Decay::HalfLife { half_life_ms: 500 })
            .created_at_ms(0)
            .build();
        // After 2 half-lives (1000ms), weight = 1.0 * 0.25 = 0.25
        assert!((e.weight_at(1000) - 0.25).abs() < 1e-6);
        // After 3 half-lives (1500ms), weight = 1.0 * 0.125 = 0.125
        assert!((e.weight_at(1500) - 0.125).abs() < 1e-6);
    }

    #[test]
    fn weight_at_with_ttl_decay() {
        let score = Score::new(0.8, 0.0, 0.0, 1.0); // effective = 0.8
        let e = Engram::builder(Kind::Task)
            .score(score)
            .decay(Decay::Ttl { ttl_ms: 2000 })
            .created_at_ms(100)
            .build();
        // Before TTL expires: weight = score.effective() * 1.0
        assert!((e.weight_at(100) - score.effective()).abs() < 1e-6);
        assert!((e.weight_at(1500) - score.effective()).abs() < 1e-6);
        // After TTL expires: weight = score.effective() * 0.0 = 0.0
        assert!((e.weight_at(2200)).abs() < 1e-6);
    }

    #[test]
    fn serde_roundtrip_all_optional_fields() {
        let fp = HdcFingerprint::new(HdcVector::from_seed(b"rt"), 2);
        let att = Attestation {
            signature: crate::attestation::Ed25519Signature([11; 64]),
            public_key: crate::attestation::PublicKey([22; 32]),
            chain_attestation: Some(crate::attestation::ChainAttestation {
                chain_id: 1,
                tx_hash: [33; 32],
                block_number: 42,
            }),
        };
        let emo = EmotionalTag::new(
            crate::PadVector::new(-0.5, 0.8, 0.0),
            0.95,
            "frustration",
            crate::PadVector::new(-0.5, 0.8, 0.0),
        );
        let e = Engram::builder(Kind::GateVerdict)
            .body(Body::text("all fields"))
            .decay(Decay::Ebbinghaus {
                strength: 2.5,
                scale_ms: 10_000,
            })
            .score(Score::new(0.7, 0.3, 1.2, 0.9))
            .provenance(Provenance::agent("roundtrip-agent"))
            .created_at_ms(999_999)
            .lineage([ContentHash([77; 32]), ContentHash([88; 32])])
            .tag("gate", "compile")
            .tag("passed", "true")
            .fingerprint(fp)
            .attestation(att)
            .emotional_tag(emo)
            .balance(0.42)
            .build();

        let json = serde_json::to_string_pretty(&e).unwrap();
        let parsed: Engram = serde_json::from_str(&json).unwrap();
        assert_eq!(e, parsed);
        // Verify specific fields survived the roundtrip.
        assert_eq!(parsed.kind, Kind::GateVerdict);
        assert_eq!(parsed.created_at_ms, 999_999);
        assert_eq!(parsed.lineage.len(), 2);
        assert!(parsed.fingerprint.is_some());
        assert!(parsed.attestation.is_some());
        assert!(parsed.emotional_tag.is_some());
        assert!((parsed.balance - 0.42).abs() < f64::EPSILON);
    }

    // ─── SignalStatus / Graduation tests ────────────────────────────────────

    #[test]
    fn signal_status_default_is_transient() {
        let e = Engram::builder(Kind::Task).build();
        assert_eq!(e.status, SignalStatus::Transient);
    }

    #[test]
    fn signal_status_is_durable() {
        assert!(!SignalStatus::Transient.is_durable());
        assert!(!SignalStatus::Working.is_durable());
        assert!(SignalStatus::Consolidated.is_durable());
        assert!(SignalStatus::Persistent.is_durable());
    }

    #[test]
    fn access_count_default_is_zero() {
        let e = Engram::builder(Kind::Task).build();
        assert_eq!(e.access_count, 0);
    }

    #[test]
    fn touch_increments_access_count() {
        let mut e = Engram::builder(Kind::Task).balance(0.1).build();
        assert_eq!(e.access_count, 0);
        e.touch();
        assert_eq!(e.access_count, 1);
        e.touch();
        assert_eq!(e.access_count, 2);
        assert!((e.balance - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn promote_transient_to_working_succeeds_with_sufficient_score() {
        let mut e = Engram::builder(Kind::Task)
            .score(Score::new(0.8, 0.0, 0.0, 1.0))
            .build();
        assert_eq!(e.status, SignalStatus::Transient);
        e.promote_to_working(0.5).unwrap();
        assert_eq!(e.status, SignalStatus::Working);
    }

    #[test]
    fn promote_to_working_fails_when_score_too_low() {
        let mut e = Engram::builder(Kind::Task)
            .score(Score::new(0.2, 0.0, 0.0, 1.0))
            .build();
        let err = e.promote_to_working(0.5).unwrap_err();
        assert!(matches!(err, GraduationError::ScoreTooLow { .. }));
        assert_eq!(e.status, SignalStatus::Transient);
    }

    #[test]
    fn promote_to_working_fails_from_wrong_status() {
        let mut e = Engram::builder(Kind::Task)
            .status(SignalStatus::Working)
            .build();
        let err = e.promote_to_working(0.0).unwrap_err();
        assert!(matches!(
            err,
            GraduationError::InvalidTransition {
                from: SignalStatus::Working,
                to: SignalStatus::Working
            }
        ));
    }

    #[test]
    fn promote_working_to_consolidated_succeeds() {
        let mut e = Engram::builder(Kind::Task)
            .status(SignalStatus::Working)
            .build();
        e.promote_to_consolidated().unwrap();
        assert_eq!(e.status, SignalStatus::Consolidated);
    }

    #[test]
    fn promote_to_consolidated_fails_from_transient() {
        let mut e = Engram::builder(Kind::Task).build();
        let err = e.promote_to_consolidated().unwrap_err();
        assert!(matches!(
            err,
            GraduationError::InvalidTransition {
                from: SignalStatus::Transient,
                to: SignalStatus::Consolidated
            }
        ));
    }

    #[test]
    fn promote_consolidated_to_persistent_succeeds() {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut e = Engram::builder(Kind::Task)
            .status(SignalStatus::Consolidated)
            .created_at_ms(now_ms - 200_000)
            .access_count(5)
            .build();
        e.promote_to_persistent(100, 3).unwrap();
        assert_eq!(e.status, SignalStatus::Persistent);
    }

    #[test]
    fn promote_to_persistent_fails_insufficient_age() {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut e = Engram::builder(Kind::Task)
            .status(SignalStatus::Consolidated)
            .created_at_ms(now_ms - 5_000)
            .access_count(10)
            .build();
        let err = e.promote_to_persistent(60, 1).unwrap_err();
        assert!(matches!(err, GraduationError::InsufficientAge { .. }));
        assert_eq!(e.status, SignalStatus::Consolidated);
    }

    #[test]
    fn promote_to_persistent_fails_insufficient_accesses() {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut e = Engram::builder(Kind::Task)
            .status(SignalStatus::Consolidated)
            .created_at_ms(now_ms - 200_000)
            .access_count(1)
            .build();
        let err = e.promote_to_persistent(60, 5).unwrap_err();
        assert!(matches!(
            err,
            GraduationError::InsufficientAccesses {
                required: 5,
                actual: 1
            }
        ));
        assert_eq!(e.status, SignalStatus::Consolidated);
    }

    #[test]
    fn builder_status_setter() {
        let e = Engram::builder(Kind::Task)
            .status(SignalStatus::Consolidated)
            .build();
        assert_eq!(e.status, SignalStatus::Consolidated);
    }

    #[test]
    fn serde_roundtrip_with_status_and_access_count() {
        let e = Engram::builder(Kind::Task)
            .status(SignalStatus::Working)
            .access_count(7)
            .build();
        let json = serde_json::to_string(&e).unwrap();
        let parsed: Engram = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, SignalStatus::Working);
        assert_eq!(parsed.access_count, 7);
    }

    #[test]
    fn serde_defaults_missing_status_to_transient() {
        let e = Engram::builder(Kind::Task)
            .body(Body::text("old"))
            .created_at_ms(0)
            .build();
        let mut json: serde_json::Value = serde_json::to_value(&e).unwrap();
        json.as_object_mut().unwrap().remove("status");
        json.as_object_mut().unwrap().remove("access_count");
        let parsed: Engram = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.status, SignalStatus::Transient);
        assert_eq!(parsed.access_count, 0);
    }

    #[test]
    fn graduation_error_display() {
        let err = GraduationError::ScoreTooLow {
            required: 0.5,
            actual: 0.2,
        };
        let s = err.to_string();
        assert!(s.contains("score too low"));
    }

    // ─── compute_fingerprint / ensure_fingerprint ────────────────────────────

    #[test]
    fn compute_fingerprint_sets_fingerprint_for_text_body() {
        let mut e = Engram::builder(Kind::Task)
            .body(Body::text("hello world"))
            .build();
        assert!(e.fingerprint.is_none());
        e.compute_fingerprint();
        let fp = e
            .fingerprint
            .expect("fingerprint should be set after compute");
        assert_eq!(fp.encoder_version, ENCODER_VERSION_TEXT_V1);
        assert_eq!(fp.vector, HdcVector::from_seed(b"hello world"));
    }

    #[test]
    fn compute_fingerprint_sets_zero_vector_for_empty_body() {
        let mut e = Engram::builder(Kind::Task).body(Body::empty()).build();
        e.compute_fingerprint();
        let fp = e
            .fingerprint
            .expect("fingerprint should be set for Empty body");
        assert_eq!(fp.encoder_version, ENCODER_VERSION_TEXT_V1);
        assert_eq!(fp.vector, HdcVector::zeros());
    }

    #[test]
    fn compute_fingerprint_skips_bytes_body() {
        let mut e = Engram::builder(Kind::Task)
            .body(Body::bytes(vec![0xde, 0xad, 0xbe, 0xef]))
            .build();
        e.compute_fingerprint();
        assert!(e.fingerprint.is_none());
    }

    #[test]
    fn ensure_fingerprint_is_idempotent_for_text_body() {
        let mut e = Engram::builder(Kind::Task)
            .body(Body::text("idempotent"))
            .build();
        e.ensure_fingerprint();
        let fp1 = e.fingerprint.unwrap();
        e.ensure_fingerprint();
        let fp2 = e.fingerprint.unwrap();
        assert_eq!(fp1, fp2);
    }

    // ─── to_pulse tests ────────────────────────────────────────────────────

    #[test]
    fn to_pulse_preserves_kind_and_body() {
        let engram = Engram::builder(Kind::Task)
            .body(Body::text("task output"))
            .provenance(Provenance::agent("my-agent"))
            .created_at_ms(0)
            .build();

        let pulse = engram.to_pulse(crate::pulse::Topic::new("task.output"), 42);
        assert_eq!(pulse.seq, 42);
        assert_eq!(pulse.topic, crate::pulse::Topic::new("task.output"));
        assert_eq!(pulse.kind, Kind::Task);
        assert_eq!(pulse.body, Body::text("task output"));
    }

    #[test]
    fn to_pulse_sets_lineage_hint_to_signal_id() {
        let engram = Engram::builder(Kind::Episode)
            .body(Body::text("logged"))
            .created_at_ms(0)
            .build();

        let pulse = engram.to_pulse(crate::pulse::Topic::new("episode.logged"), 1);
        assert_eq!(pulse.lineage_hint, Some(engram.id));
    }

    #[test]
    fn to_pulse_copies_signal_tags() {
        let engram = Engram::builder(Kind::Task)
            .body(Body::text("x"))
            .tag("plan_id", "plan-7")
            .tag("gate", "compile")
            .created_at_ms(0)
            .build();

        let pulse = engram.to_pulse(crate::pulse::Topic::new("task"), 0);
        assert_eq!(pulse.tag("plan_id"), Some("plan-7"));
        assert_eq!(pulse.tag("gate"), Some("compile"));
    }
}

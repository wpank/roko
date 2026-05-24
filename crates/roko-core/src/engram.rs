//! The universal `Engram` type.
//!
//! An [`Engram`] is every event, every piece of data, every agent output, every
//! gate verdict in the Roko system. Engrams are:
//!
//! - **Addressable** — content-hashed via BLAKE3
//! - **Decaying** — every engram has a decay function; weight fades over time
//! - **Scored** — multi-dimensional confidence/novelty/utility/reputation
//! - **Traced** — lineage tracks which engrams this derived from
//! - **Composable** — engrams combine into new engrams via [`Compose`]s

use crate::{Attestation, Body, ContentHash, Decay, EmotionalTag, Kind, Provenance, Pulse, Score};
use roko_primitives::HdcVector;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// HDC fingerprint metadata stored alongside an [`Engram`].
///
/// The vector provides semantic similarity lookup, while `encoder_version`
/// records which deterministic encoder produced it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HdcFingerprint {
    /// The semantic fingerprint vector for this engram.
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
/// See [crate-level docs](crate) for the architectural role of Engram.
///
/// # Identity
///
/// An engram's identity is its [`ContentHash`], computed from its kind, body,
/// author, and tags (see [`Engram::content_hash`]). Score and decay are
/// **excluded** from the hash — they can change without changing identity.
///
/// # Construction
///
/// Use [`Engram::builder`] for ergonomic construction:
///
/// ```
/// use roko_core::{Body, Engram, Kind};
///
/// let s = Engram::builder(Kind::Task)
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
}

impl Engram {
    /// Begin building an engram.
    #[must_use]
    pub fn builder(kind: Kind) -> EngramBuilder {
        EngramBuilder::new(kind)
    }

    /// Compute the content hash of this engram's identity fields.
    ///
    /// The hash covers: kind, body, author, taint, lineage, and tags.
    /// It does NOT cover: score, decay, timestamp, attestation, or emotional
    /// metadata — these can change without changing what the engram fundamentally is.
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

    /// The effective weight of this engram at the given current time.
    /// Combines score × decay.
    #[must_use]
    pub fn weight_at(&self, now_ms: i64) -> f32 {
        let age = now_ms - self.created_at_ms;
        self.score.effective() * self.decay.apply(age)
    }

    /// Age of this engram in milliseconds relative to a reference time.
    #[must_use]
    pub fn age_ms(&self, now_ms: i64) -> i64 {
        (now_ms - self.created_at_ms).max(0)
    }

    /// Reset the demurrage balance to full (1.0), as if freshly accessed.
    pub fn touch(&mut self) {
        self.balance = 1.0;
    }

    /// Get a tag value by key.
    #[must_use]
    pub fn tag(&self, key: &str) -> Option<&str> {
        self.tags.get(key).map(String::as_str)
    }

    /// Check if this engram's kind matches the given kind.
    #[must_use]
    pub fn is(&self, kind: &Kind) -> bool {
        &self.kind == kind
    }

    /// Emit a derived engram — new kind/body, but tracks this engram as lineage.
    /// Useful when a gate/composer/policy produces a new engram from an input.
    pub fn derive(&self, kind: Kind, body: Body) -> EngramBuilder {
        EngramBuilder::new(kind)
            .body(body)
            .lineage([self.id])
            .provenance(Provenance::agent("derived"))
    }

    /// Emit a derived gate verdict engram with explicit verdict defaults.
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
    /// The resulting engram carries the pulse's kind, body, tags, and timestamp.
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

    /// Bind this engram to another in HDC space when both fingerprints exist.
    #[must_use]
    pub fn bind(&self, other: &Engram) -> Option<HdcVector> {
        Some(self.fingerprint?.vector.bind(&other.fingerprint?.vector))
    }

    /// Bundle the fingerprints of several engrams into one consensus vector.
    #[must_use]
    pub fn bundle(engrams: &[Engram]) -> Option<HdcVector> {
        let mut vectors = Vec::with_capacity(engrams.len());
        for engram in engrams {
            vectors.push(engram.fingerprint?.vector);
        }
        let refs = vectors.iter().collect::<Vec<_>>();
        Some(HdcVector::bundle(&refs))
    }

    /// Permute this engram's fingerprint into a positional binding slot.
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

/// Ergonomic builder for [`Engram`]s.
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
}

impl EngramBuilder {
    /// Start building an engram of the given kind.
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
        }
    }

    /// Set the engram's body (payload).
    #[must_use]
    pub fn body(mut self, body: Body) -> Self {
        self.body = body;
        self
    }

    /// Set the engram's decay function.
    #[must_use]
    pub const fn decay(mut self, decay: Decay) -> Self {
        self.decay = decay;
        self
    }

    /// Set the engram's provenance (author + trust).
    #[must_use]
    pub fn provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = provenance;
        self
    }

    /// Set the engram's score.
    #[must_use]
    pub const fn score(mut self, score: Score) -> Self {
        self.score = score;
        self
    }

    /// Pin the engram's creation time (mostly useful for tests).
    #[must_use]
    pub const fn created_at_ms(mut self, t: i64) -> Self {
        self.created_at_ms = Some(t);
        self
    }

    /// Add content-hashes of parent engrams to the lineage chain.
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

    /// Stage fingerprint metadata for this engram.
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

    /// Finalize the engram, computing its content hash.
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
        };
        engram.id = engram.content_hash();
        engram
    }
}

/// Default demurrage balance for new or deserialized engrams.
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
}

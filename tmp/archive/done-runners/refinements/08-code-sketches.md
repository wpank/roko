# Code Sketches

> **TL;DR**: Concrete Rust for the new `Bus` trait, the `Pulse` type,
> the `Datum` enum, the conversion methods, and a worked
> before/after migration of `roko-conductor` (the one with the doc-23
> layer violation).

> **For first-time readers**: This is the "show me the Rust" doc. The
> types shown here target `roko-core` (Phase B of the refactoring plan
> in 06). If you want the why, read 01–05 first. If you want the when,
> read 06 last.

## 1. `crates/roko-core/src/pulse.rs` (new)

```rust
//! Pulse — the ephemeral medium of Roko's transport fabric.
//!
//! A Pulse is an in-flight event traveling on a [`Bus`](crate::Bus). Pulses
//! are typed, sequence-numbered, and timestamped. Unlike [`Engram`]s, they
//! are not content-addressed, not persisted, and not scored. They deliver
//! once and live briefly in the Bus ring buffer.
//!
//! Pulses may graduate to Engrams via [`Pulse::graduate`] when their
//! lineage becomes forensically relevant (gate verdicts, process exits,
//! safety events). Pulses that don't graduate (heartbeats, UI refreshes,
//! token-chunk stream samples) vanish when the ring wraps.
//!
//! See `docs/00-architecture/02b-pulse-ephemeral-event.md` for the
//! conversion law and graduation policy.

use crate::{Body, ContentHash, Engram, EngramBuilder, Kind, Provenance, Decay, Score};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// An in-flight event on a Bus.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pulse {
    /// Topic-local monotonic sequence number. Unique within a
    /// (bus-instance, topic) pair.
    pub seq: u64,

    /// The topic this Pulse was published to.
    pub topic: Topic,

    /// Semantic category. Reuses [`Kind`] from the Engram taxonomy so
    /// that a Pulse and the Engram it may graduate into carry the
    /// same kind.
    pub kind: Kind,

    /// Payload. Reuses [`Body`] for the same reason.
    pub body: Body,

    /// Unix milliseconds when the Pulse was published.
    pub emitted_at_ms: i64,

    /// Lightweight origin attribution.
    pub source: PulseSource,

    /// Optional ContentHash of an Engram that contextualizes this
    /// Pulse. E.g. an `agent.msg.chunk` Pulse references the Task
    /// Engram its chunk belongs to.
    pub lineage_hint: Option<ContentHash>,

    /// Optional distributed-trace id.
    pub trace_id: Option<TraceId>,
}

/// Origin attribution for a Pulse.
///
/// Heavier than nothing, lighter than Engram's full `Provenance`.
/// Upgraded to full Provenance at graduation time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PulseSource {
    /// Component that published the Pulse ("roko-orchestrator",
    /// "roko-agent-server:claude-sonnet-4-6", …).
    pub component: String,
    /// Optional agent or session identifier.
    pub agent_id: Option<String>,
}

/// Trace id for distributed tracing (W3C traceparent-shaped).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceId(pub [u8; 16]);

impl Pulse {
    /// Graduate this Pulse into an Engram for Substrate storage.
    pub fn graduate(
        &self,
        provenance: Provenance,
        decay: Decay,
        score: Score,
        tags: BTreeMap<String, String>,
    ) -> Engram {
        let lineage = self.lineage_hint.clone().into_iter().collect();
        EngramBuilder::new(self.kind.clone(), self.body.clone())
            .created_at_ms(self.emitted_at_ms)
            .provenance(provenance)
            .decay(decay)
            .score(score)
            .lineage(lineage)
            .tags(tags)
            .build()
    }
}

/// A topic string. Canonical form is dot-separated lowercase,
/// e.g. "gate.verdict.emitted".
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Topic(pub String);

impl Topic {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn matches(&self, filter: &TopicFilter) -> bool {
        filter.matches(self)
    }
}

/// Declarative filter for Bus subscriptions.
#[derive(Clone, Debug)]
pub enum TopicFilter {
    Exact(Topic),
    Glob(String),
    AnyOf(Vec<Topic>),
    All,
    And(Box<TopicFilter>, Box<TopicFilter>),
    Or(Box<TopicFilter>, Box<TopicFilter>),
    Not(Box<TopicFilter>),
}

impl TopicFilter {
    pub fn matches(&self, topic: &Topic) -> bool {
        match self {
            TopicFilter::Exact(t) => topic == t,
            TopicFilter::Glob(pattern) => glob_match(pattern, topic.as_str()),
            TopicFilter::AnyOf(ts) => ts.iter().any(|t| t == topic),
            TopicFilter::All => true,
            TopicFilter::And(a, b) => a.matches(topic) && b.matches(topic),
            TopicFilter::Or(a, b) => a.matches(topic) || b.matches(topic),
            TopicFilter::Not(inner) => !inner.matches(topic),
        }
    }
}

/// Dot-aware wildcard matcher.
///
/// Rules:
/// - `pattern` and `topic` are split by `.` into segments.
/// - A literal segment matches only itself (case-sensitive).
/// - A `*` segment matches exactly one segment, any value.
/// - A `**` segment matches zero or more segments and must be the last
///   segment or followed by a literal segment.
///
/// Examples:
///   `agent.*`           matches `agent.msg` but NOT `agent.msg.chunk`
///   `agent.**`          matches `agent.msg`, `agent.msg.chunk`, `agent`
///   `agent.*.chunk`     matches `agent.msg.chunk` but NOT `agent.msg`
///   `gate.verdict.*`    matches `gate.verdict.emitted`
///   `**.tripped`        matches `conductor.circuit.tripped`
fn glob_match(pattern: &str, topic: &str) -> bool {
    glob_segments(
        &pattern.split('.').collect::<Vec<_>>(),
        &topic.split('.').collect::<Vec<_>>(),
    )
}

fn glob_segments(pat: &[&str], top: &[&str]) -> bool {
    match (pat.first(), top.first()) {
        (None, None) => true,
        (None, Some(_)) => false,
        (Some(&"**"), _) => {
            // ** matches zero or more segments; try each split.
            if pat.len() == 1 {
                return true;
            }
            // With a literal (or *) after, scan the topic looking for a
            // suffix that matches the rest of the pattern.
            for i in 0..=top.len() {
                if glob_segments(&pat[1..], &top[i..]) {
                    return true;
                }
            }
            false
        }
        (Some(&"*"), Some(_)) => glob_segments(&pat[1..], &top[1..]),
        (Some(pp), Some(tt)) if pp == tt => glob_segments(&pat[1..], &top[1..]),
        _ => false,
    }
}

#[cfg(test)]
mod glob_tests {
    use super::*;

    #[test]
    fn exact_match() {
        assert!(glob_match("agent.msg.chunk", "agent.msg.chunk"));
        assert!(!glob_match("agent.msg.chunk", "agent.msg"));
        assert!(!glob_match("agent.msg.chunk", "agent.msg.chunk.extra"));
    }

    #[test]
    fn single_star_matches_one_segment() {
        assert!(glob_match("agent.*", "agent.msg"));
        assert!(!glob_match("agent.*", "agent.msg.chunk"));
        assert!(!glob_match("agent.*", "agent"));
    }

    #[test]
    fn double_star_matches_zero_or_more() {
        assert!(glob_match("agent.**", "agent"));
        assert!(glob_match("agent.**", "agent.msg"));
        assert!(glob_match("agent.**", "agent.msg.chunk"));
        assert!(!glob_match("agent.**", "user.msg"));
    }

    #[test]
    fn middle_star() {
        assert!(glob_match("agent.*.chunk", "agent.msg.chunk"));
        assert!(!glob_match("agent.*.chunk", "agent.msg.body"));
    }

    #[test]
    fn trailing_literal_after_double_star() {
        assert!(glob_match("**.tripped", "conductor.circuit.tripped"));
        assert!(glob_match("**.tripped", "tripped"));
        assert!(!glob_match("**.tripped", "conductor.circuit.reset"));
    }
}
```

## 2. `crates/roko-core/src/traits.rs` — adding `Bus`

```rust
// ─── Bus ───────────────────────────────────────────────────────────────────

use crate::{Pulse, Topic, TopicFilter, error::Result};
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Transport fabric for [`Pulse`]s.
///
/// See crate docs for the two-medium / two-fabric model and
/// `docs/00-architecture/07b-bus-transport-fabric.md` for the full spec.
#[async_trait]
pub trait Bus: Send + Sync {
    /// Publish a Pulse. Returns its global sequence number.
    async fn publish(&self, pulse: Pulse) -> Result<u64>;

    /// Subscribe to Pulses matching `filter`. Returns a receiver.
    async fn subscribe(&self, filter: TopicFilter) -> Result<BusReceiver>;

    /// Replay Pulses newer than `since_seq` matching `filter`, up to
    /// the ring's retention window.
    async fn replay_since(
        &self,
        since_seq: u64,
        filter: &TopicFilter,
    ) -> Result<Vec<Pulse>>;

    /// Current global sequence number.
    async fn current_seq(&self) -> Result<u64>;

    /// Total Pulses published.
    async fn total_published(&self) -> Result<u64>;

    /// Current ring buffer occupancy.
    async fn ring_len(&self) -> Result<usize>;

    /// Ring buffer capacity.
    fn ring_capacity(&self) -> usize;

    fn name(&self) -> &'static str {
        "unnamed_bus"
    }
}

/// Receiver handle for a Bus subscription.
pub struct BusReceiver {
    pub(crate) inner: mpsc::Receiver<Pulse>,
    pub(crate) last_seq: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl BusReceiver {
    pub async fn recv(&mut self) -> Option<Pulse> {
        let p = self.inner.recv().await?;
        self.last_seq
            .store(p.seq, std::sync::atomic::Ordering::Relaxed);
        Some(p)
    }

    pub fn last_seq(&self) -> u64 {
        self.last_seq.load(std::sync::atomic::Ordering::Relaxed)
    }
}
```

## 3. `crates/roko-core/src/datum.rs` (new)

```rust
//! `Datum` — the either-medium reference used by polymorphic operators.

use crate::{Body, ContentHash, Engram, Kind, Pulse};
use std::collections::BTreeMap;

/// A reference to either medium.
#[derive(Clone, Copy, Debug)]
pub enum Datum<'a> {
    Engram(&'a Engram),
    Pulse(&'a Pulse),
}

impl<'a> Datum<'a> {
    pub fn kind(&self) -> &'a Kind {
        match self {
            Datum::Engram(e) => &e.kind,
            Datum::Pulse(p) => &p.kind,
        }
    }

    pub fn body(&self) -> &'a Body {
        match self {
            Datum::Engram(e) => &e.body,
            Datum::Pulse(p) => &p.body,
        }
    }

    pub fn created_at_ms(&self) -> i64 {
        match self {
            Datum::Engram(e) => e.created_at_ms,
            Datum::Pulse(p) => p.emitted_at_ms,
        }
    }

    pub fn tags(&self) -> Option<&'a BTreeMap<String, String>> {
        match self {
            Datum::Engram(e) => Some(&e.tags),
            Datum::Pulse(_) => None,
        }
    }

    pub fn content_hash(&self) -> Option<&'a ContentHash> {
        match self {
            Datum::Engram(e) => Some(&e.id),
            Datum::Pulse(p) => p.lineage_hint.as_ref(),
        }
    }
}

impl<'a> From<&'a Engram> for Datum<'a> {
    fn from(e: &'a Engram) -> Self {
        Datum::Engram(e)
    }
}

impl<'a> From<&'a Pulse> for Datum<'a> {
    fn from(p: &'a Pulse) -> Self {
        Datum::Pulse(p)
    }
}
```

## 4. `crates/roko-std/src/bus/broadcast.rs` (new)

```rust
//! `BroadcastBus` — in-process Bus backed by `tokio::sync::broadcast`.

use async_trait::async_trait;
use roko_core::{Bus, BusReceiver, Pulse, Topic, TopicFilter, error::Result};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use tokio::sync::{broadcast, mpsc, Mutex};
use std::collections::VecDeque;

pub struct BroadcastBus {
    tx: broadcast::Sender<Pulse>,
    seq: Arc<AtomicU64>,
    ring: Arc<Mutex<VecDeque<Pulse>>>,
    capacity: usize,
}

impl BroadcastBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity.max(16));
        Self {
            tx,
            seq: Arc::new(AtomicU64::new(0)),
            ring: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            capacity,
        }
    }
}

#[async_trait]
impl Bus for BroadcastBus {
    async fn publish(&self, mut pulse: Pulse) -> Result<u64> {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        pulse.seq = seq;
        {
            let mut ring = self.ring.lock().await;
            if ring.len() == self.capacity {
                ring.pop_front();
            }
            ring.push_back(pulse.clone());
        }
        // Lagging subscribers simply miss Pulses; that's the broadcast contract.
        let _ = self.tx.send(pulse);
        Ok(seq)
    }

    async fn subscribe(&self, filter: TopicFilter) -> Result<BusReceiver> {
        let mut rx = self.tx.subscribe();
        let (tx, mpsc_rx) = mpsc::channel(self.capacity);
        let last_seq = Arc::new(AtomicU64::new(0));
        let last_seq_cloned = last_seq.clone();
        tokio::spawn(async move {
            while let Ok(p) = rx.recv().await {
                if filter.matches(&p.topic) && tx.send(p).await.is_err() {
                    break;
                }
            }
        });
        Ok(BusReceiver::new(mpsc_rx, last_seq))
    }

    async fn replay_since(
        &self,
        since_seq: u64,
        filter: &TopicFilter,
    ) -> Result<Vec<Pulse>> {
        let ring = self.ring.lock().await;
        Ok(ring
            .iter()
            .filter(|p| p.seq > since_seq && filter.matches(&p.topic))
            .cloned()
            .collect())
    }

    async fn current_seq(&self) -> Result<u64> {
        Ok(self.seq.load(Ordering::SeqCst))
    }

    async fn total_published(&self) -> Result<u64> {
        Ok(self.seq.load(Ordering::SeqCst))
    }

    async fn ring_len(&self) -> Result<usize> {
        Ok(self.ring.lock().await.len())
    }

    fn ring_capacity(&self) -> usize {
        self.capacity
    }

    fn name(&self) -> &'static str {
        "broadcast_bus"
    }
}
```

## 5. Conductor port — before / after

### 5.1 Before (doc-23 layer violation)

```rust
// crates/roko-conductor/src/circuit.rs (simplified)
use roko_learn::gate_stats::GateFailureEma;   // <── L3 reaching into L2/cross-cut

pub struct CircuitBreaker {
    ema: GateFailureEma,
    threshold: f32,
}

impl CircuitBreaker {
    pub fn observe(&mut self, verdict: &Verdict) {
        self.ema.record(verdict.passed);
    }

    pub fn tripped(&self) -> bool {
        self.ema.rate() > self.threshold
    }
}
```

### 5.2 After (Bus-mediated, no cross-layer import)

```rust
// crates/roko-conductor/src/circuit.rs
use roko_core::{Bus, BusReceiver, Pulse, Topic, TopicFilter};

pub struct CircuitBreaker<B: Bus> {
    bus: std::sync::Arc<B>,
    threshold: f32,
    current_rate: std::sync::atomic::AtomicU32, // f32 bits
}

impl<B: Bus> CircuitBreaker<B> {
    pub async fn run(self: std::sync::Arc<Self>) {
        let mut rx = self
            .bus
            .subscribe(TopicFilter::Exact(Topic::new("gate.failure.rate")))
            .await
            .expect("bus subscribe");
        while let Some(pulse) = rx.recv().await {
            if let roko_core::Body::Json(v) = &pulse.body {
                if let Some(rate) = v.get("rate").and_then(|r| r.as_f64()) {
                    self.current_rate
                        .store((rate as f32).to_bits(), std::sync::atomic::Ordering::SeqCst);
                    if rate as f32 > self.threshold {
                        let _ = self
                            .bus
                            .publish(Pulse {
                                seq: 0, // filled by bus
                                topic: Topic::new("conductor.circuit.tripped"),
                                kind: roko_core::Kind::Custom("circuit.tripped".into()),
                                body: roko_core::Body::Json(serde_json::json!({
                                    "rate": rate,
                                    "threshold": self.threshold,
                                })),
                                emitted_at_ms: now_ms(),
                                source: roko_core::PulseSource {
                                    component: "roko-conductor".into(),
                                    agent_id: None,
                                },
                                lineage_hint: None,
                                trace_id: None,
                            })
                            .await;
                    }
                }
            }
        }
    }
}
```

And on the publisher side, `roko-learn` gains a policy that publishes
the rate:

```rust
// crates/roko-learn/src/policies/failure_rate.rs (new)
use roko_core::{Bus, Pulse, Topic};

pub struct FailureRatePolicy<B: Bus> {
    bus: std::sync::Arc<B>,
    window_ms: i64,
    samples: parking_lot::Mutex<Vec<(i64, bool)>>, // (ts, passed)
}

impl<B: Bus> FailureRatePolicy<B> {
    pub async fn run(self: std::sync::Arc<Self>) {
        let mut rx = self
            .bus
            .subscribe(roko_core::TopicFilter::Exact(Topic::new("gate.verdict.emitted")))
            .await
            .expect("bus subscribe");
        while let Some(pulse) = rx.recv().await {
            let passed = verdict_passed(&pulse);
            let now = pulse.emitted_at_ms;
            let rate = {
                let mut s = self.samples.lock();
                s.push((now, passed));
                s.retain(|(t, _)| now - t < self.window_ms);
                let fails = s.iter().filter(|(_, p)| !p).count() as f32;
                fails / (s.len() as f32).max(1.0)
            };
            let _ = self
                .bus
                .publish(Pulse {
                    seq: 0,
                    topic: Topic::new("gate.failure.rate"),
                    kind: roko_core::Kind::Metric,
                    body: roko_core::Body::Json(serde_json::json!({ "rate": rate })),
                    emitted_at_ms: now,
                    source: roko_core::PulseSource {
                        component: "roko-learn".into(),
                        agent_id: None,
                    },
                    lineage_hint: pulse.lineage_hint,
                    trace_id: pulse.trace_id,
                })
                .await;
        }
    }
}

fn verdict_passed(_pulse: &Pulse) -> bool {
    // extract from pulse.body json
    todo!()
}
```

`roko-conductor`'s `Cargo.toml` loses its `roko-learn` dependency.
Both crates now depend only on `roko-core`. Layer violation dissolved.

## 6. `PlanRevisionPolicy` — the self-hosting closure

```rust
// crates/roko-cli/src/plan_revision_policy.rs (new)
use roko_core::{Bus, Pulse, Topic, TopicFilter};

pub struct PlanRevisionPolicy<B: Bus> {
    bus: std::sync::Arc<B>,
    threshold: usize, // N consecutive failures
    failures: parking_lot::Mutex<std::collections::HashMap<String, usize>>,
}

impl<B: Bus> PlanRevisionPolicy<B> {
    pub async fn run(self: std::sync::Arc<Self>) {
        let mut rx = self
            .bus
            .subscribe(TopicFilter::Exact(Topic::new("gate.verdict.emitted")))
            .await
            .unwrap();
        while let Some(pulse) = rx.recv().await {
            let task_hash = task_hash_from(&pulse);
            let passed = verdict_passed(&pulse);
            let count = {
                let mut f = self.failures.lock();
                if passed {
                    f.remove(&task_hash);
                    continue;
                }
                let e = f.entry(task_hash.clone()).or_insert(0);
                *e += 1;
                *e
            };
            if count >= self.threshold {
                let _ = self
                    .bus
                    .publish(Pulse {
                        seq: 0,
                        topic: Topic::new("plan.revision.requested"),
                        kind: roko_core::Kind::Custom("plan.revision".into()),
                        body: roko_core::Body::Json(serde_json::json!({
                            "task_hash": task_hash,
                            "failure_count": count,
                            "last_verdict_pulse_seq": pulse.seq,
                        })),
                        emitted_at_ms: now_ms(),
                        source: roko_core::PulseSource {
                            component: "roko-cli:plan-revision".into(),
                            agent_id: None,
                        },
                        lineage_hint: pulse.lineage_hint.clone(),
                        trace_id: pulse.trace_id.clone(),
                    })
                    .await;
                self.failures.lock().remove(&task_hash);
            }
        }
    }
}
```

And the orchestrator subscribes to `plan.revision.requested` and
invokes `roko prd plan <slug>` with the failure context injected.
That's CLAUDE.md item 11 done in ~80 lines of policy code.

## 7. Test sketch — `Pulse::graduate` round trip

```rust
// crates/roko-core/tests/pulse_graduation.rs
use roko_core::{Pulse, Topic, Kind, Body, PulseSource, Provenance, Decay, Score};
use std::collections::BTreeMap;

#[test]
fn graduated_engram_has_stable_content_hash() {
    let p = Pulse {
        seq: 42,
        topic: Topic::new("gate.verdict.emitted"),
        kind: Kind::GateVerdict,
        body: Body::Json(serde_json::json!({"passed": true, "gate": "compile"})),
        emitted_at_ms: 1_700_000_000_000,
        source: PulseSource { component: "test".into(), agent_id: None },
        lineage_hint: None,
        trace_id: None,
    };
    let prov = Provenance::test_default("agent-a");
    let e1 = p.graduate(prov.clone(), Decay::None, Score::default(), BTreeMap::new());
    let e2 = p.graduate(prov, Decay::None, Score::default(), BTreeMap::new());
    assert_eq!(e1.id, e2.id, "graduation must be deterministic");
}

#[test]
fn graduation_preserves_kind_and_body() {
    // ...
}

#[test]
fn pulse_to_engram_to_pulse_roundtrip_preserves_kind() {
    // project Engram → Pulse, graduate → Engram, check kind/body equal
}
```

## 8. `PrdPublishPolicy` — the automatic plan generation closure

`CLAUDE.md` item 10 (trigger `prd plan` automatically when a PRD is
published) is the twin of §6's `PlanRevisionPolicy`. Same pattern:

```rust
// crates/roko-cli/src/prd_publish_policy.rs (new)
use roko_core::{Bus, Pulse, Topic, TopicFilter};

pub struct PrdPublishPolicy<B: Bus> {
    pub bus: std::sync::Arc<B>,
}

impl<B: Bus> PrdPublishPolicy<B> {
    pub async fn run(self: std::sync::Arc<Self>) {
        let mut rx = self
            .bus
            .subscribe(TopicFilter::Exact(Topic::new("prd.published")))
            .await
            .unwrap();

        while let Some(pulse) = rx.recv().await {
            let slug = match pulse.body.as_json().and_then(|v| v.get("slug")).and_then(|s| s.as_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let _ = self
                .bus
                .publish(Pulse {
                    seq: 0,
                    topic: Topic::new("plan.generation.requested"),
                    kind: roko_core::Kind::Custom("plan.generation".into()),
                    body: roko_core::Body::Json(serde_json::json!({
                        "slug": slug,
                        "source_pulse_seq": pulse.seq,
                    })),
                    emitted_at_ms: now_ms(),
                    source: roko_core::PulseSource {
                        component: "roko-cli:prd-publish".into(),
                        agent_id: None,
                    },
                    lineage_hint: pulse.lineage_hint,
                    trace_id: pulse.trace_id,
                })
                .await;
        }
    }
}
```

The orchestrator subscribes to `plan.generation.requested` and invokes
`roko prd plan <slug>` — closing CLAUDE.md item 10 in ~40 lines.

## 9. Additional tests to land in Phase B

Beyond §7's graduation round-trip, the Phase-B kernel tests should
cover:

```rust
// tests/topic_filter_boolean.rs
#[test]
fn and_or_not_combinations() {
    let agent_msgs = TopicFilter::Glob("agent.msg.*".into());
    let not_chunk = TopicFilter::Not(Box::new(TopicFilter::Exact(
        Topic::new("agent.msg.chunk"),
    )));
    let combined = TopicFilter::And(Box::new(agent_msgs), Box::new(not_chunk));
    assert!(combined.matches(&Topic::new("agent.msg.started")));
    assert!(!combined.matches(&Topic::new("agent.msg.chunk")));
}

// tests/broadcast_ring_wrap.rs
#[tokio::test]
async fn slow_subscriber_misses_pulses_after_ring_wrap() {
    let bus = BroadcastBus::new(4);
    let mut rx = bus.subscribe(TopicFilter::All).await.unwrap();
    for i in 0..6 {
        bus.publish(test_pulse(i)).await.unwrap();
    }
    let mut received = Vec::new();
    while let Ok(Some(p)) = tokio::time::timeout(
        std::time::Duration::from_millis(10),
        rx.recv(),
    ).await {
        received.push(p.seq);
    }
    // Broadcast is lossy; exact count depends on tokio scheduling.
    // Assert invariant: if any received, they are contiguous from the tail.
    if !received.is_empty() {
        for w in received.windows(2) {
            assert_eq!(w[1], w[0] + 1, "gap in broadcast delivery");
        }
    }
}

// tests/replay_since.rs
#[tokio::test]
async fn replay_returns_everything_newer_than_cursor() {
    let bus = BroadcastBus::new(100);
    for i in 0..50 {
        bus.publish(test_pulse_with_topic(i, if i % 2 == 0 { "a" } else { "b" }))
            .await.unwrap();
    }
    let got = bus
        .replay_since(20, &TopicFilter::Exact(Topic::new("a")))
        .await
        .unwrap();
    let expected: Vec<u64> = (21..50).filter(|i| i % 2 == 0).collect();
    let actual: Vec<u64> = got.iter().map(|p| p.seq).collect();
    assert_eq!(actual, expected);
}

// tests/lineage_hint_preserved.rs
#[tokio::test]
async fn graduated_engram_retains_lineage_from_pulse_hint() {
    let parent_hash = test_engram_hash("parent");
    let p = test_pulse_with_lineage(parent_hash.clone());
    let e = p.graduate(
        Provenance::test_default("test"),
        Decay::None,
        Score::default(),
        BTreeMap::new(),
    );
    assert_eq!(e.lineage.len(), 1);
    assert_eq!(e.lineage[0], parent_hash);
}
```

## 10. Engram → Pulse projection

The reverse direction (§3 of `02-engram-vs-pulse.md`) is a short impl:

```rust
impl Engram {
    /// Project this Engram as a Pulse for broadcast to live subscribers.
    /// Used e.g. by Substrate impls to publish `substrate.engram.stored`
    /// right after a successful put.
    pub fn to_pulse(
        &self,
        topic: Topic,
        seq: u64,
        source: PulseSource,
    ) -> Pulse {
        Pulse {
            seq,
            topic,
            kind: self.kind.clone(),
            body: self.body.clone(),
            emitted_at_ms: self.created_at_ms,
            source,
            lineage_hint: Some(self.id.clone()),
            trace_id: None,
        }
    }
}
```

The Substrate's standard bridge — "emit a Pulse for every successful
put" — wires this into the put method:

```rust
async fn put(&self, engram: Engram) -> Result<ContentHash> {
    let hash = engram.id.clone();
    // ... actual persistence work ...
    if let Some(bus) = &self.bus {
        let p = engram.to_pulse(
            Topic::new("substrate.engram.stored"),
            0,
            PulseSource {
                component: "substrate:file".into(),
                agent_id: None,
            },
        );
        let _ = bus.publish(p).await;   // best-effort — persistence already succeeded
    }
    Ok(hash)
}
```

The substrate-engram-stored topic is the bridge that lets Policy
decide over live Bus Pulses even when the event was produced by a
Substrate put (see `04-operators-generalized.md` §8).

All code here is sketch-level; the actual Phase-B implementation will
need `EngramBuilder` helpers, `Provenance::from_pulse_source`, and
Tokio test setup. But the shape is correct and composable from what's
already in `roko-core` and `roko-runtime`.

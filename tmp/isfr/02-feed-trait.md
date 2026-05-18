# 02: Feed Trait — ISFRFeed as Third Concrete Implementation

How ISFRFeed integrates with the Feed trait from taskrunner Wave 4 (task 097), becoming the third concrete Feed implementation alongside FileWatchFeed and ProviderHealthFeed.

## Relationship to Task 097

Task 097 defines the `Feed` trait in `crates/roko-core/src/feed.rs`:

```rust
#[async_trait]
pub trait Feed: Cell + Send + Sync {
    fn topic(&self) -> &Topic;
    fn feed_kind(&self) -> FeedKind;
    async fn start(&self, ctx: &CellContext) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    async fn poll(&self) -> Result<Option<Pulse>>;
    async fn status(&self) -> Result<FeedRuntimeStatus>;
}
```

Task 097 implements:
- `FileWatchFeed` — watches `.roko/` dir, publishes `fs.changed` Pulses
- `ProviderHealthFeed` — polls LLM providers, publishes `provider.health` Pulses

This plan adds:
- `ISFRFeed` — subscribes to relay `feed:isfr:rates`, publishes rate Pulses into the local Bus

### Why ISFRFeed Is Different

FileWatchFeed and ProviderHealthFeed are **local producers** — they generate data from local sources (filesystem, provider pings). ISFRFeed is a **relay consumer** — it receives data from the relay WebSocket and bridges it into the local Bus as Pulses.

This is the first Feed that connects the relay's topic pub/sub to roko's in-process PulseBus. It's the bridge that IMPL-06-ISFR.md's migration note refers to:

> *"Feeds will become Pulse streams on the Bus, managed via the Connect + Trigger protocols."*

## ISFRFeed Design

**File:** `crates/roko-core/src/feed.rs` (added alongside existing implementations)

```rust
/// Feed that bridges ISFR rate data from the relay into the local Bus.
///
/// Subscribes to relay topics `feed:isfr:rates` and `feed:isfr:ranges`,
/// converts incoming envelopes to Pulses, and publishes them on the
/// local Bus for consumption by agents, graduation policies, and the
/// SSE dashboard layer.
pub struct ISFRFeed {
    id: String,
    topic: Topic,
    relay_topics: Vec<String>,
    running: Arc<AtomicBool>,
    pulses_produced: Arc<AtomicU64>,
    last_update_ms: Arc<Mutex<Option<i64>>>,
    latest_pulse: Arc<Mutex<Option<Pulse>>>,
    relay_url: String,
}

impl ISFRFeed {
    pub fn new(relay_url: impl Into<String>, chain_id: impl Into<String>) -> Self {
        let chain_topic = format!("chain:{}", chain_id.into());
        Self {
            id: "isfr-feed".into(),
            topic: Topic::new("isfr.rates"),
            relay_topics: vec![
                "feed:isfr:rates".into(),
                "feed:isfr:ranges".into(),
                chain_topic,
            ],
            running: Arc::new(AtomicBool::new(false)),
            pulses_produced: Arc::new(AtomicU64::new(0)),
            last_update_ms: Arc::new(Mutex::new(None)),
            latest_pulse: Arc::new(Mutex::new(None)),
            relay_url: relay_url.into(),
        }
    }
}

impl Cell for ISFRFeed {
    fn cell_id(&self) -> &str { &self.id }
    fn cell_name(&self) -> &str { "ISFRFeed" }
    fn protocols(&self) -> &[&str] { &["Feed", "Connect"] }
}

#[async_trait]
impl Feed for ISFRFeed {
    fn topic(&self) -> &Topic { &self.topic }
    fn feed_kind(&self) -> FeedKind { FeedKind::Derived }

    async fn start(&self, _ctx: &CellContext) -> Result<()> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        let running = Arc::clone(&self.running);
        let pulses_produced = Arc::clone(&self.pulses_produced);
        let last_update_ms = Arc::clone(&self.last_update_ms);
        let latest_pulse = Arc::clone(&self.latest_pulse);
        let relay_url = self.relay_url.clone();
        let relay_topics = self.relay_topics.clone();

        tokio::spawn(async move {
            loop {
                if !running.load(Ordering::Relaxed) { break; }

                match connect_and_subscribe(&relay_url, &relay_topics).await {
                    Ok(mut ws) => {
                        while running.load(Ordering::Relaxed) {
                            match receive_envelope(&mut ws).await {
                                Ok(envelope) => {
                                    let pulse = envelope_to_pulse(
                                        &envelope,
                                        pulses_produced.fetch_add(1, Ordering::Relaxed),
                                    );
                                    let now_ms = chrono::Utc::now().timestamp_millis();
                                    *last_update_ms.lock() = Some(now_ms);
                                    *latest_pulse.lock() = Some(pulse);
                                    // When CellContext has a bus handle, publish here
                                }
                                Err(_) => break, // reconnect
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "ISFRFeed relay connect failed, retrying");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn poll(&self) -> Result<Option<Pulse>> {
        Ok(self.latest_pulse.lock().clone())
    }

    async fn status(&self) -> Result<FeedRuntimeStatus> {
        Ok(FeedRuntimeStatus {
            connected: self.running.load(Ordering::Relaxed),
            rate_hz: 0.1, // ~1 rate observation every 10s
            last_update_ms: *self.last_update_ms.lock(),
            error: None,
            pulses_produced: self.pulses_produced.load(Ordering::Relaxed),
        })
    }
}
```

Helper functions:

```rust
/// Connect to relay and subscribe to ISFR topics.
async fn connect_and_subscribe(
    relay_url: &str,
    topics: &[String],
) -> Result<WebSocketStream> {
    let (mut ws, _) = connect_async(relay_url).await?;
    // Send hello
    ws.send(json!({
        "type": "hello",
        "agent_id": "isfr-feed-bridge",
    }).to_string().into()).await?;
    // Subscribe
    ws.send(json!({
        "type": "subscribe",
        "topics": topics,
    }).to_string().into()).await?;
    Ok(ws)
}

/// Convert a relay envelope to a Pulse.
fn envelope_to_pulse(envelope: &Value, seq: u64) -> Pulse {
    let topic_str = envelope.get("topic")
        .and_then(|v| v.as_str())
        .unwrap_or("isfr.rates");

    // Map relay topics to Bus topics
    let bus_topic = match topic_str {
        "feed:isfr:rates" => "isfr.rates",
        "feed:isfr:ranges" => "isfr.ranges",
        t if t.starts_with("chain:") => "isfr.chain_event",
        _ => "isfr.unknown",
    };

    Pulse::new(
        seq,
        Topic::new(bus_topic),
        Kind::Event, // or a new Kind::FeedData
        Body::json(envelope.get("payload").cloned().unwrap_or(Value::Null)),
    )
}
```

## Where ISFRFeed Lives

**Option A (recommended): In roko-core alongside other feeds**

```
crates/roko-core/src/feed.rs
  ├── FeedRegistry (existing metadata CRUD)
  ├── Feed trait (task 097)
  ├── FeedRuntimeStatus (task 097)
  ├── FileWatchFeed (task 097)
  ├── ProviderHealthFeed (task 097)
  └── ISFRFeed (this plan)        ← NEW
```

Pros: All Feed implementations in one place. ISFRFeed follows the exact same pattern.
Cons: roko-core gains a dependency on the relay WebSocket client (tokio-tungstenite).

**Option B: In a separate module**

```
crates/roko-chain/src/isfr_feed.rs   ← NEW
```

Pros: Keeps relay-dependent code in the chain crate.
Cons: Splits Feed implementations across crates.

**Recommendation:** Option A if roko-core already depends on tokio-tungstenite (it does via relay client). Option B if we want stricter crate boundaries.

## Generalization: RelayFeed Base

ISFRFeed is the first relay-backed feed, but it won't be the last. Extract a reusable base:

```rust
/// Generic feed that bridges relay topics into the local Bus.
///
/// Concrete feeds configure which relay topics to subscribe to
/// and how to convert envelopes to Pulses.
pub struct RelayFeed<C: RelayFeedCodec> {
    id: String,
    topic: Topic,
    relay_topics: Vec<String>,
    codec: C,
    running: Arc<AtomicBool>,
    pulses_produced: Arc<AtomicU64>,
    last_update_ms: Arc<Mutex<Option<i64>>>,
    latest_pulse: Arc<Mutex<Option<Pulse>>>,
    relay_url: String,
}

/// How to convert relay envelopes to Pulses.
pub trait RelayFeedCodec: Send + Sync + 'static {
    /// Convert an incoming relay envelope to a Pulse.
    fn decode(&self, envelope: &Value, seq: u64) -> Option<Pulse>;

    /// The Bus topic this feed publishes to.
    fn bus_topic(&self) -> &Topic;

    /// The feed kind classification.
    fn feed_kind(&self) -> FeedKind;
}
```

Then ISFRFeed becomes:

```rust
pub struct ISFRCodec;

impl RelayFeedCodec for ISFRCodec {
    fn decode(&self, envelope: &Value, seq: u64) -> Option<Pulse> {
        let relay_topic = envelope.get("topic")?.as_str()?;
        let bus_topic = match relay_topic {
            "feed:isfr:rates" => "isfr.rates",
            "feed:isfr:ranges" => "isfr.ranges",
            t if t.starts_with("chain:") => "isfr.chain_event",
            _ => return None,
        };
        Some(Pulse::new(
            seq,
            Topic::new(bus_topic),
            Kind::Event,
            Body::json(envelope.get("payload").cloned()?),
        ))
    }

    fn bus_topic(&self) -> &Topic { &Topic::new("isfr.rates") }
    fn feed_kind(&self) -> FeedKind { FeedKind::Derived }
}

/// The ISFR feed — relay-backed, subscribes to rate + range + chain topics.
pub type ISFRFeed = RelayFeed<ISFRCodec>;

impl ISFRFeed {
    /// Create an ISFRFeed for a specific chain.
    /// `chain_id` determines the chain event topic: `chain:{chain_id}`.
    /// For mirage-rs use `"mirage"`, for daeji use `"daeji"`, etc.
    pub fn new(relay_url: impl Into<String>, chain_id: &str) -> Self {
        RelayFeed::new(
            "isfr-feed",
            vec![
                "feed:isfr:rates".into(),
                "feed:isfr:ranges".into(),
                format!("chain:{chain_id}"),
            ],
            ISFRCodec,
            relay_url,
        )
    }
}
```

**This makes adding new relay-backed feeds trivial:** define a codec, specify topics, done. Future feeds (price feeds, gas feeds, liquidation alerts) follow the same pattern.

## Integration with Task 098 (Feed CLI + Engine)

Task 098 wires feeds into the Engine and SSE layer. ISFRFeed integrates the same way:

```rust
// In state.rs (ServeFeeds struct):
pub struct ServeFeeds {
    pub file_watch: Arc<FileWatchFeed>,
    pub provider_health: Arc<ProviderHealthFeed>,
    pub isfr: Arc<ISFRFeed>,  // ← NEW
}
```

CLI shows it in `roko feed list`:

```
ID                       TOPIC                            KIND       CONNECTED
--------------------------------------------------------------------------------
file-watch-roko-dir      fs.changed                       Raw        yes
provider-health-feed     provider.health                  Meta       yes
isfr-feed                isfr.rates                       Derived    yes
```

## Integration with Task 099 (Pulse Graduation)

ISFR rate Pulses can graduate to Signals for persistence:

```toml
# roko.toml
[[graduation.policies]]
watch = { Prefix = "isfr.rates" }
always = true

[[graduation.policies]]
watch = { Prefix = "isfr.ranges" }
sample_every = 10   # Graduate every 10th range coordination message
```

This means rate observations are automatically persisted in the Store, creating a historical record of ISFR rates that agents can query.

## Testing

```bash
# Unit test: ISFRCodec converts envelopes correctly
cargo test -p roko-core -- isfr_feed_codec

# Unit test: ISFRFeed starts and stops
cargo test -p roko-core -- isfr_feed_lifecycle

# Integration test: ISFRFeed receives from relay
# (requires relay running — integration test only)
cargo test -p roko-core -- isfr_feed_integration --ignored
```

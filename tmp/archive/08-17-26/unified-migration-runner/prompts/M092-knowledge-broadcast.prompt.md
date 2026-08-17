# M092 — Knowledge Signal Broadcast via Relay

## Objective
Implement knowledge Signal broadcast: when a Signal reaches Persistent tier with high confidence, optionally broadcast its HDC fingerprint and summary via the workspace relay (WebSocket or Iroh P2P). Receiving agents can request the full Signal if its similarity to their active context exceeds a threshold. This enables cross-agent knowledge sharing within a workspace.

## Scope
- Crates: `roko-runtime`
- Files: `crates/roko-runtime/src/knowledge_sync.rs` (new), `crates/roko-runtime/src/lib.rs`
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.7
- Spec ref: `tmp/unified/12-CONNECTIVITY.md` SS3 (Relay)

## Steps
1. Check existing relay or P2P code:
   ```bash
   grep -rn 'relay\|Relay\|broadcast\|peer\|P2P\|iroh' crates/roko-runtime/src/ --include='*.rs' | head -15
   grep -rn 'WebSocket\|ws\|websocket' crates/roko-runtime/src/ --include='*.rs' | head -10
   ```

2. Implement the knowledge broadcast system in `crates/roko-runtime/src/knowledge_sync.rs`:
   ```rust
   pub struct KnowledgeBroadcaster {
       bus: Arc<dyn Bus>,
       relay: Arc<dyn Relay>,
       similarity_threshold: f64,
       config: BroadcastConfig,
   }

   pub struct BroadcastConfig {
       pub enabled: bool,
       pub min_tier: String,
       pub min_confidence: f64,
       pub similarity_threshold: f64,
   }

   pub struct KnowledgeAnnouncement {
       pub signal_id: String,
       pub hdc_fingerprint: Vec<f32>,
       pub summary: String,
       pub tier: String,
       pub confidence: f64,
       pub source_agent: String,
   }
   ```

3. Implement the broadcast flow:
   ```rust
   impl KnowledgeBroadcaster {
       /// Subscribe to tier promotions and broadcast qualifying Signals.
       pub async fn run(&self, cancel: CancellationToken) {
           // 1. Subscribe to "signal.promoted.persistent" topic on Bus
           // 2. For each qualifying Signal:
           //    a. Compute HDC fingerprint + summary
           //    b. Publish KnowledgeAnnouncement on relay
       }

       /// Handle incoming announcements from other agents.
       pub async fn handle_announcement(&self, announcement: KnowledgeAnnouncement) -> Option<RequestFullSignal> {
           // 1. Compute similarity between announcement fingerprint and active context
           // 2. If similarity > threshold: request full Signal
           // 3. If received: import with decay factor (via M090)
       }
   }
   ```

4. Define the Relay trait (abstract over WebSocket and future Iroh P2P):
   ```rust
   #[async_trait]
   pub trait Relay: Send + Sync {
       async fn broadcast(&self, announcement: KnowledgeAnnouncement) -> Result<()>;
       async fn subscribe(&self) -> Result<Receiver<KnowledgeAnnouncement>>;
       async fn request_signal(&self, signal_id: &str, from: &str) -> Result<Signal>;
   }
   ```

5. Write tests:
   - Agent A publishes knowledge -> Agent B receives fingerprint -> requests full Signal -> imports into local store
   - Announcement below similarity threshold is ignored
   - Broadcast is skipped when disabled in config

## Verification
```bash
cargo check -p roko-runtime
cargo clippy -p roko-runtime --no-deps -- -D warnings
cargo test -p roko-runtime -- knowledge_sync
```

## What NOT to do
- Do NOT implement the relay transport here -- define the trait, use a mock for testing
- Do NOT broadcast full Signal content -- only fingerprint + summary
- Do NOT enable broadcast by default -- it must be opt-in
- Do NOT add authentication to the relay -- that is a separate concern

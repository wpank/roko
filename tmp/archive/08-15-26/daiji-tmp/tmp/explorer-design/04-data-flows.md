# Data Flows — Chain Data → Visual Elements

How chain data enters the system, transforms, and drives visuals.

---

## Architecture

```
                    ┌─────────────────────────┐
                    │    kora RPC / WS         │
                    │    65.109.61.210:8545    │
                    └──────────┬──────────────┘
                               │
                    ┌──────────▼──────────────┐
                    │    DataLayer             │
                    │                          │
                    │  ┌─ Poller ────────────┐ │    ← Phase 1: polling
                    │  │ eth_blockNumber 1/s │ │
                    │  │ eth_feeHistory  5/s │ │
                    │  └────────────────────┘ │
                    │                          │
                    │  ┌─ Subscriber ────────┐ │    ← Phase 2: WS push
                    │  │ newHeads            │ │
                    │  │ logs               │ │
                    │  │ pendingTransactions │ │
                    │  └────────────────────┘ │
                    │                          │
                    │  ┌─ Cache ─────────────┐ │
                    │  │ blockMap (LRU 500)  │ │
                    │  │ addressSet          │ │
                    │  │ feeHistory (ring)   │ │
                    │  └────────────────────┘ │
                    └──────────┬──────────────┘
                               │
                    ┌──────────▼──────────────┐
                    │    EventBus              │
                    │                          │
                    │  "block:new"    → Block  │
                    │  "tx:confirmed" → Tx     │
                    │  "tx:pending"   → Tx     │
                    │  "fee:update"   → Fee[]  │
                    │  "address:seen" → Addr   │
                    │  "log:new"      → Log    │
                    └──────────┬──────────────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
     ┌────────▼───────┐ ┌─────▼──────┐ ┌───────▼────────┐
     │  SceneManager  │ │ PanelState │ │ AudioEngine    │
     │                │ │            │ │ (optional)     │
     │  terrain       │ │ block info │ │ tick on block  │
     │  constellation │ │ gas chart  │ │ tone on tx     │
     │  waterfall     │ │ status     │ │ ambient drone  │
     │  consensus     │ │ search     │ │                │
     └────────────────┘ └────────────┘ └────────────────┘
```

---

## Phase 1: Polling (Works Now)

Minimal data layer using only current RPC capabilities.

### Poll Loop

```typescript
class KoraPoller {
  private lastBlock = 0n;
  private interval: number;

  constructor(
    private rpcUrl: string,
    private bus: EventBus,
  ) {}

  start() {
    // Block check: every 1s (matches block time)
    this.interval = setInterval(() => this.pollBlock(), 1000);

    // Fee history: every 5s (doesn't change often)
    setInterval(() => this.pollFees(), 5000);
  }

  async pollBlock() {
    const num = await this.rpc("eth_blockNumber");
    if (num <= this.lastBlock) return; // no new block

    // Fetch all new blocks (handles >1 block gap)
    for (let n = this.lastBlock + 1n; n <= num; n++) {
      const block = await this.rpc("eth_getBlockByNumber", [hex(n), true]);
      this.bus.emit("block:new", block);

      for (const tx of block.transactions) {
        this.bus.emit("tx:confirmed", tx);
        this.bus.emit("address:seen", tx.from);
        if (tx.to) this.bus.emit("address:seen", tx.to);
      }
    }

    this.lastBlock = num;
  }

  async pollFees() {
    const history = await this.rpc("eth_feeHistory", ["0x14", "latest", [25, 50, 75]]);
    this.bus.emit("fee:update", history);
  }
}
```

### On-Demand Fetches

Not polled — triggered by user interaction:

```typescript
// User clicks an address
async function loadAddress(addr: string) {
  const [balance, nonce, code] = await Promise.all([
    rpc("eth_getBalance", [addr, "latest"]),
    rpc("eth_getTransactionCount", [addr, "latest"]),
    rpc("eth_getCode", [addr, "latest"]),
  ]);
  return { addr, balance, nonce, isContract: code !== "0x" };
}

// User clicks a transaction
async function loadTxDetail(hash: string) {
  const [tx, receipt] = await Promise.all([
    rpc("eth_getTransactionByHash", [hash]),
    rpc("eth_getTransactionReceipt", [hash]),
  ]);
  return { tx, receipt };
}

// User searches logs
async function searchLogs(filter: LogFilter) {
  return rpc("eth_getLogs", [filter]);
}
```

---

## Phase 2: WebSocket Subscriptions (After RPC Enablement)

Replace polling with push. Same EventBus interface, different source.

```typescript
class KoraSubscriber {
  private ws: WebSocket;

  constructor(
    private wsUrl: string,
    private bus: EventBus,
  ) {}

  async connect() {
    this.ws = new WebSocket(this.wsUrl);

    this.ws.onopen = () => {
      // Subscribe to everything
      this.subscribe("newHeads");
      this.subscribe("newPendingTransactions");
      this.subscribe("logs", {});
    };

    this.ws.onmessage = (msg) => {
      const data = JSON.parse(msg.data);
      if (data.method !== "eth_subscription") return;

      const sub = this.subscriptions.get(data.params.subscription);
      switch (sub.type) {
        case "newHeads":
          // Fetch full block (header doesn't include tx bodies)
          this.fetchAndEmitBlock(data.params.result.number);
          break;

        case "newPendingTransactions":
          this.bus.emit("tx:pending", data.params.result);
          break;

        case "logs":
          this.bus.emit("log:new", data.params.result);
          break;
      }
    };
  }
}
```

### What Changes Visually with Subscriptions

| Event | Polling (Phase 1) | Subscriptions (Phase 2) |
|-------|-------------------|------------------------|
| New block | Up to 1s latency | Instant |
| New transaction | Only seen after inclusion | Seen at mempool entry, then at inclusion |
| Log event | Must poll `eth_getLogs` | Instant push |
| Chain state | Sampled | Continuous |

The visual difference: with polling, the explorer *reacts*. With subscriptions, it *anticipates*.

---

## Data → Visual Mappings

### Block → Terrain Tile

```
block.hash          → heightmap seed (primary visual)
block.gasUsed       → terrain roughness multiplier
block.transactions  → vertical light beams on tile
block.stateRoot     → color palette shift (if different from parent)
block.baseFeePerGas → ambient light intensity
block.timestamp     → x-position (1 tile per second)
```

### Block → Waterfall Card

```
block.gasUsed / gasLimit → card height (4px empty → 60px full)
block.hash              → internal barcode pattern
block.transactions      → horizontal lines within card
len(transactions)       → left accent bar brightness
block.number            → label text
```

### Block → Mosaic Tile

```
block.hash[0..8]   → pattern algorithm (8 possible patterns)
block.hash[8..16]  → color variation within rose spectrum
block.hash[16..24] → symmetry (bilateral, radial, none)
block.hash[24..32] → density/fill
block.gasUsed      → opacity (empty blocks = ghost, full = vivid)
```

### Transaction → Constellation Arc

```
tx.from         → source node position (deterministic from address hash)
tx.to           → target node position
tx.value        → arc curvature (higher value = wider arc)
                → particle count (8-32)
tx.gasPrice     → particle brightness
tx.gasUsed      → arc lifetime before fade
tx.type         → arc color (transfer=bone, create=rose, call=dream)
receipt.status  → completion animation (success=crystallize, fail=shatter)
```

### Transaction → Pending State (Phase 2 only)

```
tx enters mempool  → particle spawns at sender, dream-bright, orbiting
time in mempool    → orbit radius grows (waiting longer = wider orbit)
tx.gasPrice        → particle brightness (higher fee = brighter)
tx included        → orbit snaps to arc, color shifts dream→bone
tx dropped         → particle fades, text-ghost, dissolves
```

### Fee History → Waveform

```
feeHistory.baseFeePerGas  → waveform Y values
feeHistory.gasUsedRatio   → waveform opacity/fill
feeHistory.reward[25]     → lower bound line
feeHistory.reward[75]     → upper bound line
feeHistory.oldestBlock    → x-axis start
```

Rendered as a layered area chart:
- Fill: warning color at 0.15 opacity (burnt amber)
- Line: warning color at full opacity
- 25th/75th percentile as faint bounds

### Address → Constellation Node

```
address bytes[0..4]    → spiral angle (golden ratio distribution)
address bytes[4..8]    → radius from center
transaction_count      → node size (logarithmic, 2px-8px)
balance                → ring brightness (bone spectrum)
is_contract            → shape (circle=EOA, diamond=contract)
last_active_block      → pulse rate (recent=fast, old=slow, very old=none)
```

---

## Caching Strategy

```typescript
class ExplorerCache {
  // LRU cache of full blocks — 500 most recent
  blocks: LRUMap<bigint, Block>;

  // Set of all known addresses — grows monotonically
  addresses: Map<string, AddressMeta>;

  // Ring buffer of fee history — last 1000 data points
  feeHistory: RingBuffer<FeeDataPoint>;

  // Pending transactions (Phase 2) — evicted on inclusion/timeout
  pending: Map<string, PendingTx>;
}
```

Memory budget: ~50MB for 500 blocks with full transactions.
Address set grows unbounded but each entry is <100 bytes.

---

## Fallback Behavior

When RPC is unreachable:

1. **First 3 seconds:** Continue rendering with cached data, no visual change
2. **After 3 seconds:** Status LED shifts to warning (amber pulse)
3. **After 10 seconds:** Status LED shifts to danger (red), "DISCONNECTED" label
4. **Visuals:** Terrain freezes (no new tiles), constellation dims, waterfall stops falling
5. **On reconnect:** Backfill missed blocks, fast-forward terrain/waterfall to catch up, LED returns to green

The explorer should never crash or blank-screen on connection loss. It degrades gracefully
into a frozen but still beautiful state.

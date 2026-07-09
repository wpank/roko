# DeFi Benchmarks & Performance Targets

> **Source**: Aggregated from gap analysis documents 01–12
> **Reference**: offchainservices-agent production metrics, bardo PRD performance targets
> **Purpose**: Define measurable targets and measurement infrastructure for DeFi operations

---

## 1. Latency Targets

### 1.1 Chain Event Processing

| Metric | Target | Measurement | Source Doc |
|--------|--------|------------|-----------|
| Block event → triage complete | <100ms | Timer in WitnessEngine pipeline | Doc 01 | Batch 1.2, 1.3 |
| Event → agent notification | <250ms (gamma) | Timer from event bus emit to agent receive | Doc 06 | Batch 6.1 |
| Event subscription reconnect | <2s | Timer from disconnect to resubscribe | Doc 01 | Batch 1.2 |
| Gap backfill (100 blocks) | <10s | Timer for `get_logs` backfill cycle | Doc 01 | Batch 1.1 |

### 1.2 Decision Pipeline

| Metric | Target | Measurement | Source Doc |
|--------|--------|------------|-----------|
| Gamma tick total (fast signals) | <250ms | Timer for full OBSERVE→ANALYZE→GATE cycle | Doc 06 | Batch 6.2 |
| Theta tick total (strategy eval) | <2s | Timer for full OBSERVE→EXECUTE cycle | Doc 06 | Batch 6.2 |
| Delta tick total (rebalance) | <30s | Timer for full OBSERVE→REFLECT cycle | Doc 06 | Batch 6.2 |
| Indicator computation (all active) | <50ms | Timer in ChainOracle batch compute | Doc 03 | Batch 3.1-3.5 |
| Pre-trade guard pipeline | <10ms | Timer across all PreTradeGuard evaluations | Doc 04 | Batch 4.1 |
| Position guard evaluation | <1ms | Timer per GuardState evaluation | Doc 04, Doc 12 | Batch 4.1, 4.3 |

### 1.3 Execution

| Metric | Target | Measurement | Source Doc |
|--------|--------|------------|-----------|
| Order intent → venue submission | <100ms | Timer from OrderIntent creation to adapter.place_order() | Doc 02, Doc 12 | Batch 2.1, 2.2 |
| Fill detection | <500ms | Timer from fill event to local state update | Doc 02 | Batch 2.2 |
| TWAP child order interval | Configurable (1s–60s) | Timer between child order submissions | Doc 12 |
| Tx confirmation (L1) | <15s | Timer from broadcast to receipt | Doc 01 |
| Tx confirmation (L2) | <2s | Timer from broadcast to receipt | Doc 01 |

### 1.4 HDC Operations

| Metric | Target | Measurement | Source Doc |
|--------|--------|------------|-----------|
| HDC bind/bundle (10,240-bit) | <1μs | Microbenchmark in roko-primitives | Doc 10 | Batch 10.1 |
| HDC similarity search (1K codebook) | <100μs | Microbenchmark with realistic codebook | Doc 03, Doc 10 | Batch 10.2 |
| Somatic map unbind (gut feeling) | <100ns | Microbenchmark per PRD spec | Doc 08 | Batch 8.3 |
| Market state HDC encoding | <10μs | Timer for full market→vector encoding | Doc 10 | Batch 10.1 |

---

## 2. Accuracy Targets

### 2.1 Indicator Quality

| Metric | Target | Measurement | Source Doc |
|--------|--------|------------|-----------|
| SMA/EMA/RSI/BB vs reference impl | >99.99% correlation | Compare to pandas-ta or TA-Lib output | Doc 03 |
| MACD signal accuracy | >99.9% | Crossover timing vs reference | Doc 03 |
| Regime detection precision | >80% | Against labeled historical data | Doc 03 |
| Regime detection recall | >70% | Regime changes within 2 ticks of actual | Doc 03 |
| Breakout detection precision | >60% | True breakouts vs false signals | Doc 03 |
| Funding rate prediction (1h) | <0.01% MAE | Predicted vs actual funding rate | Doc 03 |

### 2.2 Risk & Safety

| Metric | Target | Measurement | Source Doc |
|--------|--------|------------|-----------|
| MEV detection precision | >90% | Flagged sandwiches that were actually sandwiches | Doc 04 |
| MEV detection recall | >95% | Caught sandwiches vs total actual sandwiches | Doc 04 |
| Position limit enforcement | 100% | No trade exceeds configured limits, ever | Doc 04 |
| Circuit breaker false positive rate | <5% | Halts that didn't need to happen | Doc 04 |
| Drawdown detection latency | <1 tick | Drawdown detected within 1 theta tick of occurrence | Doc 04 |
| Oracle staleness detection | 100% | Every stale oracle caught before use | Doc 04, Doc 12 |

### 2.3 Learning Quality

| Metric | Target | Measurement | Source Doc |
|--------|--------|------------|-----------|
| FIFO P&L attribution accuracy | 100% | All trades correctly matched and attributed | Doc 07 | Batch 7.1 |
| Indicator accuracy tracking error | <1% | Tracked accuracy vs true accuracy | Doc 07 | Batch 7.2 |
| CascadeRouter arm selection regret | <10% after 1K obs | Cumulative regret vs optimal arm | Doc 07 | Batch 7.5 |
| Playbook trigger precision | >70% | Triggered playbooks that produced positive outcomes | Doc 07 | Batch 7.4 |
| Strategy Sharpe estimation error | <0.1 | Estimated vs realized Sharpe ratio | Doc 07 | Batch 7.3 |

---

## 3. Throughput Targets

### 3.1 Event Processing

| Metric | Target | Measurement | Source Doc |
|--------|--------|------------|-----------|
| Events/sec (single chain) | >1,000 | Sustained event ingestion rate | Doc 01 |
| Events/sec (multi-chain, N=5) | >5,000 | Aggregate across chain subscriptions | Doc 01 |
| Triage throughput | >500 events/sec | Events through full triage pipeline | Doc 01 |
| Binary Fuse filter rate | >100K events/sec | Pre-screening throughput | Doc 01 |

### 3.2 Agent & Tool Operations

| Metric | Target | Measurement | Source Doc |
|--------|--------|------------|-----------|
| Concurrent agents | >20 | Active agents running simultaneously | Doc 05 |
| Tool invocations/min | >100 | Aggregate tool calls across all agents | Doc 02 |
| Concurrent positions (multi-slot) | >10 per agent | Active positions with independent guards | Doc 12 |
| Order throughput | >10 orders/sec | Orders submitted per second across venues | Doc 02 |

### 3.3 Learning & Storage

| Metric | Target | Measurement | Source Doc |
|--------|--------|------------|-----------|
| Episode write throughput | >100/sec | Episodes persisted to JSONL per second | Doc 07 |
| Knowledge store query | <10ms p99 | Query latency for knowledge retrieval | Doc 10 |
| Playbook match latency | <5ms | Time to find matching playbook for market state | Doc 07 |
| Dream cycle completion | <60s | Time for one full dream consolidation cycle | Doc 09 |

---

## 4. Convergence Targets

### 4.1 Learning Loop Convergence

| Metric | Target | Measurement | Source Doc |
|--------|--------|------------|-----------|
| CascadeRouter convergence | <500 observations | Arm selection stabilizes within 5% of optimal | Doc 07 |
| Indicator accuracy convergence | <200 predictions | Rolling accuracy within 5% of true accuracy | Doc 07 |
| Regime detection stabilization | <50 transitions | Regime labels consistent across 95% of similar states | Doc 07 |
| Playbook mutation convergence | <10 generations | Parameter tuning converges to local optimum | Doc 07 |
| Strategy ranking stability | <100 trades per strategy | Ranking stable within 10% reordering | Doc 07 |

### 4.2 Affect & Dreams Convergence

| Metric | Target | Measurement | Source Doc |
|--------|--------|------------|-----------|
| PAD stabilization after shock | <10 ticks | PAD vector returns within 0.1 of baseline | Doc 08 |
| Somatic map saturation | ~1K bindings | Somatic map covers 90% of encountered patterns | Doc 08 |
| Dream consolidation coverage | >80% of episodes | Proportion of episodes processed in dreams | Doc 09 |
| Knowledge tier progression (D1→D2) | <50 reinforcements | Knowledge items promoted with sufficient confidence | Doc 10 |

---

## 5. Resource Targets

### 5.1 Memory

| Metric | Target | Measurement | Source Doc |
|--------|--------|------------|-----------|
| Memory per agent (base) | <50 MB | RSS at idle after initialization | Doc 05 |
| Memory per agent (active trading) | <200 MB | RSS during active trading with indicators | Doc 05 |
| ChainOracle price buffer | <10 MB per asset | Rolling window memory consumption | Doc 03 |
| HDC codebook (1K patterns) | <2 MB | Codebook storage (1K × 10,240 bits = 1.25 MB) | Doc 10 |
| Position state per wallet | <1 MB | All position tracking state per wallet | Doc 04 |

### 5.2 CPU

| Metric | Target | Measurement | Source Doc |
|--------|--------|------------|-----------|
| Gamma tick CPU | <5% of one core | CPU utilization during gamma processing | Doc 06 |
| Theta tick CPU | <20% of one core | CPU utilization during theta processing | Doc 06 |
| Delta tick CPU | <50% of one core | CPU utilization during delta processing | Doc 06 |
| Indicator batch compute | <10% of one core | CPU for all active indicators per tick | Doc 03 |
| HDC operations | <1% of one core | CPU for HDC encode/compare operations | Doc 10 |

### 5.3 Storage

| Metric | Target | Measurement | Source Doc |
|--------|--------|------------|-----------|
| Episode log growth | <100 MB/day | `.roko/episodes.jsonl` growth rate | Doc 07 |
| Trade journal growth | <10 MB/day | Trade records per day | Doc 07, Doc 12 |
| Knowledge store size | <500 MB | Total neuro store at steady state | Doc 10 |
| Dream journal growth | <50 MB/week | Dream consolidation output | Doc 09 |
| Signal log growth | <200 MB/day | `.roko/signals.jsonl` with chain events | Doc 01 |

### 5.4 Network

| Metric | Target | Measurement | Source Doc |
|--------|--------|------------|-----------|
| WebSocket bandwidth (per chain) | <1 MB/s | Sustained WS data rate | Doc 01 |
| RPC query rate | <100 req/s per chain | Rate-limited query frequency | Doc 01 |
| Venue API bandwidth | <100 KB/s | Order submission + market data | Doc 02 |

---

## 6. Measurement Infrastructure

### 6.1 Collection Points

All metrics should be collected via roko's existing event infrastructure:

| Collection Method | Where | What |
|-------------------|-------|------|
| **Efficiency events** | `roko-learn/src/efficiency.rs` | Per-turn cost, latency, token usage |
| **Episode logger** | `roko-learn/src/episode_logger.rs` | Agent turns, decisions, outcomes |
| **Conductor watchers** | `roko-conductor/src/` | Health checks, circuit breaker state |
| **Signal log** | `.roko/signals.jsonl` | All signals including chain events |
| **TUI dashboard** | `roko-cli/src/tui/` | Real-time visualization |
| **HTTP API** | `roko-serve/src/routes/` | Programmatic metric access |

### 6.2 New Collection Points Needed

| Collection Point | Purpose | Integration |
|-----------------|---------|-------------|
| **Trade journal** | Fill records, P&L snapshots | New: `TradingReflect` module (Doc 07) |
| **Indicator accuracy log** | Prediction vs outcome per indicator | New: extend ChainOracle (Doc 03) |
| **Guard state log** | Position guard evaluations and actions | New: extend gate pipeline (Doc 04) |
| **Venue latency log** | Per-venue operation latencies | New: VenueAdapter instrumentation (Doc 02) |
| **Chain event lag** | Time from chain event to roko processing | New: WitnessEngine instrumentation (Doc 01) |

### 6.3 Reporting

| Channel | Frequency | Content |
|---------|-----------|---------|
| **TUI Dashboard tab** | Real-time | Latency histograms, throughput gauges, position state |
| **HTTP `/api/metrics`** | On-demand | JSON metrics snapshot for external monitoring |
| **Nightly REFLECT report** | Daily | P&L summary, strategy performance, risk utilization |
| **Conductor alerts** | On-event | Circuit breaker triggers, risk limit breaches |
| **Dream journal** | Per-cycle | Consolidation insights, strategy discoveries |

### 6.4 Alerting Thresholds

| Alert | Condition | Severity | Action |
|-------|-----------|----------|--------|
| **Latency spike** | Gamma tick >500ms (2x target) | Warning | Log + TUI highlight |
| **Latency critical** | Gamma tick >1s (4x target) | Critical | Reduce to delta-only mode |
| **Event backlog** | >1000 unprocessed events | Warning | Increase processing parallelism |
| **Risk limit proximity** | >80% of any risk limit | Warning | Alert + reduce position sizing |
| **Risk limit breach** | >100% of any risk limit | Critical | Halt all trading + alert |
| **MEV detected** | Sandwich or front-run flagged | Warning | Switch to private mempool |
| **Oracle stale** | Any oracle stale >45s | Critical | Halt trading for affected markets |
| **Memory pressure** | Agent RSS >500MB | Warning | GC signal log + reduce price buffer |
| **Disk pressure** | Signal log >1GB/day | Warning | Increase GC frequency |

---

## 7. Benchmark Infrastructure

### 7.1 Microbenchmarks

Existing: roko-primitives has `#[bench]` tests for HDC operations.

Needed:
- [ ] ChainOracle indicator computation benchmarks (all 60+ indicators)
- [ ] Triage pipeline throughput benchmark (events/sec)
- [ ] Guard evaluation benchmark (evaluations/sec)
- [ ] VenueAdapter operation benchmark (per-venue latency)
- [ ] HDC market encoding benchmark (encode + similarity per market state)

### 7.2 Integration Benchmarks

- [ ] End-to-end tick latency: chain event → agent decision → order submission
- [ ] Multi-agent throughput: N agents processing M events concurrently
- [ ] Learning convergence: CascadeRouter observation count to stable arm selection
- [ ] Dream cycle: time and memory for consolidation of N episodes

### 7.3 Load Testing

- [ ] Sustained event ingestion at 1K/sec for 1 hour
- [ ] 20 concurrent agents with independent positions
- [ ] Multi-chain (5 chains) simultaneous event processing
- [ ] Worst-case scenario: volatile market with gamma interrupts every tick

### 7.4 Reference Comparison

| Metric | Offchain Agent | Roko Target | Notes |
|--------|---------------|-------------|-------|
| Tick frequency | 1-5s (configurable) | 250ms/2s/30s (gamma/theta/delta) | Roko targets faster gamma |
| Strategies per agent | 14 (ensemble) | Unlimited via archetypes | Roko uses composition |
| Concurrent positions | N (multi-slot) | N (multi-slot via heartbeat) | Equivalent |
| Nightly review | 1/day (REFLECT) | 1/day + dreams | Roko adds dream consolidation |
| Risk gates | 3 levels (wallet, house, slot) | 6 layers (L1-L6) | Roko more comprehensive |
| Venues | 3 (HL, Nunchi, mock) | N (VenueAdapter trait) | Roko designed for extensibility |

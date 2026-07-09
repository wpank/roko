# WU-16: Dashboard Chain Sub-Page

**Layer**: 5 (depends on all backend WUs)
**Depends on**: WU-12 (sidecar/serve routes), WU-11 (watcher events), WU-13 (orchestrator wiring)
**Blocks**: none (leaf unit)
**Estimated effort**: 4-5 hours
**App**: `demo/demo-app`

---

## Overview

Add a **`/dashboard/chain`** sub-page to the dashboard — an interactive, always-on monitoring and query surface for verified chain state. Unlike the scripted demo scenario (WU-15), this page is a persistent dev tool for:

- Real-time block feed from chain watcher (SSE events)
- Interactive verified balance/storage queries
- Backend health overview
- Trust level visualization
- Payment verification lookup

Follows the existing dashboard page pattern: fetches from roko-serve API, subscribes to SSE events, uses the ROSEDUST design system.

---

## Pre-read

- `demo/demo-app/src/pages/dashboard/Layout.tsx` — `VIEWS` array, `NavLink` tabs, `<Outlet />`
- `demo/demo-app/src/pages/dashboard/IntegrityView.tsx` — closest reference: hash chain visualization, gate waterfall, chain-adjacent data
- `demo/demo-app/src/pages/dashboard/CostDashboard.tsx` — canonical data-fetching pattern: `fetchAll()` → `Promise.all()` → state → `useEffect` polling + SSE
- `demo/demo-app/src/transport/api.ts` — `api.get()`, `api.post()`, `ApiResult<T>`
- `demo/demo-app/src/contexts/EventStreamContext.tsx` — `useContextEventSubscription`
- `demo/demo-app/src/transport/types.ts` — `ServerEvent` type union (add new chain event types)
- `demo/demo-app/src/components/ChainActivityPanel.tsx` — existing block feed panel (reusable)
- `demo/demo-app/src/hooks/useBlockStream.ts` — existing block stream hook (may reuse or adapt for SSE)
- `demo/demo-app/src/styles/tokens.css` — ROSEDUST design tokens

---

## Tasks

### 16.1 Add route to dashboard layout

**File**: `demo/demo-app/src/pages/dashboard/Layout.tsx`

Add to `VIEWS` array:
```typescript
const VIEWS = [
  // ... existing views ...
  { to: '/dashboard/chain', label: 'Chain', icon: 'chain' },
];
```

### 16.2 Add route to router

**File**: `demo/demo-app/src/main.tsx`

Add inside the `/dashboard` `<Route>`:
```tsx
<Route path="chain" element={<Suspense fallback={<LoadingFallback />}><ChainDashboard /></Suspense>} />
```

Add lazy import:
```tsx
const ChainDashboard = lazy(() => import('./pages/dashboard/ChainDashboard'));
```

### 16.3 Add chain event types to transport/types.ts

**File**: `demo/demo-app/src/transport/types.ts`

Add to the `ServerEvent` union type:

```typescript
| { type: 'chain_new_block'; backend: string; blockNumber: number; blockHash: string; timestamp: number }
| { type: 'chain_events_matched'; backend: string; blockNumber: number; eventCount: number; summary: string }
| { type: 'chain_watcher_health'; backend: string; healthy: boolean; message: string }
```

### 16.4 Handle new events in DataHub

**File**: `demo/demo-app/src/app/DataHub.ts`

Add to the `handleServerEvent()` switch:

```typescript
case 'chain_new_block': {
  // Store in a ring buffer of recent blocks
  const blocks = get().chainBlocks ?? [];
  const next = [...blocks, event].slice(-50); // keep last 50
  set({ chainBlocks: next });
  break;
}
case 'chain_watcher_health': {
  set({ chainWatcherHealth: { backend: event.backend, healthy: event.healthy, message: event.message } });
  break;
}
```

Add to the store interface:
```typescript
chainBlocks: ChainBlock[];
chainWatcherHealth: { backend: string; healthy: boolean; message: string } | null;
```

Where:
```typescript
interface ChainBlock {
  backend: string;
  blockNumber: number;
  blockHash: string;
  timestamp: number;
}
```

### 16.5 Create the dashboard page

**File**: `demo/demo-app/src/pages/dashboard/ChainDashboard.tsx`

The page has 4 sections:

```
┌─────────────────────────────────────────────────────────────┐
│  BACKENDS                                                    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                 │
│  │ tempo-mod│  │ local-dev│  │ (empty)  │  ← BackendCards  │
│  │ ●healthy │  │ ○offline │  │          │                   │
│  │ BLS      │  │ RPC      │  │          │                   │
│  └──────────┘  └──────────┘  └──────────┘                 │
├─────────────────────────────────────────────────────────────┤
│  VERIFIED HEAD                              BLOCK FEED      │
│  Block #142,857                            │ #142857 0xa3..│
│  Hash: 0xa3b4c5d6...                       │ #142856 0xf1..│
│  State Root: 0x7890ab...                   │ #142855 0xe2..│
│  Trust: ██████ Cryptographic               │ #142854 0xd4..│
│  Consensus: threshold_bls                   │              │
├─────────────────────────────────────────────────────────────┤
│  QUERY                                                       │
│  ┌─ Verified Balance ──────────────────────────────────┐    │
│  │ Address: [0x________________________] [Query]       │    │
│  │ Result: 1.234567 ETH                                │    │
│  │ Trust: RpcTrusted │ Block: 142857 │ Network: tempo  │    │
│  └─────────────────────────────────────────────────────┘    │
│  ┌─ Verify Transfer ──────────────────────────────────┐    │
│  │ Tx Hash: [0x________________________] [Verify]      │    │
│  │ Status: Success │ Gas: 21000 │ Block: 142850        │    │
│  │ Trust: Cryptographic │ Consensus: threshold_bls     │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

Implementation:

```tsx
import { useCallback, useEffect, useState } from 'react';
import { api } from '../../transport/api';
import { useContextEventSubscription } from '../../contexts/EventStreamContext';
import { PageShell } from '../../components/layout/PageShell';
import { DataSurface } from '../../components/layout/DataSurface';

// Types for API responses
interface BackendInfo {
  name: string;
  rpc_url: string | null;
  chain_id: number | null;
  consensus: string;
  label: string | null;
}

interface HeadInfo {
  block_number: number;
  block_hash: string | null;
  state_root: string | null;
  timestamp: number | null;
}

interface VerifiedBalanceResult {
  address: string;
  balance_wei: string;
  balance_eth?: string;
  block_number: number;
  trust_level: string;
  consensus_mechanism: string;
  network: string;
}

interface VerifyTransferResult {
  tx_hash: string;
  status: string;
  block_number: number;
  gas_used: number;
  trust_level: string;
  consensus_mechanism: string;
}

export default function ChainDashboard() {
  const [backends, setBackends] = useState<BackendInfo[]>([]);
  const [head, setHead] = useState<HeadInfo | null>(null);
  const [watcherHealthy, setWatcherHealthy] = useState<boolean | null>(null);
  const [balanceQuery, setBalanceQuery] = useState('');
  const [balanceResult, setBalanceResult] = useState<VerifiedBalanceResult | null>(null);
  const [txQuery, setTxQuery] = useState('');
  const [txResult, setTxResult] = useState<VerifyTransferResult | null>(null);
  const [loading, setLoading] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Fetch backends and head on mount + interval
  const fetchAll = useCallback(async () => {
    const [backendsRes, headRes] = await Promise.all([
      api.get<{ backends: BackendInfo[]; default: string | null }>('/api/chain/backends'),
      api.get<HeadInfo>('/api/chain/head'),
    ]);
    if (backendsRes.ok) setBackends(backendsRes.data.backends);
    if (headRes.ok) setHead(headRes.data);
  }, []);

  useEffect(() => {
    fetchAll();
    const id = setInterval(fetchAll, 15_000);
    return () => clearInterval(id);
  }, [fetchAll]);

  // SSE subscription for live block events
  useContextEventSubscription(
    ['chain_new_block', 'chain_watcher_health'],
    useCallback((event: any) => {
      if (event.type === 'chain_new_block') {
        setHead({
          block_number: event.blockNumber,
          block_hash: event.blockHash,
          state_root: null, // SSE doesn't include state_root
          timestamp: event.timestamp,
        });
      }
      if (event.type === 'chain_watcher_health') {
        setWatcherHealthy(event.healthy);
      }
    }, []),
  );

  // Query verified balance
  const queryBalance = async () => {
    if (!balanceQuery.trim()) return;
    setLoading('balance');
    setError(null);
    const res = await api.get<VerifiedBalanceResult>(
      `/api/chain/verified/balance/${encodeURIComponent(balanceQuery.trim())}`,
    );
    if (res.ok) {
      setBalanceResult(res.data);
    } else {
      setError(`Balance query failed: ${res.error}`);
    }
    setLoading(null);
  };

  // Verify transfer (placeholder — needs backend route wired)
  const verifyTransfer = async () => {
    if (!txQuery.trim()) return;
    setLoading('tx');
    setError(null);
    // TODO: Wire to /api/chain/verified/transfer/:hash when route exists
    setError('Transfer verification route not yet wired');
    setLoading(null);
  };

  return (
    <PageShell title="Chain">
      {/* Section 1: Backends */}
      <DataSurface title="Backends">
        <div className="chain-backends-grid">
          {backends.length === 0 && (
            <p className="chain-empty">No chain backends configured. Add [chain.backends.*] to roko.toml.</p>
          )}
          {backends.map((b) => (
            <div key={b.name} className="chain-backend-card">
              <div className="chain-backend-name">{b.label || b.name}</div>
              <div className="chain-backend-consensus">{b.consensus}</div>
              <div className="chain-backend-chain-id">Chain {b.chain_id}</div>
              {b.rpc_url && <div className="chain-backend-rpc">{truncateUrl(b.rpc_url)}</div>}
            </div>
          ))}
        </div>
      </DataSurface>

      {/* Section 2: Verified Head + Block Feed */}
      <div className="chain-head-row">
        <DataSurface title="Verified Head">
          {head ? (
            <div className="chain-head-info">
              <div className="chain-head-block">
                <span className="chain-label">Block</span>
                <span className="chain-value chain-mono">#{head.block_number.toLocaleString()}</span>
              </div>
              {head.block_hash && (
                <div className="chain-head-hash">
                  <span className="chain-label">Hash</span>
                  <span className="chain-value chain-mono">{truncateHash(head.block_hash)}</span>
                </div>
              )}
              {head.state_root && (
                <div className="chain-head-root">
                  <span className="chain-label">State Root</span>
                  <span className="chain-value chain-mono">{truncateHash(head.state_root)}</span>
                </div>
              )}
              {head.timestamp && (
                <div className="chain-head-time">
                  <span className="chain-label">Timestamp</span>
                  <span className="chain-value">{new Date(head.timestamp * 1000).toISOString()}</span>
                </div>
              )}
              <div className="chain-head-health">
                <span className={`chain-health-dot ${watcherHealthy === false ? 'unhealthy' : 'healthy'}`} />
                <span>{watcherHealthy === false ? 'Watcher unhealthy' : 'Watcher active'}</span>
              </div>
            </div>
          ) : (
            <p className="chain-empty">No block data. Is roko serve running with [chain] configured?</p>
          )}
        </DataSurface>
      </div>

      {/* Section 3: Interactive Queries */}
      <DataSurface title="Verified Queries">
        {error && <div className="chain-error">{error}</div>}

        {/* Balance query */}
        <div className="chain-query-section">
          <h4>Verified Balance</h4>
          <div className="chain-query-row">
            <input
              className="chain-input"
              placeholder="0x address..."
              value={balanceQuery}
              onChange={(e) => setBalanceQuery(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && queryBalance()}
            />
            <button className="chain-btn" onClick={queryBalance} disabled={loading === 'balance'}>
              {loading === 'balance' ? 'Querying...' : 'Query'}
            </button>
          </div>
          {balanceResult && (
            <div className="chain-result">
              <div className="chain-result-row">
                <span className="chain-label">Balance</span>
                <span className="chain-value chain-mono">{balanceResult.balance_eth || balanceResult.balance_wei} {balanceResult.balance_eth ? 'ETH' : 'wei'}</span>
              </div>
              <div className="chain-result-row">
                <span className="chain-label">Trust</span>
                <TrustBadge level={balanceResult.trust_level} />
              </div>
              <div className="chain-result-row">
                <span className="chain-label">Consensus</span>
                <span className="chain-value">{balanceResult.consensus_mechanism}</span>
              </div>
              <div className="chain-result-row">
                <span className="chain-label">Block</span>
                <span className="chain-value chain-mono">#{balanceResult.block_number}</span>
              </div>
              <div className="chain-result-row">
                <span className="chain-label">Network</span>
                <span className="chain-value">{balanceResult.network}</span>
              </div>
            </div>
          )}
        </div>

        {/* Transfer verification */}
        <div className="chain-query-section">
          <h4>Verify Transfer</h4>
          <div className="chain-query-row">
            <input
              className="chain-input"
              placeholder="0x tx hash..."
              value={txQuery}
              onChange={(e) => setTxQuery(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && verifyTransfer()}
            />
            <button className="chain-btn" onClick={verifyTransfer} disabled={loading === 'tx'}>
              {loading === 'tx' ? 'Verifying...' : 'Verify'}
            </button>
          </div>
          {txResult && (
            <div className="chain-result">
              <div className="chain-result-row">
                <span className="chain-label">Status</span>
                <span className={`chain-value ${txResult.status === 'success' ? 'chain-success' : 'chain-fail'}`}>
                  {txResult.status}
                </span>
              </div>
              <div className="chain-result-row">
                <span className="chain-label">Trust</span>
                <TrustBadge level={txResult.trust_level} />
              </div>
              <div className="chain-result-row">
                <span className="chain-label">Block</span>
                <span className="chain-value chain-mono">#{txResult.block_number}</span>
              </div>
              <div className="chain-result-row">
                <span className="chain-label">Gas Used</span>
                <span className="chain-value">{txResult.gas_used.toLocaleString()}</span>
              </div>
            </div>
          )}
        </div>
      </DataSurface>
    </PageShell>
  );
}

// ── Helper components ────────────────────────────────────────────

function TrustBadge({ level }: { level: string }) {
  const color = level === 'Cryptographic' ? 'var(--sage)'
    : level === 'RpcTrusted' ? 'var(--warning)'
    : 'var(--text-dim)';
  return (
    <span className="chain-trust-badge" style={{ borderColor: color, color }}>
      {level}
    </span>
  );
}

function truncateHash(hash: string): string {
  if (hash.length <= 18) return hash;
  return `${hash.slice(0, 10)}...${hash.slice(-6)}`;
}

function truncateUrl(url: string): string {
  try {
    const u = new URL(url);
    return u.hostname;
  } catch {
    return url.slice(0, 30);
  }
}
```

### 16.6 Add CSS

**File**: `demo/demo-app/src/pages/dashboard/ChainDashboard.css`

Use ROSEDUST tokens:

```css
.chain-backends-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 1rem;
}

.chain-backend-card {
  background: var(--bg-raised);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 1rem;
}

.chain-backend-name {
  font-family: var(--font-mono);
  font-weight: 600;
  color: var(--text);
  margin-bottom: 0.5rem;
}

.chain-backend-consensus {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  color: var(--rose);
}

.chain-backend-chain-id,
.chain-backend-rpc {
  font-size: var(--text-xs);
  color: var(--text-dim);
  margin-top: 0.25rem;
}

.chain-head-row {
  display: grid;
  grid-template-columns: 1fr;
  gap: 1rem;
}

.chain-head-info {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.chain-label {
  font-size: var(--text-xs);
  color: var(--text-dim);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  min-width: 90px;
  display: inline-block;
}

.chain-value {
  color: var(--text);
}

.chain-mono {
  font-family: var(--font-mono);
}

.chain-health-dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-right: 0.5rem;
}
.chain-health-dot.healthy { background: var(--sage); }
.chain-health-dot.unhealthy { background: var(--error); }

.chain-query-section {
  margin-bottom: 1.5rem;
}

.chain-query-section h4 {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  color: var(--rose);
  margin-bottom: 0.5rem;
}

.chain-query-row {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
}

.chain-input {
  flex: 1;
  background: var(--bg-void);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 0.5rem 0.75rem;
  color: var(--text);
  font-family: var(--font-mono);
  font-size: var(--text-sm);
}

.chain-input:focus {
  border-color: var(--rose);
  outline: none;
}

.chain-btn {
  background: var(--rose-dim);
  color: var(--text);
  border: 1px solid var(--rose);
  border-radius: 6px;
  padding: 0.5rem 1rem;
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: background 0.15s;
}

.chain-btn:hover { background: var(--rose); }
.chain-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.chain-result {
  background: var(--bg-raised);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.chain-result-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.chain-trust-badge {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  border: 1px solid;
  border-radius: 4px;
  padding: 0.15rem 0.5rem;
}

.chain-success { color: var(--sage); }
.chain-fail { color: var(--error); }

.chain-empty {
  color: var(--text-dim);
  font-style: italic;
}

.chain-error {
  color: var(--error);
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  background: var(--bg-raised);
  border: 1px solid var(--error);
  border-radius: 6px;
  padding: 0.75rem;
  margin-bottom: 1rem;
}
```

Import the CSS at the top of `ChainDashboard.tsx`:
```tsx
import './ChainDashboard.css';
```

---

## API Endpoints Required

These routes should already exist from WU-12:

| Method | Path | Source |
|--------|------|--------|
| `GET` | `/api/chain/backends` | roko-serve (WU-12) |
| `GET` | `/api/chain/head` | roko-serve (WU-12) |
| `GET` | `/api/chain/verified/balance/:address` | roko-serve (WU-12) |
| `GET` | `/api/chain/status` | roko-serve (existing) |

**New route needed** (add to WU-12 or as a follow-up):
| `GET` | `/api/chain/verified/transfer/:hash` | roko-serve |

SSE events (from WU-11):
| Event | Source |
|-------|--------|
| `chain_new_block` | roko-serve event bus via watcher bridge |
| `chain_watcher_health` | roko-serve event bus via watcher bridge |

---

## Verification Checklist

- [ ] Route `/dashboard/chain` exists and renders `ChainDashboard`
- [ ] Dashboard tab bar shows "Chain" tab with chain icon
- [ ] `VIEWS` array in `Layout.tsx` includes the new entry
- [ ] `main.tsx` has the lazy route for `ChainDashboard`
- [ ] Backends section shows cards for each configured backend
- [ ] Head section shows block number, hash, state root, timestamp
- [ ] Watcher health dot shows green/red based on SSE events
- [ ] Balance query sends GET to `/api/chain/verified/balance/:address`
- [ ] Balance result shows trust badge with color coding
- [ ] Transfer verify input present (wired to backend when route exists)
- [ ] SSE subscription updates head in real-time as blocks arrive
- [ ] Empty states shown when no backends / no data
- [ ] Error states shown when API calls fail
- [ ] Uses ROSEDUST tokens (no inline colors, no Tailwind)
- [ ] CSS in separate `.css` file using custom properties
- [ ] `npm run build` in `demo/demo-app` succeeds
- [ ] Page renders without console errors
- [ ] Responsive — usable at 1024px+ width

# F4: Create ISFR Dashboard Page

## Context

A new dashboard page at `/dashboard/isfr` shows live ISFR keeper data: current composite rate, per-class breakdown, rate history sparkline, source health table, and confidence meter. It connects to roko-serve for data (REST + SSE stream) via the same patterns as CostDashboard.

The frontend is at `/Users/will/dev/nunchi/roko/roko/demo/demo-app` — a plain React SPA (Vite + React 19 + Zustand). There is NO Tauri. All data comes from `roko serve` via REST and SSE.

## Files to Create

- `demo/demo-app/src/pages/dashboard/IsfrDashboard.tsx` (NEW)
- `demo/demo-app/src/pages/dashboard/IsfrDashboard.css` (NEW)

## Files to Modify

- `demo/demo-app/src/main.tsx` — add lazy import + route under `/dashboard/isfr`
- `demo/demo-app/src/pages/dashboard/Layout.tsx` — add nav entry for ISFR

## Prerequisites

- F1 (roko-serve ISFR REST API: `/api/isfr/status`, `/api/isfr/current`, `/api/isfr/history`, `/api/isfr/sources`)
- F2 (SSE events: `isfr_rate_computed`, `isfr_source_health`)
- F3 (creates `src/lib/isfr-api.ts` with types + REST functions, adds ISFR events to DataHub)

## Pre-Check

```bash
# See how CostDashboard is structured
grep -n "useLiveApi\|useContextEventSubscription\|useDebouncedRefetch" demo/demo-app/src/pages/dashboard/CostDashboard.tsx

# See existing route registrations
grep -n "Route\|lazy" demo/demo-app/src/main.tsx | head -20

# See dashboard nav entries
grep -n "to:" demo/demo-app/src/pages/dashboard/Layout.tsx

# Confirm F3 API module exists
ls demo/demo-app/src/lib/isfr-api.ts
```

## API Shapes (from F3 `src/lib/isfr-api.ts`)

F3 creates these types and functions. This task IMPORTS them — does not redefine them.

```typescript
// Types (flat fields, no nesting):
export interface IsfrStatus {
  enabled: boolean;
  keeper_running: boolean;
  sources_count: number;
  current_rate_bps: number | null;
  current_confidence: number | null;
  current_epoch: number | null;
  poll_interval_secs: number;
  epoch_duration_secs: number;
}

export interface IsfrRate {
  composite_bps: number;
  lending_bps: number;
  structured_bps: number;
  funding_bps: number;
  staking_bps: number;
  confidence_bps: number;   // 0–10000 (not 0.0–1.0)
  timestamp_ms: number;
  readings: unknown[];
}

export interface IsfrSource {
  id: string;
  name: string;
  class: string;
  weight: number;
  last_rate_bps: number | null;
  health: 'healthy' | 'degraded' | 'stale' | 'failed';
  last_poll_ms: number | null;
}

// Functions:
export function fetchIsfrStatus(): Promise<IsfrStatus>;
export function fetchIsfrCurrentRate(): Promise<IsfrRate | null>;
export function fetchIsfrHistory(limit?: number): Promise<IsfrRate[]>;
export function fetchIsfrSources(): Promise<IsfrSource[]>;
export function formatBps(bps: number | null | undefined): string;
export function formatPercent(bps: number | null | undefined): string;
export function formatConfidence(confidenceBps: number | null | undefined): string;
```

## Implementation

### Step 1: Create `src/pages/dashboard/IsfrDashboard.tsx`

Follow the CostDashboard pattern exactly: `useLiveApi` for REST, `useContextEventSubscription`
for SSE-triggered refetch, `useDebouncedRefetch` for throttling, `useCountUp` for animated
numbers, phosphor-flash on value change.

```tsx
import { useState, useEffect, useCallback, useRef } from 'react';
import { useLiveApi } from '../../hooks/useLiveApi';
import { useContextEventSubscription } from '../../contexts/EventStreamContext';
import { useDebouncedRefetch } from '../../hooks/useDebouncedRefetch';
import { useCountUp } from '../../hooks/useCountUp';
import { useCanvasSetup } from '../../hooks/useCanvasSetup';
import Pane from '../../components/Pane';
import Mosaic, { MosaicCell } from '../../components/Mosaic';
import {
  type IsfrRate,
  type IsfrSource,
  type IsfrStatus,
  formatBps,
  formatPercent,
  formatConfidence,
} from '../../lib/isfr-api';
import '../dashboard/dashboard.css';
import './IsfrDashboard.css';

/* ── Phosphor decay hook ─────────────────────────────────── */

function usePhosphorDecay(value: number): boolean {
  const prevRef = useRef(value);
  const [flashing, setFlashing] = useState(false);
  useEffect(() => {
    if (prevRef.current !== value && value !== 0) {
      setFlashing(true);
      const id = setTimeout(() => setFlashing(false), 300);
      prevRef.current = value;
      return () => clearTimeout(id);
    }
    prevRef.current = value;
  }, [value]);
  return flashing;
}

/* ── Constants ───────────────────────────────────────────── */

const MAX_HISTORY = 60;

const HEALTH_COLORS: Record<string, string> = {
  healthy: 'var(--success)',
  degraded: 'var(--warning)',
  stale: 'var(--text-dim)',
  failed: 'var(--rose-bright)',
};

/* ── Component ───────────────────────────────────────────── */

export default function IsfrDashboard() {
  const { get } = useLiveApi();
  const [status, setStatus] = useState<IsfrStatus | null>(null);
  const [currentRate, setCurrentRate] = useState<IsfrRate | null>(null);
  const [sources, setSources] = useState<IsfrSource[]>([]);
  const [history, setHistory] = useState<IsfrRate[]>([]);
  const [initialLoading, setInitialLoading] = useState(true);

  const fetchAll = useCallback(async () => {
    const [s, r, src, h] = await Promise.all([
      get<IsfrStatus>('/api/isfr/status'),
      get<IsfrRate | null>('/api/isfr/current'),
      get<IsfrSource[]>('/api/isfr/sources'),
      get<IsfrRate[]>('/api/isfr/history?limit=' + MAX_HISTORY),
    ]);
    setStatus(s);
    setCurrentRate(r);
    setSources(src);
    setHistory(h);
  }, [get]);

  // Initial fetch + 30s fallback poll
  useEffect(() => {
    fetchAll().finally(() => setInitialLoading(false));
    const id = setInterval(fetchAll, 30_000);
    return () => clearInterval(id);
  }, [fetchAll]);

  // SSE-triggered refetch (debounced 2s)
  const debouncedRefetch = useDebouncedRefetch(fetchAll, 2000);
  useContextEventSubscription(
    ['isfr_rate_computed', 'isfr_source_health'],
    debouncedRefetch,
  );

  /* Derived values */
  const compositeBps = currentRate?.composite_bps ?? 0;
  const confidenceBps = currentRate?.confidence_bps ?? 0;
  const keeperRunning = status?.keeper_running ?? false;
  const sourcesCount = status?.sources_count ?? 0;
  const epoch = status?.current_epoch ?? 0;
  const confidencePct = confidenceBps / 100; // 0–100 range

  /* Animated counters */
  const animComposite = useCountUp(compositeBps, 900);
  const animConfidence = useCountUp(confidencePct, 800);

  /* Phosphor flash */
  const compositeFlash = usePhosphorDecay(compositeBps);
  const confidenceFlash = usePhosphorDecay(confidenceBps);

  /* Sparkline data: extract composite_bps from history */
  const sparklineData = history.map((r) => r.composite_bps);

  /* Source health summary */
  const healthyCt = sources.filter((s) => s.health === 'healthy').length;

  if (initialLoading) {
    return (
      <div className="dash-page progressive-reveal cd-skeleton-layout">
        <div className="skeleton cd-skeleton-hero" />
        <div className="cd-skeleton-grid">
          <div className="skeleton-card skeleton" />
          <div className="skeleton-card skeleton" />
          <div className="skeleton-card skeleton" />
        </div>
        <div className="skeleton-chart skeleton" />
      </div>
    );
  }

  return (
    <div className="dash-page">
      {/* STATUS BANNER */}
      <div className="dash-stagger gradient-border-subtle" style={{ '--stagger-i': 0 } as React.CSSProperties}>
        <Mosaic columns={5}>
          <MosaicCell
            label="KEEPER"
            value={
              <span className="dash-inline">
                <span
                  className="dash-dot"
                  style={{
                    background: keeperRunning ? 'var(--success)' : 'var(--rose-bright)',
                    boxShadow: keeperRunning ? 'var(--glow-success)' : 'var(--glow-error)',
                    animation: keeperRunning ? 'pulse-dot 2s ease-in-out infinite' : 'none',
                  }}
                />
                <span className="dash-mono-label">{keeperRunning ? 'Running' : 'Stopped'}</span>
              </span>
            }
            color="success"
            sub={`epoch ${epoch}`}
          />
          <MosaicCell
            label="COMPOSITE RATE"
            value={
              <span className={compositeFlash ? 'phosphor-flash' : ''}>
                {formatBps(Math.round(animComposite))}
              </span>
            }
            color="rose"
            mono
            sub={formatPercent(compositeBps)}
          />
          <MosaicCell
            label="CONFIDENCE"
            value={
              <span className={confidenceFlash ? 'phosphor-flash' : ''}>
                {animConfidence.toFixed(1)}%
              </span>
            }
            color="bone"
            mono
            sub={`${confidenceBps} / 10000 bps`}
          />
          <MosaicCell
            label="SOURCES"
            value={`${healthyCt}/${sourcesCount}`}
            color="dream"
            sub={`${healthyCt} healthy`}
          />
          <MosaicCell
            label="EPOCH"
            value={String(epoch)}
            color="warning"
            mono
            sub={status?.epoch_duration_secs ? `${status.epoch_duration_secs}s duration` : '—'}
          />
        </Mosaic>
      </div>

      {/* MIDDLE ROW: Rate Breakdown + Sparkline */}
      <div className="dash-flex-row">
        {/* Left: Per-class rate breakdown */}
        <div className="dash-flex-1 dash-stagger" style={{ '--stagger-i': 1 } as React.CSSProperties}>
          <Pane
            title="RATE BREAKDOWN"
            badge={<span className="dash-badge--glow">{formatBps(compositeBps)}</span>}
          >
            <Mosaic columns={4}>
              <MosaicCell
                label="LENDING"
                value={formatBps(currentRate?.lending_bps ?? null)}
                color="rose"
                mono
                sparkline={history.map((r) => r.lending_bps)}
              />
              <MosaicCell
                label="STRUCTURED"
                value={formatBps(currentRate?.structured_bps ?? null)}
                color="bone"
                mono
                sparkline={history.map((r) => r.structured_bps)}
              />
              <MosaicCell
                label="FUNDING"
                value={formatBps(currentRate?.funding_bps ?? null)}
                color="dream"
                mono
                sparkline={history.map((r) => r.funding_bps)}
              />
              <MosaicCell
                label="STAKING"
                value={formatBps(currentRate?.staking_bps ?? null)}
                color="warning"
                mono
                sparkline={history.map((r) => r.staking_bps)}
              />
            </Mosaic>
          </Pane>
        </div>

        {/* Right: Composite rate sparkline */}
        <div className="dash-flex-1 dash-stagger" style={{ '--stagger-i': 2 } as React.CSSProperties}>
          <Pane
            title="RATE HISTORY"
            badge={<span className="dash-badge">{history.length} readings</span>}
          >
            <div className="dash-chart-enter">
              <IsfrRateChart data={sparklineData} height={140} />
            </div>
          </Pane>
        </div>
      </div>

      {/* CONFIDENCE METER */}
      <div className="dash-stagger" style={{ '--stagger-i': 3 } as React.CSSProperties}>
        <Pane title="CONFIDENCE" badge={<span className="dash-badge">{formatConfidence(confidenceBps)}</span>}>
          <div className="isfr-confidence-meter">
            <div className="isfr-confidence-track">
              <div
                className="isfr-confidence-fill"
                style={{
                  width: `${confidencePct}%`,
                  background: confidencePct >= 80
                    ? 'var(--success)'
                    : confidencePct >= 50
                      ? 'var(--warning)'
                      : 'var(--rose-bright)',
                }}
              />
            </div>
            <div className="isfr-confidence-labels">
              <span>0%</span>
              <span>50%</span>
              <span>100%</span>
            </div>
          </div>
        </Pane>
      </div>

      {/* SOURCE HEALTH TABLE */}
      <div className="dash-stagger" style={{ '--stagger-i': 4 } as React.CSSProperties}>
        <Pane
          title="SOURCE HEALTH"
          badge={
            <span className="dash-badge">
              {healthyCt}/{sourcesCount} healthy
            </span>
          }
        >
          <div className="isfr-source-table">
            <div className="isfr-source-header">
              <span>Source</span>
              <span>Class</span>
              <span>Weight</span>
              <span>Last Rate</span>
              <span>Health</span>
            </div>
            {sources.length === 0 && (
              <div className="isfr-source-empty">No sources registered</div>
            )}
            {sources.map((src) => (
              <div key={src.id} className="isfr-source-row">
                <span className="isfr-source-name">{src.name}</span>
                <span className="isfr-source-class">{src.class}</span>
                <span className="isfr-source-weight">{(src.weight * 100).toFixed(0)}%</span>
                <span className="isfr-source-rate mono">{formatBps(src.last_rate_bps)}</span>
                <span className="isfr-source-health">
                  <span
                    className="dash-dot--7"
                    style={{
                      background: HEALTH_COLORS[src.health] ?? 'var(--text-dim)',
                      boxShadow: src.health === 'healthy' ? 'var(--glow-success)' : 'none',
                      animation: src.health === 'healthy' ? 'pulse-dot 2s ease-in-out infinite' : 'none',
                    }}
                  />
                  <span>{src.health}</span>
                </span>
              </div>
            ))}
          </div>
        </Pane>
      </div>
    </div>
  );
}

/* ── ISFR Rate Chart (canvas sparkline) ──────────────────── */

interface IsfrRateChartProps {
  data: number[];
  height?: number;
}

function IsfrRateChart({ data, height = 140 }: IsfrRateChartProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useCanvasSetup(canvasRef, (ctx, w, h) => {
    const pad = { top: 16, right: 12, bottom: 12, left: 44 };
    const plotW = w - pad.left - pad.right;
    const plotH = h - pad.top - pad.bottom;

    ctx.clearRect(0, 0, w, h);

    if (data.length < 2) {
      ctx.fillStyle = getCssVar('--text-ghost');
      ctx.font = '10px "JetBrains Mono", monospace';
      ctx.fillText('Waiting for rate data...', pad.left, pad.top + 20);
      return;
    }

    const min = Math.min(...data) - 10;
    const max = Math.max(...data) + 10;
    const range = max - min || 1;

    // Y-axis grid lines
    ctx.strokeStyle = 'rgba(255,255,255,0.05)';
    ctx.lineWidth = 1;
    ctx.fillStyle = getCssVar('--text-ghost');
    ctx.font = '9px "JetBrains Mono", monospace';
    ctx.textAlign = 'right';
    for (let i = 0; i <= 3; i++) {
      const yy = pad.top + plotH * (1 - i / 3);
      ctx.beginPath();
      ctx.moveTo(pad.left, yy);
      ctx.lineTo(pad.left + plotW, yy);
      ctx.stroke();
      const label = Math.round(min + (range * i) / 3);
      ctx.fillText(`${label}`, pad.left - 6, yy + 3);
    }

    // Gradient fill under line
    const lineColor = getCssVar('--rose-glow');
    ctx.beginPath();
    for (let i = 0; i < data.length; i++) {
      const x = pad.left + (i / (data.length - 1)) * plotW;
      const y = pad.top + plotH - ((data[i] - min) / range) * plotH;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.lineTo(pad.left + plotW, pad.top + plotH);
    ctx.lineTo(pad.left, pad.top + plotH);
    ctx.closePath();
    ctx.fillStyle = hexToRgba(lineColor, 0.08);
    ctx.fill();

    // Main line
    ctx.beginPath();
    ctx.strokeStyle = lineColor;
    ctx.lineWidth = 2;
    ctx.lineJoin = 'round';
    ctx.lineCap = 'round';
    for (let i = 0; i < data.length; i++) {
      const x = pad.left + (i / (data.length - 1)) * plotW;
      const y = pad.top + plotH - ((data[i] - min) / range) * plotH;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.stroke();

    // Endpoint dot with glow
    const lastX = pad.left + plotW;
    const lastY = pad.top + plotH - ((data[data.length - 1] - min) / range) * plotH;
    ctx.beginPath();
    ctx.arc(lastX, lastY, 3, 0, Math.PI * 2);
    ctx.fillStyle = lineColor;
    ctx.shadowColor = hexToRgba(lineColor, 0.45);
    ctx.shadowBlur = 8;
    ctx.fill();
    ctx.shadowBlur = 0;
    ctx.shadowColor = 'transparent';

    // Latest value label
    ctx.fillStyle = lineColor;
    ctx.font = '10px "JetBrains Mono", monospace';
    ctx.textAlign = 'right';
    ctx.fillText(`${data[data.length - 1]} bps`, w - pad.right, pad.top - 2);
  }, [data]);

  return (
    <div className="chart-container" style={{ height }}>
      <canvas ref={canvasRef} className="chart-canvas" role="img" aria-label="ISFR composite rate history" />
    </div>
  );
}

/* ── Helpers ─────────────────────────────────────────────── */

function getCssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || '#e8b5ce';
}

function hexToRgba(color: string, alpha: number): string {
  // Handle hex
  if (color.startsWith('#')) {
    const hex = color.slice(1);
    const r = parseInt(hex.slice(0, 2), 16);
    const g = parseInt(hex.slice(2, 4), 16);
    const b = parseInt(hex.slice(4, 6), 16);
    return `rgba(${r},${g},${b},${alpha})`;
  }
  // Handle rgb(r,g,b)
  const match = color.match(/(\d+),\s*(\d+),\s*(\d+)/);
  if (match) return `rgba(${match[1]},${match[2]},${match[3]},${alpha})`;
  return `rgba(200,150,180,${alpha})`;
}
```

### Step 2: Create `src/pages/dashboard/IsfrDashboard.css`

```css
/* ISFR Dashboard page styles */

.isfr-confidence-meter {
  padding: var(--sp-2) 0;
}

.isfr-confidence-track {
  height: 8px;
  background: var(--glass-1);
  border-radius: var(--radius-full);
  overflow: hidden;
}

.isfr-confidence-fill {
  height: 100%;
  border-radius: var(--radius-full);
  transition: width 0.8s cubic-bezier(0.22, 1, 0.36, 1), background 0.4s ease;
  box-shadow: 0 0 12px color-mix(in srgb, currentColor 40%, transparent);
}

.isfr-confidence-labels {
  display: flex;
  justify-content: space-between;
  margin-top: var(--sp-1);
  font-family: var(--mono, var(--font-mono));
  font-size: var(--text-xs);
  color: var(--text-ghost);
}

/* Source health table */

.isfr-source-table {
  display: flex;
  flex-direction: column;
  gap: 0;
  font-size: var(--text-sm);
}

.isfr-source-header {
  display: grid;
  grid-template-columns: 2fr 1fr 0.7fr 1fr 1.2fr;
  gap: var(--sp-2);
  padding: var(--sp-1) var(--sp-2);
  font-family: var(--mono, var(--font-mono));
  font-size: var(--text-xs);
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-ghost);
  border-bottom: 1px solid var(--glass-2-border);
}

.isfr-source-row {
  display: grid;
  grid-template-columns: 2fr 1fr 0.7fr 1fr 1.2fr;
  gap: var(--sp-2);
  padding: var(--sp-1) var(--sp-2);
  align-items: center;
  border-bottom: 1px solid var(--glass-1);
  transition: background 0.15s ease;
}

.isfr-source-row:hover {
  background: var(--glass-1);
}

.isfr-source-row:last-child {
  border-bottom: none;
}

.isfr-source-name {
  color: var(--text-primary);
  font-weight: 500;
}

.isfr-source-class {
  color: var(--text-soft);
  font-family: var(--mono, var(--font-mono));
  font-size: var(--text-xs);
  text-transform: uppercase;
}

.isfr-source-weight {
  color: var(--bone-bright);
  font-family: var(--mono, var(--font-mono));
}

.isfr-source-rate {
  color: var(--rose-bright);
}

.isfr-source-health {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--text-soft);
  font-family: var(--mono, var(--font-mono));
  font-size: var(--text-xs);
}

.isfr-source-empty {
  padding: var(--sp-4);
  text-align: center;
  color: var(--text-ghost);
  font-style: italic;
}
```

### Step 3: Register route in `src/main.tsx`

Add lazy import (after the existing dashboard page imports, around line 30):

```typescript
const IsfrDashboard = lazy(() => import('./pages/dashboard/IsfrDashboard'));
```

Add route (inside the `<Route path="dashboard" ...>` block, after the existing child routes):

```tsx
<Route path="isfr" element={<IsfrDashboard />} />
```

### Step 4: Add nav entry in `src/pages/dashboard/Layout.tsx`

Add to the `VIEWS` array:

```typescript
{ to: '/dashboard/isfr', label: 'ISFR', icon: 'rate', end: false },
```

**Note**: The `icon` value should match a `FlatIconName` that exists in the FlatIcon component.
If `'rate'` is not available, use `'route'` or `'event'` as a fallback — check:
```bash
grep -n "rate\|trend\|chart" demo/demo-app/src/components/FlatIcon.tsx | head -10
```

## Design Decisions

1. **REST + SSE pattern** (matches CostDashboard): `useLiveApi.get()` on mount + 30s fallback poll.
   SSE events `isfr_rate_computed` and `isfr_source_health` trigger debounced refetch.

2. **Mosaic cells for top stats**: Keeper status, composite rate (animated count-up), confidence,
   sources count, current epoch. Same 5-column Mosaic as CostDashboard uses 6.

3. **Pane component for sections**: Rate breakdown (inner 4-col Mosaic with sparklines),
   rate history (canvas chart), confidence meter (styled bar), source health (table).

4. **Canvas sparkline**: Uses `useCanvasSetup` from hooks (handles DPR, resize observer).
   Same approach as `CFactorSparkline` and `CostChart`.

5. **Phosphor flash**: Value changes animate with the flash class, matching the design system's
   `phosphor-flash` CSS animation.

6. **No Tauri, no invoke**: All data via `useLiveApi().get('/api/isfr/...')`.

## Verification

```bash
cd demo/demo-app && npm run build
# Or dev mode: npm run dev
# Navigate to http://localhost:5173/dashboard/isfr
# Start roko serve + keeper → page shows live rates
# Verify SSE updates trigger refetch (watch network tab)
```

## Dependencies

- F1 (roko-serve ISFR REST endpoints)
- F2 (SSE event emission for `isfr_rate_computed`, `isfr_source_health`)
- F3 (creates `src/lib/isfr-api.ts` with types + functions, wires ISFR events into DataHub)

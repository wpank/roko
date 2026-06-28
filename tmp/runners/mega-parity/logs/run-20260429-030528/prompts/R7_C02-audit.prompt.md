# AUDIT: Batch R7_C02 — Dashboard renders seed data as seed data mode

Run: run-20260429-030528 | Phase: AUDIT | Model: codex-5.5

## Your Role

You are an **auditor**. A fast implementation agent just completed batch `R7_C02`.
Your job is to verify correctness and fix any issues — do NOT rewrite from scratch.

## Audit Checklist

1. **Compiles:** `cargo check -p <crate>` for each crate touched by this batch
2. **Clippy clean:** `cargo clippy -p <crate> --no-deps -- -D warnings`
3. **Prompt compliance:** Compare the implementation against the original prompt below
4. **No regressions:** Changed files don't break existing functionality
5. **Anti-patterns:** No stubs that silently pass, no inline prompts, no raw CLI spawns
6. **Correct types:** Field names, method signatures, and imports match the actual codebase
7. **Tests pass:** If the prompt required tests, verify they pass

## If You Find Issues

Fix them directly in the files. Then run the verification commands from the prompt.
If you cannot fix an issue, leave a comment in the file explaining why.

## Scope

Only touch files in the batch's write scope. Do NOT refactor unrelated code.

---

## Original Implementation Prompt

## Task
Dashboard renders seed data as seed data mode

## Runner Context
You are working in runner `mega-parity`, batch R7_C02.
This batch is part of Runner 7: mori-polish — Complete remaining Mori-like UX polish after core contracts stable.
Depends on R7_C01 which adds `source: "seed"` to all seeded records.

## Problem
The dashboard cannot distinguish between seed data and real data. When all data is seeded, it should
show a "SEED DATA" indicator so users know they're looking at demo content, not their own runs.

## Architecture Contract
- Truth before appearance: detect seed mode from the actual data, never from a flag
- All entries have `source:"seed"` → show "SEED DATA" badge
- Mixed (some seed, some real) → show "LIVE" indicator
- All real (no `source:"seed"`) → show "LIVE" indicator or no indicator
- Badge must be visible but not distracting
- Does not change the data display itself, only adds the mode indicator
- Does not change the R1 demo/fallback behavior

## Files to Modify

### 1. `/Users/will/dev/nunchi/roko/roko/demo/demo-app/src/hooks/useApiWithFallback.ts`

Current file (lines 1–81):
```typescript
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useApi } from './useApi';
import { SERVE_URL } from '../lib/serve-url';
import * as Demo from '../lib/demo-data';
import * as BenchDemo from '../lib/bench-demo-data';

function getFallback(path: string): unknown { ... }

let _serverLive: boolean | null = null;
let _probePromise: Promise<void> | null = null;

function probeServer(): Promise<void> { ... }

export function useApiWithFallback() {
  const api = useApi();
  const [isLive, setIsLive] = useState(_serverLive === true);

  useEffect(() => {
    probeServer().then(() => {
      setIsLive(_serverLive === true);
    });
  }, []);

  const get = useCallback(async <T = unknown>(path: string): Promise<T> => { ... }, [api]);
  const post = useCallback(async <T = unknown>(path: string, body?: unknown): Promise<T> => { ... }, [api]);

  return useMemo(() => ({ get, post, baseUrl: api.baseUrl, isLive }), [get, post, api.baseUrl, isLive]);
}
```

The hook currently returns `{ get, post, baseUrl, isLive }`.

#### What to add

Add a `dataMode` field to the returned object: `'seed' | 'live' | 'unknown'`.

Detection logic:
1. After each successful `get()` call that returns an array, check whether every item has `source === 'seed'`.
2. Maintain a module-level tally: `_seedCount` and `_nonSeedCount`.
3. Derive `dataMode`:
   - If `_nonSeedCount === 0 && _seedCount > 0` → `'seed'`
   - If `_nonSeedCount > 0` → `'live'`
   - Otherwise → `'unknown'`

**Exact diff to apply:**

After the `_probePromise` variable declaration (line 30), add:
```typescript
// Tally of seed vs non-seed records observed across all API responses.
let _seedCount = 0;
let _nonSeedCount = 0;

function tallySourceField(data: unknown): void {
  if (!Array.isArray(data)) return;
  for (const item of data) {
    if (item && typeof item === 'object' && 'source' in item) {
      if ((item as Record<string, unknown>).source === 'seed') {
        _seedCount += 1;
      } else {
        _nonSeedCount += 1;
      }
    }
  }
}

function deriveDataMode(): 'seed' | 'live' | 'unknown' {
  if (_nonSeedCount === 0 && _seedCount > 0) return 'seed';
  if (_nonSeedCount > 0) return 'live';
  return 'unknown';
}
```

Inside `useApiWithFallback()`, add a `dataMode` state:
```typescript
const [dataMode, setDataMode] = useState<'seed' | 'live' | 'unknown'>('unknown');
```

In the `get` callback, after a successful result, call `tallySourceField` and update state:
```typescript
const get = useCallback(async <T = unknown>(path: string): Promise<T> => {
  if (_serverLive === false) {
    const result = getFallback(path) as T;
    tallySourceField(result);
    setDataMode(deriveDataMode());
    return result;
  }
  try {
    const data = await api.get<T>(path);
    tallySourceField(data);
    setDataMode(deriveDataMode());
    return data;
  } catch {
    const result = getFallback(path) as T;
    tallySourceField(result);
    setDataMode(deriveDataMode());
    return result;
  }
}, [api, setDataMode]);
```

Change the return to include `dataMode`:
```typescript
return useMemo(
  () => ({ get, post, baseUrl: api.baseUrl, isLive, dataMode }),
  [get, post, api.baseUrl, isLive, dataMode]
);
```

Update the exported type (the inferred return type of `useApiWithFallback`): the `dataMode` field
will be automatically inferred by TypeScript as `'seed' | 'live' | 'unknown'`.

### 2. `/Users/will/dev/nunchi/roko/roko/demo/demo-app/src/components/AppShell.tsx`

Current file (lines 1–38):
```tsx
import { useEffect } from 'react';
import { Outlet } from 'react-router';
import Grain from './Grain';
import HeroParticleField from './HeroParticleField';
import Curtain from './Curtain';
import ScrollTrack from './ScrollTrack';
import TopNav from './TopNav';

export default function AppShell() {
  useEffect(() => {
    const io = new IntersectionObserver(
      (entries) => {
        entries.forEach((e) => {
          if (e.isIntersecting) {
            e.target.classList.add('in');
            io.unobserve(e.target);
          }
        });
      },
      { threshold: 0.18 },
    );
    document.querySelectorAll('.reveal').forEach((el) => io.observe(el));
    return () => io.disconnect();
  }, []);

  return (
    <>
      <Grain />
      <HeroParticleField />
      <Curtain />
      <ScrollTrack />
      <TopNav />
      <div className="app-frame" style={{ paddingTop: 48, position: 'relative', zIndex: 1, minHeight: '100vh' }}>
        <Outlet />
      </div>
    </>
  );
}
```

#### What to add

1. Import `useApiWithFallback` at the top.
2. Call it to get `dataMode`.
3. Render a badge when `dataMode === 'seed'`.

**Exact replacement:**

```tsx
import { useEffect } from 'react';
import { Outlet } from 'react-router';
import Grain from './Grain';
import HeroParticleField from './HeroParticleField';
import Curtain from './Curtain';
import ScrollTrack from './ScrollTrack';
import TopNav from './TopNav';
import { useApiWithFallback } from '../hooks/useApiWithFallback';

export default function AppShell() {
  const { dataMode } = useApiWithFallback();

  useEffect(() => {
    const io = new IntersectionObserver(
      (entries) => {
        entries.forEach((e) => {
          if (e.isIntersecting) {
            e.target.classList.add('in');
            io.unobserve(e.target);
          }
        });
      },
      { threshold: 0.18 },
    );
    document.querySelectorAll('.reveal').forEach((el) => io.observe(el));
    return () => io.disconnect();
  }, []);

  return (
    <>
      <Grain />
      <HeroParticleField />
      <Curtain />
      <ScrollTrack />
      <TopNav />
      {dataMode === 'seed' && (
        <div
          style={{
            position: 'fixed',
            top: 52,
            right: 12,
            zIndex: 9000,
            background: 'rgba(255, 200, 0, 0.15)',
            border: '1px solid rgba(255, 200, 0, 0.4)',
            borderRadius: 4,
            padding: '2px 8px',
            fontSize: 11,
            fontFamily: 'monospace',
            color: 'rgba(255, 200, 0, 0.9)',
            letterSpacing: '0.08em',
            pointerEvents: 'none',
            userSelect: 'none',
          }}
        >
          SEED DATA
        </div>
      )}
      <div className="app-frame" style={{ paddingTop: 48, position: 'relative', zIndex: 1, minHeight: '100vh' }}>
        <Outlet />
      </div>
    </>
  );
}
```

## Step-by-Step Instructions

1. Open `/Users/will/dev/nunchi/roko/roko/demo/demo-app/src/hooks/useApiWithFallback.ts`.
   - After line 30 (the `_probePromise` declaration), insert the `_seedCount`, `_nonSeedCount`,
     `tallySourceField`, and `deriveDataMode` declarations shown above.
   - Inside `useApiWithFallback()`, add `const [dataMode, setDataMode] = useState<'seed' | 'live' | 'unknown'>('unknown');` after the `isLive` state.
   - Replace the `get` callback body to call `tallySourceField` and `setDataMode` on every path.
   - Add `dataMode` to the returned `useMemo` object and its dependency array.

2. Open `/Users/will/dev/nunchi/roko/roko/demo/demo-app/src/components/AppShell.tsx`.
   - Replace the entire file contents with the new version shown above.

3. Run the TypeScript check:
   ```bash
   cd /Users/will/dev/nunchi/roko/roko/demo/demo-app && npx tsc --noEmit
   ```

## Acceptance Criteria
- [ ] `useApiWithFallback` returns `dataMode: 'seed' | 'live' | 'unknown'`
- [ ] `dataMode` is `'seed'` when all fetched records have `source === 'seed'`
- [ ] `dataMode` is `'live'` when any fetched record lacks `source === 'seed'`
- [ ] `AppShell` shows a "SEED DATA" badge when `dataMode === 'seed'`
- [ ] Badge is fixed-position, does not obscure content, pointer-events: none
- [ ] Badge is NOT shown when `dataMode === 'live'` or `'unknown'`
- [ ] Does not change R1 fallback behavior (`getFallback` still returns the same data)
- [ ] `npx tsc --noEmit` passes with zero errors

## Verification
```bash
cd /Users/will/dev/nunchi/roko/roko/demo/demo-app && npx tsc --noEmit
```

## Do NOT
- Remove seed data from display (only add an indicator, not filter)
- Change what `getFallback` returns
- Change the `isLive` behavior
- Add complex data filtering or transformation
- Use a feature flag or env var to detect seed mode (use data content only)

---

## Current Implementation (as written by implementation agent)

### `demo/demo-app/src/hooks/useApiWithFallback.ts`

```rust
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useApi } from './useApi';
import { SERVE_URL } from '../lib/serve-url';
import * as Demo from '../lib/demo-data';
import * as BenchDemo from '../lib/bench-demo-data';

// Map API paths to demo fallback data (used only when offline or endpoint returns nothing)
function getFallback(path: string): unknown {
  if (path.includes('/health') && !path.includes('/providers/health')) return Demo.DEMO_HEALTH;
  if (path.includes('/managed-agents')) return Demo.DEMO_AGENTS;
  if (path.includes('/agents/topology')) return Demo.DEMO_AGENT_TOPOLOGY;
  if (path.includes('/knowledge/entries')) return Demo.DEMO_KNOWLEDGE_ENTRIES;
  if (path.includes('/knowledge/edges')) return Demo.DEMO_KNOWLEDGE_EDGES;
  if (path.includes('/episodes')) return Demo.DEMO_EPISODES;
  if (path.includes('/learn/efficiency')) return Demo.DEMO_EFFICIENCY;
  if (path.includes('/metrics/c_factor')) return Demo.DEMO_CFACTOR;
  if (path.includes('/c-factor/trend')) return Demo.DEMO_CFACTOR_TREND;
  if (path.includes('/learn/cascade-router')) return Demo.DEMO_ROUTER_MODELS;
  if (path.includes('/gates/summary')) return Demo.DEMO_GATES_SUMMARY;
  if (path.includes('/gates/history')) return Demo.DEMO_GATE_HISTORY;
  if (path.includes('/learn/adaptive-thresholds')) return Demo.DEMO_ADAPTIVE_THRESHOLDS;
  if (path.includes('/status')) return Demo.DEMO_STATUS;
  if (path.includes('/statehub/events')) return Demo.DEMO_EVENTS;
  if (path.includes('/dashboard')) return Demo.DEMO_DASHBOARD;
  if (path.includes('/learn/provider-outcomes') || path.includes('/providers/health')) return Demo.DEMO_PROVIDER_HEALTH;
  if (path.includes('/cost-race') || path.includes('/bench/cost-summary')) return Demo.DEMO_COST_RACE;
  if (path.includes('/bench/suites')) return BenchDemo.DEMO_BENCH_SUITES;
  if (path.includes('/bench/models')) return BenchDemo.DEMO_BENCH_MODELS;
  if (path.includes('/bench/runs')) return BenchDemo.DEMO_BENCH_RUNS;
  if (path.includes('/dream/journal')) return Demo.DEMO_DREAM_JOURNAL;
  if (path.includes('/share/')) return null;
  return {};
}

// Probe: is the server reachable at all? That's the only question.
let _serverLive: boolean | null = null; // null = unknown
let _probePromise: Promise<void> | null = null;

// Tally of seed vs non-seed records observed across all API responses.
let _seedCount = 0;
let _nonSeedCount = 0;

function tallySourceField(data: unknown): void {
  if (!Array.isArray(data)) return;
  for (const item of data) {
    if (item && typeof item === 'object' && 'source' in item) {
      if ((item as Record<string, unknown>).source === 'seed') {
        _seedCount += 1;
      } else {
        _nonSeedCount += 1;
      }
    }
  }
}

function deriveDataMode(): 'seed' | 'live' | 'unknown' {
  if (_nonSeedCount === 0 && _seedCount > 0) return 'seed';
  if (_nonSeedCount > 0) return 'live';
  return 'unknown';
}

function probeServer(): Promise<void> {
  if (_probePromise) return _probePromise;
  _probePromise = (async () => {
    try {
      const res = await fetch(`${SERVE_URL}/health`, { signal: AbortSignal.timeout(2000) });
      _serverLive = res.ok;
    } catch {
      _serverLive = false;
    }
  })();
  return _probePromise;
}

export function useApiWithFallback() {
  const api = useApi();
  const [isLive, setIsLive] = useState(_serverLive === true);
  const [dataMode, setDataMode] = useState<'seed' | 'live' | 'unknown'>('unknown');

  useEffect(() => {
    probeServer().then(() => {
      setIsLive(_serverLive === true);
    });
  }, []);

  const get = useCallback(async <T = unknown>(path: string): Promise<T> => {
    // Offline → straight to fallback
    if (_serverLive === false) {
      const result = getFallback(path) as T;
      tallySourceField(result);
      setDataMode(deriveDataMode());
      return result;
    }

    // Server is live (or probe hasn't finished) — always try the real API
    try {
      const data = await api.get<T>(path);
      tallySourceField(data);
      setDataMode(deriveDataMode());
      return data;
    } catch {
      // Network error or 4xx/5xx → fallback for this endpoint
      const result = getFallback(path) as T;
      tallySourceField(result);
      setDataMode(deriveDataMode());
      return result;
    }
  }, [api, setDataMode]);

  const post = useCallback(async <T = unknown>(path: string, body?: unknown): Promise<T> => {
    if (_serverLive === false) return {} as T;
    try {
      return await api.post<T>(path, body);
    } catch {
      return {} as T;
    }
  }, [api]);

  return useMemo(
    () => ({ get, post, baseUrl: api.baseUrl, isLive, dataMode }),
    [get, post, api.baseUrl, isLive, dataMode],
  );
}
```

### `demo/demo-app/src/components/AppShell.tsx`

```rust
import { useEffect } from 'react';
import { Outlet } from 'react-router';
import Grain from './Grain';
import HeroParticleField from './HeroParticleField';
import Curtain from './Curtain';
import ScrollTrack from './ScrollTrack';
import TopNav from './TopNav';
import { useApiWithFallback } from '../hooks/useApiWithFallback';

export default function AppShell() {
  const { dataMode } = useApiWithFallback();

  useEffect(() => {
    const io = new IntersectionObserver(
      (entries) => {
        entries.forEach((e) => {
          if (e.isIntersecting) {
            e.target.classList.add('in');
            io.unobserve(e.target);
          }
        });
      },
      { threshold: 0.18 },
    );
    document.querySelectorAll('.reveal').forEach((el) => io.observe(el));
    return () => io.disconnect();
  }, []);

  return (
    <>
      <Grain />
      <HeroParticleField />
      <Curtain />
      <ScrollTrack />
      <TopNav />
      {dataMode === 'seed' && (
        <div
          style={{
            position: 'fixed',
            top: 52,
            right: 12,
            zIndex: 9000,
            background: 'rgba(255, 200, 0, 0.15)',
            border: '1px solid rgba(255, 200, 0, 0.4)',
            borderRadius: 4,
            padding: '2px 8px',
            fontSize: 11,
            fontFamily: 'monospace',
            color: 'rgba(255, 200, 0, 0.9)',
            letterSpacing: '0.08em',
            pointerEvents: 'none',
            userSelect: 'none',
          }}
        >
          SEED DATA
        </div>
      )}
      <div className="app-frame" style={{ paddingTop: 48, position: 'relative', zIndex: 1, minHeight: '100vh' }}>
        <Outlet />
      </div>
    </>
  );
}
```

---

## Read-Only Context (do not modify)

### `crates/roko-cli/src/demo_seed.rs` (1696 lines — signatures only)

```rust
29:pub struct DemoSeedReport {
31:    pub seeded_groups: Vec<String>,
33:    pub skipped_groups: Vec<String>,
36:impl DemoSeedReport {
39:    pub fn any_seeded(&self) -> bool {
45:    pub fn summary(&self) -> String {
81:struct DemoTaskSpec {
105:struct DemoKnowledgeEntrySpec {
127:struct SeedEnvelopeRef<'a, T> {
135:struct SeededCascadeObservation {
148:struct SeededCascadeModelStats {
155:struct SeededCascadeSnapshot {
166:pub fn seed_demo_workspace(workdir: impl AsRef<Path>, config: Option<&RokoConfig>) -> Result<DemoSeedReport> {
269:fn demo_task_specs() -> Vec<DemoTaskSpec> {
384:fn demo_knowledge_entry_specs() -> Vec<DemoKnowledgeEntrySpec> {
541:fn build_episodes(
620:fn build_efficiency_events(
656:fn build_task_metrics(
705:fn build_cfactor_snapshots(specs: &[DemoTaskSpec], now: DateTime<Utc>) -> Vec<CFactor> {
818:fn build_knowledge_seeds(
873:fn build_knowledge_entries(
919:fn build_cascade_snapshot(
958:fn demo_router_observations(
1003:fn demo_role_table(model_pool: &[String]) -> HashMap<AgentRole, String> {
1018:fn build_efficiency_event(
1105:fn build_cfactor_snapshot(
1123:fn prompt_sections_for_task(spec: &DemoTaskSpec, primary: bool) -> Vec<PromptSectionMeta> {
1157:fn prompt_sections_json(spec: &DemoTaskSpec, primary: bool) -> Vec<Value> {
1173:fn existing_episode_ids(workdir: &Path) -> Vec<String> {
1204:fn episode_ids_for_indices(episode_ids: &[String], indices: &[usize]) -> Vec<String> {
1217:fn write_jsonl_group_if_absent<T>(
1245:fn write_jsonl_if_absent<T>(
1267:fn write_json_if_absent<T>(
1286:fn jsonl_payload<T>(
1298:fn seeded_jsonl_line<T: Serialize>(item: &T) -> Result<String> {
1306:fn config_hash(config: Option<&RokoConfig>) -> ConfigHash {
1313:fn demo_model_pool(config: Option<&RokoConfig>) -> Vec<String> {
1358:fn push_model(pool: &mut Vec<String>, seen: &mut HashSet<String>, model: String) {
1368:fn model_for_slot(model_pool: &[String], slot: usize) -> String {
1373:fn backend_for_model(model: &str) -> &'static str {
```

---

## Verification Commands

Run these and fix any failures:
```bash
cargo check --workspace
cargo clippy --workspace --no-deps -- -D warnings
```

## Do NOT

- Rewrite the entire implementation from scratch
- Add features not in the original prompt
- Modify files outside the write scope
- Skip running verification commands

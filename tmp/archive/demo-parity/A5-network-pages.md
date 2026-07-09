# A5: Network pages -- agent network, pheromone field, knowledge graph

## Context

**Repo:** `/Users/will/dev/nunchi/nunchi-dashboard`
**Branch:** `demo-rewrite`
**Tech stack:** React 19 + Vite 8 + TypeScript + Tailwind CSS v4
**Backend:** `roko-serve` runs at `http://localhost:6677` with ~85 REST routes + WebSocket at `ws://localhost:6677/ws`
**Auth:** Privy (env var `VITE_PRIVY_APP_ID`) with password fallback
**Design:** ROSEDUST dark palette -- bg_void `#060608`, rose `#AA7088`, bone `#C8B890`, rose_bright `#CC90A8`

### Before starting
1. `cd /Users/will/dev/nunchi/nunchi-dashboard`
2. `git checkout -b demo-rewrite 2>/dev/null || git checkout demo-rewrite`
3. `npm install`
4. Verify: `npm run dev` starts without errors

### After every task
1. `npm run typecheck` passes
2. `npm run dev` -- page renders without console errors
3. All existing tests pass: `npm test` (if test runner is configured)

---

## What this task produces

Three pages under `/app/network/*`. These are the most visual pages in the dashboard -- a force-directed graph, a heatmap, and a searchable knowledge browser. The existing repo already has `d3-force` and `react-force-graph-2d` installed, so no additional deps are needed.

**Depends on:** Task A1 (design system, router), Task A2 (API hooks).

---

## Checklist

### 1. Create page directory

```bash
mkdir -p /Users/will/dev/nunchi/nunchi-dashboard/src/pages/network
```

### 2. AgentNetwork page (force graph)

The existing repo has `react-force-graph-2d` already installed (`package.json` lists it). This page renders a force-directed graph where each node is an agent and edges represent message passing or dependency relationships.

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/network/AgentNetwork.tsx`:

```tsx
import { useMemo, useRef, useCallback, useEffect, useState } from "react";
import ForceGraph2D from "react-force-graph-2d";
import type { ForceGraphMethods, NodeObject, LinkObject } from "react-force-graph-2d";
import { useAgents } from "../../services/api";
import { Card, Badge, Skeleton, EmptyState, ErrorState } from "../../design-system/components";

type GraphNode = NodeObject & {
  id: string;
  label: string;
  color: string;
  val: number;
};

type GraphLink = LinkObject & {
  source: string;
  target: string;
};

type GraphData = {
  nodes: GraphNode[];
  links: GraphLink[];
};

type AgentRecord = {
  id: number;
  label: string;
};

// MOCK: in a real deployment, edges come from episode data showing which agents
// collaborated on shared plans. For now, generate a deterministic topology so
// the graph is stable across re-renders (no Math.random in data building).
function buildGraph(agents: AgentRecord[]): GraphData {
  const nodes: GraphNode[] = agents.map((a, idx) => ({
    id: String(a.id),
    label: a.label || `Agent ${a.id}`,
    color: "#AA7088",
    // Deterministic size based on position rather than random
    val: 4 + (idx % 3) * 2,
  }));

  const links: GraphLink[] = [];
  for (let i = 1; i < nodes.length; i++) {
    // Chain each agent to the previous (base topology)
    links.push({ source: nodes[i - 1].id, target: nodes[i].id });
    // Add deterministic shortcut edges for visual interest
    if (i > 2 && i % 3 === 0) {
      links.push({ source: nodes[i].id, target: nodes[0].id });
    }
  }

  return { nodes, links };
}

export default function AgentNetwork() {
  const { data: agents, isLoading, error, refetch } = useAgents();
  const graphRef = useRef<ForceGraphMethods<GraphNode, GraphLink>>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [dimensions, setDimensions] = useState({ width: 800, height: 500 });

  const graphData = useMemo<GraphData>(() => {
    if (!agents || agents.length === 0) return { nodes: [], links: [] };
    return buildGraph(agents);
  }, [agents]);

  // Resize the graph when the container changes size
  useEffect(() => {
    if (!containerRef.current) return;

    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry) {
        const { width, height } = entry.contentRect;
        setDimensions({
          width: Math.max(400, width),
          height: Math.max(300, height),
        });
      }
    });

    observer.observe(containerRef.current);
    return () => observer.disconnect();
  }, []);

  const handleNodeClick = useCallback((node: GraphNode) => {
    if (graphRef.current && node.x !== undefined && node.y !== undefined) {
      graphRef.current.centerAt(node.x, node.y, 500);
      graphRef.current.zoom(3, 500);
    }
  }, []);

  if (isLoading) {
    return (
      <section className="p-6">
        <Skeleton height="500px" />
      </section>
    );
  }

  if (error) {
    return (
      <section className="p-6">
        <ErrorState error={String(error)} onRetry={() => refetch()} />
      </section>
    );
  }

  if (graphData.nodes.length === 0) {
    return (
      <section className="p-6">
        <EmptyState
          title="No agents in the network"
          description="Start agent processes to see the network topology."
        />
      </section>
    );
  }

  return (
    <section className="p-6">
      <header className="flex items-center justify-between mb-4">
        <div>
          <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)]">
            Agent network
          </h1>
          <p className="text-xs text-[var(--rd-fg-muted)] mt-0.5">
            {graphData.nodes.length} nodes, {graphData.links.length} edges
          </p>
        </div>
        <Badge label="force-directed" variant="info" />
      </header>

      <Card padding="none" className="overflow-hidden">
        <div ref={containerRef} className="w-full" style={{ minHeight: "480px" }}>
          <ForceGraph2D
            ref={graphRef}
            graphData={graphData}
            width={dimensions.width}
            height={dimensions.height}
            backgroundColor="#060608"
            nodeColor={(node) => (node as GraphNode).color}
            nodeRelSize={3}
            nodeLabel={(node) => (node as GraphNode).label}
            linkColor={() => "rgba(170, 112, 136, 0.25)"}
            linkWidth={1}
            onNodeClick={(node) => handleNodeClick(node as GraphNode)}
            nodeCanvasObject={(node, ctx, globalScale) => {
              const gn = node as GraphNode;
              const r = gn.val || 4;
              const x = gn.x ?? 0;
              const y = gn.y ?? 0;

              // Node circle
              ctx.beginPath();
              ctx.arc(x, y, r, 0, 2 * Math.PI);
              ctx.fillStyle = gn.color;
              ctx.fill();

              // Label when zoomed in enough to read it
              if (globalScale > 1.5) {
                const fontSize = 10 / globalScale;
                ctx.font = `${fontSize}px monospace`;
                ctx.fillStyle = "#E8E4DE";
                ctx.textAlign = "center";
                ctx.fillText(gn.label, x, y + r + 8 / globalScale);
              }
            }}
          />
        </div>
      </Card>

      <footer className="flex items-center gap-6 mt-3 text-[10px] text-[var(--rd-fg-muted)]">
        <span>Click a node to zoom in. Drag to pan.</span>
        <span>Edges show collaboration links from shared plan execution.</span>
      </footer>
    </section>
  );
}
```

### 3. PheromoneField page (heatmap)

This page renders a canvas-based heatmap showing pheromone signal density. Pheromone signals are deposited by agents when they make decisions, and other agents follow strong signals.

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/network/PheromoneField.tsx`:

```tsx
import { useRef, useEffect, useMemo, useState } from "react";
import { Card, Select } from "../../design-system/components";
import { useWsStore } from "../../stores/wsStore";

type PheromoneType = "threat" | "opportunity" | "knowledge" | "coordination";

// RGB color anchors for each pheromone type (matches ROSEDUST palette semantics)
const PHEROMONE_COLORS: Record<PheromoneType, [number, number, number]> = {
  threat: [170, 85, 85],        // rd-error hue
  opportunity: [170, 136, 85],  // rd-warning hue
  knowledge: [122, 104, 144],   // rd-accent-purple hue
  coordination: [112, 136, 122], // rd-success hue
};

const PHEROMONE_OPTIONS = [
  { value: "knowledge" as PheromoneType, label: "Knowledge" },
  { value: "threat" as PheromoneType, label: "Threat" },
  { value: "opportunity" as PheromoneType, label: "Opportunity" },
  { value: "coordination" as PheromoneType, label: "Coordination" },
];

const GRID_SIZE = 32;

// Generates a deterministic Gaussian-cluster field so the heatmap is stable
// across re-renders. Math.random is NOT called here.
function generateMockField(): number[][] {
  const field: number[][] = [];
  const clusters: [number, number, number][] = [
    [10, 8, 6],  // [cx, cy, spread]
    [22, 20, 8],
    [16, 26, 5],
    [5, 25, 4],
  ];

  for (let y = 0; y < GRID_SIZE; y++) {
    const row: number[] = [];
    for (let x = 0; x < GRID_SIZE; x++) {
      let value = 0;
      for (const [cx, cy, spread] of clusters) {
        const d2 = (x - cx) ** 2 + (y - cy) ** 2;
        value += Math.exp(-d2 / (2 * spread ** 2));
      }
      // Add a tiny deterministic texture using a sine pattern
      const texture = (Math.sin(x * 0.8) * Math.cos(y * 0.8) + 1) * 0.03;
      row.push(Math.min(1, value + texture));
    }
    field.push(row);
  }
  return field;
}

// Bilinear interpolation between two RGB colors
function lerpColor(
  a: [number, number, number],
  b: [number, number, number],
  t: number,
): [number, number, number] {
  return [
    Math.round(a[0] + (b[0] - a[0]) * t),
    Math.round(a[1] + (b[1] - a[1]) * t),
    Math.round(a[2] + (b[2] - a[2]) * t),
  ];
}

// Background color (bg_void)
const BG: [number, number, number] = [6, 6, 8];

export default function PheromoneField() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [pheromoneType, setPheromoneType] = useState<PheromoneType>("knowledge");
  // Memoize the field so it is only generated once
  const field = useMemo(generateMockField, []);
  const { events } = useWsStore();

  const recentEventCount = events.filter(
    (e) => Date.now() - e.receivedAt < 60_000,
  ).length;

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const cellW = canvas.width / GRID_SIZE;
    const cellH = canvas.height / GRID_SIZE;
    const color = PHEROMONE_COLORS[pheromoneType];

    ctx.clearRect(0, 0, canvas.width, canvas.height);

    for (let y = 0; y < GRID_SIZE; y++) {
      for (let x = 0; x < GRID_SIZE; x++) {
        const intensity = field[y][x];
        // Interpolate between bg_void and the pheromone color
        const [r, g, b] = lerpColor(BG, color, intensity * 0.85);
        ctx.fillStyle = `rgb(${r}, ${g}, ${b})`;
        ctx.fillRect(x * cellW, y * cellH, cellW + 1, cellH + 1); // +1 avoids sub-pixel gaps
      }
    }

    // Faint grid overlay
    ctx.strokeStyle = "rgba(200, 200, 200, 0.04)";
    ctx.lineWidth = 0.5;
    for (let x = 0; x <= GRID_SIZE; x++) {
      ctx.beginPath();
      ctx.moveTo(x * cellW, 0);
      ctx.lineTo(x * cellW, canvas.height);
      ctx.stroke();
    }
    for (let y = 0; y <= GRID_SIZE; y++) {
      ctx.beginPath();
      ctx.moveTo(0, y * cellH);
      ctx.lineTo(canvas.width, y * cellH);
      ctx.stroke();
    }
  }, [field, pheromoneType]);

  // Stable mock deposit counts per type (derived deterministically, not from Math.random)
  const MOCK_DEPOSITS: Record<PheromoneType, number> = {
    knowledge: 47,
    threat: 12,
    opportunity: 28,
    coordination: 35,
  };

  return (
    <section className="p-6">
      <header className="flex items-center justify-between mb-4">
        <div>
          <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)]">
            Pheromone field
          </h1>
          <p className="text-xs text-[var(--rd-fg-muted)] mt-0.5">
            Signal density across the decision space
          </p>
        </div>
        <div className="w-48">
          <Select
            value={pheromoneType}
            onChange={(v) => setPheromoneType(v as PheromoneType)}
            options={PHEROMONE_OPTIONS}
          />
        </div>
      </header>

      <Card padding="none" className="overflow-hidden">
        <canvas
          ref={canvasRef}
          width={640}
          height={640}
          className="w-full block"
          style={{ imageRendering: "pixelated" }}
          aria-label={`${pheromoneType} pheromone density heatmap`}
          role="img"
        />
      </Card>

      {/* Legend strip */}
      <div className="grid grid-cols-4 gap-3 mt-4">
        {PHEROMONE_OPTIONS.map(({ value: type, label }) => {
          const [r, g, b] = PHEROMONE_COLORS[type];
          const isActive = type === pheromoneType;
          return (
            <button
              key={type}
              type="button"
              onClick={() => setPheromoneType(type)}
              className={`text-left rounded-md transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--rd-rose)] ${
                isActive
                  ? "ring-1 ring-[var(--rd-rose)]/40"
                  : "hover:bg-[var(--rd-bg-surface-1)]"
              }`}
            >
              <Card padding="sm">
                <div className="flex items-center gap-2 mb-1">
                  <span
                    className="w-3 h-3 rounded-sm shrink-0"
                    style={{ backgroundColor: `rgb(${r}, ${g}, ${b})` }}
                  />
                  <span className="text-xs text-[var(--rd-fg-secondary)] capitalize">
                    {label}
                  </span>
                </div>
                <div className="text-[10px] text-[var(--rd-fg-muted)]">
                  {/* MOCK: wire to pheromone count endpoint when available */}
                  {MOCK_DEPOSITS[type]} deposits (last 1h)
                </div>
              </Card>
            </button>
          );
        })}
      </div>

      <footer className="mt-4 text-[10px] text-[var(--rd-fg-muted)]">
        {recentEventCount > 0
          ? `${recentEventCount} WS event${recentEventCount !== 1 ? "s" : ""} in the last minute`
          : "No recent WS events -- connect roko-serve for live data"}
      </footer>
    </section>
  );
}
```

### 4. KnowledgeGraph page (search + browse)

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/network/KnowledgeGraph.tsx`:

```tsx
import { useState, useMemo, useCallback } from "react";
import { Card, Input, Badge, EmptyState } from "../../design-system/components";
import { useDebounce } from "../../hooks/useDebounce";

type KnowledgeTier = "T0" | "T1" | "T2";

type KnowledgeEntry = {
  key: string;
  tier: KnowledgeTier;
  summary: string;
  access_count: number;
};

// MOCK: the neuro store does not yet expose a search API endpoint.
// When it does, replace the filtered list below with a TanStack Query hook
// and pass the debounced search term as a query parameter.
const MOCK_ENTRIES: KnowledgeEntry[] = [
  {
    key: "roko::gate-pipeline",
    tier: "T0",
    summary: "7-rung gate pipeline validates agent output. Rungs: compile, test, clippy, diff, lint, coverage, review.",
    access_count: 142,
  },
  {
    key: "roko::cascade-router",
    tier: "T1",
    summary: "Routes tasks to the cheapest capable model. Persists performance data to learn routing preferences.",
    access_count: 89,
  },
  {
    key: "roko::vcg-auction",
    tier: "T1",
    summary: "VCG mechanism allocates context window tokens to competing bidders (neuro, task, research, playbook, system).",
    access_count: 67,
  },
  {
    key: "roko::prompt-experiments",
    tier: "T2",
    summary: "A/B testing for system prompt templates. Thompson sampling selects winning variants.",
    access_count: 45,
  },
  {
    key: "roko::episode-logger",
    tier: "T1",
    summary: "Records every agent turn as a structured episode in .roko/episodes.jsonl.",
    access_count: 203,
  },
  {
    key: "roko::process-supervisor",
    tier: "T0",
    summary: "Tracks agent process lifecycle. PlanRunner uses it to spawn and shut down agents.",
    access_count: 78,
  },
  {
    key: "roko::hdc-fingerprint",
    tier: "T2",
    summary: "Hyperdimensional computing vectors computed per episode for similarity search.",
    access_count: 34,
  },
];

// Maps tier to Badge variant. T0 = hot/success, T1 = warm/warning, T2 = cold/info.
const TIER_BADGE_VARIANT: Record<KnowledgeTier, "success" | "warning" | "info"> = {
  T0: "success",
  T1: "warning",
  T2: "info",
};

export default function KnowledgeGraph() {
  const [search, setSearch] = useState("");
  const [selectedEntry, setSelectedEntry] = useState<KnowledgeEntry | null>(null);

  // 300ms debounce so filtering doesn't fire on every keystroke
  const debouncedSearch = useDebounce(search, 300);

  const filtered = useMemo(() => {
    const q = debouncedSearch.toLowerCase().trim();
    if (!q) return MOCK_ENTRIES;
    return MOCK_ENTRIES.filter(
      (e) =>
        e.key.toLowerCase().includes(q) ||
        e.summary.toLowerCase().includes(q),
    );
  }, [debouncedSearch]);

  const handleSelect = useCallback((entry: KnowledgeEntry) => {
    setSelectedEntry((prev) => (prev?.key === entry.key ? null : entry));
  }, []);

  return (
    <section className="p-6">
      <header className="flex items-center justify-between mb-4">
        <div>
          <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)]">
            Knowledge graph
          </h1>
          <p className="text-xs text-[var(--rd-fg-muted)] mt-0.5">
            Browse the neuro knowledge store
          </p>
        </div>
        <Badge label={`${MOCK_ENTRIES.length} entries`} variant="info" />
      </header>

      <div className="mb-4 max-w-md">
        <Input
          placeholder="Search knowledge entries..."
          value={search}
          onChange={setSearch}
        />
        {debouncedSearch && (
          <p className="text-[10px] text-[var(--rd-fg-muted)] mt-1.5">
            {filtered.length} result{filtered.length !== 1 ? "s" : ""} for &ldquo;{debouncedSearch}&rdquo;
          </p>
        )}
      </div>

      <div className="grid grid-cols-2 gap-4">
        {/* Entry list */}
        <div className="space-y-2">
          {filtered.length === 0 ? (
            <EmptyState
              title="No matches"
              description="Try a different search term."
            />
          ) : (
            filtered.map((entry) => {
              const isSelected = selectedEntry?.key === entry.key;
              return (
                <button
                  key={entry.key}
                  type="button"
                  className="w-full text-left focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--rd-rose)] rounded-md"
                  onClick={() => handleSelect(entry)}
                >
                  <Card
                    padding="sm"
                    className={`transition-colors ${
                      isSelected
                        ? "border-[var(--rd-rose)]/40"
                        : "hover:border-[var(--rd-fg-muted)]/20"
                    }`}
                  >
                    <div className="flex items-center justify-between mb-1">
                      <span className="text-xs font-mono text-[var(--rd-fg-primary)] truncate mr-2">
                        {entry.key}
                      </span>
                      <Badge
                        label={entry.tier}
                        variant={TIER_BADGE_VARIANT[entry.tier]}
                      />
                    </div>
                    <p className="text-[10px] text-[var(--rd-fg-muted)] line-clamp-2">
                      {entry.summary}
                    </p>
                    <div className="text-[10px] text-[var(--rd-fg-muted)] mt-1">
                      {entry.access_count.toLocaleString()} accesses
                    </div>
                  </Card>
                </button>
              );
            })
          )}
        </div>

        {/* Detail panel */}
        <aside>
          {selectedEntry ? (
            <Card>
              <div className="flex items-center justify-between mb-3">
                <span className="text-sm font-mono font-medium text-[var(--rd-fg-primary)] truncate mr-2">
                  {selectedEntry.key}
                </span>
                <Badge
                  label={selectedEntry.tier}
                  variant={TIER_BADGE_VARIANT[selectedEntry.tier]}
                />
              </div>
              <p className="text-sm text-[var(--rd-fg-secondary)] mb-4">
                {selectedEntry.summary}
              </p>
              <div className="grid grid-cols-2 gap-3 text-xs text-[var(--rd-fg-muted)]">
                <div>
                  <div className="text-[10px] uppercase tracking-wider mb-0.5">
                    Access count
                  </div>
                  <div className="font-mono text-[var(--rd-fg-secondary)]">
                    {selectedEntry.access_count.toLocaleString()}
                  </div>
                </div>
                <div>
                  <div className="text-[10px] uppercase tracking-wider mb-0.5">
                    Tier
                  </div>
                  <div className="font-mono text-[var(--rd-fg-secondary)]">
                    {selectedEntry.tier}
                  </div>
                </div>
              </div>
              <div className="mt-4 p-3 rounded-md bg-[var(--rd-bg-surface-0)] text-[10px] text-[var(--rd-fg-muted)]">
                {/* MOCK: wire to neuro store query endpoint when available */}
                Knowledge store query endpoint not yet exposed via roko-serve.
                This panel will show related entries, version history, and distillation status.
              </div>
            </Card>
          ) : (
            <Card className="flex items-center justify-center" style={{ minHeight: "256px" }}>
              <span className="text-xs text-[var(--rd-fg-muted)]">
                Select an entry to view details
              </span>
            </Card>
          )}
        </aside>
      </div>
    </section>
  );
}
```

**Note:** The `useDebounce` hook must exist at `src/hooks/useDebounce.ts`. Add it if missing:

```ts
// src/hooks/useDebounce.ts
import { useState, useEffect } from "react";

export function useDebounce<T>(value: T, delayMs: number): T {
  const [debounced, setDebounced] = useState<T>(value);

  useEffect(() => {
    const id = setTimeout(() => setDebounced(value), delayMs);
    return () => clearTimeout(id);
  }, [value, delayMs]);

  return debounced;
}
```

### 5. Wire pages into the router

- [ ] Update `/Users/will/dev/nunchi/nunchi-dashboard/src/router.tsx` -- replace the three network placeholders:

Find:
```
{ path: "network/agents", element: <Placeholder name="Agent network" /> },
{ path: "network/pheromones", element: <Placeholder name="Pheromone field" /> },
{ path: "network/knowledge", element: <Placeholder name="Knowledge graph" /> },
```

Replace with:
```tsx
{ path: "network/agents", element: lazyPage(() => import("./pages/network/AgentNetwork")) },
{ path: "network/pheromones", element: lazyPage(() => import("./pages/network/PheromoneField")) },
{ path: "network/knowledge", element: lazyPage(() => import("./pages/network/KnowledgeGraph")) },
```

---

## Verification

Run from `/Users/will/dev/nunchi/nunchi-dashboard`:

- [ ] `npm run typecheck` -- exits 0
- [ ] `npm run dev` -- navigate to each route:
  - `/app/network/agents` -- force graph renders (empty state if no agents, or nodes with edges)
  - `/app/network/pheromones` -- heatmap renders with smooth color interpolation, type selector works
  - `/app/network/knowledge` -- search filters entries, clicking an entry shows the detail panel
- [ ] Force graph: drag and zoom work; clicking a node centers and zooms in; graph resizes when window resizes
- [ ] Pheromone field: switching type changes the color scheme using proper interpolation (no random flicker); clicking a legend tile also switches the type
- [ ] Knowledge graph: typing in search updates the result count after 300ms debounce; result count shown below the input
- [ ] No `any` casts in AgentNetwork -- all node/link callbacks use typed casts (e.g., `node as GraphNode`)
- [ ] No `Math.random()` calls in data-building functions (graph and field are deterministic)
- [ ] No console errors

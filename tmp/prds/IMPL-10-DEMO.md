# IMPL-10: Minimum viable demo -- Thursday April 24, 2026

**Deadline:** Thursday April 24, 2026 (3 working days from Monday April 21)
**Execution method:** Claude agents running in parallel across two repos + one crate workspace
**Goal:** A person sits down, lands on the Nunchi dashboard, creates a bounty, watches an agent do the work, and sees results -- all live, all real data where possible, all mock data clearly tagged where not.

---

## Demo success criteria

The demo passes when all six of these hold:

1. A visitor lands on the redesigned Nunchi dashboard and understands the system within two minutes via an interactive landing page.
2. Every page in the dashboard renders. Pages backed by unfinished APIs show mock data with visible `MOCK:` tags in source.
3. A visitor creates a research bounty from the dashboard. A roko agent picks it up and delivers a report visible in the job detail view.
4. A visitor creates a coding bounty from the dashboard. A roko agent produces code that passes the gate pipeline. Results appear in the job detail view.
5. Live agent activity is visible: cognitive frequency, turn log, heartbeat status, and cost metrics.
6. `roko plan run` executes multi-task plans reliably enough for dogfooding (no silent failures, no lost state on restart).

---

## Repositories and paths

| Repo | Absolute path | Branch |
|---|---|---|
| nunchi-dashboard | `/Users/will/dev/nunchi/nunchi-dashboard` | `demo-rewrite` |
| roko | `/Users/will/dev/nunchi/roko/roko` | `demo-backend` |

---

## Three parallel streams

| Stream | Repo | Scope | Days |
|---|---|---|---|
| **A -- Dashboard complete rewrite** | nunchi-dashboard | Gut and rebuild the React app from scratch | 1-3 |
| **B -- Roko backend stabilization + jobs** | roko | Job types, routes, execution, heartbeats, file watchers | 1-3 |
| **C -- TUI enhancements** | roko | New tabs (F8, F9), sub-views, bug fixes | 1-3 |

Streams A, B, and C run fully in parallel. Tasks within each stream run sequentially unless marked **(parallel-safe)**. Stream C can share agents with Stream B since both operate on the roko workspace.

Day boundaries:
- **Day 1 (Tue Apr 22):** Foundation. Dashboard scaffolding + design system. Backend job types + store + routes. TUI new tabs.
- **Day 2 (Wed Apr 23):** Pages. All dashboard pages built. Job execution pipeline wired. TUI sub-views + bug fixes.
- **Day 3 (Thu Apr 24):** Integration. Real data flowing. Polish. End-to-end demo rehearsal.

---

## Mock data rules

Every mock value in every file must carry this exact comment format:

```tsx
// MOCK: wire to GET /api/jobs — depends on task B2
```

```rust
// MOCK: replace with data from HeartbeatAggregator — depends on task B7
```

The comment includes: (1) the `MOCK:` prefix, (2) what to wire it to, (3) which task produces the real data. This makes `grep -rn "MOCK:" src/` a complete integration checklist.

---

## Prerequisites and environment

### Dashboard repo
- **Path:** `/Users/will/dev/nunchi/nunchi-dashboard`
- **Branch:** Create `demo-rewrite` from current HEAD
- **Start dev server:** `npm run dev` (serves on localhost:5174)
- **TypeScript check:** `npx tsc --noEmit`
- **Entry point:** `src/main.jsx` (JSX, not TSX — keep this)

### Roko repo
- **Path:** `/Users/will/dev/nunchi/roko/roko`
- **Branch:** Create `demo-backend` from current HEAD
- **Build:** `cargo build --workspace`
- **Pre-commit:** `cargo +nightly fmt --all && cargo clippy --workspace --no-deps -- -D warnings && cargo test --workspace`
- **Start server:** `roko init && cargo run -p roko-cli -- serve` (serves on localhost:6677)

### Environment variables
Create `/Users/will/dev/nunchi/nunchi-dashboard/.env.local`:
```
VITE_ROKO_URL=http://localhost:6677
VITE_CHAIN_URL=http://localhost:8545
VITE_PRIVY_APP_ID=placeholder-app-id
```
The password fallback ("daeji") activates when PRIVY_APP_ID is placeholder. For the demo, use password auth.

### CORS configuration
Add to `/Users/will/dev/nunchi/roko/roko/roko.toml` (create if missing):
```toml
[serve]
cors_origins = ["http://localhost:5174", "http://localhost:5173"]
```

### Vite proxy (alternative to CORS)
In `/Users/will/dev/nunchi/nunchi-dashboard/vite.config.js`, add:
```js
server: {
  proxy: {
    '/api': 'http://localhost:6677',
    '/ws': { target: 'ws://localhost:6677', ws: true }
  }
}
```
This eliminates CORS issues entirely. If using the proxy, API calls use relative URLs (`/api/jobs` not `http://localhost:6677/api/jobs`).

### Styling approach
The dashboard uses **Tailwind CSS v4** (via `@tailwindcss/vite` plugin). Continue using Tailwind utility classes. Design tokens go in `src/index.css` as CSS custom properties, referenced via Tailwind's `var()` support. Do NOT use inline `style={{}}` objects or CSS modules.

---

# Stream A: Dashboard complete rewrite

---

## Task A1: Project restructure + routing + design system

**Effort:** 4 hours
**Stream:** A -- Dashboard
**Repo:** nunchi-dashboard at `/Users/will/dev/nunchi/nunchi-dashboard`
**Day:** 1
**Prerequisites:** none

### Files to create

```
/Users/will/dev/nunchi/nunchi-dashboard/src/router.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/layouts/AppLayout.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/layouts/LandingLayout.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/tokens.css
/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/Card.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/Badge.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/Button.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/Input.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/Select.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/Gauge.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/Sparkline.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/Modal.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/Toast.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/Skeleton.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/StatusDot.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/EmptyState.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/ErrorState.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/index.ts
/Users/will/dev/nunchi/nunchi-dashboard/src/stores/authStore.ts
/Users/will/dev/nunchi/nunchi-dashboard/src/stores/wsStore.ts
/Users/will/dev/nunchi/nunchi-dashboard/src/stores/uiStore.ts
```

### Files to modify

```
/Users/will/dev/nunchi/nunchi-dashboard/src/App.tsx — gut entirely, replace with RouterProvider
/Users/will/dev/nunchi/nunchi-dashboard/src/main.jsx — fix Privy theming, add dark mode detection
/Users/will/dev/nunchi/nunchi-dashboard/package.json — add react-router-dom, zustand, @tanstack/react-query
```

### Implementation steps

**Step 1: Install dependencies**

```bash
cd /Users/will/dev/nunchi/nunchi-dashboard
npm install react-router-dom@7 zustand@5 @tanstack/react-query@5
```

**Step 2: Create design tokens** (`src/design-system/tokens.css`)

Define the ROSEDUST palette as CSS custom properties on `:root`. This is the single source of truth for all colors, spacing, and typography across the app.

```css
:root {
  /* --- Palette: ROSEDUST --- */
  --bg-void: #060608;
  --bg-surface-0: #0C0C10;
  --bg-surface-1: #121218;
  --bg-surface-2: #1A1A24;
  --bg-surface-3: #24242E;

  --fg-primary: #E8E4DE;
  --fg-secondary: #9B9590;       /* Must meet 4.5:1 on surface-0 */
  --fg-muted: #6B655F;           /* Must meet 3:1 on surface-0 for large text only */

  --rose: #AA7088;
  --rose-dim: #7A4F62;
  --rose-bright: #CC8FA8;

  --bone: #C8B890;
  --bone-dim: #8A7E62;

  --accent-gold: #FFD700;
  --accent-blue: #60A5FA;
  --accent-purple: #A78BFA;
  --accent-green: #4ADE80;
  --accent-red: #F87171;
  --accent-amber: #FBBF24;

  /* --- Spacing scale (px) --- */
  --sp-1: 4px;
  --sp-2: 8px;
  --sp-3: 12px;
  --sp-4: 16px;
  --sp-5: 24px;
  --sp-6: 32px;
  --sp-7: 48px;
  --sp-8: 64px;

  /* --- Typography scale --- */
  --text-xs: 12px;
  --text-sm: 14px;
  --text-base: 16px;
  --text-lg: 20px;
  --text-xl: 24px;
  --text-2xl: 32px;
  --text-3xl: 48px;
  --text-4xl: 72px;

  --font-mono: "Berkeley Mono", "JetBrains Mono", "Fira Code", monospace;
  --font-sans: "Inter", -apple-system, BlinkMacSystemFont, sans-serif;

  /* --- Border radius --- */
  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 12px;
  --radius-xl: 16px;
  --radius-full: 9999px;

  /* --- Shadows --- */
  --shadow-sm: 0 1px 2px rgba(0,0,0,0.4);
  --shadow-md: 0 4px 12px rgba(0,0,0,0.5);
  --shadow-lg: 0 8px 24px rgba(0,0,0,0.6);

  /* --- Z-index scale --- */
  --z-base: 0;
  --z-dropdown: 100;
  --z-modal: 200;
  --z-toast: 300;
}
```

**Step 3: Build design system components** (`src/design-system/components/`)

Each component is a small, composable building block. No business logic. Pure presentation.

`Card.tsx`:
```tsx
interface CardProps {
  children: React.ReactNode;
  padding?: "sm" | "md" | "lg";    // maps to sp-3, sp-4, sp-5
  surface?: 0 | 1 | 2 | 3;        // maps to bg-surface-N
  border?: boolean;                 // 1px solid surface-3
  hover?: boolean;                  // slight brightness increase on hover
  className?: string;
}
// Renders a <div> with the appropriate bg, padding, border-radius (radius-md), and optional hover transition.
```

`Badge.tsx`:
```tsx
interface BadgeProps {
  label: string;
  variant: "default" | "rose" | "gold" | "blue" | "green" | "red" | "amber";
  size?: "sm" | "md";
}
// Renders a <span> with pill shape (radius-full), font-size text-xs, padding sp-1 sp-2.
// Variant maps to background color at 20% opacity + text color at full.
```

`Button.tsx`:
```tsx
interface ButtonProps {
  children: React.ReactNode;
  variant?: "primary" | "secondary" | "ghost" | "danger";
  size?: "sm" | "md" | "lg";
  disabled?: boolean;
  loading?: boolean;
  onClick?: () => void;
  type?: "button" | "submit";
  className?: string;
}
// primary: bg rose, text fg-primary, hover rose-bright
// secondary: bg surface-2, text fg-secondary, hover surface-3
// ghost: bg transparent, text fg-secondary, hover surface-1
// danger: bg accent-red at 20%, text accent-red, hover accent-red at 30%
// loading: shows a 16px CSS spinner replacing children
```

`Input.tsx`:
```tsx
interface InputProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  type?: "text" | "number" | "url" | "email";
  placeholder?: string;
  error?: string;
  required?: boolean;
  min?: number;
  max?: number;
  disabled?: boolean;
}
// Label: text-sm, fg-secondary, margin-bottom sp-1.
// Input: bg surface-1, border 1px surface-3, radius-md, padding sp-2 sp-3, text-base fg-primary.
// Focus: border rose.
// Error: border accent-red, error text below in text-xs accent-red.
```

`Gauge.tsx`:
```tsx
interface GaugeProps {
  value: number;      // 0 to 1
  label?: string;
  color?: string;     // CSS color, defaults to rose
  size?: "sm" | "md"; // sm: 48px circle, md: 80px circle
}
// SVG circle gauge. Stroke-dasharray technique. Label text centered inside.
```

`Sparkline.tsx`:
```tsx
interface SparklineProps {
  data: number[];
  width?: number;     // px, default 120
  height?: number;    // px, default 32
  color?: string;     // default rose
  fill?: boolean;     // area fill at 10% opacity
}
// SVG polyline. No axes, no labels. Pure shape.
```

`Modal.tsx`:
```tsx
interface ModalProps {
  open: boolean;
  onClose: () => void;
  title: string;
  children: React.ReactNode;
  width?: "sm" | "md" | "lg";  // 400px, 600px, 800px
}
// Fixed position overlay. bg-void at 60% opacity backdrop. Card with surface-1.
// Close on Escape key. Close on backdrop click.
// Animate: opacity 0→1, transform translateY(8px)→0.
```

`Toast.tsx`:
```tsx
interface ToastProps {
  message: string;
  type: "info" | "success" | "error" | "warning";
  duration?: number;  // ms, default 4000
  onDismiss: () => void;
}
// Fixed bottom-right, z-toast. Slide in from right. Auto-dismiss after duration.
// type maps to left border color: blue/green/red/amber.
```

`Skeleton.tsx`:
```tsx
interface SkeletonProps {
  width?: string;     // CSS width, default "100%"
  height?: string;    // CSS height, default "20px"
  variant?: "text" | "circle" | "rect";
  count?: number;     // repeat N times vertically
}
// bg surface-2, shimmer animation (left-to-right gradient sweep, 1.5s infinite).
```

`StatusDot.tsx`:
```tsx
interface StatusDotProps {
  status: "active" | "idle" | "stopped" | "error";
  size?: number;  // px, default 8
  pulse?: boolean; // default true when active
}
// active: accent-green with pulse animation.
// idle: accent-amber, no pulse.
// stopped: fg-muted, no pulse.
// error: accent-red with pulse.
// Pulse: @keyframes pulse { 0%,100% { opacity:1 } 50% { opacity:0.4 } } 1.5s infinite.
```

`EmptyState.tsx`:
```tsx
interface EmptyStateProps {
  title: string;
  description: string;
  action?: { label: string; onClick: () => void };
}
// Centered vertically. fg-muted icon placeholder (dashed border circle).
// Title in text-lg fg-primary. Description in text-sm fg-secondary.
// Optional action button below (variant secondary).
```

`ErrorState.tsx`:
```tsx
interface ErrorStateProps {
  title?: string;     // default "Something went wrong"
  error: string;
  onRetry?: () => void;
}
// Similar to EmptyState but with accent-red icon and retry button.
```

`index.ts` re-exports everything.

**Step 4: Create Zustand stores** (`src/stores/`)

`authStore.ts`:
```ts
interface AuthState {
  user: { id: string; address?: string; email?: string } | null;
  token: string | null;
  setUser: (user: AuthState["user"]) => void;
  setToken: (token: string | null) => void;
  logout: () => void;
}
// persist to localStorage under key "nunchi-auth"
```

`wsStore.ts`:
```ts
interface WsState {
  status: "connecting" | "connected" | "disconnected" | "error";
  lastEvent: WsEvent | null;
  eventBuffer: WsEvent[];      // ring buffer, max 200
  setStatus: (s: WsState["status"]) => void;
  pushEvent: (e: WsEvent) => void;
}

interface WsEvent {
  type: string;
  payload: unknown;
  timestamp: number;
}
```

`uiStore.ts`:
```ts
interface UiState {
  sidebarCollapsed: boolean;
  rightPanelOpen: boolean;
  rightPanelContent: "cfactor" | "isfr" | "agent" | null;
  activeModal: string | null;
  toggleSidebar: () => void;
  toggleRightPanel: () => void;
  setRightPanel: (content: UiState["rightPanelContent"]) => void;
  openModal: (id: string) => void;
  closeModal: () => void;
}
```

**Step 5: Create router** (`src/router.tsx`)

```tsx
import { createBrowserRouter } from "react-router-dom";

// Lazy-load all page components
const Landing = lazy(() => import("./pages/Landing"));
const LiveAgents = lazy(() => import("./pages/observatory/LiveAgents"));
const Plans = lazy(() => import("./pages/observatory/Plans"));
const Learning = lazy(() => import("./pages/observatory/Learning"));
const Conductor = lazy(() => import("./pages/observatory/Conductor"));
const Costs = lazy(() => import("./pages/observatory/Costs"));
const AgentNetwork = lazy(() => import("./pages/network/AgentNetwork"));
const PheromoneField = lazy(() => import("./pages/network/PheromoneField"));
const KnowledgeGraph = lazy(() => import("./pages/network/KnowledgeGraph"));
const JobBoard = lazy(() => import("./pages/marketplace/JobBoard"));
const CreateJob = lazy(() => import("./pages/marketplace/CreateJob"));
const JobDetail = lazy(() => import("./pages/marketplace/JobDetail"));
const AgentOverview = lazy(() => import("./pages/agents/AgentOverview"));
const AgentStrategy = lazy(() => import("./pages/agents/AgentStrategy"));
const AgentKeys = lazy(() => import("./pages/agents/AgentKeys"));
const AgentDeploy = lazy(() => import("./pages/agents/AgentDeploy"));
const Chat = lazy(() => import("./pages/command/Chat"));
const Research = lazy(() => import("./pages/command/Research"));
const Atelier = lazy(() => import("./pages/atelier/Atelier"));
const PrdBrowser = lazy(() => import("./pages/atelier/PrdBrowser"));
const ExecutionMonitor = lazy(() => import("./pages/atelier/ExecutionMonitor"));
const Settings = lazy(() => import("./pages/settings/Settings"));

export const router = createBrowserRouter([
  {
    path: "/",
    element: <LandingLayout />,
    children: [{ index: true, element: <Landing /> }],
  },
  {
    path: "/app",
    element: <AppLayout />,
    children: [
      // Command
      { path: "chat", element: <Chat /> },
      { path: "research", element: <Research /> },

      // Observatory
      { index: true, element: <LiveAgents /> },
      { path: "observatory", element: <LiveAgents /> },
      { path: "observatory/plans", element: <Plans /> },
      { path: "observatory/learning", element: <Learning /> },
      { path: "observatory/conductor", element: <Conductor /> },
      { path: "observatory/costs", element: <Costs /> },

      // Network
      { path: "network", element: <AgentNetwork /> },
      { path: "network/pheromones", element: <PheromoneField /> },
      { path: "network/knowledge", element: <KnowledgeGraph /> },

      // Marketplace
      { path: "marketplace", element: <JobBoard /> },
      { path: "marketplace/create", element: <CreateJob /> },
      { path: "marketplace/create/:type", element: <CreateJob /> },
      { path: "marketplace/jobs/:id", element: <JobDetail /> },

      // Agent Studio
      { path: "agents", element: <AgentOverview /> },
      { path: "agents/strategy", element: <AgentStrategy /> },
      { path: "agents/keys", element: <AgentKeys /> },
      { path: "agents/deploy", element: <AgentDeploy /> },

      // Atelier
      { path: "atelier", element: <Atelier /> },
      { path: "atelier/prds", element: <PrdBrowser /> },
      { path: "atelier/execution", element: <ExecutionMonitor /> },

      // Settings
      { path: "settings", element: <Settings /> },
    ],
  },
]);
```

**Step 6: Create AppLayout** (`src/layouts/AppLayout.tsx`)

Three-zone responsive layout:

```
Desktop (>= 1200px):
  ┌──────────┬──────────────────────────┬──────────┐
  │ Left nav │     Main content         │ Right    │
  │  240px   │     flex-grow            │ panel    │
  │          │                          │  280px   │
  └──────────┴──────────────────────────┴──────────┘

Tablet (768px - 1199px):
  ┌────┬─────────────────────────────────────────┐
  │ 56 │     Main content                        │
  │ px │     (right panel hidden)                │
  └────┴─────────────────────────────────────────┘

Mobile (< 768px):
  ┌──────────────────────────────────────────────┐
  │  Top bar (hamburger + breadcrumbs)           │
  ├──────────────────────────────────────────────┤
  │  Main content (full width)                   │
  └──────────────────────────────────────────────┘
```

Left nav sections:
```
COMMAND
  Chat
  Research

OBSERVATORY
  Live Agents        (default /app)
  Plans
  Learning
  Conductor
  Costs

NETWORK
  Agent Network
  Pheromone Field
  Knowledge Graph

MARKETPLACE
  Job Board
  Create Job

AGENT STUDIO
  Overview
  Strategy
  Keys
  Deploy

ATELIER
  Workspace
  PRDs
  Execution

SETTINGS
  Settings
```

Top bar contains:
- Breadcrumbs derived from current route path
- Search input (placeholder, no backend yet -- tag `// MOCK: wire to /api/search`)
- Network pulse strip: `N agents online` + `block #` (both from `useHealth()`)
- Auth button (from Privy `usePrivy()`)

Right panel:
- Controlled by `uiStore.rightPanelContent`
- Shows C-Factor summary, ISFR card, or agent detail card
- Toggle via button in top bar
- Collapsed by default on first load

**Step 7: Rewrite App.tsx**

Remove all 411 lines. Replace with:

```tsx
import { RouterProvider } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { PrivyProvider } from "@privy-io/react-auth";
import { router } from "./router";
import "./design-system/tokens.css";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30_000,
      retry: 2,
      refetchOnWindowFocus: false,
    },
  },
});

export default function App() {
  return (
    <PrivyProvider appId={import.meta.env.VITE_PRIVY_APP_ID} config={{ /* theme config */ }}>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </PrivyProvider>
  );
}
```

**Step 8: Rewrite main.jsx**

Add system dark mode detection. Ensure `<html>` gets `class="dark"` since the app is dark-mode-only for now.

### Mock tags

- `// MOCK: wire search to /api/search when backend supports it`
- `// MOCK: block number from useHealth() -- depends on B4 server state`

### Done when

- [ ] `npm run dev` starts without errors
- [ ] Navigating to `/` shows the LandingLayout (blank page is fine -- A3 fills it)
- [ ] Navigating to `/app` shows the AppLayout with left nav, top bar, and empty main area
- [ ] Navigating to `/app/marketplace` loads (can show empty state)
- [ ] All route paths resolve without 404 (lazy components can be stub `export default () => <div>TODO</div>`)
- [ ] Left nav highlights the active route
- [ ] Sidebar collapses on tablet viewport (768-1199px)
- [ ] Mobile viewport shows hamburger menu
- [ ] Zustand stores initialize without errors (check React DevTools)
- [ ] No console errors on any route transition
- [ ] All design system components render in isolation (spot-check Card, Button, Badge, Modal)

---

## Task A2: API layer rewrite + WebSocket client

**Effort:** 3 hours
**Stream:** A -- Dashboard
**Repo:** nunchi-dashboard at `/Users/will/dev/nunchi/nunchi-dashboard`
**Day:** 1 (parallel-safe with A1 -- no file overlap)
**Prerequisites:** none (uses new files only; wired into app in A3+)

### Files to create

```
/Users/will/dev/nunchi/nunchi-dashboard/src/services/api.ts
/Users/will/dev/nunchi/nunchi-dashboard/src/services/ws.ts
/Users/will/dev/nunchi/nunchi-dashboard/src/services/queryKeys.ts
/Users/will/dev/nunchi/nunchi-dashboard/src/types/api.ts
```

### Files to modify

```
/Users/will/dev/nunchi/nunchi-dashboard/src/services/mirage-api.ts — add deprecation comment at top, keep for backward compat during migration
```

### Implementation steps

**Step 1: Define API types** (`src/types/api.ts`)

```ts
// --- Health ---
export interface HealthResponse {
  status: "ok" | "degraded" | "error";
  version: string;
  uptime_secs: number;
  agent_count: number;
  task_count: number;
  insight_count: number;
  isfr_count: number;
  block_number: number;
}

// --- Agents ---
export interface Agent {
  id: string;
  name: string;
  role: string;
  status: "active" | "idle" | "stopped" | "error";
  current_task: string | null;
  tier: number;
  token_burn_rate: number;
  cost_cumulative: number;
  episode_count: number;
  gate_pass_rate: number;
  last_heartbeat: string | null;
  capabilities: string[];
}

// --- Plans ---
export interface Plan {
  id: string;
  name: string;
  status: "pending" | "running" | "completed" | "failed" | "cancelled";
  task_count: number;
  tasks_completed: number;
  created_at: string;
  updated_at: string;
}

export interface PlanDetail extends Plan {
  tasks: PlanTask[];
  gate_results: GateResult[];
}

export interface PlanTask {
  id: string;
  name: string;
  status: "pending" | "running" | "passed" | "failed" | "skipped";
  agent_id: string | null;
  started_at: string | null;
  completed_at: string | null;
  depends_on: string[];
}

// --- Jobs ---
export type JobType = "research_brief" | "coding_task" | "code_review" | "custom";
export type JobState = "open" | "assigned" | "in_progress" | "submitted" | "under_review" | "completed" | "rejected" | "expired";

export interface Job {
  id: string;
  job_type: JobType;
  title: string;
  description: string;
  bounty_daeji: number;
  min_tier: number;
  deadline: string;
  state: JobState;
  poster: string;
  assigned_worker: string | null;
  submission: JobSubmission | null;
  evaluation: JobEvaluation | null;
  created_at: string;
  updated_at: string;
}

export interface JobSubmission {
  result_hash: string;
  deliverable_url: string | null;
  summary: string;
  gate_results: GateResult[] | null;
  submitted_at: string;
}

export interface JobEvaluation {
  accepted: boolean;
  reason: string;
  evaluator: string;
  evaluated_at: string;
}

export interface CreateJobRequest {
  job_type: JobType;
  title: string;
  description: string;
  bounty_daeji: number;
  min_tier?: number;
  deadline_hours?: number;
  metadata?: Record<string, unknown>;
}

// --- Gates ---
export interface GateResult {
  gate: string;
  passed: boolean;
  message: string;
  rung: number;
  timestamp: string;
}

// --- Learning ---
export interface CFactorSummary {
  overall: number;
  trend: number[];
  breakdown: { name: string; score: number }[];
}

export interface Experiment {
  id: string;
  name: string;
  variant_a: string;
  variant_b: string;
  winner: string | null;
  sample_size: number;
  p_value: number | null;
}

// --- Providers ---
export interface Provider {
  id: string;
  name: string;
  status: "healthy" | "degraded" | "down";
  latency_ms: number;
  error_rate: number;
  models: string[];
}

// --- Heartbeats ---
export interface Heartbeat {
  agent_id: string;
  status: "working" | "idle" | "error";
  current_task: string | null;
  cognitive_tier: number;
  context_utilization: number;
  token_burn_rate: number;
  episode_count: number;
  gate_pass_rate: number;
  cumulative_cost: number;
  timestamp: string;
}

// --- Network ---
export interface NetworkStats {
  agents_online: number;
  domains: Record<string, number>;
  total_tasks_completed: number;
  avg_cost_per_task: number;
}

// --- PRDs ---
export interface Prd {
  slug: string;
  title: string;
  status: "idea" | "draft" | "published" | "planned";
  created_at: string;
}

// --- Config ---
export interface RokoConfig {
  [key: string]: unknown;
}
```

**Step 2: Create query key factory** (`src/services/queryKeys.ts`)

```ts
export const queryKeys = {
  health: ["health"] as const,
  agents: {
    all: ["agents"] as const,
    detail: (id: string) => ["agents", id] as const,
    stats: (id: string) => ["agents", id, "stats"] as const,
    episodes: (id: string) => ["agents", id, "episodes"] as const,
    heartbeat: (id: string) => ["agents", id, "heartbeat"] as const,
  },
  plans: {
    all: ["plans"] as const,
    detail: (id: string) => ["plans", id] as const,
  },
  jobs: {
    all: ["jobs"] as const,
    filtered: (filters: Record<string, string>) => ["jobs", filters] as const,
    detail: (id: string) => ["jobs", id] as const,
    stats: ["jobs", "stats"] as const,
  },
  learning: {
    cfactor: ["learning", "cfactor"] as const,
    experiments: ["learning", "experiments"] as const,
    costTiers: ["learning", "cost-tiers"] as const,
    gates: ["learning", "gates"] as const,
  },
  providers: {
    all: ["providers"] as const,
  },
  network: {
    stats: ["network", "stats"] as const,
    topology: ["network", "topology"] as const,
    pheromones: ["network", "pheromones"] as const,
  },
  knowledge: {
    entries: ["knowledge", "entries"] as const,
  },
  prds: {
    all: ["prds"] as const,
    detail: (slug: string) => ["prds", slug] as const,
  },
  config: ["config"] as const,
  heartbeats: ["heartbeats"] as const,
  diagnosis: ["diagnosis"] as const,
};
```

**Step 3: Create API service** (`src/services/api.ts`)

```ts
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { queryKeys } from "./queryKeys";
import type * as T from "../types/api";

const BASE_URL = import.meta.env.VITE_ROKO_API_URL || "http://localhost:6677/api";

async function fetchApi<R>(path: string, init?: RequestInit): Promise<R> {
  const token = localStorage.getItem("nunchi-auth-token");
  const res = await fetch(`${BASE_URL}${path}`, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...init?.headers,
    },
  });
  if (!res.ok) {
    const body = await res.text().catch(() => "");
    throw new Error(`API ${res.status}: ${body}`);
  }
  return res.json();
}

// --- Query hooks ---

export function useHealth() {
  return useQuery({
    queryKey: queryKeys.health,
    queryFn: () => fetchApi<T.HealthResponse>("/status"),
    refetchInterval: 10_000,
    staleTime: 5_000,
  });
}

export function useAgents() {
  return useQuery({
    queryKey: queryKeys.agents.all,
    queryFn: () => fetchApi<T.Agent[]>("/agents"),
    refetchInterval: 15_000,
  });
}

export function useAgent(id: string) {
  return useQuery({
    queryKey: queryKeys.agents.detail(id),
    queryFn: () => fetchApi<T.Agent>(`/agents/${id}`),
    enabled: !!id,
  });
}

export function usePlans() {
  return useQuery({
    queryKey: queryKeys.plans.all,
    queryFn: () => fetchApi<T.Plan[]>("/plans"),
    refetchInterval: 15_000,
  });
}

export function usePlan(id: string) {
  return useQuery({
    queryKey: queryKeys.plans.detail(id),
    queryFn: () => fetchApi<T.PlanDetail>(`/plans/${id}`),
    enabled: !!id,
    refetchInterval: 5_000,
  });
}

export function useJobs(filters?: Record<string, string>) {
  const params = filters ? "?" + new URLSearchParams(filters).toString() : "";
  return useQuery({
    queryKey: filters ? queryKeys.jobs.filtered(filters) : queryKeys.jobs.all,
    queryFn: () => fetchApi<T.Job[]>(`/jobs${params}`),
    refetchInterval: 10_000,
  });
}

export function useJob(id: string) {
  return useQuery({
    queryKey: queryKeys.jobs.detail(id),
    queryFn: () => fetchApi<T.Job>(`/jobs/${id}`),
    enabled: !!id,
    refetchInterval: 5_000,
  });
}

export function useJobStats() {
  return useQuery({
    queryKey: queryKeys.jobs.stats,
    queryFn: () => fetchApi<Record<string, number>>("/jobs/stats"),
    refetchInterval: 30_000,
  });
}

export function useCFactor() {
  return useQuery({
    queryKey: queryKeys.learning.cfactor,
    queryFn: () => fetchApi<T.CFactorSummary>("/metrics/c_factor"),
    refetchInterval: 30_000,
  });
}

export function useExperiments() {
  return useQuery({
    queryKey: queryKeys.learning.experiments,
    queryFn: () => fetchApi<T.Experiment[]>("/learning/experiments"),
    staleTime: 60_000,
  });
}

export function useProviders() {
  return useQuery({
    queryKey: queryKeys.providers.all,
    queryFn: () => fetchApi<T.Provider[]>("/providers"),
    refetchInterval: 30_000,
  });
}

export function useNetworkStats() {
  return useQuery({
    queryKey: queryKeys.network.stats,
    queryFn: () => fetchApi<T.NetworkStats>("/network/stats"),
    refetchInterval: 10_000,
  });
}

export function useHeartbeats() {
  return useQuery({
    queryKey: queryKeys.heartbeats,
    queryFn: () => fetchApi<T.Heartbeat[]>("/heartbeats"),
    refetchInterval: 10_000,
  });
}

export function usePrds() {
  return useQuery({
    queryKey: queryKeys.prds.all,
    queryFn: () => fetchApi<T.Prd[]>("/prds"),
    staleTime: 60_000,
  });
}

export function useDiagnosis() {
  return useQuery({
    queryKey: queryKeys.diagnosis,
    queryFn: () => fetchApi<unknown[]>("/diagnosis/recent"),
    staleTime: 30_000,
  });
}

export function useConfig() {
  return useQuery({
    queryKey: queryKeys.config,
    queryFn: () => fetchApi<T.RokoConfig>("/config"),
    staleTime: 120_000,
  });
}

// --- Mutation hooks ---

export function useCreateJob() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (req: T.CreateJobRequest) =>
      fetchApi<T.Job>("/jobs", { method: "POST", body: JSON.stringify(req) }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: queryKeys.jobs.all });
    },
  });
}

export function useExecutePlan() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (planId: string) =>
      fetchApi<void>(`/plans/${planId}/execute`, { method: "POST" }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: queryKeys.plans.all });
    },
  });
}

export function useEvaluateJob() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, accepted, reason }: { id: string; accepted: boolean; reason: string }) =>
      fetchApi<T.Job>(`/jobs/${id}/evaluate`, {
        method: "POST",
        body: JSON.stringify({ accepted, reason }),
      }),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: queryKeys.jobs.detail(vars.id) });
      qc.invalidateQueries({ queryKey: queryKeys.jobs.all });
    },
  });
}
```

**Step 4: Create WebSocket client** (`src/services/ws.ts`)

```ts
import { useWsStore } from "../stores/wsStore";

const WS_URL = import.meta.env.VITE_ROKO_WS_URL || "ws://localhost:6677/ws";

let socket: WebSocket | null = null;
let reconnectTimer: number | null = null;
let reconnectAttempt = 0;
const MAX_RECONNECT_DELAY = 30_000;

export function connectWs(): void {
  if (socket?.readyState === WebSocket.OPEN) return;

  const store = useWsStore.getState();
  store.setStatus("connecting");

  socket = new WebSocket(WS_URL);

  socket.onopen = () => {
    store.setStatus("connected");
    reconnectAttempt = 0;
  };

  socket.onmessage = (event) => {
    try {
      const parsed = JSON.parse(event.data);
      store.pushEvent({
        type: parsed.type || "unknown",
        payload: parsed,
        timestamp: Date.now(),
      });
    } catch {
      // non-JSON message, ignore
    }
  };

  socket.onclose = () => {
    store.setStatus("disconnected");
    scheduleReconnect();
  };

  socket.onerror = () => {
    store.setStatus("error");
    socket?.close();
  };
}

function scheduleReconnect() {
  if (reconnectTimer) return;
  const delay = Math.min(1000 * Math.pow(2, reconnectAttempt), MAX_RECONNECT_DELAY);
  reconnectAttempt++;
  reconnectTimer = window.setTimeout(() => {
    reconnectTimer = null;
    connectWs();
  }, delay);
}

export function disconnectWs(): void {
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
  socket?.close();
  socket = null;
}

export function sendWs(message: unknown): void {
  if (socket?.readyState === WebSocket.OPEN) {
    socket.send(JSON.stringify(message));
  }
}
```

**Step 5: Deprecate mirage-api.ts**

Add to the top of `/Users/will/dev/nunchi/nunchi-dashboard/src/services/mirage-api.ts`:

```ts
// DEPRECATED: This file is being replaced by src/services/api.ts (TanStack Query hooks).
// During migration, existing components still import from here.
// New components must use src/services/api.ts instead.
// Remove this file after all components are migrated.
```

### Mock tags

None in this task. The API layer calls real endpoints. When endpoints are unreachable, TanStack Query's error/loading states surface that to the UI. Mock data lives in the components that consume these hooks (tagged there).

### Done when

- [ ] `api.ts` exports all listed hooks without TypeScript errors
- [ ] `ws.ts` connects and reconnects (test against `wscat -l 6677` or equivalent)
- [ ] `queryKeys.ts` compiles
- [ ] `types/api.ts` compiles
- [ ] No circular imports
- [ ] mirage-api.ts has deprecation comment
- [ ] `npm run typecheck` passes

---

## Task A3: Landing page

**Effort:** 4 hours
**Stream:** A -- Dashboard
**Repo:** nunchi-dashboard at `/Users/will/dev/nunchi/nunchi-dashboard`
**Day:** 1
**Prerequisites:** A1 (design system + router)

### Files to create

```
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/Landing.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/components/landing/HeroSection.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/components/landing/ArchitectureExplorer.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/components/landing/ContextAuction.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/components/landing/StigmergyCanvas.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/components/landing/StatsStrip.tsx
```

### Files to modify

```
/Users/will/dev/nunchi/nunchi-dashboard/src/layouts/LandingLayout.tsx — ensure it renders <Outlet /> full-screen, no nav
```

### Implementation steps

**Step 1: LandingLayout**

Minimal wrapper: full-viewport, `bg-void`, renders `<Outlet />`. No sidebar, no top bar. The landing page owns its entire viewport.

**Step 2: Landing.tsx**

Five sections stacked vertically, each `min-height: 100vh` or auto. Smooth scroll between sections. No external animation library -- CSS `@keyframes` and `requestAnimationFrame` only.

```tsx
export default function Landing() {
  return (
    <div className="landing" style={{ background: "var(--bg-void)", color: "var(--fg-primary)" }}>
      <HeroSection />
      <ArchitectureExplorer />
      <ContextAuction />
      <StigmergyCanvas />
      <StatsStrip />
    </div>
  );
}
```

**Step 3: HeroSection.tsx**

Full viewport height. Centered content with animated background.

```
Layout:
  ┌─────────────────────────────────────────────────┐
  │                                                   │
  │         N U N C H I                               │
  │         ──────────                                │
  │         Hyperdimensional Intelligence              │
  │                                                   │
  │   ┌─────────┐  ┌─────────┐  ┌─────────┐         │
  │   │ 12      │  │ 847     │  │ 2,341   │         │
  │   │ agents  │  │ tasks   │  │ insights│         │
  │   │ online  │  │ done    │  │ stored  │         │
  │   └─────────┘  └─────────┘  └─────────┘         │
  │                                                   │
  │              [ Launch Dashboard ]                  │
  │                                                   │
  └─────────────────────────────────────────────────┘
```

- Title: `font-size: var(--text-4xl)`, `font-weight: 800`, `letter-spacing: 0.2em`, `color: var(--rose)`
- Subtitle: `font-size: var(--text-xl)`, `color: var(--fg-secondary)`
- Counter strip: three cards with numbers that animate in (count-up from 0 over 1.5s using `requestAnimationFrame`). Data from `useHealth()`. Fallback to mock values on error.
- CTA button: `variant="primary"`, `size="lg"`, navigates to `/app`
- Background: animated radial gradient. Two overlapping radial gradients that slowly rotate via CSS `@keyframes`:
  ```css
  @keyframes gradient-rotate {
    0% { background-position: 0% 50%; }
    50% { background-position: 100% 50%; }
    100% { background-position: 0% 50%; }
  }
  ```
  Colors: `var(--rose-dim)` at 10% opacity, `var(--accent-purple)` at 5% opacity, rest transparent.

Mock tags:
```tsx
{/* MOCK: agents_online from useHealth() -- falls back to 12 when API unreachable */}
{/* MOCK: tasks_done from useHealth() -- falls back to 847 */}
{/* MOCK: insights from useHealth() -- falls back to 2341 */}
```

**Step 4: ArchitectureExplorer.tsx**

Interactive SVG showing the three cognitive tiers radiating from a central agent node.

```
Layout:
         T0 Reflex
           │
  T1 ──── Agent ──── T2
  Deliberate         Reflective
```

- Central circle: "Roko Agent", diameter 120px, fill `var(--rose-dim)`, stroke `var(--rose)`
- Three tier nodes connected by animated dashed lines (SVG `stroke-dasharray` with `stroke-dashoffset` animation)
- T0 node: `var(--accent-gold)`, label "Reflex", subtitle "Rust FSM, ~$0, 80% of decisions"
- T1 node: `var(--accent-blue)`, label "Deliberate", subtitle "Haiku, ~$0.002, 15%"
- T2 node: `var(--accent-purple)`, label "Reflective", subtitle "Opus, ~$0.10, 5%"
- On click of any tier node: expand a detail panel below the SVG showing what happens at that tier:
  - T0: "Pattern-matched signals trigger pre-authorized actions. Safety contracts enforce hard stops. No LLM round-trip."
  - T1: "VCG auction allocates 8K context tokens across 9 bidders. The model sees what matters. Attention is priced, not assumed."
  - T2: "Cross-agent knowledge consolidation. HDC vectors encode prior episodes. The agent remembers what worked."
- Expand/collapse via `max-height` CSS transition. Only one tier expanded at a time.

Fully static data. No mock tags needed.

**Step 5: ContextAuction.tsx**

Visual showing the VCG auction. Horizontal bar chart of token allocations.

```
Layout:
  Context engineering: how agents decide what to think about

  ┌──────────────────────────────────────────┐
  │ Neuro Store        ████████████  2,400   │
  │ Task History       █████████     1,800   │
  │ Research Context   ███████       1,400   │
  │ Pheromone Field    █████         1,000   │
  │ Domain Profile     ████            800   │
  │ Safety Contracts   ██              400   │
  │ Operating Freq     █               200   │
  │                                          │
  │ Total allocated: 8,000 / 8,192 tokens    │
  └──────────────────────────────────────────┘
```

- Section heading: `font-size: var(--text-2xl)`, `color: var(--fg-primary)`
- Subheading: "9 bidders compete for 8K context tokens via second-price auction"
- Bar chart: plain HTML divs with CSS width transitions. Each bar is a row:
  - Label (text-sm, fg-secondary, 180px fixed width)
  - Bar (height 24px, color from bidder, width proportional to value/max)
  - Value (text-sm, fg-muted, right-aligned)
- Bars animate in sequentially (each bar 100ms delayed) using CSS `@keyframes slideIn` on mount.
- All values are hardcoded. Accurate to the VCG implementation in roko-compose.

No mock tags -- this is explanatory content.

**Step 6: StigmergyCanvas.tsx**

Animated HTML5 canvas showing agents as particles with pheromone-trail connections.

- Canvas: `width: 100%`, `height: 400px`, transparent background
- 30 particles, each with:
  ```ts
  interface Particle {
    x: number; y: number;
    vx: number; vy: number;
    role: "research" | "coding" | "keeper" | "planner";
    color: string;  // gold / blue / purple / green
    radius: number; // 4-6px
  }
  ```
- Every frame (`requestAnimationFrame`):
  1. Move each particle by `(vx, vy)` (speed: 0.3-0.8 px/frame)
  2. Bounce off canvas edges
  3. For each pair within 80px distance: draw a line with opacity inversely proportional to distance (`1 - dist/80`), color blended from both particles
  4. Leave a fading trail: every 60 frames, each particle drops a "pheromone" dot at its position. Dots fade from alpha 0.3 to 0 over 300 frames. Draw these dots before particles.
- Section heading: "Stigmergy: knowledge sharing without coordination"
- Subheading: "Agents leave traces for each other. No central orchestrator needed."

No mock tags.

**Step 7: StatsStrip.tsx**

Four stats in a horizontal row (flex-wrap on mobile) + final CTA.

```tsx
const stats = [
  { label: "Agents active", value: health?.agent_count, fallback: 12 },
  { label: "Tasks completed", value: health?.task_count, fallback: 847 },
  { label: "Knowledge entries", value: health?.insight_count, fallback: 2341 },
  { label: "ISFR instruments", value: health?.isfr_count, fallback: 0 },
];
// MOCK: all values fall back to hardcoded numbers when useHealth() errors
```

Below stats: large CTA button "Launch dashboard", variant primary, size lg, navigates to `/app`.

### Mock tags

- `// MOCK: stats values fall back to [12, 847, 2341, 0] when /api/status unreachable -- wired by B4`
- `// MOCK: ISFR count always 0 until mirage-rs /api/isfr/count exists`

### Done when

- [ ] Landing page renders at `/` without errors
- [ ] Hero section shows animated gradient background
- [ ] Counter numbers animate in (count-up effect)
- [ ] "Launch dashboard" buttons navigate to `/app`
- [ ] Architecture SVG renders with three tier nodes
- [ ] Clicking a tier node expands its detail panel; clicking another collapses the first
- [ ] Context auction bar chart renders with animated bars
- [ ] Stigmergy canvas animates (particles move, connections draw, pheromone trails fade)
- [ ] Stats strip shows four values
- [ ] Page is usable at 375px mobile width (sections stack, canvas resizes)
- [ ] No external animation libraries used (CSS + requestAnimationFrame only)
- [ ] Performance: 60fps on the canvas animation (check with Chrome DevTools Performance tab)

---

## Task A4: Observatory pages (5 pages)

**Effort:** 5 hours
**Stream:** A -- Dashboard
**Repo:** nunchi-dashboard at `/Users/will/dev/nunchi/nunchi-dashboard`
**Day:** 1-2
**Prerequisites:** A1 (layout), A2 (API hooks)

### Files to create

```
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/observatory/LiveAgents.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/observatory/Plans.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/observatory/Learning.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/observatory/Conductor.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/observatory/Costs.tsx
```

### Implementation steps

**Step 1: LiveAgents.tsx** (default /app page)

```
Layout:
  ┌──────────────────────────────────────────────────────────────┐
  │ Activity feed (real-time events)                     Filter  │
  ├──────────────────────────────────────────────────────────────┤
  │ ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌──────────┐ │
  │ │ Agent card  │ │ Agent card  │ │ Agent card  │ │ + more   │ │
  │ │ status dot  │ │ status dot  │ │ status dot  │ │          │ │
  │ │ name/role   │ │ name/role   │ │ name/role   │ │          │ │
  │ │ current tsk │ │ current tsk │ │ current tsk │ │          │ │
  │ │ burn rate   │ │ burn rate   │ │ burn rate   │ │          │ │
  │ │ tier bars   │ │ tier bars   │ │ tier bars   │ │          │ │
  │ └────────────┘ └────────────┘ └────────────┘ └──────────┘ │
  ├──────────────────────────────────────────────────────────────┤
  │ Conductor alerts                                             │
  │  [WARNING] Agent roko-02 gate failure rate above threshold   │
  │  [INFO] Circuit breaker reset for provider anthropic         │
  └──────────────────────────────────────────────────────────────┘
```

- Activity feed: scrollable div, max-height 200px. Shows WS events from `useWsStore().eventBuffer`. Each event: timestamp (text-xs, fg-muted), type badge, message text. Newest on top.
- Agent cards: grid layout, `grid-template-columns: repeat(auto-fill, minmax(280px, 1fr))`. Data from `useAgents()`. Each card is a `<Card>` component with:
  - StatusDot (top-right)
  - Name (`text-lg`) and role badge (`text-xs`)
  - Current task (text-sm, truncated to 60 chars, or "idle" in fg-muted)
  - Token burn rate: `text-xs fg-secondary`, e.g. "0.42 tok/s"
  - Cognitive tier bars: three tiny horizontal bars (4px height, 80px total width) showing T0/T1/T2 distribution. Colors: gold/blue/purple.
- Conductor alerts: from `useDiagnosis()`. Each alert: severity badge (WARNING amber, INFO blue, ERROR red), message text, timestamp. Max 5 shown.
- Loading state: show Skeleton components for cards
- Error state: show ErrorState with retry
- Empty state: show EmptyState with "No agents registered yet"

Mock tags:
```tsx
// MOCK: cognitive tier bars use [0.2, 0.6, 0.2] until /api/agents/:id/efficiency exists -- depends on B7
// MOCK: conductor alerts fall back to empty array until /api/diagnosis/recent works
```

**Step 2: Plans.tsx**

```
Layout:
  ┌────────────────────────────────────┬───────────────────────┐
  │ Plan list                          │ Plan detail           │
  │  [x] plan-001 feat: add jobs  100% │ Task DAG (SVG)        │
  │  [ ] plan-002 fix: gate ...    40% │                       │
  │  [ ] plan-003 refactor: ...     0% │ Gate results table    │
  │                                    │                       │
  │  [Execute selected] [Refresh]      │ [Execute] [Cancel]    │
  └────────────────────────────────────┴───────────────────────┘
```

- Left panel (40%): plan list from `usePlans()`. Each row: status icon (checkmark/spinner/dash), name (truncated), progress bar (percent complete = tasks_completed/task_count).
- Right panel (60%): selected plan detail from `usePlan(selectedId)`. Shows:
  - Task DAG: render as a vertical list with indentation based on `depends_on`. Each task: status dot + name + agent badge. No external graph library -- indent with `padding-left: depth * 24px`.
  - Gate results table: columns = gate name, passed (green check / red X), message (truncated), rung number, timestamp.
  - Execute button: calls `useExecutePlan()` mutation.
- Click a plan on the left to load its detail on the right.

Mock tags:
```tsx
// MOCK: plan list falls back to 3 hardcoded plans when /api/plans unreachable -- depends on B4
// MOCK: task DAG renders flat list until /api/plans/:id returns depends_on data
```

**Step 3: Learning.tsx**

Four cards in a 2x2 grid:

1. **C-Factor card**: overall score as a Gauge component + trend as a Sparkline. Data from `useCFactor()`. Breakdown table below: name, score (as horizontal bar 0-1).
2. **Gate pass rates**: table with columns: gate name, pass rate (percent), total runs. Data from `useCFactor()?.breakdown` or fallback to `/api/gates/summary`.
3. **A/B experiments**: table with columns: name, variant A, variant B, winner (or "running"), sample size. Data from `useExperiments()`.
4. **Cost tier breakdown**: three cards showing T0/T1/T2 with cost-per-decision and decision count. Source: `/api/learning/cost-tiers`.

Mock tags:
```tsx
// MOCK: C-Factor overall defaults to 0.72, trend [0.65, 0.68, 0.70, 0.72] -- depends on B8
// MOCK: experiments list empty until /api/learning/experiments returns data
// MOCK: cost tiers hardcoded { t0: $0.00, t1: $0.002, t2: $0.10 } until wired
```

**Step 4: Conductor.tsx**

```
Layout:
  ┌──────────────────────────────────────────────────────────┐
  │ Watcher status table                                      │
  │  Name           │ Status  │ Last check  │ Interventions   │
  │  drift_watcher  │ active  │ 2s ago      │ 3               │
  │  cost_watcher   │ active  │ 5s ago      │ 1               │
  │  ...                                                      │
  ├──────────────────────────────────────────────────────────┤
  │ Intervention history                                      │
  │  [12:04:02] drift_watcher: "Agent roko-02 drifting..."   │
  │  [12:03:45] cost_watcher: "Budget exceeded for..."       │
  ├──────────────────────────────────────────────────────────┤
  │ Circuit breaker status                                    │
  │  anthropic: CLOSED (healthy)                              │
  │  openai: HALF_OPEN (testing)                              │
  └──────────────────────────────────────────────────────────┘
```

Data from `useDiagnosis()`. Parse the response into watcher entries, intervention entries, and circuit breaker state.

Mock tags:
```tsx
// MOCK: watcher status hardcoded to 10 watchers all "active" -- depends on /api/diagnosis/watchers
// MOCK: circuit breakers hardcoded ["anthropic: CLOSED", "openai: CLOSED"] -- depends on /api/providers/circuit-breakers
```

**Step 5: Costs.tsx**

```
Layout:
  ┌────────────────────────────────┬─────────────────────────┐
  │ Per-agent cost table           │ Provider health cards    │
  │  Agent    │ Cost    │ Rate     │ ┌────────┐ ┌────────┐  │
  │  roko-01  │ $0.42   │ $0.02/m  │ │Anthropic│ │ OpenAI │  │
  │  roko-02  │ $1.23   │ $0.05/m  │ │ healthy │ │degraded│  │
  │           │         │          │ │ 142ms   │ │ 890ms  │  │
  ├────────────────────────────────┤ └────────┘ └────────┘  │
  │ Budget tracking                │                         │
  │  Spent: $4.21 / $50.00        │ Per-model cost chart     │
  │  Burn rate: $0.12/hr          │ (Sparkline per model)    │
  │  Est. runway: 381 hours       │                         │
  └────────────────────────────────┴─────────────────────────┘
```

Data from `useAgents()` (cost fields), `useProviders()`, and `useCFactor()`.

Mock tags:
```tsx
// MOCK: per-agent costs from agent.cost_cumulative -- real when /api/agents returns cost data (B7)
// MOCK: budget $50.00 hardcoded -- wire to /api/config budget field
// MOCK: burn rate calculated from last 10min of cost changes -- approximation until real tracking
```

### Done when

- [ ] All five observatory pages render at their routes without errors
- [ ] LiveAgents shows agent cards with status dots, tier bars, and burn rate
- [ ] Plans page shows plan list on left, detail on right when clicked
- [ ] Plans execute button calls mutation (can fail if backend not ready -- that is fine)
- [ ] Learning page shows four cards with data or skeleton loaders
- [ ] Conductor page shows watcher table and circuit breaker status
- [ ] Costs page shows per-agent cost table and provider health cards
- [ ] All pages show Skeleton loaders during data fetch
- [ ] All pages show ErrorState on API failure
- [ ] All pages show EmptyState when no data

---

## Task A5: Network + Knowledge pages (3 pages)

**Effort:** 3 hours
**Stream:** A -- Dashboard
**Repo:** nunchi-dashboard at `/Users/will/dev/nunchi/nunchi-dashboard`
**Day:** 2
**Prerequisites:** A1, A2

### Files to create

```
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/network/AgentNetwork.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/network/PheromoneField.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/network/KnowledgeGraph.tsx
```

### Implementation steps

**Step 1: AgentNetwork.tsx**

Reuse the existing `react-force-graph-2d` dependency (already in package.json). Migrate the existing `NetworkPanel.tsx` (973 lines) force-directed graph into this page.

- Import `ForceGraph2D` from `react-force-graph-2d`
- Data: `useAgents()` provides nodes. Build edges from shared capabilities or shared plan membership.
- Node appearance: circle sized by episode_count, colored by role, StatusDot overlay for status.
- Edge appearance: thin line, opacity based on connection strength.
- Controls: zoom (scroll), pan (drag background), hover tooltip showing agent details.
- Top stats strip: `N agents online | N domains | avg latency Xms`
- Fall back to existing TopologyCanvas or NetworkPanel logic where possible. Port, do not rewrite from scratch.

Mock tags:
```tsx
// MOCK: edge weights hardcoded to 0.5 until /api/agents/topology returns real topology -- depends on B8
// MOCK: network stats strip uses agent count from useAgents() -- latency from useProviders()
```

**Step 2: PheromoneField.tsx**

Canvas-based heatmap visualization.

- Canvas: `width: 100%`, `height: 500px`
- Grid: 20x20 cells. Each cell colored based on "pheromone intensity" (0-1 scale).
- Color mapping: 0 = `var(--bg-surface-0)`, 0.5 = `var(--rose-dim)`, 1.0 = `var(--rose-bright)`
- Agent positions: overlay small circles at agent positions on the grid.
- Data: poll `/api/pheromones/heatmap` for cell values. If unavailable, generate random data that evolves: each frame, each cell decays by 0.01 and gets random additions when an agent is nearby.
- Animate with `requestAnimationFrame`, target 30fps.

Mock tags:
```tsx
// MOCK: pheromone grid uses random decay simulation -- wire to GET /api/pheromones/heatmap when backend supports it
// MOCK: agent positions from useAgents() mapped to grid coords -- approximation
```

**Step 3: KnowledgeGraph.tsx**

InsightStore browser with search.

- Top: search input with debounced query (300ms)
- Results list: each entry shows title, confidence bar (Gauge, size sm), source badge, created_at
- Click entry: expand to show full content below
- Data: `/api/knowledge/entries?q=<search>`. If unavailable, show mock entries.
- Reuse logic from existing `InsightStoreView.tsx` (521 lines) where possible.

Mock tags:
```tsx
// MOCK: knowledge entries hardcoded to 5 sample entries -- wire to GET /api/knowledge/entries
```

### Done when

- [ ] AgentNetwork renders force-directed graph with agent nodes
- [ ] PheromoneField renders animated heatmap canvas
- [ ] KnowledgeGraph renders searchable entry list
- [ ] All three pages handle loading, error, and empty states
- [ ] No console errors on any page

---

## Task A6: Marketplace pages (3 pages)

**Effort:** 5 hours
**Stream:** A -- Dashboard
**Repo:** nunchi-dashboard at `/Users/will/dev/nunchi/nunchi-dashboard`
**Day:** 2
**Prerequisites:** A1 (layout), A2 (API hooks, types)

### Files to create

```
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/marketplace/JobBoard.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/marketplace/CreateJob.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/marketplace/JobDetail.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/components/marketplace/JobCard.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/components/marketplace/StatusTimeline.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/components/marketplace/DeliverableViewer.tsx
```

### Implementation steps

**Step 1: JobCard.tsx**

Reusable card for job list items.

```tsx
interface JobCardProps {
  job: Job;
  onClick: () => void;
}

// Renders:
//   ┌──────────────────────────────────┐
//   │ [RESEARCH]  2h ago        100 DJ │
//   │ Analyze EIP-7702 MEV impact      │
//   │ [OPEN]   Posted by will          │
//   └──────────────────────────────────┘
//
// - Type badge: Badge component, variant based on job_type
//     research_brief → rose, coding_task → blue, code_review → purple, custom → bone
// - Time: relative time (use a 20-line helper, no library)
// - Bounty: right-aligned, text-lg accent-gold, "DJ" suffix
// - Title: text-base fg-primary, truncated to 80 chars
// - Status badge + poster: bottom row
// - Hover: border-color transitions to rose
// - Click: calls onClick
```

**Step 2: StatusTimeline.tsx**

Horizontal step indicator for job lifecycle.

```tsx
interface StatusTimelineProps {
  currentState: JobState;
}

const STEPS: { state: JobState; label: string }[] = [
  { state: "open", label: "Posted" },
  { state: "assigned", label: "Assigned" },
  { state: "in_progress", label: "Working" },
  { state: "submitted", label: "Submitted" },
  { state: "under_review", label: "Review" },
  { state: "completed", label: "Complete" },
];

// Renders:
//   (o)-----(o)-----(*)-----(-)-----(-)-----(-)
//  Posted  Assign  Working Submit  Review  Done
//
// Legend:
//   (o) = completed step: bg accent-green, white checkmark
//   (*) = current step: bg rose, pulse animation
//   (-) = future step: bg surface-3
//   Line between steps: solid accent-green for completed, dashed surface-3 for future
// Mobile: vertical stack, circles on left, labels right
```

**Step 3: DeliverableViewer.tsx**

Renders job submission content based on job type.

```tsx
interface DeliverableViewerProps {
  job: Job;
}

// For research_brief:
//   - Render submission.summary as formatted text
//   - Simple markdown: ## → h3, ** → bold, - → list item, ``` → pre
//   - Do NOT use an external markdown library. Write a 50-line regex parser:
//       line.startsWith("## ") → <h3>
//       line.startsWith("- ") → <li>
//       line.match(/```(\w*)/) → toggle <pre> block
//       text.replace(/\*\*(.*?)\*\*/g, "<strong>$1</strong>")
//   - Link to deliverable_url if present

// For coding_task:
//   - Render submission.summary as preformatted text
//   - Show gate results table if present:
//     Gate     │ Result │ Message
//     compile  │   OK   │ "0 errors"
//     test     │   OK   │ "47 passed"
//     clippy   │   OK   │ "0 warnings"
//     diff     │   OK   │ "reviewed"
//   - Link to deliverable_url (e.g., PR link)

// For both: show evaluated_at, evaluator, acceptance status if evaluation present
```

**Step 4: JobBoard.tsx**

Main marketplace page.

```
Layout:
  ┌──────────────────────────────────────────────────────────┐
  │ Marketplace                          [Create bounty v]   │
  │                                                          │
  │ Filters: [All] [Open] [Active] [Completed]              │
  │          [Research] [Coding] [Review]                     │
  │          Bounty: [min] - [max] DJ                        │
  │                                                          │
  │ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐      │
  │ │ JobCard      │ │ JobCard      │ │ JobCard      │      │
  │ └──────────────┘ └──────────────┘ └──────────────┘      │
  │ ┌──────────────┐ ┌──────────────┐                       │
  │ │ JobCard      │ │ JobCard      │                       │
  │ └──────────────┘ └──────────────┘                       │
  │                                                          │
  │ Showing 5 of 23 jobs                  [Load more]        │
  └──────────────────────────────────────────────────────────┘
```

- "Create bounty" dropdown: two options -- "Research bounty" → `/app/marketplace/create/research`, "Coding bounty" → `/app/marketplace/create/coding`
- Filters: implemented as URL search params. Each filter button toggles a param. Filter state synced to URL via `useSearchParams()`.
- Job cards: grid layout, `repeat(auto-fill, minmax(320px, 1fr))`. Data from `useJobs(filters)`.
- Click card: navigate to `/app/marketplace/jobs/:id`
- Loading: Skeleton cards (6 placeholders)
- Empty: EmptyState with "No jobs found. Create one?"

Mock tags:
```tsx
// MOCK: job list falls back to 5 hardcoded jobs when /api/jobs unreachable -- depends on B2
```

Mock job data for fallback:
```ts
const MOCK_JOBS: Job[] = [
  {
    id: "job-001",
    job_type: "research_brief",
    title: "Analyze EIP-7702 impact on MEV extraction",
    description: "Research how account abstraction via EIP-7702 changes MEV dynamics...",
    bounty_daeji: 100,
    min_tier: 1,
    deadline: new Date(Date.now() + 86400000 * 3).toISOString(),
    state: "open",
    poster: "will",
    assigned_worker: null,
    submission: null,
    evaluation: null,
    created_at: new Date(Date.now() - 3600000).toISOString(),
    updated_at: new Date(Date.now() - 3600000).toISOString(),
  },
  {
    id: "job-002",
    job_type: "coding_task",
    title: "Add WebSocket heartbeat aggregation to roko-serve",
    description: "Implement heartbeat collection and aggregation endpoint...",
    bounty_daeji: 250,
    min_tier: 1,
    deadline: new Date(Date.now() + 86400000 * 2).toISOString(),
    state: "in_progress",
    poster: "will",
    assigned_worker: "roko-agent-01",
    submission: null,
    evaluation: null,
    created_at: new Date(Date.now() - 7200000).toISOString(),
    updated_at: new Date(Date.now() - 1800000).toISOString(),
  },
  {
    id: "job-003",
    job_type: "code_review",
    title: "Review roko-gate threshold configuration PR",
    description: "Review the adaptive gate threshold changes in orchestrate.rs...",
    bounty_daeji: 50,
    min_tier: 1,
    deadline: new Date(Date.now() + 86400000 * 1).toISOString(),
    state: "assigned",
    poster: "will",
    assigned_worker: "roko-agent-02",
    submission: null,
    evaluation: null,
    created_at: new Date(Date.now() - 10800000).toISOString(),
    updated_at: new Date(Date.now() - 3600000).toISOString(),
  },
  {
    id: "job-004",
    job_type: "research_brief",
    title: "Survey of HDC encoding schemes for agent memory",
    description: "Research hyperdimensional computing encoding methods suitable for roko-neuro...",
    bounty_daeji: 150,
    min_tier: 2,
    deadline: new Date(Date.now() - 3600000).toISOString(), // past deadline
    state: "submitted",
    poster: "will",
    assigned_worker: "roko-agent-01",
    submission: {
      result_hash: "a3f8c2e1b4d7",
      deliverable_url: null,
      summary: "## HDC Encoding Survey\n\nHyperdimensional computing (HDC) encodes data as high-dimensional binary vectors...\n\nKey schemes reviewed:\n1. **Random projection**: Fast, approximate\n2. **Thermometer encoding**: Ordered values\n3. **Circular shift**: Temporal sequences\n\nRecommendation for roko-neuro: circular shift for episode sequences, random projection for semantic similarity.",
      gate_results: null,
      submitted_at: new Date(Date.now() - 1800000).toISOString(),
    },
    evaluation: null,
    created_at: new Date(Date.now() - 86400000).toISOString(),
    updated_at: new Date(Date.now() - 1800000).toISOString(),
  },
  {
    id: "job-005",
    job_type: "coding_task",
    title: "Implement IncrementalTailer for JSONL dashboard reads",
    description: "Add byte-offset tracking to the TUI dashboard to avoid O(N) JSONL re-reads...",
    bounty_daeji: 200,
    min_tier: 1,
    deadline: new Date(Date.now() + 86400000 * 5).toISOString(),
    state: "completed",
    poster: "will",
    assigned_worker: "roko-agent-03",
    submission: {
      result_hash: "c9d4e8f2a1b5",
      deliverable_url: null,
      summary: "Implemented IncrementalTailer in crates/roko-cli/src/tui/jsonl_cursor.rs. Wired into dashboard.rs refresh cycle for episodes.jsonl and efficiency.jsonl. Refresh time drops from O(total_lines) to O(new_lines).",
      gate_results: [
        { gate: "compile", passed: true, message: "0 errors", rung: 1 },
        { gate: "test", passed: true, message: "47 passed", rung: 2 },
        { gate: "clippy", passed: true, message: "0 warnings", rung: 3 },
        { gate: "diff", passed: true, message: "reviewed", rung: 4 },
      ],
      submitted_at: new Date(Date.now() - 86400000 * 0.5).toISOString(),
    },
    evaluation: {
      accepted: true,
      reason: "All gates passed. Performance improvement confirmed.",
      evaluator: "will",
      evaluated_at: new Date(Date.now() - 3600000).toISOString(),
    },
    created_at: new Date(Date.now() - 86400000 * 2).toISOString(),
    updated_at: new Date(Date.now() - 3600000).toISOString(),
  },
];
```

**Step 5: CreateJob.tsx**

Two-column form with live preview. Route param `:type` pre-selects the job type.

```
Layout (desktop):
  ┌──────────────────────────┬──────────────────────────┐
  │ Create bounty            │ Preview                   │
  │                          │                           │
  │ Type: (o) Research       │ ┌──────────────────────┐ │
  │       ( ) Coding         │ │ [RESEARCH]  100 DJ   │ │
  │       ( ) Code review    │ │ Analyze EIP-7702...  │ │
  │       ( ) Custom         │ │                      │ │
  │                          │ │ Depth: Medium        │ │
  │ Title: [_____________]   │ │ Sources: Academic    │ │
  │                          │ │                      │ │
  │ Description:             │ │ What happens next:   │ │
  │ [                      ] │ │ 1. Posted to board   │ │
  │ [                      ] │ │ 2. Agent picks up    │ │
  │ [                      ] │ │ 3. Agent executes    │ │
  │                          │ │ 4. Result delivered   │ │
  │ === Research fields ===  │ │ 5. Bounty released    │ │
  │ Depth: [Shallow|Med|Deep]│ └──────────────────────┘ │
  │ Sources:                 │                           │
  │  [x] Academic            │                           │
  │  [ ] On-chain data       │                           │
  │  [ ] Recent news         │                           │
  │                          │                           │
  │ === Coding fields ===    │                           │
  │ Repo URL: [___________]  │                           │
  │ Acceptance tests:        │                           │
  │ [                      ] │                           │
  │ Target files: [________] │                           │
  │                          │                           │
  │ Bounty: [100] DAEJI      │                           │
  │ Deadline: [72] hours     │                           │
  │                          │                           │
  │ [Post bounty]            │                           │
  └──────────────────────────┴──────────────────────────┘
```

- Type radio: shows/hides type-specific fields. Pre-selected from URL param `:type`.
- Research fields shown when type = research_brief: depth radio (shallow/medium/deep), citation checkboxes (academic, on-chain, recent news).
- Coding fields shown when type = coding_task: repo URL (validated as URL), acceptance tests textarea, target files input.
- Preview panel updates live as form state changes. Uses the JobCard component at the top, plus a "What happens next" step list below.
- "What happens next" steps differ by type:
  - Research: "Posted to job board" → "Research agent picks up" → "Agent runs: `roko research topic`" → "Report delivered and reviewed" → "Bounty released on acceptance"
  - Coding: "Posted to job board" → "Coding agent picks up" → "Agent generates plan from spec" → "Agent executes: `roko plan run`" → "Gate pipeline: compile, test, clippy, diff" → "Code submitted for review" → "Bounty released on acceptance"
- Submit: calls `useCreateJob()` mutation. On success: navigate to `/app/marketplace/jobs/:id`. On error: show inline error below submit button.
- Validation:
  - Title required, min 5 chars
  - Description required, min 20 chars
  - Bounty required, min 1
  - Repo URL must be valid URL when type = coding_task
  - Show validation errors inline below each field

Mock tags:
```tsx
// MOCK: createJob mutation posts to /api/jobs -- depends on B2
// MOCK: on network error, generate a local mock job ID and navigate to detail page showing "pending submission"
```

**Step 6: JobDetail.tsx**

Full detail view for a single job.

```
Layout:
  ┌──────────────────────────────────────────────────────────┐
  │ [<- Back to board]                                        │
  │                                                           │
  │ [RESEARCH]  Analyze EIP-7702 impact on MEV     100 DAEJI │
  │ Posted by will, 2 hours ago                               │
  ├──────────────────────────────────────────────────────────┤
  │ (o)------(o)-------(*)-------(-)-------(-)-------(-)     │
  │ Posted  Assigned  Working  Submitted  Review   Complete  │
  ├───────────────────────────────┬──────────────────────────┤
  │ Description                   │ Agent                     │
  │ Research how account          │ ┌──────────────────────┐ │
  │ abstraction via EIP-7702      │ │ * roko-agent-01      │ │
  │ changes MEV dynamics...       │ │   Tier 1 Deliberate  │ │
  │                               │ │   4 episodes done    │ │
  │ Depth: Medium                 │ │   Gate: 87% pass     │ │
  │ Sources: Academic, On-chain   │ │   [View in Studio]   │ │
  │                               │ └──────────────────────┘ │
  ├───────────────────────────────┴──────────────────────────┤
  │ Deliverable (shown when submitted/completed)              │
  │ ┌──────────────────────────────────────────────────────┐ │
  │ │ ## EIP-7702 and MEV: A Technical Analysis            │ │
  │ │                                                       │ │
  │ │ EIP-7702 introduces a new transaction type...         │ │
  │ │ ...                                                   │ │
  │ └──────────────────────────────────────────────────────┘ │
  └──────────────────────────────────────────────────────────┘
```

- Header: type badge, title (text-2xl), bounty (accent-gold), poster + relative time.
- StatusTimeline component showing current state.
- Left panel: job description, type-specific metadata (depth, sources for research; repo, tests for coding).
- Right panel: agent card when `assigned_worker` is set. Shows agent name, tier, episode count, gate pass rate. "View in Studio" link to `/app/agents?id=<id>`.
- Bottom: DeliverableViewer component, shown only when state is submitted, under_review, or completed.
- Real-time updates: subscribe to WS events where `event.payload.job_id === id`. On event, `queryClient.invalidateQueries(queryKeys.jobs.detail(id))`. Fallback: refetch every 5s via the query's `refetchInterval`.
- Back button: navigates to `/app/marketplace`.

Mock tags:
```tsx
// MOCK: agent detail (tier, episodes, gate rate) from useAgent(job.assigned_worker) -- depends on B7
// MOCK: WS events for job updates -- depends on B8. Falls back to 5s polling.
```

### Done when

- [ ] JobBoard renders at `/app/marketplace` with job cards in a grid
- [ ] Filters work: clicking "Open" shows only open jobs
- [ ] "Create bounty" dropdown navigates to correct create pages
- [ ] CreateJob renders at `/app/marketplace/create` and `/app/marketplace/create/research`
- [ ] Form validates all fields, shows inline errors
- [ ] Preview panel updates live as fields change
- [ ] Submit calls mutation and navigates on success
- [ ] JobDetail renders at `/app/marketplace/jobs/:id`
- [ ] StatusTimeline shows correct active step
- [ ] Agent card appears when job is assigned
- [ ] DeliverableViewer renders markdown for research, gate results for coding
- [ ] Back button works
- [ ] All three pages handle loading, error, empty states
- [ ] Mobile: form stacks to single column, timeline goes vertical

---

## Task A7: Agent Studio + Command + Atelier + Settings pages (8 pages)

**Effort:** 5 hours
**Stream:** A -- Dashboard
**Repo:** nunchi-dashboard at `/Users/will/dev/nunchi/nunchi-dashboard`
**Day:** 2
**Prerequisites:** A1, A2

### Files to create

```
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/agents/AgentOverview.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/agents/AgentStrategy.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/agents/AgentKeys.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/agents/AgentDeploy.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/command/Chat.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/command/Research.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/atelier/Atelier.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/atelier/PrdBrowser.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/atelier/ExecutionMonitor.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/pages/settings/Settings.tsx
```

### Implementation steps

**Step 1: AgentOverview.tsx**

Rewrite of existing `AgentOverviewPanel.tsx` (530 lines). Port the working parts, replace hardcoded data.

- Agent selector dropdown: populated from `useAgents()`. Shows all registered agents, not a hardcoded list.
- Selected agent detail:
  - Name, role, status, tier
  - Stats row: episodes (`agent.episode_count`), gate pass rate (`agent.gate_pass_rate`), cost (`agent.cost_cumulative`)
  - Heartbeat: last heartbeat time, status dot
  - Recent episodes table: from `/api/agents/:id/episodes` (or mock 5 entries)
  - Cognitive trace: from `/api/agents/:id/trace` (or mock text showing tier transitions)
- Remove all hardcoded prediction data from existing panel.

Mock tags:
```tsx
// MOCK: recent episodes hardcoded to 5 entries -- wire to GET /api/agents/:id/episodes
// MOCK: cognitive trace shows "T1 → T0 → T1 → T2 → T1" pattern -- wire to GET /api/agents/:id/trace
```

**Step 2: AgentStrategy.tsx**

Config form for agent strategy parameters. Port from existing `StrategyPanel.tsx` (900 lines).

- All config changes POST to `/api/config` on save.
- Fields: model routing preferences, context budget, gate thresholds, learning rate.
- Each field: label + input/slider + current value display.
- "Save" button at bottom. Show toast on success/error.

Mock tags:
```tsx
// MOCK: config values from useConfig() -- falls back to defaults until /api/config works
// MOCK: save POST to /api/config -- depends on B4
```

**Step 3: AgentKeys.tsx**

API key management. Port from existing `KeysPanel.tsx` (61 lines -- almost nothing there).

- Table: key name, prefix (first 8 chars), created date, status, revoke button.
- "Create key" button opens modal with name input. On submit: POST `/api/keys`. Show full key once, then mask.
- Data from `/api/keys`.

Mock tags:
```tsx
// MOCK: key list hardcoded to 1 entry -- wire to GET /api/keys
// MOCK: create key returns mock key -- wire to POST /api/keys
```

**Step 4: AgentDeploy.tsx**

Agent deployment configuration. Port from existing `DeployPanel.tsx` (249 lines).

- Show deployment status, binding code generation, environment selector.
- Generate binding code via API call.

Mock tags:
```tsx
// MOCK: deployment status always "not deployed" -- wire to GET /api/deployments/:id
// MOCK: binding code generated client-side -- wire to POST /api/deployments/bind
```

**Step 5: Chat.tsx**

Wire to real agent messaging. Port from existing chat logic in `AskPanel` (if present in the codebase).

- Message input at bottom, message history scrolling above.
- On send: POST to roko-serve `/api/agents/:id/message` with the message text.
- Response appears in history.
- Agent selector in top bar.

Mock tags:
```tsx
// MOCK: agent responses echo back "Agent received: <message>" -- wire to POST /api/agents/:id/message (real dispatch via B5/B6)
```

**Step 6: Research.tsx**

Research interface. Port from existing `ResearchPanel.tsx` (442 lines).

- Topic input, depth selector, submit button.
- On submit: POST `/api/research/topic` with topic and depth.
- Poll for results or subscribe to WS events.
- Render completed research report with DeliverableViewer.

Mock tags:
```tsx
// MOCK: research submission falls back to showing a pre-written report -- wire to POST /api/research/topic
```

**Step 7: Atelier.tsx + PrdBrowser.tsx + ExecutionMonitor.tsx**

Atelier is the workspace dashboard.

`Atelier.tsx`:
- Top: workspace status from `/api/status` (agent count, plan count, task count).
- Two columns: left shows recent PRDs, right shows active plans.
- Quick actions: "Create PRD", "Run plan", "View logs".

`PrdBrowser.tsx`:
- PRD list from `usePrds()`. Each item: slug, title, status badge, created date.
- Click item: expand to show full PRD content (fetched from `/api/prds/:slug`).

`ExecutionMonitor.tsx`:
- Live plan execution view. Shows currently running plans from `usePlans()` filtered to `status: "running"`.
- Each plan: progress bar, task list with live status updates.
- WS subscription for real-time updates.

Mock tags:
```tsx
// MOCK: workspace status uses counts from useHealth() -- depends on B4
// MOCK: PRD content shows slug + title only until /api/prds/:slug returns full content
// MOCK: execution monitor polls every 5s as WS fallback -- depends on B8
```

**Step 8: Settings.tsx**

Config editor with theme selector.

- Config: rendered as key-value pairs from `useConfig()`. Editable fields.
- Theme: for now, dark-mode-only. Show toggle that's disabled with "Light mode coming soon".
- Notifications: placeholder section.
- Save button: POST to `/api/config`.

Mock tags:
```tsx
// MOCK: config fields from useConfig() -- depends on B4
```

### Done when

- [ ] All 10 pages render at their routes without errors
- [ ] AgentOverview shows real agent data from useAgents() (or mock fallback)
- [ ] Chat sends and receives messages (or mock echo)
- [ ] Research submits and shows results (or mock report)
- [ ] Atelier shows workspace stats
- [ ] PrdBrowser lists PRDs
- [ ] ExecutionMonitor shows running plans
- [ ] Settings renders config fields
- [ ] No console errors on any page

---

## Task A8: Integration wiring + visual polish

**Effort:** 4 hours
**Stream:** A -- Dashboard
**Repo:** nunchi-dashboard at `/Users/will/dev/nunchi/nunchi-dashboard`
**Day:** 3
**Prerequisites:** A1-A7

### Files to modify

```
All page files from A3-A7 (under /Users/will/dev/nunchi/nunchi-dashboard/src/)
/Users/will/dev/nunchi/nunchi-dashboard/src/layouts/AppLayout.tsx
/Users/will/dev/nunchi/nunchi-dashboard/src/services/api.ts (if endpoint adjustments needed)
/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/tokens.css (contrast fixes)
```

### Implementation steps

**Step 1: Connect WebSocket globally**

In `AppLayout.tsx`, call `connectWs()` on mount, `disconnectWs()` on unmount. All pages that need real-time updates read from `useWsStore().eventBuffer` and call `queryClient.invalidateQueries()` when relevant events arrive.

Relevant event types and their query invalidations:
```ts
const WS_INVALIDATION_MAP: Record<string, () => void> = {
  "job_created": () => qc.invalidateQueries({ queryKey: queryKeys.jobs.all }),
  "job_assigned": () => qc.invalidateQueries({ queryKey: queryKeys.jobs.all }),
  "job_submitted": () => qc.invalidateQueries({ queryKey: queryKeys.jobs.all }),
  "job_completed": () => qc.invalidateQueries({ queryKey: queryKeys.jobs.all }),
  "plan_started": () => qc.invalidateQueries({ queryKey: queryKeys.plans.all }),
  "plan_completed": () => qc.invalidateQueries({ queryKey: queryKeys.plans.all }),
  "agent_heartbeat": () => qc.invalidateQueries({ queryKey: queryKeys.agents.all }),
  "agent_registered": () => qc.invalidateQueries({ queryKey: queryKeys.agents.all }),
};
```

**Step 2: Visual polish pass**

Run through every page and check:
- Loading: every data-dependent section shows `<Skeleton>` while loading
- Error: every query failure shows `<ErrorState>` with retry button
- Empty: every empty list shows `<EmptyState>` with descriptive message
- Spacing: consistent `var(--sp-N)` gaps between sections
- Typography: headings use `text-xl` or `text-2xl`, body uses `text-base`, metadata uses `text-sm`
- Contrast: verify `fg-primary` on `bg-surface-0` meets 4.5:1 (E8E4DE on 0C0C10 = ~12:1, good). Verify `fg-secondary` on `bg-surface-0` meets 4.5:1 (9B9590 on 0C0C10 = ~5.8:1, good). Verify `fg-muted` for large text only (6B655F on 0C0C10 = ~3.2:1, acceptable for large text).

**Step 3: Navigation completeness**

- Verify every left nav item leads to a rendering page
- Verify every back button works
- Verify breadcrumbs show correct path
- Verify keyboard navigation: Tab through interactive elements, Enter activates buttons
- Test at three viewports: 1440px (desktop), 768px (tablet), 375px (mobile)

**Step 4: Remove dead code**

- Delete unused imports across all new files
- Remove any remaining `console.log` statements
- Verify `npm run typecheck` passes
- Verify `npm run lint` passes (fix all ESLint errors)
- Verify `npm run build` succeeds (no build errors)

### Done when

- [ ] WebSocket connects on AppLayout mount and reconnects on disconnect
- [ ] WS events trigger query invalidation for relevant data types
- [ ] Every page has loading skeletons, error states, and empty states
- [ ] No contrast ratio below 4.5:1 for normal text
- [ ] Every nav item navigates to a rendering page
- [ ] Breadcrumbs work on all routes
- [ ] `npm run typecheck` passes
- [ ] `npm run lint` passes (0 errors)
- [ ] `npm run build` succeeds
- [ ] Mobile layout (375px) does not break on any page
- [ ] No console errors on any route transition

---

## Task A9: Demo flow rehearsal

**Effort:** 2 hours
**Stream:** A -- Dashboard
**Repo:** nunchi-dashboard at `/Users/will/dev/nunchi/nunchi-dashboard`
**Day:** 3
**Prerequisites:** A8, B10

### Files to modify

```
Any files under /Users/will/dev/nunchi/nunchi-dashboard/src/ or /Users/will/dev/nunchi/roko/roko/crates/ that break during the rehearsal
```

### Implementation steps

Run through the three demo flows manually:

**Flow 1: First visitor experience**
1. Open `/` -- landing page loads, gradient animates, stats count up
2. Read architecture section, click each tier, see detail panels expand
3. Scroll to context auction, see bars animate in
4. Scroll to stigmergy, see particles move
5. Click "Launch dashboard"
6. Dashboard loads at `/app`, left nav visible, agent cards or empty state shown

**Flow 2: Research bounty**
1. Navigate to `/app/marketplace`
2. Click "Create bounty" → "Research bounty"
3. Fill in: topic "Impact of EIP-7702 on MEV extraction", depth "Medium", sources "Academic", bounty 100
4. See preview update live
5. Click "Post bounty" → navigates to job detail page
6. Watch status timeline progress (if backend running: open → assigned → in_progress → submitted → completed)
7. See deliverable viewer show the research report

**Flow 3: Coding bounty**
1. Navigate to `/app/marketplace`
2. Click "Create bounty" → "Coding bounty"
3. Fill in: repo URL, description, acceptance tests, bounty 250
4. Click "Post bounty" → navigates to job detail page
5. Watch status timeline progress
6. See gate results table in deliverable viewer

Fix everything that breaks. This is the final task.

### Done when

- [ ] All three flows complete without errors
- [ ] No broken pages at any point in any flow
- [ ] Loading and error states appear correctly when backend is unavailable
- [ ] Mock data renders cleanly where real data is not available
- [ ] The demo tells a coherent story from landing to completion

---

# Stream B: Roko backend stabilization + jobs

---

## Task B1: Job types and store

**Effort:** 3 hours
**Stream:** B -- Backend
**Repo:** roko at `/Users/will/dev/nunchi/roko/roko`
**Day:** 1
**Prerequisites:** none

### Files to create

```
crates/roko-core/src/jobs.rs
```

### Files to modify

```
crates/roko-core/src/lib.rs — add `pub mod jobs;`
crates/roko-core/Cargo.toml — ensure chrono, serde, serde_json dependencies present (already are)
```

### Implementation steps

**Step 1: Define job types**

Create `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/jobs.rs`:

```rust
//! Job types, state machine, and file-backed store.
//!
//! A job represents a unit of work posted to the Nunchi marketplace.
//! Jobs follow a linear state machine: Open → Assigned → InProgress →
//! Submitted → UnderReview → Completed (or Rejected/Expired).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    ResearchBrief,
    CodingTask,
    CodeReview,
    Custom,
}

impl std::fmt::Display for JobType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ResearchBrief => write!(f, "research_brief"),
            Self::CodingTask => write!(f, "coding_task"),
            Self::CodeReview => write!(f, "code_review"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Open,
    Assigned,
    InProgress,
    Submitted,
    UnderReview,
    Completed,
    Rejected,
    Expired,
}

impl JobState {
    /// Returns true for terminal states.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Rejected | Self::Expired)
    }

    /// Valid transitions from this state.
    pub fn valid_transitions(self) -> &'static [JobState] {
        match self {
            Self::Open => &[Self::Assigned, Self::Expired],
            Self::Assigned => &[Self::InProgress, Self::Open], // can unassign
            Self::InProgress => &[Self::Submitted],
            Self::Submitted => &[Self::UnderReview],
            Self::UnderReview => &[Self::Completed, Self::Rejected],
            Self::Completed | Self::Rejected | Self::Expired => &[],
        }
    }

    /// Check if transitioning to `next` is valid.
    pub fn can_transition_to(self, next: Self) -> bool {
        self.valid_transitions().contains(&next)
    }
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub job_type: JobType,
    pub title: String,
    pub description: String,
    pub spec_hash: Option<String>,
    pub bounty_daeji: f64,
    pub min_tier: u8,
    pub deadline: DateTime<Utc>,
    pub state: JobState,
    pub poster: String,
    pub assigned_worker: Option<String>,
    pub submission: Option<JobSubmission>,
    pub evaluation: Option<JobEvaluation>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Type-specific metadata (research depth, repo URL, etc.)
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSubmission {
    pub result_hash: String,
    pub deliverable_url: Option<String>,
    pub summary: String,
    pub gate_results: Option<Vec<JobGateResult>>,
    pub submitted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobGateResult {
    pub gate: String,
    pub passed: bool,
    pub message: String,
    pub rung: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEvaluation {
    pub accepted: bool,
    pub reason: String,
    pub evaluator: String,
    pub evaluated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateJobRequest {
    pub job_type: JobType,
    pub title: String,
    pub description: String,
    pub bounty_daeji: f64,
    pub min_tier: Option<u8>,
    pub deadline_hours: Option<u64>,
    pub poster: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JobFilter {
    pub state: Option<JobState>,
    pub job_type: Option<JobType>,
    pub poster: Option<String>,
    pub min_bounty: Option<f64>,
    pub max_bounty: Option<f64>,
    pub limit: Option<usize>,
}

// ---------------------------------------------------------------------------
// Store trait
// ---------------------------------------------------------------------------

/// Trait for job persistence backends.
pub trait JobStore: Send + Sync {
    fn list_jobs(&self, filter: &JobFilter) -> anyhow::Result<Vec<Job>>;
    fn get_job(&self, id: &str) -> anyhow::Result<Option<Job>>;
    fn create_job(&self, req: CreateJobRequest) -> anyhow::Result<Job>;
    fn update_job(&self, id: &str, f: &dyn Fn(&mut Job)) -> anyhow::Result<Job>;
    fn count_by_state(&self) -> anyhow::Result<std::collections::HashMap<JobState, usize>>;
}

// ---------------------------------------------------------------------------
// File-backed implementation
// ---------------------------------------------------------------------------

/// Stores each job as a separate JSON file in `{base_dir}/{id}.json`.
pub struct FileJobStore {
    base_dir: PathBuf,
}

impl FileJobStore {
    pub fn new(base_dir: impl AsRef<Path>) -> anyhow::Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&base_dir)?;
        Ok(Self { base_dir })
    }

    fn job_path(&self, id: &str) -> PathBuf {
        self.base_dir.join(format!("{}.json", id))
    }

    fn read_job(&self, path: &Path) -> anyhow::Result<Job> {
        let contents = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&contents)?)
    }

    fn write_job(&self, job: &Job) -> anyhow::Result<()> {
        let path = self.job_path(&job.id);
        let contents = serde_json::to_string_pretty(job)?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    fn generate_id() -> String {
        format!("job-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("0000"))
    }
}

impl JobStore for FileJobStore {
    fn list_jobs(&self, filter: &JobFilter) -> anyhow::Result<Vec<Job>> {
        let mut jobs = Vec::new();
        let entries = std::fs::read_dir(&self.base_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                if let Ok(job) = self.read_job(&path) {
                    // Apply filters
                    if let Some(ref state) = filter.state {
                        if job.state != *state { continue; }
                    }
                    if let Some(ref jt) = filter.job_type {
                        if job.job_type != *jt { continue; }
                    }
                    if let Some(ref poster) = filter.poster {
                        if job.poster != *poster { continue; }
                    }
                    if let Some(min_b) = filter.min_bounty {
                        if job.bounty_daeji < min_b { continue; }
                    }
                    if let Some(max_b) = filter.max_bounty {
                        if job.bounty_daeji > max_b { continue; }
                    }
                    jobs.push(job);
                }
            }
        }
        // Sort by created_at descending
        jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        if let Some(limit) = filter.limit {
            jobs.truncate(limit);
        }
        Ok(jobs)
    }

    fn get_job(&self, id: &str) -> anyhow::Result<Option<Job>> {
        let path = self.job_path(id);
        if path.exists() {
            Ok(Some(self.read_job(&path)?))
        } else {
            Ok(None)
        }
    }

    fn create_job(&self, req: CreateJobRequest) -> anyhow::Result<Job> {
        let now = Utc::now();
        let deadline_hours = req.deadline_hours.unwrap_or(72);
        let job = Job {
            id: Self::generate_id(),
            job_type: req.job_type,
            title: req.title,
            description: req.description,
            spec_hash: None,
            bounty_daeji: req.bounty_daeji,
            min_tier: req.min_tier.unwrap_or(1),
            deadline: now + chrono::Duration::hours(deadline_hours as i64),
            state: JobState::Open,
            poster: req.poster.unwrap_or_else(|| "anonymous".to_string()),
            assigned_worker: None,
            submission: None,
            evaluation: None,
            created_at: now,
            updated_at: now,
            metadata: req.metadata.unwrap_or(serde_json::Value::Null),
        };
        self.write_job(&job)?;
        Ok(job)
    }

    fn update_job(&self, id: &str, f: &dyn Fn(&mut Job)) -> anyhow::Result<Job> {
        let path = self.job_path(id);
        anyhow::ensure!(path.exists(), "job not found: {}", id);
        let mut job = self.read_job(&path)?;
        f(&mut job);
        job.updated_at = Utc::now();
        self.write_job(&job)?;
        Ok(job)
    }

    fn count_by_state(&self) -> anyhow::Result<std::collections::HashMap<JobState, usize>> {
        let all = self.list_jobs(&JobFilter::default())?;
        let mut counts = std::collections::HashMap::new();
        for job in &all {
            *counts.entry(job.state).or_insert(0) += 1;
        }
        Ok(counts)
    }
}
```

**Step 2: Wire into roko-core/src/lib.rs**

Add `pub mod jobs;` to the module list in `crates/roko-core/src/lib.rs`. Place it alphabetically among existing modules. This is required — without it, no other crate can import from `roko_core::jobs`.

**Step 3: Verify uuid dependency**

Check `crates/roko-core/Cargo.toml` for `uuid`. If missing, add:
```toml
uuid = { version = "1", features = ["v4"] }
```

**Step 4: Verify compilation**

```bash
cd /Users/will/dev/nunchi/roko/roko
cargo check -p roko-core
```

### Mock tags

None. This is pure backend code.

### Done when

- [ ] `cargo check -p roko-core` passes
- [ ] `cargo +nightly fmt --all` passes
- [ ] `JobType`, `JobState`, `Job`, `JobSubmission`, `JobEvaluation` structs defined
- [ ] `JobState::can_transition_to()` validates state machine transitions
- [ ] `FileJobStore` creates `.roko/jobs/` directory
- [ ] `FileJobStore::create_job()` writes a JSON file and returns the job
- [ ] `FileJobStore::list_jobs()` reads all JSON files and applies filters
- [ ] `FileJobStore::update_job()` modifies and persists a job
- [ ] `FileJobStore::count_by_state()` returns counts
- [ ] No clippy warnings (`cargo clippy --workspace --no-deps -- -D warnings`)

---

## Task B2: Job API routes

**Effort:** 3 hours
**Stream:** B -- Backend
**Repo:** roko at `/Users/will/dev/nunchi/roko/roko`
**Day:** 1 (parallel-safe with B1 after B1 types are defined)
**Prerequisites:** B1

### Files to create

```
crates/roko-serve/src/routes/jobs.rs
```

### Files to modify

```
crates/roko-serve/src/routes/mod.rs — add `mod jobs;` and `.merge(jobs::routes())`
crates/roko-serve/src/state.rs — add `job_store: Arc<dyn JobStore>` to AppState
crates/roko-serve/src/lib.rs — initialize FileJobStore when building AppState
crates/roko-serve/Cargo.toml — add roko-core dependency if not present
```

### Implementation steps

**Step 1: Add job store to AppState**

In `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/state.rs`, add a field:

```rust
pub job_store: Arc<dyn roko_core::jobs::JobStore>,
```

Initialize it in the AppState constructor with:

```rust
let job_store = Arc::new(
    roko_core::jobs::FileJobStore::new(data_dir.join("jobs"))
        .expect("Failed to initialize job store")
);
```

**Step 2: Create route handlers** (`crates/roko-serve/src/routes/jobs.rs`)

```rust
//! Job marketplace API routes.

use std::sync::Arc;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use roko_core::jobs::*;
use serde::{Deserialize, Serialize};

use crate::state::AppState;

// NOTE: Check the existing routes for the path parameter syntax.
// If existing routes use `:id` (axum 0.7), use `:id` throughout.
// If they use `{id}` (axum 0.8+), use `{id}`.
// Look at `crates/roko-serve/src/routes/plans.rs` for the pattern used in this codebase.
// The example below uses `{id}` — change to `:id` if that is what plans.rs uses.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/jobs", get(list_jobs).post(create_job))
        .route("/jobs/stats", get(job_stats))
        .route("/jobs/{id}", get(get_job))
        .route("/jobs/{id}/assign", post(assign_job))
        .route("/jobs/{id}/start", post(start_job))
        .route("/jobs/{id}/submit", post(submit_job))
        .route("/jobs/{id}/evaluate", post(evaluate_job))
        .route("/jobs/{id}/cancel", post(cancel_job))
}

// --- Query params ---

#[derive(Debug, Deserialize)]
struct ListJobsQuery {
    state: Option<String>,
    job_type: Option<String>,
    poster: Option<String>,
    min_bounty: Option<f64>,
    max_bounty: Option<f64>,
    limit: Option<usize>,
}

// --- Handlers ---

async fn list_jobs(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListJobsQuery>,
) -> Result<Json<Vec<Job>>, StatusCode> {
    let filter = JobFilter {
        state: q.state.and_then(|s| serde_json::from_value(
            serde_json::Value::String(s)
        ).ok()),
        job_type: q.job_type.and_then(|s| serde_json::from_value(
            serde_json::Value::String(s)
        ).ok()),
        poster: q.poster,
        min_bounty: q.min_bounty,
        max_bounty: q.max_bounty,
        limit: q.limit,
    };
    state.job_store.list_jobs(&filter)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn create_job(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateJobRequest>,
) -> Result<(StatusCode, Json<Job>), StatusCode> {
    let job = state.job_store.create_job(req)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Emit WS event
    // NOTE: The `EventBus<ServerEvent>` is always present in `AppState`. Call
    // `state.event_bus.publish(event)` directly — no `Option` unwrapping needed.
    state.event_bus.publish(crate::events::ServerEvent::JobCreated {
        job_id: job.id.clone(),
        job_type: job.job_type.to_string(),
    });

    Ok((StatusCode::CREATED, Json(job)))
}

async fn get_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Job>, StatusCode> {
    state.job_store.get_job(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn job_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<std::collections::HashMap<String, usize>>, StatusCode> {
    let counts = state.job_store.count_by_state()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let result: std::collections::HashMap<String, usize> = counts
        .into_iter()
        .map(|(k, v)| (format!("{:?}", k).to_lowercase(), v))
        .collect();
    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
struct AssignRequest {
    worker: String,
}

async fn assign_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<AssignRequest>,
) -> Result<Json<Job>, StatusCode> {
    let job = state.job_store.update_job(&id, &|job| {
        if job.state.can_transition_to(JobState::Assigned) {
            job.state = JobState::Assigned;
            job.assigned_worker = Some(req.worker.clone());
        }
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state.event_bus.publish(crate::events::ServerEvent::JobAssigned {
        job_id: job.id.clone(),
        worker: req.worker,
    });

    Ok(Json(job))
}

async fn start_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Job>, StatusCode> {
    let job = state.job_store.update_job(&id, &|job| {
        if job.state.can_transition_to(JobState::InProgress) {
            job.state = JobState::InProgress;
        }
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state.event_bus.publish(crate::events::ServerEvent::JobStarted {
        job_id: job.id.clone(),
    });

    Ok(Json(job))
}

#[derive(Debug, Deserialize)]
struct SubmitRequest {
    result_hash: String,
    deliverable_url: Option<String>,
    summary: String,
    gate_results: Option<Vec<JobGateResult>>,
}

async fn submit_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<SubmitRequest>,
) -> Result<Json<Job>, StatusCode> {
    let job = state.job_store.update_job(&id, &|job| {
        if job.state.can_transition_to(JobState::Submitted) {
            job.state = JobState::Submitted;
            job.submission = Some(JobSubmission {
                result_hash: req.result_hash.clone(),
                deliverable_url: req.deliverable_url.clone(),
                summary: req.summary.clone(),
                gate_results: req.gate_results.clone(),
                submitted_at: chrono::Utc::now(),
            });
        }
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state.event_bus.publish(crate::events::ServerEvent::JobSubmitted {
        job_id: job.id.clone(),
    });

    Ok(Json(job))
}

#[derive(Debug, Deserialize)]
struct EvaluateRequest {
    accepted: bool,
    reason: String,
    evaluator: Option<String>,
}

async fn evaluate_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<EvaluateRequest>,
) -> Result<Json<Job>, StatusCode> {
    let new_state = if req.accepted { JobState::Completed } else { JobState::Rejected };
    let job = state.job_store.update_job(&id, &|job| {
        // Transition through UnderReview first if needed
        if job.state == JobState::Submitted {
            job.state = JobState::UnderReview;
        }
        if job.state.can_transition_to(new_state) {
            job.state = new_state;
            job.evaluation = Some(JobEvaluation {
                accepted: req.accepted,
                reason: req.reason.clone(),
                evaluator: req.evaluator.clone().unwrap_or_else(|| "system".to_string()),
                evaluated_at: chrono::Utc::now(),
            });
        }
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let event = if req.accepted {
        crate::events::ServerEvent::JobCompleted { job_id: job.id.clone() }
    } else {
        crate::events::ServerEvent::JobRejected { job_id: job.id.clone() }
    };
    state.event_bus.publish(event);

    Ok(Json(job))
}

async fn cancel_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Job>, StatusCode> {
    let job = state.job_store.update_job(&id, &|job| {
        if !job.state.is_terminal() {
            job.state = JobState::Expired;
        }
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(job))
}
```

**Step 3: Add WebSocket event types**

In `crates/roko-serve/src/events.rs`, add the job variants to the existing `ServerEvent` enum. The enum uses `#[serde(tag = "type", rename_all = "snake_case")]`. Add AFTER the existing variants, BEFORE the closing brace:

```rust
    // Job lifecycle events (add after existing variants, before closing `}`)
    JobCreated { job_id: String, job_type: String },
    JobAssigned { job_id: String, worker: String },
    JobStarted { job_id: String },
    JobSubmitted { job_id: String },
    JobCompleted { job_id: String },
    JobRejected { job_id: String },
```

Do NOT replace the entire enum — append to it. If `events.rs` does not yet exist, create it with the full enum including both the job variants above and a placeholder for future system events.

**Step 4: Register routes**

In `crates/roko-serve/src/routes/mod.rs`:
1. Add `pub mod jobs;` at the top of the file alongside the other module declarations (e.g., near `pub mod plans;`, `pub mod agents;`, etc.). Without this line the module will not compile.
2. Inside the `build_router` function, locate the `let api = Router::new()` chain. Add `.merge(jobs::routes())` after the last existing `.merge(...)` call.

**Step 5: Verify**

```bash
cargo check -p roko-serve
```

### Mock tags

None in backend code. The `EventBus<ServerEvent>` is always present in `AppState` — call `state.event_bus.publish(event)` directly. No `Option` unwrapping needed.

### Done when

- [ ] `cargo check -p roko-serve` passes
- [ ] `cargo +nightly fmt --all` passes
- [ ] POST /api/jobs creates a job file in `.roko/jobs/`
- [ ] GET /api/jobs returns the created job
- [ ] GET /api/jobs?state=open filters correctly
- [ ] GET /api/jobs/:id returns a single job
- [ ] POST /api/jobs/:id/assign transitions to Assigned
- [ ] POST /api/jobs/:id/start transitions to InProgress
- [ ] POST /api/jobs/:id/submit transitions to Submitted with deliverable
- [ ] POST /api/jobs/:id/evaluate transitions to Completed or Rejected
- [ ] POST /api/jobs/:id/cancel transitions to Expired
- [ ] Invalid state transitions are rejected (no-op, not error)
- [ ] GET /api/jobs/stats returns count-by-state map
- [ ] WS events emit on each state change
- [ ] No clippy warnings (`cargo clippy --workspace --no-deps -- -D warnings`)

---

## Task B3: Incremental file watchers

**Effort:** 2 hours
**Stream:** B -- Backend
**Repo:** roko at `/Users/will/dev/nunchi/roko/roko`
**Day:** 1 (parallel-safe with B1/B2)
**Prerequisites:** none

### Files to create

```
(none -- extending existing file)
```

### Files to modify

```
crates/roko-cli/src/tui/jsonl_cursor.rs — add IncrementalTailer
crates/roko-cli/src/tui/dashboard.rs — use IncrementalTailer instead of full re-reads
```

### Implementation steps

**Step 1: Add IncrementalTailer to jsonl_cursor.rs**

```rust
/// Tracks byte offset in a file and reads only new lines since last read.
/// Avoids the O(N) cost of re-reading entire JSONL files on every refresh.
pub struct IncrementalTailer {
    path: PathBuf,
    last_offset: u64,
    last_mtime: Option<std::time::SystemTime>,
}

impl IncrementalTailer {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            last_offset: 0,
            last_mtime: None,
        }
    }

    /// Read lines added since the last call. Returns empty vec if file
    /// has not been modified.
    pub fn read_new_lines(&mut self) -> Vec<String> {
        let meta = match std::fs::metadata(&self.path) {
            Ok(m) => m,
            Err(_) => return Vec::new(),
        };

        let mtime = meta.modified().ok();
        if mtime == self.last_mtime && meta.len() <= self.last_offset {
            return Vec::new();
        }

        let mut file = match std::fs::File::open(&self.path) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };

        use std::io::{BufRead, Seek, SeekFrom};
        if self.last_offset > 0 {
            let _ = file.seek(SeekFrom::Start(self.last_offset));
        }

        let reader = std::io::BufReader::new(&file);
        let mut lines = Vec::new();
        let mut offset = self.last_offset;

        for line in reader.lines() {
            match line {
                Ok(l) => {
                    offset += l.len() as u64 + 1; // +1 for newline
                    if !l.is_empty() {
                        lines.push(l);
                    }
                }
                Err(_) => break,
            }
        }

        self.last_offset = offset;
        self.last_mtime = mtime;
        lines
    }

    /// Reset to beginning of file (e.g., after file rotation).
    pub fn reset(&mut self) {
        self.last_offset = 0;
        self.last_mtime = None;
    }
}
```

**Step 2: Apply to dashboard.rs refresh**

In `dashboard.rs`, identify the 7 JSONL data sources that are currently read in full on every refresh cycle. For each one:

1. Add an `IncrementalTailer` field to the dashboard state struct.
2. On refresh, call `tailer.read_new_lines()` instead of reading the entire file.
3. Parse only the new lines and append to the existing in-memory data.
4. Keep the full data in memory for display purposes.

Target files (from CLAUDE.md items 70-78):
- `.roko/episodes.jsonl` (episodes)
- `.roko/signals.jsonl` (signals)
- `.roko/learn/efficiency.jsonl` (efficiency events)
- `.roko/learn/gate-thresholds.json` (not JSONL -- keep as full read, small file)
- `.roko/learn/cascade-router.json` (not JSONL -- keep as full read, small file)
- gate verdict outputs (from task output files)
- task output logs

For non-JSONL files (small JSON configs), keep the existing full-read approach. Only apply `IncrementalTailer` to JSONL files that grow over time.

### Done when

- [ ] `IncrementalTailer` reads only new bytes since last call
- [ ] `IncrementalTailer::read_new_lines()` returns empty vec when file unchanged
- [ ] Dashboard refresh uses `IncrementalTailer` for at least episodes and efficiency JSONL
- [ ] Performance: refresh cycle time drops from O(total_lines) to O(new_lines)
- [ ] `cargo check -p roko-cli` passes
- [ ] `cargo +nightly fmt --all` passes
- [ ] No clippy warnings (`cargo clippy --workspace --no-deps -- -D warnings`)

---

## Task B4: Server state persistence

**Effort:** 1 hour
**Stream:** B -- Backend
**Repo:** roko at `/Users/will/dev/nunchi/roko/roko`
**Day:** 1 (parallel-safe with B1-B3)
**Prerequisites:** none

### Files to modify

```
crates/roko-serve/src/state.rs
crates/roko-serve/src/lib.rs (or wherever AppState is initialized)
```

### Implementation steps

**Step 1: Define serializable snapshot**

// NOTE: Check state.rs to find the actual type of `discovered_agents` in AppState.
// If it is `Vec<AgentRegistrationRecord>` (or similar), use that type here instead
// of `Vec<String>` to avoid serialization mismatches. Update the snapshot struct to match.
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerStateSnapshot {
    pub discovered_agents: Vec<String>, // adapt to match AppState field type — may be Vec<AgentRegistrationRecord>
    pub active_plan_ids: Vec<String>,
    pub operation_log: Vec<String>,    // last 100 operation IDs
    pub deployment_ids: Vec<String>,
    pub saved_at: chrono::DateTime<chrono::Utc>,
}
```

**Step 2: Implement save/restore**

// NOTE: `AppState` uses `tokio::sync::RwLock`, not `std::sync::RwLock`.
// All lock operations are async — use `.read().await` and `.write().await`.
// There is no `.read().unwrap()` for tokio locks; the await never fails (tokio
// RwLock does not poison). Remove all `.unwrap()` calls on lock acquisitions.
```rust
impl AppState {
    const STATE_FILE: &str = ".roko/state/server-state.json";

    // NOTE: discovered_agents type — check state.rs. If it is a
    // `Vec<AgentRegistrationRecord>`, serialize the full struct, not just IDs.
    // Change `Vec<String>` in `ServerStateSnapshot.discovered_agents` to match.
    pub async fn save_snapshot(&self) -> anyhow::Result<()> {
        let snapshot = ServerStateSnapshot {
            discovered_agents: self.discovered_agents.read().await.clone(),
            active_plan_ids: self.active_plans.read().await.keys().cloned().collect(),
            operation_log: self.operations.read().await
                .iter().rev().take(100).cloned().collect(),
            deployment_ids: self.deployments.read().await.keys().cloned().collect(),
            saved_at: chrono::Utc::now(),
        };
        let path = self.data_dir.join("state/server-state.json");
        std::fs::create_dir_all(path.parent().unwrap())?;
        let json = serde_json::to_string_pretty(&snapshot)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    pub async fn restore_snapshot(&self) -> anyhow::Result<()> {
        let path = self.data_dir.join("state/server-state.json");
        if !path.exists() {
            return Ok(());
        }
        let json = std::fs::read_to_string(&path)?;
        let snapshot: ServerStateSnapshot = serde_json::from_str(&json)?;
        // Restore what we can
        *self.discovered_agents.write().await = snapshot.discovered_agents;
        tracing::info!("Restored server state from {}", snapshot.saved_at);
        Ok(())
    }
}
```

**Step 3: Auto-save on changes**

Add a debounced save that triggers 5 seconds after any state mutation. Use a `tokio::sync::Notify` + background task pattern:

```rust
// In server startup, spawn:
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        if let Err(e) = state_clone.save_snapshot() {
            tracing::warn!("Failed to save server state: {}", e);
        }
    }
});
```

Every 30 seconds is sufficient for demo purposes. Full debounced-on-change can come later.

**Step 4: Restore on startup**

In the server initialization code, after creating AppState, call `state.restore_snapshot()`.

### Done when

- [ ] Server state writes to `.roko/state/server-state.json` every 30 seconds
- [ ] On startup, server restores discovered agents from the snapshot
- [ ] State file is valid JSON and can be inspected manually
- [ ] `cargo check -p roko-serve` passes
- [ ] `cargo +nightly fmt --all` passes
- [ ] No clippy warnings (`cargo clippy --workspace --no-deps -- -D warnings`)

---

## Task B5: Research job execution pipeline

**Effort:** 4 hours
**Stream:** B -- Backend
**Repo:** roko at `/Users/will/dev/nunchi/roko/roko`
**Day:** 2
**Prerequisites:** B1, B2

### Files to create

```
crates/roko-cli/src/job_runner.rs
```

### Files to modify

```
crates/roko-cli/src/orchestrate.rs — add job polling call in the main loop
crates/roko-cli/src/lib.rs — add `pub mod job_runner;` (required — without this the module won't compile)
crates/roko-cli/Cargo.toml — ensure reqwest has `json` feature; add md5 = "0.7" (or use sha2 from workspace)
```

### Dependency note

Before implementing, check `crates/roko-cli/Cargo.toml`:
1. Ensure `reqwest` has the `json` feature enabled: `reqwest = { version = "...", features = ["json", "...existing features..."] }`
2. For the `result_hash` field, `format!("{:x}", md5::compute(...))` requires `md5 = "0.7"` in dependencies. Alternatively, use `sha2` which may already be in the workspace — check `Cargo.toml` at workspace root. If `sha2` is available, use it instead to avoid adding a new dependency.

### Implementation steps

**Step 1: Create JobRunner**

```rust
//! Job runner: polls for open jobs and executes them.
//!
//! The runner checks the job store (or API) for open jobs matching
//! this agent's capabilities, then executes them using the existing
//! research and plan-run pipelines.

use roko_core::jobs::*;
use std::sync::Arc;

pub struct JobRunner {
    agent_id: String,
    capabilities: Vec<JobType>,
    api_base: String,
    client: reqwest::Client,
}

impl JobRunner {
    pub fn new(agent_id: String, api_base: String) -> Self {
        Self {
            agent_id,
            capabilities: vec![JobType::ResearchBrief, JobType::CodingTask],
            api_base,
            client: reqwest::Client::new(),
        }
    }

    /// Poll for open jobs that match this agent's capabilities.
    /// Returns the first matching job, if any.
    pub async fn poll_for_jobs(&self) -> anyhow::Result<Option<Job>> {
        for cap in &self.capabilities {
            let url = format!(
                "{}/jobs?state=open&job_type={}&limit=1",
                self.api_base, cap
            );
            let resp = self.client.get(&url).send().await?;
            if resp.status().is_success() {
                let jobs: Vec<Job> = resp.json().await?;
                if let Some(job) = jobs.into_iter().next() {
                    // Check tier requirement
                    // MOCK: agent tier hardcoded to 1 -- wire to CascadeRouter current tier
                    let agent_tier: u8 = 1;
                    if job.min_tier <= agent_tier {
                        return Ok(Some(job));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Assign this agent to the job via the API.
    async fn assign_self(&self, job_id: &str) -> anyhow::Result<Job> {
        let url = format!("{}/jobs/{}/assign", self.api_base, job_id);
        let resp = self.client
            .post(&url)
            .json(&serde_json::json!({ "worker": self.agent_id }))
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    /// Mark the job as in-progress.
    async fn start_job(&self, job_id: &str) -> anyhow::Result<Job> {
        let url = format!("{}/jobs/{}/start", self.api_base, job_id);
        let resp = self.client.post(&url).send().await?;
        Ok(resp.json().await?)
    }

    /// Submit results for the job.
    async fn submit_job(
        &self,
        job_id: &str,
        summary: &str,
        deliverable_url: Option<&str>,
        gate_results: Option<Vec<JobGateResult>>,
    ) -> anyhow::Result<Job> {
        let url = format!("{}/jobs/{}/submit", self.api_base, job_id);
        let body = serde_json::json!({
            "result_hash": format!("{:x}", md5::compute(summary.as_bytes())),
            "deliverable_url": deliverable_url,
            "summary": summary,
            "gate_results": gate_results,
        });
        let resp = self.client.post(&url).json(&body).send().await?;
        Ok(resp.json().await?)
    }

    /// Auto-evaluate a completed job.
    async fn auto_evaluate(&self, job_id: &str, accepted: bool, reason: &str) -> anyhow::Result<Job> {
        let url = format!("{}/jobs/{}/evaluate", self.api_base, job_id);
        let body = serde_json::json!({
            "accepted": accepted,
            "reason": reason,
            "evaluator": self.agent_id,
        });
        let resp = self.client.post(&url).json(&body).send().await?;
        Ok(resp.json().await?)
    }

    /// Execute a research job end-to-end.
    pub async fn execute_research_job(&self, job: &Job) -> anyhow::Result<()> {
        tracing::info!("Executing research job: {} - {}", job.id, job.title);

        // 1. Assign
        self.assign_self(&job.id).await?;

        // 2. Start
        self.start_job(&job.id).await?;

        // 3. Run the existing research pipeline
        // Extract topic from job description/title
        let topic = &job.title;

        // Call the existing research command
        // This invokes: roko research topic "<topic>"
        let output = tokio::process::Command::new("cargo")
            .args(["run", "-p", "roko-cli", "--", "research", "topic", topic])
            .current_dir(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // 4. Read the research output
        // The research command writes to .roko/research/<slug>.md
        let slug = topic.to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != ' ', "")
            .replace(' ', "-");
        let research_path = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(format!(".roko/research/{}.md", slug));
        let summary = std::fs::read_to_string(&research_path)
            .unwrap_or_else(|_| format!(
                "Research completed for: {}\n\nOutput:\n{}\n\nStderr:\n{}",
                topic, stdout, stderr
            ));
        // NOTE: `research_path` is now a PathBuf; `read_to_string(&research_path)` works fine.

        // 5. Submit
        self.submit_job(&job.id, &summary, None, None).await?;

        // 6. Auto-evaluate
        let word_count = summary.split_whitespace().count();
        let accepted = word_count > 100; // basic quality check
        let reason = if accepted {
            format!("Research report accepted: {} words", word_count)
        } else {
            format!("Research report too short: {} words (minimum 100)", word_count)
        };
        self.auto_evaluate(&job.id, accepted, &reason).await?;

        tracing::info!("Research job {} completed, accepted={}", job.id, accepted);
        Ok(())
    }
}
```

**Step 2: Wire into orchestrate.rs**

Add a function that can be called from the main orchestration loop:

```rust
/// Check for and execute any available jobs. Called periodically from run_all().
pub async fn maybe_poll_jobs(runner: &JobRunner) {
    match runner.poll_for_jobs().await {
        Ok(Some(job)) => {
            match job.job_type {
                JobType::ResearchBrief => {
                    if let Err(e) = runner.execute_research_job(&job).await {
                        tracing::error!("Research job {} failed: {}", job.id, e);
                    }
                }
                JobType::CodingTask => {
                    // Handled in B6
                    tracing::info!("Coding job {} found, execution not yet wired", job.id);
                }
                _ => {
                    tracing::info!("Unsupported job type: {:?}", job.job_type);
                }
            }
        }
        Ok(None) => {} // no jobs available
        Err(e) => {
            tracing::debug!("Job poll error (expected if serve not running): {}", e);
        }
    }
}
```

In the main `run_all()` loop, add a periodic call (every 30 seconds):

```rust
// Inside the main loop, after existing per-iteration work:
if last_job_poll.elapsed() > Duration::from_secs(30) {
    maybe_poll_jobs(&job_runner).await;
    last_job_poll = Instant::now();
}
```

### Mock tags

```rust
// MOCK: agent tier hardcoded to 1 -- wire to CascadeRouter current tier
// MOCK: md5 hash for result_hash -- replace with proper content-addressable hash
```

### Done when

- [ ] `JobRunner::poll_for_jobs()` correctly queries the API for open jobs
- [ ] `execute_research_job()` runs the full lifecycle: assign → start → research → submit → evaluate
- [ ] Research output is read from `.roko/research/` and submitted as the deliverable
- [ ] Auto-evaluation checks word count (basic quality gate)
- [ ] `maybe_poll_jobs()` integrated into orchestrate.rs main loop
- [ ] `cargo check -p roko-cli` passes
- [ ] `cargo +nightly fmt --all` passes
- [ ] No clippy warnings

---

## Task B6: Coding job execution pipeline

**Effort:** 4 hours
**Stream:** B -- Backend
**Repo:** roko at `/Users/will/dev/nunchi/roko/roko`
**Day:** 2
**Prerequisites:** B1, B2, B5 (shares JobRunner)

### Files to modify

```
crates/roko-cli/src/job_runner.rs — add execute_coding_job method
crates/roko-cli/src/orchestrate.rs — wire CodingTask into maybe_poll_jobs
```

### Implementation steps

**Step 1: Add execute_coding_job to JobRunner**

```rust
impl JobRunner {
    /// Execute a coding job end-to-end.
    pub async fn execute_coding_job(&self, job: &Job) -> anyhow::Result<()> {
        tracing::info!("Executing coding job: {} - {}", job.id, job.title);

        // 1. Assign
        self.assign_self(&job.id).await?;

        // 2. Start
        self.start_job(&job.id).await?;

        // 3. Generate a plan from the job spec
        // Create a temporary PRD from the job description
        let slug = format!("job-{}", job.id);
        let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let prd_dir = workspace.join(".roko/prd");
        std::fs::create_dir_all(&prd_dir)?;

        let prd_content = format!(
            "---\ntitle: {}\nslug: {}\nstatus: published\n---\n\n## Goal\n\n{}\n\n## Acceptance criteria\n\n{}",
            job.title,
            slug,
            job.description,
            job.metadata.get("acceptance_tests")
                .and_then(|v| v.as_str())
                .unwrap_or("Code compiles and passes existing tests.")
        );
        std::fs::write(prd_dir.join(format!("{}.md", slug)), &prd_content)?;

        // 4. Generate plan from PRD
        let plan_output = tokio::process::Command::new("cargo")
            .args(["run", "-p", "roko-cli", "--", "prd", "plan", &slug])
            .current_dir(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
            .output()
            .await?;

        if !plan_output.status.success() {
            let stderr = String::from_utf8_lossy(&plan_output.stderr);
            tracing::warn!("Plan generation produced stderr: {}", stderr);
            // Continue anyway -- plan may still have been generated
        }

        // 5. Run the plan through the gate pipeline
        let plans_dir = workspace.join(".roko/plans").join(&slug);
        let plans_dir_str = plans_dir.to_string_lossy();
        let run_output = tokio::process::Command::new("cargo")
            .args(["run", "-p", "roko-cli", "--", "plan", "run", plans_dir_str.as_ref()])
            .current_dir(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let stderr = String::from_utf8_lossy(&run_output.stderr);

        // 6. Collect gate results
        // Parse gate results from the plan execution output
        let gate_results = self.parse_gate_results(&stdout, &stderr);

        // 7. Build summary
        let all_passed = gate_results.iter().all(|g| g.passed);
        let summary = format!(
            "Coding job completed for: {}\n\nGate results:\n{}\n\nOutput:\n{}",
            job.title,
            gate_results.iter()
                .map(|g| format!("  {} — {}: {}", g.gate, if g.passed { "PASS" } else { "FAIL" }, g.message))
                .collect::<Vec<_>>()
                .join("\n"),
            &stdout[..stdout.len().min(2000)]
        );

        // 8. Submit
        self.submit_job(&job.id, &summary, None, Some(gate_results.clone())).await?;

        // 9. Auto-evaluate based on gate results
        let reason = if all_passed {
            "All gates passed: compile, test, clippy, diff".to_string()
        } else {
            let failed: Vec<_> = gate_results.iter()
                .filter(|g| !g.passed)
                .map(|g| g.gate.as_str())
                .collect();
            format!("Gate failures: {}", failed.join(", "))
        };
        self.auto_evaluate(&job.id, all_passed, &reason).await?;

        tracing::info!("Coding job {} completed, all_gates_passed={}", job.id, all_passed);
        Ok(())
    }

    /// Parse gate results from plan execution output.
    fn parse_gate_results(&self, stdout: &str, stderr: &str) -> Vec<JobGateResult> {
        // Look for gate result patterns in output
        let mut results = Vec::new();

        // Check for compile
        let compile_passed = !stderr.contains("error[E") && !stderr.contains("could not compile");
        results.push(JobGateResult {
            gate: "compile".to_string(),
            passed: compile_passed,
            message: if compile_passed { "0 errors".to_string() } else { "compilation failed".to_string() },
            rung: 1,
        });

        // Check for tests
        let test_passed = stdout.contains("test result: ok") || !stdout.contains("test result: FAILED");
        results.push(JobGateResult {
            gate: "test".to_string(),
            passed: test_passed,
            message: if test_passed { "tests passed".to_string() } else { "test failures detected".to_string() },
            rung: 2,
        });

        // Check for clippy
        let clippy_passed = !stderr.contains("warning:") || stderr.contains("0 warnings");
        results.push(JobGateResult {
            gate: "clippy".to_string(),
            passed: clippy_passed,
            message: if clippy_passed { "0 warnings".to_string() } else { "clippy warnings found".to_string() },
            rung: 3,
        });

        // Diff review is always pass for now
        results.push(JobGateResult {
            gate: "diff".to_string(),
            passed: true,
            message: "diff reviewed".to_string(),
            rung: 4,
        });

        results
    }
}
```

**Step 2: Wire CodingTask into maybe_poll_jobs**

Update the match arm in `maybe_poll_jobs`:

```rust
JobType::CodingTask => {
    if let Err(e) = runner.execute_coding_job(&job).await {
        tracing::error!("Coding job {} failed: {}", job.id, e);
    }
}
```

### Mock tags

```rust
// MOCK: gate result parsing is heuristic (string matching) -- wire to real GatePipeline output structs
// MOCK: diff gate always passes -- wire to real diff review when oracle gates are stable
```

### Done when

- [ ] `execute_coding_job()` runs: assign → start → generate plan → run plan → submit → evaluate
- [ ] Gate results are parsed from build output
- [ ] Auto-evaluation accepts when all gates pass, rejects otherwise
- [ ] CodingTask arm wired in `maybe_poll_jobs()`
- [ ] `cargo check -p roko-cli` passes
- [ ] `cargo +nightly fmt --all` passes
- [ ] No clippy warnings (`cargo clippy --workspace --no-deps -- -D warnings`)

---

## Task B7: Heartbeat emission + aggregation

**Effort:** 2 hours
**Stream:** B -- Backend
**Repo:** roko at `/Users/will/dev/nunchi/roko/roko`
**Day:** 2 (parallel-safe with B5/B6)
**Prerequisites:** B2 (routes exist)

### Files to create

```
crates/roko-serve/src/routes/heartbeats.rs (or add to existing routes)
```

### Files to modify

```
crates/roko-cli/src/orchestrate.rs — emit heartbeat every 30s in main loop
crates/roko-serve/src/routes/mod.rs — add heartbeat routes
crates/roko-serve/src/state.rs — add heartbeat ring buffer
```

### Implementation steps

**Step 1: Define heartbeat types**

Create `crates/roko-core/src/heartbeat.rs` (NOT in `jobs.rs` — keep the two concerns separate).
Add `pub mod heartbeat;` to `crates/roko-core/src/lib.rs`.

File: `crates/roko-core/src/heartbeat.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatPayload {
    pub agent_id: String,
    pub status: String,         // "working", "idle", "error"
    pub current_task: Option<String>,
    pub cognitive_tier: u8,
    pub context_utilization: f64,
    pub token_burn_rate: f64,
    pub episode_count: usize,
    pub gate_pass_rate: f64,
    pub cumulative_cost: f64,
    pub capabilities: Vec<String>,
    pub timestamp: DateTime<Utc>,
}
```

**Step 2: Add heartbeat routes to roko-serve**

POST /api/heartbeats: store in ring buffer (capacity 1000)
GET /api/heartbeats: return recent heartbeats (last 100)
GET /api/network/stats: aggregate from heartbeats

// NOTE: AppState uses tokio::sync::RwLock. Use `.read().await` and `.write().await`.
```rust
// In state.rs:
pub heartbeats: Arc<tokio::sync::RwLock<VecDeque<HeartbeatPayload>>>,

// In heartbeat route handler:
async fn post_heartbeat(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<HeartbeatPayload>,
) -> StatusCode {
    let mut hbs = state.heartbeats.write().await;
    if hbs.len() >= 1000 {
        hbs.pop_front();
    }
    hbs.push_back(payload);
    StatusCode::OK
}

async fn get_heartbeats(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<HeartbeatPayload>> {
    let hbs = state.heartbeats.read().await;
    Json(hbs.iter().rev().take(100).cloned().collect())
}

async fn network_stats(
    State(state): State<Arc<AppState>>,
) -> Json<NetworkStatsResponse> {
    let hbs = state.heartbeats.read().await;
    let now = Utc::now();
    let recent: Vec<_> = hbs.iter()
        .filter(|h| (now - h.timestamp).num_seconds() < 120)
        .collect();
    let online = recent.iter()
        .map(|h| &h.agent_id)
        .collect::<std::collections::HashSet<_>>()
        .len();
    Json(NetworkStatsResponse {
        agents_online: online,
        total_heartbeats: hbs.len(),
    })
}
```

**Step 3: Emit heartbeat from orchestrate.rs**

In the main `run_all()` loop, every 30 seconds. An agent implementing this MUST READ `run_all()` first to identify the available local variables before writing the emission code. The variables available in that scope are:

- `self.config.agent_id()` — for `agent_id`
- `self.executor.active_task_count()` — number of concurrently active agents/tasks (for determining "working" vs "idle" status)
- `self.executor.current_task_id()` — for `current_task`
- `DashboardData` snapshot — for `episode_count`, `gate_pass_rate`, and cost totals

Use these to build the payload. Do NOT invent variable names — check what the struct actually exposes:

```rust
if last_heartbeat.elapsed() > Duration::from_secs(30) {
    // READ run_all() locals — adapt variable names to what actually exists in scope
    let active_count = self.executor.active_task_count();
    let current_task = self.executor.current_task_id();
    let data = self.dashboard_data_snapshot(); // or whatever the snapshot method is
    let payload = HeartbeatPayload {
        agent_id: self.config.agent_id().to_string(),
        status: if active_count > 0 { "working" } else { "idle" }.to_string(),
        current_task,
        cognitive_tier: 1, // MOCK: wire to CascadeRouter current tier
        context_utilization: 0.65, // MOCK: wire to real context window tracking
        token_burn_rate: 0.0,      // MOCK: wire to per-turn token counting
        episode_count: data.episodes.len(),
        gate_pass_rate: data.gate_pass_rate(),
        cumulative_cost: data.total_cost(),
        capabilities: vec!["research_brief".into(), "coding_task".into()],
        timestamp: Utc::now(),
    };
    // Fire and forget -- heartbeat failure should not block execution
    let client = reqwest::Client::new();
    let url = format!("{}/heartbeats", api_base);
    tokio::spawn(async move {
        let _ = client.post(&url).json(&payload).send().await;
    });
    last_heartbeat = Instant::now();
}
```

### Mock tags

```rust
// MOCK: context_utilization hardcoded to 0.65 -- wire to real context window tracking
// MOCK: token_burn_rate uses rough estimate -- wire to per-turn token counting
```

### Done when

- [ ] `pub mod heartbeat;` added to `crates/roko-core/src/lib.rs`
- [ ] POST /api/heartbeats stores payload in ring buffer
- [ ] GET /api/heartbeats returns recent heartbeats
- [ ] GET /api/network/stats returns agent count derived from recent heartbeats
- [ ] orchestrate.rs emits heartbeat every 30s during plan execution
- [ ] Heartbeat emission does not block the main loop (fire-and-forget)
- [ ] `cargo check -p roko-serve -p roko-cli` passes
- [ ] `cargo +nightly fmt --all` passes
- [ ] No clippy warnings (`cargo clippy --workspace --no-deps -- -D warnings`)

---

## Task B8: WebSocket event enrichment

**Effort:** 2 hours
**Stream:** B -- Backend
**Repo:** roko at `/Users/will/dev/nunchi/roko/roko`
**Day:** 3
**Prerequisites:** B2, B7

### Files to modify

```
crates/roko-serve/src/routes/ws.rs — broadcast job + heartbeat events
crates/roko-serve/src/events.rs — ensure all event types serialize
```

### Implementation steps

**Step 1: Ensure event bus receives all job events**

Verify that every route handler in `jobs.rs` sends to the event bus. The event bus is the source for WS broadcast.

**Step 2: Subscribe WS clients to event bus**

In `ws.rs`, when a WebSocket client connects:
1. Subscribe to the event bus channel
2. On each event, serialize to JSON and send to the WebSocket client
3. Handle client disconnection gracefully

```rust
// Pseudocode -- adapt to existing ws.rs structure:
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: Arc<AppState>) {
    // EventBus is always present — subscribe directly (no Option unwrapping)
    let mut rx = state.event_bus.subscribe();

    loop {
        tokio::select! {
            Ok(event) = rx.recv() => {
                let json = serde_json::to_string(&event).unwrap_or_default();
                if socket.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
            Some(msg) = socket.recv() => {
                // Handle incoming messages from client (subscriptions, pings)
                match msg {
                    Ok(Message::Close(_)) => break,
                    Ok(Message::Ping(data)) => {
                        let _ = socket.send(Message::Pong(data)).await;
                    }
                    _ => {}
                }
            }
        }
    }
}
```

**Step 3: Add heartbeat events to the broadcast**

When a heartbeat is received via POST /api/heartbeats, also emit it to the event bus so WS clients receive it.

### Done when

- [ ] WS clients receive job lifecycle events (created, assigned, started, submitted, completed, rejected)
- [ ] WS clients receive heartbeat events
- [ ] WS reconnection works (client disconnects and reconnects)
- [ ] Events are valid JSON matching the `ServerEvent` enum format
- [ ] `cargo check -p roko-serve` passes
- [ ] `cargo +nightly fmt --all` passes
- [ ] No clippy warnings (`cargo clippy --workspace --no-deps -- -D warnings`)

---

## Task B9: Auth middleware upgrade

**Effort:** 2 hours
**Stream:** B -- Backend
**Repo:** roko at `/Users/will/dev/nunchi/roko/roko`
**Day:** 3
**Prerequisites:** none

### Files to modify

```
crates/roko-serve/src/middleware.rs
```

### Implementation steps

**Step 1: Support JWT Bearer tokens alongside API keys**

READ `crates/roko-serve/src/middleware.rs` FIRST. The current middleware checks `X-Api-Key` header against `ServeAuthConfig.api_key: String`. Do NOT rewrite it from scratch — add a second check branch within the same `require_api_key` function.

The function signature is `async fn require_api_key(...)`. Extend it to ALSO accept `Authorization: Bearer <token>` header. Add JWT validation as a second code path:

```rust
// Inside the existing require_api_key function, after the existing X-Api-Key check,
// add a second branch for Authorization: Bearer tokens.
// Adapt the exact structure to match what already exists in middleware.rs.

// Second check: Authorization: Bearer header
let auth_header = req.headers()
    .get("authorization")
    .and_then(|v| v.to_str().ok());

if let Some(value) = auth_header {
    if value.starts_with("Bearer ") {
        let token = &value[7..];
        // Accept if it matches the configured api_key value directly
        if token == auth_config.api_key {
            return Ok(next.run(req).await);
        }
        // Try JWT structural validation
        if validate_jwt(token).is_ok() {
            return Ok(next.run(req).await);
        }
    }
}

fn validate_jwt(token: &str) -> anyhow::Result<()> {
    // For demo: validate JWT structure only (3 dot-separated base64 segments)
    // Full Privy JWKS signature verification is post-demo
    // MOCK: JWT validation checks structure only, not signature -- add JWKS verification for production
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        anyhow::bail!("Invalid JWT format");
    }
    use base64::Engine as _;
    let _header = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[0])
        .map_err(|_| anyhow::anyhow!("Invalid JWT header encoding"))?;
    let _payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|_| anyhow::anyhow!("Invalid JWT payload encoding"))?;
    Ok(())
}
```

Check `crates/roko-serve/Cargo.toml` for `base64` — add `base64 = "0.22"` if not present.

### Mock tags

```rust
// MOCK: JWT validation checks structure only, not signature -- add JWKS verification for production
```

### Done when

- [ ] API key auth still works (existing behavior unchanged)
- [ ] Bearer token with API key value works
- [ ] Structurally valid JWT Bearer tokens are accepted
- [ ] Invalid tokens return 401
- [ ] Missing auth header returns 401 (when auth enabled)
- [ ] `cargo check -p roko-serve` passes
- [ ] `cargo +nightly fmt --all` passes
- [ ] No clippy warnings (`cargo clippy --workspace --no-deps -- -D warnings`)

---

## Task B10: End-to-end integration test

**Effort:** 3 hours
**Stream:** B -- Backend
**Repo:** roko at `/Users/will/dev/nunchi/roko/roko` + `/Users/will/dev/nunchi/nunchi-dashboard`
**Day:** 3
**Prerequisites:** B1-B9, A8

### Implementation steps

This is a manual integration test. No code to write -- just run through the flows and fix what breaks.

**Test 1: Backend alone**

```bash
# Terminal 1: start roko serve
cd /Users/will/dev/nunchi/roko/roko
cargo run -p roko-cli -- serve

# Terminal 2: test job API
curl -X POST http://localhost:6677/api/jobs \
  -H "Content-Type: application/json" \
  -d '{
    "job_type": "research_brief",
    "title": "Impact of EIP-7702 on MEV",
    "description": "Research how account abstraction changes MEV dynamics",
    "bounty_daeji": 100
  }'

# Verify job was created
curl http://localhost:6677/api/jobs
curl http://localhost:6677/api/jobs/stats

# Test state transitions
JOB_ID=$(curl -s http://localhost:6677/api/jobs | jq -r '.[0].id')
curl -X POST "http://localhost:6677/api/jobs/${JOB_ID}/assign" \
  -H "Content-Type: application/json" \
  -d '{"worker": "roko-test-agent"}'
curl -s "http://localhost:6677/api/jobs/${JOB_ID}" | jq .state
# Should be "assigned"
```

**Test 2: Dashboard connects to backend**

```bash
# Terminal 1: roko serve (from Test 1)
# Terminal 3: start dashboard
cd /Users/will/dev/nunchi/nunchi-dashboard
VITE_ROKO_API_URL=http://localhost:6677/api VITE_ROKO_WS_URL=ws://localhost:6677/ws npm run dev
```

Open browser to http://localhost:5173. Verify:
1. Landing page loads
2. Navigate to /app -- agent cards should show (or empty state if no agents registered)
3. Navigate to /app/marketplace -- job board should show the job created in Test 1
4. Create a research bounty from the dashboard
5. Check that the job appears in the job board
6. Check that WebSocket events arrive (open browser console, check ws messages)

**Test 3: Agent picks up job**

```bash
# Terminal 4: run an agent that polls for jobs
cd /Users/will/dev/nunchi/roko/roko
cargo run -p roko-cli -- plan run plans/ --job-poll
```

Or trigger job execution manually by calling the research command:
```bash
cargo run -p roko-cli -- research topic "Impact of EIP-7702 on MEV"
```

Then manually submit the results via API:
```bash
curl -X POST "http://localhost:6677/api/jobs/${JOB_ID}/submit" \
  -H "Content-Type: application/json" \
  -d '{
    "result_hash": "abc123",
    "summary": "## EIP-7702 MEV Analysis\n\nEIP-7702 changes MEV by...",
    "gate_results": null
  }'
```

Verify: dashboard job detail page shows the submitted report.

**Test 4: Coding bounty flow**

Repeat Test 2-3 with a coding bounty:
1. Create coding bounty from dashboard
2. Submit mock results with gate results
3. Verify gate results table renders in dashboard

**Fix everything that breaks.**

### Done when

- [ ] roko serve starts without errors
- [ ] POST/GET /api/jobs works
- [ ] Dashboard connects to backend (job list loads)
- [ ] Creating a bounty from dashboard creates a real job via API
- [ ] Job state transitions work through the full lifecycle
- [ ] WebSocket events flow from backend to dashboard
- [ ] Dashboard renders the job detail with deliverable
- [ ] No server panics during the test
- [ ] No dashboard console errors during the test

---

# Stream C: TUI enhancements

---

## Task C1: F8 Marketplace tab

**Effort:** 4 hours
**Stream:** C -- TUI
**Repo:** roko at `/Users/will/dev/nunchi/roko/roko`
**Day:** 1
**Prerequisites:** none

### Files to create

```
crates/roko-cli/src/tui/views/marketplace_view.rs
```

### Files to modify

```
crates/roko-cli/src/tui/tabs.rs — add F8 binding
crates/roko-cli/src/tui/views/mod.rs — add pub mod marketplace_view
crates/roko-cli/src/tui/app.rs — render marketplace_view on F8
```

### Implementation steps

**Step 1: Define the view**

```rust
//! F8 Marketplace tab: browse and manage jobs.
//!
//! Layout:
//!   ┌───────────── Jobs (35%) ──────────────┬────────── Detail (65%) ──────────┐
//!   │ ▸ job-001 [RESEARCH] Open    100 DJ   │ Title: Analyze EIP-7702...       │
//!   │   job-002 [CODING]   Active  250 DJ   │ Type: research_brief             │
//!   │   job-003 [REVIEW]   Done     50 DJ   │ Status: open                     │
//!   │                                        │ Bounty: 100 DAEJI               │
//!   │                                        │ Posted: 2h ago                   │
//!   │                                        │                                  │
//!   │                                        │ Description:                     │
//!   │                                        │ Research how account abstraction │
//!   │                                        │ via EIP-7702 changes MEV...      │
//!   ├────────────────────────────────────────┴──────────────────────────────────┤
//!   │ Open: 3  Active: 1  Completed: 7  Total bounty: 1,450 DJ                │
//!   └──────────────────────────────────────────────────────────────────────────┘

use ratatui::prelude::*;
use ratatui::widgets::*;

pub struct MarketplaceView {
    job_list: Vec<JobListItem>,
    selected_index: usize,
    scroll_offset: usize,
}

struct JobListItem {
    id: String,
    job_type: String,
    title: String,
    state: String,
    bounty: f64,
    posted_ago: String,
}
```

**Step 2: Data loading**

Load jobs from one of:
1. HTTP GET to roko-serve /api/jobs (if `ws_client` is connected)
2. Direct file read from `.roko/jobs/*.json` (fallback)

Prefer the file-read approach for the TUI since it does not depend on `roko serve` running. Read all JSON files from `.roko/jobs/` directory, parse into `JobListItem`.

**Step 3: Rendering**

Left panel (35% width):
- List widget with job entries. Each line: `job_type` badge (3-char abbreviation: RES/COD/REV/CUS), title (truncated to fit), state badge, bounty amount.
- Selected item highlighted with `Style::default().bg(Color::DarkGray)`.
- Navigation: `j`/`k` or arrow keys to move selection. `Enter` to view detail.

Right panel (65% width):
- All fields of the selected job, formatted in a Block with borders.
- Title, type, state, bounty, poster, deadline, description (wrapped text).
- If submission present: show summary and gate results.
- If evaluation present: show accepted/rejected + reason.

Bottom stats bar:
- `Open: N | Active: N | Completed: N | Total bounty: N DJ`

**Step 4: Keybindings**

```rust
// In input.rs, add F8 handling:
// F8 → switch to MarketplaceView tab
// Within MarketplaceView:
//   j/Down → select next job
//   k/Up → select previous job
//   Enter → toggle detail panel expansion
//   r → refresh job list from disk
//   n → (future) create new job modal
```

**Step 5: Tab registration**

The `Tab` enum in `crates/roko-cli/src/tui/tabs.rs` currently has 7 variants and `ALL: [Tab; 7]`. To add F8 and F9:

1. Add `Marketplace` and `Atelier` variants to the `Tab` enum.
2. Update `ALL` to `[Tab; 9]` with the new variants appended.
3. Add `fkey()` match arms: `Tab::Marketplace => F(8)`, `Tab::Atelier => F(9)`.
4. Add `from_key()` match arms for `F(8)` and `F(9)`.
5. Add `label()` and `label_with_key()` match arms: `Tab::Marketplace => "Marketplace"` / `"F8 Marketplace"`, `Tab::Atelier => "Atelier"` / `"F9 Atelier"`.
6. Update `index()` to return `7` for `Marketplace` and `8` for `Atelier`.
7. Update existing tests: `next_prev_cycle` now cycles through 9 tabs, `index_is_sequential` now checks 9 entries.
8. In `views/mod.rs`: add `pub mod marketplace_view;` and `pub mod atelier_view;`, add match arms in `render_tab_content()`, add `SubView` variants for the new tabs in `SubView::for_tab()`.

Wire in `app.rs` to render `MarketplaceView` when the tab is `Tab::Marketplace`.

### Mock tags

```rust
// MOCK: job list reads from .roko/jobs/ -- falls back to empty list with "No jobs. Create one via the dashboard."
```

### Done when

- [ ] F8 key switches to the Marketplace tab
- [ ] Job list renders from `.roko/jobs/` directory
- [ ] Selecting a job shows its detail in the right panel
- [ ] j/k navigation works
- [ ] Stats bar shows correct counts
- [ ] Empty state shows when no jobs exist
- [ ] `cargo check -p roko-cli` passes
- [ ] `cargo +nightly fmt --all` passes
- [ ] No clippy warnings (`cargo clippy --workspace --no-deps -- -D warnings`)

---

## Task C2: F9 Atelier tab

**Effort:** 4 hours
**Stream:** C -- TUI
**Repo:** roko at `/Users/will/dev/nunchi/roko/roko`
**Day:** 1 (parallel-safe with C1)
**Prerequisites:** none

### Files to create

```
crates/roko-cli/src/tui/views/atelier_view.rs
```

### Files to modify

```
crates/roko-cli/src/tui/tabs.rs — add F9 binding
crates/roko-cli/src/tui/views/mod.rs — add pub mod atelier_view
crates/roko-cli/src/tui/app.rs — render atelier_view on F9
```

### Implementation steps

**Step 1: Define the view**

```
Layout:
  ┌─────────── Workspace Status ──────────────────────────────┐
  │ PRDs: 12  Plans: 5  Tasks: 47  Agents: 3  Episodes: 231 │
  ├─────────── PRDs (40%) ─────────────┬─── Plans (60%) ──────┤
  │ ▸ system-prompt-wiring [planned]   │ Plan: system-prompt   │
  │   gate-recovery [published]        │ ████████░░ 80%        │
  │   tui-rewrite [draft]              │                       │
  │   job-marketplace [idea]           │ Tasks:                │
  │                                     │   [x] Define types    │
  │                                     │   [x] Wire builder    │
  │                                     │   [>] Integration     │
  │                                     │   [ ] Tests           │
  ├─────────────────────────────────────┴──────────────────────┤
  │ Quick: (p)RD list  (r)un plan  (l)ogs  (s)tatus           │
  └────────────────────────────────────────────────────────────┘
```

**Step 2: Data loading**

- PRD list: read from `.roko/prd/` directory. Parse frontmatter for slug, title, status.
- Plan list: read from `.roko/plans/` or `plans/` directories. Parse `tasks.toml` for task list and completion status.
- Workspace stats: count files in each directory.

**Step 3: Rendering**

Top bar: workspace stats in a single line.

Left panel: PRD list with status badges:
- idea → dim text
- draft → amber
- published → green
- planned → blue

Right panel: selected PRD's associated plan (match by slug). If plan exists, show:
- Progress bar (completed tasks / total tasks)
- Task list with status: `[x]` done, `[>]` running, `[ ]` pending, `[!]` failed

Bottom: keybind hints.

**Step 4: Keybindings and tab registration**

See the Tab enum instructions in C1 Step 5 — both `Marketplace` (F8) and `Atelier` (F9) are added together. Follow that same checklist for `Tab::Atelier`.

Wire in `app.rs` to render `AtelierView` when the tab is `Tab::Atelier`.

```
F9 → switch to Atelier tab
j/k → navigate PRD list
Enter → expand/collapse plan detail
p → (future) open PRD in editor
r → run selected plan
```

### Mock tags

```rust
// MOCK: plan task status derived from task.toml status field -- actual running status requires PlanRunner integration
```

### Done when

- [ ] F9 key switches to the Atelier tab
- [ ] PRD list renders from `.roko/prd/` directory
- [ ] Plan detail shows for PRDs that have associated plans
- [ ] Progress bar calculates from task completion
- [ ] Workspace stats count correctly
- [ ] `cargo check -p roko-cli` passes
- [ ] `cargo +nightly fmt --all` passes
- [ ] No clippy warnings (`cargo clippy --workspace --no-deps -- -D warnings`)

---

## Task C3: F7 sub-views (EngramDag, EpisodeReplay, KnowledgeBrowse)

**Effort:** 4 hours
**Stream:** C -- TUI
**Repo:** roko at `/Users/will/dev/nunchi/roko/roko`
**Day:** 2
**Prerequisites:** none

### Files to modify

```
crates/roko-cli/src/tui/views/context_view.rs — add sub-view routing + rendering
crates/roko-cli/src/tui/input.rs — add sub-view navigation keys
```

### Implementation steps

**Step 1: Check existing SubView enum before writing any code**

The `SubView` enum in `crates/roko-cli/src/tui/views/mod.rs` ALREADY declares `EngramDag`, `EpisodeReplay`, `KnowledgeBrowse`, `ProviderHealth`, and `ModelComparison`. The `SubView::for_tab()` dispatch is ALREADY wired.

Do NOT create new sub-view enums. What is missing is the RENDERING CODE inside `context_view.rs` and `config_view.rs`.

In `context_view.rs`, add a match on `view_state.sub_tab` (or whatever the existing field is named — read the file first) to dispatch to rendering functions:
- `SubView::EngramDag` → call `render_engram_dag(f, area, data)`
- `SubView::EpisodeReplay` → call `render_episode_replay(f, area, data)`
- `SubView::KnowledgeBrowse` → call `render_knowledge_browse(f, area, data)`

The local sub-view state type in `context_view.rs` may already exist. Check what `ContextSubView` variants are defined there (if any) before adding to it. If a `ContextSubView` enum already exists with these names, add the rendering — do not re-declare the enum.

**Step 2: EngramDag sub-view**

Reads `.roko/engrams.jsonl` (or equivalent knowledge store file). Renders as an ASCII tree showing dependency relationships.

```
Engrams:
  root
  ├─ "VCG auction design" (confidence: 0.92)
  │  ├─ "Context bidder impl" (0.87)
  │  └─ "Attention pricing" (0.78)
  ├─ "Gate pipeline" (0.95)
  │  ├─ "Compile gate" (0.99)
  │  ├─ "Test gate" (0.95)
  │  └─ "Clippy gate" (0.91)
  └─ "Agent dispatch" (0.88)
```

If the file does not exist, show: "No engrams recorded yet. Run tasks to build knowledge."

**Step 3: EpisodeReplay sub-view**

Reads `.roko/episodes.jsonl`. Provides step-through replay of agent episodes.

```
Episode #231 — agent: roko-01, task: wire-prompt-builder
  Turn 1/12                                         [j/k to step]
  ┌──────────────────────────────────────────────────────────┐
  │ Role: system                                              │
  │ Tokens: 2,847                                             │
  │ Tier: T1 (Deliberate)                                     │
  │                                                           │
  │ Content (first 500 chars):                                │
  │ You are a Rust developer working on the roko toolkit...   │
  └──────────────────────────────────────────────────────────┘
  Gate result: compile PASS, test PASS, clippy PASS
```

Navigation:
- `j`/`k` — step through turns within an episode
- `n`/`p` — jump to next/previous episode
- `Enter` — expand turn content (show full text, scrollable)

**Step 4: KnowledgeBrowse sub-view**

Reads `.roko/memory/` directory. Lists knowledge entries with confidence bars.

```
Knowledge store: 47 entries
  ┌──────────────────────────────────────────────────────────┐
  │ ▸ VCG auction token allocation    ████████░░  0.92       │
  │   Gate pipeline configuration     █████████░  0.95       │
  │   Agent dispatch patterns         ████████░░  0.88       │
  │   HDC fingerprint encoding        ███████░░░  0.78       │
  │   Research citation format        ██████░░░░  0.65       │
  └──────────────────────────────────────────────────────────┘
```

Search: `/` to enter search mode, type to filter entries.

**Step 5: Sub-view navigation in input.rs**

Within the F7 (Context) tab:
- `1` → Overview (existing)
- `2` → EngramDag
- `3` → EpisodeReplay
- `4` → KnowledgeBrowse

Show keybind hints in the status bar when F7 is active.

### Mock tags

```rust
// MOCK: engram dag builds tree from .roko/engrams.jsonl -- shows empty state if file missing
// MOCK: episode replay reads .roko/episodes.jsonl -- shows empty state if no episodes recorded
// MOCK: knowledge browse reads .roko/memory/ -- shows empty state if directory empty
```

### Done when

- [ ] Pressing `2` on F7 tab shows EngramDag view
- [ ] Pressing `3` shows EpisodeReplay with step-through
- [ ] Pressing `4` shows KnowledgeBrowse with confidence bars
- [ ] `j`/`k` navigation works in EpisodeReplay
- [ ] Search works in KnowledgeBrowse
- [ ] All sub-views handle missing files gracefully
- [ ] `cargo check -p roko-cli` passes
- [ ] `cargo +nightly fmt --all` passes
- [ ] No clippy warnings (`cargo clippy --workspace --no-deps -- -D warnings`)

---

## Task C4: F6 sub-views (ProviderHealth, ModelComparison)

**Effort:** 3 hours
**Stream:** C -- TUI
**Repo:** roko at `/Users/will/dev/nunchi/roko/roko`
**Day:** 2
**Prerequisites:** none (parallel-safe with C3)

### Files to modify

```
crates/roko-cli/src/tui/views/config_view.rs — add sub-view routing + rendering
crates/roko-cli/src/tui/input.rs — add sub-view keys for F6
```

### Implementation steps

**Step 1: Check existing SubView enum before writing any code**

READ `crates/roko-cli/src/tui/views/mod.rs` first. The `SubView` enum ALREADY declares `ProviderHealth` and `ModelComparison`. The `SubView::for_tab()` dispatch is ALREADY wired.

Do NOT create a new `ConfigSubView` enum. What is missing is the RENDERING CODE inside `config_view.rs`.

In `config_view.rs`, add a match on the existing sub-view state to dispatch to:
- `SubView::ProviderHealth` → call `render_provider_health(f, area, data)`
- `SubView::ModelComparison` → call `render_model_comparison(f, area, data)`

Read `config_view.rs` to find the existing sub-view dispatch pattern (if any) before adding these arms.

**Step 2: ProviderHealth**

Read from `.roko/learn/cascade-router.json` for model routing data and provider stats.

```
Provider health:
  ┌────────────────────────────────────────────────────────┐
  │ Provider    │ Status  │ Latency │ Error % │ Models     │
  │─────────────┼─────────┼─────────┼─────────┼────────────│
  │ anthropic   │ OK      │  142ms  │  0.2%   │ 4          │
  │ openai      │ OK      │  189ms  │  0.5%   │ 3          │
  │ ollama      │ WARN    │  890ms  │  2.1%   │ 2          │
  └────────────────────────────────────────────────────────┘
```

Data source: parse cascade-router.json for model assignments, estimate latency from recent episode data.

**Step 3: ModelComparison**

Side-by-side comparison table.

```
Model comparison:
  ┌──────────────────────────────────────────────────────────┐
  │ Model              │ Cost/1K │ Tier │ Gate % │ Uses      │
  │────────────────────┼─────────┼──────┼────────┼───────────│
  │ claude-sonnet-4    │ $0.003  │ T1   │  94%   │ 847       │
  │ claude-haiku-3.5   │ $0.001  │ T0   │  88%   │ 2,341     │
  │ claude-opus-4      │ $0.015  │ T2   │  97%   │ 42        │
  │ gpt-4.1            │ $0.002  │ T1   │  91%   │ 156       │
  └──────────────────────────────────────────────────────────┘
```

**Step 4: Navigation**

Within F6: `1` Overview, `2` ProviderHealth, `3` ModelComparison.

### Mock tags

```rust
// MOCK: provider latency estimated from cascade-router.json timestamps -- wire to real latency tracking
// MOCK: model cost/1K uses hardcoded pricing table -- wire to provider API pricing
// MOCK: gate pass rate per model from efficiency.jsonl -- approximation
```

### Done when

- [ ] `2` on F6 shows ProviderHealth table
- [ ] `3` on F6 shows ModelComparison table
- [ ] Tables render with correct alignment
- [ ] Missing data shows "N/A" rather than crashing
- [ ] `cargo check -p roko-cli` passes
- [ ] `cargo +nightly fmt --all` passes
- [ ] No clippy warnings (`cargo clippy --workspace --no-deps -- -D warnings`)

---

## Task C5: Bug fixes

**Effort:** 3 hours
**Stream:** C -- TUI
**Repo:** roko at `/Users/will/dev/nunchi/roko/roko`
**Day:** 2-3
**Prerequisites:** none (parallel-safe)

### Files to modify

```
crates/roko-cli/src/tui/widgets/plan_tree.rs — fix vfy column
crates/roko-cli/src/tui/dashboard.rs — add cached log vec
crates/roko-cli/src/tui/widgets/wave_progress.rs — fix collapse toggle
crates/roko-cli/src/tui/input.rs — wire wave collapse key
crates/roko-cli/src/tui/views/git_view.rs — fix git log parser
crates/roko-cli/src/tui/widgets/status_bar.rs — differentiate keybinds
```

### Implementation steps

**Step 1: plan_tree vfy column**

In `plan_tree.rs`, the "vfy" (verify) column shows a stub instead of actual gate status. Fix:
- Read gate results from `data.gate_results_page` (the DashboardData field)
- Match gate results to tasks by task ID
- Show: green checkmark if all gates passed, red X if any failed, dash if not yet run

**Step 2: Logs O(N) rebuild**

In `dashboard.rs`, the unified log view rebuilds a `Vec<LogEntry>` from scratch every frame. Fix:
- Add a field `cached_unified_log: Vec<LogEntry>` to the dashboard state
- Rebuild only when new data arrives (check a generation counter or last-modified timestamp)
- On frame render, use the cached vec directly

**Step 3: Wave collapse toggle**

In `wave_progress.rs`, the `expanded` field exists but pressing `h`/`l` or `Enter` does not toggle it. Fix:
- In `input.rs`, when the wave progress widget is focused, handle:
  - `h` or `l` or `Enter` → toggle `wave.expanded`
  - Update the wave widget state

**Step 4: Git parser**

In `git_view.rs`, the git log is parsed from `git log --graph` output, which produces inconsistent formatting. Fix:
- Change the git command to: `git log --format=%H%x09%s%x09%an%x09%cr -20`
- Parse output as tab-separated fields: hash, subject, author, relative date
- Render in a clean table format

**Step 5: Status bar keybind hints**

In `status_bar.rs`, the keybind hints do not differentiate between states (e.g., when there are failures vs. when everything is clean). Fix:
- When `has_failures` is true, show: `[Enter] Retry failed | [d] Show diff | [q] Quit`
- When `has_failures` is false, show: `[j/k] Navigate | [Enter] Expand | [q] Quit`

### Done when

- [ ] plan_tree vfy column shows real gate status from data
- [ ] Log view does not rebuild every frame (check with profiling or frame timing)
- [ ] Wave progress expands/collapses on keypress
- [ ] Git view shows clean table output (no graph artifacts)
- [ ] Status bar shows context-appropriate keybinds
- [ ] `cargo check -p roko-cli` passes
- [ ] `cargo +nightly fmt --all` passes
- [ ] No clippy warnings (`cargo clippy --workspace --no-deps -- -D warnings`)

---

## Task C6: Header bar network stats

**Effort:** 2 hours
**Stream:** C -- TUI
**Repo:** roko at `/Users/will/dev/nunchi/roko/roko`
**Day:** 3
**Prerequisites:** B7 (heartbeat data available)

### Files to modify

```
crates/roko-cli/src/tui/widgets/header_bar.rs
crates/roko-cli/src/tui/dashboard.rs — add network stats to DashboardData
```

### Implementation steps

**Step 1: Add network stats to DashboardData**

In `dashboard.rs`, add fields:
```rust
pub agents_online: usize,
pub isfr_summary: Option<String>,
```

Populate from:
1. HTTP GET to `http://localhost:6677/api/network/stats` (if serve is running)
2. Count `.roko/jobs/*.json` with state != terminal as fallback for "active" count

Use the WS client if already connected, or fall back to direct file reads.

**Step 2: Render in header_bar.rs**

Add to the right side of the header bar:

```
┌─── ROKO ──────────────────── 3 agents online │ ISFR: -- ────┐
```

- `N agents online`: from `data.agents_online`, colored green if > 0, muted if 0
- `ISFR: --`: from `data.isfr_summary`, or `--` if unavailable

### Mock tags

```rust
// MOCK: agents_online from /api/network/stats -- falls back to counting .roko/jobs active files
// MOCK: ISFR always "--" until mirage-rs integration
```

### Done when

- [ ] Header bar shows agent count on the right
- [ ] Count updates on refresh
- [ ] ISFR shows "--" (placeholder)
- [ ] Header does not break layout when numbers are large
- [ ] `cargo check -p roko-cli` passes
- [ ] `cargo +nightly fmt --all` passes
- [ ] No clippy warnings (`cargo clippy --workspace --no-deps -- -D warnings`)

---

# Dependency graph

```
Stream A (Dashboard):
  A1 ──────────┐
  A2 (parallel) ├──→ A3 ──→ A4 ──→ A5 ──→ A6 ──→ A7 ──→ A8 ──→ A9
               │     (A3-A7 can parallelize among themselves)
               │
Stream B (Backend):
  B1 ──→ B2 ──→ B5 ──→ B6
  B3 (parallel)  ↑     ↑
  B4 (parallel)  │     │
                 B7 ──→ B8 ──→ B10
                 B9 (parallel)  ↑
                                │
Stream C (TUI):                 │
  C1 (parallel)                 │
  C2 (parallel)                 │
  C3 ──→ C5                    │
  C4 (parallel)                │
  C6 (needs B7)────────────────┘
```

Cross-stream dependencies:
- A8 (dashboard integration) benefits from B2 (job routes) and B8 (WS events) being done
- A9 (demo rehearsal) needs B10 (backend integration) to be done
- C6 (header stats) needs B7 (heartbeats) to be done

Everything else within each stream can proceed independently.

---

# Effort summary

| Task | Stream | Day | Hours | Description |
|---|---|---|---|---|
| A1 | Dashboard | 1 | 4 | Project restructure + routing + design system |
| A2 | Dashboard | 1 | 3 | API layer rewrite + WebSocket client |
| A3 | Dashboard | 1 | 4 | Landing page (5 sections) |
| A4 | Dashboard | 1-2 | 5 | Observatory pages (5 pages) |
| A5 | Dashboard | 2 | 3 | Network + Knowledge pages (3 pages) |
| A6 | Dashboard | 2 | 5 | Marketplace pages (3 pages + components) |
| A7 | Dashboard | 2 | 5 | Agent Studio + Command + Atelier + Settings (10 pages) |
| A8 | Dashboard | 3 | 4 | Integration wiring + visual polish |
| A9 | Dashboard | 3 | 2 | Demo flow rehearsal |
| B1 | Backend | 1 | 3 | Job types and store |
| B2 | Backend | 1 | 3 | Job API routes |
| B3 | Backend | 1 | 2 | Incremental file watchers |
| B4 | Backend | 1 | 1 | Server state persistence |
| B5 | Backend | 2 | 4 | Research job execution |
| B6 | Backend | 2 | 4 | Coding job execution |
| B7 | Backend | 2 | 2 | Heartbeat emission + aggregation |
| B8 | Backend | 3 | 2 | WebSocket event enrichment |
| B9 | Backend | 3 | 2 | Auth middleware upgrade |
| B10 | Backend | 3 | 3 | End-to-end integration test |
| C1 | TUI | 1 | 4 | F8 Marketplace tab |
| C2 | TUI | 1 | 4 | F9 Atelier tab |
| C3 | TUI | 2 | 4 | F7 sub-views (3 views) |
| C4 | TUI | 2 | 3 | F6 sub-views (2 views) |
| C5 | TUI | 2-3 | 3 | Bug fixes (5 items) |
| C6 | TUI | 3 | 2 | Header bar network stats |

**Totals:**
- Stream A: 35 hours (3 parallel agents over 3 days = ~12 agent-hours/day)
- Stream B: 26 hours (2-3 parallel agents over 3 days = ~9 agent-hours/day)
- Stream C: 20 hours (1-2 parallel agents over 3 days = ~7 agent-hours/day)
- **Grand total: 81 agent-hours across 3 days**

With 3 streams running in parallel and multiple tasks parallelizable within each stream, this fits in 3 calendar days with Claude agents executing 6-8 tasks concurrently.

---

## API endpoints not yet created (will show empty/error states in demo)

The dashboard references these endpoints. Those marked "NOT created" are not implemented by any task in this plan — the dashboard should degrade gracefully (empty state or mock data) when they return 404 or are unreachable.

| Endpoint | Status | Notes |
|---|---|---|
| `GET /api/agents/:id/episodes` | **Exists** in roko-serve | Wire to `useQuery` in AgentOverview |
| `GET /api/agents/:id/trace` | **Exists** via aggregator | Wire to AgentOverview cognitive trace section |
| `GET /api/agents/:id/stats` | **Exists** via aggregator | Wire to AgentOverview stats row |
| `GET /api/keys` | **NOT created** | Keys page shows hardcoded 1-entry mock list |
| `POST /api/keys` | **NOT created** | Create key returns mock key (`key-demo-xxxx`) |
| `GET /api/deployments/bind` | **NOT created** | Binding code is generated client-side |
| `GET /api/diagnosis/watchers` | **NOT created** | Conductor page shows 10 hardcoded watchers, all "active" |
| `GET /api/pheromones/heatmap` | **Exists** in mirage-rs | Wire to PheromoneField canvas data source |
| `GET /api/knowledge/entries` | **Exists** via aggregator | Wire to KnowledgeGraph search |
| `GET /api/learning/cost-tiers` | **NOT created** | Learning page cost tier cards use hardcoded `{ t0: $0.00, t1: $0.002, t2: $0.10 }` |
| `GET /api/providers/circuit-breakers` | **NOT created** | Conductor circuit breaker section shows hardcoded `["anthropic: CLOSED", "openai: CLOSED"]` |
| `GET /api/agents/topology` | **NOT created** | Network topology edge weights default to 0.5 |

Pages that will show empty or mock state at demo time due to missing endpoints: AgentKeys, AgentDeploy, Conductor (partial), Learning cost-tiers. All other pages have real backing data from completed tasks.

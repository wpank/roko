# A7: Remaining pages — studio, command, atelier, settings

## Context

**Repo:** `/Users/will/dev/nunchi/nunchi-dashboard`
**Branch:** `demo-rewrite`
**Tech stack:** React 19 + Vite 8 + TypeScript + Tailwind CSS v4
**Backend:** `roko-serve` runs at `http://localhost:6677` with ~85 REST routes + WebSocket at `ws://localhost:6677/ws`
**Auth:** Privy (env var `VITE_PRIVY_APP_ID`) with password fallback
**Design:** ROSEDUST dark palette — bg_void `#060608`, rose `#AA7088`, bone `#C8B890`, rose_bright `#CC90A8`

### Before starting

1. `cd /Users/will/dev/nunchi/nunchi-dashboard`
2. `git checkout -b demo-rewrite 2>/dev/null || git checkout demo-rewrite`
3. `npm install`
4. Verify: `npm run dev` starts without errors

### After every task

1. `npm run typecheck` passes
2. `npm run dev` — page renders without console errors
3. All existing tests pass: `npm test` (if test runner is configured)

---

## What this task produces

Ten pages that fill every remaining placeholder in the router: four Agent Studio pages, two Command pages, three Atelier pages, and Settings. After this task, every route in the app renders a real page — no more `<Placeholder>` components.

**Depends on:** Task A1 (design system, router, stores), Task A2 (API hooks).

The `Agent` type imported from A2 has the shape:

```ts
type Agent = {
  id: number;       // PID
  label: string;    // human-readable name
  role?: string;
  model?: string;
};
```

Use this type for any agent-related data. Do not redeclare it locally — import from `../../services/api`.

---

## Checklist

### 1. Create directories

```bash
mkdir -p /Users/will/dev/nunchi/nunchi-dashboard/src/pages/studio
mkdir -p /Users/will/dev/nunchi/nunchi-dashboard/src/pages/command
mkdir -p /Users/will/dev/nunchi/nunchi-dashboard/src/pages/atelier
mkdir -p /Users/will/dev/nunchi/nunchi-dashboard/src/pages/settings
```

### 2. Agent Studio: Overview

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/studio/AgentOverview.tsx`:

```tsx
import type { Agent } from "../../services/api";
import { useAgents } from "../../services/api";
import { Badge, Card, EmptyState, Gauge, Skeleton, StatusDot } from "../../design-system/components";

// Static profile data not available from the live endpoint yet.
// Wire to GET /api/agents/:id when richer agent profiles are added.
type AgentProfile = {
  id: string;
  role: string;
  model: string;
  successRate: number;
  tasksCompleted: number;
  uptimeHours: number;
};

const AGENT_PROFILES: AgentProfile[] = [
  { id: "conductor-01", role: "conductor", model: "claude-opus-4", successRate: 0.94, tasksCompleted: 142, uptimeHours: 48 },
  { id: "researcher-02", role: "researcher", model: "claude-sonnet-4", successRate: 0.87, tasksCompleted: 89, uptimeHours: 36 },
  { id: "validator-03", role: "validator", model: "claude-haiku-3", successRate: 0.96, tasksCompleted: 203, uptimeHours: 72 },
];

function isAgentLive(profile: AgentProfile, liveAgents: Agent[]): boolean {
  return liveAgents.some((a) => a.label === profile.id || String(a.id) === profile.id);
}

export default function AgentOverview() {
  const { data: liveAgents = [], isLoading, error } = useAgents();

  if (isLoading) {
    return (
      <div className="p-6 space-y-3">
        {Array.from({ length: 3 }, (_, i) => (
          <Skeleton key={i} height="120px" />
        ))}
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-6">
        <EmptyState
          title="Could not load agents"
          description="Check that roko-serve is running on port 6677."
        />
      </div>
    );
  }

  return (
    <div className="p-6">
      <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)] mb-1">
        Agent overview
      </h1>
      <p className="text-xs text-[var(--rd-fg-muted)] mb-6">
        Your deployed agents and their performance.
      </p>

      {AGENT_PROFILES.length === 0 ? (
        <EmptyState
          title="No agents configured"
          description="Deploy an agent from the Deploy tab to get started."
        />
      ) : (
        <div className="space-y-3">
          {AGENT_PROFILES.map((profile) => {
            const live = isAgentLive(profile, liveAgents);
            return (
              <Card key={profile.id}>
                <div className="flex items-start justify-between mb-3">
                  <div className="flex items-center gap-3">
                    <StatusDot status={live ? "online" : "offline"} />
                    <div>
                      <div className="text-sm font-medium text-[var(--rd-fg-primary)]">
                        {profile.id}
                      </div>
                      <div className="flex items-center gap-2 mt-0.5">
                        <Badge label={profile.role} variant="rose" />
                        <span className="text-[10px] font-mono text-[var(--rd-fg-muted)]">
                          {profile.model}
                        </span>
                      </div>
                    </div>
                  </div>
                  <Badge
                    label={live ? "live" : "offline"}
                    variant={live ? "success" : "default"}
                  />
                </div>

                <div className="grid grid-cols-3 gap-4">
                  <div>
                    <div className="text-[10px] text-[var(--rd-fg-muted)] mb-1">
                      Success rate
                    </div>
                    <Gauge value={profile.successRate} size="sm" />
                  </div>
                  <div>
                    <div className="text-[10px] text-[var(--rd-fg-muted)] mb-1">
                      Tasks completed
                    </div>
                    <div className="text-lg font-mono text-[var(--rd-fg-primary)]">
                      {profile.tasksCompleted}
                    </div>
                  </div>
                  <div>
                    <div className="text-[10px] text-[var(--rd-fg-muted)] mb-1">
                      Uptime
                    </div>
                    <div className="text-lg font-mono text-[var(--rd-fg-primary)]">
                      {profile.uptimeHours}h
                    </div>
                  </div>
                </div>
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}
```

### 3. Agent Studio: Strategy

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/studio/AgentStrategy.tsx`:

```tsx
import { useState } from "react";
import { Button, Card, Gauge, Select, useToast } from "../../design-system/components";

type RiskTolerance = "conservative" | "moderate" | "aggressive";
type ModelPreference = "auto" | "quality" | "cost" | "balanced";
type Concurrency = "1" | "2" | "4" | "8";

export default function AgentStrategy() {
  const { toast } = useToast();
  const [riskTolerance, setRiskTolerance] = useState<RiskTolerance>("moderate");
  const [modelPreference, setModelPreference] = useState<ModelPreference>("auto");
  const [concurrency, setConcurrency] = useState<Concurrency>("4");

  function handleSave() {
    // Wire to PATCH /api/config when backend config mutations are supported.
    toast("Strategy saved", "success");
  }

  return (
    <div className="p-6 max-w-2xl">
      <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)] mb-1">
        Agent strategy
      </h1>
      <p className="text-xs text-[var(--rd-fg-muted)] mb-6">
        Configure how your agents make decisions and allocate resources.
      </p>

      <Card className="mb-4">
        <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-3">
          Routing preferences
        </div>
        <div className="space-y-4">
          <Select
            label="Model selection"
            value={modelPreference}
            onChange={(v) => setModelPreference(v as ModelPreference)}
            options={[
              { value: "auto", label: "Auto (cascade router decides)" },
              { value: "quality", label: "Quality first (prefer opus)" },
              { value: "cost", label: "Cost first (prefer haiku)" },
              { value: "balanced", label: "Balanced (prefer sonnet)" },
            ]}
          />
          <Select
            label="Risk tolerance"
            value={riskTolerance}
            onChange={(v) => setRiskTolerance(v as RiskTolerance)}
            options={[
              { value: "conservative", label: "Conservative — extra gate validation" },
              { value: "moderate", label: "Moderate — standard pipeline" },
              { value: "aggressive", label: "Aggressive — fewer gates, faster" },
            ]}
          />
          <Select
            label="Max concurrency"
            value={concurrency}
            onChange={(v) => setConcurrency(v as Concurrency)}
            options={[
              { value: "1", label: "1 (sequential)" },
              { value: "2", label: "2" },
              { value: "4", label: "4 (default)" },
              { value: "8", label: "8" },
            ]}
          />
        </div>
      </Card>

      <Card className="mb-4">
        <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-3">
          Current allocation
        </div>
        {/* Wire to actual agent config from /api/config once available. */}
        <div className="space-y-3">
          <Gauge value={0.65} label="Token budget utilization" />
          <Gauge value={0.82} label="Gate pass rate (30d)" />
          <Gauge value={0.45} label="Cost efficiency vs. solo baseline" />
        </div>
      </Card>

      <div className="flex justify-end">
        <Button onClick={handleSave}>Save strategy</Button>
      </div>
    </div>
  );
}
```

### 4. Agent Studio: Keys

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/studio/AgentKeys.tsx`:

```tsx
import { useState } from "react";
import { Badge, Button, Card, Input, useToast } from "../../design-system/components";

type ApiKey = {
  id: string;
  name: string;
  prefix: string;
  created: string;
  lastUsed: string | null;
};

// Wire to agent token management endpoints when they are available.
const INITIAL_KEYS: ApiKey[] = [
  { id: "key-1", name: "Production", prefix: "rk_prod_****", created: "2026-04-10", lastUsed: "2026-04-21" },
  { id: "key-2", name: "Development", prefix: "rk_dev_****", created: "2026-04-15", lastUsed: null },
];

export default function AgentKeys() {
  const { toast } = useToast();
  const [keys, setKeys] = useState<ApiKey[]>(INITIAL_KEYS);
  const [newKeyName, setNewKeyName] = useState("");

  function handleRevoke(id: string) {
    setKeys((prev) => prev.filter((k) => k.id !== id));
    toast("Key revoked", "warning");
  }

  function handleGenerate() {
    const name = newKeyName.trim();
    if (!name) {
      toast("Name is required", "error");
      return;
    }
    const newKey: ApiKey = {
      id: `key-${Date.now()}`,
      name,
      prefix: "rk_new_****",
      created: new Date().toISOString().slice(0, 10),
      lastUsed: null,
    };
    setKeys((prev) => [...prev, newKey]);
    toast("Key generated", "success");
    setNewKeyName("");
  }

  return (
    <div className="p-6 max-w-2xl">
      <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)] mb-1">
        API keys
      </h1>
      <p className="text-xs text-[var(--rd-fg-muted)] mb-6">
        Manage authentication tokens for your agents.
      </p>

      <Card className="mb-4">
        <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-3">
          Active keys
        </div>
        {keys.length === 0 ? (
          <p className="text-xs text-[var(--rd-fg-muted)] py-4 text-center">
            No keys. Generate one below.
          </p>
        ) : (
          <div className="space-y-2">
            {keys.map((key) => (
              <div
                key={key.id}
                className="flex items-center justify-between px-3 py-2.5 rounded-md bg-[var(--rd-bg-surface-0)]"
              >
                <div>
                  <div className="text-xs font-medium text-[var(--rd-fg-primary)]">
                    {key.name}
                  </div>
                  <div className="text-[10px] font-mono text-[var(--rd-fg-muted)]">
                    {key.prefix}
                  </div>
                </div>
                <div className="flex items-center gap-3">
                  <span className="text-[10px] text-[var(--rd-fg-muted)]">
                    {key.lastUsed ? `Used ${key.lastUsed}` : "Never used"}
                  </span>
                  <Button
                    size="sm"
                    variant="danger"
                    onClick={() => handleRevoke(key.id)}
                  >
                    Revoke
                  </Button>
                </div>
              </div>
            ))}
          </div>
        )}
      </Card>

      <Card>
        <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-3">
          Generate new key
        </div>
        <div className="flex items-end gap-3">
          <div className="flex-1">
            <Input
              label="Key name"
              placeholder="e.g., Staging"
              value={newKeyName}
              onChange={setNewKeyName}
            />
          </div>
          <Button onClick={handleGenerate}>Generate</Button>
        </div>
      </Card>
    </div>
  );
}
```

### 5. Agent Studio: Deploy

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/studio/AgentDeploy.tsx`:

```tsx
import { useState } from "react";
import { Badge, Button, Card, StatusDot, useToast } from "../../design-system/components";
import { useHealth } from "../../services/api";

type Step = {
  n: number;
  label: string;
  command: string;
};

const SETUP_STEPS: Step[] = [
  { n: 1, label: "Install the CLI", command: "cargo install --path crates/roko-cli" },
  { n: 2, label: "Initialize a workspace", command: "roko init" },
  { n: 3, label: "Start the control plane", command: "roko serve" },
  { n: 4, label: "Run a plan", command: "roko plan run plans/" },
];

export default function AgentDeploy() {
  const { toast } = useToast();
  const { data: health, refetch, isFetching } = useHealth();
  const [copied, setCopied] = useState<number | null>(null);

  function handleCopy(step: Step) {
    navigator.clipboard.writeText(step.command).catch(() => null);
    setCopied(step.n);
    setTimeout(() => setCopied(null), 1500);
  }

  function handleTestConnection() {
    refetch().then(() => toast("Connection checked", "success"));
  }

  return (
    <div className="p-6 max-w-2xl">
      <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)] mb-1">
        Deploy
      </h1>
      <p className="text-xs text-[var(--rd-fg-muted)] mb-6">
        Set up and connect agents to the network.
      </p>

      <Card className="mb-4">
        <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-3">
          Quick start
        </div>
        <div className="space-y-2">
          {SETUP_STEPS.map((step) => (
            <div
              key={step.n}
              className="group flex items-center gap-3 p-3 rounded-md bg-[var(--rd-bg-surface-0)]"
            >
              <span className="shrink-0 w-5 h-5 rounded-full bg-[var(--rd-bg-surface-2)] flex items-center justify-center text-[10px] font-medium text-[var(--rd-fg-muted)]">
                {step.n}
              </span>
              <div className="flex-1 min-w-0">
                <div className="text-xs font-medium text-[var(--rd-fg-primary)] mb-0.5">
                  {step.label}
                </div>
                <code className="text-[10px] font-mono text-[var(--rd-rose)] block bg-[var(--rd-bg-surface-2)] rounded px-2 py-1 truncate">
                  {step.command}
                </code>
              </div>
              <button
                className="shrink-0 text-[10px] text-[var(--rd-fg-muted)] opacity-0 group-hover:opacity-100 transition-opacity px-2 py-1 rounded hover:bg-[var(--rd-bg-surface-2)]"
                onClick={() => handleCopy(step)}
                aria-label={`Copy: ${step.command}`}
              >
                {copied === step.n ? "Copied" : "Copy"}
              </button>
            </div>
          ))}
        </div>
      </Card>

      <Card>
        <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-3">
          Connection status
        </div>
        <div className="flex items-center justify-between px-3 py-2.5 rounded-md bg-[var(--rd-bg-surface-0)] mb-3">
          <div className="flex items-center gap-2">
            <StatusDot status={health ? "online" : "offline"} />
            <span className="text-xs text-[var(--rd-fg-secondary)]">roko-serve</span>
          </div>
          <Badge
            label={health ? health.status : "unreachable"}
            variant={health ? "success" : "default"}
          />
        </div>
        <Button
          variant="secondary"
          size="sm"
          loading={isFetching}
          onClick={handleTestConnection}
        >
          Test connection
        </Button>
      </Card>
    </div>
  );
}
```

### 6. Command: Chat

The chat page auto-scrolls on new messages. Shift+Enter inserts a newline; Enter sends.

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/command/Chat.tsx`:

```tsx
import { useEffect, useRef, useState } from "react";
import { Button } from "../../design-system/components";
import { useStartRun } from "../../services/api";
import { useWsStore } from "../../stores/wsStore";

type Role = "user" | "assistant";

type Message = {
  id: number;
  role: Role;
  content: string;
  timestamp: number;
};

function formatTime(ts: number): string {
  return new Date(ts).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

export default function Chat() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);
  const idRef = useRef(0);
  const startRun = useStartRun();
  const { events } = useWsStore();

  // Auto-scroll to bottom whenever messages change.
  useEffect(() => {
    const el = scrollRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  }, [messages.length]);

  // Append agent_output WebSocket events as assistant messages.
  useEffect(() => {
    const latest = events.at(-1);
    if (!latest) return;
    const payload = latest.payload as Record<string, unknown> | null;
    if (payload?.type === "agent_output" && typeof payload.content === "string") {
      setMessages((prev) => [
        ...prev,
        { id: ++idRef.current, role: "assistant", content: payload.content as string, timestamp: Date.now() },
      ]);
    }
  // Only trigger when a new event arrives, not on every re-render.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [events.length]);

  function send() {
    const content = input.trim();
    if (!content) return;

    setMessages((prev) => [
      ...prev,
      { id: ++idRef.current, role: "user", content, timestamp: Date.now() },
    ]);
    setInput("");
    startRun.mutate(content);
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  return (
    <div className="flex flex-col h-[calc(100vh-3.5rem)]">
      {/* Message list */}
      <div
        ref={scrollRef}
        className="flex-1 overflow-y-auto p-6 space-y-4"
      >
        {messages.length === 0 && (
          <div className="flex flex-col items-center justify-center h-full text-center select-none">
            <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-[var(--rd-rose)] to-[var(--rd-rose-dim)] flex items-center justify-center text-white text-lg font-bold mb-4">
              N
            </div>
            <h2 className="text-sm font-medium text-[var(--rd-fg-primary)] mb-1">
              What should we work on?
            </h2>
            <p className="text-xs text-[var(--rd-fg-muted)] max-w-xs">
              Send a prompt to trigger a run. Agent output streams back via WebSocket.
            </p>
          </div>
        )}

        {messages.map((msg) => (
          <div
            key={msg.id}
            className={`flex ${msg.role === "user" ? "justify-end" : "justify-start"}`}
          >
            <div
              className={[
                "max-w-lg px-4 py-3 rounded-xl text-sm",
                msg.role === "user"
                  ? "bg-[var(--rd-rose)] text-white"
                  : "bg-[var(--rd-bg-surface-1)] text-[var(--rd-fg-secondary)] border border-[var(--rd-bg-surface-3)]",
              ].join(" ")}
            >
              <div className="whitespace-pre-wrap leading-relaxed">{msg.content}</div>
              <div
                className={[
                  "text-[9px] mt-1.5",
                  msg.role === "user" ? "text-white/60" : "text-[var(--rd-fg-muted)]",
                ].join(" ")}
              >
                {formatTime(msg.timestamp)}
              </div>
            </div>
          </div>
        ))}

        {startRun.isPending && (
          <div className="flex justify-start">
            <div className="px-4 py-3 rounded-xl bg-[var(--rd-bg-surface-1)] border border-[var(--rd-bg-surface-3)]">
              <div className="flex items-center gap-1.5">
                {[0, 150, 300].map((delay) => (
                  <span
                    key={delay}
                    className="w-1.5 h-1.5 rounded-full bg-[var(--rd-fg-muted)] animate-bounce"
                    style={{ animationDelay: `${delay}ms` }}
                  />
                ))}
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Input area */}
      <div className="border-t border-[var(--rd-bg-surface-2)] p-4">
        <div className="flex items-end gap-3 max-w-3xl mx-auto">
          <textarea
            rows={1}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Send a prompt to roko… (Shift+Enter for newline)"
            className="flex-1 resize-none bg-[var(--rd-bg-surface-1)] border border-[var(--rd-bg-surface-3)] text-[var(--rd-fg-primary)] placeholder-[var(--rd-fg-muted)] rounded-lg px-4 py-2.5 text-sm focus:outline-none focus:ring-1 focus:ring-[var(--rd-rose)] max-h-40 overflow-y-auto"
            style={{ fieldSizing: "content" } as React.CSSProperties}
          />
          <Button
            onClick={send}
            disabled={!input.trim() || startRun.isPending}
          >
            Send
          </Button>
        </div>
        <p className="text-center text-[9px] text-[var(--rd-fg-muted)] mt-2">
          Enter to send · Shift+Enter for newline
        </p>
      </div>
    </div>
  );
}
```

### 7. Command: Research

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/command/Research.tsx`:

```tsx
import { useState } from "react";
import { Badge, Button, Card, EmptyState, Input, Select, useToast } from "../../design-system/components";
import { useResearchTopic } from "../../services/api";

type ResearchIntent = "explore" | "position" | "evaluate" | "monitor" | "audit";
type ResearchStatus = "running" | "completed" | "failed";

type HistoryEntry = {
  id: string;
  topic: string;
  intent: ResearchIntent;
  status: ResearchStatus;
  created: string;
};

const INTENT_OPTIONS: { value: ResearchIntent; label: string }[] = [
  { value: "explore", label: "Explore" },
  { value: "position", label: "Position" },
  { value: "evaluate", label: "Evaluate" },
  { value: "monitor", label: "Monitor" },
  { value: "audit", label: "Audit" },
];

const STATUS_VARIANT: Record<ResearchStatus, "success" | "warning" | "danger"> = {
  completed: "success",
  running: "warning",
  failed: "danger",
};

// Wire to GET /api/research/history when the endpoint is available.
const MOCK_HISTORY: HistoryEntry[] = [
  {
    id: "r-1",
    topic: "Funding rate arbitrage across perpetual futures venues",
    intent: "position",
    status: "completed",
    created: "2026-04-20",
  },
  {
    id: "r-2",
    topic: "HDC vector similarity for episode clustering",
    intent: "explore",
    status: "completed",
    created: "2026-04-19",
  },
];

export default function Research() {
  const { toast } = useToast();
  const [topic, setTopic] = useState("");
  const [intent, setIntent] = useState<ResearchIntent>("explore");
  const [history] = useState<HistoryEntry[]>(MOCK_HISTORY);
  const researchTopic = useResearchTopic();

  function handleSubmit() {
    const trimmed = topic.trim();
    if (!trimmed) {
      toast("Topic is required", "error");
      return;
    }
    researchTopic.mutate(
      { topic: trimmed, intent },
      {
        onSuccess: () => { toast("Research started", "success"); setTopic(""); },
        onError: (err) => toast(`Failed: ${String(err)}`, "error"),
      }
    );
  }

  return (
    <div className="p-6">
      <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)] mb-1">
        Research
      </h1>
      <p className="text-xs text-[var(--rd-fg-muted)] mb-6">
        Launch deep research with citations. Results are saved to{" "}
        <code className="text-[var(--rd-rose)]">.roko/research/</code>.
      </p>

      <Card className="mb-6">
        <div className="space-y-4">
          <Input
            label="Topic"
            placeholder="What should we research?"
            value={topic}
            onChange={setTopic}
            type="textarea"
          />
          <div className="flex items-end gap-3">
            <div className="w-48">
              <Select
                label="Intent"
                value={intent}
                onChange={(v) => setIntent(v as ResearchIntent)}
                options={INTENT_OPTIONS}
              />
            </div>
            <Button onClick={handleSubmit} loading={researchTopic.isPending}>
              Start research
            </Button>
          </div>
        </div>
      </Card>

      <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-2">
        History
      </div>

      {history.length === 0 ? (
        <EmptyState
          title="No research yet"
          description="Start a research topic above."
        />
      ) : (
        <div className="space-y-2">
          {history.map((entry) => (
            <Card key={entry.id} padding="sm">
              <div className="flex items-start justify-between gap-4">
                <div className="min-w-0">
                  <div className="text-xs font-medium text-[var(--rd-fg-primary)] leading-snug">
                    {entry.topic}
                  </div>
                  <div className="flex items-center gap-2 mt-1">
                    <Badge label={entry.intent} variant="info" />
                    <span className="text-[10px] text-[var(--rd-fg-muted)]">
                      {entry.created}
                    </span>
                  </div>
                </div>
                <Badge label={entry.status} variant={STATUS_VARIANT[entry.status]} />
              </div>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
```

### 8. Atelier: Dashboard

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/atelier/Atelier.tsx`:

```tsx
import { usePlans, usePrds, useVelocity } from "../../services/api";
import { Badge, Card, Gauge, Skeleton } from "../../design-system/components";

type WorkflowStep = { n: number; command: string; label: string };

const WORKFLOW: WorkflowStep[] = [
  { n: 1, command: "roko prd idea", label: "capture work items" },
  { n: 2, command: "roko prd draft", label: "agent drafts a PRD" },
  { n: 3, command: "roko prd plan", label: "generate implementation tasks" },
  { n: 4, command: "roko plan run", label: "execute with gates and learning" },
];

export default function Atelier() {
  const { data: plans, isLoading: plansLoading } = usePlans();
  const { data: prds } = usePrds();
  const { data: velocity } = useVelocity();

  const totalPlans = plans?.length ?? 0;
  const completedPlans = plans?.filter((p) => p.completed).length ?? 0;
  const totalPrds = prds?.length ?? 0;
  const velocityValue = typeof (velocity as { value?: number } | undefined)?.value === "number"
    ? ((velocity as { value: number }).value).toFixed(1)
    : null;

  if (plansLoading) {
    return (
      <div className="p-6 grid grid-cols-3 gap-4">
        {Array.from({ length: 3 }, (_, i) => (
          <Skeleton key={i} height="120px" />
        ))}
      </div>
    );
  }

  return (
    <div className="p-6">
      <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)] mb-1">
        Atelier
      </h1>
      <p className="text-xs text-[var(--rd-fg-muted)] mb-6">
        Self-development workspace. PRDs, plans, execution, and velocity.
      </p>

      {/* Summary cards */}
      <div className="grid grid-cols-3 gap-4 mb-6">
        <Card>
          <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-1">
            PRDs
          </div>
          <div className="text-2xl font-mono text-[var(--rd-fg-primary)]">{totalPrds}</div>
        </Card>

        <Card>
          <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-1">
            Plans
          </div>
          <div className="text-2xl font-mono text-[var(--rd-fg-primary)] mb-1">{totalPlans}</div>
          {totalPlans > 0 && (
            <Gauge
              value={totalPlans > 0 ? completedPlans / totalPlans : 0}
              label={`${completedPlans} / ${totalPlans} completed`}
              size="sm"
            />
          )}
        </Card>

        <Card>
          <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-1">
            Velocity
          </div>
          <div className="text-2xl font-mono text-[var(--rd-fg-primary)]">
            {velocityValue ?? "—"}
          </div>
          <div className="text-[10px] text-[var(--rd-fg-muted)]">tasks / day (7d avg)</div>
        </Card>
      </div>

      {/* Self-hosting workflow reference */}
      <Card>
        <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-3">
          Self-hosting workflow
        </div>
        <div className="space-y-2">
          {WORKFLOW.map((step) => (
            <div key={step.n} className="flex items-center gap-2 text-xs">
              <Badge label={String(step.n)} variant="rose" />
              <code className="text-[var(--rd-rose)]">{step.command}</code>
              <span className="text-[var(--rd-fg-muted)]">— {step.label}</span>
            </div>
          ))}
        </div>
      </Card>
    </div>
  );
}
```

### 9. Atelier: PRD browser

The PRD list has status filter tabs (all / ideas / drafts / published).

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/atelier/PrdBrowser.tsx`:

```tsx
import { useState } from "react";
import { Badge, Button, Card, EmptyState, ErrorState, Input, Skeleton, useToast } from "../../design-system/components";
import { useGeneratePlan, usePostIdea, usePromotePrd, usePrds } from "../../services/api";

type PrdStatus = "idea" | "draft" | "published";
type FilterTab = "all" | PrdStatus;

const TABS: { value: FilterTab; label: string }[] = [
  { value: "all", label: "All" },
  { value: "idea", label: "Ideas" },
  { value: "draft", label: "Drafts" },
  { value: "published", label: "Published" },
];

const STATUS_VARIANT: Record<PrdStatus, "default" | "warning" | "success"> = {
  idea: "default",
  draft: "warning",
  published: "success",
};

export default function PrdBrowser() {
  const { data: prds, isLoading, error, refetch } = usePrds();
  const postIdea = usePostIdea();
  const promotePrd = usePromotePrd();
  const generatePlan = useGeneratePlan();
  const { toast } = useToast();
  const [ideaText, setIdeaText] = useState("");
  const [activeTab, setActiveTab] = useState<FilterTab>("all");

  if (isLoading) {
    return (
      <div className="p-6 space-y-3">
        {Array.from({ length: 4 }, (_, i) => (
          <Skeleton key={i} height="64px" />
        ))}
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-6">
        <ErrorState error={String(error)} onRetry={() => refetch()} />
      </div>
    );
  }

  function handlePostIdea() {
    const text = ideaText.trim();
    if (!text) return;
    postIdea.mutate(text, {
      onSuccess: () => { toast("Idea captured", "success"); setIdeaText(""); },
      onError: (err) => toast(`Failed: ${String(err)}`, "error"),
    });
  }

  const allPrds = prds ?? [];
  const filtered = activeTab === "all"
    ? allPrds
    : allPrds.filter((p) => p.status === activeTab);

  return (
    <div className="p-6">
      <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)] mb-1">
        PRD browser
      </h1>
      <p className="text-xs text-[var(--rd-fg-muted)] mb-6">
        Product requirement documents powering the self-hosting loop.
      </p>

      {/* Quick idea capture */}
      <Card className="mb-6">
        <div className="flex items-end gap-3">
          <div className="flex-1">
            <Input
              label="Quick idea"
              placeholder="Capture a work item…"
              value={ideaText}
              onChange={setIdeaText}
            />
          </div>
          <Button onClick={handlePostIdea} loading={postIdea.isPending} size="sm">
            Capture
          </Button>
        </div>
      </Card>

      {/* Filter tabs */}
      <div className="flex items-center gap-1 mb-3">
        {TABS.map((tab) => {
          const count = tab.value === "all"
            ? allPrds.length
            : allPrds.filter((p) => p.status === tab.value).length;
          const active = activeTab === tab.value;
          return (
            <button
              key={tab.value}
              onClick={() => setActiveTab(tab.value)}
              className={[
                "flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs transition-colors",
                active
                  ? "bg-[var(--rd-bg-surface-2)] text-[var(--rd-fg-primary)]"
                  : "text-[var(--rd-fg-muted)] hover:text-[var(--rd-fg-secondary)]",
              ].join(" ")}
            >
              {tab.label}
              <span className={[
                "text-[10px] font-mono",
                active ? "text-[var(--rd-fg-muted)]" : "text-[var(--rd-fg-muted)]/60",
              ].join(" ")}>
                {count}
              </span>
            </button>
          );
        })}
      </div>

      {/* PRD list */}
      {filtered.length === 0 ? (
        <EmptyState
          title={activeTab === "all" ? "No PRDs found" : `No ${activeTab} PRDs`}
          description={activeTab === "all"
            ? "Capture ideas or create PRDs via the CLI."
            : `Switch to a different tab or create new ${activeTab}s.`}
        />
      ) : (
        <div className="space-y-2">
          {filtered.map((prd) => (
            <Card key={prd.slug} padding="sm">
              <div className="flex items-center justify-between gap-4">
                <div className="min-w-0">
                  <div className="text-sm font-medium text-[var(--rd-fg-primary)] truncate">
                    {prd.title ?? prd.slug}
                  </div>
                  <div className="flex items-center gap-2 mt-0.5">
                    <Badge
                      label={prd.status}
                      variant={STATUS_VARIANT[prd.status as PrdStatus] ?? "default"}
                    />
                    <span className="text-[10px] font-mono text-[var(--rd-fg-muted)]">
                      {prd.slug}
                    </span>
                    {prd.has_plan && <Badge label="has plan" variant="info" />}
                  </div>
                </div>
                <div className="flex items-center gap-2 shrink-0">
                  {prd.status === "draft" && (
                    <Button
                      size="sm"
                      variant="secondary"
                      loading={promotePrd.isPending}
                      onClick={() =>
                        promotePrd.mutate(prd.slug, {
                          onSuccess: () => toast("PRD promoted", "success"),
                          onError: (err) => toast(`Failed: ${String(err)}`, "error"),
                        })
                      }
                    >
                      Promote
                    </Button>
                  )}
                  {prd.status === "published" && !prd.has_plan && (
                    <Button
                      size="sm"
                      variant="secondary"
                      loading={generatePlan.isPending}
                      onClick={() =>
                        generatePlan.mutate(prd.slug, {
                          onSuccess: () => toast("Plan generation started", "success"),
                          onError: (err) => toast(`Failed: ${String(err)}`, "error"),
                        })
                      }
                    >
                      Generate plan
                    </Button>
                  )}
                </div>
              </div>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
```

### 10. Atelier: Execution monitor

Events are shown newest-first. Each row uses a colored badge matching the event severity.

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/atelier/ExecutionMonitor.tsx`:

```tsx
import { Badge, Card, Skeleton } from "../../design-system/components";
import { useSessionStatus } from "../../services/api";
import { useWsStore } from "../../stores/wsStore";

const EXEC_EVENTS = new Set([
  "run_started",
  "run_completed",
  "plan_started",
  "plan_completed",
  "gate_result",
  "agent_output",
]);

type BadgeVariant = "info" | "success" | "warning" | "danger";

function eventVariant(type: string): BadgeVariant {
  if (type.endsWith("_completed") || type === "gate_result") return "success";
  if (type.endsWith("_started")) return "info";
  if (type === "agent_output") return "warning";
  return "info";
}

function formatTime(ms: number): string {
  return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

export default function ExecutionMonitor() {
  const { events } = useWsStore();
  const { data: status, isLoading } = useSessionStatus();

  const execEvents = events
    .filter((e) => EXEC_EVENTS.has(e.type))
    .slice()
    .reverse();

  const s = status as Record<string, unknown> | null | undefined;

  if (isLoading) {
    return (
      <div className="p-6 space-y-3">
        <Skeleton height="80px" />
        <Skeleton height="300px" />
      </div>
    );
  }

  return (
    <div className="p-6">
      <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)] mb-1">
        Execution monitor
      </h1>
      <p className="text-xs text-[var(--rd-fg-muted)] mb-6">
        Live execution events from the WebSocket stream.
      </p>

      {/* Session status */}
      <Card className="mb-4">
        <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-2">
          Session
        </div>
        {s ? (
          <div className="grid grid-cols-4 gap-4 text-xs">
            {[
              { label: "Session ID", value: String(s.session_id ?? "—"), mono: true },
              { label: "Daemon", value: s.daemon_running ? "running" : "stopped", badge: true },
              { label: "Signals", value: String(s.signal_count ?? 0), mono: true },
              { label: "Episodes", value: String(s.episode_count ?? 0), mono: true },
            ].map(({ label, value, mono, badge }) => (
              <div key={label}>
                <div className="text-[10px] text-[var(--rd-fg-muted)] mb-0.5">{label}</div>
                {badge ? (
                  <Badge label={value} variant={value === "running" ? "success" : "default"} />
                ) : (
                  <div className={mono ? "font-mono text-[var(--rd-fg-secondary)]" : "text-[var(--rd-fg-secondary)]"}>
                    {value}
                  </div>
                )}
              </div>
            ))}
          </div>
        ) : (
          <div className="text-xs text-[var(--rd-fg-muted)]">No session data</div>
        )}
      </Card>

      {/* Event stream */}
      <Card>
        <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-2">
          Event stream
          <span className="ml-1 text-[var(--rd-fg-muted)]/60">({execEvents.length})</span>
        </div>
        {execEvents.length === 0 ? (
          <div className="text-xs text-[var(--rd-fg-muted)] py-8 text-center">
            No execution events yet. Start a run or plan execution to see events here.
          </div>
        ) : (
          <div className="space-y-1 max-h-[400px] overflow-y-auto pr-1">
            {execEvents.map((event, i) => (
              <div
                key={i}
                className="flex items-center gap-3 px-3 py-1.5 rounded-md bg-[var(--rd-bg-surface-0)] text-xs"
              >
                <Badge label={event.type} variant={eventVariant(event.type)} />
                <span className="text-[var(--rd-fg-secondary)] font-mono truncate flex-1 text-[10px]">
                  {JSON.stringify(event.payload).slice(0, 120)}
                </span>
                <span className="text-[10px] text-[var(--rd-fg-muted)] shrink-0">
                  {formatTime(event.receivedAt)}
                </span>
              </div>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}
```

### 11. Settings page

The roko.toml config view is read-only. There is no edit functionality because the config is managed by the CLI and file system.

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/settings/Settings.tsx`:

```tsx
import { useAuthStore } from "../../stores/authStore";
import { useConfig } from "../../services/api";
import { Button, Card, ErrorState, Skeleton } from "../../design-system/components";

export default function Settings() {
  const { data: config, isLoading, error, refetch } = useConfig();
  const { token, logout } = useAuthStore();

  const apiUrl = import.meta.env.VITE_ROKO_API_URL ?? "http://localhost:6677";

  if (isLoading) {
    return (
      <div className="p-6 max-w-2xl">
        <Skeleton height="200px" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-6 max-w-2xl">
        <ErrorState error={String(error)} onRetry={() => refetch()} />
      </div>
    );
  }

  return (
    <div className="p-6 max-w-2xl">
      <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)] mb-1">
        Settings
      </h1>
      <p className="text-xs text-[var(--rd-fg-muted)] mb-6">
        Dashboard and backend configuration.
      </p>

      {/* Backend connection (read-only info) */}
      <Card className="mb-4">
        <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-3">
          Backend connection
        </div>
        <div className="flex items-center justify-between px-3 py-2 rounded-md bg-[var(--rd-bg-surface-0)]">
          <span className="text-[10px] text-[var(--rd-fg-muted)]">API URL</span>
          <code className="text-xs font-mono text-[var(--rd-fg-secondary)]">{apiUrl}</code>
        </div>
        <p className="mt-2 text-[10px] text-[var(--rd-fg-muted)]">
          Set via{" "}
          <code className="text-[var(--rd-rose)]">VITE_ROKO_API_URL</code> in{" "}
          <code className="text-[var(--rd-rose)]">.env.local</code>. Requires a dev-server restart.
        </p>
      </Card>

      {/* roko.toml — read-only display */}
      <Card className="mb-4">
        <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-3">
          roko.toml (from backend)
        </div>
        {config ? (
          <pre className="text-[10px] font-mono text-[var(--rd-fg-secondary)] bg-[var(--rd-bg-surface-0)] rounded-md p-3 max-h-64 overflow-auto leading-relaxed">
            {JSON.stringify(config, null, 2)}
          </pre>
        ) : (
          <div className="text-xs text-[var(--rd-fg-muted)] py-4 text-center">
            Not connected — start roko-serve to see the config.
          </div>
        )}
        <p className="mt-2 text-[10px] text-[var(--rd-fg-muted)]">
          This is a read-only view. Edit{" "}
          <code className="text-[var(--rd-rose)]">roko.toml</code> on disk and restart roko-serve to apply changes.
        </p>
      </Card>

      {/* Authentication */}
      <Card>
        <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-3">
          Authentication
        </div>
        <div className="flex items-center justify-between">
          <span className="text-xs text-[var(--rd-fg-secondary)]">
            {token ? "Authenticated" : "Not authenticated"}
          </span>
          {token && (
            <Button size="sm" variant="danger" onClick={logout}>
              Log out
            </Button>
          )}
        </div>
      </Card>
    </div>
  );
}
```

### 12. Wire all pages into the router

- [ ] Update `/Users/will/dev/nunchi/nunchi-dashboard/src/router.tsx` — replace all remaining `<Placeholder>` entries:

```tsx
// Command
{ path: "chat",     element: lazyPage(() => import("./pages/command/Chat"))     },
{ path: "research", element: lazyPage(() => import("./pages/command/Research")) },

// Agent Studio
{ path: "studio/overview", element: lazyPage(() => import("./pages/studio/AgentOverview")) },
{ path: "studio/strategy", element: lazyPage(() => import("./pages/studio/AgentStrategy")) },
{ path: "studio/keys",     element: lazyPage(() => import("./pages/studio/AgentKeys"))     },
{ path: "studio/deploy",   element: lazyPage(() => import("./pages/studio/AgentDeploy"))   },

// Atelier
{ path: "atelier",           element: lazyPage(() => import("./pages/atelier/Atelier"))          },
{ path: "atelier/prds",      element: lazyPage(() => import("./pages/atelier/PrdBrowser"))       },
{ path: "atelier/execution", element: lazyPage(() => import("./pages/atelier/ExecutionMonitor")) },

// Settings
{ path: "settings", element: lazyPage(() => import("./pages/settings/Settings")) },
```

After wiring these routes, the `Placeholder` helper function in `router.tsx` is unused. Remove it and its import.

---

## Verification

Run from `/Users/will/dev/nunchi/nunchi-dashboard`:

- [ ] `npm run typecheck` — exits 0
- [ ] `npm run dev` — navigate to every route. None should show "Placeholder":

| Route | Expected content |
|---|---|
| `/app/chat` | Chat interface with textarea input and empty state |
| `/app/research` | Research form with intent selector and mock history |
| `/app/studio/overview` | Agent cards with live/offline badge and gauges |
| `/app/studio/strategy` | Form with selects and gauge display |
| `/app/studio/keys` | Key list with revoke buttons and generate form |
| `/app/studio/deploy` | Step-by-step guide with copy buttons and connection status |
| `/app/atelier` | Summary cards with PRD/plan counts |
| `/app/atelier/prds` | Filter tabs + PRD list with promote/generate buttons |
| `/app/atelier/execution` | Event stream from WS (or empty message) |
| `/app/settings` | Read-only config viewer and auth controls |

- [ ] Chat: typing a message and pressing Enter calls `POST /api/run` (check Network tab)
- [ ] Chat: Shift+Enter inserts a newline instead of sending
- [ ] PRD browser: "Capture" button calls `POST /api/prds/ideas`
- [ ] PRD browser: filter tabs narrow the list correctly
- [ ] Settings: config section shows "read-only" note, not an editable field
- [ ] No console errors

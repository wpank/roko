# A3: Landing page -- hero, architecture explorer, context auction, stigmergy canvas

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

A full landing page at `/` with five sections that scroll vertically. The page demonstrates the system's architecture and live state to someone who has never seen it before. Each section is a self-contained component in `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/landing/`.

**Depends on:** Task A1 (design tokens, router, LandingLayout) and Task A2 (useHealth hook).

---

## Checklist

### 1. Create landing component directory

```bash
mkdir -p /Users/will/dev/nunchi/nunchi-dashboard/src/pages/landing
```

### 2. StatsStrip component

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/landing/StatsStrip.tsx`:

```tsx
import { useEffect, useRef, useState, memo } from "react";
import { useMediaQuery, BREAKPOINTS } from "../../design-system/useMediaQuery";

type StatItem = { label: string; value: number; suffix?: string };

// ── CountUp ───────────────────────────────────────────────────────────────────
// Uses requestAnimationFrame with ease-out-cubic easing.
// Respects prefers-reduced-motion: snaps to the target value immediately.

const CountUp = memo(function CountUp({
  target,
  duration = 1500,
}: {
  target: number;
  duration?: number;
}) {
  const [current, setCurrent] = useState(0);
  const prefersReduced = useMediaQuery(BREAKPOINTS.reducedMotion);

  useEffect(() => {
    if (prefersReduced) {
      setCurrent(target);
      return;
    }

    const start = performance.now();
    let frameId: number;

    const step = (now: number) => {
      const elapsed = now - start;
      const progress = Math.min(elapsed / duration, 1);
      // ease-out cubic: decelerates into the final value
      const eased = 1 - Math.pow(1 - progress, 3);
      setCurrent(Math.floor(eased * target));
      if (progress < 1) {
        frameId = requestAnimationFrame(step);
      } else {
        setCurrent(target);
      }
    };

    frameId = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frameId);
  }, [target, duration, prefersReduced]);

  return <>{current.toLocaleString()}</>;
});

// ── StatsStrip ────────────────────────────────────────────────────────────────

type StatsStripProps = {
  stats: StatItem[];
  onLaunch: () => void;
};

export function StatsStrip({ stats, onLaunch }: StatsStripProps) {
  return (
    <div className="flex flex-col items-center gap-8">
      <div className="flex flex-wrap items-center justify-center gap-8 sm:gap-12">
        {stats.map((stat) => (
          <div key={stat.label} className="text-center">
            <div className="text-2xl font-mono text-[var(--rd-fg-primary)]">
              <CountUp target={stat.value} />
              {stat.suffix && (
                <span className="text-sm text-[var(--rd-fg-muted)] ml-1">
                  {stat.suffix}
                </span>
              )}
            </div>
            <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mt-1">
              {stat.label}
            </div>
          </div>
        ))}
      </div>
      <button
        onClick={onLaunch}
        className={[
          "px-8 py-3 rounded-xl text-sm font-medium",
          "bg-[var(--rd-rose)] text-white",
          "hover:bg-[var(--rd-rose-bright)] active:bg-[var(--rd-rose-dim)]",
          "transition-colors duration-[var(--rd-transition-fast)]",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--rd-rose)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--rd-bg-void)]",
          "will-change-transform",
        ].join(" ")}
      >
        Launch dashboard
      </button>
    </div>
  );
}
```

### 3. HeroSection component

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/landing/HeroSection.tsx`:

```tsx
import { useHealth } from "../../services/api";
import { StatsStrip } from "./StatsStrip";

type HeroSectionProps = {
  onLaunch: () => void;
};

export function HeroSection({ onLaunch }: HeroSectionProps) {
  const { data: health } = useHealth();
  const isConnected = Boolean(health);

  // MOCK: wire agent count + plans to GET /api/health and /api/metrics/summary.
  // health.active_agents and health.active_plans are available when connected.
  const stats = [
    { label: "Agents online",  value: health?.active_agents ?? 0 },
    { label: "Plans executed", value: 1_284 },
    { label: "Gate pass rate", value: 94, suffix: "%" },
    { label: "C-Factor",       value: 18, suffix: "%" },
  ];

  return (
    <section className="min-h-screen flex flex-col items-center justify-center px-6 relative overflow-hidden">
      {/* Animated radial gradient -- breathes slowly */}
      <div
        aria-hidden="true"
        className="absolute inset-0 pointer-events-none animate-gradient-breathe"
        style={{
          background:
            "radial-gradient(ellipse 80% 60% at 50% 40%, rgba(170,112,136,0.12) 0%, transparent 70%)",
        }}
      />

      {/* Subtle grid overlay */}
      <div
        aria-hidden="true"
        className="absolute inset-0 pointer-events-none opacity-[0.03]"
        style={{
          backgroundImage:
            "linear-gradient(var(--rd-fg-muted) 1px, transparent 1px), linear-gradient(90deg, var(--rd-fg-muted) 1px, transparent 1px)",
          backgroundSize: "60px 60px",
        }}
      />

      <div className="relative z-10 text-center max-w-2xl">
        {/* Logo */}
        <div
          aria-hidden="true"
          className="w-20 h-20 rounded-2xl bg-gradient-to-br from-[var(--rd-rose)] to-[var(--rd-rose-dim)] flex items-center justify-center text-white text-3xl font-bold mb-8 mx-auto shadow-lg shadow-[var(--rd-rose)]/20 will-change-transform"
        >
          N
        </div>

        <h1 className="text-5xl font-bold text-[var(--rd-fg-primary)] mb-4 tracking-tight">
          Nunchi
        </h1>
        <p className="text-xl text-[var(--rd-fg-secondary)] mb-2">
          Hyperdimensional intelligence for autonomous agent orchestration
        </p>
        <p className="text-sm text-[var(--rd-fg-muted)] mb-12 max-w-md mx-auto">
          Agents that build themselves. Plan, execute, verify, learn, iterate.
          The cognitive loop runs continuously.
        </p>

        <StatsStrip stats={stats} onLaunch={onLaunch} />

        {/* Connection status */}
        <div className="mt-8 flex items-center justify-center gap-2" aria-live="polite">
          <span
            className={[
              "w-1.5 h-1.5 rounded-full transition-colors duration-[var(--rd-transition-normal)]",
              isConnected ? "bg-[var(--rd-success)]" : "bg-[var(--rd-fg-muted)]",
            ].join(" ")}
            aria-hidden="true"
          />
          <span className="text-[10px] text-[var(--rd-fg-muted)] font-mono">
            {isConnected ? "roko-serve connected" : "roko-serve offline"}
          </span>
        </div>
      </div>
    </section>
  );
}
```

### 4. ArchitectureExplorer component

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/landing/ArchitectureExplorer.tsx`:

```tsx
import { useState, useEffect, useCallback, memo } from "react";
import { cn } from "../../design-system/cn";

type Tier = {
  id: string;
  label: string;
  sublabel: string;
  color: string;
  components: string[];
  description: string;
};

const TIERS: Tier[] = [
  {
    id: "t0",
    label: "T0 — Reflexive",
    sublabel: "Hot path, <100ms",
    color: "var(--rd-success)",
    components: [
      "Gate pipeline (compile, test, clippy, diff)",
      "Adaptive thresholds",
      "Process supervisor",
      "Signal substrate (JSONL)",
    ],
    description:
      "The reflexive layer validates every agent output before it touches the codebase. Seven gate rungs execute in parallel, each with EMA-adjusted pass thresholds.",
  },
  {
    id: "t1",
    label: "T1 — Deliberative",
    sublabel: "Task-scoped, seconds",
    color: "var(--rd-warning)",
    components: [
      "DAG executor (parallel + merge queue)",
      "Cascade router (model selection)",
      "System prompt builder (9-layer)",
      "VCG context auction",
      "Episode logger",
    ],
    description:
      "The deliberative layer plans multi-step work. The DAG executor resolves dependencies, dispatches agents to the cheapest capable model via cascade routing, and records outcomes for the learning loop.",
  },
  {
    id: "t2",
    label: "T2 — Reflective",
    sublabel: "Cross-session, minutes",
    color: "var(--rd-rose-bright)",
    components: [
      "Prompt experiments (A/B)",
      "Efficiency events",
      "Playbook store",
      "Gate failure replan",
      "PRD auto-plan trigger",
    ],
    description:
      "The reflective layer learns from history. Prompt experiments identify winning templates. Efficiency events feed the cascade router. Gate failures trigger automatic replanning.",
  },
] as const;

const TIER_IDS = TIERS.map((t) => t.id);

// ── TierButton ────────────────────────────────────────────────────────────────

const TierButton = memo(function TierButton({
  tier,
  isActive,
  onClick,
}: {
  tier: Tier;
  isActive: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      aria-pressed={isActive}
      className={cn(
        "px-5 py-3 rounded-lg border transition-all duration-[var(--rd-transition-fast)] text-left",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--rd-rose)]",
        isActive
          ? "border-current bg-current/10"
          : "border-[var(--rd-bg-surface-3)] bg-[var(--rd-bg-surface-1)] hover:border-[var(--rd-fg-muted)]"
      )}
      style={isActive ? { color: tier.color } : undefined}
    >
      <div className="text-sm font-medium text-[var(--rd-fg-primary)]">
        {tier.label}
      </div>
      <div className="text-[10px] text-[var(--rd-fg-muted)]">{tier.sublabel}</div>
    </button>
  );
});

// ── ArchitectureExplorer ──────────────────────────────────────────────────────

export function ArchitectureExplorer() {
  const [activeId, setActiveId] = useState<string>("t1");
  const active = TIERS.find((t) => t.id === activeId) ?? TIERS[1];

  // Keyboard navigation: ArrowLeft / ArrowRight cycle through tiers
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
      const idx = TIER_IDS.indexOf(activeId);
      if (e.key === "ArrowLeft") {
        setActiveId(TIER_IDS[(idx - 1 + TIER_IDS.length) % TIER_IDS.length]);
      } else {
        setActiveId(TIER_IDS[(idx + 1) % TIER_IDS.length]);
      }
    },
    [activeId]
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  return (
    <section className="py-24 px-6">
      <div className="max-w-4xl mx-auto">
        <h2 className="text-2xl font-semibold text-[var(--rd-fg-primary)] mb-2 text-center">
          Three cognitive tiers
        </h2>
        <p className="text-sm text-[var(--rd-fg-muted)] text-center mb-12 max-w-md mx-auto">
          Every agent action flows through reflexive validation, deliberative
          planning, and reflective learning.
        </p>

        {/* Tier selector */}
        <div
          role="group"
          aria-label="Cognitive tier selector"
          className="flex justify-center gap-4 mb-8"
        >
          {TIERS.map((tier) => (
            <TierButton
              key={tier.id}
              tier={tier}
              isActive={activeId === tier.id}
              onClick={() => setActiveId(tier.id)}
            />
          ))}
        </div>

        <p className="text-center text-[10px] text-[var(--rd-fg-muted)] mb-6 -mt-4">
          Use arrow keys to navigate tiers
        </p>

        {/* Detail panel */}
        <div className="bg-[var(--rd-bg-surface-1)] border border-[var(--rd-bg-surface-3)] rounded-xl p-8">
          <p className="text-sm text-[var(--rd-fg-secondary)] mb-6">
            {active.description}
          </p>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            {active.components.map((comp) => (
              <div
                key={comp}
                className="flex items-center gap-2 px-3 py-2 rounded-md bg-[var(--rd-bg-surface-0)]"
              >
                <span
                  className="w-1.5 h-1.5 rounded-full shrink-0"
                  aria-hidden="true"
                  style={{ backgroundColor: active.color }}
                />
                <span className="text-xs text-[var(--rd-fg-secondary)]">
                  {comp}
                </span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
```

### 5. ContextAuction component

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/landing/ContextAuction.tsx`:

```tsx
import { useEffect, useState, memo } from "react";
import { useMediaQuery, BREAKPOINTS } from "../../design-system/useMediaQuery";

type Bidder = {
  name: string;
  /** 0–1 */
  allocation: number;
  color: string;
};

// MOCK: wire to GET /api/metrics when the VCG endpoint is available.
const BIDDERS: Bidder[] = [
  { name: "Neuro (knowledge)",    allocation: 0.35, color: "var(--rd-accent-purple)" },
  { name: "Task (context)",       allocation: 0.30, color: "var(--rd-rose-bright)" },
  { name: "Research (citations)", allocation: 0.20, color: "var(--rd-bone)" },
  { name: "Playbook (history)",   allocation: 0.10, color: "var(--rd-accent-gold)" },
  { name: "System (safety)",      allocation: 0.05, color: "var(--rd-success)" },
];

// ── AnimatedBar ───────────────────────────────────────────────────────────────

const AnimatedBar = memo(function AnimatedBar({
  allocation,
  color,
  delay,
}: {
  allocation: number;
  color: string;
  delay: number;
}) {
  const [width, setWidth] = useState(0);
  const prefersReduced = useMediaQuery(BREAKPOINTS.reducedMotion);

  useEffect(() => {
    if (prefersReduced) {
      setWidth(allocation * 100);
      return;
    }
    const timer = setTimeout(() => setWidth(allocation * 100), delay);
    return () => clearTimeout(timer);
  }, [allocation, delay, prefersReduced]);

  return (
    <div
      role="progressbar"
      aria-valuenow={Math.round(allocation * 100)}
      aria-valuemin={0}
      aria-valuemax={100}
      className="h-6 bg-[var(--rd-bg-surface-2)] rounded-md overflow-hidden"
    >
      <div
        className="h-full rounded-md transition-[width] duration-700 ease-out will-change-[width]"
        style={{ width: `${width}%`, backgroundColor: color }}
      />
    </div>
  );
});

// ── ContextAuction ────────────────────────────────────────────────────────────

export function ContextAuction() {
  return (
    <section className="py-24 px-6 bg-[var(--rd-bg-surface-0)]">
      <div className="max-w-3xl mx-auto">
        <h2 className="text-2xl font-semibold text-[var(--rd-fg-primary)] mb-2 text-center">
          VCG context auction
        </h2>
        <p className="text-sm text-[var(--rd-fg-muted)] text-center mb-12 max-w-md mx-auto">
          Five context bidders compete for token budget in every system prompt.
          The Vickrey-Clarke-Groves mechanism allocates space truthfully.
        </p>

        <div className="space-y-4">
          {BIDDERS.map((bidder, i) => (
            <div key={bidder.name}>
              <div className="flex items-center justify-between mb-1.5">
                <span className="text-sm text-[var(--rd-fg-secondary)]">
                  {bidder.name}
                </span>
                <span className="text-xs font-mono text-[var(--rd-fg-muted)]">
                  {Math.round(bidder.allocation * 100)}%
                </span>
              </div>
              <AnimatedBar
                allocation={bidder.allocation}
                color={bidder.color}
                delay={i * 150}
              />
            </div>
          ))}
        </div>

        <div className="mt-8 p-4 rounded-lg bg-[var(--rd-bg-surface-1)] border border-[var(--rd-bg-surface-3)]">
          <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-1">
            How it works
          </div>
          <p className="text-xs text-[var(--rd-fg-secondary)] leading-relaxed">
            Each bidder reports its marginal value for context tokens. The VCG
            mechanism computes the socially optimal allocation and charges each
            bidder what its presence costs the others. This prevents
            over-allocation — the knowledge store cannot crowd out safety
            constraints, and the task context cannot inflate its bid without
            paying the true social cost.
          </p>
        </div>
      </div>
    </section>
  );
}
```

### 6. StigmergyCanvas component

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/landing/StigmergyCanvas.tsx`:

```tsx
import { useRef, useEffect, memo } from "react";
import { useMediaQuery, BREAKPOINTS } from "../../design-system/useMediaQuery";

type Particle = {
  x: number;
  y: number;
  vx: number;
  vy: number;
  hue: number;
};

const PARTICLE_COUNT = 30;
const MAX_SPEED = 80;        // px/sec
const DRIFT_STRENGTH = 30;   // px/sec² random walk
const CONNECTION_DIST = 80;  // px
const TRAIL_DECAY = 0.97;
const CANVAS_HEIGHT = 300;

function createParticles(width: number, height: number): Particle[] {
  return Array.from({ length: PARTICLE_COUNT }, () => ({
    x: Math.random() * width,
    y: Math.random() * height,
    // Initial speed in px/sec (roughly 60 fps * 1.5 px/frame ≈ 90 px/sec)
    vx: (Math.random() - 0.5) * 90,
    vy: (Math.random() - 0.5) * 90,
    hue: 320 + Math.random() * 40,  // rose spectrum
  }));
}

// ── StigmergyCanvas ───────────────────────────────────────────────────────────

export const StigmergyCanvas = memo(function StigmergyCanvas() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const trailRef = useRef<HTMLCanvasElement>(null);
  const prefersReduced = useMediaQuery(BREAKPOINTS.reducedMotion);

  useEffect(() => {
    const canvas = canvasRef.current;
    const trailCanvas = trailRef.current;
    if (!canvas || !trailCanvas) return;

    const ctx = canvas.getContext("2d");
    const trailCtx = trailCanvas.getContext("2d");
    if (!ctx || !trailCtx) return;

    let frameId: number;
    let lastTimestamp = 0;

    const resize = () => {
      const w = canvas.parentElement?.getBoundingClientRect().width ?? 800;
      canvas.width = w;
      canvas.height = CANVAS_HEIGHT;
      trailCanvas.width = w;
      trailCanvas.height = CANVAS_HEIGHT;
    };

    resize();

    const resizeObserver = new ResizeObserver(resize);
    if (canvas.parentElement) resizeObserver.observe(canvas.parentElement);

    const particles = createParticles(canvas.width, CANVAS_HEIGHT);

    // If user prefers reduced motion, render a single static frame then stop.
    if (prefersReduced) {
      ctx.clearRect(0, 0, canvas.width, CANVAS_HEIGHT);
      for (const p of particles) {
        ctx.fillStyle = `hsla(${p.hue}, 50%, 65%, 0.5)`;
        ctx.beginPath();
        ctx.arc(p.x, p.y, 2.5, 0, Math.PI * 2);
        ctx.fill();
      }
      return () => resizeObserver.disconnect();
    }

    const tick = (timestamp: number) => {
      // Delta-time in seconds; cap at 100ms to avoid large jumps after tab focus
      const dt = Math.min((timestamp - (lastTimestamp || timestamp)) / 1000, 0.1);
      lastTimestamp = timestamp;

      const w = canvas.width;
      const h = CANVAS_HEIGHT;

      // Fade trails on the trail canvas
      trailCtx.fillStyle = `rgba(6, 6, 8, ${1 - TRAIL_DECAY})`;
      trailCtx.fillRect(0, 0, w, h);

      // Composite: clear main, draw trails first
      ctx.clearRect(0, 0, w, h);
      ctx.drawImage(trailCanvas, 0, 0);

      for (const p of particles) {
        // Integrate position
        p.x += p.vx * dt;
        p.y += p.vy * dt;

        // Bounce off walls
        if (p.x < 0)   { p.x = 0; p.vx = Math.abs(p.vx); }
        if (p.x > w)   { p.x = w; p.vx = -Math.abs(p.vx); }
        if (p.y < 0)   { p.y = 0; p.vy = Math.abs(p.vy); }
        if (p.y > h)   { p.y = h; p.vy = -Math.abs(p.vy); }

        // Random walk (accelerate, then clamp speed)
        p.vx += (Math.random() - 0.5) * DRIFT_STRENGTH * dt;
        p.vy += (Math.random() - 0.5) * DRIFT_STRENGTH * dt;

        const speed = Math.hypot(p.vx, p.vy);
        if (speed > MAX_SPEED) {
          const scale = MAX_SPEED / speed;
          p.vx *= scale;
          p.vy *= scale;
        }

        // Trail dot
        trailCtx.fillStyle = `hsla(${p.hue}, 40%, 50%, 0.15)`;
        trailCtx.beginPath();
        trailCtx.arc(p.x, p.y, 3, 0, Math.PI * 2);
        trailCtx.fill();

        // Agent dot
        ctx.fillStyle = `hsla(${p.hue}, 50%, 65%, 0.8)`;
        ctx.beginPath();
        ctx.arc(p.x, p.y, 2.5, 0, Math.PI * 2);
        ctx.fill();
      }

      // Proximity connections
      for (let i = 0; i < particles.length; i++) {
        for (let j = i + 1; j < particles.length; j++) {
          const dx = particles[i].x - particles[j].x;
          const dy = particles[i].y - particles[j].y;
          const dist = Math.hypot(dx, dy);
          if (dist < CONNECTION_DIST) {
            const alpha = (1 - dist / CONNECTION_DIST) * 0.15;
            ctx.strokeStyle = `rgba(170, 112, 136, ${alpha})`;
            ctx.lineWidth = 0.5;
            ctx.beginPath();
            ctx.moveTo(particles[i].x, particles[i].y);
            ctx.lineTo(particles[j].x, particles[j].y);
            ctx.stroke();
          }
        }
      }

      frameId = requestAnimationFrame(tick);
    };

    frameId = requestAnimationFrame(tick);

    return () => {
      cancelAnimationFrame(frameId);
      resizeObserver.disconnect();
    };
  }, [prefersReduced]);

  return (
    <section className="py-24 px-6">
      <div className="max-w-4xl mx-auto">
        <h2 className="text-2xl font-semibold text-[var(--rd-fg-primary)] mb-2 text-center">
          Stigmergic coordination
        </h2>
        <p className="text-sm text-[var(--rd-fg-muted)] text-center mb-8 max-w-md mx-auto">
          Agents deposit pheromone signals — traces of their decisions and
          discoveries — that other agents follow. No central coordinator needed.
        </p>

        <div
          className="relative rounded-xl overflow-hidden border border-[var(--rd-bg-surface-3)] bg-[var(--rd-bg-void)]"
          style={{ height: CANVAS_HEIGHT }}
        >
          {/* Trail layer (absolute, underneath) */}
          <canvas
            ref={trailRef}
            aria-hidden="true"
            className="absolute inset-0 will-change-transform"
            style={{ width: "100%", height: CANVAS_HEIGHT }}
          />
          {/* Agent layer (relative, on top) */}
          <canvas
            ref={canvasRef}
            aria-label="Stigmergy simulation: particles representing agents drift and leave fading trails"
            className="relative will-change-transform"
            style={{ width: "100%", height: CANVAS_HEIGHT }}
          />
        </div>

        {/* Legend */}
        <div className="flex items-center justify-center gap-6 mt-4">
          <div className="flex items-center gap-1.5">
            <span className="w-2 h-2 rounded-full bg-[var(--rd-rose)]" aria-hidden="true" />
            <span className="text-[10px] text-[var(--rd-fg-muted)]">Agent</span>
          </div>
          <div className="flex items-center gap-1.5">
            <span className="w-4 h-0.5 bg-[var(--rd-rose)]/30 inline-block" aria-hidden="true" />
            <span className="text-[10px] text-[var(--rd-fg-muted)]">Pheromone trail</span>
          </div>
          <div className="flex items-center gap-1.5">
            <span className="w-4 h-px bg-[var(--rd-rose)]/20 inline-block" aria-hidden="true" />
            <span className="text-[10px] text-[var(--rd-fg-muted)]">Proximity link</span>
          </div>
        </div>
      </div>
    </section>
  );
});
```

### 7. Assemble the landing page

- [ ] Replace `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/Landing.tsx`:

```tsx
import { useNavigate } from "react-router-dom";
import { HeroSection } from "./landing/HeroSection";
import { ArchitectureExplorer } from "./landing/ArchitectureExplorer";
import { ContextAuction } from "./landing/ContextAuction";
import { StigmergyCanvas } from "./landing/StigmergyCanvas";

export default function Landing() {
  const navigate = useNavigate();
  const handleLaunch = () => navigate("/app/chat");

  return (
    <div>
      <HeroSection onLaunch={handleLaunch} />
      <ArchitectureExplorer />
      <ContextAuction />
      <StigmergyCanvas />

      <footer className="py-12 px-6 border-t border-[var(--rd-bg-surface-2)] text-center">
        <p className="text-xs text-[var(--rd-fg-muted)]">
          Nunchi — hyperdimensional intelligence for autonomous agent orchestration
        </p>
      </footer>
    </div>
  );
}
```

---

## Verification

Run from `/Users/will/dev/nunchi/nunchi-dashboard`:

- [ ] `npm run typecheck` -- exits 0
- [ ] `npm run dev` -- open `http://localhost:5173/`
  - Hero section: breathing gradient backdrop, logo, title, four stats with count-up animation, "Launch dashboard" button
  - Architecture explorer: three tier buttons, click or arrow-key to switch detail panel
  - Context auction: five bars animate in with a staggered delay, percentages shown on the right
  - Stigmergy canvas: 30 dots drift smoothly, leave fading trails, proximity links appear between nearby agents
  - Footer at bottom
  - "Launch dashboard" navigates to `/app/chat`
- [ ] No console errors in devtools
- [ ] Resize the browser -- canvas resizes via ResizeObserver, layout stays centered
- [ ] Open devtools accessibility tree -- canvas has an `aria-label` describing the simulation
- [ ] Enable "Prefers reduced motion" in devtools (Rendering tab) -- count-up snaps to target, canvas shows a static frame, gradient stops breathing
- [ ] Keyboard test: focus the Architecture Explorer tier buttons, press ArrowRight/ArrowLeft to cycle tiers without using the mouse

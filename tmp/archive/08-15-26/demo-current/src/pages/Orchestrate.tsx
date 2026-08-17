import { useState, useCallback, useRef, useEffect, type CSSProperties } from 'react';
import type { GateState } from '../lib/types';
import { SectionHero } from '../design/SectionHero';
import { GateBar } from '../design/GateBar';
import { IdlePhase } from './orchestrate/IdlePhase';
import { scenarios, scenarioData } from './orchestrate/scenarios';
import { useTerminal, type GateEvent } from '../hooks/useTerminal';
import type { DemoConfig } from './orchestrate/ConfigPanel';
import { useConfig } from '../lib/config-context';
import { SERVE_URL } from '../lib/config';

// ─── Types ───

type PipelinePhase = 'idea' | 'prd' | 'plan' | 'tasks' | 'run' | 'done';

interface Step {
  label: string;
  command: string;
  phase: PipelinePhase;
  timeoutMs?: number;
}

interface PrdData {
  title: string;
  slug: string;
  status: string;
  created: string;
  requirementsCount: number;
  acceptancesCount: number;
  summary: string;
  reasons: string[];
}

interface TaskData {
  id: string;
  title: string;
  description: string;
  tier: string;
  status: string;
  subtasks: string[];
}

interface PlanData {
  taskCount: number;
  tierBreakdown: { tier: string; count: number }[];
  doneCount: number;
  currentTask: TaskData | null;
}

// ─── Steps for each scenario ───

function getSteps(slug: string, config: DemoConfig): Step[] {
  const globalFlags = config.model ? ` --model ${config.model}` : '';
  const roko = `roko${globalFlags}`;
  const ideaText = `Build a ${slug.replace(/-/g, ' ')}`;

  return [
    {
      label: 'Capture idea',
      command: `roko prd idea "${ideaText}"`,
      phase: 'idea',
    },
    {
      label: 'Draft PRD',
      command: `${roko} prd draft new "${slug}"`,
      phase: 'prd',
      timeoutMs: 300_000,
    },
    {
      label: 'Generate plan',
      command: `${roko} prd plan ${slug}`,
      phase: 'plan',
      timeoutMs: 300_000,
    },
    {
      label: 'Execute plan',
      command: `${roko} plan run .roko/plans/`,
      phase: 'run',
      timeoutMs: 600_000,
    },
  ];
}

// ─── Styles ───

const pageStyle: CSSProperties = {
  height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden',
};

const topBarStyle: CSSProperties = {
  display: 'flex', alignItems: 'center', gap: 12,
  padding: '8px 20px',
  borderBottom: '1px solid var(--border-soft)',
  background: 'rgba(10, 8, 16, 0.6)',
};

const mainLayout: CSSProperties = {
  flex: 1, display: 'grid', gridTemplateColumns: '1fr 1fr',
  gap: 0, minHeight: 0,
};

const contentPanelStyle: CSSProperties = {
  display: 'flex', flexDirection: 'column', overflow: 'auto',
  borderRight: '1px solid var(--border-soft)',
};

const termPanelStyle: CSSProperties = {
  display: 'flex', flexDirection: 'column', minHeight: 0,
};

const phaseTabBar: CSSProperties = {
  display: 'flex', gap: 0, borderBottom: '1px solid var(--border-soft)',
};

const btnBase: CSSProperties = {
  fontFamily: 'var(--mono)', fontSize: '9px', fontWeight: 600,
  letterSpacing: '0.08em', textTransform: 'uppercase',
  padding: '5px 10px', border: '1px solid var(--border)',
  color: 'var(--text-dim)', cursor: 'pointer',
  transition: 'all var(--duration-fast) var(--ease-out)',
  whiteSpace: 'nowrap', background: 'none',
};

// ─── Phase tab component ───

function PhaseTab({ label, status, active, onClick }: {
  label: string;
  status: 'pending' | 'active' | 'done';
  active: boolean;
  onClick: () => void;
}) {
  const dotColor = status === 'done' ? 'var(--success)' :
    status === 'active' ? 'var(--rose)' : 'var(--text-ghost)';

  return (
    <button onClick={onClick} style={{
      flex: 1, display: 'flex', alignItems: 'center', gap: 6,
      padding: '8px 12px', border: 'none', cursor: 'pointer',
      background: active ? 'rgba(58, 32, 48, 0.15)' : 'transparent',
      borderBottom: active ? '2px solid var(--rose)' : '2px solid transparent',
      transition: 'all 150ms ease',
    }}>
      <span style={{
        width: 7, height: 7, borderRadius: '50%', background: dotColor,
        boxShadow: status === 'active' ? '0 0 6px var(--rose)' : 'none',
        animation: status === 'active' ? 'pulse 2.2s ease-in-out infinite' : 'none',
      }} />
      <span style={{
        fontFamily: 'var(--mono)', fontSize: '10px', fontWeight: 600,
        letterSpacing: '0.1em', textTransform: 'uppercase',
        color: active ? 'var(--rose)' :
          status === 'done' ? 'var(--success)' : 'var(--text-ghost)',
      }}>
        {label}
      </span>
    </button>
  );
}

// ─── Status dot ───

function StatusDot({ status }: { status: 'connected' | 'connecting' | 'disconnected' }) {
  const bg = status === 'connected' ? 'var(--success)' :
    status === 'connecting' ? 'var(--warning)' : 'var(--rose-dim)';
  return (
    <span style={{
      width: 6, height: 6, borderRadius: '50%', display: 'inline-block',
      background: bg,
      boxShadow: status === 'connected' ? '0 0 6px rgba(122, 138, 120, 0.5)' : 'none',
    }} />
  );
}

// ─── Content panels per phase ───

function IdeaContent({ text, slug }: { text: string; slug: string }) {
  return (
    <div style={{ padding: 20 }}>
      <div style={{
        borderLeft: '3px solid var(--rose-dim)', padding: '16px 20px',
        fontFamily: 'var(--display)', fontStyle: 'italic',
        fontSize: '18px', lineHeight: 1.5, color: 'var(--text-strong)',
      }}>
        {text}
      </div>
      <div style={{
        marginTop: 16, fontFamily: 'var(--mono)', fontSize: '11px',
        color: 'var(--text-soft)', lineHeight: 1.6,
      }}>
        {text}
      </div>
      <div style={{ marginTop: 16, display: 'flex', gap: 8, flexWrap: 'wrap' }}>
        <Tag>{slug}</Tag>
      </div>
    </div>
  );
}

function PrdContent({ prd }: { prd: PrdData }) {
  return (
    <div style={{ padding: 20 }}>
      <div style={{
        display: 'flex', justifyContent: 'space-between', alignItems: 'center',
        marginBottom: 8,
      }}>
        <span style={{
          fontFamily: 'var(--mono)', fontSize: '9px', letterSpacing: '0.12em',
          textTransform: 'uppercase', color: 'var(--text-ghost)',
        }}>GENERATED PRD</span>
        <Tag color="rose">{prd.status.toUpperCase()}</Tag>
      </div>
      <h2 style={{
        fontFamily: 'var(--display)', fontSize: '22px', fontWeight: 400,
        color: 'var(--text-strong)', margin: '0 0 12px',
      }}>{prd.title}</h2>
      <div style={{
        fontFamily: 'var(--mono)', fontSize: '11px', color: 'var(--text-dim)',
        marginBottom: 16,
      }}>
        id: prd-{prd.slug} title: {prd.title} status: {prd.status} created: {prd.created}
      </div>
      {prd.summary && (
        <div style={{
          fontFamily: 'var(--mono)', fontSize: '11px', color: 'var(--text-soft)',
          lineHeight: 1.6, marginBottom: 16,
        }}>
          {prd.summary}
        </div>
      )}
      <table style={{
        width: '100%', borderCollapse: 'collapse',
        fontFamily: 'var(--mono)', fontSize: '11px',
      }}>
        <thead>
          <tr>
            {['REQUIREMENTS', 'ACCEPTANCE', 'SLUG'].map(h => (
              <th key={h} style={{
                textAlign: 'left', padding: '6px 10px',
                borderBottom: '1px solid var(--border-soft)',
                color: 'var(--text-ghost)', fontSize: '9px',
                letterSpacing: '0.1em',
              }}>{h}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          <tr>
            <td style={{ padding: '8px 10px', color: 'var(--bone)' }}>{prd.requirementsCount}</td>
            <td style={{ padding: '8px 10px', color: 'var(--bone)' }}>{prd.acceptancesCount}</td>
            <td style={{ padding: '8px 10px', color: 'var(--bone)' }}>{prd.slug}</td>
          </tr>
        </tbody>
      </table>
      {prd.reasons.length > 0 && (
        <div style={{ marginTop: 16, display: 'flex', gap: 8, flexWrap: 'wrap' }}>
          {prd.reasons.map((r, i) => <Tag key={i}>{r}</Tag>)}
        </div>
      )}
    </div>
  );
}

function TasksContent({ plan, slug }: { plan: PlanData; slug: string }) {
  const pct = plan.taskCount > 0 ? Math.round((plan.doneCount / plan.taskCount) * 100) : 0;
  return (
    <div style={{ padding: 20 }}>
      <div style={{
        fontFamily: 'var(--display)', fontSize: '14px', color: 'var(--text-strong)',
        marginBottom: 4,
      }}>
        {slug.replace(/-/g, ' ').replace(/\b\w/g, c => c.toUpperCase())}
        <span style={{
          fontFamily: 'var(--mono)', fontSize: '11px', color: 'var(--text-dim)',
          marginLeft: 12,
        }}>
          {pct}% ({plan.doneCount}/{plan.taskCount})
        </span>
      </div>
      <div style={{
        height: 4, background: 'var(--border)', marginBottom: 16, overflow: 'hidden',
      }}>
        <div style={{
          height: '100%', width: `${pct}%`,
          background: 'var(--rose)', transition: 'width 300ms ease',
        }} />
      </div>

      {plan.currentTask && (
        <div style={{
          padding: '16px 20px', border: '1px solid var(--border)',
          background: 'rgba(10, 8, 16, 0.5)', marginBottom: 16,
        }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
            <h3 style={{
              fontFamily: 'var(--mono)', fontSize: '14px', fontWeight: 700,
              letterSpacing: '0.04em', textTransform: 'uppercase',
              color: 'var(--text-strong)', margin: 0,
            }}>
              {plan.currentTask.title}
            </h3>
            <Tag color="dim">{plan.currentTask.tier}</Tag>
          </div>
          {plan.currentTask.description && (
            <p style={{
              fontFamily: 'var(--mono)', fontSize: '11px', color: 'var(--text-soft)',
              margin: '8px 0 0', lineHeight: 1.5,
            }}>
              {plan.currentTask.description}
            </p>
          )}
          {plan.currentTask.subtasks.length > 0 && (
            <ul style={{
              margin: '12px 0 0', padding: '0 0 0 16px',
              fontFamily: 'var(--mono)', fontSize: '11px', color: 'var(--text-dim)',
              lineHeight: 1.6,
            }}>
              {plan.currentTask.subtasks.map((s, i) => <li key={i}>{s}</li>)}
            </ul>
          )}
        </div>
      )}

      <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
        {plan.tierBreakdown.map(t => (
          <Tag key={t.tier} color={
            t.tier.includes('1') || t.tier.toLowerCase().includes('fast') ? 'dim' :
            t.tier.includes('2') || t.tier.toLowerCase().includes('build') ? 'bone' : 'rose'
          }>
            {t.tier}: {t.count}
          </Tag>
        ))}
        <Tag>{plan.doneCount}/{plan.taskCount} DONE</Tag>
      </div>
    </div>
  );
}

function RunContent({ plan, gates, metrics, elapsed }: {
  plan: PlanData; gates: GateState[]; metrics: { cost: number; tokens: number }; elapsed: number;
}) {
  return (
    <div style={{ padding: 20 }}>
      <div style={{
        fontFamily: 'var(--mono)', fontSize: '9px', letterSpacing: '0.12em',
        textTransform: 'uppercase', color: 'var(--text-ghost)', marginBottom: 12,
      }}>EXECUTION</div>
      <div style={{ marginBottom: 16 }}>
        <GateBar gates={gates} />
      </div>
      <div style={{
        display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 12,
        fontFamily: 'var(--mono)', fontSize: '11px',
      }}>
        <div>
          <div style={{ color: 'var(--text-ghost)', fontSize: '9px', marginBottom: 4 }}>COST</div>
          <div style={{ color: 'var(--bone)', fontSize: '16px' }}>
            ${metrics.cost > 0 ? metrics.cost.toFixed(4) : '0.0000'}
          </div>
        </div>
        <div>
          <div style={{ color: 'var(--text-ghost)', fontSize: '9px', marginBottom: 4 }}>TOKENS</div>
          <div style={{ color: 'var(--bone)', fontSize: '16px' }}>{metrics.tokens.toLocaleString()}</div>
        </div>
        <div>
          <div style={{ color: 'var(--text-ghost)', fontSize: '9px', marginBottom: 4 }}>ELAPSED</div>
          <div style={{ color: 'var(--bone)', fontSize: '16px' }}>{elapsed.toFixed(1)}s</div>
        </div>
      </div>
      {plan.currentTask && (
        <div style={{ marginTop: 16, fontFamily: 'var(--mono)', fontSize: '10px', color: 'var(--text-dim)' }}>
          Running: {plan.currentTask.id} — {plan.currentTask.title}
        </div>
      )}
    </div>
  );
}

function DoneContent({ metrics, elapsed, plan }: {
  metrics: { cost: number; tokens: number }; elapsed: number; plan: PlanData;
}) {
  return (
    <div style={{ padding: 20 }}>
      <div style={{
        fontFamily: 'var(--mono)', fontSize: '14px', color: 'var(--success)',
        fontWeight: 600, marginBottom: 12,
      }}>
        {'\u2713'} ALL STEPS COMPLETE
      </div>
      <div style={{
        display: 'grid', gridTemplateColumns: '1fr 1fr 1fr 1fr', gap: 12,
        fontFamily: 'var(--mono)', fontSize: '11px',
      }}>
        {[
          ['COST', `$${metrics.cost.toFixed(4)}`],
          ['TOKENS', metrics.tokens.toLocaleString()],
          ['ELAPSED', `${elapsed.toFixed(1)}s`],
          ['TASKS', `${plan.doneCount}/${plan.taskCount}`],
        ].map(([label, value]) => (
          <div key={label}>
            <div style={{ color: 'var(--text-ghost)', fontSize: '9px', marginBottom: 4 }}>{label}</div>
            <div style={{ color: 'var(--bone)' }}>{value}</div>
          </div>
        ))}
      </div>
    </div>
  );
}

function Tag({ children, color = 'ghost' }: { children: React.ReactNode; color?: string }) {
  const c = color === 'rose' ? 'var(--rose)' :
    color === 'bone' ? 'var(--bone)' :
    color === 'dim' ? 'var(--text-dim)' :
    color === 'success' ? 'var(--success)' : 'var(--text-ghost)';
  return (
    <span style={{
      fontFamily: 'var(--mono)', fontSize: '9px', fontWeight: 600,
      letterSpacing: '0.08em', padding: '3px 8px',
      border: `1px solid ${c}40`, color: c,
      textTransform: 'uppercase',
    }}>
      {children}
    </span>
  );
}

// ─── Helpers ───

function extractPrdFromSeed(scenarioId: string): PrdData {
  const data = scenarioData[scenarioId];
  if (!data) return { title: '', slug: '', status: 'draft', created: '', requirementsCount: 0, acceptancesCount: 0, summary: '', reasons: [] };
  return {
    title: data.prd.title,
    slug: data.prd.slug,
    status: 'draft',
    created: new Date().toISOString().split('T')[0],
    requirementsCount: data.prd.requirementsCount,
    acceptancesCount: data.prd.acceptancesCount,
    summary: data.prd.summary,
    reasons: data.plan.tierBreakdown.map(t => `${t.tier}: ${t.count} tasks`),
  };
}

function extractPlanFromSeed(scenarioId: string, doneCount: number): PlanData {
  const data = scenarioData[scenarioId];
  if (!data) return { taskCount: 0, tierBreakdown: [], doneCount: 0, currentTask: null };
  const tasks = data.tasks || [];
  const current = tasks.find(t => t.status === 'pending' || t.status === 'running') || tasks[0];
  return {
    taskCount: data.plan.taskCount,
    tierBreakdown: data.plan.tierBreakdown,
    doneCount,
    currentTask: current ? {
      id: current.id, title: current.name, description: '',
      tier: current.tier, status: current.status, subtasks: [],
    } : null,
  };
}

// ─── Live PRD polling ───

function useLivePrd(slug: string, phase: PipelinePhase): PrdData | null {
  const [data, setData] = useState<PrdData | null>(null);

  useEffect(() => {
    if (phase === 'idea' || !slug) return;

    const fetchPrd = async () => {
      try {
        const res = await fetch(`${SERVE_URL}/api/prds/${slug}`);
        if (!res.ok) return;
        const prd = await res.json();
        setData({
          title: prd.title || slug,
          slug: prd.slug || slug,
          status: prd.status || 'draft',
          created: prd.created || new Date().toISOString().split('T')[0],
          requirementsCount: prd.requirements_count ?? 0,
          acceptancesCount: prd.acceptance_count ?? 0,
          summary: prd.summary || '',
          reasons: [],
        });
      } catch { /* use seed data */ }
    };

    fetchPrd();
    const interval = setInterval(fetchPrd, 5000);
    return () => clearInterval(interval);
  }, [slug, phase]);

  return data;
}

// ─── Main Page ───

export function OrchestratePage() {
  const [selectedId, setSelectedId] = useState('simple');
  const { config } = useConfig();
  const [appPhase, setAppPhase] = useState<'idle' | 'running' | 'complete'>('idle');
  const [currentStep, setCurrentStep] = useState(0);
  const [activePhase, setActivePhase] = useState<PipelinePhase>('idea');
  const [autoMode, setAutoMode] = useState(false);
  const autoRef = useRef(false);
  const [gates, setGates] = useState<GateState[]>([
    { name: 'compile', status: 'pending' },
    { name: 'test', status: 'pending' },
    { name: 'clippy', status: 'pending' },
    { name: 'diff', status: 'pending' },
  ]);
  const [metrics, setMetrics] = useState({ cost: 0, tokens: 0 });
  const [elapsed, setElapsed] = useState(0);
  const [workspaceDir, setWorkspaceDir] = useState<string | null>(null);
  const [needsInit, setNeedsInit] = useState(false);
  const [doneCount, setDoneCount] = useState(0);
  const [runningStep, setRunningStep] = useState(false);
  const startTimeRef = useRef(0);
  const timerRef = useRef<ReturnType<typeof setInterval> | undefined>(undefined);

  const selected = scenarios.find(s => s.id === selectedId)!;
  const steps = getSteps(selected.slug, config);
  const PHASES: PipelinePhase[] = ['idea', 'prd', 'plan', 'tasks', 'run', 'done'];

  // Data
  const livePrd = useLivePrd(selected.slug, activePhase);
  const prdData = livePrd || extractPrdFromSeed(selectedId);
  const planData = extractPlanFromSeed(selectedId, doneCount);

  // Terminal callbacks
  const handleGate = useCallback((event: GateEvent) => {
    setGates(prev => prev.map(g =>
      g.name === event.name ? { ...g, status: event.status } : g
    ));
  }, []);
  const handleCost = useCallback((cost: number) => {
    setMetrics(prev => ({ ...prev, cost: prev.cost + cost }));
  }, []);
  const handleTokens = useCallback((tokens: number) => {
    setMetrics(prev => ({ ...prev, tokens: prev.tokens + tokens }));
  }, []);

  const { containerRef, handle, status: termStatus } = useTerminal({
    onGate: handleGate,
    onCost: handleCost,
    onTokens: handleTokens,
  });

  // Elapsed timer
  useEffect(() => {
    if (appPhase === 'running') {
      startTimeRef.current = Date.now();
      timerRef.current = setInterval(() => {
        setElapsed((Date.now() - startTimeRef.current) / 1000);
      }, 250);
    } else {
      clearInterval(timerRef.current);
    }
    return () => clearInterval(timerRef.current);
  }, [appPhase]);

  useEffect(() => { autoRef.current = autoMode; }, [autoMode]);

  // START
  const handleStart = useCallback(() => {
    setAppPhase('running');
    setCurrentStep(0);
    setActivePhase('idea');
    setAutoMode(false);
    setGates([
      { name: 'compile', status: 'pending' },
      { name: 'test', status: 'pending' },
      { name: 'clippy', status: 'pending' },
      { name: 'diff', status: 'pending' },
    ]);
    setMetrics({ cost: 0, tokens: 0 });
    setElapsed(0);
    setDoneCount(0);
    setWorkspaceDir(null);
    setNeedsInit(true);
  }, []);

  // Workspace init
  useEffect(() => {
    if (!needsInit || !handle || termStatus !== 'connected') return;
    setNeedsInit(false);

    const init = async () => {
      const dir = `/tmp/roko-demo-${Date.now().toString(36)}`;
      setWorkspaceDir(dir);
      handle.clearBuffer();
      await handle.execCmd(`mkdir -p ${dir} && cd ${dir} && git init -q && roko init`);
      handle.clearTerminal();
    };

    init();
  }, [needsInit, handle, termStatus]);

  // Execute a step
  const execStep = useCallback(async (stepIndex?: number) => {
    if (!handle || runningStep) return;
    const idx = stepIndex ?? currentStep;
    const step = steps[idx];
    if (!step) return;

    setRunningStep(true);
    setActivePhase(step.phase);
    await handle.execCmd(step.command, step.timeoutMs);

    if (idx < steps.length - 1) {
      const nextIdx = idx + 1;
      setCurrentStep(nextIdx);
      setActivePhase(steps[nextIdx].phase);
    } else {
      setAppPhase('complete');
      setActivePhase('done');
    }
    setRunningStep(false);
  }, [handle, steps, currentStep, runningStep]);

  // Auto mode
  useEffect(() => {
    if (!autoMode || !handle || appPhase !== 'running' || runningStep) return;
    if (currentStep >= steps.length) return;
    const timer = setTimeout(() => {
      if (autoRef.current) execStep(currentStep);
    }, 500);
    return () => clearTimeout(timer);
  }, [autoMode, handle, appPhase, currentStep, runningStep, execStep, steps.length]);

  // Reset
  const handleReset = useCallback(() => {
    setAppPhase('idle');
    setCurrentStep(0);
    setActivePhase('idea');
    setAutoMode(false);
    setGates([
      { name: 'compile', status: 'pending' },
      { name: 'test', status: 'pending' },
      { name: 'clippy', status: 'pending' },
      { name: 'diff', status: 'pending' },
    ]);
    setMetrics({ cost: 0, tokens: 0 });
    setElapsed(0);
    setDoneCount(0);
    if (workspaceDir && handle) {
      handle.sendRaw(`cd /tmp && rm -rf ${workspaceDir}\r`);
    }
    setWorkspaceDir(null);
  }, [workspaceDir, handle]);

  // ─── Idle ───
  if (appPhase === 'idle') {
    return (
      <div style={{ ...pageStyle, overflow: 'auto' }}>
        <SectionHero line="One request. Autonomous planning, routing, execution, and verification." />
        <IdlePhase scenarios={scenarios} selectedId={selectedId} onSelect={setSelectedId} onStart={handleStart} />
      </div>
    );
  }

  // ─── Running / Complete ───
  const phaseStatus = (p: PipelinePhase): 'pending' | 'active' | 'done' => {
    const pi = PHASES.indexOf(p);
    const ci = PHASES.indexOf(activePhase);
    if (pi < ci) return 'done';
    if (pi === ci) return 'active';
    return 'pending';
  };

  return (
    <div style={pageStyle}>
      {/* Top bar: workflow label + controls */}
      <div style={topBarStyle}>
        <Tag color="rose">CORE</Tag>
        <span style={{ fontFamily: 'var(--mono)', fontSize: '12px', fontWeight: 600, color: 'var(--text-strong)' }}>
          PRD Pipeline
        </span>
        <span style={{ fontFamily: 'var(--mono)', fontSize: '11px', color: 'var(--text-ghost)', flex: 1 }}>
          Pick an example, generate the PRD, generate tasks.toml, then watch execution.
        </span>
        <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
          <span style={{ display: 'flex', alignItems: 'center', gap: 4, fontFamily: 'var(--mono)', fontSize: '9px', color: 'var(--success)' }}>
            <StatusDot status={termStatus} /> LIVE
          </span>
          <button style={btnBase} disabled>1x</button>
          <button
            style={{ ...btnBase, ...(autoMode ? { borderColor: 'var(--success)', color: 'var(--success)', background: 'rgba(122, 138, 120, 0.15)' } : {}) }}
            onClick={() => setAutoMode(true)}
          >Auto</button>
          <button style={btnBase} onClick={() => setAutoMode(false)}>Stop</button>
          <button
            style={{ ...btnBase, borderColor: 'var(--rose-dim)', color: 'var(--rose-glow)', background: runningStep ? 'rgba(58, 32, 48, 0.4)' : 'rgba(58, 32, 48, 0.15)' }}
            onClick={() => execStep()}
            disabled={!handle || termStatus !== 'connected' || runningStep || appPhase === 'complete'}
          >{runningStep ? '\u25A0' : '\u25B6'}</button>
          <button style={btnBase} onClick={() => { if (currentStep > 0) setCurrentStep(p => p - 1); }} disabled={currentStep === 0}>{'<'}</button>
          <button style={btnBase} onClick={handleReset}>{'\u21BA'}</button>
        </div>
      </div>

      {/* Main layout: content LEFT, terminal RIGHT */}
      <div style={mainLayout}>
        {/* LEFT: content */}
        <div style={contentPanelStyle}>
          <div style={phaseTabBar}>
            {PHASES.map(p => (
              <PhaseTab key={p} label={p} status={phaseStatus(p)} active={p === activePhase} onClick={() => setActivePhase(p)} />
            ))}
          </div>

          {activePhase === 'idea' && <IdeaContent text={`Build a ${selected.slug.replace(/-/g, ' ')}`} slug={selected.slug} />}
          {activePhase === 'prd' && <PrdContent prd={prdData} />}
          {(activePhase === 'plan' || activePhase === 'tasks') && <TasksContent plan={planData} slug={selected.slug} />}
          {activePhase === 'run' && <RunContent plan={planData} gates={gates} metrics={metrics} elapsed={elapsed} />}
          {activePhase === 'done' && <DoneContent metrics={metrics} elapsed={elapsed} plan={planData} />}

          {/* Bottom status */}
          <div style={{ marginTop: 'auto', padding: '8px 16px', borderTop: '1px solid var(--border-soft)', display: 'flex', gap: 8, alignItems: 'center' }}>
            <Tag color={termStatus === 'connected' ? 'success' : 'ghost'}>
              {termStatus === 'connected' ? 'RS LIVE' : `RS ${termStatus.toUpperCase()}`}
            </Tag>
            <Tag>{selected.slug}</Tag>
            {appPhase === 'complete' && (
              <button style={{ ...btnBase, borderColor: 'var(--success)', color: 'var(--success)', marginLeft: 'auto' }} onClick={handleReset}>
                RUN ANOTHER
              </button>
            )}
          </div>
        </div>

        {/* RIGHT: terminal */}
        <div style={termPanelStyle}>
          <div style={{
            display: 'flex', alignItems: 'center', gap: 6,
            padding: '6px 14px', borderBottom: '1px solid var(--border-soft)',
            background: 'rgba(10, 8, 16, 0.9)', fontFamily: 'var(--mono)', fontSize: '10px',
          }}>
            <StatusDot status={termStatus} />
            <span style={{ fontWeight: 600, color: 'var(--text-strong)', letterSpacing: '0.1em', textTransform: 'uppercase', fontSize: '9px' }}>
              ROKO COMMANDS
            </span>
            <span style={{ marginLeft: 'auto', color: 'var(--text-ghost)', fontSize: 9 }}>
              {termStatus.toUpperCase()}
            </span>
          </div>
          <div ref={containerRef} style={{ flex: 1, padding: '4px 8px', minHeight: 0 }} />
        </div>
      </div>
    </div>
  );
}

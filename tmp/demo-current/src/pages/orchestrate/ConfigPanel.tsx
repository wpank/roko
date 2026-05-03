import { useState, useEffect, type CSSProperties } from 'react';
import { SERVE_URL } from '../../lib/config';

// ─── Config types ───

export interface DemoConfig {
  provider: string;
  model: string;
  effort: 'low' | 'medium' | 'high';
  clippy_enabled: boolean;
  skip_tests: boolean;
  max_iterations: number;
  max_plan_usd: number;
  max_turn_usd: number;
  pipeline: 'mechanical' | 'focused' | 'integrative' | 'architectural';
  routing_mode: 'auto_override' | 'manual' | 'auto';
  max_agents: number;
  parallel_enabled: boolean;
  toml_overrides: string;
}

export const DEFAULT_CONFIG: DemoConfig = {
  provider: 'anthropic',
  model: 'claude-sonnet-4-6',
  effort: 'medium',
  clippy_enabled: true,
  skip_tests: false,
  max_iterations: 3,
  max_plan_usd: 25.0,
  max_turn_usd: 3.0,
  pipeline: 'focused',
  routing_mode: 'auto_override',
  max_agents: 8,
  parallel_enabled: false,
  toml_overrides: '',
};

// Fallback — used if /api/providers fails
const FALLBACK_PROVIDERS: Record<string, string[]> = {
  anthropic: ['claude-haiku-4-5', 'claude-sonnet-4-6', 'claude-opus-4-6'],
  openai: ['gpt-4.1', 'gpt-4.1-mini', 'gpt-4.1-nano', 'o3', 'o3-mini', 'o4-mini', 'codex-mini-latest'],
  perplexity: ['sonar', 'sonar-pro', 'sonar-reasoning-pro'],
  moonshot: ['kimi-k2.6', 'kimi-k2.5', 'kimi-k2'],
  zhipu: ['glm-5.1', 'glm-5-turbo', 'glm-4.5-flash', 'glm-4-plus'],
  gemini: ['gemini-2.5-flash', 'gemini-2.5-pro'],
  cerebras: ['llama-3.3-70b', 'llama-3.1-8b', 'llama-4-scout-17b-16e'],
  ollama: ['llama3.1', 'codellama', 'mistral', 'deepseek-coder-v2'],
};

const PIPELINES = ['mechanical', 'focused', 'integrative', 'architectural'] as const;

interface ConfigPanelProps {
  config: DemoConfig;
  onChange: (config: DemoConfig) => void;
}

// ─── Styles ───

const panelStyle: CSSProperties = {
  width: '100%',
  border: '1px solid var(--border-soft)',
  background: 'var(--bg-raised)',
  overflow: 'hidden',
  boxShadow: 'inset 0 1px 0 rgba(255,255,255,0.03)',
};

const headerStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  padding: '8px 14px',
  cursor: 'pointer',
  userSelect: 'none',
  transition: 'background var(--duration-fast) var(--ease-out)',
};

const bodyStyle: CSSProperties = {
  display: 'grid',
  gridTemplateColumns: '1fr 1fr',
  gap: 0,
  borderTop: '1px solid var(--border-soft)',
};

const sectionStyle: CSSProperties = {
  padding: '10px 14px',
  borderBottom: '1px solid var(--border-soft)',
  borderRight: '1px solid var(--border-soft)',
};

const sectionLabel: CSSProperties = {
  fontFamily: 'var(--mono)',
  fontSize: '10px',
  fontWeight: 600,
  letterSpacing: '0.14em',
  textTransform: 'uppercase',
  color: 'var(--text-ghost)',
  marginBottom: 6,
};

const fieldStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  marginBottom: 5,
};

const labelStyle: CSSProperties = {
  fontFamily: 'var(--mono)',
  fontSize: '11px',
  color: 'var(--text-dim)',
};

const selectStyle: CSSProperties = {
  fontFamily: 'var(--mono)',
  fontSize: '11px',
  color: 'var(--bone)',
  background: 'var(--bg-deeper)',
  border: '1px solid var(--border)',
  padding: '3px 6px',
  minWidth: 120,
  cursor: 'pointer',
  outline: 'none',
};

const inputStyle: CSSProperties = {
  ...selectStyle,
  width: 64,
  minWidth: 0,
  textAlign: 'right',
};

const checkStyle: CSSProperties = {
  accentColor: 'var(--rose)',
  cursor: 'pointer',
  width: 13,
  height: 13,
};

const fullWidthSection: CSSProperties = {
  ...sectionStyle,
  gridColumn: '1 / -1',
  borderRight: 'none',
};

const textareaStyle: CSSProperties = {
  ...selectStyle,
  width: '100%',
  minHeight: 48,
  resize: 'vertical' as const,
  textAlign: 'left',
  lineHeight: 1.4,
  boxSizing: 'border-box',
};

// ─── Component ───

export function ConfigPanel({ config, onChange }: ConfigPanelProps) {
  const [expanded, setExpanded] = useState(false);
  const [providers, setProviders] = useState<Record<string, string[]>>(FALLBACK_PROVIDERS);

  // Try to fetch live provider/model list from roko-serve
  useEffect(() => {
    fetch(`${SERVE_URL}/api/providers`)
      .then(r => r.ok ? r.json() : null)
      .then(data => {
        if (data && typeof data === 'object') {
          const map: Record<string, string[]> = {};
          // API returns array of { name, models: [...] } or similar
          if (Array.isArray(data)) {
            for (const p of data) {
              if (p.name && Array.isArray(p.models)) {
                map[p.name] = p.models;
              }
            }
          } else if (data.providers) {
            for (const [k, v] of Object.entries(data.providers)) {
              const pv = v as { models?: string[] };
              if (pv.models) map[k] = pv.models;
            }
          }
          if (Object.keys(map).length > 0) setProviders(map);
        }
      })
      .catch(() => { /* use fallback */ });
  }, []);

  const set = <K extends keyof DemoConfig>(key: K, value: DemoConfig[K]) => {
    onChange({ ...config, [key]: value });
  };

  const models = providers[config.provider] ?? [];

  return (
    <div style={panelStyle}>
      <div
        style={headerStyle}
        onClick={() => setExpanded(!expanded)}
        onMouseEnter={e => { e.currentTarget.style.background = 'rgba(58, 32, 48, 0.08)'; }}
        onMouseLeave={e => { e.currentTarget.style.background = ''; }}
      >
        <span style={{
          fontFamily: 'var(--mono)', fontSize: '9px', fontWeight: 600,
          letterSpacing: '0.12em', textTransform: 'uppercase', color: 'var(--rose)',
        }}>
          CONFIGURATION
        </span>
        <span style={{
          fontFamily: 'var(--mono)', fontSize: '10px', color: 'var(--text-ghost)',
          display: 'flex', alignItems: 'center', gap: 6,
        }}>
          <span style={{ color: 'var(--text-dim)' }}>
            {config.provider}/{config.model}
          </span>
          <span style={{
            fontSize: 8,
            transition: 'transform 150ms ease',
            transform: expanded ? 'rotate(180deg)' : 'rotate(0deg)',
            color: 'var(--text-ghost)',
          }}>
            {'\u25BC'}
          </span>
        </span>
      </div>

      {expanded && (
        <div style={bodyStyle}>
          {/* Agent */}
          <div style={sectionStyle}>
            <div style={sectionLabel}>AGENT</div>
            <div style={fieldStyle}>
              <span style={labelStyle}>Provider</span>
              <select
                style={selectStyle}
                value={config.provider}
                onChange={e => {
                  const p = e.target.value;
                  const m = providers[p]?.[0] ?? '';
                  onChange({ ...config, provider: p, model: m });
                }}
              >
                {Object.keys(providers).map(p => (
                  <option key={p} value={p}>{p}</option>
                ))}
              </select>
            </div>
            <div style={fieldStyle}>
              <span style={labelStyle}>Model</span>
              <select
                style={selectStyle}
                value={config.model}
                onChange={e => set('model', e.target.value)}
              >
                {models.map(m => (
                  <option key={m} value={m}>{m}</option>
                ))}
              </select>
            </div>
            <div style={fieldStyle}>
              <span style={labelStyle}>Effort</span>
              <select
                style={selectStyle}
                value={config.effort}
                onChange={e => set('effort', e.target.value as DemoConfig['effort'])}
              >
                <option value="low">low</option>
                <option value="medium">medium</option>
                <option value="high">high</option>
              </select>
            </div>
          </div>

          {/* Gates */}
          <div style={{ ...sectionStyle, borderRight: 'none' }}>
            <div style={sectionLabel}>GATES</div>
            <div style={fieldStyle}>
              <span style={labelStyle}>Clippy</span>
              <input type="checkbox" style={checkStyle} checked={config.clippy_enabled}
                onChange={e => set('clippy_enabled', e.target.checked)} />
            </div>
            <div style={fieldStyle}>
              <span style={labelStyle}>Skip tests</span>
              <input type="checkbox" style={checkStyle} checked={config.skip_tests}
                onChange={e => set('skip_tests', e.target.checked)} />
            </div>
            <div style={fieldStyle}>
              <span style={labelStyle}>Max iters</span>
              <input type="number" style={inputStyle} value={config.max_iterations}
                min={1} max={10} onChange={e => set('max_iterations', parseInt(e.target.value) || 1)} />
            </div>
          </div>

          {/* Budget */}
          <div style={sectionStyle}>
            <div style={sectionLabel}>BUDGET</div>
            <div style={fieldStyle}>
              <span style={labelStyle}>Plan ($)</span>
              <input type="number" style={inputStyle} value={config.max_plan_usd}
                step={1} min={0.1} onChange={e => set('max_plan_usd', parseFloat(e.target.value) || 1)} />
            </div>
            <div style={fieldStyle}>
              <span style={labelStyle}>Turn ($)</span>
              <input type="number" style={inputStyle} value={config.max_turn_usd}
                step={0.5} min={0.1} onChange={e => set('max_turn_usd', parseFloat(e.target.value) || 0.5)} />
            </div>
          </div>

          {/* Pipeline */}
          <div style={{ ...sectionStyle, borderRight: 'none' }}>
            <div style={sectionLabel}>PIPELINE</div>
            <div style={fieldStyle}>
              <span style={labelStyle}>Mode</span>
              <select style={selectStyle} value={config.pipeline}
                onChange={e => set('pipeline', e.target.value as DemoConfig['pipeline'])}>
                {PIPELINES.map(p => <option key={p} value={p}>{p}</option>)}
              </select>
            </div>
            <div style={fieldStyle}>
              <span style={labelStyle}>Routing</span>
              <select style={selectStyle} value={config.routing_mode}
                onChange={e => set('routing_mode', e.target.value as DemoConfig['routing_mode'])}>
                <option value="auto_override">auto_override</option>
                <option value="auto">auto</option>
                <option value="manual">manual</option>
              </select>
            </div>
            <div style={fieldStyle}>
              <span style={labelStyle}>Agents</span>
              <input type="number" style={inputStyle} value={config.max_agents}
                min={1} max={16} onChange={e => set('max_agents', parseInt(e.target.value) || 1)} />
            </div>
            <div style={fieldStyle}>
              <span style={labelStyle}>Parallel</span>
              <input type="checkbox" style={checkStyle} checked={config.parallel_enabled}
                onChange={e => set('parallel_enabled', e.target.checked)} />
            </div>
          </div>

          {/* TOML overrides */}
          <div style={fullWidthSection}>
            <div style={sectionLabel}>TOML OVERRIDES</div>
            <textarea
              style={textareaStyle}
              value={config.toml_overrides}
              onChange={e => set('toml_overrides', e.target.value)}
              placeholder={'# Extra TOML merged into roko.toml\n# [learning]\n# replan_on_gate_failure = true'}
              spellCheck={false}
            />
          </div>
        </div>
      )}
    </div>
  );
}

/**
 * Generate TOML patch content from config.
 *
 * roko init already creates [agent] with default_backend and default_model,
 * so we DON'T emit [agent] here — the --model global flag handles model
 * selection at runtime. We only emit sections init doesn't create.
 */
export function configToTomlPatch(cfg: DemoConfig): string {
  const lines: string[] = [
    '',
    '# ── Demo config overrides ──',
    '[gates]',
    `clippy_enabled = ${cfg.clippy_enabled}`,
    `skip_tests = ${cfg.skip_tests}`,
    `max_iterations = ${cfg.max_iterations}`,
    '',
    '[budget]',
    `max_plan_usd = ${cfg.max_plan_usd}`,
    `max_turn_usd = ${cfg.max_turn_usd}`,
    '',
    '[routing]',
    `mode = "${cfg.routing_mode}"`,
    '',
    '[conductor]',
    `max_agents = ${cfg.max_agents}`,
    `parallel_enabled = ${cfg.parallel_enabled}`,
  ];

  if (cfg.toml_overrides.trim()) {
    lines.push('', '# User overrides', cfg.toml_overrides.trim());
  }

  return lines.join('\n');
}

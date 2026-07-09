import { useState, type CSSProperties } from 'react';

interface HealthBarProps {
  apiLatency?: number;
  sseState?: string;
  dataMode?: string;
  lastProbe?: string;
}

const barStyle: CSSProperties = {
  position: 'fixed',
  bottom: 0,
  left: 0,
  right: 0,
  height: 24,
  background: 'var(--bg-deeper)',
  borderTop: '1px solid var(--border-soft)',
  display: 'flex',
  alignItems: 'center',
  gap: 'var(--gap-lg)',
  padding: '0 var(--gap-md)',
  fontFamily: 'var(--mono)',
  fontSize: '10px',
  color: 'var(--text-dim)',
  zIndex: 9990,
  cursor: 'pointer',
  transition: `height var(--duration-normal) var(--ease-snappy)`,
};

const expandedStyle: CSSProperties = {
  ...barStyle,
  height: 120,
  flexDirection: 'column',
  alignItems: 'flex-start',
  justifyContent: 'flex-start',
  padding: 'var(--gap-sm) var(--gap-md)',
  gap: 'var(--gap-xs)',
};

const itemStyle: CSSProperties = {
  display: 'flex',
  gap: 'var(--gap-sm)',
};

const labelStyle: CSSProperties = {
  color: 'var(--text-ghost)',
};

export function HealthBar({ apiLatency, sseState, dataMode, lastProbe }: HealthBarProps) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div
      style={expanded ? expandedStyle : barStyle}
      onClick={() => setExpanded(!expanded)}
    >
      <span style={itemStyle}>
        <span style={labelStyle}>API</span>
        <span>{apiLatency ? `${apiLatency}ms` : '—'}</span>
      </span>
      <span style={itemStyle}>
        <span style={labelStyle}>SSE</span>
        <span>{sseState ?? 'idle'}</span>
      </span>
      <span style={itemStyle}>
        <span style={labelStyle}>DATA</span>
        <span>{dataMode ?? 'seed'}</span>
      </span>
      <span style={itemStyle}>
        <span style={labelStyle}>PROBE</span>
        <span>{lastProbe ?? 'never'}</span>
      </span>
      {expanded && (
        <div style={{ color: 'var(--text-ghost)', marginTop: 'var(--gap-sm)' }}>
          Click to collapse. Activated via ?debug=true or keyboard shortcut D.
        </div>
      )}
    </div>
  );
}

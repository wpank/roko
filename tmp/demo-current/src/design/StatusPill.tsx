import { type CSSProperties } from 'react';
import { Led } from './Led';

interface StatusPillProps {
  mode: 'live' | 'seed' | 'reconnecting' | 'offline';
  uptime?: string;
}

const pillStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '6px',
  fontFamily: 'var(--mono)',
  fontSize: '10px',
  fontWeight: 500,
  letterSpacing: '0.06em',
  textTransform: 'uppercase' as const,
  padding: '4px 10px',
  border: '1px solid var(--border)',
  cursor: 'default',
  position: 'relative',
};

const tooltipText: Record<StatusPillProps['mode'], string> = {
  live: 'Connected to roko-serve',
  seed: 'Start roko serve to see live data',
  reconnecting: 'Attempting to reconnect...',
  offline: 'Check that roko serve is running on port 6677',
};

const modeLabel: Record<StatusPillProps['mode'], string> = {
  live: 'LIVE',
  seed: 'SEED DATA',
  reconnecting: 'RECONNECTING',
  offline: 'OFFLINE',
};

const modeColor: Record<StatusPillProps['mode'], 'success' | 'bone' | 'warning' | 'rose'> = {
  live: 'success',
  seed: 'bone',
  reconnecting: 'warning',
  offline: 'rose',
};

export function StatusPill({ mode, uptime }: StatusPillProps) {
  const color = modeColor[mode];

  return (
    <div style={pillStyle} title={tooltipText[mode]}>
      <Led color={color} pulse={mode === 'reconnecting'} />
      <span style={{ color: `var(--${color === 'rose' ? 'text-dim' : color === 'success' ? 'success' : color === 'bone' ? 'bone' : 'warning'})` }}>
        {modeLabel[mode]}
      </span>
      {mode === 'live' && uptime && (
        <span style={{ color: 'var(--text-dim)' }}>{uptime}</span>
      )}
    </div>
  );
}

import { type CSSProperties } from 'react';

interface Gate {
  name: string;
  status: 'pass' | 'fail' | 'running' | 'pending';
}

interface GateBarProps {
  gates: Gate[];
}

const barStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  gap: 'var(--gap-lg)',
  fontFamily: 'var(--mono)',
  fontSize: '10px',
  fontWeight: 500,
  letterSpacing: '0.06em',
  textTransform: 'uppercase' as const,
};

const statusConfig: Record<Gate['status'], { icon: string; color: string; animation?: string }> = {
  pass: {
    icon: '\u2713',
    color: 'var(--success)',
    animation: 'gate-pass 200ms var(--ease-snappy)',
  },
  fail: {
    icon: '\u2715',
    color: 'var(--rose-bright)',
  },
  running: {
    icon: '\u25C9',
    color: 'var(--bone)',
  },
  pending: {
    icon: '\u25CB',
    color: 'var(--text-ghost)',
  },
};

export function GateBar({ gates }: GateBarProps) {
  return (
    <div style={barStyle}>
      {gates.map(gate => {
        const cfg = statusConfig[gate.status];
        const itemStyle: CSSProperties = {
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          color: cfg.color,
          transition: `color 120ms var(--ease-out)`,
        };
        const iconStyle: CSSProperties = {
          fontSize: '12px',
          ...(cfg.animation ? { animation: cfg.animation } : {}),
          ...(gate.status === 'running'
            ? { animation: 'pulse 2.2s ease-in-out infinite' }
            : {}),
        };

        return (
          <span key={gate.name} style={itemStyle}>
            <span style={iconStyle}>{cfg.icon}</span>
            <span>{gate.name}</span>
          </span>
        );
      })}
    </div>
  );
}

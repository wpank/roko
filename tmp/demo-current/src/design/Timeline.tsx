import { type CSSProperties } from 'react';

interface TimelineProps {
  phases: string[];
  current: number;
  failed?: number;
}

const containerStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  gap: 0,
  padding: 'var(--gap-md) 0',
};

export function Timeline({ phases, current, failed }: TimelineProps) {
  return (
    <div style={containerStyle}>
      {phases.map((phase, i) => {
        const isDone = i < current;
        const isCurrent = i === current;
        const isFailed = i === failed;
        const isPending = i > current;

        let dotColor: string;
        let dotBg: string;
        let lineColor: string;

        if (isFailed) {
          dotColor = 'var(--rose-bright)';
          dotBg = 'var(--rose-bright)';
          lineColor = 'var(--border-soft)';
        } else if (isDone) {
          dotColor = 'var(--success)';
          dotBg = 'var(--success)';
          lineColor = 'var(--success)';
        } else if (isCurrent) {
          dotColor = 'var(--rose-glow)';
          dotBg = 'transparent';
          lineColor = 'var(--rose-dim)';
        } else {
          dotColor = 'var(--text-ghost)';
          dotBg = 'transparent';
          lineColor = 'var(--border-soft)';
        }

        const dotStyle: CSSProperties = {
          width: 10,
          height: 10,
          borderRadius: '50%',
          backgroundColor: dotBg,
          border: `2px solid ${dotColor}`,
          flexShrink: 0,
          transition: 'background-color 200ms var(--ease-out), border-color 200ms var(--ease-out)',
          ...(isCurrent
            ? {
                boxShadow: `0 0 0 1px ${dotColor}66, 0 0 8px ${dotColor}40`,
                animation: 'pulse 2.2s ease-in-out infinite',
              }
            : {}),
          ...(isFailed
            ? { boxShadow: `0 0 8px var(--rose-bright)40` }
            : {}),
          ...(isDone
            ? { transform: 'scale(1)', animation: 'gate-pass 200ms var(--ease-snappy)' }
            : {}),
        };

        const labelStyle: CSSProperties = {
          fontFamily: 'var(--mono)',
          fontSize: '10px',
          fontWeight: 500,
          letterSpacing: '0.06em',
          textTransform: 'uppercase' as const,
          color: isDone ? 'var(--text-soft)' : isCurrent ? 'var(--rose-glow)' : 'var(--text-ghost)',
          marginTop: '8px',
          transition: 'color 200ms var(--ease-out)',
        };

        const lineStyle: CSSProperties = {
          width: 40,
          height: 1,
          backgroundColor: lineColor,
          flexShrink: 0,
          transition: 'background-color 300ms var(--ease-out)',
          ...(isDone && !isPending
            ? { animation: 'line-draw 300ms var(--ease-out)' }
            : {}),
        };

        return (
          <div key={phase} style={{ display: 'flex', alignItems: 'center' }}>
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center' }}>
              <div style={dotStyle} />
              <span style={labelStyle}>{phase}</span>
            </div>
            {i < phases.length - 1 && <div style={lineStyle} />}
          </div>
        );
      })}
    </div>
  );
}

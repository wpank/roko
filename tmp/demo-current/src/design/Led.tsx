import { type CSSProperties } from 'react';

const colorMap: Record<string, string> = {
  rose: 'var(--rose-glow)',
  bone: 'var(--bone-bright)',
  dream: 'var(--dream-bright)',
  success: 'var(--success)',
  warning: 'var(--warning)',
};

interface LedProps {
  color: 'rose' | 'bone' | 'dream' | 'success' | 'warning';
  pulse?: boolean;
}

export function Led({ color, pulse }: LedProps) {
  const c = colorMap[color];
  const style: CSSProperties = {
    display: 'inline-block',
    width: 5,
    height: 5,
    borderRadius: '50%',
    backgroundColor: c,
    boxShadow: `0 0 0 1px ${c}66, 0 0 8px ${c}40`,
    transition: `background-color 200ms var(--ease-out), box-shadow 200ms var(--ease-out)`,
    flexShrink: 0,
    ...(pulse
      ? { animation: 'pulse 2.2s ease-in-out infinite' }
      : {}),
  };

  return <span style={style} aria-hidden="true" />;
}

import { type CSSProperties } from 'react';

interface StagLabelProps {
  num?: string;
  label: string;
}

const containerStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 'var(--gap-md)',
  marginBottom: '80px',
  fontFamily: 'var(--mono)',
  fontSize: '11px',
  fontWeight: 500,
  letterSpacing: '0.32em',
  textTransform: 'uppercase' as const,
  color: 'var(--text-dim)',
};

const dashStyle: CSSProperties = {
  display: 'inline-block',
  width: 24,
  height: 1,
  background: 'var(--rose-dim)',
  flexShrink: 0,
};

const numStyle: CSSProperties = {
  color: 'var(--text-ghost)',
};

export function StagLabel({ num, label }: StagLabelProps) {
  return (
    <div style={containerStyle}>
      <span style={dashStyle} aria-hidden="true" />
      {num && <span style={numStyle}>{num}</span>}
      <span>{label}</span>
    </div>
  );
}

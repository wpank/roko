import { type CSSProperties } from 'react';

interface MosaicCellProps {
  label: string;
  value: string | number;
  sub?: string;
  color?: 'rose' | 'bone' | 'dream' | 'success';
}

const colorMap: Record<string, string> = {
  rose: 'var(--rose-glow)',
  bone: 'var(--bone-bright)',
  dream: 'var(--dream-bright)',
  success: 'var(--success)',
};

const cellStyle: CSSProperties = {
  padding: '30px 28px',
  background: 'var(--bg-glass)',
  boxShadow: 'inset 0 1px 0 rgba(255, 255, 255, 0.04)',
};

const labelStyle: CSSProperties = {
  fontFamily: 'var(--mono)',
  fontSize: '10px',
  fontWeight: 500,
  letterSpacing: '0.28em',
  textTransform: 'uppercase' as const,
  color: 'var(--text-dim)',
  marginBottom: 'var(--gap-sm)',
};

const subStyle: CSSProperties = {
  fontFamily: 'var(--display)',
  fontWeight: 300,
  fontSize: '14px',
  color: 'var(--text-soft)',
  marginTop: 'var(--gap-xs)',
};

export function MosaicCell({ label, value, sub, color = 'bone' }: MosaicCellProps) {
  const valueStyle: CSSProperties = {
    fontFamily: 'var(--display)',
    fontStyle: 'italic',
    fontWeight: 400,
    fontSize: '38px',
    letterSpacing: '-0.015em',
    color: colorMap[color],
    lineHeight: 1.1,
  };

  return (
    <div style={cellStyle}>
      <div style={labelStyle}>{label}</div>
      <div style={valueStyle}>{value}</div>
      {sub && <div style={subStyle}>{sub}</div>}
    </div>
  );
}

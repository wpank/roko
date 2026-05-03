import { type ReactNode, type CSSProperties } from 'react';
import { Led } from './Led';

interface PaneProps {
  label?: string;
  badge?: ReactNode;
  footer?: ReactNode;
  led?: 'rose' | 'bone' | 'dream' | 'success' | 'warning';
  flat?: boolean;
  children: ReactNode;
  style?: CSSProperties;
  className?: string;
}

const paneStyle: CSSProperties = {
  background: 'var(--bg-glass)',
  border: '1px solid var(--border)',
  borderLeft: '2px solid var(--rose-dim)',
  boxShadow: 'var(--shadow-sm), -2px 0 12px rgba(122, 80, 96, 0.08)',
  backdropFilter: 'blur(12px) saturate(180%)',
  WebkitBackdropFilter: 'blur(12px) saturate(180%)',
  animation: 'fadeUp 200ms var(--ease-expo) forwards',
  transition: 'border-color var(--duration-fast) var(--ease-out)',
  willChange: 'transform',
};

const headerStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 'var(--gap-sm)',
  padding: '12px 20px',
  borderBottom: '1px solid var(--border-soft)',
  fontFamily: 'var(--mono)',
  fontSize: '10.5px',
  fontWeight: 500,
  letterSpacing: '0.06em',
  textTransform: 'uppercase' as const,
  color: 'var(--text-dim)',
};

const badgeStyle: CSSProperties = {
  marginLeft: 'auto',
  color: 'var(--text-soft)',
};

const footerStyle: CSSProperties = {
  padding: '12px 20px',
  borderTop: '1px solid var(--border-soft)',
  fontFamily: 'var(--mono)',
  fontSize: '11px',
  color: 'var(--text-dim)',
};

export function Pane({ label, badge, footer, led, flat, children, style, className }: PaneProps) {
  return (
    <div
      className={className}
      style={{ ...paneStyle, ...style }}
      onMouseEnter={e => { (e.currentTarget as HTMLElement).style.borderColor = 'rgba(255,255,255,0.14)'; }}
      onMouseLeave={e => { (e.currentTarget as HTMLElement).style.borderColor = ''; (e.currentTarget as HTMLElement).style.borderLeftColor = 'var(--rose-dim)'; }}
    >
      {label && (
        <div style={headerStyle}>
          {led && <Led color={led} />}
          <span>{label}</span>
          {badge && <span style={badgeStyle}>{badge}</span>}
        </div>
      )}
      <div style={flat ? undefined : { padding: '20px' }}>
        {children}
      </div>
      {footer && <div style={footerStyle}>{footer}</div>}
    </div>
  );
}

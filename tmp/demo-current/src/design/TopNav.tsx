import { NavLink, useLocation } from 'react-router';
import { type CSSProperties, useState, useCallback } from 'react';
import { StatusPill } from './StatusPill';

interface TopNavProps {
  dataMode: 'live' | 'seed' | 'reconnecting' | 'offline';
  uptime?: string;
}

const navItems = [
  { path: '/app/orchestrate', label: 'ORCHESTRATE', subtitle: 'Watch it build' },
  { path: '/app/observe', label: 'OBSERVE', subtitle: 'See what\u2019s running' },
  { path: '/app/evaluate', label: 'EVALUATE', subtitle: 'The evidence' },
  { path: '/app/build', label: 'BUILD', subtitle: 'Try it yourself' },
];

const navStyle: CSSProperties = {
  position: 'fixed',
  top: 0,
  left: 0,
  right: 0,
  height: 56,
  display: 'flex',
  alignItems: 'center',
  padding: '0 var(--gap-lg)',
  background: 'rgba(6, 6, 8, 0.85)',
  backdropFilter: 'blur(16px) saturate(180%)',
  WebkitBackdropFilter: 'blur(16px) saturate(180%)',
  borderBottom: '1px solid var(--border-soft)',
  zIndex: 1000,
};

const brandStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 'var(--gap-sm)',
  fontFamily: 'var(--mono)',
  fontSize: '13px',
  fontWeight: 500,
  letterSpacing: '0.08em',
  color: 'var(--bone)',
  marginRight: 'var(--gap-xl)',
};

const linksStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 'var(--gap-lg)',
  flex: 1,
  justifyContent: 'center',
};

const linkBaseStyle: CSSProperties = {
  fontFamily: 'var(--mono)',
  fontSize: '11px',
  fontWeight: 500,
  letterSpacing: '0.06em',
  textTransform: 'uppercase' as const,
  color: 'var(--text-dim)',
  textDecoration: 'none',
  position: 'relative',
  padding: '4px 0 6px',
  transition: `color var(--duration-instant) var(--ease-out)`,
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
};

const activeBorderStyle: CSSProperties = {
  position: 'absolute',
  bottom: -1,
  left: 0,
  right: 0,
  height: 2,
  background: 'var(--rose-glow)',
  boxShadow: '0 2px 8px rgba(220, 165, 189, 0.3)',
};

const subtitleStyle: CSSProperties = {
  fontFamily: 'var(--mono)',
  fontSize: '10px',
  fontWeight: 400,
  letterSpacing: '0.04em',
  color: 'var(--text-ghost)',
  marginTop: 2,
  opacity: 0,
  transform: 'translateY(2px)',
  transition: `opacity 120ms var(--ease-out), transform 120ms var(--ease-snappy)`,
  pointerEvents: 'none',
};

const subtitleVisibleStyle: CSSProperties = {
  opacity: 1,
  transform: 'translateY(0)',
};

function NavItem({ path, label, subtitle }: typeof navItems[number]) {
  const [hovered, setHovered] = useState(false);
  const location = useLocation();
  const isActive = location.pathname.startsWith(path);

  const onEnter = useCallback(() => setHovered(true), []);
  const onLeave = useCallback(() => setHovered(false), []);

  return (
    <NavLink
      to={path}
      style={{
        ...linkBaseStyle,
        color: isActive ? 'var(--rose-glow)' : hovered ? 'var(--text-soft)' : 'var(--text-dim)',
      }}
      onMouseEnter={onEnter}
      onMouseLeave={onLeave}
    >
      <span>{label}</span>
      <span style={{ ...subtitleStyle, ...(hovered && !isActive ? subtitleVisibleStyle : {}) }}>
        {subtitle}
      </span>
      {isActive && <span style={activeBorderStyle} />}
    </NavLink>
  );
}

export function TopNav({ dataMode, uptime }: TopNavProps) {
  return (
    <nav style={navStyle}>
      <div style={brandStyle}>
        <span style={{ fontSize: '16px' }}>{'\u25C6'}</span>
        <span>ROKO</span>
      </div>
      <div style={linksStyle}>
        {navItems.map(item => (
          <NavItem key={item.path} {...item} />
        ))}
      </div>
      <StatusPill mode={dataMode} uptime={uptime} />
    </nav>
  );
}

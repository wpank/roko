import { useCallback, useEffect, useRef, useState } from 'react';
import { NavLink, Link, useLocation } from 'react-router';
import { useDataApi } from '../hooks/useDataApi';
import { fmtUptime } from '../lib/format';
import { useServerHealthController } from '../data/selectors';
import Tooltip from './Tooltip';
import { PulseIcon, SpinnerIcon } from './icons/AnimatedIcons';
import './TopNav.css';

interface NavItem {
  to: string;
  label: string;
  full: string;
  /** Active only on this exact path. */
  end?: boolean;
}

/** The showcase (S10 §4.2): measured claims first, the legacy pages under LAB. */
const NAV_LINKS: NavItem[] = [
  { to: '/', label: 'OVERVIEW', full: 'Measured claims: overview', end: true },
  { to: '/p1/head-to-head', label: 'HEAD-TO-HEAD', full: 'Cheap + harness vs frontier-direct' },
  { to: '/p2/audits', label: 'AUDITS', full: 'M4 audit lottery' },
  { to: '/replays', label: 'REPLAYS', full: 'Recorded bundles' },
  { to: '/lab', label: 'LAB', full: 'Legacy pages (local only)' },
];

/** The legacy pages, shown in a second row while a /lab page is open (D23). */
const LAB_LINKS: NavItem[] = [
  { to: '/lab/demo', label: 'DEMO', full: 'Interactive Demo Scenarios' },
  { to: '/lab/dashboard', label: 'DASH', full: 'System Dashboard' },
  { to: '/lab/bench', label: 'BENCH', full: 'Benchmark Lab' },
  { to: '/lab/explorer', label: 'EXPLORE', full: 'Crate & Route Explorer' },
  { to: '/lab/builder', label: 'BUILD', full: 'Pipeline Builder' },
  { to: '/lab/terminal', label: 'TERM', full: 'Terminal Sessions' },
  { to: '/lab/settings', label: 'CONFIG', full: 'Configuration' },
];

interface HealthResponse {
  status?: string;
  uptime_secs?: number;
}

interface IndicatorStyle {
  left: number;
  width: number;
}

export default function TopNav() {
  const { get } = useDataApi();
  const { status: serverHealth } = useServerHealthController();
  const [uptime, setUptime] = useState<number | null>(null);
  const [scrolled, setScrolled] = useState(false);
  const [indicator, setIndicator] = useState<IndicatorStyle | null>(null);
  const linksRef = useRef<HTMLDivElement>(null);
  const location = useLocation();
  // The server status is a lab concern: the showcase shows only REPLAY and LIVE badges.
  const inLab = location.pathname === '/lab' || location.pathname.startsWith('/lab/');

  useEffect(() => {
    if (!inLab) return;
    let cancelled = false;
    const poll = async () => {
      try {
        const data = await get<HealthResponse>('/api/health');
        if (!cancelled) {
          if (data.uptime_secs != null) setUptime(data.uptime_secs);
        }
      } catch {
        if (!cancelled) setUptime(null);
      }
    };
    poll();
    const id = setInterval(poll, 5_000);
    return () => { cancelled = true; clearInterval(id); };
  }, [get, inLab]);

  // Scroll shadow detection
  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 4);
    window.addEventListener('scroll', onScroll, { passive: true });
    onScroll();
    return () => window.removeEventListener('scroll', onScroll);
  }, []);

  // Morphing indicator: track active tab rect
  const updateIndicator = useCallback(() => {
    if (!linksRef.current) return;
    const active = linksRef.current.querySelector<HTMLElement>('.nav-link.active');
    if (active) {
      const containerRect = linksRef.current.getBoundingClientRect();
      const activeRect = active.getBoundingClientRect();
      setIndicator({
        left: activeRect.left - containerRect.left,
        width: activeRect.width,
      });
    } else {
      setIndicator(null);
    }
  }, []);

  useEffect(() => {
    updateIndicator();
  }, [location.pathname, updateIndicator]);

  // Re-measure on resize
  useEffect(() => {
    window.addEventListener('resize', updateIndicator);
    return () => window.removeEventListener('resize', updateIndicator);
  }, [updateIndicator]);

  const isLive = serverHealth === 'connected' && uptime != null;
  const isConnecting = serverHealth === 'checking';

  return (
    <>
      <nav className={`topnav${scrolled ? ' scrolled' : ''}`} role="navigation" aria-label="Main navigation">
        <Link to="/" className="brand" style={{ textDecoration: 'none' }}>
          <span className="mark" aria-hidden="true" />
          <span className="brand-text">{'\u2308'} NUNCHI {'\u230B'}</span>
        </Link>

        <span className="nav-sep" aria-hidden="true">{'\u2502'}</span>

        <div className="links" ref={linksRef}>
          {/* Morphing indicator pill */}
          {indicator && (
            <span
              className="nav-indicator"
              style={{
                transform: `translateX(${indicator.left}px)`,
                width: `${indicator.width}px`,
              }}
            />
          )}

          {NAV_LINKS.map((l, i) => (
            <Tooltip content={l.full} placement="bottom" key={l.to}>
              <NavLink
                to={l.to}
                end={l.end}
                className={({ isActive }) => `nav-link${isActive ? ' active' : ''}`}
                style={{ animationDelay: `${i * 40}ms` }}
              >
                {({ isActive }) => (
                  <span className="nav-link-inner">
                    {isActive ? `\u25B8 ${l.label} \u25C2` : l.label}
                  </span>
                )}
              </NavLink>
            </Tooltip>
          ))}
        </div>

        <div className="right">
          {inLab && (
            <span
              className={`status-pill ${isLive ? 'live' : isConnecting ? 'connecting' : 'demo'}`}
              role="status"
              aria-live="polite"
              aria-label={isLive ? `Server live, uptime ${fmtUptime(uptime)}` : isConnecting ? 'Connecting to server' : 'Using seed data'}
            >
              {isLive
                ? <PulseIcon size={8} color="var(--success)" />
                : isConnecting
                  ? <SpinnerIcon size={8} />
                  : <span className="status-dot" />}
              {isLive ? `LIVE ${fmtUptime(uptime)}` : isConnecting ? 'SYNC' : 'SEED'}
            </span>
          )}
        </div>
      </nav>
      {inLab && (
        <nav className="labnav" aria-label="Lab navigation">
          <span className="labnav__label">LAB</span>
          {LAB_LINKS.map((l) => (
            <Tooltip content={l.full} placement="bottom" key={l.to}>
              <NavLink
                to={l.to}
                className={({ isActive }) => `nav-link${isActive ? ' active' : ''}`}
              >
                {l.label}
              </NavLink>
            </Tooltip>
          ))}
        </nav>
      )}
    </>
  );
}

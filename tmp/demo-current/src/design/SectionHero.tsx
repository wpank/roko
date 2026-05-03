import { type CSSProperties } from 'react';
import { useNavigate } from 'react-router';

interface SectionHeroProps {
  line: string;
  cue?: { label: string; section: string };
}

const lineStyle: CSSProperties = {
  fontFamily: 'var(--display)',
  fontStyle: 'italic',
  fontWeight: 300,
  fontSize: '20px',
  letterSpacing: '-0.008em',
  lineHeight: 1.4,
  color: 'var(--text-soft)',
  textAlign: 'center',
  padding: '16px 0 8px',
  maxWidth: 640,
  margin: '0 auto',
};

const cueContainerStyle: CSSProperties = {
  display: 'flex',
  justifyContent: 'flex-end',
  padding: '0 var(--gap-lg)',
};

const cueStyle: CSSProperties = {
  fontFamily: 'var(--mono)',
  fontSize: '11px',
  color: 'var(--text-dim)',
  cursor: 'pointer',
  transition: `color var(--duration-fast) var(--ease-out)`,
  border: 'none',
  background: 'none',
  padding: 'var(--gap-sm) 0',
};

export function SectionHero({ line, cue }: SectionHeroProps) {
  const navigate = useNavigate();

  return (
    <div>
      <div style={lineStyle}>{line}</div>
      {cue && (
        <div style={cueContainerStyle}>
          <button
            style={cueStyle}
            onClick={() => navigate(`/app/${cue.section.toLowerCase()}`)}
            onMouseEnter={e => { (e.currentTarget as HTMLElement).style.color = 'var(--text-soft)'; }}
            onMouseLeave={e => { (e.currentTarget as HTMLElement).style.color = 'var(--text-dim)'; }}
          >
            {cue.label} → {cue.section.toUpperCase()}
          </button>
        </div>
      )}
    </div>
  );
}

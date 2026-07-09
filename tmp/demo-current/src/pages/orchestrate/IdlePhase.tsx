import { type CSSProperties } from 'react';
import type { Scenario } from '../../lib/types';
import { Timeline } from '../../design/Timeline';
import { ScenarioCard } from './ScenarioCard';
import { useConfig } from '../../lib/config-context';

interface IdlePhaseProps {
  scenarios: Scenario[];
  selectedId: string;
  onSelect: (id: string) => void;
  onStart: () => void;
}

const containerStyle: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  gap: 'var(--gap-lg)',
  padding: 'var(--gap-lg) var(--gap-lg) var(--gap-md)',
  maxWidth: 960,
  margin: '0 auto',
  animation: 'fadeUp 200ms var(--ease-expo) forwards',
};

const cardsStyle: CSSProperties = {
  display: 'flex',
  gap: 'var(--gap-md)',
  width: '100%',
};

const descriptionStyle: CSSProperties = {
  fontFamily: 'var(--display)',
  fontStyle: 'italic',
  fontWeight: 300,
  fontSize: '26px',
  letterSpacing: '-0.012em',
  lineHeight: 1.25,
  color: 'var(--text-strong)',
  textAlign: 'center',
  maxWidth: 640,
  animation: 'fadeIn 150ms ease-out',
};

const ctaStyle: CSSProperties = {
  fontFamily: 'var(--mono)',
  fontSize: '13px',
  fontWeight: 500,
  letterSpacing: '0.08em',
  textTransform: 'uppercase' as const,
  color: 'var(--text-strong)',
  padding: '14px 40px',
  border: '1px solid var(--rose-dim)',
  background: 'var(--bg-glass)',
  cursor: 'pointer',
  boxShadow: 'var(--shadow-sm)',
  transition: `border-color var(--duration-fast) var(--ease-out), box-shadow var(--duration-fast) var(--ease-out), transform var(--duration-fast) var(--ease-snappy)`,
  willChange: 'transform',
};

const phases = ['idea', 'prd', 'plan', 'run'];

export function IdlePhase({ scenarios, selectedId, onSelect, onStart }: IdlePhaseProps) {
  const { config } = useConfig();
  const selected = scenarios.find(s => s.id === selectedId)!

  return (
    <div style={containerStyle}>
      <div style={cardsStyle}>
        {scenarios.map((s, i) => (
          <div
            key={s.id}
            style={{
              flex: '1 1 0',
              animation: `fadeUp 200ms var(--ease-expo) forwards`,
              animationDelay: `${i * 40}ms`,
              opacity: 0,
            }}
          >
            <ScenarioCard
              scenario={s}
              selected={s.id === selectedId}
              onSelect={onSelect}
            />
          </div>
        ))}
      </div>

      <div style={descriptionStyle} key={selectedId}>
        {selected.description}
      </div>

      <button
        style={ctaStyle}
        onClick={onStart}
        onMouseEnter={e => {
          const el = e.currentTarget as HTMLElement;
          el.style.borderColor = 'var(--rose-glow)';
          el.style.boxShadow = 'var(--shadow-glow-rose)';
        }}
        onMouseLeave={e => {
          const el = e.currentTarget as HTMLElement;
          el.style.borderColor = 'var(--rose-dim)';
          el.style.boxShadow = 'var(--shadow-sm)';
        }}
        onMouseDown={e => {
          (e.currentTarget as HTMLElement).style.transform = 'scale(0.97) translateY(1px)';
          (e.currentTarget as HTMLElement).style.transition = 'transform 50ms var(--ease-snappy)';
        }}
        onMouseUp={e => {
          (e.currentTarget as HTMLElement).style.transform = '';
          (e.currentTarget as HTMLElement).style.transition = 'transform 120ms var(--ease-snappy)';
        }}
      >
        {'\u25B6'}  START LIVE RUN
      </button>

      <Timeline phases={phases} current={-1} />

      <div style={{
        fontFamily: 'var(--mono)',
        fontSize: '11px',
        color: 'var(--text-ghost)',
        padding: '8px 16px',
        background: 'var(--bg-deeper)',
        border: '1px solid var(--border-soft)',
        width: '100%',
        textAlign: 'center',
      }}>
        <span style={{ color: 'var(--text-dim)' }}>{config.provider}</span>
        <span style={{ color: 'var(--border-strong)', margin: '0 6px' }}>/</span>
        <span style={{ color: 'var(--bone-dim)' }}>{config.model}</span>
        <span style={{ color: 'var(--border-strong)', margin: '0 6px' }}>|</span>
        <span style={{ color: 'var(--text-dim)' }}>effort: {config.effort}</span>
        <span style={{ color: 'var(--border-strong)', margin: '0 6px' }}>|</span>
        <span style={{ color: 'var(--text-dim)' }}>pipeline: {config.pipeline}</span>
      </div>
    </div>
  );
}

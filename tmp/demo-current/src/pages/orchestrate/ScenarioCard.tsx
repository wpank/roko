import { type CSSProperties, useCallback } from 'react';
import type { Scenario } from '../../lib/types';

interface ScenarioCardProps {
  scenario: Scenario;
  selected: boolean;
  onSelect: (id: string) => void;
}

const complexityColors: Record<string, string> = {
  simple: 'var(--success)',
  medium: 'var(--bone)',
  complex: 'var(--rose-bright)',
};

export function ScenarioCard({ scenario, selected, onSelect }: ScenarioCardProps) {
  const onClick = useCallback(() => onSelect(scenario.id), [scenario.id, onSelect]);

  const cardStyle: CSSProperties = {
    width: '100%',
    padding: '24px 20px',
    background: selected ? 'rgba(58, 32, 48, 0.35)' : 'rgba(16, 14, 22, 1)',
    border: selected ? '1px solid var(--rose-dim)' : '1px solid rgba(255, 255, 255, 0.12)',
    borderLeft: selected ? '2px solid var(--rose-glow)' : '2px solid rgba(255, 255, 255, 0.08)',
    boxShadow: selected
      ? '0 0 0 1px rgba(220, 165, 189, 0.2), 0 0 20px rgba(170, 112, 136, 0.1), inset 0 1px 0 rgba(255,255,255,0.06)'
      : '0 1px 3px rgba(0,0,0,0.4), inset 0 1px 0 rgba(255,255,255,0.06)',
    cursor: 'pointer',
    transition: `transform var(--duration-fast) var(--ease-snappy), box-shadow var(--duration-fast) var(--ease-snappy), border-color var(--duration-fast) var(--ease-out), background-color var(--duration-fast) var(--ease-out)`,
    willChange: 'transform',
    display: 'flex',
    flexDirection: 'column',
    gap: '8px',
    textAlign: 'left',
  };

  return (
    <button
      style={cardStyle}
      onClick={onClick}
      onMouseEnter={e => {
        const el = e.currentTarget as HTMLElement;
        if (!selected) {
          el.style.transform = 'translateY(-2px)';
          el.style.borderColor = 'rgba(255, 255, 255, 0.14)';
        }
      }}
      onMouseLeave={e => {
        const el = e.currentTarget as HTMLElement;
        el.style.transform = '';
        el.style.borderColor = '';
      }}
      onMouseDown={e => {
        (e.currentTarget as HTMLElement).style.transform = 'scale(0.97) translateY(1px)';
        (e.currentTarget as HTMLElement).style.transition = 'transform 50ms var(--ease-snappy)';
      }}
      onMouseUp={e => {
        (e.currentTarget as HTMLElement).style.transform = '';
        (e.currentTarget as HTMLElement).style.transition = `transform 120ms var(--ease-snappy), box-shadow var(--duration-fast) var(--ease-snappy), border-color var(--duration-fast) var(--ease-out), background-color var(--duration-fast) var(--ease-out)`;
      }}
      aria-pressed={selected}
    >
      <span style={{
        fontFamily: 'var(--mono)',
        fontSize: '11px',
        fontWeight: 500,
        letterSpacing: '0.08em',
        textTransform: 'uppercase' as const,
        color: complexityColors[scenario.complexity],
      }}>
        {scenario.label}
      </span>
      <span style={{
        fontFamily: 'var(--display)',
        fontStyle: 'italic',
        fontWeight: 400,
        fontSize: '15px',
        lineHeight: 1.4,
        color: 'var(--text-strong)',
      }}>
        {scenario.prompt}
      </span>
      <span style={{
        fontFamily: 'var(--mono)',
        fontSize: '10px',
        color: 'var(--text-dim)',
        marginTop: 'auto',
      }}>
        {scenario.detail}
      </span>
    </button>
  );
}

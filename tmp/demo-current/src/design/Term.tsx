import { useState, useRef, useCallback, type CSSProperties } from 'react';

interface TermProps {
  label: string;
  tooltip: string;
}

const shownTerms = new Set<string>();

const labelBaseStyle: CSSProperties = {
  position: 'relative',
  cursor: 'help',
  color: 'var(--text-soft)',
  transition: 'border-bottom-color var(--duration-fast) var(--ease-out)',
};

const tooltipStyle: CSSProperties = {
  position: 'absolute',
  bottom: 'calc(100% + 8px)',
  left: '50%',
  transform: 'translateX(-50%) translateY(4px) scale(0.97)',
  maxWidth: 280,
  padding: '10px 14px',
  background: 'var(--bg-raised)',
  border: '1px solid var(--border)',
  boxShadow: 'var(--shadow-md)',
  fontFamily: 'var(--mono)',
  fontSize: '11px',
  lineHeight: 1.5,
  color: 'var(--bone)',
  whiteSpace: 'normal' as const,
  zIndex: 100,
  opacity: 0,
  pointerEvents: 'none' as const,
  transition: `opacity 120ms var(--ease-out), transform 120ms var(--ease-snappy)`,
};

const tooltipVisibleStyle: CSSProperties = {
  opacity: 1,
  transform: 'translateX(-50%) translateY(0) scale(1)',
};

export function Term({ label, tooltip }: TermProps) {
  const [visible, setVisible] = useState(false);
  const [dismissed, setDismissed] = useState(() => shownTerms.has(label));
  const timerRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  const show = useCallback(() => {
    clearTimeout(timerRef.current);
    setVisible(true);
    if (!shownTerms.has(label)) {
      shownTerms.add(label);
      setTimeout(() => setDismissed(true), 2000);
    }
  }, [label]);

  const hide = useCallback(() => {
    timerRef.current = setTimeout(() => setVisible(false), 100);
  }, []);

  const underline = dismissed ? 'none' : '1px dotted var(--text-dim)';

  return (
    <span
      style={{
        ...labelBaseStyle,
        borderBottom: underline,
        display: 'inline-block',
      }}
      onMouseEnter={show}
      onMouseLeave={hide}
      onFocus={show}
      onBlur={hide}
      tabIndex={0}
      role="term"
      aria-describedby={visible ? `term-${label}` : undefined}
    >
      {label}
      <span
        id={`term-${label}`}
        role="tooltip"
        style={{
          ...tooltipStyle,
          ...(visible ? tooltipVisibleStyle : {}),
        }}
      >
        {tooltip}
      </span>
    </span>
  );
}

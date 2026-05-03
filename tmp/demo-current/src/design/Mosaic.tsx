import { type ReactNode, type CSSProperties, Children } from 'react';

interface MosaicProps {
  columns?: 2 | 3 | 4 | 5 | 6;
  children: ReactNode;
}

export function Mosaic({ columns = 3, children }: MosaicProps) {
  const gridStyle: CSSProperties = {
    display: 'grid',
    gridTemplateColumns: `repeat(${columns}, 1fr)`,
    gap: '1px',
    background: 'var(--border)',
    border: '1px solid var(--border)',
    boxShadow: 'var(--shadow-sm)',
  };

  return (
    <div style={gridStyle}>
      {Children.map(children, (child, i) => (
        <div
          key={i}
          style={{
            animation: `fadeUp 200ms var(--ease-expo) forwards`,
            animationDelay: `${i * 40}ms`,
            opacity: 0,
          }}
        >
          {child}
        </div>
      ))}
    </div>
  );
}

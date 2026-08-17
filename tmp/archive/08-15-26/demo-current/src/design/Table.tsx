import { type CSSProperties, type ReactNode } from 'react';
import { EmptyState } from './EmptyState';

export interface Column {
  key: string;
  label: string;
  width?: string;
  align?: 'left' | 'center' | 'right';
}

export interface Row {
  id: string;
  [key: string]: ReactNode;
}

interface TableProps {
  columns: Column[];
  rows: Row[];
  onRowClick?: (row: Row) => void;
  dense?: boolean;
  emptyMessage?: string;
  emptyAction?: string;
}

const tableStyle: CSSProperties = {
  width: '100%',
  borderCollapse: 'collapse',
  fontFamily: 'var(--mono)',
  fontSize: '13px',
};

const thStyle: CSSProperties = {
  fontFamily: 'var(--mono)',
  fontSize: '10px',
  fontWeight: 500,
  letterSpacing: '0.06em',
  textTransform: 'uppercase' as const,
  color: 'var(--text-dim)',
  textAlign: 'left',
  padding: '0 12px 12px',
  borderBottom: 'none',
};

export function Table({ columns, rows, onRowClick, dense, emptyMessage, emptyAction }: TableProps) {
  const pad = dense ? '10px 12px' : '14px 12px';

  if (rows.length === 0) {
    return (
      <EmptyState
        message={emptyMessage ?? 'No data available'}
        action={emptyAction}
      />
    );
  }

  return (
    <table style={tableStyle}>
      <thead>
        <tr>
          {columns.map(col => (
            <th
              key={col.key}
              style={{
                ...thStyle,
                width: col.width,
                textAlign: col.align ?? 'left',
              }}
            >
              {col.label}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {rows.map((row, i) => {
          const rowStyle: CSSProperties = {
            borderBottom: '1px solid var(--border-soft)',
            cursor: onRowClick ? 'pointer' : undefined,
            transition: `background-color var(--duration-instant) var(--ease-out), transform var(--duration-instant) var(--ease-out)`,
            animation: `fadeUp 200ms var(--ease-expo) forwards`,
            animationDelay: `${i * 40}ms`,
            opacity: 0,
            willChange: 'transform',
          };

          return (
            <tr
              key={row.id}
              style={rowStyle}
              onClick={() => onRowClick?.(row)}
              onMouseEnter={e => {
                (e.currentTarget as HTMLElement).style.backgroundColor = 'var(--bg-glass-hover)';
                if (onRowClick) (e.currentTarget as HTMLElement).style.transform = 'translateY(-1px)';
              }}
              onMouseLeave={e => {
                (e.currentTarget as HTMLElement).style.backgroundColor = '';
                (e.currentTarget as HTMLElement).style.transform = '';
              }}
            >
              {columns.map((col, ci) => {
                const isFirst = ci === 0;
                const cellStyle: CSSProperties = {
                  padding: pad,
                  textAlign: col.align ?? 'left',
                  color: isFirst ? 'var(--text-strong)' : 'var(--text-primary)',
                  ...(isFirst
                    ? {
                        fontFamily: 'var(--display)',
                        fontStyle: 'italic',
                        fontSize: '16px',
                      }
                    : {}),
                };
                return (
                  <td key={col.key} style={cellStyle}>
                    {row[col.key]}
                  </td>
                );
              })}
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

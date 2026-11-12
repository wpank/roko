import React from 'react';

type FoldProps = { summary: React.ReactNode; open?: boolean; children: React.ReactNode } & Omit<
  React.HTMLAttributes<HTMLDetailsElement>,
  'children'
>;

/** Folded detail: `▸ details`, turning to `▾` when open. Every fold starts closed unless told otherwise. */
export function Fold({ summary, open, children, ...rest }: FoldProps) {
  return (
    <details className="rk-fold" open={open} {...rest}>
      <summary>{summary}</summary>
      <div className="rk-fold-body">{children}</div>
    </details>
  );
}

import React from 'react';

/** Folded detail: `▸ details`, turning to `▾` when open. Every fold starts closed unless told otherwise. */
export function Fold({
  summary,
  open,
  children,
  ...rest
}: { summary: string; open?: boolean; children: React.ReactNode } & React.HTMLAttributes<HTMLDetailsElement>) {
  return (
    <details className="rk-fold" open={open} {...rest}>
      <summary>{summary}</summary>
      <div className="rk-fold-body">{children}</div>
    </details>
  );
}

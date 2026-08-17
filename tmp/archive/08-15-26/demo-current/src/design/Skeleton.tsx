import { type CSSProperties } from 'react';

interface SkeletonProps {
  height?: number;
  width?: string;
  variant?: 'text' | 'cell' | 'pane';
}

const baseStyle: CSSProperties = {
  background: `linear-gradient(
    90deg,
    rgba(255, 255, 255, 0.03) 0%,
    rgba(255, 255, 255, 0.07) 40%,
    rgba(255, 255, 255, 0.03) 80%
  )`,
  backgroundSize: '200% 100%',
  animation: 'shimmer 1.8s ease-in-out infinite',
  borderRadius: 2,
};

const variantDefaults: Record<string, CSSProperties> = {
  text: { height: 14, width: '100%' },
  cell: { height: 100, padding: '30px 28px' },
  pane: { height: 200, width: '100%' },
};

export function Skeleton({ height, width, variant = 'text' }: SkeletonProps) {
  const defaults = variantDefaults[variant];
  const style: CSSProperties = {
    ...baseStyle,
    height: height ?? defaults.height,
    width: width ?? defaults.width,
    ...(variant === 'cell' ? { padding: defaults.padding } : {}),
  };

  return <div style={style} aria-hidden="true" />;
}

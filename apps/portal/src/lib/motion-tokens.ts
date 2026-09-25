/**
 * Motion tokens for Framer Motion animations.
 *
 * Use these constants instead of inline values to keep motion behaviour
 * consistent across the portal.  All easing curves are expressed as cubic
 * bezier arrays compatible with Framer Motion's `ease` prop.
 */

/** Cubic bezier easing curves. */
export const easing = {
  /** Exponential deceleration — snappy panel slides, drawers, overlays. */
  outExpo: [0.16, 1, 0.3, 1] as const,
  /** Symmetrical cubic ease-in-out — value transitions, chart updates. */
  inOutCubic: [0.4, 0, 0.2, 1] as const,
  /** Cubic deceleration — hover reveals, micro-interactions. */
  outCubic: [0.33, 1, 0.68, 1] as const,
} as const;

/** Animation durations in seconds. */
export const duration = {
  /** 80 ms — hover states, micro-interactions. */
  fast: 0.08,
  /** 200 ms — panel transitions, value updates. */
  normal: 0.2,
  /** 350 ms — drawer slides, page transitions. */
  slow: 0.35,
  /** 2000 ms — status LED pulse cycle. */
  pulse: 2,
} as const;

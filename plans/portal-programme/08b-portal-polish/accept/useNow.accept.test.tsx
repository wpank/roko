// @vitest-environment jsdom
/**
 * Acceptance: useNow(active) — a wall clock that ticks once a second while
 * something runs, so elapsed times move between events, and costs nothing
 * while idle. Copied verbatim from plans/portal-programme/08b-portal-polish/accept/.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, renderHook } from '@testing-library/react';
import { useNow } from '@/lib/useNow';

const T0 = 1_700_000_000_000;

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(T0);
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe('useNow', () => {
  it('returns the current time on mount', () => {
    const { result } = renderHook(() => useNow(false));
    expect(result.current).toBe(T0);
  });

  it('ticks once a second while active', () => {
    const { result } = renderHook(() => useNow(true));
    act(() => {
      vi.advanceTimersByTime(3_000);
    });
    expect(result.current).toBe(T0 + 3_000);
  });

  it('holds still and keeps no timer while inactive', () => {
    const { result } = renderHook(() => useNow(false));
    act(() => {
      vi.advanceTimersByTime(5_000);
    });
    expect(result.current).toBe(T0);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('catches up and starts ticking when it becomes active', () => {
    const { result, rerender } = renderHook(({ active }) => useNow(active), {
      initialProps: { active: false },
    });
    act(() => {
      vi.advanceTimersByTime(4_000);
    });
    rerender({ active: true });
    expect(result.current).toBe(T0 + 4_000);
    act(() => {
      vi.advanceTimersByTime(1_000);
    });
    expect(result.current).toBe(T0 + 5_000);
  });

  it('stops ticking when it becomes inactive', () => {
    const { result, rerender } = renderHook(({ active }) => useNow(active), {
      initialProps: { active: true },
    });
    act(() => {
      vi.advanceTimersByTime(2_000);
    });
    rerender({ active: false });
    expect(vi.getTimerCount()).toBe(0);
    act(() => {
      vi.advanceTimersByTime(3_000);
    });
    expect(result.current).toBe(T0 + 2_000);
  });

  it('clears its timer on unmount', () => {
    const { unmount } = renderHook(() => useNow(true));
    expect(vi.getTimerCount()).toBe(1);
    unmount();
    expect(vi.getTimerCount()).toBe(0);
  });

  it('honours a custom interval', () => {
    const { result } = renderHook(() => useNow(true, 250));
    act(() => {
      vi.advanceTimersByTime(250);
    });
    expect(result.current).toBe(T0 + 250);
  });
});

/**
 * roko's number formats (the design system's §6.3), the same on every surface.
 */
import { describe, expect, it } from 'vitest';
import { clock, clockLabel, count, duration, money } from '@/lib/words';

describe('duration', () => {
  it('shows tenths under 10 s, whole seconds under a minute', () => {
    expect(duration(5_300)).toBe('5.3s');
    expect(duration(7_190)).toBe('7.1s');
    expect(duration(42_000)).toBe('42s');
  });

  it('pads seconds under an hour, minutes past one', () => {
    expect(duration(245_000)).toBe('4m 05s');
    expect(duration(72 * 60_000)).toBe('1h 12m');
  });
});

describe('clock', () => {
  it('reads like a clock on the glass', () => {
    expect(clock(42_000)).toBe('0:42');
    expect(clock(245_000)).toBe('4:05');
    expect(clock(4_350_000)).toBe('1:12:30');
  });

  it('says its value in words for a screen reader', () => {
    expect(clockLabel(245_000)).toBe('4 minutes 5 seconds');
    expect(clockLabel(1_000)).toBe('1 second');
    expect(clockLabel(0)).toBe('0 seconds');
  });
});

describe('money', () => {
  it('always has two decimals', () => {
    expect(money(0.4363)).toBe('$0.44');
    expect(money(12.4)).toBe('$12.40');
  });

  it('says less than a cent, and nothing as $0.00', () => {
    expect(money(0.0012)).toBe('<$0.01');
    expect(money(0)).toBe('$0.00');
  });
});

describe('count', () => {
  it('puts the noun in the singular for one', () => {
    expect(count(1, 'task')).toBe('1 task');
    expect(count(3, 'task')).toBe('3 tasks');
  });
});

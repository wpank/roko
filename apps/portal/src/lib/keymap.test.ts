import { describe, it, expect } from 'vitest';
import { resolveKey, isEditableTarget } from './keymap';
import type { KeyInput } from './keymap';

// Convenience builder — editable defaults to false, no modifiers.
function k(key: string, overrides: Partial<KeyInput> = {}): KeyInput {
  return { key, metaKey: false, ctrlKey: false, altKey: false, editable: false, ...overrides };
}

// ── Bindings ───────────────────────────────────────────────────────────────────

describe('resolveKey — bindings', () => {
  it('ArrowUp → prev', () => {
    expect(resolveKey(k('ArrowUp'))).toBe('prev');
  });

  it('ArrowDown → next', () => {
    expect(resolveKey(k('ArrowDown'))).toBe('next');
  });

  it('r → run', () => {
    expect(resolveKey(k('r'))).toBe('run');
  });

  it('n → new', () => {
    expect(resolveKey(k('n'))).toBe('new');
  });

  it('/ → filter', () => {
    expect(resolveKey(k('/'))).toBe('filter');
  });

  it('o → stream', () => {
    expect(resolveKey(k('o'))).toBe('stream');
  });

  it('Escape → escape', () => {
    expect(resolveKey(k('Escape'))).toBe('escape');
  });
});

// ── Modifiers ignored ──────────────────────────────────────────────────────────

describe('resolveKey — modifiers', () => {
  it('metaKey + r → null (⌘R reload stays with browser)', () => {
    expect(resolveKey(k('r', { metaKey: true }))).toBeNull();
  });

  it('ctrlKey + ArrowUp → null', () => {
    expect(resolveKey(k('ArrowUp', { ctrlKey: true }))).toBeNull();
  });

  it('altKey + n → null', () => {
    expect(resolveKey(k('n', { altKey: true }))).toBeNull();
  });

  it('metaKey + Escape → null (modifier always wins)', () => {
    expect(resolveKey(k('Escape', { metaKey: true }))).toBeNull();
  });
});

// ── Editable target ────────────────────────────────────────────────────────────

describe('resolveKey — editable target', () => {
  it('r inside editable → null', () => {
    expect(resolveKey(k('r', { editable: true }))).toBeNull();
  });

  it('ArrowUp inside editable → null', () => {
    expect(resolveKey(k('ArrowUp', { editable: true }))).toBeNull();
  });

  it('Escape inside editable → escape (allowed)', () => {
    expect(resolveKey(k('Escape', { editable: true }))).toBe('escape');
  });
});

// ── Tab and unbound ────────────────────────────────────────────────────────────

describe('resolveKey — Tab and unbound keys', () => {
  it('Tab → null', () => {
    expect(resolveKey(k('Tab'))).toBeNull();
  });

  it('unbound key x → null', () => {
    expect(resolveKey(k('x'))).toBeNull();
  });

  it('Enter → null', () => {
    expect(resolveKey(k('Enter'))).toBeNull();
  });
});

// ── isEditableTarget ───────────────────────────────────────────────────────────

describe('isEditableTarget', () => {
  it('null → false', () => {
    expect(isEditableTarget(null)).toBe(false);
  });

  // Cast plain objects to EventTarget — isEditableTarget is intentionally duck-typed.
  function el(props: Record<string, unknown>): EventTarget {
    return props as unknown as EventTarget;
  }

  it('input element → true', () => {
    expect(isEditableTarget(el({ tagName: 'INPUT' }))).toBe(true);
  });

  it('textarea element → true', () => {
    expect(isEditableTarget(el({ tagName: 'TEXTAREA' }))).toBe(true);
  });

  it('select element → true', () => {
    expect(isEditableTarget(el({ tagName: 'SELECT' }))).toBe(true);
  });

  it('contenteditable div → true', () => {
    expect(isEditableTarget(el({ tagName: 'DIV', isContentEditable: true }))).toBe(true);
  });

  it('plain div → false', () => {
    expect(isEditableTarget(el({ tagName: 'DIV', isContentEditable: false }))).toBe(false);
  });

  it('button → false', () => {
    expect(isEditableTarget(el({ tagName: 'BUTTON' }))).toBe(false);
  });
});

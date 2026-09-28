/**
 * keymap.ts — pure key-binding resolution (no DOM dependencies).
 *
 * resolveKey maps a KeyboardEvent-shaped input to a KeyAction.
 * Rules:
 *  - Any meta, ctrl, or alt modifier → null (browser shortcuts are untouched).
 *  - While an editable element has focus only Escape resolves.
 *  - Tab → null (accessibility tree owns it).
 *  - Unrecognised keys → null.
 */

export type KeyAction = 'prev' | 'next' | 'run' | 'new' | 'filter' | 'stream' | 'escape';

export interface KeyInput {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  /** True when an editable element (input, textarea, select, contenteditable) has focus. */
  editable: boolean;
}

/** Bare-key → action map (no modifiers). */
const BINDINGS: Record<string, KeyAction> = {
  ArrowUp: 'prev',
  ArrowDown: 'next',
  r: 'run',
  n: 'new',
  '/': 'filter',
  o: 'stream',
  Escape: 'escape',
};

/**
 * Resolve a keyboard input to a KeyAction, or null if it should be ignored.
 */
export function resolveKey(input: KeyInput): KeyAction | null {
  // Any modifier → hand off to the browser.
  if (input.metaKey || input.ctrlKey || input.altKey) return null;

  const action = BINDINGS[input.key] ?? null;
  if (action === null) return null;

  // While in an editable field, only Escape passes through.
  if (input.editable && action !== 'escape') return null;

  return action;
}

/**
 * Duck-typed editable-target check — works in Node (no HTMLElement).
 *
 * Accepts: input, textarea, select, and any element with isContentEditable.
 */
export function isEditableTarget(target: EventTarget | null): boolean {
  if (target === null) return false;
  const el = target as {
    tagName?: unknown;
    isContentEditable?: unknown;
  };
  const tag = typeof el.tagName === 'string' ? el.tagName.toLowerCase() : '';
  if (tag === 'input' || tag === 'textarea' || tag === 'select') return true;
  if (el.isContentEditable === true) return true;
  return false;
}

'use client';

import { useEffect } from 'react';
import { resolveKey, isEditableTarget } from './keymap';
import type { KeyAction } from './keymap';

/**
 * useKeyboard — attach a single window keydown listener.
 *
 * For each keydown event, resolveKey decides which KeyAction fires (or null).
 * When an action has a handler, the event is consumed (preventDefault) and the
 * handler is called.
 */
export function useKeyboard(handlers: Partial<Record<KeyAction, () => void>>): void {
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent): void {
      const action = resolveKey({
        key: event.key,
        metaKey: event.metaKey,
        ctrlKey: event.ctrlKey,
        altKey: event.altKey,
        editable: isEditableTarget(event.target),
      });

      if (action === null) return;

      const handler = handlers[action];
      if (!handler) return;

      event.preventDefault();
      handler();
    }

    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  // Re-register when handler identities change.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [handlers]);
}

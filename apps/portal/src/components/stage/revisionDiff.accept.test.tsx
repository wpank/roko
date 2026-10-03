// @vitest-environment jsdom
/**
 * Acceptance (3229): after a revision lands, the plan view shows what the
 * planner changed. A result with one removed task and one changed verify
 * command renders both, the verify change first; an empty diff renders
 * nothing.
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render } from '@testing-library/react';
import type { WirePlanDiff } from '@/api/contracts';
import { RevisionDiff } from '@/components/stage/RevisionDiff';
import { textOf } from '@/test/dom';

afterEach(() => {
  cleanup();
});

const diff: WirePlanDiff = {
  meta: [],
  added: [],
  removed: ['T3'],
  changed: [
    {
      id: 'T1',
      keys: [
        {
          key: 'description',
          before: 'Add greet.sh.',
          after: 'Add greet.sh, which prints hello.',
        },
        {
          key: 'verify',
          before: '[{ command = "test -f greet.sh", phase = "test" }]',
          after: '[{ command = "bash greet.sh | grep -qx hello", phase = "test" }]',
        },
      ],
    },
  ],
};

describe('RevisionDiff', () => {
  it('renders the removed task and the changed verify command, verify first', () => {
    const onDismiss = vi.fn();
    render(<RevisionDiff diff={diff} onDismiss={onDismiss} />);
    expect(textOf(document.querySelector('[data-diff="removed"][data-task="T3"]'))).toContain(
      'T3 removed',
    );
    const fields = [
      ...document.querySelectorAll('[data-diff="changed"][data-task="T1"] [data-field]'),
    ];
    const names = fields.map((field) => field.getAttribute('data-field'));
    expect(names).toEqual(['verify', 'description']);
    expect(textOf(fields[0]!)).toContain('test -f greet.sh');
    expect(textOf(fields[0]!)).toContain('bash greet.sh | grep -qx hello');
    fireEvent.click(document.querySelector('button.rd-notice__close')!);
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it('renders nothing for an empty or absent diff', () => {
    const empty: WirePlanDiff = { meta: [], added: [], removed: [], changed: [] };
    expect(render(<RevisionDiff diff={empty} />).container.innerHTML).toBe('');
    expect(render(<RevisionDiff diff={null} />).container.innerHTML).toBe('');
  });
});

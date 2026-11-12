// @vitest-environment jsdom
/**
 * Acceptance: the alert is one sentence under the top bar, with its light, its keys and a close key. (The header's
 * own cases moved to look.accept.test.tsx with gap-195374.)
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render } from '@testing-library/react';
import { AlertBand } from '@/components/shell/AlertBand';
import { textOf } from '@/test/dom';

afterEach(() => cleanup());

describe('AlertBand', () => {
  it('renders one sentence with its severity’s light, its text and its keys', () => {
    const onAction = vi.fn();
    const onDismiss = vi.fn();
    const { container } = render(
      <AlertBand
        alert={{
          key: 'k1',
          severity: 'error',
          text: 'T03 failed: test — tests/hello.rs:7',
          actions: [{ kind: 'select-task', planId: 'hello', taskId: 'T03' }, { kind: 'retry', planId: 'hello' }],
        }}
        onAction={onAction}
        onDismiss={onDismiss}
      />,
    );
    const band = container.querySelector('[data-region="alert"]') as HTMLElement;
    expect(band.getAttribute('data-severity')).toBe('error');
    expect(band.querySelector('.rk-led[data-state="fail"]')).not.toBeNull();
    expect(textOf(band)).toContain('T03 failed: test — tests/hello.rs:7');
    const keys = [...band.querySelectorAll('button')].map((b) => b.textContent?.trim());
    expect(keys).toEqual(['show', 'resume', 'close']);
    fireEvent.click(band.querySelector('[data-alert-action="select-task"]')!);
    expect(onAction).toHaveBeenCalledWith({ kind: 'select-task', planId: 'hello', taskId: 'T03' });
    fireEvent.click(band.querySelector('[aria-label="Dismiss this notice"]')!);
    expect(onDismiss).toHaveBeenCalledWith('k1');
  });

  it('takes no space without an alert', () => {
    const { container } = render(<AlertBand alert={null} onAction={() => {}} onDismiss={() => {}} />);
    expect(container.innerHTML).toBe('');
  });
});

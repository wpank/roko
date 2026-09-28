// @vitest-environment jsdom
/**
 * Acceptance: every request failure in the editor and the prompt is the one
 * Notice — a missing route as "unsupported", anything else as "error" — with a
 * compact Close inside its row instead of a separate full-width button.
 * Copied verbatim from plans/portal-programme/08e-portal-refine/accept/.
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { OUTDATED_SERVER } from '@/lib/apiErrors';
import { Notice } from '@/components/primitives/Notice';
import { PromptPanel } from '@/components/stage/PromptPanel';
import { SourceEditor } from '@/components/stage/SourceEditor';
import { renderWithClient, stubFetch, textOf } from '@/test/dom';

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const note = (kind: string) => document.querySelector(`[data-notice="${kind}"]`);
const closeIn = (el: Element | null) => el?.querySelector('button.rd-notice__close') ?? null;

async function submit(sentence: string) {
  fireEvent.change(document.querySelector('[data-prompt]')!, { target: { value: sentence } });
  fireEvent.click(document.querySelector('[data-action="submit-prompt"]')!);
}

describe('Notice close', () => {
  it('puts a compact Close inside the notice row when it can be closed', () => {
    const onClose = vi.fn();
    render(<Notice kind="error" onClose={onClose}>Could not reach roko serve.</Notice>);
    const close = closeIn(note('error'));
    expect(note('error')?.getAttribute('role')).toBe('note');
    expect(textOf(close)).toBe('Close');
    expect(close?.getAttribute('type')).toBe('button');
    fireEvent.click(close!);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('draws no Close when nothing can close it', () => {
    render(<Notice>This roko serve does not support editing plans yet.</Notice>);
    expect(closeIn(note('unsupported'))).toBeNull();
  });
});

describe('editor and prompt notices', () => {
  it('gives the editor’s missing-route notice its only Close', async () => {
    stubFetch([
      { path: '/api/plans/hello/source', status: 404, body: { error: 'not_found', message: 'No route matches /api/plans/hello/source' } },
    ]);
    const onClose = vi.fn();
    renderWithClient(<SourceEditor planId="hello" running={false} onClose={onClose} onDirtyChange={() => {}} />);
    await waitFor(() => expect(note('unsupported')).not.toBeNull());
    expect(screen.getAllByRole('button', { name: 'Close' })).toEqual([closeIn(note('unsupported'))]);
    fireEvent.click(closeIn(note('unsupported'))!);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('shows any other editor load failure as an error notice with the same Close', async () => {
    stubFetch([{ path: '/api/plans/hello/source', status: 500, body: { message: 'disk full' } }]);
    const onClose = vi.fn();
    renderWithClient(<SourceEditor planId="hello" running={false} onClose={onClose} onDirtyChange={() => {}} />);
    await waitFor(() => expect(note('error')).not.toBeNull());
    expect(textOf(note('error'))).toContain('disk full');
    expect(screen.getAllByRole('button', { name: 'Close' })).toEqual([closeIn(note('error'))]);
    fireEvent.click(closeIn(note('error'))!);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('shows generate’s error in an error notice whose Close clears it', async () => {
    stubFetch([
      {
        method: 'POST',
        path: '/api/plans/generate',
        status: 400,
        body: { code: 'invalid_json', message: 'request body must be valid JSON', details: { reason: 'missing field `slug`' } },
      },
    ]);
    renderWithClient(<PromptPanel mode="generate" workspace="hello" firstRun={false} onDone={() => {}} />);
    await submit('a rust app that prints hello world');
    await screen.findByText(OUTDATED_SERVER);
    expect(textOf(note('error'))).toContain(OUTDATED_SERVER);
    expect(note('error')?.querySelector('[data-error]')).not.toBeNull();
    fireEvent.click(closeIn(note('error'))!);
    expect(note('error')).toBeNull();
    expect(textOf(document.body)).not.toContain(OUTDATED_SERVER);
  });

  it('lets the missing-route notice be closed too', async () => {
    stubFetch([{ method: 'POST', path: '/api/plans/generate', status: 404 }]);
    renderWithClient(<PromptPanel mode="generate" workspace="hello" firstRun={false} onDone={() => {}} />);
    await submit('a rust app that prints hello world');
    await waitFor(() => expect(note('unsupported')).not.toBeNull());
    fireEvent.click(closeIn(note('unsupported'))!);
    expect(note('unsupported')).toBeNull();
  });
});

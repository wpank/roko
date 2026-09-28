// @vitest-environment jsdom
/**
 * Acceptance: the editor and the prompt say "not supported" the same way (the
 * Notice), the editor reports unsaved text honestly, and an older server's
 * parser error becomes a sentence.
 * Copied verbatim from plans/portal-programme/08d-portal-legibility/accept/.
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, screen, waitFor } from '@testing-library/react';
import { OUTDATED_SERVER } from '@/lib/apiErrors';
import { PromptPanel } from '@/components/stage/PromptPanel';
import { SourceEditor } from '@/components/stage/SourceEditor';
import { renderWithClient, stubFetch, textOf } from '@/test/dom';

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const TOML = '[meta]\nplan = "hello"\n';
const notice = () => document.querySelector('[data-notice="unsupported"]');

describe('SourceEditor', () => {
  it('shows a missing source route as the notice, with a way out, and reports no unsaved text', async () => {
    stubFetch([
      { path: '/api/plans/hello/source', status: 404, body: { error: 'not_found', message: 'No route matches /api/plans/hello/source' } },
    ]);
    const onDirtyChange = vi.fn();
    renderWithClient(<SourceEditor planId="hello" running={false} onClose={() => {}} onDirtyChange={onDirtyChange} />);
    await waitFor(() => expect(notice()).not.toBeNull());
    expect(textOf(notice())).toContain('This roko serve does not support editing plans yet.');
    expect(screen.getByRole('button', { name: 'Close' })).toBeTruthy();
    expect(onDirtyChange).not.toHaveBeenCalledWith(true);
  });

  it('reports unsaved text while it differs from what loaded', async () => {
    stubFetch([{ path: '/api/plans/hello/source', body: { id: 'hello', path: 'plans/hello/tasks.toml', toml: TOML } }]);
    const onDirtyChange = vi.fn();
    renderWithClient(<SourceEditor planId="hello" running={false} onClose={() => {}} onDirtyChange={onDirtyChange} />);
    const editor = (await screen.findByLabelText('Plan source TOML')) as HTMLTextAreaElement;
    await waitFor(() => expect(editor.value).toBe(TOML));
    fireEvent.change(editor, { target: { value: TOML + '# edit\n' } });
    expect(onDirtyChange).toHaveBeenLastCalledWith(true);
    fireEvent.change(editor, { target: { value: TOML } });
    expect(onDirtyChange).toHaveBeenLastCalledWith(false);
  });
});

describe('PromptPanel', () => {
  async function submit(sentence: string) {
    fireEvent.change(document.querySelector('[data-prompt]')!, { target: { value: sentence } });
    fireEvent.click(document.querySelector('[data-action="submit-prompt"]')!);
  }

  it('shows a missing generate route as the notice', async () => {
    stubFetch([{ method: 'POST', path: '/api/plans/generate', status: 404 }]);
    renderWithClient(<PromptPanel mode="generate" workspace="hello" firstRun={false} onDone={() => {}} />);
    await submit('a rust app that prints hello world');
    await waitFor(() => expect(notice()).not.toBeNull());
    expect(textOf(notice())).toContain('This roko serve does not support generating plans yet.');
  });

  it('shows a missing revise route as the same notice', async () => {
    stubFetch([{ method: 'POST', path: '/api/plans/hello/revise', status: 405, statusText: 'Method Not Allowed' }]);
    renderWithClient(<PromptPanel mode="revise" planId="hello" workspace="hello" firstRun={false} onDone={() => {}} />);
    await submit('also print the date');
    await waitFor(() => expect(notice()).not.toBeNull());
    expect(textOf(notice())).toContain('This roko serve does not support revising plans yet.');
  });

  it('turns an older server’s parser error into a sentence', async () => {
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
    expect(textOf(document.body)).not.toContain('request body must be valid JSON');
    expect(notice()).toBeNull();
  });
});

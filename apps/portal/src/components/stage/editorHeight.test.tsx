// @vitest-environment jsdom
/**
 * The plan editor fills the stage (design §2, §4a): its text box once showed
 * two rows, 66 px of a 616 px file, because it took a percentage of a box as
 * tall as its content. jsdom lays nothing out, so this reads the classes that
 * give each box its height, from the text box up to the stage.
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, screen, waitFor } from '@testing-library/react';
import type { WirePlanSummary, WirePlanTasks, WireValidation } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { Stage } from '@/components/stage/Stage';
import { initialRunState } from '@/lib/runState';
import { renderWithClient, setStore, stubFetch } from '@/test/dom';

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const PLAN: WirePlanSummary = {
  id: 'hello',
  title: 'Hello world',
  task_count: 1,
  tasks_failed: 0,
  completed: false,
  status: 'pending',
  old_format: false,
};

const TASKS: WirePlanTasks = {
  plan_id: 'hello',
  task_count: 1,
  max_parallel: 1,
  tasks: [
    { id: 'T01', title: 'Print hello world', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['structural'] },
  ],
};

const VALID: WireValidation = { valid: true, errors: [], warnings: [], diagnostics: [] };
const TOML = '[meta]\nplan = "hello"\n';

const has = (el: Element, ...names: string[]) => names.every((n) => el.classList.contains(n));

/**
 * Whether `el` takes its height from `parent` rather than from its content: a
 * positioned box sized to its positioned parent, a flex item growing into a
 * column, or a percentage of the stage, the one box whose height is set from
 * outside (the main column flexes it). A percentage of any other box here is a
 * percentage of its content.
 */
function fillsParent(el: Element, parent: Element): boolean {
  if (has(el, 'absolute', 'h-full')) return has(parent, 'relative');
  if (has(el, 'flex-1')) return has(parent, 'flex', 'flex-col');
  return parent.getAttribute('data-region') === 'stage' && (has(el, 'h-full') || has(el, 'min-h-full'));
}

describe('the plan editor', () => {
  it('fills the stage: every box from the text box up takes its height from its parent', async () => {
    setStore(initialRunState());
    stubFetch([{ path: '/api/plans/hello/source', body: { id: 'hello', path: 'plans/hello/tasks.toml', toml: TOML } }]);
    renderWithClient(
      <Stage
        plans={[PLAN]}
        loaded
        selection={{ plan: 'hello', task: null }}
        promptOpen={false}
        workspace="hello"
        onSelect={() => {}}
        onClosePrompt={() => {}}
        onRequestError={() => {}}
      />,
      { seed: [[queryKeys.planTasks('hello'), TASKS], [queryKeys.validation('hello'), VALID]] },
    );
    fireEvent.click(document.querySelector('[data-action="edit"]')!);
    const editor = (await screen.findByLabelText('Plan source TOML')) as HTMLTextAreaElement;
    await waitFor(() => expect(editor.value).toBe(TOML));

    const stage = document.querySelector('[data-region="stage"]')!;
    const path: string[] = [];
    for (let el: Element = editor; el !== stage; el = el.parentElement!) {
      path.push(`<${el.tagName.toLowerCase()} class="${el.className}">`);
      expect(fillsParent(el, el.parentElement!), path.join(' in ')).toBe(true);
    }
    // Never so short that the stage has no room for it: it scrolls instead.
    expect(has(editor.parentElement!, 'min-h-32')).toBe(true);
  });
});

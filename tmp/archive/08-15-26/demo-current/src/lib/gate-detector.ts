import type { GateState } from './types';

const gatePatterns: { name: string; pass: RegExp; fail: RegExp; running: RegExp }[] = [
  {
    name: 'compile',
    pass: /(?:compile|build)\s*(?:passed|success|ok|✓)/i,
    fail: /(?:compile|build)\s*(?:failed|error|✕)/i,
    running: /(?:compiling|building)/i,
  },
  {
    name: 'test',
    pass: /(?:test|tests)\s*(?:passed|success|ok|✓)/i,
    fail: /(?:test|tests)\s*(?:failed|error|✕)/i,
    running: /(?:testing|running tests)/i,
  },
  {
    name: 'clippy',
    pass: /clippy\s*(?:passed|clean|ok|✓)/i,
    fail: /clippy\s*(?:failed|warnings?|error|✕)/i,
    running: /(?:running clippy|linting)/i,
  },
  {
    name: 'diff',
    pass: /diff\s*(?:passed|clean|ok|✓)/i,
    fail: /diff\s*(?:failed|dirty|✕)/i,
    running: /(?:checking diff|diffing)/i,
  },
];

export function detectGateChanges(line: string, current: GateState[]): GateState[] | null {
  let changed = false;
  const next = current.map(g => ({ ...g }));

  for (const pattern of gatePatterns) {
    const idx = next.findIndex(g => g.name === pattern.name);
    if (idx === -1) continue;

    if (pattern.pass.test(line) && next[idx].status !== 'pass') {
      next[idx].status = 'pass';
      changed = true;
    } else if (pattern.fail.test(line) && next[idx].status !== 'fail') {
      next[idx].status = 'fail';
      changed = true;
    } else if (pattern.running.test(line) && next[idx].status !== 'running') {
      next[idx].status = 'running';
      changed = true;
    }
  }

  return changed ? next : null;
}

export function defaultGates(): GateState[] {
  return gatePatterns.map(p => ({ name: p.name, status: 'pending' as const }));
}

/**
 * waves.ts — Kahn-based dependency-wave layering for portal task DAGs.
 *
 * computeWaves(tasks) → { waves, diagnostics }
 *
 * Algorithm:
 *  1. Strip and report dependencies on unknown task ids (dangling).
 *  2. Run Kahn's topological-sort wave-by-wave on the reduced graph,
 *     preserving input order within each wave.
 *  3. Any tasks that never become ready form cycle(s); report each cycle
 *     path via DFS and append all stuck tasks as a final wave so a broken
 *     plan still renders.
 *  Never throws.
 */

export interface WaveDiagnostic {
  kind: 'cycle' | 'dangling';
  taskIds: string[];
  message: string;
}

export interface WaveResult {
  waves: string[][];
  diagnostics: WaveDiagnostic[];
}

type Task = { id: string; depends_on: readonly string[] };

export function computeWaves(tasks: ReadonlyArray<Task>): WaveResult {
  const diagnostics: WaveDiagnostic[] = [];
  const knownIds = new Set(tasks.map((t) => t.id));
  const taskById = new Map<string, Task>();
  for (const t of tasks) {
    taskById.set(t.id, t);
  }

  // ── 1. Report dangling dependencies ──────────────────────────────────────
  for (const task of tasks) {
    for (const dep of task.depends_on) {
      if (!knownIds.has(dep)) {
        diagnostics.push({
          kind: 'dangling',
          taskIds: [task.id],
          message: `${task.id} depends on ${dep}, which does not exist`,
        });
      }
    }
  }

  // ── 2. Build reduced adjacency graph (known deps only) ───────────────────
  // inDegree[id] = number of known deps remaining
  const inDegree = new Map<string, number>();
  // adj[dep] = tasks that depend on dep (forward edges for decrement)
  const adj = new Map<string, string[]>();

  for (const t of tasks) {
    inDegree.set(t.id, 0);
    adj.set(t.id, []);
  }
  for (const t of tasks) {
    for (const dep of t.depends_on) {
      if (knownIds.has(dep)) {
        inDegree.set(t.id, inDegree.get(t.id)! + 1);
        adj.get(dep)!.push(t.id);
      }
    }
  }

  // ── 3. Kahn layering (input order preserved within each wave) ────────────
  const inputOrder = tasks.map((t) => t.id);
  const placed = new Set<string>();
  const waves: string[][] = [];

  let wave = inputOrder.filter((id) => inDegree.get(id) === 0);
  while (wave.length > 0) {
    waves.push(wave);
    for (const id of wave) {
      placed.add(id);
    }
    for (const id of wave) {
      for (const dependent of adj.get(id)!) {
        inDegree.set(dependent, inDegree.get(dependent)! - 1);
      }
    }
    wave = inputOrder.filter((id) => !placed.has(id) && inDegree.get(id) === 0);
  }

  // ── 4. Cycle detection for stuck tasks ───────────────────────────────────
  const stuckIds = inputOrder.filter((id) => !placed.has(id));
  if (stuckIds.length > 0) {
    const stuckSet = new Set(stuckIds);

    // DFS within the stuck subgraph to find cycle paths.
    // We follow the depends_on direction (task → dep it needs).
    const visited = new Set<string>();
    const stackPos = new Map<string, number>(); // id → index in current path
    const path: string[] = [];

    const dfs = (id: string): void => {
      if (visited.has(id)) return;
      visited.add(id);
      stackPos.set(id, path.length);
      path.push(id);

      const task = taskById.get(id)!;
      for (const dep of task.depends_on) {
        if (!stuckSet.has(dep)) continue;
        if (stackPos.has(dep)) {
          // Back-edge found: extract the cycle segment.
          const start = stackPos.get(dep)!;
          const cyclePath = path.slice(start);
          const pathStr = [...cyclePath, cyclePath[0]!].join(' → ');
          diagnostics.push({
            kind: 'cycle',
            taskIds: [...cyclePath],
            message: `${pathStr} form a cycle`,
          });
        } else {
          dfs(dep);
        }
      }

      path.pop();
      stackPos.delete(id);
    };

    for (const id of stuckIds) {
      if (!visited.has(id)) {
        dfs(id);
      }
    }

    // Append ALL stuck tasks as a final wave (input order) so the plan renders.
    waves.push(stuckIds);
  }

  return { waves, diagnostics };
}

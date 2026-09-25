'use client';

import React, { useMemo, useCallback } from 'react';
import { clsx } from 'clsx';
import type { TaskState, TaskStatus } from '@/api/types';

export interface PlanDAGViewerProps {
  tasks: TaskState[];
  selectedTaskId?: string;
  onSelectTask?: (id: string) => void;
}

// ---------------------------------------------------------------------------
// Layout constants
// ---------------------------------------------------------------------------

const NODE_WIDTH = 160;
const NODE_HEIGHT = 36;
const WAVE_GAP_Y = 80;
const NODE_GAP_X = 24;
const PADDING_X = 32;
const PADDING_Y = 32;
const ARROW_SIZE = 6;

// ---------------------------------------------------------------------------
// Status color map
// ---------------------------------------------------------------------------

interface StatusColors {
  fill: string;
  stroke: string;
  text: string;
}

const STATUS_COLORS: Record<TaskStatus, StatusColors> = {
  pending:     { fill: 'var(--bg-raised)',    stroke: 'var(--text-ghost)',    text: 'var(--text-ghost)' },
  dispatching: { fill: 'var(--bg-raised)',    stroke: 'var(--warning)',       text: 'var(--warning)' },
  running:     { fill: 'var(--bg-raised)',    stroke: 'var(--warning)',       text: 'var(--warning)' },
  gating:      { fill: 'var(--bg-raised)',    stroke: 'var(--dream)',         text: 'var(--dream)' },
  completed:   { fill: 'var(--bg-raised)',    stroke: 'var(--sage)',          text: 'var(--sage)' },
  failed:      { fill: 'var(--bg-raised)',    stroke: 'var(--accent-error)',  text: 'var(--accent-error)' },
};

// ---------------------------------------------------------------------------
// Layout computation
// ---------------------------------------------------------------------------

interface PositionedTask {
  task: TaskState;
  x: number;
  y: number;
  cx: number; // center x
  cy: number; // center y
}

function layoutTasks(tasks: TaskState[]): {
  nodes: PositionedTask[];
  svgWidth: number;
  svgHeight: number;
} {
  if (tasks.length === 0) {
    return { nodes: [], svgWidth: 200, svgHeight: 100 };
  }

  // Group tasks by wave for topological layout.
  const waveMap = new Map<number, TaskState[]>();
  for (const task of tasks) {
    const w = task.wave ?? 0;
    if (!waveMap.has(w)) waveMap.set(w, []);
    waveMap.get(w)!.push(task);
  }

  const waves = Array.from(waveMap.keys()).sort((a, b) => a - b);
  const nodes: PositionedTask[] = [];

  // Compute max column count for centering narrower waves.
  const maxCols = Math.max(...Array.from(waveMap.values()).map((t) => t.length));
  const totalWidth = maxCols * NODE_WIDTH + (maxCols - 1) * NODE_GAP_X + PADDING_X * 2;

  for (let wi = 0; wi < waves.length; wi++) {
    const waveTasks = waveMap.get(waves[wi])!;
    const rowWidth =
      waveTasks.length * NODE_WIDTH + (waveTasks.length - 1) * NODE_GAP_X;
    const rowStartX = PADDING_X + (totalWidth - PADDING_X * 2 - rowWidth) / 2;
    const y = PADDING_Y + wi * (NODE_HEIGHT + WAVE_GAP_Y);

    for (let ni = 0; ni < waveTasks.length; ni++) {
      const x = rowStartX + ni * (NODE_WIDTH + NODE_GAP_X);
      nodes.push({
        task: waveTasks[ni],
        x,
        y,
        cx: x + NODE_WIDTH / 2,
        cy: y + NODE_HEIGHT / 2,
      });
    }
  }

  const svgWidth = totalWidth;
  const svgHeight =
    PADDING_Y * 2 + waves.length * (NODE_HEIGHT + WAVE_GAP_Y) - WAVE_GAP_Y;

  return { nodes, svgWidth, svgHeight };
}

// ---------------------------------------------------------------------------
// Edge path helper
// ---------------------------------------------------------------------------

function edgePath(
  fromCx: number,
  fromBottom: number,
  toCx: number,
  toTop: number,
): string {
  const midY = (fromBottom + toTop) / 2;
  return `M ${fromCx} ${fromBottom} C ${fromCx} ${midY}, ${toCx} ${midY}, ${toCx} ${toTop}`;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function PlanDAGViewer({
  tasks,
  selectedTaskId,
  onSelectTask,
}: PlanDAGViewerProps) {
  const { nodes, svgWidth, svgHeight } = useMemo(
    () => layoutTasks(tasks),
    [tasks],
  );

  // Build a quick lookup from task id → positioned node.
  const nodeById = useMemo(() => {
    const map = new Map<string, PositionedTask>();
    for (const n of nodes) map.set(n.task.id, n);
    return map;
  }, [nodes]);

  // Build edge list.
  const edges = useMemo(() => {
    const result: Array<{ fromId: string; toId: string; path: string }> = [];
    for (const node of nodes) {
      for (const depId of node.task.dependsOn) {
        const from = nodeById.get(depId);
        if (!from) continue;
        result.push({
          fromId: depId,
          toId: node.task.id,
          path: edgePath(
            from.cx,
            from.y + NODE_HEIGHT,
            node.cx,
            node.y,
          ),
        });
      }
    }
    return result;
  }, [nodes, nodeById]);

  const handleKeyDown = useCallback(
    (id: string) => (e: React.KeyboardEvent) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        onSelectTask?.(id);
      }
    },
    [onSelectTask],
  );

  if (tasks.length === 0) {
    return (
      <div className="flex items-center justify-center h-24 text-[var(--text-ghost)] font-mono text-xs">
        no tasks
      </div>
    );
  }

  return (
    <div className="w-full overflow-x-auto">
      <svg
        width={svgWidth}
        height={svgHeight}
        viewBox={`0 0 ${svgWidth} ${svgHeight}`}
        aria-label="Plan task dependency graph"
        className="block"
      >
        <defs>
          {/* Arrow marker */}
          <marker
            id="dag-arrow"
            viewBox="0 0 10 10"
            refX="9"
            refY="5"
            markerWidth={ARROW_SIZE}
            markerHeight={ARROW_SIZE}
            orient="auto-start-reverse"
          >
            <path d="M 0 0 L 10 5 L 0 10 z" fill="var(--text-ghost)" />
          </marker>
          <marker
            id="dag-arrow-selected"
            viewBox="0 0 10 10"
            refX="9"
            refY="5"
            markerWidth={ARROW_SIZE}
            markerHeight={ARROW_SIZE}
            orient="auto-start-reverse"
          >
            <path d="M 0 0 L 10 5 L 0 10 z" fill="var(--rose)" />
          </marker>
        </defs>

        {/* Edges — rendered behind nodes */}
        {edges.map((edge) => {
          const isSelected =
            edge.fromId === selectedTaskId || edge.toId === selectedTaskId;
          return (
            <path
              key={`${edge.fromId}-${edge.toId}`}
              d={edge.path}
              fill="none"
              stroke={isSelected ? 'var(--rose)' : 'var(--text-ghost)'}
              strokeWidth={isSelected ? 1.5 : 1}
              strokeOpacity={isSelected ? 0.8 : 0.4}
              markerEnd={
                isSelected ? 'url(#dag-arrow-selected)' : 'url(#dag-arrow)'
              }
            />
          );
        })}

        {/* Nodes */}
        {nodes.map((node) => {
          const isSelected = node.task.id === selectedTaskId;
          const colors = STATUS_COLORS[node.task.status] ?? STATUS_COLORS.pending;
          const isRunning = node.task.status === 'running';

          return (
            <g
              key={node.task.id}
              role="button"
              tabIndex={onSelectTask ? 0 : undefined}
              aria-pressed={isSelected}
              aria-label={node.task.name}
              onClick={() => onSelectTask?.(node.task.id)}
              onKeyDown={handleKeyDown(node.task.id)}
              style={{ cursor: onSelectTask ? 'pointer' : 'default' }}
            >
              {/* Glow behind selected nodes */}
              {isSelected && (
                <rect
                  x={node.x - 3}
                  y={node.y - 3}
                  width={NODE_WIDTH + 6}
                  height={NODE_HEIGHT + 6}
                  rx={5}
                  ry={5}
                  fill="none"
                  stroke="var(--rose)"
                  strokeWidth={2}
                  strokeOpacity={0.5}
                />
              )}

              {/* Node body — rounded rect (intentional exception for graph nodes) */}
              <rect
                x={node.x}
                y={node.y}
                width={NODE_WIDTH}
                height={NODE_HEIGHT}
                rx={4}
                ry={4}
                fill={colors.fill}
                stroke={isSelected ? 'var(--rose)' : colors.stroke}
                strokeWidth={isSelected ? 1.5 : 1}
                strokeOpacity={isRunning ? undefined : 0.7}
              >
                {isRunning && (
                  <animate
                    attributeName="stroke-opacity"
                    values="0.4;1;0.4"
                    dur="1.6s"
                    repeatCount="indefinite"
                  />
                )}
              </rect>

              {/* Status dot */}
              <circle
                cx={node.x + 12}
                cy={node.cy}
                r={3}
                fill={colors.stroke}
                fillOpacity={isRunning ? undefined : 0.8}
              >
                {isRunning && (
                  <animate
                    attributeName="fill-opacity"
                    values="0.3;1;0.3"
                    dur="1.6s"
                    repeatCount="indefinite"
                  />
                )}
              </circle>

              {/* Task name */}
              <text
                x={node.x + 24}
                y={node.cy + 1}
                dominantBaseline="middle"
                fontFamily="var(--font-mono)"
                fontSize={10}
                fill={isSelected ? 'var(--text-strong)' : colors.text}
                textAnchor="start"
              >
                <title>{node.task.name}</title>
                {node.task.name.length > 16
                  ? `${node.task.name.slice(0, 15)}…`
                  : node.task.name}
              </text>

              {/* Wave label (top-right corner) */}
              <text
                x={node.x + NODE_WIDTH - 6}
                y={node.y + 7}
                fontFamily="var(--font-mono)"
                fontSize={8}
                fill="var(--text-ghost)"
                textAnchor="end"
                dominantBaseline="middle"
              >
                w{node.task.wave}
              </text>
            </g>
          );
        })}
      </svg>
    </div>
  );
}

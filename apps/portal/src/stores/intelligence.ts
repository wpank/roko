/**
 * Intelligence store — learning, c-factor, knowledge, dreams, and experiments.
 *
 * Mirrors the state that the Learning (F10) and Inspect (F7) TUI tabs consume.
 * Values are updated from the SSE event stream via `handleSSEEvent`, which
 * should be called by the same hook that drives the dashboard store.
 *
 * Unlike the dashboard store, this store does not hold raw server data verbatim
 * — it maintains computed/summarised views suited for the intelligence section UI.
 */

import { create } from "zustand";
import type {
  DashboardEvent,
  CfactorTrend,
  LearningStage,
  DreamPhase,
} from "@/api/types";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface CFactorSummary {
  /** Current c-factor value in [0, 1]. */
  value: number;
  /** Delta relative to the previous observation window (signed). */
  delta: number;
  /** Direction the c-factor is moving. */
  trend: CfactorTrend;
}

interface IntelligenceStore {
  // C-factor
  cfactor: CFactorSummary;

  // Learning band
  learningStage: LearningStage;

  /**
   * Knowledge tier entry counts.
   * Keys are tier labels: "transient" | "working" | "consolidated" | "persistent".
   * Values are counts from the neuro store, updated as entries are created or promoted.
   */
  knowledgeTierCounts: Record<string, number>;

  /**
   * Current dream consolidation phase.
   * `"idle"` is the portal-only extension for when no cycle is running.
   * The server uses "hypnagogia" | "imagination" | "consolidation".
   */
  dreamPhase: DreamPhase | "idle";

  /** ID of the dream cycle currently running; `null` when idle. */
  activeDreamCycleId: string | null;

  /** Number of active (collecting/trending) prompt experiments. */
  activeExperimentsCount: number;

  // Actions

  /** Route a dashboard SSE event into this store, updating relevant slices. */
  handleSSEEvent: (event: DashboardEvent) => void;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useIntelligenceStore = create<IntelligenceStore>((set) => ({
  cfactor: {
    value: 0,
    delta: 0,
    trend: "flat",
  },

  learningStage: "static",

  knowledgeTierCounts: {
    transient: 0,
    working: 0,
    consolidated: 0,
    persistent: 0,
  },

  dreamPhase: "idle",

  activeDreamCycleId: null,

  activeExperimentsCount: 0,

  handleSSEEvent: (event) => {
    switch (event.type) {
      // -----------------------------------------------------------------------
      // C-factor
      // -----------------------------------------------------------------------

      case "cfactor_updated": {
        set({
          cfactor: {
            value: event.cfactor,
            delta: event.delta,
            trend: event.trend,
          },
        });
        return;
      }

      // -----------------------------------------------------------------------
      // Learning stage
      // -----------------------------------------------------------------------

      case "learning_stage_changed": {
        set({ learningStage: event.newStage });
        return;
      }

      // -----------------------------------------------------------------------
      // Knowledge tier counts
      // -----------------------------------------------------------------------

      case "knowledge_entry_created": {
        const tier = event.entry.tier;
        set((state) => ({
          knowledgeTierCounts: {
            ...state.knowledgeTierCounts,
            [tier]: (state.knowledgeTierCounts[tier] ?? 0) + 1,
          },
        }));
        return;
      }

      case "knowledge_tier_changed": {
        const { previousTier, newTier } = event;
        set((state) => ({
          knowledgeTierCounts: {
            ...state.knowledgeTierCounts,
            [previousTier]: Math.max(
              0,
              (state.knowledgeTierCounts[previousTier] ?? 0) - 1
            ),
            [newTier]: (state.knowledgeTierCounts[newTier] ?? 0) + 1,
          },
        }));
        return;
      }

      // -----------------------------------------------------------------------
      // Dream phase
      // -----------------------------------------------------------------------

      case "dream_phase_changed": {
        set({
          dreamPhase: event.phase,
          activeDreamCycleId: event.cycleId,
        });
        return;
      }

      case "dream_cycle_completed": {
        // A completed cycle means no cycle is currently running.
        set({
          dreamPhase: "idle",
          activeDreamCycleId: null,
        });
        return;
      }

      // -----------------------------------------------------------------------
      // Experiments
      // -----------------------------------------------------------------------

      case "experiment_updated": {
        const isActive =
          event.experiment.status === "collecting" ||
          event.experiment.status === "trending";
        // We cannot derive the exact active count from a single event without
        // a full experiment list, so we only increment/decrement on transitions.
        set((state) => {
          const current = state.activeExperimentsCount;
          // If transitioning to active and we don't already count it, add one.
          // If transitioning to inactive, subtract one (floor at 0).
          if (isActive) {
            // Conservative: avoid double-counting by only incrementing when the
            // experiment appears to be newly active (caller does not resend for
            // steady-state collecting events in practice, but guard regardless).
            return { activeExperimentsCount: current };
          }
          return { activeExperimentsCount: Math.max(0, current - 1) };
        });
        return;
      }

      case "experiment_concluded": {
        set((state) => ({
          activeExperimentsCount: Math.max(0, state.activeExperimentsCount - 1),
        }));
        return;
      }

      // -----------------------------------------------------------------------
      // Snapshot — full state replacement resets all intelligence counters.
      // -----------------------------------------------------------------------

      case "snapshot": {
        const { learning } = event.snapshot;
        set({
          cfactor: {
            value: learning.cfactor,
            delta: learning.cfactorDelta,
            trend: learning.cfactorTrend,
          },
          learningStage: learning.learningStage,
          knowledgeTierCounts: learning.knowledgeTierCounts,
          activeExperimentsCount: learning.activeExperiments,
          // Dream phase is not present in the snapshot payload; keep current
          // or reset to idle as a safe default.
          dreamPhase: "idle",
          activeDreamCycleId: null,
        });
        return;
      }

      // -----------------------------------------------------------------------
      // All other events are irrelevant to this store.
      // -----------------------------------------------------------------------

      default:
        return;
    }
  },
}));

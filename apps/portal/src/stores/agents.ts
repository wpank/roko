/**
 * Agents store — UI selection and filter state for the Agents tab.
 *
 * This store is deliberately thin: it holds only ephemeral UI state that
 * does not need to survive a page reload. The live agent data itself lives
 * in the dashboard store and is read via `useDashboardStore`.
 */

import { create } from "zustand";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/**
 * The four sub-tabs available in the agent detail panel.
 *
 * - `output`   — streaming agent stdout / LLM tokens
 * - `gates`    — gate results for this agent's current/last task
 * - `metrics`  — token counts, latency, cost
 * - `episodes` — episode entries attributed to this agent
 */
export type AgentOutputSubTab = "output" | "gates" | "metrics" | "episodes";

interface AgentsStore {
  /** The currently selected agent id, or `null` when no agent is focused. */
  selectedAgentId: string | null;

  /**
   * Active role filter applied to the agent list.
   *
   * `"all"` means no filter. Any other string is matched against
   * `AgentState.role` (case-insensitive exact match on the normalised label).
   */
  roleFilter: string;

  /**
   * Which sub-tab is active in the agent detail/output panel.
   * Defaults to `"output"` so the streaming text is visible immediately.
   */
  outputSubTab: AgentOutputSubTab;

  // Actions

  /**
   * Select or deselect an agent.
   * Passing `null` collapses the detail panel.
   */
  selectAgent: (id: string | null) => void;

  /**
   * Update the role filter.
   * Pass `"all"` to clear the filter.
   */
  setRoleFilter: (role: string) => void;

  /** Switch the active sub-tab in the agent detail panel. */
  setOutputSubTab: (tab: AgentOutputSubTab) => void;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useAgentsStore = create<AgentsStore>((set) => ({
  selectedAgentId: null,
  roleFilter: "all",
  outputSubTab: "output",

  selectAgent: (id) => {
    set((state) => ({
      selectedAgentId: id,
      // When switching to a new agent, reset the sub-tab so the output panel
      // always starts on the streaming output view — the most useful default.
      outputSubTab: id !== state.selectedAgentId ? "output" : state.outputSubTab,
    }));
  },

  setRoleFilter: (role) => set({ roleFilter: role }),

  setOutputSubTab: (tab) => set({ outputSubTab: tab }),
}));

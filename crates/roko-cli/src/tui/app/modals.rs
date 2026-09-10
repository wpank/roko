//! Modal state management: approval requests, confirmation dialogs, and
//! TUI command dispatch for confirmed actions.

use super::*;

impl App {
    pub(super) fn open_confirm_modal(&mut self, action: ConfirmAction) {
        self.tui_state.input_mode = InputMode::Confirm;
        self.tui_state.pending_confirm = Some(action.clone());
        let modal_action = modals_mod::ConfirmAction::Custom {
            message: action.to_string(),
        };
        self.tui_state.active_modal = Some(ModalState::Confirm {
            action: modal_action,
        });
    }


    pub(super) fn resolve_active_approval(&mut self, approved: bool) -> bool {
        if !matches!(
            self.tui_state.active_modal,
            Some(ModalState::Approval { .. })
        ) {
            return false;
        }

        if let Some(response_tx) = self.pending_approval_response.take() {
            let _ = response_tx.send(approved);
        }

        self.tui_state.pending_approval = None;
        self.tui_state.active_modal = None;
        if self.tui_state.input_mode == InputMode::Confirm {
            self.tui_state.input_mode = InputMode::Normal;
        }
        true
    }


    pub(super) fn accept_approval_request(&mut self, request: ApprovalRequest) {
        let ApprovalRequest {
            role,
            command,
            approval_id,
            response_tx,
        } = request;

        if self.pending_approval_response.is_some() {
            let _ = response_tx.send(false);
            return;
        }

        self.tui_state.pending_approval = Some(PendingApproval {
            agent_id: role.clone(),
            description: approval_id,
            command: command.clone(),
            run_id: None,
            approval_id: None,
        });
        self.pending_approval_response = Some(response_tx);
        self.tui_state.input_mode = InputMode::Confirm;
        self.tui_state.active_modal = Some(ModalState::Approval { role, command });
    }


    pub(super) fn drain_approval_requests(&mut self) {
        let Some(mut rx) = self.approval_rx.take() else {
            return;
        };

        let mut disconnected = false;
        let mut got_request = false;
        loop {
            match rx.try_recv() {
                Ok(request) => {
                    self.accept_approval_request(request);
                    got_request = true;
                }
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }

        if got_request {
            self.render_dirty.insert(RenderDirty::MODAL);
        }

        if !disconnected {
            self.approval_rx = Some(rx);
        }
    }


    pub(super) fn resolve_confirm_action(&self, action: ConfirmAction) -> ConfirmAction {
        match action {
            ConfirmAction::DiagnosePlan(plan_id) if plan_id.is_empty() => {
                ConfirmAction::DiagnosePlan(self.selected_plan_id().unwrap_or_default())
            }
            ConfirmAction::MergePlan { plan_id, branch }
                if plan_id.is_empty() || branch.is_empty() =>
            {
                ConfirmAction::MergePlan {
                    plan_id: if plan_id.is_empty() {
                        self.selected_plan_id().unwrap_or_default()
                    } else {
                        plan_id
                    },
                    branch: if branch.is_empty() {
                        self.current_git_branch()
                    } else {
                        branch
                    },
                }
            }
            ConfirmAction::MergeAllDone { branches } if branches.is_empty() => {
                ConfirmAction::MergeAllDone {
                    branches: self.completed_plan_branches(),
                }
            }
            ConfirmAction::ResetSelectedPlan(plan_id) if plan_id.is_empty() => {
                ConfirmAction::ResetSelectedPlan(self.selected_plan_id().unwrap_or_default())
            }
            other => other,
        }
    }

    /// Map a confirmed `ConfirmAction` to the corresponding
    /// `ExecutionCommand` and send it through the in-process channel (if
    /// connected to an executor).
    ///
    /// Returns `true` when the action maps to an executor command AND the
    /// channel was available (i.e. the command was actually dispatched).
    /// Returns `false` when the action does not map to an executor command or
    /// when no executor channel is connected (standalone TUI).
    pub(super) fn send_tui_command_for_confirm(&self, action: &ConfirmAction) -> bool {
        use crate::execution_control::ExecutionCommandKind;

        let (kind, plan_id, task_id) = match action {
            ConfirmAction::SoftRetryPlan(plan_id) => {
                (ExecutionCommandKind::SoftRetry, Some(plan_id.clone()), None)
            }
            ConfirmAction::RepairPlanPreserve(plan_id) => (
                ExecutionCommandKind::Repair {
                    preserve_completed: true,
                },
                Some(plan_id.clone()),
                None,
            ),
            ConfirmAction::RepairPlanClean(plan_id) => (
                ExecutionCommandKind::Repair {
                    preserve_completed: false,
                },
                Some(plan_id.clone()),
                None,
            ),
            ConfirmAction::ReverifyPlan(plan_id) => (
                ExecutionCommandKind::ReverifyGates,
                Some(plan_id.clone()),
                None,
            ),
            ConfirmAction::ForceAdvance(plan_id) => {
                let task_id = self
                    .tui_state
                    .plans
                    .get(self.tui_state.selected_plan_idx)
                    .and_then(|p| {
                        p.tasks
                            .iter()
                            .find(|t| t.status == TaskRowStatus::Failed)
                            .map(|t| t.id.clone())
                    })
                    .unwrap_or_default();
                (
                    ExecutionCommandKind::Skip,
                    Some(plan_id.clone()),
                    Some(task_id),
                )
            }
            ConfirmAction::ResetSelectedPlan(plan_id) => {
                (ExecutionCommandKind::Cancel, Some(plan_id.clone()), None)
            }
            // Other confirm actions don't map to executor commands.
            _ => return false,
        };
        if let Some(sender) = &self.exec_cmd_sender {
            let cmd = sender.build_command(kind, plan_id, task_id, None);
            let _ = sender.try_send(cmd);
            true
        } else {
            // P1.1: no executor channel — caller will show feedback.
            false
        }
    }


}

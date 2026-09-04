//! Runner-v2 execution command adapter (#233 + #255).
//!
//! This module bridges the executor-neutral `ExecutionCommand` transport
//! into the existing runner-v2 event-loop branches. It delegates to the
//! same scheduling logic the legacy `TuiCommand` match arms used, without
//! changing any scheduling semantics.
//!
//! #255 added `Approve`, `RejectApproval`, and `Reset` handling.

use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::execution_control::{
    CommandAck, CommandAckStatus, ExecutionCommand, ExecutionCommandKind, ack_for,
};

/// Outcome of processing one `ExecutionCommand` through the runner adapter.
///
/// The caller uses this to decide whether to flip `control_paused`, cancel
/// the run, or log a rejection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnerCommandEffect {
    /// The runner should set `control_paused = true`.
    Pause,
    /// The runner should set `control_paused = false`.
    Resume,
    /// The runner should cancel the current run via `CancellationToken`.
    Cancel { plan_id: Option<String> },
    /// Re-queue all failed tasks in the plan for retry from scratch.
    SoftRetry { plan_id: String },
    /// Skip a specific task and advance the plan past it.
    Skip { plan_id: String, task_id: String },
    /// Repair a plan: re-run failed/pending tasks (optionally preserve completed).
    Repair {
        plan_id: String,
        preserve_completed: bool,
    },
    /// Re-run the gate pipeline on a plan's tasks without re-executing them.
    ReverifyGates { plan_id: String },
    /// An approval was resolved. The runner should unblock the pending
    /// approval gate for the given approval ID.
    ApprovalResolved {
        approval_id: String,
        approved: bool,
        reason: Option<String>,
    },
    /// The runner should reset eligible graph state using receipt-preserving
    /// rules (committed receipts are never erased).
    Reset,
    /// The command was accepted but the operation is not yet implemented.
    /// The runner should log a rejection to the TUI bridge.
    NotImplemented {
        kind: ExecutionCommandKind,
        message: String,
    },
    /// The command was rejected (e.g. stale run ID).
    Rejected { message: String },
}

/// The runner-v2 adapter for `ExecutionCommand` processing.
///
/// Receives commands from the command channel, validates the run ID,
/// and returns the effect the event loop should apply. Also sends an
/// acknowledgement back through the ack channel.
pub struct RunnerExecutionCommandAdapter {
    run_id: String,
    ack_tx: mpsc::Sender<CommandAck>,
}

impl RunnerExecutionCommandAdapter {
    /// Create a new adapter for the given run.
    pub fn new(run_id: impl Into<String>, ack_tx: mpsc::Sender<CommandAck>) -> Self {
        Self {
            run_id: run_id.into(),
            ack_tx,
        }
    }

    /// Process a single `ExecutionCommand` and return the effect the event
    /// loop should apply. Also sends an `CommandAck` through the ack channel.
    ///
    /// Commands with a run ID different from the adapter's run ID are
    /// rejected with `"stale run"`.
    pub async fn process(&self, cmd: &ExecutionCommand) -> RunnerCommandEffect {
        // ── Run ID validation ───────────────────────────────────────
        if cmd.run_id != self.run_id {
            warn!(
                command_id = %cmd.command_id,
                cmd_run_id = %cmd.run_id,
                adapter_run_id = %self.run_id,
                "execution command rejected: stale run"
            );
            let ack = ack_for(cmd, CommandAckStatus::Rejected, Some("stale run".into()));
            let _ = self.ack_tx.send(ack).await;
            return RunnerCommandEffect::Rejected {
                message: "stale run".into(),
            };
        }

        // ── Dispatch by kind ────────────────────────────────────────
        let (effect, ack_status, ack_msg) = match &cmd.kind {
            ExecutionCommandKind::Pause => {
                info!(command_id = %cmd.command_id, "execution command: pause");
                (
                    RunnerCommandEffect::Pause,
                    CommandAckStatus::Completed,
                    None,
                )
            }
            ExecutionCommandKind::Resume => {
                info!(command_id = %cmd.command_id, "execution command: resume");
                (
                    RunnerCommandEffect::Resume,
                    CommandAckStatus::Completed,
                    None,
                )
            }
            ExecutionCommandKind::Cancel => {
                let plan_id = cmd.plan_id.clone();
                warn!(
                    command_id = %cmd.command_id,
                    plan_id = ?plan_id,
                    "execution command: cancel"
                );
                (
                    RunnerCommandEffect::Cancel { plan_id },
                    CommandAckStatus::Completed,
                    None,
                )
            }
            ExecutionCommandKind::SoftRetry => {
                let plan_id = cmd.plan_id.clone().unwrap_or_default();
                if plan_id.is_empty() {
                    warn!(
                        command_id = %cmd.command_id,
                        "execution command rejected: soft retry requires a plan_id"
                    );
                    let msg = "Soft retry requires a plan ID";
                    (
                        RunnerCommandEffect::Rejected {
                            message: msg.to_string(),
                        },
                        CommandAckStatus::Rejected,
                        Some(msg.to_string()),
                    )
                } else {
                    info!(
                        command_id = %cmd.command_id,
                        plan_id = %plan_id,
                        "execution command: soft retry"
                    );
                    (
                        RunnerCommandEffect::SoftRetry { plan_id },
                        CommandAckStatus::Accepted,
                        None,
                    )
                }
            }
            ExecutionCommandKind::Repair { preserve_completed } => {
                let plan_id = cmd.plan_id.clone().unwrap_or_default();
                if plan_id.is_empty() {
                    warn!(
                        command_id = %cmd.command_id,
                        "execution command rejected: repair requires a plan_id"
                    );
                    let msg = "Repair requires a plan ID";
                    (
                        RunnerCommandEffect::Rejected {
                            message: msg.to_string(),
                        },
                        CommandAckStatus::Rejected,
                        Some(msg.to_string()),
                    )
                } else {
                    info!(
                        command_id = %cmd.command_id,
                        plan_id = %plan_id,
                        preserve_completed,
                        "execution command: repair"
                    );
                    (
                        RunnerCommandEffect::Repair {
                            plan_id,
                            preserve_completed: *preserve_completed,
                        },
                        CommandAckStatus::Accepted,
                        None,
                    )
                }
            }
            ExecutionCommandKind::ReverifyGates => {
                let plan_id = cmd.plan_id.clone().unwrap_or_default();
                if plan_id.is_empty() {
                    warn!(
                        command_id = %cmd.command_id,
                        "execution command rejected: gate reverify requires a plan_id"
                    );
                    let msg = "Gate reverify requires a plan ID";
                    (
                        RunnerCommandEffect::Rejected {
                            message: msg.to_string(),
                        },
                        CommandAckStatus::Rejected,
                        Some(msg.to_string()),
                    )
                } else {
                    info!(
                        command_id = %cmd.command_id,
                        plan_id = %plan_id,
                        "execution command: reverify gates"
                    );
                    (
                        RunnerCommandEffect::ReverifyGates { plan_id },
                        CommandAckStatus::Accepted,
                        None,
                    )
                }
            }
            ExecutionCommandKind::Skip => {
                let plan_id = cmd.plan_id.clone().unwrap_or_default();
                let task_id = cmd.task_id.clone().unwrap_or_default();
                if plan_id.is_empty() || task_id.is_empty() {
                    warn!(
                        command_id = %cmd.command_id,
                        "execution command rejected: skip requires both plan_id and task_id"
                    );
                    let msg = "Skip requires both a plan ID and a task ID";
                    (
                        RunnerCommandEffect::Rejected {
                            message: msg.to_string(),
                        },
                        CommandAckStatus::Rejected,
                        Some(msg.to_string()),
                    )
                } else {
                    info!(
                        command_id = %cmd.command_id,
                        plan_id = %plan_id,
                        task_id = %task_id,
                        "execution command: skip task"
                    );
                    (
                        RunnerCommandEffect::Skip { plan_id, task_id },
                        CommandAckStatus::Accepted,
                        None,
                    )
                }
            }
            ExecutionCommandKind::Approve { approval_id } => {
                info!(
                    command_id = %cmd.command_id,
                    approval_id = %approval_id,
                    "execution command: approve"
                );
                (
                    RunnerCommandEffect::ApprovalResolved {
                        approval_id: approval_id.clone(),
                        approved: true,
                        reason: None,
                    },
                    CommandAckStatus::Completed,
                    None,
                )
            }
            ExecutionCommandKind::RejectApproval {
                approval_id,
                reason,
            } => {
                warn!(
                    command_id = %cmd.command_id,
                    approval_id = %approval_id,
                    reason = %reason,
                    "execution command: reject approval"
                );
                (
                    RunnerCommandEffect::ApprovalResolved {
                        approval_id: approval_id.clone(),
                        approved: false,
                        reason: Some(reason.clone()),
                    },
                    CommandAckStatus::Completed,
                    None,
                )
            }
            ExecutionCommandKind::Reset => {
                info!(command_id = %cmd.command_id, "execution command: reset");
                (
                    RunnerCommandEffect::Reset,
                    CommandAckStatus::Completed,
                    None,
                )
            }
        };

        let ack = ack_for(cmd, ack_status, ack_msg);
        let _ = self.ack_tx.send(ack).await;
        effect
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_control::ExecutionCommandSender;

    #[tokio::test]
    async fn tui_command_pause_resume_roundtrip() {
        let (sender, mut cmd_rx, ack_tx, ack_rx) = ExecutionCommandSender::channel("run-adapter");
        let adapter = RunnerExecutionCommandAdapter::new("run-adapter", ack_tx);
        let mut ack_receiver = crate::execution_control::CommandAckReceiver::new(ack_rx);

        // Pause
        let cmd = sender.build_command(ExecutionCommandKind::Pause, None, None, None);
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received).await;
        assert_eq!(effect, RunnerCommandEffect::Pause);

        // Resume
        let cmd = sender.build_command(ExecutionCommandKind::Resume, None, None, None);
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received).await;
        assert_eq!(effect, RunnerCommandEffect::Resume);

        let acks = ack_receiver.drain();
        assert_eq!(acks.len(), 2);
        assert!(acks.iter().all(|a| a.status == CommandAckStatus::Completed));
    }

    #[tokio::test]
    async fn tui_command_cancel_roundtrip() {
        let (sender, mut cmd_rx, ack_tx, ack_rx) = ExecutionCommandSender::channel("run-cancel");
        let adapter = RunnerExecutionCommandAdapter::new("run-cancel", ack_tx);
        let mut ack_receiver = crate::execution_control::CommandAckReceiver::new(ack_rx);

        let cmd = sender.build_command(
            ExecutionCommandKind::Cancel,
            Some("plan-abc".into()),
            None,
            None,
        );
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received).await;
        assert_eq!(
            effect,
            RunnerCommandEffect::Cancel {
                plan_id: Some("plan-abc".into())
            }
        );

        let acks = ack_receiver.drain();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, CommandAckStatus::Completed);
    }

    #[tokio::test]
    async fn tui_command_stale_run_rejected() {
        let (sender, mut cmd_rx, ack_tx, ack_rx) = ExecutionCommandSender::channel("run-old");
        let adapter = RunnerExecutionCommandAdapter::new("run-current", ack_tx);
        let mut ack_receiver = crate::execution_control::CommandAckReceiver::new(ack_rx);

        let cmd = sender.build_command(ExecutionCommandKind::Pause, None, None, None);
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received).await;
        assert_eq!(
            effect,
            RunnerCommandEffect::Rejected {
                message: "stale run".into()
            }
        );

        let acks = ack_receiver.drain();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, CommandAckStatus::Rejected);
        assert_eq!(acks[0].message.as_deref(), Some("stale run"));
    }

    #[tokio::test]
    async fn tui_command_recovery_kinds_accepted() {
        let (sender, mut cmd_rx, ack_tx, ack_rx) = ExecutionCommandSender::channel("run-recov");
        let adapter = RunnerExecutionCommandAdapter::new("run-recov", ack_tx);
        let mut ack_receiver = crate::execution_control::CommandAckReceiver::new(ack_rx);

        // SoftRetry with plan_id
        let cmd = sender.build_command(
            ExecutionCommandKind::SoftRetry,
            Some("plan-x".into()),
            None,
            None,
        );
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received).await;
        assert_eq!(
            effect,
            RunnerCommandEffect::SoftRetry {
                plan_id: "plan-x".into()
            }
        );

        // Skip with plan_id + task_id
        let cmd = sender.build_command(
            ExecutionCommandKind::Skip,
            Some("plan-x".into()),
            Some("task-y".into()),
            None,
        );
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received).await;
        assert_eq!(
            effect,
            RunnerCommandEffect::Skip {
                plan_id: "plan-x".into(),
                task_id: "task-y".into(),
            }
        );

        // Repair with plan_id
        let cmd = sender.build_command(
            ExecutionCommandKind::Repair {
                preserve_completed: true,
            },
            Some("plan-x".into()),
            None,
            None,
        );
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received).await;
        assert_eq!(
            effect,
            RunnerCommandEffect::Repair {
                plan_id: "plan-x".into(),
                preserve_completed: true,
            }
        );

        // ReverifyGates with plan_id
        let cmd = sender.build_command(
            ExecutionCommandKind::ReverifyGates,
            Some("plan-x".into()),
            None,
            None,
        );
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received).await;
        assert_eq!(
            effect,
            RunnerCommandEffect::ReverifyGates {
                plan_id: "plan-x".into()
            }
        );

        let acks = ack_receiver.drain();
        assert_eq!(acks.len(), 4);
        assert!(
            acks.iter().all(|a| a.status == CommandAckStatus::Accepted),
            "all recovery commands should be accepted"
        );
    }

    #[tokio::test]
    async fn tui_command_recovery_kinds_rejected_without_ids() {
        let (sender, mut cmd_rx, ack_tx, ack_rx) = ExecutionCommandSender::channel("run-noid");
        let adapter = RunnerExecutionCommandAdapter::new("run-noid", ack_tx);
        let mut ack_receiver = crate::execution_control::CommandAckReceiver::new(ack_rx);

        // SoftRetry without plan_id → rejected
        let cmd = sender.build_command(ExecutionCommandKind::SoftRetry, None, None, None);
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received).await;
        assert!(
            matches!(effect, RunnerCommandEffect::Rejected { .. }),
            "SoftRetry without plan_id should be rejected"
        );

        // Skip without task_id → rejected
        let cmd = sender.build_command(
            ExecutionCommandKind::Skip,
            Some("plan-x".into()),
            None,
            None,
        );
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received).await;
        assert!(
            matches!(effect, RunnerCommandEffect::Rejected { .. }),
            "Skip without task_id should be rejected"
        );

        let acks = ack_receiver.drain();
        assert_eq!(acks.len(), 2);
        assert!(acks.iter().all(|a| a.status == CommandAckStatus::Rejected));
    }

    #[tokio::test]
    async fn tui_command_approve_roundtrip() {
        let (sender, mut cmd_rx, ack_tx, ack_rx) = ExecutionCommandSender::channel("run-approve");
        let adapter = RunnerExecutionCommandAdapter::new("run-approve", ack_tx);
        let mut ack_receiver = crate::execution_control::CommandAckReceiver::new(ack_rx);

        let cmd = sender.build_command(
            ExecutionCommandKind::Approve {
                approval_id: "ap-42".into(),
            },
            Some("plan-1".into()),
            Some("task-1".into()),
            None,
        );
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received).await;
        assert_eq!(
            effect,
            RunnerCommandEffect::ApprovalResolved {
                approval_id: "ap-42".into(),
                approved: true,
                reason: None,
            }
        );

        let acks = ack_receiver.drain();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, CommandAckStatus::Completed);
    }

    #[tokio::test]
    async fn tui_command_reject_approval_roundtrip() {
        let (sender, mut cmd_rx, ack_tx, ack_rx) = ExecutionCommandSender::channel("run-reject");
        let adapter = RunnerExecutionCommandAdapter::new("run-reject", ack_tx);
        let mut ack_receiver = crate::execution_control::CommandAckReceiver::new(ack_rx);

        let cmd = sender.build_command(
            ExecutionCommandKind::RejectApproval {
                approval_id: "ap-99".into(),
                reason: "unsafe tool call".into(),
            },
            Some("plan-1".into()),
            None,
            None,
        );
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received).await;
        assert_eq!(
            effect,
            RunnerCommandEffect::ApprovalResolved {
                approval_id: "ap-99".into(),
                approved: false,
                reason: Some("unsafe tool call".into()),
            }
        );

        let acks = ack_receiver.drain();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, CommandAckStatus::Completed);
    }

    #[tokio::test]
    async fn tui_command_reset_roundtrip() {
        let (sender, mut cmd_rx, ack_tx, ack_rx) = ExecutionCommandSender::channel("run-reset");
        let adapter = RunnerExecutionCommandAdapter::new("run-reset", ack_tx);
        let mut ack_receiver = crate::execution_control::CommandAckReceiver::new(ack_rx);

        let cmd = sender.build_command(ExecutionCommandKind::Reset, None, None, None);
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received).await;
        assert_eq!(effect, RunnerCommandEffect::Reset);

        let acks = ack_receiver.drain();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, CommandAckStatus::Completed);
    }

    #[tokio::test]
    async fn tui_command_all_ten_variants_produce_effects() {
        let (sender, mut cmd_rx, ack_tx, _ack_rx) = ExecutionCommandSender::channel("run-ten");
        let adapter = RunnerExecutionCommandAdapter::new("run-ten", ack_tx);

        let kinds = vec![
            ExecutionCommandKind::Pause,
            ExecutionCommandKind::Resume,
            ExecutionCommandKind::SoftRetry,
            ExecutionCommandKind::Repair {
                preserve_completed: false,
            },
            ExecutionCommandKind::ReverifyGates,
            ExecutionCommandKind::Skip,
            ExecutionCommandKind::Cancel,
            ExecutionCommandKind::Approve {
                approval_id: "ap-1".into(),
            },
            ExecutionCommandKind::RejectApproval {
                approval_id: "ap-2".into(),
                reason: "test".into(),
            },
            ExecutionCommandKind::Reset,
        ];

        let mut effects = Vec::new();
        for kind in kinds {
            let cmd = sender.build_command(kind, Some("p".into()), Some("t".into()), None);
            sender.try_send(cmd).unwrap();
            let received = cmd_rx.recv().await.unwrap();
            effects.push(adapter.process(&received).await);
        }

        assert_eq!(effects.len(), 10);
        assert_eq!(effects[0], RunnerCommandEffect::Pause);
        assert_eq!(effects[1], RunnerCommandEffect::Resume);
        assert!(
            matches!(effects[2], RunnerCommandEffect::SoftRetry { .. }),
            "SoftRetry should produce SoftRetry effect"
        );
        assert!(
            matches!(effects[3], RunnerCommandEffect::Repair { .. }),
            "Repair should produce Repair effect"
        );
        assert!(
            matches!(effects[4], RunnerCommandEffect::ReverifyGates { .. }),
            "ReverifyGates should produce ReverifyGates effect"
        );
        assert!(
            matches!(effects[5], RunnerCommandEffect::Skip { .. }),
            "Skip should produce Skip effect"
        );
        assert!(matches!(effects[6], RunnerCommandEffect::Cancel { .. }));
        assert!(matches!(
            effects[7],
            RunnerCommandEffect::ApprovalResolved { approved: true, .. }
        ));
        assert!(matches!(
            effects[8],
            RunnerCommandEffect::ApprovalResolved {
                approved: false,
                ..
            }
        ));
        assert_eq!(effects[9], RunnerCommandEffect::Reset);
    }
}

//! Graph execution control adapter (#255).
//!
//! This module bridges the CLI-layer `ExecutionCommand` transport into the
//! graph-layer `ExecutionControlService`. It translates
//! `ExecutionCommandKind` (roko-cli) into `ControlCommandKind` (roko-graph)
//! and returns `CommandAck` from durable control receipts.
//!
//! # Scope boundary
//!
//! This adapter owns command translation and receipt-to-ack mapping. The
//! graph-layer service owns approval/receipt state. The outer controller
//! (#256/#257) alone flushes the final checkpoint and writes run terminal
//! state. The adapter never independently writes terminal state or releases
//! a #249 lease.

use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::execution_control::{
    CommandAck, CommandAckStatus, ExecutionCommand, ExecutionCommandKind, ack_for,
};

use roko_graph::control::{
    ControlCommandKind, ControlEffect, ControlReceiptV1, ExecutionControlService, ReceiptStatus,
};

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

/// Bridges the CLI `ExecutionCommand` channel to the graph-layer
/// `ExecutionControlService`.
///
/// For each command received, the adapter:
/// 1. Maps `ExecutionCommandKind` -> `ControlCommandKind`.
/// 2. Calls `ExecutionControlService::process_command`.
/// 3. Maps the resulting `ControlEffect` into a `GraphControlAdapterEffect`.
/// 4. Sends a `CommandAck` back through the ack channel, derived from the
///    durable control receipt.
pub struct GraphControlAdapter {
    service: ExecutionControlService,
    ack_tx: mpsc::Sender<CommandAck>,
}

impl GraphControlAdapter {
    /// Create a new adapter backed by the given control service and ack
    /// channel.
    pub fn new(service: ExecutionControlService, ack_tx: mpsc::Sender<CommandAck>) -> Self {
        Self { service, ack_tx }
    }

    /// Borrow the underlying control service (e.g. for approval registration).
    #[must_use]
    pub fn service(&self) -> &ExecutionControlService {
        &self.service
    }

    /// Process a single `ExecutionCommand` and return the scheduling effect.
    /// Also sends a `CommandAck` through the ack channel.
    pub async fn process(&self, cmd: &ExecutionCommand) -> GraphControlAdapterEffect {
        let kind = map_command_kind(&cmd.kind);
        let effect = self.service.process_command(
            &cmd.command_id,
            &cmd.correlation_id,
            &cmd.run_id,
            cmd.plan_id.clone(),
            cmd.task_id.clone(),
            &kind,
        );

        let (adapter_effect, ack_status, ack_msg) = match &effect {
            ControlEffect::Pause => {
                info!(command_id = %cmd.command_id, "graph control: pause");
                (
                    GraphControlAdapterEffect::Pause,
                    CommandAckStatus::Completed,
                    None,
                )
            }
            ControlEffect::Resume => {
                info!(command_id = %cmd.command_id, "graph control: resume");
                (
                    GraphControlAdapterEffect::Resume,
                    CommandAckStatus::Completed,
                    None,
                )
            }
            ControlEffect::SoftRetry { plan_id, task_id } => {
                info!(
                    command_id = %cmd.command_id,
                    plan_id = ?plan_id,
                    task_id = ?task_id,
                    "graph control: soft retry"
                );
                (
                    GraphControlAdapterEffect::SoftRetry {
                        plan_id: plan_id.clone(),
                        task_id: task_id.clone(),
                    },
                    CommandAckStatus::Completed,
                    None,
                )
            }
            ControlEffect::Repair { preserve_completed } => {
                info!(
                    command_id = %cmd.command_id,
                    preserve_completed,
                    "graph control: repair"
                );
                (
                    GraphControlAdapterEffect::Repair {
                        preserve_completed: *preserve_completed,
                    },
                    CommandAckStatus::Completed,
                    None,
                )
            }
            ControlEffect::ReverifyGates { plan_id, task_id } => {
                info!(
                    command_id = %cmd.command_id,
                    plan_id = ?plan_id,
                    task_id = ?task_id,
                    "graph control: reverify gates"
                );
                (
                    GraphControlAdapterEffect::ReverifyGates {
                        plan_id: plan_id.clone(),
                        task_id: task_id.clone(),
                    },
                    CommandAckStatus::Completed,
                    None,
                )
            }
            ControlEffect::Skip { plan_id, task_id } => {
                info!(
                    command_id = %cmd.command_id,
                    plan_id = ?plan_id,
                    task_id = ?task_id,
                    "graph control: skip"
                );
                (
                    GraphControlAdapterEffect::Skip {
                        plan_id: plan_id.clone(),
                        task_id: task_id.clone(),
                    },
                    CommandAckStatus::Completed,
                    None,
                )
            }
            ControlEffect::Reset => {
                info!(command_id = %cmd.command_id, "graph control: reset");
                (
                    GraphControlAdapterEffect::Reset,
                    CommandAckStatus::Completed,
                    None,
                )
            }
            ControlEffect::ApprovalResolved {
                approval_id,
                resolution,
            } => {
                let approved = resolution.is_approved();
                info!(
                    command_id = %cmd.command_id,
                    approval_id = %approval_id,
                    approved,
                    "graph control: approval resolved"
                );
                (
                    GraphControlAdapterEffect::ApprovalResolved {
                        approval_id: approval_id.clone(),
                        approved,
                    },
                    CommandAckStatus::Completed,
                    None,
                )
            }
            ControlEffect::Cancel { plan_id, intent } => {
                warn!(
                    command_id = %cmd.command_id,
                    plan_id = ?plan_id,
                    "graph control: cancel with finalization intent"
                );
                (
                    GraphControlAdapterEffect::Cancel {
                        plan_id: plan_id.clone(),
                        finalization_receipt: intent.receipt.clone(),
                    },
                    CommandAckStatus::Completed,
                    Some("cancellation initiated".to_string()),
                )
            }
            ControlEffect::Rejected { reason, receipt } => {
                warn!(
                    command_id = %cmd.command_id,
                    reason = %reason,
                    receipt_status = %receipt.status,
                    "graph control: command rejected"
                );
                (
                    GraphControlAdapterEffect::Rejected {
                        reason: reason.clone(),
                    },
                    CommandAckStatus::Rejected,
                    Some(reason.clone()),
                )
            }
        };

        let ack = ack_for(cmd, ack_status, ack_msg);
        let _ = self.ack_tx.send(ack).await;

        adapter_effect
    }
}

// ---------------------------------------------------------------------------
// Adapter effect
// ---------------------------------------------------------------------------

/// The scheduling effect returned by the graph control adapter.
///
/// The caller (graph scheduler) applies these at safe scheduler boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphControlAdapterEffect {
    /// Set paused; allow current in-flight to finish.
    Pause,
    /// Clear paused.
    Resume,
    /// Create a new task attempt.
    SoftRetry {
        plan_id: Option<String>,
        task_id: Option<String>,
    },
    /// Reset failed/pending nodes.
    Repair { preserve_completed: bool },
    /// Re-run gate checks without a provider call.
    ReverifyGates {
        plan_id: Option<String>,
        task_id: Option<String>,
    },
    /// Mark a not-started task as skipped.
    Skip {
        plan_id: Option<String>,
        task_id: Option<String>,
    },
    /// Reset eligible graph state.
    Reset,
    /// An approval was resolved.
    ApprovalResolved { approval_id: String, approved: bool },
    /// Cancel with finalization receipt.
    Cancel {
        plan_id: Option<String>,
        finalization_receipt: ControlReceiptV1,
    },
    /// The command was rejected.
    Rejected { reason: String },
}

// ---------------------------------------------------------------------------
// Command kind mapping
// ---------------------------------------------------------------------------

/// Map CLI-layer `ExecutionCommandKind` to graph-layer `ControlCommandKind`.
fn map_command_kind(kind: &ExecutionCommandKind) -> ControlCommandKind {
    match kind {
        ExecutionCommandKind::Pause => ControlCommandKind::Pause,
        ExecutionCommandKind::Resume => ControlCommandKind::Resume,
        ExecutionCommandKind::SoftRetry => ControlCommandKind::SoftRetry,
        ExecutionCommandKind::Repair { preserve_completed } => ControlCommandKind::Repair {
            preserve_completed: *preserve_completed,
        },
        ExecutionCommandKind::ReverifyGates => ControlCommandKind::ReverifyGates,
        ExecutionCommandKind::Skip => ControlCommandKind::Skip,
        ExecutionCommandKind::Cancel => ControlCommandKind::Cancel,
        ExecutionCommandKind::Approve { approval_id } => ControlCommandKind::Approve {
            approval_id: approval_id.clone(),
        },
        ExecutionCommandKind::RejectApproval {
            approval_id,
            reason,
        } => ControlCommandKind::RejectApproval {
            approval_id: approval_id.clone(),
            reason: reason.clone(),
        },
        ExecutionCommandKind::Reset => ControlCommandKind::Reset,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_control::{CommandAckReceiver, ExecutionCommandSender};
    use roko_graph::control::ApprovalRequestV1;

    fn make_adapter(
        run_id: &str,
    ) -> (
        ExecutionCommandSender,
        mpsc::Receiver<ExecutionCommand>,
        GraphControlAdapter,
        CommandAckReceiver,
    ) {
        let (sender, cmd_rx, ack_tx, ack_rx) = ExecutionCommandSender::channel(run_id);
        let service = ExecutionControlService::new(run_id);
        let adapter = GraphControlAdapter::new(service, ack_tx);
        let ack_receiver = CommandAckReceiver::new(ack_rx);
        (sender, cmd_rx, adapter, ack_receiver)
    }

    fn test_approval(id: &str) -> ApprovalRequestV1 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        ApprovalRequestV1 {
            approval_id: id.to_string(),
            run_id: "run-graph".to_string(),
            plan_id: "plan-1".to_string(),
            task_id: "task-1".to_string(),
            node_id: "node-1".to_string(),
            attempt: 0,
            capability_summary: "file_write".to_string(),
            tool_summary: "write test.rs".to_string(),
            deadline_ms: now + 300_000,
            fingerprint: "fp-test".to_string(),
            created_at_ms: now,
        }
    }

    #[tokio::test]
    async fn pause_resume_roundtrip() {
        let (sender, mut cmd_rx, adapter, mut ack_receiver) = make_adapter("run-graph");

        // Pause
        let cmd = sender.build_command(ExecutionCommandKind::Pause, None, None, None);
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received).await;
        assert_eq!(effect, GraphControlAdapterEffect::Pause);

        // Resume
        let cmd = sender.build_command(ExecutionCommandKind::Resume, None, None, None);
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received).await;
        assert_eq!(effect, GraphControlAdapterEffect::Resume);

        let acks = ack_receiver.drain();
        assert_eq!(acks.len(), 2);
        assert!(acks.iter().all(|a| a.status == CommandAckStatus::Completed));
    }

    #[tokio::test]
    async fn approve_with_registered_approval() {
        let (sender, mut cmd_rx, adapter, mut ack_receiver) = make_adapter("run-graph");
        adapter.service().register_approval(test_approval("ap-ga"));

        let cmd = sender.build_command(
            ExecutionCommandKind::Approve {
                approval_id: "ap-ga".into(),
            },
            Some("plan-1".into()),
            Some("task-1".into()),
            None,
        );
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received).await;
        assert!(matches!(
            effect,
            GraphControlAdapterEffect::ApprovalResolved { approved: true, .. }
        ));

        let acks = ack_receiver.drain();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, CommandAckStatus::Completed);
    }

    #[tokio::test]
    async fn reject_approval_produces_rejection_effect() {
        let (sender, mut cmd_rx, adapter, mut ack_receiver) = make_adapter("run-graph");
        adapter
            .service()
            .register_approval(test_approval("ap-rej-ga"));

        let cmd = sender.build_command(
            ExecutionCommandKind::RejectApproval {
                approval_id: "ap-rej-ga".into(),
                reason: "unsafe tool".into(),
            },
            None,
            None,
            None,
        );
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received).await;
        assert!(matches!(
            effect,
            GraphControlAdapterEffect::ApprovalResolved {
                approved: false,
                ..
            }
        ));

        let acks = ack_receiver.drain();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, CommandAckStatus::Completed);
    }

    #[tokio::test]
    async fn approve_without_registration_rejected() {
        let (_sender, _cmd_rx, adapter, mut ack_receiver) = make_adapter("run-graph");

        let cmd = ExecutionCommand {
            command_id: "cmd-no-reg".into(),
            correlation_id: "cor-no-reg".into(),
            run_id: "run-graph".into(),
            plan_id: None,
            task_id: None,
            attempt: None,
            issued_at_ms: 1000,
            kind: ExecutionCommandKind::Approve {
                approval_id: "ap-missing".into(),
            },
        };
        let effect = adapter.process(&cmd).await;
        assert!(matches!(effect, GraphControlAdapterEffect::Rejected { .. }));

        let acks = ack_receiver.drain();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, CommandAckStatus::Rejected);
    }

    #[tokio::test]
    async fn cancel_produces_finalization_receipt() {
        let (sender, mut cmd_rx, adapter, mut ack_receiver) = make_adapter("run-graph");

        let cmd = sender.build_command(
            ExecutionCommandKind::Cancel,
            Some("plan-1".into()),
            None,
            None,
        );
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received).await;
        match effect {
            GraphControlAdapterEffect::Cancel {
                plan_id,
                finalization_receipt,
            } => {
                assert_eq!(plan_id.as_deref(), Some("plan-1"));
                assert_eq!(finalization_receipt.status, ReceiptStatus::Finalized);
            }
            other => panic!("expected Cancel, got {other:?}"),
        }

        let acks = ack_receiver.drain();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, CommandAckStatus::Completed);
    }

    #[tokio::test]
    async fn reset_roundtrip() {
        let (sender, mut cmd_rx, adapter, mut ack_receiver) = make_adapter("run-graph");

        let cmd = sender.build_command(ExecutionCommandKind::Reset, None, None, None);
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received).await;
        assert_eq!(effect, GraphControlAdapterEffect::Reset);

        let acks = ack_receiver.drain();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, CommandAckStatus::Completed);
    }

    #[tokio::test]
    async fn stale_run_rejected() {
        let (_sender, _cmd_rx, adapter, mut ack_receiver) = make_adapter("run-graph");

        let cmd = ExecutionCommand {
            command_id: "cmd-stale".into(),
            correlation_id: "cor-stale".into(),
            run_id: "run-old".into(),
            plan_id: None,
            task_id: None,
            attempt: None,
            issued_at_ms: 1000,
            kind: ExecutionCommandKind::Pause,
        };
        let effect = adapter.process(&cmd).await;
        assert!(matches!(
            effect,
            GraphControlAdapterEffect::Rejected { reason } if reason == "stale run"
        ));

        let acks = ack_receiver.drain();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, CommandAckStatus::Rejected);
    }

    #[tokio::test]
    async fn cancel_then_resume_rejected() {
        let (sender, mut cmd_rx, adapter, _ack_receiver) = make_adapter("run-graph");

        // Cancel first.
        let cmd = sender.build_command(ExecutionCommandKind::Cancel, None, None, None);
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        adapter.process(&received).await;

        // Resume should be rejected.
        let cmd = sender.build_command(ExecutionCommandKind::Resume, None, None, None);
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received).await;
        assert!(matches!(
            effect,
            GraphControlAdapterEffect::Rejected { reason } if reason.contains("cancellation")
        ));
    }

    #[tokio::test]
    async fn all_ten_command_kinds_produce_effects() {
        let (sender, mut cmd_rx, adapter, _ack_receiver) = make_adapter("run-graph");
        adapter
            .service()
            .register_approval(test_approval("ap-all-ga"));

        let kinds = vec![
            ExecutionCommandKind::Pause,
            ExecutionCommandKind::Resume,
            ExecutionCommandKind::SoftRetry,
            ExecutionCommandKind::Repair {
                preserve_completed: true,
            },
            ExecutionCommandKind::ReverifyGates,
            ExecutionCommandKind::Skip,
            ExecutionCommandKind::Reset,
            ExecutionCommandKind::Approve {
                approval_id: "ap-all-ga".into(),
            },
            ExecutionCommandKind::RejectApproval {
                approval_id: "ap-missing-ok".into(),
                reason: "test".into(),
            },
            ExecutionCommandKind::Cancel,
        ];

        let mut effects = Vec::new();
        for kind in kinds {
            let cmd = sender.build_command(kind, Some("p".into()), Some("t".into()), None);
            sender.try_send(cmd).unwrap();
            let received = cmd_rx.recv().await.unwrap();
            effects.push(adapter.process(&received).await);
        }

        assert_eq!(effects.len(), 10);
        assert_eq!(effects[0], GraphControlAdapterEffect::Pause);
        assert_eq!(effects[1], GraphControlAdapterEffect::Resume);
        assert!(matches!(
            effects[2],
            GraphControlAdapterEffect::SoftRetry { .. }
        ));
        assert!(matches!(
            effects[3],
            GraphControlAdapterEffect::Repair { .. }
        ));
        assert!(matches!(
            effects[4],
            GraphControlAdapterEffect::ReverifyGates { .. }
        ));
        assert!(matches!(effects[5], GraphControlAdapterEffect::Skip { .. }));
        assert_eq!(effects[6], GraphControlAdapterEffect::Reset);
        assert!(matches!(
            effects[7],
            GraphControlAdapterEffect::ApprovalResolved { approved: true, .. }
        ));
        // ap-missing-ok was never registered, so the reject-approval
        // command itself gets rejected by the service.
        assert!(matches!(
            effects[8],
            GraphControlAdapterEffect::Rejected { .. }
        ));
        assert!(matches!(
            effects[9],
            GraphControlAdapterEffect::Cancel { .. }
        ));
    }
}

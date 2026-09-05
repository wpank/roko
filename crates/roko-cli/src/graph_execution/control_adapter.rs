//! Graph execution control adapter (#255).
//!
//! This module bridges the executor-neutral `ExecutionCommand` transport
//! (from `execution_control.rs`) to the graph-layer `ExecutionControlService`
//! (from `roko_graph::control`). It performs the following responsibilities:
//!
//! 1. Maps `ExecutionCommandKind` -> `ControlCommandKind` for the graph layer.
//! 2. Delegates to `ExecutionControlService::process_command` for durable
//!    receipt tracking, approval lifecycle, and run-ID validation.
//! 3. Maps `ControlEffect` back to `CommandAck` for the TUI/CLI.
//! 4. On `Cancel`, coordinates with `ProcessSupervisor` for clean agent shutdown.
//! 5. Exposes shared `AtomicBool` flags for pause/cancel that the engine and
//!    cells can observe without holding the adapter.
//!
//! # Scope boundary
//!
//! This adapter owns command translation and process cancellation coordination.
//! The outer controller (#256/#257) alone flushes the final checkpoint, follows
//! #249 release policy, and writes run terminal state. This adapter never
//! independently writes terminal state or releases a #249 lease.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use roko_graph::control::{
    ControlCommandKind, ControlEffect, ControlReceiptV1, ExecutionControlService,
    FinalizationIntent, ReceiptStatus,
};
use roko_runtime::process::ProcessSupervisor;

use crate::execution_control::{
    CommandAck, CommandAckStatus, ExecutionCommand, ExecutionCommandKind, ack_for,
};

// ---------------------------------------------------------------------------
// Adapter effect
// ---------------------------------------------------------------------------

/// The scheduling effect produced by processing one `ExecutionCommand`.
///
/// The caller (graph host loop) uses this to decide next steps:
/// - `Paused`/`Resumed`: flip the scheduling flag
/// - `CancelComplete`: stop scheduling, return finalization intent
/// - `Rejected`/`CommandApplied`: log and continue
#[derive(Debug, Clone)]
pub enum GraphCommandEffect {
    /// The graph should pause scheduling. Current in-flight nodes finish.
    Paused,
    /// The graph should resume scheduling.
    Resumed,
    /// A cancel has been initiated and all processes shut down.
    CancelComplete {
        /// The finalization intent for the outer controller.
        intent: FinalizationIntent,
        /// Node IDs of cancelled nodes, for lease release.
        cancelled_nodes: Vec<String>,
    },
    /// A non-terminal command was applied (soft retry, repair, etc.).
    CommandApplied {
        /// The durable receipt from the control service.
        receipt: ControlReceiptV1,
        /// The underlying control effect for the caller to act on.
        effect: ControlEffect,
    },
    /// The command was rejected by the control service.
    Rejected {
        /// Human-readable rejection reason.
        reason: String,
        /// The durable receipt (if one was created).
        receipt: Option<ControlReceiptV1>,
    },
}

// ---------------------------------------------------------------------------
// GraphExecutionControlAdapter
// ---------------------------------------------------------------------------

/// Bridges `ExecutionCommand` (CLI/TUI transport) to the graph-layer
/// `ExecutionControlService` and coordinates process cancellation.
///
/// Holds shared flags that the engine and cells can observe via `CellContext`.
pub struct GraphExecutionControlAdapter {
    /// The graph-layer control service that manages approvals and receipts.
    control_service: Arc<ExecutionControlService>,
    /// ACK channel back to the TUI/CLI.
    ack_tx: mpsc::Sender<CommandAck>,
    /// Shared atomic pause flag. The engine checks this between nodes.
    pause_flag: Arc<AtomicBool>,
    /// Shared atomic cancel flag. The engine and cells check this.
    cancel_flag: Arc<AtomicBool>,
    /// Graph-level cancellation token (from `FlowHandle` or `start`).
    cancellation_token: CancellationToken,
    /// Process supervisor for shutting down running agents on cancel.
    process_supervisor: Option<Arc<ProcessSupervisor>>,
}

impl GraphExecutionControlAdapter {
    /// Create a new adapter.
    ///
    /// # Arguments
    ///
    /// - `control_service`: The graph-layer control service (manages approvals,
    ///   receipts, and run-ID validation).
    /// - `ack_tx`: The acknowledgement channel back to the TUI/CLI.
    /// - `cancellation_token`: The engine's cancellation token.
    /// - `process_supervisor`: Optional supervisor for clean agent shutdown.
    pub fn new(
        control_service: Arc<ExecutionControlService>,
        ack_tx: mpsc::Sender<CommandAck>,
        cancellation_token: CancellationToken,
        process_supervisor: Option<Arc<ProcessSupervisor>>,
    ) -> Self {
        Self {
            control_service,
            ack_tx,
            pause_flag: Arc::new(AtomicBool::new(false)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            cancellation_token,
            process_supervisor,
        }
    }

    /// Return a clone of the shared pause flag for wiring into `CellContext`.
    #[must_use]
    pub fn pause_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.pause_flag)
    }

    /// Return a clone of the shared cancel flag for wiring into `CellContext`.
    #[must_use]
    pub fn cancel_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancel_flag)
    }

    /// Whether the executor is currently paused.
    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.pause_flag.load(Ordering::Acquire)
    }

    /// Whether cancellation has been initiated.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Acquire)
    }

    /// Return a reference to the underlying control service.
    #[must_use]
    pub fn control_service(&self) -> &ExecutionControlService {
        &self.control_service
    }

    /// Process a single `ExecutionCommand`, returning the effect the graph
    /// host loop should apply.
    ///
    /// Also sends a `CommandAck` back through the ack channel.
    pub async fn process(
        &self,
        cmd: &ExecutionCommand,
        running_node_ids: &[String],
    ) -> GraphCommandEffect {
        // Map CLI command kind -> graph control command kind.
        let graph_kind = map_command_kind(&cmd.kind);

        // Delegate to the control service for validation, receipts, and effects.
        let effect = self.control_service.process_command(
            &cmd.command_id,
            &cmd.correlation_id,
            &cmd.run_id,
            cmd.plan_id.clone(),
            cmd.task_id.clone(),
            &graph_kind,
        );

        // Map the control effect to a graph command effect and send ack.
        match effect {
            ControlEffect::Pause => {
                self.pause_flag.store(true, Ordering::Release);
                info!(
                    command_id = %cmd.command_id,
                    "graph control: paused"
                );
                let ack = ack_for(cmd, CommandAckStatus::Completed, Some("paused".into()));
                let _ = self.ack_tx.send(ack).await;
                GraphCommandEffect::Paused
            }

            ControlEffect::Resume => {
                self.pause_flag.store(false, Ordering::Release);
                info!(
                    command_id = %cmd.command_id,
                    "graph control: resumed"
                );
                let ack = ack_for(cmd, CommandAckStatus::Completed, Some("resumed".into()));
                let _ = self.ack_tx.send(ack).await;
                GraphCommandEffect::Resumed
            }

            ControlEffect::Cancel { intent, .. } => {
                self.cancel_flag.store(true, Ordering::Release);
                self.cancellation_token.cancel();

                // Shut down all running agent processes via supervisor.
                if let Some(supervisor) = &self.process_supervisor {
                    let outcomes = supervisor.shutdown_all().await;
                    info!(
                        command_id = %cmd.command_id,
                        processes_stopped = outcomes.len(),
                        "graph control: cancelled, processes shut down"
                    );
                } else {
                    info!(
                        command_id = %cmd.command_id,
                        "graph control: cancelled (no supervisor)"
                    );
                }

                let cancelled_nodes = running_node_ids.to_vec();
                let ack = ack_for(
                    cmd,
                    CommandAckStatus::Completed,
                    Some("cancellation complete".into()),
                );
                let _ = self.ack_tx.send(ack).await;

                GraphCommandEffect::CancelComplete {
                    intent,
                    cancelled_nodes,
                }
            }

            ControlEffect::Rejected { reason, receipt } => {
                warn!(
                    command_id = %cmd.command_id,
                    reason = %reason,
                    "graph control: command rejected"
                );
                let ack = ack_for(cmd, CommandAckStatus::Rejected, Some(reason.clone()));
                let _ = self.ack_tx.send(ack).await;
                GraphCommandEffect::Rejected {
                    reason,
                    receipt: Some(receipt),
                }
            }

            // All other effects: applied commands.
            other => {
                let receipt = self
                    .control_service
                    .get_receipt(&cmd.command_id)
                    .unwrap_or_else(|| {
                        ControlReceiptV1::received(
                            &cmd.command_id,
                            &cmd.correlation_id,
                            &cmd.run_id,
                            cmd.plan_id.clone(),
                            cmd.task_id.clone(),
                            map_command_kind(&cmd.kind).label(),
                        )
                    });
                info!(
                    command_id = %cmd.command_id,
                    kind = %cmd.kind,
                    receipt_status = %receipt.status,
                    "graph control: command applied"
                );
                let ack = ack_for(cmd, CommandAckStatus::Completed, None);
                let _ = self.ack_tx.send(ack).await;
                GraphCommandEffect::CommandApplied {
                    receipt,
                    effect: other,
                }
            }
        }
    }
}

impl std::fmt::Debug for GraphExecutionControlAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphExecutionControlAdapter")
            .field("paused", &self.is_paused())
            .field("cancelled", &self.is_cancelled())
            .field("has_supervisor", &self.process_supervisor.is_some())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Command kind mapping
// ---------------------------------------------------------------------------

/// Map a CLI `ExecutionCommandKind` to a graph `ControlCommandKind`.
///
/// The two enums are intentionally separate to avoid a dependency from
/// `roko-graph` (layer 2) to `roko-cli` (layer 4).
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
    use roko_graph::control::{ApprovalResolution, build_approval_request};

    fn test_adapter(
        run_id: &str,
    ) -> (
        ExecutionCommandSender,
        mpsc::Receiver<ExecutionCommand>,
        GraphExecutionControlAdapter,
        CommandAckReceiver,
    ) {
        let (sender, cmd_rx, ack_tx, ack_rx) = ExecutionCommandSender::channel(run_id);
        let control_service = Arc::new(ExecutionControlService::new(run_id));
        let cancel_token = CancellationToken::new();
        let adapter = GraphExecutionControlAdapter::new(
            control_service,
            ack_tx,
            cancel_token,
            None, // no supervisor for tests
        );
        let ack_receiver = CommandAckReceiver::new(ack_rx);
        (sender, cmd_rx, adapter, ack_receiver)
    }

    #[tokio::test]
    async fn pause_sets_flag_and_acks() {
        let (sender, mut cmd_rx, adapter, mut ack_rx) = test_adapter("run-1");

        let cmd = sender.build_command(ExecutionCommandKind::Pause, None, None, None);
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();

        assert!(!adapter.is_paused());
        let effect = adapter.process(&received, &[]).await;
        assert!(adapter.is_paused());
        assert!(matches!(effect, GraphCommandEffect::Paused));

        let acks = ack_rx.drain();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, CommandAckStatus::Completed);
    }

    #[tokio::test]
    async fn resume_clears_flag_and_acks() {
        let (sender, mut cmd_rx, adapter, mut ack_rx) = test_adapter("run-2");

        // Pause first.
        let cmd = sender.build_command(ExecutionCommandKind::Pause, None, None, None);
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        adapter.process(&received, &[]).await;
        assert!(adapter.is_paused());

        // Resume.
        let cmd = sender.build_command(ExecutionCommandKind::Resume, None, None, None);
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received, &[]).await;
        assert!(!adapter.is_paused());
        assert!(matches!(effect, GraphCommandEffect::Resumed));

        let acks = ack_rx.drain();
        assert_eq!(acks.len(), 2);
    }

    #[tokio::test]
    async fn cancel_sets_flags_and_returns_intent() {
        let (sender, mut cmd_rx, adapter, mut ack_rx) = test_adapter("run-3");

        let cmd = sender.build_command(
            ExecutionCommandKind::Cancel,
            Some("plan-1".into()),
            None,
            None,
        );
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();

        let running_nodes = vec!["node-a".to_string(), "node-b".to_string()];
        let effect = adapter.process(&received, &running_nodes).await;

        assert!(adapter.is_cancelled());
        match effect {
            GraphCommandEffect::CancelComplete {
                intent,
                cancelled_nodes,
            } => {
                assert_eq!(intent.run_id, "run-3");
                assert_eq!(intent.plan_id.as_deref(), Some("plan-1"));
                assert_eq!(cancelled_nodes, vec!["node-a", "node-b"]);
            }
            other => panic!("expected CancelComplete, got {other:?}"),
        }

        let acks = ack_rx.drain();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, CommandAckStatus::Completed);
    }

    #[tokio::test]
    async fn stale_run_rejected() {
        let (sender, cmd_rx, adapter, mut ack_rx) = test_adapter("run-current");

        // The sender targets "run-current" but the adapter was built for "run-current" --
        // however the sender's build_command uses the sender's run_id ("run-current"),
        // so let's create a mismatched command manually.
        let cmd = ExecutionCommand {
            command_id: "cmd-stale".to_string(),
            correlation_id: "cor-stale".to_string(),
            run_id: "run-old".to_string(), // stale!
            plan_id: None,
            task_id: None,
            attempt: None,
            issued_at_ms: 1000,
            kind: ExecutionCommandKind::Pause,
        };
        // Send directly to the channel.
        let (tx, mut rx) = mpsc::channel(64);
        tx.send(cmd.clone()).await.unwrap();
        let received = rx.recv().await.unwrap();
        drop(tx);
        drop(rx);

        let effect = adapter.process(&received, &[]).await;

        match effect {
            GraphCommandEffect::Rejected { reason, .. } => {
                assert_eq!(reason, "stale run");
            }
            other => panic!("expected Rejected, got {other:?}"),
        }

        let acks = ack_rx.drain();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, CommandAckStatus::Rejected);
        // Clean up unused variables.
        drop(sender);
        drop(cmd_rx);
    }

    #[tokio::test]
    async fn approval_flow_approve_resolves() {
        let (sender, mut cmd_rx, adapter, mut ack_rx) = test_adapter("run-approve");

        // Register an approval.
        let req = build_approval_request(
            "ap-test-1",
            "run-approve",
            "plan-1",
            "task-1",
            "node-1",
            0,
            "file_write",
            "write main.rs",
        );
        adapter.control_service().register_approval(req);

        // Send approve command.
        let cmd = sender.build_command(
            ExecutionCommandKind::Approve {
                approval_id: "ap-test-1".into(),
            },
            Some("plan-1".into()),
            Some("task-1".into()),
            None,
        );
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received, &[]).await;

        match effect {
            GraphCommandEffect::CommandApplied { effect, .. } => {
                assert!(matches!(
                    effect,
                    ControlEffect::ApprovalResolved {
                        resolution: ApprovalResolution::Approved,
                        ..
                    }
                ));
            }
            other => panic!("expected CommandApplied with approval, got {other:?}"),
        }

        let acks = ack_rx.drain();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, CommandAckStatus::Completed);
    }

    #[tokio::test]
    async fn approval_rejection_launches_zero_work() {
        let (sender, mut cmd_rx, adapter, mut ack_rx) = test_adapter("run-reject");

        let req = build_approval_request(
            "ap-rej-1",
            "run-reject",
            "plan-1",
            "task-1",
            "node-1",
            0,
            "shell_exec",
            "run rm -rf /",
        );
        adapter.control_service().register_approval(req);

        let cmd = sender.build_command(
            ExecutionCommandKind::RejectApproval {
                approval_id: "ap-rej-1".into(),
                reason: "dangerous command".into(),
            },
            Some("plan-1".into()),
            Some("task-1".into()),
            None,
        );
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received, &[]).await;

        match effect {
            GraphCommandEffect::CommandApplied { effect, .. } => {
                assert!(matches!(
                    effect,
                    ControlEffect::ApprovalResolved {
                        resolution: ApprovalResolution::Rejected { .. },
                        ..
                    }
                ));
            }
            other => panic!("expected CommandApplied with rejection, got {other:?}"),
        }

        // No approval should be pending.
        assert!(adapter.control_service().pending_approval_ids().is_empty());

        let acks = ack_rx.drain();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, CommandAckStatus::Completed);
    }

    #[tokio::test]
    async fn approval_missing_id_rejected() {
        let (sender, mut cmd_rx, adapter, mut ack_rx) = test_adapter("run-miss");

        // Approve without registering first.
        let cmd = sender.build_command(
            ExecutionCommandKind::Approve {
                approval_id: "ap-nonexistent".into(),
            },
            None,
            None,
            None,
        );
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received, &[]).await;

        match effect {
            GraphCommandEffect::Rejected { reason, .. } => {
                assert!(reason.contains("not found"));
            }
            other => panic!("expected Rejected for missing approval, got {other:?}"),
        }

        let acks = ack_rx.drain();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, CommandAckStatus::Rejected);
    }

    #[tokio::test]
    async fn resume_after_cancel_rejected() {
        let (sender, mut cmd_rx, adapter, mut ack_rx) = test_adapter("run-cancel-resume");

        // Cancel first.
        let cmd = sender.build_command(ExecutionCommandKind::Cancel, None, None, None);
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        adapter.process(&received, &[]).await;

        // Resume should be rejected.
        let cmd = sender.build_command(ExecutionCommandKind::Resume, None, None, None);
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received, &[]).await;

        match effect {
            GraphCommandEffect::Rejected { reason, .. } => {
                assert!(reason.contains("cancellation"));
            }
            other => panic!("expected Rejected after cancel, got {other:?}"),
        }

        let acks = ack_rx.drain();
        assert_eq!(acks.len(), 2);
    }

    #[tokio::test]
    async fn cancel_flag_wires_into_cell_context() {
        let (_sender, _cmd_rx, adapter, _ack_rx) = test_adapter("run-flags");

        let ctx = roko_graph::CellContext::new()
            .with_cancel_flag(adapter.cancel_flag())
            .with_pause_flag(adapter.pause_flag());

        assert!(!ctx.is_cancelled());
        assert!(!ctx.is_paused());

        adapter.cancel_flag.store(true, Ordering::Release);
        assert!(ctx.is_cancelled());

        adapter.pause_flag.store(true, Ordering::Release);
        assert!(ctx.is_paused());
    }

    #[tokio::test]
    async fn soft_retry_returns_applied() {
        let (sender, mut cmd_rx, adapter, mut ack_rx) = test_adapter("run-retry");

        let cmd = sender.build_command(
            ExecutionCommandKind::SoftRetry,
            Some("plan-1".into()),
            Some("task-1".into()),
            None,
        );
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received, &[]).await;

        match effect {
            GraphCommandEffect::CommandApplied { effect, .. } => {
                assert!(matches!(effect, ControlEffect::SoftRetry { .. }));
            }
            other => panic!("expected CommandApplied for SoftRetry, got {other:?}"),
        }

        let acks = ack_rx.drain();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, CommandAckStatus::Completed);
    }

    #[tokio::test]
    async fn skip_returns_applied() {
        let (sender, mut cmd_rx, adapter, mut ack_rx) = test_adapter("run-skip");

        let cmd = sender.build_command(
            ExecutionCommandKind::Skip,
            Some("plan-1".into()),
            Some("task-1".into()),
            None,
        );
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received, &[]).await;

        match effect {
            GraphCommandEffect::CommandApplied { effect, .. } => {
                assert!(matches!(effect, ControlEffect::Skip { .. }));
            }
            other => panic!("expected CommandApplied for Skip, got {other:?}"),
        }

        let acks = ack_rx.drain();
        assert_eq!(acks.len(), 1);
    }

    #[tokio::test]
    async fn reset_returns_applied() {
        let (sender, mut cmd_rx, adapter, mut ack_rx) = test_adapter("run-reset");

        let cmd = sender.build_command(ExecutionCommandKind::Reset, None, None, None);
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received, &[]).await;

        match effect {
            GraphCommandEffect::CommandApplied { effect, .. } => {
                assert_eq!(effect, ControlEffect::Reset);
            }
            other => panic!("expected CommandApplied for Reset, got {other:?}"),
        }

        let acks = ack_rx.drain();
        assert_eq!(acks.len(), 1);
    }

    #[tokio::test]
    async fn repair_returns_applied() {
        let (sender, mut cmd_rx, adapter, mut ack_rx) = test_adapter("run-repair");

        let cmd = sender.build_command(
            ExecutionCommandKind::Repair {
                preserve_completed: true,
            },
            Some("plan-1".into()),
            None,
            None,
        );
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received, &[]).await;

        match effect {
            GraphCommandEffect::CommandApplied { effect, .. } => {
                assert_eq!(
                    effect,
                    ControlEffect::Repair {
                        preserve_completed: true,
                    }
                );
            }
            other => panic!("expected CommandApplied for Repair, got {other:?}"),
        }

        let acks = ack_rx.drain();
        assert_eq!(acks.len(), 1);
    }

    #[tokio::test]
    async fn reverify_gates_returns_applied() {
        let (sender, mut cmd_rx, adapter, mut ack_rx) = test_adapter("run-reverify");

        let cmd = sender.build_command(
            ExecutionCommandKind::ReverifyGates,
            Some("plan-1".into()),
            Some("task-1".into()),
            None,
        );
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        let effect = adapter.process(&received, &[]).await;

        match effect {
            GraphCommandEffect::CommandApplied { effect, .. } => {
                assert!(matches!(effect, ControlEffect::ReverifyGates { .. }));
            }
            other => panic!("expected CommandApplied for ReverifyGates, got {other:?}"),
        }

        let acks = ack_rx.drain();
        assert_eq!(acks.len(), 1);
    }

    #[tokio::test]
    async fn all_ten_command_variants_produce_effects() {
        let (sender, mut cmd_rx, adapter, _ack_rx) = test_adapter("run-ten");

        // Register an approval for the approve/reject tests.
        let req1 = build_approval_request("ap-1", "run-ten", "p", "t", "n", 0, "cap", "tool");
        let req2 = build_approval_request("ap-2", "run-ten", "p", "t", "n", 0, "cap", "tool");
        adapter.control_service().register_approval(req1);
        adapter.control_service().register_approval(req2);

        let kinds = vec![
            ExecutionCommandKind::Pause,
            ExecutionCommandKind::Resume,
            ExecutionCommandKind::SoftRetry,
            ExecutionCommandKind::Repair {
                preserve_completed: false,
            },
            ExecutionCommandKind::ReverifyGates,
            ExecutionCommandKind::Skip,
            ExecutionCommandKind::Approve {
                approval_id: "ap-1".into(),
            },
            ExecutionCommandKind::RejectApproval {
                approval_id: "ap-2".into(),
                reason: "test".into(),
            },
            ExecutionCommandKind::Reset,
            ExecutionCommandKind::Cancel,
        ];

        let mut effects = Vec::new();
        for kind in kinds {
            let cmd = sender.build_command(kind, Some("p".into()), Some("t".into()), None);
            sender.try_send(cmd).unwrap();
            let received = cmd_rx.recv().await.unwrap();
            effects.push(adapter.process(&received, &["node-1".into()]).await);
        }

        assert_eq!(effects.len(), 10);
        assert!(matches!(effects[0], GraphCommandEffect::Paused));
        assert!(matches!(effects[1], GraphCommandEffect::Resumed));
        assert!(matches!(
            effects[2],
            GraphCommandEffect::CommandApplied {
                effect: ControlEffect::SoftRetry { .. },
                ..
            }
        ));
        assert!(matches!(
            effects[3],
            GraphCommandEffect::CommandApplied {
                effect: ControlEffect::Repair { .. },
                ..
            }
        ));
        assert!(matches!(
            effects[4],
            GraphCommandEffect::CommandApplied {
                effect: ControlEffect::ReverifyGates { .. },
                ..
            }
        ));
        assert!(matches!(
            effects[5],
            GraphCommandEffect::CommandApplied {
                effect: ControlEffect::Skip { .. },
                ..
            }
        ));
        assert!(matches!(
            effects[6],
            GraphCommandEffect::CommandApplied {
                effect: ControlEffect::ApprovalResolved {
                    resolution: ApprovalResolution::Approved,
                    ..
                },
                ..
            }
        ));
        assert!(matches!(
            effects[7],
            GraphCommandEffect::CommandApplied {
                effect: ControlEffect::ApprovalResolved {
                    resolution: ApprovalResolution::Rejected { .. },
                    ..
                },
                ..
            }
        ));
        assert!(matches!(
            effects[8],
            GraphCommandEffect::CommandApplied {
                effect: ControlEffect::Reset,
                ..
            }
        ));
        assert!(matches!(
            effects[9],
            GraphCommandEffect::CancelComplete { .. }
        ));
    }

    #[tokio::test]
    async fn standalone_dashboard_no_panic() {
        // When the command channel is disconnected, the adapter should
        // still function for direct process calls without panicking.
        let (sender, cmd_rx, adapter, ack_rx) = test_adapter("run-standalone");
        drop(cmd_rx);
        drop(ack_rx);

        // try_send should fail with Disconnected, not panic.
        let cmd = sender.build_command(ExecutionCommandKind::Pause, None, None, None);
        let err = sender.try_send(cmd);
        assert!(err.is_err());

        // The adapter's flags should still work.
        assert!(!adapter.is_paused());
        assert!(!adapter.is_cancelled());
    }

    #[tokio::test]
    async fn duplicate_command_rejected() {
        let (sender, mut cmd_rx, adapter, mut ack_rx) = test_adapter("run-dup");

        // Send pause.
        let cmd = sender.build_command(ExecutionCommandKind::Pause, None, None, None);
        let cmd_id = cmd.command_id.clone();
        sender.try_send(cmd).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        adapter.process(&received, &[]).await;

        // Send same command ID again (duplicate detection is in the control service).
        let dup_cmd = ExecutionCommand {
            command_id: cmd_id,
            correlation_id: "cor-dup".into(),
            run_id: "run-dup".into(),
            plan_id: None,
            task_id: None,
            attempt: None,
            issued_at_ms: 2000,
            kind: ExecutionCommandKind::Resume,
        };
        let effect = adapter.process(&dup_cmd, &[]).await;

        assert!(
            matches!(effect, GraphCommandEffect::Rejected { reason, .. } if reason == "duplicate command_id")
        );

        let acks = ack_rx.drain();
        assert_eq!(acks.len(), 2);
    }

    #[tokio::test]
    async fn snapshot_and_restore_preserves_state() {
        let (_sender, _cmd_rx, adapter, _ack_rx) = test_adapter("run-snap");

        // Register an approval and process a command.
        let req = build_approval_request(
            "ap-snap", "run-snap", "plan-1", "task-1", "node-1", 0, "cap", "tool",
        );
        adapter.control_service().register_approval(req);

        // Snapshot.
        let snapshot = adapter.control_service().snapshot();
        assert!(snapshot.pending_approvals.contains_key("ap-snap"));

        // Restore into a fresh service.
        let fresh = ExecutionControlService::new("run-snap");
        fresh.restore(snapshot.clone());
        assert_eq!(fresh.snapshot(), snapshot);
    }
}

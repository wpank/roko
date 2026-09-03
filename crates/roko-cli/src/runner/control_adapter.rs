//! Runner-v2 execution command adapter (#233).
//!
//! This module bridges the executor-neutral `ExecutionCommand` transport
//! into the existing runner-v2 event-loop branches. It delegates to the
//! same scheduling logic the legacy `TuiCommand` match arms used, without
//! changing any scheduling semantics.
//!
//! #255 will add `Approve`, `RejectApproval`, and `Reset` handling to the
//! same adapter.

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
    Cancel {
        plan_id: Option<String>,
    },
    /// The command was accepted but the operation is not yet implemented.
    /// The runner should log a rejection to the TUI bridge.
    NotImplemented {
        kind: ExecutionCommandKind,
        message: String,
    },
    /// The command was rejected (e.g. stale run ID).
    Rejected {
        message: String,
    },
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
                let plan_id_str = cmd.plan_id.as_deref().unwrap_or("?");
                warn!(
                    command_id = %cmd.command_id,
                    plan_id = %plan_id_str,
                    "execution command rejected: soft retry is not implemented"
                );
                let msg = "Soft retry is not available during this run; no state changed";
                (
                    RunnerCommandEffect::NotImplemented {
                        kind: cmd.kind.clone(),
                        message: msg.to_string(),
                    },
                    CommandAckStatus::Rejected,
                    Some(msg.to_string()),
                )
            }
            ExecutionCommandKind::Repair { preserve_completed } => {
                let plan_id_str = cmd.plan_id.as_deref().unwrap_or("?");
                warn!(
                    command_id = %cmd.command_id,
                    plan_id = %plan_id_str,
                    preserve_completed,
                    "execution command rejected: repair is not implemented"
                );
                let msg = "Repair is not available during this run; no state changed";
                (
                    RunnerCommandEffect::NotImplemented {
                        kind: cmd.kind.clone(),
                        message: msg.to_string(),
                    },
                    CommandAckStatus::Rejected,
                    Some(msg.to_string()),
                )
            }
            ExecutionCommandKind::ReverifyGates => {
                let plan_id_str = cmd.plan_id.as_deref().unwrap_or("?");
                warn!(
                    command_id = %cmd.command_id,
                    plan_id = %plan_id_str,
                    "execution command rejected: gate reverify is not implemented"
                );
                let msg = "Gate reverify is not available during this run; no state changed";
                (
                    RunnerCommandEffect::NotImplemented {
                        kind: cmd.kind.clone(),
                        message: msg.to_string(),
                    },
                    CommandAckStatus::Rejected,
                    Some(msg.to_string()),
                )
            }
            ExecutionCommandKind::Skip => {
                let plan_id_str = cmd.plan_id.as_deref().unwrap_or("?");
                let task_id_str = cmd.task_id.as_deref().unwrap_or("?");
                warn!(
                    command_id = %cmd.command_id,
                    plan_id = %plan_id_str,
                    task_id = %task_id_str,
                    "execution command rejected: task skip is not implemented"
                );
                let msg = "Task skip is not available during this run; no state changed";
                (
                    RunnerCommandEffect::NotImplemented {
                        kind: cmd.kind.clone(),
                        message: msg.to_string(),
                    },
                    CommandAckStatus::Rejected,
                    Some(msg.to_string()),
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
        let (sender, mut cmd_rx, ack_tx, ack_rx) =
            ExecutionCommandSender::channel("run-adapter");
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
        let (sender, mut cmd_rx, ack_tx, ack_rx) =
            ExecutionCommandSender::channel("run-cancel");
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
        let (sender, mut cmd_rx, ack_tx, ack_rx) =
            ExecutionCommandSender::channel("run-old");
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
    async fn tui_command_unimplemented_kinds_rejected() {
        let (sender, mut cmd_rx, ack_tx, ack_rx) =
            ExecutionCommandSender::channel("run-unimpl");
        let adapter = RunnerExecutionCommandAdapter::new("run-unimpl", ack_tx);
        let mut ack_receiver = crate::execution_control::CommandAckReceiver::new(ack_rx);

        let unimplemented_kinds = vec![
            ExecutionCommandKind::SoftRetry,
            ExecutionCommandKind::Repair {
                preserve_completed: true,
            },
            ExecutionCommandKind::ReverifyGates,
            ExecutionCommandKind::Skip,
        ];

        for kind in &unimplemented_kinds {
            let cmd = sender.build_command(
                kind.clone(),
                Some("plan-x".into()),
                Some("task-y".into()),
                None,
            );
            sender.try_send(cmd).unwrap();
            let received = cmd_rx.recv().await.unwrap();
            let effect = adapter.process(&received).await;
            assert!(
                matches!(effect, RunnerCommandEffect::NotImplemented { .. }),
                "expected NotImplemented for {kind}, got {effect:?}"
            );
        }

        let acks = ack_receiver.drain();
        assert_eq!(acks.len(), unimplemented_kinds.len());
        assert!(acks.iter().all(|a| a.status == CommandAckStatus::Rejected));
    }

    #[tokio::test]
    async fn tui_command_all_seven_variants_produce_effects() {
        let (sender, mut cmd_rx, ack_tx, _ack_rx) =
            ExecutionCommandSender::channel("run-seven");
        let adapter = RunnerExecutionCommandAdapter::new("run-seven", ack_tx);

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
        ];

        let mut effects = Vec::new();
        for kind in kinds {
            let cmd = sender.build_command(kind, Some("p".into()), Some("t".into()), None);
            sender.try_send(cmd).unwrap();
            let received = cmd_rx.recv().await.unwrap();
            effects.push(adapter.process(&received).await);
        }

        assert_eq!(effects.len(), 7);
        assert_eq!(effects[0], RunnerCommandEffect::Pause);
        assert_eq!(effects[1], RunnerCommandEffect::Resume);
        assert!(matches!(effects[2], RunnerCommandEffect::NotImplemented { .. }));
        assert!(matches!(effects[3], RunnerCommandEffect::NotImplemented { .. }));
        assert!(matches!(effects[4], RunnerCommandEffect::NotImplemented { .. }));
        assert!(matches!(effects[5], RunnerCommandEffect::NotImplemented { .. }));
        assert!(matches!(effects[6], RunnerCommandEffect::Cancel { .. }));
    }
}

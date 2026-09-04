//! Executor-neutral TUI command transport (#233).
//!
//! This module defines the bounded UI-to-live-executor transport that both
//! the interactive TUI and CLI file-based control paths use to issue commands
//! to a running executor (runner-v2, graph engine, or a fake test adapter).
//!
//! # Design
//!
//! - `ExecutionCommand` is the universal command envelope: every command has
//!   a unique `command_id`, a `correlation_id` for request/response pairing,
//!   and an `ExecutionCommandKind` discriminant.
//! - `CommandAck` is the universal acknowledgement: the executor fills in
//!   `status` and an optional human-readable `message`.
//! - `ExecutionCommandSender` wraps a bounded Tokio MPSC sender (capacity 64)
//!   and provides `try_send` semantics safe for synchronous TUI key handlers.
//! - `CommandAckReceiver` wraps the acknowledgement return channel (capacity
//!   128) so the TUI render loop can drain acks without blocking.
//!
//! # Scope boundary
//!
//! This module owns transport only. Graph scheduling, approval semantics,
//! persistent command receipts, and crash behavior live in
//! `roko_graph::control::ExecutionControlService` (#255).

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// Command types
// ---------------------------------------------------------------------------

/// A single command from the UI (TUI or CLI) to a running executor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionCommand {
    /// Unique identifier for this command instance.
    pub command_id: String,
    /// Correlation identifier for request/response pairing across channels.
    pub correlation_id: String,
    /// The run this command targets. Commands with a stale run ID are rejected.
    pub run_id: String,
    /// Optional plan scope.
    pub plan_id: Option<String>,
    /// Optional task scope (requires `plan_id`).
    pub task_id: Option<String>,
    /// Optional attempt number for retry-scoped commands.
    pub attempt: Option<u32>,
    /// Wall-clock timestamp (milliseconds since UNIX epoch) when this command
    /// was issued by the UI.
    pub issued_at_ms: u64,
    /// The command variant.
    pub kind: ExecutionCommandKind,
}

/// The command discriminant — one-for-one with the legacy `TuiCommand` variants
/// plus the approval and reset commands added by #255.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionCommandKind {
    /// Pause the executor: finish the current agent turn, then stop dispatching.
    Pause,
    /// Resume dispatching after a pause.
    Resume,
    /// Retry failed tasks in a plan from scratch.
    SoftRetry,
    /// Repair a plan: re-run only failed/pending tasks, preserving completed ones.
    Repair {
        /// If true, keep completed tasks as-is; only re-run failed/pending.
        preserve_completed: bool,
    },
    /// Re-run gate checks for a plan without re-executing tasks.
    ReverifyGates,
    /// Skip a specific task within a plan.
    Skip,
    /// Cancel a running plan.
    Cancel,
    /// Approve a pending approval request (#255).
    ///
    /// Resolves the matching pending approval before provider process spawn;
    /// stale or mismatched approval IDs are rejected.
    Approve {
        /// The approval request ID to resolve.
        approval_id: String,
    },
    /// Reject a pending approval request (#255).
    ///
    /// Stale or mismatched approval IDs are rejected. Rejection launches
    /// zero provider work.
    RejectApproval {
        /// The approval request ID to reject.
        approval_id: String,
        /// Human-readable reason for the rejection.
        reason: String,
    },
    /// Reset eligible graph state (#255).
    ///
    /// Uses the same receipt-preserving rules as repair-clean: committed
    /// receipts are never erased.
    Reset,
}

impl fmt::Display for ExecutionCommandKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pause => write!(f, "pause"),
            Self::Resume => write!(f, "resume"),
            Self::SoftRetry => write!(f, "soft-retry"),
            Self::Repair { preserve_completed } => {
                write!(f, "repair(preserve={})", preserve_completed)
            }
            Self::ReverifyGates => write!(f, "reverify-gates"),
            Self::Skip => write!(f, "skip"),
            Self::Cancel => write!(f, "cancel"),
            Self::Approve { approval_id } => write!(f, "approve({})", approval_id),
            Self::RejectApproval {
                approval_id,
                reason,
            } => write!(f, "reject-approval({}, {})", approval_id, reason),
            Self::Reset => write!(f, "reset"),
        }
    }
}

// ---------------------------------------------------------------------------
// Acknowledgement types
// ---------------------------------------------------------------------------

/// Acknowledgement sent by the executor back to the UI after processing a
/// command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandAck {
    /// The `command_id` of the command being acknowledged.
    pub command_id: String,
    /// The correlation ID echoed back from the command.
    pub correlation_id: String,
    /// The run ID that processed this command.
    pub run_id: String,
    /// Result status.
    pub status: CommandAckStatus,
    /// Optional human-readable detail.
    pub message: Option<String>,
    /// Wall-clock timestamp (milliseconds since UNIX epoch) when the
    /// acknowledgement was produced.
    pub completed_at_ms: u64,
}

/// Outcome status for a command acknowledgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandAckStatus {
    /// The executor accepted the command and will act on it.
    Accepted,
    /// The executor rejected the command (e.g. stale run, unsupported).
    Rejected,
    /// The command has been fully applied.
    Completed,
    /// The command failed during execution.
    Failed,
}

impl fmt::Display for CommandAckStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Accepted => write!(f, "accepted"),
            Self::Rejected => write!(f, "rejected"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

// ---------------------------------------------------------------------------
// Channel wrappers
// ---------------------------------------------------------------------------

/// Bounded command sender (capacity 64) safe for synchronous TUI key handlers.
///
/// The TUI render loop must never await channel capacity; `try_send` is
/// mandatory from key handling code.
#[derive(Debug, Clone)]
pub struct ExecutionCommandSender {
    tx: mpsc::Sender<ExecutionCommand>,
    /// The run ID of the executor this sender targets. Commands issued with a
    /// different run ID will be rejected by the executor.
    run_id: String,
}

/// Capacity of the command channel.
pub const COMMAND_CHANNEL_CAPACITY: usize = 64;

/// Capacity of the acknowledgement channel.
pub const ACK_CHANNEL_CAPACITY: usize = 128;

/// Error returned when `try_send` fails.
#[derive(Debug)]
pub enum CommandSendError {
    /// The channel is full — the executor is not draining fast enough.
    Full(ExecutionCommand),
    /// The executor has disconnected (receiver dropped).
    Disconnected(ExecutionCommand),
}

impl fmt::Display for CommandSendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full(_) => write!(f, "command queue full"),
            Self::Disconnected(_) => write!(f, "executor disconnected"),
        }
    }
}

impl std::error::Error for CommandSendError {}

impl ExecutionCommandSender {
    /// Create a new sender/receiver pair with the specified run ID.
    ///
    /// Returns `(sender, command_receiver, ack_sender, ack_receiver)`.
    pub fn channel(
        run_id: impl Into<String>,
    ) -> (
        Self,
        mpsc::Receiver<ExecutionCommand>,
        mpsc::Sender<CommandAck>,
        mpsc::Receiver<CommandAck>,
    ) {
        let run_id = run_id.into();
        let (cmd_tx, cmd_rx) = mpsc::channel(COMMAND_CHANNEL_CAPACITY);
        let (ack_tx, ack_rx) = mpsc::channel(ACK_CHANNEL_CAPACITY);
        let sender = Self { tx: cmd_tx, run_id };
        (sender, cmd_rx, ack_tx, ack_rx)
    }

    /// The run ID this sender targets.
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    /// Try to send a command without blocking. Returns an error if the
    /// channel is full or the receiver has been dropped.
    pub fn try_send(&self, cmd: ExecutionCommand) -> Result<(), CommandSendError> {
        self.tx.try_send(cmd).map_err(|e| match e {
            mpsc::error::TrySendError::Full(cmd) => CommandSendError::Full(cmd),
            mpsc::error::TrySendError::Closed(cmd) => CommandSendError::Disconnected(cmd),
        })
    }

    /// Build an `ExecutionCommand` with a fresh command ID and timestamp,
    /// targeting this sender's run ID.
    pub fn build_command(
        &self,
        kind: ExecutionCommandKind,
        plan_id: Option<String>,
        task_id: Option<String>,
        attempt: Option<u32>,
    ) -> ExecutionCommand {
        ExecutionCommand {
            command_id: new_command_id(),
            correlation_id: new_command_id(),
            run_id: self.run_id.clone(),
            plan_id,
            task_id,
            attempt,
            issued_at_ms: now_ms(),
            kind,
        }
    }

    /// Convenience: build and try-send in one call.
    pub fn send_kind(
        &self,
        kind: ExecutionCommandKind,
        plan_id: Option<String>,
        task_id: Option<String>,
    ) -> Result<String, CommandSendError> {
        let cmd = self.build_command(kind, plan_id, task_id, None);
        let command_id = cmd.command_id.clone();
        self.try_send(cmd)?;
        Ok(command_id)
    }
}

/// Receiver wrapper for acknowledgements. The TUI drains this on every tick.
#[derive(Debug)]
pub struct CommandAckReceiver {
    rx: mpsc::Receiver<CommandAck>,
}

impl CommandAckReceiver {
    /// Wrap a raw receiver.
    pub fn new(rx: mpsc::Receiver<CommandAck>) -> Self {
        Self { rx }
    }

    /// Drain all available acknowledgements without blocking.
    pub fn drain(&mut self) -> Vec<CommandAck> {
        let mut acks = Vec::new();
        while let Ok(ack) = self.rx.try_recv() {
            acks.push(ack);
        }
        acks
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate a short unique command ID (UUID v4 hex, first 12 chars).
fn new_command_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let ms = now_ms();
    format!("cmd-{ms:x}-{seq:04x}")
}

/// Milliseconds since UNIX epoch.
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Build a `CommandAck` for the given command.
pub fn ack_for(
    cmd: &ExecutionCommand,
    status: CommandAckStatus,
    message: Option<String>,
) -> CommandAck {
    CommandAck {
        command_id: cmd.command_id.clone(),
        correlation_id: cmd.correlation_id.clone(),
        run_id: cmd.run_id.clone(),
        status,
        message,
        completed_at_ms: now_ms(),
    }
}

// ---------------------------------------------------------------------------
// Conversion: ControlCommand -> ExecutionCommand at the polling boundary
// ---------------------------------------------------------------------------

/// Convert a file-based `ControlCommand` (from `.roko/state/control.json`)
/// into an `ExecutionCommand`, scoped to the given run.
pub fn control_command_to_execution(
    ctrl: &crate::runner::types::ControlCommand,
    run_id: &str,
) -> ExecutionCommand {
    use crate::runner::types::ControlAction;

    let kind = match ctrl.command {
        ControlAction::Pause => ExecutionCommandKind::Pause,
        ControlAction::Resume => ExecutionCommandKind::Resume,
        ControlAction::Cancel => ExecutionCommandKind::Cancel,
        ControlAction::Retry => ExecutionCommandKind::SoftRetry,
    };

    ExecutionCommand {
        command_id: new_command_id(),
        correlation_id: new_command_id(),
        run_id: run_id.to_string(),
        plan_id: ctrl.plan_id.clone(),
        task_id: ctrl.task_id.clone(),
        attempt: None,
        issued_at_ms: now_ms(),
        kind,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_ids_are_unique() {
        let id1 = new_command_id();
        let id2 = new_command_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("cmd-"));
    }

    #[test]
    fn ack_echoes_command_fields() {
        let cmd = ExecutionCommand {
            command_id: "c1".to_string(),
            correlation_id: "cor1".to_string(),
            run_id: "run-abc".to_string(),
            plan_id: Some("plan-1".to_string()),
            task_id: None,
            attempt: None,
            issued_at_ms: 1000,
            kind: ExecutionCommandKind::Pause,
        };
        let ack = ack_for(&cmd, CommandAckStatus::Completed, Some("done".into()));
        assert_eq!(ack.command_id, "c1");
        assert_eq!(ack.correlation_id, "cor1");
        assert_eq!(ack.run_id, "run-abc");
        assert_eq!(ack.status, CommandAckStatus::Completed);
        assert_eq!(ack.message.as_deref(), Some("done"));
        assert!(ack.completed_at_ms > 0);
    }

    #[tokio::test]
    async fn channel_send_and_receive() {
        let (sender, mut cmd_rx, _ack_tx, _ack_rx) = ExecutionCommandSender::channel("run-1");
        let cmd = sender.build_command(ExecutionCommandKind::Pause, None, None, None);
        sender.try_send(cmd.clone()).unwrap();
        let received = cmd_rx.recv().await.unwrap();
        assert_eq!(received.kind, ExecutionCommandKind::Pause);
        assert_eq!(received.run_id, "run-1");
    }

    #[tokio::test]
    async fn full_channel_returns_error() {
        // Create a channel with capacity 1 for easy filling.
        let (tx, _rx) = mpsc::channel(1);
        let sender = ExecutionCommandSender {
            tx,
            run_id: "run-x".to_string(),
        };
        let cmd1 = sender.build_command(ExecutionCommandKind::Pause, None, None, None);
        let cmd2 = sender.build_command(ExecutionCommandKind::Resume, None, None, None);
        sender.try_send(cmd1).unwrap();
        let err = sender.try_send(cmd2).unwrap_err();
        assert!(matches!(err, CommandSendError::Full(_)));
        assert_eq!(err.to_string(), "command queue full");
    }

    #[tokio::test]
    async fn disconnected_channel_returns_error() {
        let (sender, cmd_rx, _ack_tx, _ack_rx) = ExecutionCommandSender::channel("run-y");
        drop(cmd_rx); // disconnect
        let cmd = sender.build_command(ExecutionCommandKind::Cancel, None, None, None);
        let err = sender.try_send(cmd).unwrap_err();
        assert!(matches!(err, CommandSendError::Disconnected(_)));
        assert_eq!(err.to_string(), "executor disconnected");
    }

    #[tokio::test]
    async fn ack_receiver_drain() {
        let (_sender, _cmd_rx, ack_tx, ack_rx) = ExecutionCommandSender::channel("run-z");
        let mut ack_receiver = CommandAckReceiver::new(ack_rx);

        // Send two acks.
        let a1 = CommandAck {
            command_id: "c1".into(),
            correlation_id: "cor1".into(),
            run_id: "run-z".into(),
            status: CommandAckStatus::Completed,
            message: None,
            completed_at_ms: 100,
        };
        let a2 = CommandAck {
            command_id: "c2".into(),
            correlation_id: "cor2".into(),
            run_id: "run-z".into(),
            status: CommandAckStatus::Rejected,
            message: Some("stale run".into()),
            completed_at_ms: 200,
        };
        ack_tx.send(a1).await.unwrap();
        ack_tx.send(a2).await.unwrap();

        let drained = ack_receiver.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].command_id, "c1");
        assert_eq!(drained[1].status, CommandAckStatus::Rejected);

        // Second drain returns empty.
        let drained2 = ack_receiver.drain();
        assert!(drained2.is_empty());
    }

    #[test]
    fn send_kind_convenience() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let (sender, mut cmd_rx, _ack_tx, _ack_rx) =
                ExecutionCommandSender::channel("run-conv");
            let cmd_id = sender
                .send_kind(ExecutionCommandKind::Cancel, Some("plan-a".into()), None)
                .unwrap();
            assert!(cmd_id.starts_with("cmd-"));
            let received = cmd_rx.recv().await.unwrap();
            assert_eq!(received.kind, ExecutionCommandKind::Cancel);
            assert_eq!(received.plan_id.as_deref(), Some("plan-a"));
        });
    }

    #[test]
    fn control_command_conversion() {
        let ctrl = crate::runner::types::ControlCommand {
            command: crate::runner::types::ControlAction::Pause,
            plan_id: Some("p1".into()),
            task_id: None,
        };
        let exec_cmd = control_command_to_execution(&ctrl, "run-42");
        assert_eq!(exec_cmd.kind, ExecutionCommandKind::Pause);
        assert_eq!(exec_cmd.run_id, "run-42");
        assert_eq!(exec_cmd.plan_id.as_deref(), Some("p1"));
    }

    #[test]
    fn display_impls() {
        assert_eq!(ExecutionCommandKind::Pause.to_string(), "pause");
        assert_eq!(ExecutionCommandKind::Resume.to_string(), "resume");
        assert_eq!(ExecutionCommandKind::SoftRetry.to_string(), "soft-retry");
        assert_eq!(
            ExecutionCommandKind::Repair {
                preserve_completed: true
            }
            .to_string(),
            "repair(preserve=true)"
        );
        assert_eq!(
            ExecutionCommandKind::ReverifyGates.to_string(),
            "reverify-gates"
        );
        assert_eq!(ExecutionCommandKind::Skip.to_string(), "skip");
        assert_eq!(ExecutionCommandKind::Cancel.to_string(), "cancel");
        assert_eq!(
            ExecutionCommandKind::Approve {
                approval_id: "ap-1".into()
            }
            .to_string(),
            "approve(ap-1)"
        );
        assert_eq!(
            ExecutionCommandKind::RejectApproval {
                approval_id: "ap-2".into(),
                reason: "unsafe".into()
            }
            .to_string(),
            "reject-approval(ap-2, unsafe)"
        );
        assert_eq!(ExecutionCommandKind::Reset.to_string(), "reset");

        assert_eq!(CommandAckStatus::Accepted.to_string(), "accepted");
        assert_eq!(CommandAckStatus::Rejected.to_string(), "rejected");
        assert_eq!(CommandAckStatus::Completed.to_string(), "completed");
        assert_eq!(CommandAckStatus::Failed.to_string(), "failed");
    }

    // ── Fake adapter ────────────────────────────────────────────────

    /// A fake executor adapter for testing the command transport without a
    /// real runner. Receives commands and immediately returns a configurable
    /// acknowledgement.
    pub struct FakeExecutionCommandAdapter {
        cmd_rx: mpsc::Receiver<ExecutionCommand>,
        ack_tx: mpsc::Sender<CommandAck>,
        run_id: String,
        /// Commands received (for assertions).
        pub received: Vec<ExecutionCommand>,
    }

    impl FakeExecutionCommandAdapter {
        /// Create a new fake adapter wired to the given channel pair.
        pub fn new(
            cmd_rx: mpsc::Receiver<ExecutionCommand>,
            ack_tx: mpsc::Sender<CommandAck>,
            run_id: impl Into<String>,
        ) -> Self {
            Self {
                cmd_rx,
                ack_tx,
                run_id: run_id.into(),
                received: Vec::new(),
            }
        }

        /// Process one command: validate the run ID, record it, and send
        /// back an acknowledgement.
        pub async fn process_one(&mut self) -> Option<ExecutionCommand> {
            let cmd = self.cmd_rx.recv().await?;
            let ack = if cmd.run_id != self.run_id {
                ack_for(
                    &cmd,
                    CommandAckStatus::Rejected,
                    Some("stale run".to_string()),
                )
            } else {
                ack_for(&cmd, CommandAckStatus::Completed, None)
            };
            let _ = self.ack_tx.send(ack).await;
            self.received.push(cmd.clone());
            Some(cmd)
        }
    }

    #[tokio::test]
    async fn fake_adapter_accepts_matching_run() {
        let (sender, cmd_rx, ack_tx, ack_rx) = ExecutionCommandSender::channel("run-fake");
        let mut adapter = FakeExecutionCommandAdapter::new(cmd_rx, ack_tx, "run-fake");
        let mut ack_receiver = CommandAckReceiver::new(ack_rx);

        let cmd = sender.build_command(ExecutionCommandKind::Pause, None, None, None);
        sender.try_send(cmd).unwrap();
        adapter.process_one().await.unwrap();

        let acks = ack_receiver.drain();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, CommandAckStatus::Completed);
        assert_eq!(adapter.received.len(), 1);
        assert_eq!(adapter.received[0].kind, ExecutionCommandKind::Pause);
    }

    #[tokio::test]
    async fn fake_adapter_rejects_stale_run() {
        let (sender, cmd_rx, ack_tx, ack_rx) = ExecutionCommandSender::channel("run-old");
        let mut adapter = FakeExecutionCommandAdapter::new(cmd_rx, ack_tx, "run-new");
        let mut ack_receiver = CommandAckReceiver::new(ack_rx);

        let cmd = sender.build_command(
            ExecutionCommandKind::Cancel,
            Some("plan-1".into()),
            None,
            None,
        );
        sender.try_send(cmd).unwrap();
        adapter.process_one().await.unwrap();

        let acks = ack_receiver.drain();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, CommandAckStatus::Rejected);
        assert_eq!(acks[0].message.as_deref(), Some("stale run"));
    }

    #[tokio::test]
    async fn standalone_dashboard_never_panics() {
        // Without a sender, the TUI should not panic — it just shows
        // notifications instead of sending commands.
        let (sender, cmd_rx, _ack_tx, ack_rx) = ExecutionCommandSender::channel("run-test");
        let mut ack_receiver = CommandAckReceiver::new(ack_rx);
        drop(cmd_rx); // simulate no executor

        let cmd = sender.build_command(ExecutionCommandKind::Pause, None, None, None);
        let err = sender.try_send(cmd);
        assert!(matches!(err, Err(CommandSendError::Disconnected(_))));

        // Draining an empty ack channel is safe.
        let acks = ack_receiver.drain();
        assert!(acks.is_empty());
    }

    #[tokio::test]
    async fn fake_adapter_processes_all_command_kinds() {
        let (sender, cmd_rx, ack_tx, ack_rx) = ExecutionCommandSender::channel("run-all");
        let mut adapter = FakeExecutionCommandAdapter::new(cmd_rx, ack_tx, "run-all");
        let mut ack_receiver = CommandAckReceiver::new(ack_rx);

        let kinds = vec![
            ExecutionCommandKind::Pause,
            ExecutionCommandKind::Resume,
            ExecutionCommandKind::SoftRetry,
            ExecutionCommandKind::Repair {
                preserve_completed: true,
            },
            ExecutionCommandKind::ReverifyGates,
            ExecutionCommandKind::Skip,
            ExecutionCommandKind::Cancel,
            ExecutionCommandKind::Approve {
                approval_id: "ap-test".into(),
            },
            ExecutionCommandKind::RejectApproval {
                approval_id: "ap-test-2".into(),
                reason: "test rejection".into(),
            },
            ExecutionCommandKind::Reset,
        ];

        for kind in &kinds {
            let cmd = sender.build_command(
                kind.clone(),
                Some("plan-x".into()),
                Some("task-y".into()),
                None,
            );
            sender.try_send(cmd).unwrap();
            adapter.process_one().await.unwrap();
        }

        let acks = ack_receiver.drain();
        assert_eq!(acks.len(), kinds.len());
        assert!(acks.iter().all(|a| a.status == CommandAckStatus::Completed));
        assert_eq!(adapter.received.len(), kinds.len());
    }
}

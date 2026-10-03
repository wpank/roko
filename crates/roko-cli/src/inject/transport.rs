//! The socket each `roko plan run` listens on for `roko inject`, and the
//! client that talks to it (gap-f118b3).
//!
//! A run binds `.roko/runtime/inject/<pid>.sock` and writes the token a
//! client must present first to `<pid>.token`, both owner-only (`0600`), as
//! the StateHub IPC socket does. When that socket path is too long for a Unix
//! socket (a deep checkout), the run binds `<pid>.sock` in a short private
//! directory and names it in `<pid>.sock.path` instead, as the StateHub
//! socket does too (1224). The frames are that socket's (a 4-byte
//! big-endian length, then a JSON body). The client sends a hello carrying
//! the token, then one [`InjectWireRequest`]. The socket turns it into an
//! [`ExecutionCommand`](crate::execution_control::ExecutionCommand) for the
//! running plan its session names (a directive or context becomes
//! `ExecutionCommandKind::Inject`, an abort `Cancel`), sends it on the run's
//! inject command channel, which the plan-set driver routes like a TUI
//! command, and writes back the run's acknowledgement as an
//! [`InjectWireReply`]. `roko plan pause`, `resume`, `cancel` and `retry`
//! send their commands this way too (1209): for those the session is the
//! plan the command names as given, or empty for the whole run, and the
//! driver decides. So does `roko plan budget raise` (backlog 2118), whose
//! payload is the plan's new ceiling in USD. An accepted request is answered
//! again, not delivered again, when it is sent a second time. A hello,
//! request or acknowledgement that does not arrive in time ends the
//! exchange, and no side logs the payload.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::execution_control::{CommandAck, CommandAckStatus, ExecutionCommandSender};

/// How long a run's inject socket waits for the run's acknowledgement.
pub const INJECT_ANSWER_TIMEOUT: Duration = Duration::from_secs(30);

/// The directory that holds each plan run's inject socket and token.
#[must_use]
pub fn inject_socket_dir(workdir: &Path) -> PathBuf {
    workdir.join(".roko").join("runtime").join("inject")
}

/// What `roko inject` sends a plan run, after its token.
///
/// No `Debug`: the payload may be sensitive, and must not reach a log.
#[derive(Clone, Serialize, Deserialize)]
pub struct InjectWireRequest {
    /// Unique to one `roko inject`, so a resent request is delivered once.
    pub request_id: String,
    /// The running plan, or its checkpoint run, the request is for. For a
    /// plan control kind, the plan as named, or empty for the whole run.
    pub session: String,
    /// `directive`, `context` or `abort`, or a plan control kind: `pause`,
    /// `resume`, `cancel`, `retry` or `raise_budget`.
    pub kind: String,
    /// The text to deliver; empty for an abort. For `raise_budget`, the
    /// plan's new ceiling in USD.
    pub payload: String,
}

/// How a plan run answered an inject request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InjectOutcome {
    /// The run took it: a directive or context waits for the next task of
    /// the plan to start, or the plan is cancelled.
    Accepted,
    /// The run has no running plan or checkpoint run of that name.
    UnknownSession,
    /// The run refused it, and says why.
    Rejected,
}

/// A plan run's answer to an [`InjectWireRequest`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InjectWireReply {
    /// The request this answers.
    pub request_id: String,
    /// What the run did with it.
    pub outcome: InjectOutcome,
    /// What happened, or why not.
    pub message: String,
}

impl InjectWireReply {
    /// The answer to `request`.
    #[must_use]
    pub fn new(
        request: &InjectWireRequest,
        outcome: InjectOutcome,
        message: impl Into<String>,
    ) -> Self {
        Self {
            request_id: request.request_id.clone(),
            outcome,
            message: message.into(),
        }
    }

    /// What `roko inject` hears of the run's acknowledgement `ack`.
    fn for_ack(request: &InjectWireRequest, ack: &CommandAck) -> Self {
        let outcome = match ack.status {
            CommandAckStatus::Accepted | CommandAckStatus::Completed => InjectOutcome::Accepted,
            CommandAckStatus::Rejected | CommandAckStatus::Failed => InjectOutcome::Rejected,
        };
        let message = ack
            .message
            .clone()
            .unwrap_or_else(|| format!("{} {}", request.kind, ack.status));
        Self::new(request, outcome, message)
    }
}

/// The running plan a `roko inject` session names, if any.
pub type InjectTarget = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// How a plan run's inject socket reaches the run.
pub struct InjectLink {
    /// The run's inject command channel, which its plan-set driver routes
    /// like the TUI's.
    pub commands: ExecutionCommandSender,
    /// Where the run acknowledges those commands.
    pub acks: mpsc::Receiver<CommandAck>,
    /// The running plan a session names: by plan id, or by checkpoint run.
    pub target: InjectTarget,
    /// How long the socket waits for an acknowledgement.
    pub answer_timeout: Duration,
}

/// Listen for `roko inject` in `workdir` through `link`. A socket that
/// cannot be set up is logged, and the run goes on without it: `roko inject`
/// then reports that no run is listening.
#[must_use]
pub fn listen_for_inject(workdir: &Path, link: InjectLink) -> Option<InjectServer> {
    #[cfg(unix)]
    {
        match start_inject_server(workdir, link) {
            Ok(server) => Some(server),
            Err(error) => {
                tracing::warn!(%error, "roko inject cannot reach this run: no socket");
                None
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (workdir, link);
        None
    }
}

#[cfg(unix)]
mod unix {
    use std::collections::{HashMap, VecDeque};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Duration;

    use anyhow::{Context as _, Result};
    use serde::{Deserialize, Serialize};
    use tokio::net::{UnixListener, UnixStream};
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use super::{InjectLink, InjectOutcome, InjectTarget, InjectWireReply, InjectWireRequest};
    use crate::execution_control::{
        CommandAck, CommandSendError, ExecutionCommandKind, ExecutionCommandSender, InjectedKind,
        InjectedText,
    };
    use crate::state_hub_ipc::{
        bind_socket, bound_socket_path, mint_hub_token, read_frame, socket_pointer_path,
        tokens_match, write_frame, write_hub_token,
    };

    /// The first frame a client sends: the token from the run's token file.
    ///
    /// No `Debug`, so the token cannot reach a log.
    #[derive(Serialize, Deserialize)]
    struct InjectHello {
        token: String,
    }

    /// Largest hello the run reads; a token is 64 bytes.
    const MAX_HELLO_FRAME_BYTES: u32 = 4096;
    /// Largest request the run reads: a directive's text and its framing.
    const MAX_REQUEST_FRAME_BYTES: u32 = 64 * 1024;
    /// Largest answer the client reads.
    const MAX_REPLY_FRAME_BYTES: u32 = 64 * 1024;
    /// How long either side waits to connect, or for the other's frame.
    const FRAME_TIMEOUT: Duration = Duration::from_secs(5);
    /// How many accepted requests a run remembers, oldest forgotten first.
    const MAX_REMEMBERED_ANSWERS: usize = 1024;

    /// A plan run's inject socket. Dropping it stops the listener and
    /// removes the socket, its pointer file and its token.
    pub struct InjectServer {
        shutdown: CancellationToken,
        /// Where the socket is bound: its home in the workspace, or the path
        /// the pointer file names.
        socket: PathBuf,
        /// The file naming the socket's path when that is not its home.
        pointer: PathBuf,
        token: PathBuf,
    }

    impl Drop for InjectServer {
        fn drop(&mut self) {
            self.shutdown.cancel();
            let _ = std::fs::remove_file(&self.socket);
            let _ = std::fs::remove_file(&self.pointer);
            let _ = std::fs::remove_file(&self.token);
        }
    }

    /// Listen for `roko inject` in `workdir`, sending each request that
    /// presents this run's token to the run through `link`.
    pub fn start_inject_server(workdir: &Path, link: InjectLink) -> Result<InjectServer> {
        let dir = super::inject_socket_dir(workdir);
        std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
        let pid = std::process::id();
        let home = dir.join(format!("{pid}.sock"));
        let mut server = InjectServer {
            shutdown: CancellationToken::new(),
            pointer: socket_pointer_path(&home),
            socket: home,
            token: dir.join(format!("{pid}.token")),
        };
        // What an earlier process with this pid left behind.
        let _ = std::fs::remove_file(&server.socket);
        // The token goes first, so every client that finds the socket also
        // finds the token it has to present.
        let token: Arc<str> = mint_hub_token().into();
        write_hub_token(&server.token, &token)?;
        // Owner-only (0600), at its home or, when that is too long for a
        // Unix socket, at the path its pointer file names.
        let (listener, socket) = bind_socket(workdir, &server.socket)
            .with_context(|| format!("bind inject socket {}", server.socket.display()))?;
        server.socket = socket;
        let exchange = Arc::new(tokio::sync::Mutex::new(Exchange::new(link)));
        tokio::spawn(accept_requests(
            listener,
            token,
            exchange,
            server.shutdown.clone(),
        ));
        Ok(server)
    }

    /// Serve each connection until `shutdown`.
    async fn accept_requests(
        listener: UnixListener,
        token: Arc<str>,
        exchange: Arc<tokio::sync::Mutex<Exchange>>,
        shutdown: CancellationToken,
    ) {
        loop {
            let stream = tokio::select! {
                () = shutdown.cancelled() => return,
                accepted = listener.accept() => match accepted {
                    Ok((stream, _)) => stream,
                    Err(error) => {
                        tracing::warn!(%error, "the inject socket stopped accepting");
                        return;
                    }
                },
            };
            let token = Arc::clone(&token);
            let exchange = Arc::clone(&exchange);
            tokio::spawn(async move {
                if let Err(error) = serve_request(stream, &token, &exchange).await {
                    tracing::debug!(%error, "an inject request ended without an answer");
                }
            });
        }
    }

    /// Read one request from `stream`, once its hello carries `token`, and
    /// write back the run's answer.
    async fn serve_request(
        mut stream: UnixStream,
        token: &str,
        exchange: &tokio::sync::Mutex<Exchange>,
    ) -> Result<()> {
        let hello = tokio::time::timeout(
            FRAME_TIMEOUT,
            read_frame::<InjectHello>(&mut stream, MAX_HELLO_FRAME_BYTES),
        )
        .await
        .context("no hello in time")??;
        anyhow::ensure!(
            tokens_match(&hello.token, token),
            "the inject token does not match"
        );
        let request = tokio::time::timeout(
            FRAME_TIMEOUT,
            read_frame::<InjectWireRequest>(&mut stream, MAX_REQUEST_FRAME_BYTES),
        )
        .await
        .context("no request in time")??;
        // One exchange with the run at a time, so each acknowledgement meets
        // the request it answers.
        let reply = exchange.lock().await.answer(&request).await;
        write_frame(&mut stream, &reply).await
    }

    /// The socket's side of the run's inject command channel.
    struct Exchange {
        commands: ExecutionCommandSender,
        acks: mpsc::Receiver<CommandAck>,
        target: InjectTarget,
        answer_timeout: Duration,
        /// Accepted requests' answers, by request id, so a request sent
        /// again gets its answer again rather than a second delivery.
        answered: HashMap<String, InjectWireReply>,
        answered_order: VecDeque<String>,
    }

    impl Exchange {
        fn new(link: InjectLink) -> Self {
            Self {
                commands: link.commands,
                acks: link.acks,
                target: link.target,
                answer_timeout: link.answer_timeout,
                answered: HashMap::new(),
                answered_order: VecDeque::new(),
            }
        }

        /// Send `request` to the run as a command for the running plan its
        /// session names, and answer with the run's acknowledgement.
        async fn answer(&mut self, request: &InjectWireRequest) -> InjectWireReply {
            if let Some(reply) = self.answered.get(&request.request_id) {
                return reply.clone();
            }
            let (kind, plan_id) = if let Some(kind) = plan_control_kind(&request.kind) {
                // The driver knows best which plan a control command can
                // reach: a cancel reaches a plan that has not started, and a
                // retry one that has ended.
                let plan_id = Some(request.session.clone()).filter(|plan| !plan.is_empty());
                (kind, plan_id)
            } else if request.kind == "raise_budget" {
                // `roko plan budget raise` (backlog 2118): the driver raises
                // the ceiling of the running plan the session names.
                let Some(ceiling_micro_usd) = request
                    .payload
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .and_then(crate::graph_task_dispatch::plan_ceiling_micro_usd)
                else {
                    return InjectWireReply::new(
                        request,
                        InjectOutcome::Rejected,
                        "a budget raise needs a positive amount in USD",
                    );
                };
                let kind = ExecutionCommandKind::RaiseBudget {
                    ceiling_micro_usd,
                    requested_by: "roko plan budget raise".to_string(),
                };
                let plan_id = Some(request.session.clone()).filter(|plan| !plan.is_empty());
                (kind, plan_id)
            } else {
                let Some(plan_id) = (self.target)(&request.session) else {
                    return InjectWireReply::new(
                        request,
                        InjectOutcome::UnknownSession,
                        format!(
                            "this run has no running plan or checkpoint run '{}'",
                            request.session
                        ),
                    );
                };
                let kind = match request.kind.as_str() {
                    "directive" => ExecutionCommandKind::Inject {
                        kind: InjectedKind::Directive,
                        text: InjectedText::new(request.payload.clone()),
                    },
                    "context" => ExecutionCommandKind::Inject {
                        kind: InjectedKind::Context,
                        text: InjectedText::new(request.payload.clone()),
                    },
                    "abort" => ExecutionCommandKind::Cancel,
                    other => {
                        return InjectWireReply::new(
                            request,
                            InjectOutcome::Rejected,
                            format!("unknown inject kind '{other}'"),
                        );
                    }
                };
                (kind, Some(plan_id))
            };
            let mut command = self.commands.build_command(kind, plan_id, None, None);
            command.command_id.clone_from(&request.request_id);
            if let Err(error) = self.commands.try_send(command) {
                let reason = match error {
                    CommandSendError::Full(_) => "the plan run is busy; send it again",
                    CommandSendError::Disconnected(_) => "the plan run has finished",
                };
                return InjectWireReply::new(request, InjectOutcome::Rejected, reason);
            }
            let acknowledged =
                tokio::time::timeout(self.answer_timeout, self.ack_for(&request.request_id)).await;
            let reply = match acknowledged {
                Ok(Some(ack)) => InjectWireReply::for_ack(request, &ack),
                Ok(None) => InjectWireReply::new(
                    request,
                    InjectOutcome::Rejected,
                    "the plan run has finished",
                ),
                Err(_) => InjectWireReply::new(
                    request,
                    InjectOutcome::Rejected,
                    "the plan run did not answer in time",
                ),
            };
            if reply.outcome == InjectOutcome::Accepted {
                self.remember(reply.clone());
            }
            reply
        }

        /// The run's acknowledgement of command `command_id`, passing over
        /// those of commands whose wait ran out. `None` once the run has
        /// finished.
        async fn ack_for(&mut self, command_id: &str) -> Option<CommandAck> {
            loop {
                let ack = self.acks.recv().await?;
                if ack.command_id == command_id {
                    return Some(ack);
                }
            }
        }

        /// Keep `reply`, forgetting the oldest beyond the bound.
        fn remember(&mut self, reply: InjectWireReply) {
            if self.answered_order.len() >= MAX_REMEMBERED_ANSWERS
                && let Some(oldest) = self.answered_order.pop_front()
            {
                self.answered.remove(&oldest);
            }
            self.answered_order.push_back(reply.request_id.clone());
            self.answered.insert(reply.request_id.clone(), reply);
        }
    }

    /// The command of a plan control request kind (`roko plan pause`,
    /// `resume`, `cancel` and `retry`, 1209), or `None` for an inject kind.
    fn plan_control_kind(kind: &str) -> Option<ExecutionCommandKind> {
        match kind {
            "pause" => Some(ExecutionCommandKind::Pause),
            "resume" => Some(ExecutionCommandKind::Resume),
            "cancel" => Some(ExecutionCommandKind::Cancel),
            "retry" => Some(ExecutionCommandKind::SoftRetry),
            _ => None,
        }
    }

    /// Ask each plan run listening in `workdir`, in turn, to take `request`,
    /// and return the answer of the one that runs the plan or checkpoint
    /// run it names. A run that does not answer in time, or answers another
    /// request, counts as not listening. `None` when no run answered;
    /// otherwise the answer that took or refused the request, else the last
    /// "unknown session".
    pub async fn deliver(workdir: &Path, request: &InjectWireRequest) -> Option<InjectWireReply> {
        let mut sockets: Vec<PathBuf> = std::fs::read_dir(super::inject_socket_dir(workdir))
            .ok()?
            .filter_map(Result::ok)
            .filter_map(|entry| socket_home(&entry.path()))
            .collect();
        sockets.sort();
        sockets.dedup();
        let mut unknown = None;
        for socket in sockets {
            match ask(&socket, request).await {
                Ok(reply) if reply.outcome == InjectOutcome::UnknownSession => {
                    unknown = Some(reply);
                }
                Ok(reply) => return Some(reply),
                Err(error) => tracing::debug!(
                    socket = %socket.display(),
                    %error,
                    "a plan run's inject socket did not answer"
                ),
            }
        }
        unknown
    }

    /// The home of the run socket an inject directory entry stands for: the
    /// socket itself (`<pid>.sock`), or the pointer file of one bound
    /// elsewhere (`<pid>.sock.path`, 1224).
    fn socket_home(entry: &Path) -> Option<PathBuf> {
        let home = if entry.extension().is_some_and(|ext| ext == "path") {
            entry.with_extension("")
        } else {
            entry.to_path_buf()
        };
        home.extension()
            .is_some_and(|ext| ext == "sock")
            .then_some(home)
    }

    /// One exchange with the run whose socket's home is `socket`.
    async fn ask(socket: &Path, request: &InjectWireRequest) -> Result<InjectWireReply> {
        let token = std::fs::read_to_string(socket.with_extension("token"))
            .context("read the run's inject token")?;
        let bound = bound_socket_path(socket);
        let mut stream = tokio::time::timeout(FRAME_TIMEOUT, UnixStream::connect(&bound))
            .await
            .context("no connection in time")??;
        let hello = InjectHello {
            token: token.trim().to_string(),
        };
        write_frame(&mut stream, &hello).await?;
        write_frame(&mut stream, request).await?;
        let reply = tokio::time::timeout(
            super::INJECT_ANSWER_TIMEOUT + FRAME_TIMEOUT,
            read_frame::<InjectWireReply>(&mut stream, MAX_REPLY_FRAME_BYTES),
        )
        .await
        .context("no answer in time")??;
        anyhow::ensure!(
            reply.request_id == request.request_id,
            "the run answered another request"
        );
        Ok(reply)
    }

    #[cfg(test)]
    mod tests {
        use tempfile::tempdir;

        use super::*;
        use crate::execution_control::{CommandAckStatus, ExecutionCommand, ack_for};

        const TEXT: &str = "keep the API stable";

        fn request(session: &str) -> InjectWireRequest {
            InjectWireRequest {
                request_id: format!("req-{session}"),
                session: session.to_string(),
                kind: "directive".to_string(),
                payload: TEXT.to_string(),
            }
        }

        /// The run's end of an [`InjectLink`].
        struct RunSide {
            commands: mpsc::Receiver<ExecutionCommand>,
            acks: mpsc::Sender<CommandAck>,
        }

        impl RunSide {
            /// Take the next command and accept it.
            async fn accept_next(&mut self) -> ExecutionCommand {
                let command = self.commands.recv().await.expect("a command");
                let ack = ack_for(&command, CommandAckStatus::Accepted, Some("queued".into()));
                self.acks.send(ack).await.expect("acknowledge");
                command
            }
        }

        /// A link to a run whose only running plan is `plan-1`, with the
        /// socket waiting `answer_timeout` for its acknowledgements.
        fn link(answer_timeout: Duration) -> (InjectLink, RunSide) {
            let (commands, command_rx, ack_tx, acks) =
                ExecutionCommandSender::channel("graph-engine");
            let target: InjectTarget =
                Arc::new(|session: &str| (session == "plan-1").then(|| session.to_string()));
            let link = InjectLink {
                commands,
                acks,
                target,
                answer_timeout,
            };
            let run = RunSide {
                commands: command_rx,
                acks: ack_tx,
            };
            (link, run)
        }

        /// gap-f118b3: a directive reaches the run as an inject command for
        /// the plan it names, under the request's id; the client gets the
        /// run's acknowledgement; once the run stops listening, no answer.
        #[tokio::test]
        async fn a_directive_reaches_the_run_and_gets_its_acknowledgement() {
            let workdir = tempdir().expect("tempdir");
            let (link, mut run) = link(Duration::from_secs(5));
            let server = start_inject_server(workdir.path(), link).expect("listen");
            let accepted = tokio::spawn(async move { run.accept_next().await });

            let reply = deliver(workdir.path(), &request("plan-1"))
                .await
                .expect("an answer");

            assert_eq!(reply.outcome, InjectOutcome::Accepted);
            assert_eq!(reply.message, "queued");
            let command = accepted.await.expect("run");
            assert_eq!(command.command_id, "req-plan-1");
            assert_eq!(command.plan_id.as_deref(), Some("plan-1"));
            let ExecutionCommandKind::Inject { kind, text } = &command.kind else {
                panic!("an inject command, got {}", command.kind);
            };
            assert_eq!(*kind, InjectedKind::Directive);
            assert_eq!(text.as_str(), TEXT);
            drop(server);
            assert!(
                deliver(workdir.path(), &request("plan-1")).await.is_none(),
                "nothing listens once the run has stopped"
            );
        }

        /// gap-f118b3: a request sent again is answered again from what the
        /// run said the first time, and the run sees it once.
        #[tokio::test]
        async fn a_request_sent_again_is_not_delivered_again() {
            let workdir = tempdir().expect("tempdir");
            let (link, mut run) = link(Duration::from_secs(5));
            let _server = start_inject_server(workdir.path(), link).expect("listen");
            let accepted = tokio::spawn(async move {
                run.accept_next().await;
                run
            });

            let first = deliver(workdir.path(), &request("plan-1")).await;
            let mut run = accepted.await.expect("run");
            let again = deliver(workdir.path(), &request("plan-1")).await;

            assert_eq!(first, again);
            let outcome = again.map(|reply| reply.outcome);
            assert_eq!(outcome, Some(InjectOutcome::Accepted));
            assert!(run.commands.try_recv().is_err(), "delivered once");
        }

        /// gap-f118b3: an unknown session, a run that does not acknowledge
        /// in time and a run that has finished are all refusals.
        #[tokio::test]
        async fn unknown_silent_and_finished_runs_refuse() {
            let workdir = tempdir().expect("tempdir");
            let (link, run) = link(Duration::from_millis(200));
            let _server = start_inject_server(workdir.path(), link).expect("listen");

            let unknown = deliver(workdir.path(), &request("plan-9")).await;
            assert_eq!(
                unknown.map(|reply| reply.outcome),
                Some(InjectOutcome::UnknownSession)
            );
            let silent = deliver(workdir.path(), &request("plan-1"))
                .await
                .expect("an answer");
            assert_eq!(silent.outcome, InjectOutcome::Rejected);
            assert!(silent.message.contains("in time"), "{}", silent.message);

            drop(run);
            let mut after = request("plan-1");
            after.request_id = "req-after".to_string();
            let ended = deliver(workdir.path(), &after).await.expect("an answer");
            assert_eq!(ended.outcome, InjectOutcome::Rejected);
            assert!(ended.message.contains("finished"), "{}", ended.message);
        }

        /// 1209: `roko plan pause` reaches the whole run, and a cancel
        /// reaches the plan it names, running or not: the run's driver
        /// decides, and its answer comes back.
        #[tokio::test]
        async fn plan_control_commands_reach_the_run_without_a_session_lookup() {
            let workdir = tempdir().expect("tempdir");
            let (link, mut run) = link(Duration::from_secs(5));
            let _server = start_inject_server(workdir.path(), link).expect("listen");
            let accepted = tokio::spawn(async move {
                let pause = run.accept_next().await;
                let cancel = run.accept_next().await;
                (pause, cancel)
            });
            let control = |kind: &str, plan: &str| InjectWireRequest {
                request_id: format!("req-{kind}"),
                session: plan.to_string(),
                kind: kind.to_string(),
                payload: String::new(),
            };

            let paused = deliver(workdir.path(), &control("pause", ""))
                .await
                .expect("an answer");
            let cancelled = deliver(workdir.path(), &control("cancel", "plan-9"))
                .await
                .expect("an answer");

            assert_eq!(paused.outcome, InjectOutcome::Accepted);
            assert_eq!(cancelled.outcome, InjectOutcome::Accepted);
            let (pause, cancel) = accepted.await.expect("run");
            assert_eq!(pause.kind, ExecutionCommandKind::Pause);
            assert_eq!(pause.plan_id, None, "a pause applies to the whole run");
            assert_eq!(cancel.kind, ExecutionCommandKind::Cancel);
            assert_eq!(cancel.plan_id.as_deref(), Some("plan-9"));
        }

        /// backlog 2118: `roko plan budget raise` reaches the run as a raise
        /// of the plan it names, carrying the new ceiling in micro-USD. A
        /// raise that names no positive amount is refused before the run
        /// sees it.
        #[tokio::test]
        async fn a_budget_raise_reaches_the_run_with_its_ceiling() {
            let workdir = tempdir().expect("tempdir");
            let (link, mut run) = link(Duration::from_secs(5));
            let _server = start_inject_server(workdir.path(), link).expect("listen");
            let raise = |id: &str, payload: &str| InjectWireRequest {
                request_id: id.to_string(),
                session: "plan-9".to_string(),
                kind: "raise_budget".to_string(),
                payload: payload.to_string(),
            };

            let refused = deliver(workdir.path(), &raise("req-nan", "ten dollars"))
                .await
                .expect("an answer");
            assert_eq!(refused.outcome, InjectOutcome::Rejected);
            assert!(run.commands.try_recv().is_err(), "the run never saw it");

            let accepted = tokio::spawn(async move { run.accept_next().await });
            let raised = deliver(workdir.path(), &raise("req-raise", " 0.25 "))
                .await
                .expect("an answer");
            assert_eq!(raised.outcome, InjectOutcome::Accepted);
            let command = accepted.await.expect("run");
            assert_eq!(command.plan_id.as_deref(), Some("plan-9"));
            assert_eq!(
                command.kind,
                ExecutionCommandKind::RaiseBudget {
                    ceiling_micro_usd: 250_000,
                    requested_by: "roko plan budget raise".to_string(),
                }
            );
        }

        /// 1224: in a workspace whose inject socket path is longer than a
        /// Unix socket allows, the run binds in a short private directory
        /// and names it in `<pid>.sock.path`; `deliver` follows the pointer,
        /// and the request reaches the run. Stopping the run removes the
        /// socket and the pointer.
        #[tokio::test]
        async fn inject_socket_binds_under_a_long_workspace_path() {
            let root = tempdir().expect("tempdir");
            let workdir = root.path().join("a-checkout-nested-deeply-enough".repeat(3));
            std::fs::create_dir_all(&workdir).expect("create the deep workspace");
            let home = super::super::inject_socket_dir(&workdir)
                .join(format!("{}.sock", std::process::id()));
            assert!(home.as_os_str().len() > 110, "{}", home.display());
            let (link, mut run) = link(Duration::from_secs(5));
            let server = start_inject_server(&workdir, link).expect("listen");
            let bound = bound_socket_path(&home);
            assert_ne!(bound, home, "the socket is bound away from its home");
            assert!(bound.exists(), "{}", bound.display());
            let accepted = tokio::spawn(async move { run.accept_next().await });

            let reply = deliver(&workdir, &request("plan-1"))
                .await
                .expect("an answer");

            assert_eq!(reply.outcome, InjectOutcome::Accepted);
            let command = accepted.await.expect("run");
            assert_eq!(command.command_id, "req-plan-1");
            drop(server);
            assert!(!bound.exists(), "the socket is removed");
            assert!(!socket_pointer_path(&home).exists(), "the pointer is removed");
        }

        /// A client that cannot present the run's token gets no answer, and
        /// its request never reaches the run.
        #[tokio::test]
        async fn a_client_without_the_token_reaches_nothing() {
            let workdir = tempdir().expect("tempdir");
            let (link, mut run) = link(Duration::from_secs(5));
            let _server = start_inject_server(workdir.path(), link).expect("listen");
            let token = super::super::inject_socket_dir(workdir.path())
                .join(format!("{}.token", std::process::id()));
            std::fs::write(&token, "not-the-token").expect("replace the token");

            assert!(deliver(workdir.path(), &request("plan-1")).await.is_none());
            assert!(run.commands.try_recv().is_err(), "nothing reached the run");
        }
    }
}

#[cfg(unix)]
pub use unix::{InjectServer, deliver, start_inject_server};

/// Without Unix sockets no plan run listens for `roko inject`.
#[cfg(not(unix))]
pub struct InjectServer;

/// Without Unix sockets no plan run listens: `None`.
#[cfg(not(unix))]
pub async fn deliver(_workdir: &Path, _request: &InjectWireRequest) -> Option<InjectWireReply> {
    None
}

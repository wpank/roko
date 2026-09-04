//! agent command handlers.

use crate::*;

pub(crate) async fn cmd_agent(cli: &Cli, cmd: AgentCmd) -> Result<i32> {
    let workdir = resolve_workdir(cli);
    prepare_runtime_hooks(&workdir, cli.quiet);

    // Managed agent lifecycle operations write manifests, process registry,
    // and runtime state. Serialize those mutations with plan/run/serve.
    let _workspace_lock = match &cmd {
        AgentCmd::Create { workdir, .. }
        | AgentCmd::Delete { workdir, .. }
        | AgentCmd::Start { workdir, .. }
        | AgentCmd::Stop { workdir, .. } => {
            let workdir = workdir.clone().unwrap_or_else(|| resolve_workdir(cli));
            Some(roko_cli::workspace_lock::acquire_workspace_lock(
                &workdir.join(".roko"),
            )?)
        }
        _ => None,
    };

    agent_serve::run(cmd).await?;
    Ok(EXIT_SUCCESS)
}

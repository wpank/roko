//! history command handler.

use crate::*;

pub(crate) fn cmd_history(cli: &Cli, id: Option<String>, workdir: Option<PathBuf>) -> Result<i32> {
    let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
    let truncate = |value: &str, max_chars: usize| -> String {
        value.chars().take(max_chars).collect()
    };

    match id {
        None => {
            let sessions = roko_cli::chat_history::list_sessions(&wd, 20);
            if sessions.is_empty() {
                println!(
                    "no chat sessions found in {}",
                    roko_cli::chat_history::sessions_dir(&wd).display()
                );
            } else {
                println!(
                    "{:<40} {:<16} {:<8} {}",
                    "session", "model", "turns", "started"
                );
                println!("{}", "-".repeat(80));
                for session in &sessions {
                    println!(
                        "{:<40} {:<16} {:<8} {}",
                        truncate(&session.session_id, 40),
                        truncate(&session.model_key, 16),
                        session.turn_count,
                        truncate(&session.started_at, 19),
                    );
                }
            }
        }
        Some(id) => match roko_cli::chat_history::load_session(&wd, &id) {
            Some(session) => {
                println!("session_id:    {}", session.session_id);
                println!("agent_id:      {}", session.agent_id);
                println!("provider:      {}", session.provider);
                println!("model_key:     {}", session.model_key);
                println!("started_at:    {}", session.started_at);
                println!("ended_at:      {}", session.ended_at);
                println!("turn_count:    {}", session.turn_count);
                println!("total_tokens:  {}", session.total_tokens);
                println!("total_cost_usd:  {:?}", session.total_cost_usd);
                if !session.first_message.is_empty() {
                    println!("first_message: {}", session.first_message);
                }
                if !session.last_message.is_empty() {
                    println!("last_message:  {}", session.last_message);
                }
            }
            None => {
                eprintln!("session not found: {id}");
                if matches!(id.to_ascii_lowercase().trim(), "list" | "ls" | "all") {
                    eprintln!("Hint: `roko history` (no argument) lists sessions.");
                }
                return Ok(EXIT_FAILURE);
            }
        },
    }

    Ok(EXIT_SUCCESS)
}

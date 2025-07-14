use colored::*;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::sync::CancellationToken;

use crate::runner::models::CargoMessage;

/// Extracts and formats compiler errors from `cargo` JSON output.
pub fn format_build_error_output(raw_output: &str) -> String {
    let error_messages: Vec<String> = raw_output
        .lines()
        .filter_map(|line| serde_json::from_str::<CargoMessage>(line).ok())
        .filter_map(|msg| {
            if msg.reason == "compiler-message" {
                if let Some(diag) = msg.message {
                    if diag.level == "error" {
                        // Prefer the colorful rendered output if available
                        return diag.rendered.or(Some(diag.message));
                    }
                }
            }
            None
        })
        .collect();

    if error_messages.is_empty() {
        // If we can't find a specific error, return a snippet of the raw output.
        let snippet = raw_output.lines().take(50).collect::<Vec<_>>().join("\n");
        format!(
            "{}\n\n{}",
            "Could not parse specific compiler errors. Raw output snippet:".yellow(),
            snippet
        )
    } else {
        error_messages.join("\n")
    }
}

/// Spawns a command and captures its stdout and stderr streams.
/// If a stop_signal is provided and set, it will attempt to kill the child process.
/// Returns the exit status, the combined output, and a boolean indicating if the process
/// was killed due to the cancellation token.
pub async fn spawn_and_capture(
    mut cmd: tokio::process::Command,
    stop_token: Option<CancellationToken>,
) -> (
    std::io::Result<std::process::ExitStatus>,
    String,
    bool, // was_cancelled
) {
    // Capture stdout and stderr
    let mut child = match cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            // If spawning fails, we return the error and an empty string for the output.
            return (Err(e), String::new(), false);
        }
    };

    let stdout = child
        .stdout
        .take()
        .expect("Failed to capture stdout of child process");
    let stderr = child
        .stderr
        .take()
        .expect("Failed to capture stderr of child process");

    let output = Arc::new(tokio::sync::Mutex::new(String::new()));

    let stdout_output = Arc::clone(&output);
    let stdout_handle = tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let mut output = stdout_output.lock().await;
            output.push_str(&line);
            output.push('\n');
        }
    });

    let stderr_output = Arc::clone(&output);
    let stderr_handle = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let mut output = stderr_output.lock().await;
            output.push_str(&line);
            output.push('\n');
        }
    });

    let (status, was_cancelled) = if let Some(token) = stop_token {
        tokio::select! {
            _ = token.cancelled() => {
                if let Err(e) = child.start_kill() {
                    eprintln!("Failed to kill child process: {}", e);
                }
                (child.wait().await, true)
            },
            status = child.wait() => {
                (status, false)
            }
        }
    } else {
        (child.wait().await, false)
    };

    stdout_handle.await.unwrap();
    stderr_handle.await.unwrap();

    (status, output.lock().await.clone(), was_cancelled)
}

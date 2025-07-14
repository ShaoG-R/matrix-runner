use colored::*;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::runner::i18n;
use crate::runner::i18n::I18nKey;
use crate::runner::models::CargoMessage;

/// Extracts and formats compiler errors from `cargo` JSON output.
/// It filters for compiler messages, extracts error diagnostics, and prefers
/// the colorful "rendered" output if available.
///
/// # Arguments
/// * `raw_output` - The raw string output from a `cargo` command.
///
/// # Returns
/// A formatted string containing only the error messages, or a snippet of the
/// raw output if no specific errors can be parsed.
///
/// 从 `cargo` 的 JSON 输出中提取并格式化编译器错误。
/// 它会筛选编译器消息，提取错误诊断，并优先使用带颜色的 "rendered" 输出（如果可用）。
///
/// # Arguments
/// * `raw_output` - `cargo` 命令的原始字符串输出。
///
/// # Returns
/// 一个格式化的字符串，仅包含错误消息；如果无法解析出特定错误，则返回原始输出的摘要。
pub fn format_build_error_output(raw_output: &str) -> String {
    let error_messages: Vec<String> = raw_output
        .lines()
        .filter_map(|line| serde_json::from_str::<CargoMessage>(line).ok())
        .filter_map(|msg| {
            if msg.reason == "compiler-message" {
                if let Some(diag) = msg.message {
                    if diag.level == "error" {
                        // Prefer the colorful rendered output if available
                        // 如果有带颜色的渲染输出，则优先使用
                        return diag.rendered.or(Some(diag.message));
                    }
                }
            }
            None
        })
        .collect();

    if error_messages.is_empty() {
        // If we can't find a specific error, return a snippet of the raw output.
        // This helps debug cases where cargo fails without a proper JSON error message.
        // 如果找不到特定的错误，则返回原始输出的摘要。
        // 这有助于调试 cargo 失败但没有正确 JSON 错误消息的情况。
        let snippet = raw_output.lines().take(50).collect::<Vec<_>>().join("\n");
        format!(
            "{}\n\n{}",
            i18n::t(I18nKey::CompilerErrorParseFailed).yellow(),
            snippet
        )
    } else {
        error_messages.join("\n")
    }
}

/// Spawns a command, captures its stdout and stderr.
/// The output streams are read concurrently and combined into a single string.
///
/// # Arguments
/// * `cmd` - The `tokio::process::Command` to execute.
///
/// # Returns
/// A tuple containing:
/// - The `ExitStatus` of the process wrapped in an `io::Result`.
/// - The combined stdout and stderr as a `String`.
///
/// 派生一个命令，捕获其 stdout 和 stderr。
/// 输出流被并发读取并合并到一个字符串中。
///
/// # Arguments
/// * `cmd` - 要执行的 `tokio::process::Command`。
///
/// # Returns
/// 一个元组，包含：
/// - 进程的 `ExitStatus`（包装在 `io::Result` 中）。
/// - 合并的 stdout 和 stderr，为一个 `String`。
pub async fn spawn_and_capture(
    mut cmd: tokio::process::Command,
) -> (std::io::Result<std::process::ExitStatus>, String) {
    // Configure the command to capture stdout and stderr.
    // 配置命令以捕获 stdout 和 stderr。
    let mut child = match cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            // If spawning fails, we return the error and an empty string for the output.
            // 如果派生失败，我们返回错误和空字符串作为输出。
            return (Err(e), String::new());
        }
    };

    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            return (
                Err(std::io::Error::other(i18n::t(I18nKey::CaptureStdoutFailed))),
                String::new(),
            );
        }
    };
    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            return (
                Err(std::io::Error::other(i18n::t(I18nKey::CaptureStderrFailed))),
                String::new(),
            );
        }
    };

    // Use an Arc<Mutex<String>> to allow concurrent writes from stdout and stderr tasks.
    // 使用 Arc<Mutex<String>> 来允许多个任务（stdout 和 stderr）并发写入。
    let output = Arc::new(tokio::sync::Mutex::new(String::new()));

    // Spawn a task to read stdout line by line.
    // 派生一个任务来逐行读取 stdout。
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

    // Spawn a task to read stderr line by line.
    // 派生一个任务来逐行读取 stderr。
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

    // Wait for the process to exit.
    // 等待进程退出。
    let status = child.wait().await;

    // Wait for the stdout and stderr reading tasks to complete to ensure all output is captured.
    // 等待 stdout 和 stderr 读取任务完成，以确保所有输出都被捕获。
    if let Err(e) = stdout_handle.await {
        eprintln!("Failed to join stdout task: {}", e);
    }
    if let Err(e) = stderr_handle.await {
        eprintln!("Failed to join stderr task: {}", e);
    }

    (status, output.lock().await.clone())
}

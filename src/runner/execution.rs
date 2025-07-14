use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::path::PathBuf;

use crate::runner::command::spawn_and_capture;
use crate::runner::config::TestCase;
use crate::runner::i18n;
use crate::runner::i18n::I18nKey;
use crate::runner::models::CargoMessage;
use crate::runner::models::{BuiltTest, FailureReason, TestResult};
use crate::runner::utils::{copy_dir_all, create_build_dir};

/// Runs a complete test case, which involves two main phases: building and running.
/// This function orchestrates the entire lifecycle for a single test configuration.
/// If the build phase fails, the function will return early with a build failure.
///
/// # Arguments
/// * `case` - The `TestCase` configuration to run.
/// * `project_root` - The absolute path to the root of the project being tested.
/// * `crate_name` - The name of the crate being tested.
/// * `stop_token` - An optional `CancellationToken` to gracefully stop the execution.
///
/// # Returns
/// A `Result` containing either a successful `TestResult` or an error `TestResult`.
/// The `Err` variant is used to signal failure to the caller, which can then decide
/// whether to stop other parallel tests.
///
/// 运行一个完整的测试用例，包括两个主要阶段：构建和运行。
/// 此函数协调单个测试配置的整个生命周期。
/// 如果构建阶段失败，函数将提前返回一个构建失败结果。
///
/// # Arguments
/// * `case` - 要运行的 `TestCase` 配置。
/// * `project_root` - 被测试项目根目录的绝对路径。
/// * `crate_name` - 被测试的 crate 名称。
/// * `stop_token` - 一个可选的 `CancellationToken`，用于优雅地停止执行。
///
/// # Returns
/// 一个 `Result`，包含成功的 `TestResult` 或错误的 `TestResult`。
/// `Err` 变体用于向调用者发信号表示失败，调用者可以据此决定是否停止其他并行测试。
pub async fn run_test_case(
    case: TestCase,
    project_root: &PathBuf,
    crate_name: &str,
) -> Result<TestResult> {
    // If a custom command is provided, we use the custom command flow.
    // The command is expected to handle both "build" and "test" logic.
    if let Some(custom_command) = &case.command {
        println!(
            "{}",
            i18n::t_fmt(I18nKey::RunningTest, &[&case.name]).blue()
        );

        let start_time = std::time::Instant::now();

        let expanded_command = shellexpand::full(custom_command)
            .with_context(|| format!("Failed to expand command: {custom_command}"))?
            .to_string();

        let parts = shlex::split(&expanded_command)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse command: {}", expanded_command))?;

        if parts.is_empty() {
            return Err(anyhow::anyhow!("Empty command after parsing."));
        }

        let program = &parts[0];
        let args = &parts[1..];

        let mut cmd = tokio::process::Command::new(program);
        cmd.args(args);
        cmd.kill_on_drop(true);
        cmd.current_dir(project_root);

        let (status_res, output) = spawn_and_capture(cmd).await;
        let status = status_res.context("Failed to get process status")?;
        let duration = start_time.elapsed();

        // Always print the output from the custom command
        if !output.trim().is_empty() {
            println!("{}", output.trim());
        }

        if status.success() {
            println!(
                "{}",
                i18n::t_fmt(
                    I18nKey::TestPassed,
                    &[&case.name, &format!("{duration:.2?}")]
                )
                .green()
            );
            return Ok(TestResult::Passed { case, output });
        } else {
            println!(
                "{}",
                i18n::t_fmt(
                    I18nKey::TestFailed,
                    &[&case.name, &format!("{duration:.2?}")]
                )
                .red()
            );
            // For custom commands, we don't have artifacts to preserve in the same way,
            // so we just return the failure. The output from the command is the primary artifact.
            return Ok(TestResult::Failed {
                case,
                output,
                reason: FailureReason::Test, // Or a new `CustomCommand` reason
            });
        }
    }

    // --- Default flow (no custom command) ---
    let built_test = match build_test_case(case.clone(), project_root, crate_name).await {
        Ok(built_test) => built_test,
        Err(e) => {
            return Ok(TestResult::Failed {
                case,
                output: e.to_string(),
                reason: FailureReason::Build,
            });
        }
    };

    Ok(run_built_test(built_test, project_root).await?)
}

/// Builds a single test case using `cargo test --no-run`.
/// It creates a temporary, isolated directory for the build artifacts to prevent
/// interference between parallel test runs.
///
/// # Returns
/// On success, returns a `BuiltTest` struct containing the path to the executable
/// and the build context. On failure, returns a `TestResult` with `FailureReason::Build`.
///
/// 使用 `cargo test --no-run` 构建单个测试用例。
/// 它为构建产物创建一个临时的、隔离的目录，以防止并行测试运行之间发生干扰。
///
/// # Returns
/// 成功时，返回一个 `BuiltTest` 结构体，其中包含可执行文件的路径和构建上下文。
/// 失败时，返回一个带有 `FailureReason::Build` 的 `TestResult`。
async fn build_test_case(
    case: TestCase,
    project_root: &PathBuf,
    crate_name: &str,
) -> Result<BuiltTest> {
    println!(
        "{}",
        i18n::t_fmt(I18nKey::BuildingTest, &[&case.name]).blue()
    );

    let build_ctx = create_build_dir(crate_name, &case.features, case.no_default_features)?;

    let mut cmd = tokio::process::Command::new("cargo");
    cmd.kill_on_drop(true);
    cmd.current_dir(project_root);
    cmd.arg("test")
        .arg("--lib")
        .arg("--no-run")
        .arg("--message-format=json-diagnostic-rendered-ansi")
        .arg("--locked")
        .arg("--offline")
        .arg("--target-dir")
        .arg(&build_ctx.target_path);

    if case.no_default_features {
        cmd.arg("--no-default-features");
    }

    if !case.features.is_empty() {
        cmd.arg("--features").arg(&case.features);
    }

    let command_string = format!(
        "cargo test --lib --no-run --message-format=json-diagnostic-rendered-ansi --locked --offline --target-dir \"{}\" {} {}",
        build_ctx.target_path.display(),
        if case.no_default_features { "--no-default-features" } else { "" },
        if !case.features.is_empty() { format!("--features \"{}\"", case.features) } else { "".to_string() }
    ).split_whitespace().collect::<Vec<&str>>().join(" ");

    let (status_res, output) = spawn_and_capture(cmd).await;
    let status = status_res.context("Failed to get process status")?;

    if !status.success() {
        let sanitized_name = case
            .name
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>();
        let error_dir_path = project_root.join("target-errors").join(sanitized_name);

        println!(
            "{}\n  Command: {}",
            i18n::t_fmt(
                I18nKey::BuildFailedPreserving,
                &[&case.name, &error_dir_path.display()]
            )
            .yellow(),
            command_string.cyan()
        );

        if error_dir_path.exists() {
            fs::remove_dir_all(&error_dir_path)
                .context(i18n::t(I18nKey::CleanupOldArtifactsFailed).to_string())?;
        }

        copy_dir_all(&build_ctx.target_path, &error_dir_path)
            .context(i18n::t_fmt(I18nKey::CopyArtifactsFailed, &[&case.name, &""]).to_string())?;

        return Err(anyhow::anyhow!(output));
    }

    let executable = output
        .lines()
        .filter_map(|line| serde_json::from_str::<CargoMessage>(line).ok())
        .find_map(|msg| {
            if msg.reason == "compiler-artifact" {
                if let (Some(target), Some(executable_path)) = (msg.target, msg.executable) {
                    if target.name == crate_name && target.test {
                        return Some(executable_path);
                    }
                }
            }
            None
        })
        .context(i18n::t(I18nKey::FindExecutableFailed))?;

    println!(
        "{}",
        i18n::t_fmt(I18nKey::BuildSuccessful, &[&case.name]).green()
    );

    Ok(BuiltTest {
        case,
        executable,
        build_ctx,
    })
}

/// Runs a test executable that has already been built.
/// It captures the output and status of the executable. If the test fails,
/// its build artifacts are preserved for debugging.
///
/// # Returns
/// A `Result<TestResult, TestResult>` indicating the outcome. On success, the temporary
/// build directory is cleaned up. On failure, it is preserved.
///
/// 运行一个已经构建好的测试可执行文件。
/// 它会捕获可执行文件的输出和状态。如果测试失败，其构建产物将被保留以供调试。
///
/// # Returns
/// 一个 `Result<TestResult, TestResult>`，指示执行结果。成功时，临时构建目录会被清理。
/// 失败时，它将被保留。
async fn run_built_test(built_test: BuiltTest, project_root: &PathBuf) -> Result<TestResult> {
    let case = built_test.case;
    let executable = built_test.executable;
    let _build_ctx = built_test.build_ctx; // Transferred ownership for cleanup

    let start_time = std::time::Instant::now();
    println!(
        "{}",
        i18n::t_fmt(I18nKey::RunningTest, &[&case.name]).blue()
    );

    let mut cmd = tokio::process::Command::new(&executable);
    cmd.kill_on_drop(true);

    let (status_res, output) = spawn_and_capture(cmd).await;
    let status = status_res.context("Failed to get process status")?;

    let duration = start_time.elapsed();

    if status.success() {
        println!(
            "{}",
            i18n::t_fmt(
                I18nKey::TestPassed,
                &[&case.name, &format!("{duration:.2?}")]
            )
            .green()
        );
        Ok(TestResult::Passed { case, output })
    } else {
        println!(
            "{}",
            i18n::t_fmt(
                I18nKey::TestFailed,
                &[&case.name, &format!("{duration:.2?}")]
            )
            .red()
        );

        let sanitized_name = case
            .name
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>();
        let error_dir_path = project_root.join("target-errors").join(sanitized_name);

        if !error_dir_path.exists() {
            println!(
                "{}",
                i18n::t_fmt(I18nKey::PreservingArtifacts, &[&error_dir_path.display()]).yellow()
            );
            copy_dir_all(&_build_ctx.target_path, &error_dir_path).unwrap_or_else(|e| {
                eprintln!(
                    "{}",
                    i18n::t_fmt(I18nKey::CopyArtifactsFailed, &[&case.name, &e.to_string()])
                )
            });
        }

        Ok(TestResult::Failed {
            case,
            output,
            reason: FailureReason::Test,
        })
    }
}

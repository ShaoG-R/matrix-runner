//! # Test Execution Engine Module / 测试执行引擎模块
//!
//! This module provides the core functionality for executing test cases.
//! It handles the complete test lifecycle from building to execution,
//! including timeouts, retries, and result collection.
//!
//! 此模块为执行测试用例提供核心功能。
//! 它处理从构建到执行的完整测试生命周期，
//! 包括超时、重试和结果收集。

use anyhow::{Context, Result};
use colored::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::sync::mpsc;

use crate::{
    core::{
        config::TestCase,
        models::{BuildContext, BuiltTest, FailureReason, TestResult},
    },
    infra::{command, t},
};

/// The main entry point for running a single test case.
/// It wraps the core execution logic with timeout and retry handling.
///
/// # Arguments
/// * `case` - The test case configuration to execute
/// * `project_root` - Path to the project root directory
/// * `crate_name` - Name of the crate being tested
///
/// # Returns
/// A `TestResult` indicating the outcome of the test execution
pub async fn run_test_case(
    case: TestCase,
    project_root: &PathBuf,
    crate_name: &str,
    temp_dir_tx: mpsc::UnboundedSender<TempDir>,
) -> Result<TestResult> {
    let max_attempts = 1 + case.retries.unwrap_or(0);
    let mut last_result: Option<TestResult> = None;

    for attempt in 1..=max_attempts {
        let case_name = case.name.clone();
        let timeout_dur = case.timeout_secs.map(std::time::Duration::from_secs);

        let execution_future =
            run_test_case_inner(case.clone(), project_root, crate_name, temp_dir_tx.clone());

        let result = if let Some(duration) = timeout_dur {
            match tokio::time::timeout(duration, execution_future).await {
                Ok(res) => res,
                Err(_) => {
                    println!(
                        "{}",
                        t!("run.test_timeout", name = case_name, timeout = duration.as_secs()).red()
                    );
                    Ok(TestResult::Failed {
                        case: case.clone(),
                        output: t!("run.test_timeout_message").to_string(),
                        reason: FailureReason::Timeout,
                        duration,
                    })
                }
            }
        } else {
            execution_future.await
        };

        match result {
            Ok(TestResult::Passed {
                case,
                output,
                duration,
                ..
            }) => {
                let final_result = TestResult::Passed {
                    case,
                    output,
                    duration,
                    retries: attempt as u8,
                };
                if attempt > 1 {
                    println!(
                        "{}",
                        t!("run.test_passed_on_retry", name = case_name, retries = attempt - 1).green()
                    );
                }
                return Ok(final_result);
            }
            Ok(res) => {
                if res.is_timeout() {
                    return Ok(res);
                }
                if attempt < max_attempts {
                    println!(
                        "{}",
                        t!("run.test_retrying", name = case_name, attempt = attempt, retries = max_attempts - 1).yellow()
                    );
                } else {
                    println!(
                        "{}",
                        t!("run.test_failed_after_retries", name = case_name, retries = case.retries.unwrap_or(0)).red()
                    );
                }
                last_result = Some(res);
            }
            Err(e) => {
                eprintln!("A critical error occurred during test execution: {}", e);
                return Err(e.context(format!("Critical error in test case {}", case_name)));
            }
        }
    }
    Ok(last_result.unwrap_or(TestResult::Skipped))
}

/// Dispatches to the correct execution flow based on whether a custom command is present.
async fn run_test_case_inner(
    case: TestCase,
    project_root: &PathBuf,
    crate_name: &str,
    temp_dir_tx: mpsc::UnboundedSender<TempDir>,
) -> Result<TestResult> {
    if let Some(custom_command) = &case.command {
        run_custom_command_case(case.clone(), project_root, custom_command).await
    } else {
        run_default_flow_case(case, project_root, crate_name, temp_dir_tx).await
    }
}

/// Executes a test case defined by a custom shell command.
async fn run_custom_command_case(
    case: TestCase,
    project_root: &PathBuf,
    custom_command: &str,
) -> Result<TestResult> {
    println!(
        "{}",
        t!("run.running_test", name = case.name).blue()
    );

    let start_time = Instant::now();
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
    cmd.args(args).kill_on_drop(true).current_dir(project_root);

    let (status_res, output) = command::spawn_and_capture(cmd).await;
    let status = status_res.context("Failed to get process status")?;
    let duration = start_time.elapsed();

    let command_log = format!(
        "{} {}\n",
        t!("run.command_prefix").blue(),
        expanded_command
    );
    let output = format!("{command_log}{output}");

    if !output.trim().is_empty() {
        println!("{}", output.trim());
    }

    if status.success() {
        println!(
            "{}",
            t!("run.test_passed", name = &case.name, duration = &duration.as_secs_f64().to_string()).green()
        );
        Ok(TestResult::Passed {
            case,
            output,
            duration,
            retries: 1,
        })
    } else {
        println!(
            "{}",
            t!("run.test_failed", name = &case.name, duration = &duration.as_secs_f64().to_string()).red()
        );
        Ok(TestResult::Failed {
            case,
            output,
            reason: FailureReason::CustomCommand,
            duration,
        })
    }
}

/// Executes the default test flow: build the test, then run the resulting binary.
async fn run_default_flow_case(
    case: TestCase,
    project_root: &PathBuf,
    crate_name: &str,
    temp_dir_tx: mpsc::UnboundedSender<TempDir>,
) -> Result<TestResult> {
    match build_test_case(case.clone(), project_root, crate_name, temp_dir_tx).await {
        Ok(built_test) => run_built_test(built_test, project_root).await,
        Err(e) => {
            let error_string = e.to_string();
            let final_error_result = if let Ok(test_result) = e.downcast::<TestResult>() {
                test_result
            } else {
                println!(
                    "{}",
                    t!("run.build_failed_unexpected").red()
                );
                println!("  Error: {}", error_string);
                TestResult::Failed {
                    case,
                    output: error_string,
                    reason: FailureReason::BuildFailed,
                    duration: Duration::from_secs(0),
                }
            };
            Ok(final_error_result)
        }
    }
}

/// Builds a single test case using `cargo test --no-run`.
async fn build_test_case(
    case: TestCase,
    project_root: &PathBuf,
    crate_name: &str,
    temp_dir_tx: mpsc::UnboundedSender<TempDir>,
) -> Result<BuiltTest> {
    let (build_path, temp_dir) = crate::infra::fs::create_build_dir(project_root, &case.name)?;
    temp_dir_tx
        .send(temp_dir)
        .map_err(|e| anyhow::anyhow!("Failed to send temp dir through channel: {}", e))?;
    let build_ctx = BuildContext::new(build_path);
    let build_start_time = Instant::now();

    let mut cmd = tokio::process::Command::new("cargo");
    cmd.arg("test")
        .arg("--no-run")
        .arg("--message-format=json")
        .arg("--target-dir")
        .arg(&build_ctx.path)
        .arg("-p")
        .arg(crate_name);

    if case.no_default_features {
        cmd.arg("--no-default-features");
    }
    if !case.features.is_empty() {
        cmd.arg("--features").arg(&case.features);
    }

    cmd.kill_on_drop(true).current_dir(project_root);

    println!(
        "{}",
        t!("run.building_test", name = &case.name).blue()
    );

    let (status_res, output) = command::spawn_and_capture(cmd).await;
    let build_duration = build_start_time.elapsed();

    let status = status_res.with_context(|| "Failed to get build process status")?;

    if !status.success() {
        println!(
            "{}",
            t!("run.build_failed", duration = build_duration.as_secs_f64()).red()
        );

        // Format and return the error output
        let error_output = command::format_build_error_output(&output);
        return Err(anyhow::anyhow!(TestResult::Failed {
            case,
            output: error_output,
            reason: FailureReason::Build,
            duration: build_duration,
        }));
    }

    let mut test_binary: Option<PathBuf> = None;

    for line in output.lines() {
        if let Ok(message) = serde_json::from_str::<crate::core::models::CargoMessage>(line) {
            if let Some(artifact) = message.into_artifact() {
                if let (Some(target), Some(executable)) = (artifact.target, artifact.executable) {
                    if target.test {
                        test_binary = Some(executable);
                        break;
                    }
                }
            }
        }
    }

    let executable_path = test_binary.unwrap_or_default();
    println!(
        "{}",
        t!("run.build_success", duration = build_duration.as_secs_f64()).green()
    );

    Ok(BuiltTest::new(
        case,
        executable_path,
        build_duration,
        build_ctx,
    ))
}

/// Executes a previously built test binary.
async fn run_built_test(built_test: BuiltTest, project_root: &PathBuf) -> Result<TestResult> {
    let case = built_test.case.clone();
    if built_test.executable_path.as_os_str().is_empty() {
        println!(
            "{}",
            t!("run.test_no_binaries", name = case.name).yellow()
        );
        return Ok(TestResult::Passed {
            case,
            output: t!("run.test_no_binaries_message").to_string(),
            duration: built_test.duration,
            retries: 1,
        });
    }

    println!(
        "{}",
        t!("run.running_test", name = case.name).blue()
    );

    let mut cmd = tokio::process::Command::new(&built_test.executable_path);
    cmd.kill_on_drop(true).current_dir(project_root);

    let run_start_time = Instant::now();
    let (status_res, output) = command::spawn_and_capture(cmd).await;
    let run_duration = run_start_time.elapsed();
    let total_duration = built_test.duration + run_duration;

    let command_log = format!(
        "{} {}\n",
        t!("run.command_prefix").blue(),
        built_test.executable_path.display()
    );
    let output = format!("{command_log}{output}");

    let status = match status_res {
        Ok(s) => s,
        Err(e) => {
            return Err(anyhow::anyhow!(
                "Failed to get test process status for executable: '{}'. OS Error: {}",
                built_test.executable_path.display(),
                e
            ));
        }
    };

    if !output.trim().is_empty() {
        println!("{}", output.trim());
    }

    if status.success() {
        println!(
            "{}",
            t!(
                "run.test_passed",
                name = &case.name,
                duration = &total_duration.as_secs_f64().to_string()
            )
            .green()
        );
        Ok(TestResult::Passed {
            case,
            output,
            duration: total_duration,
            retries: 1,
        })
    } else {
        println!(
            "{}",
            t!(
                "run.test_failed",
                name = &case.name,
                duration = &total_duration.as_secs_f64().to_string()
            )
            .red()
        );
        Ok(TestResult::Failed {
            case,
            output,
            reason: FailureReason::TestFailed,
            duration: total_duration,
        })
    }
} 
//! # Test Execution Module
//!
//! This module provides the core functionality for executing test cases.
//! It handles the complete test lifecycle from building to execution,
//! including timeouts, retries, and result collection.

use anyhow::{bail, Context, Result};
use colored::*;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::timeout;

use crate::{
    runner::{
        command,
        config::TestCase,
        models::{BuildContext, BuiltTest, FailureReason, TestResult},
    },
    t,
};

/// The main entry point for running a single test case.
/// It wraps the core execution logic with timeout and retry handling.
pub async fn run_test_case(
    case: TestCase,
    project_root: &PathBuf,
    crate_name: &str,
) -> Result<TestResult> {
    let max_attempts = 1 + case.retries.unwrap_or(0);
    let mut last_result: Option<TestResult> = None;

    for attempt in 1..=max_attempts {
        let case_name = case.name.clone();
        let timeout = case.timeout_secs.map(std::time::Duration::from_secs);

        let execution_future = run_test_case_inner(case.clone(), project_root, crate_name);

        let result = if let Some(duration) = timeout {
            match tokio::time::timeout(duration, execution_future).await {
                Ok(res) => res,
                Err(_) => {
                    println!(
                        "{}",
                        t!("test_timeout", name = case_name, duration = duration.as_secs()).red()
                    );
                    Ok(TestResult::Failed {
                        case: case.clone(),
                        output: t!("test_timeout_message").to_string(),
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
                        t!("test_passed_on_retry", name = case_name, attempt = attempt - 1).green()
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
                        t!("test_retrying", name = case_name, attempt = attempt, total = max_attempts).yellow()
                    );
                } else {
                    println!(
                        "{}",
                        t!("test_failed_after_retries", name = case_name, retries = case.retries.unwrap_or(0)).red()
                    );
                }
                last_result = Some(res);
            }
            Err(e) => {
                eprintln!("A critical error occurred during test execution: {}", e);
                return Ok(TestResult::Skipped);
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
) -> Result<TestResult> {
    if let Some(custom_command) = &case.command {
        run_custom_command_case(case.clone(), project_root, custom_command).await
    } else {
        run_default_flow_case(case, project_root, crate_name).await
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
        t!("running_test", name = case.name).blue()
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
    cmd.args(args).kill_on_drop(true).current_dir(project_root);

    let (status_res, output) = command::spawn_and_capture(cmd).await;
    let status = status_res.context("Failed to get process status")?;
    let duration = start_time.elapsed();

    if !output.trim().is_empty() {
        println!("{}", output.trim());
    }

    if status.success() {
        println!(
            "{}",
            t!("test_passed", name = case.name, duration = duration.as_secs()).green()
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
            t!("test_failed", name = case.name, duration = duration.as_secs()).red()
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
) -> Result<TestResult> {
    match build_test_case(case.clone(), project_root, crate_name).await {
        Ok(built_test) => {
            if built_test.executable_path.as_os_str().is_empty() {
                println!(
                    "{}",
                    t!("test_no_binaries", name = case.name).yellow()
                );
                return Ok(TestResult::Passed {
                    case,
                    output: t!("test_no_binaries_message").to_string(),
                    duration: built_test.duration,
                    retries: 1,
                });
            }
            run_built_test(built_test, project_root).await
        }
        Err(e) => {
            let error_string = e.to_string();
            let final_error_result = if let Ok(test_result) = e.downcast::<TestResult>() {
                test_result
            } else {
                println!(
                    "{}",
                    t!("build_failed_unexpected", name = case.name).red()
                );
                println!("  Error: {}", error_string);
                TestResult::Failed {
                    case,
                    output: error_string,
                    reason: FailureReason::BuildFailed,
                    duration: std::time::Duration::from_secs(0),
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
) -> Result<BuiltTest> {
    let build_dir = crate::runner::utils::create_build_dir(project_root, &case.name)?;
    let build_start_time = std::time::Instant::now();

    let mut cmd = tokio::process::Command::new("cargo");
    cmd.arg("test")
        .arg("--no-run")
        .arg("--message-format=json")
        .arg("--target-dir")
        .arg(build_dir.path())
        .arg("-p")
        .arg(crate_name);

    if case.no_default_features {
        cmd.arg("--no-default-features");
    }
    if !case.features.is_empty() {
        cmd.arg("--features").arg(&case.features);
    }

    cmd.kill_on_drop(true).current_dir(project_root);

    let (status_res, output) = command::spawn_and_capture(cmd).await;
    let status = status_res.context("Failed to get cargo build status")?;
    let build_duration = build_start_time.elapsed();

    if !status.success() {
        let formatted_error = crate::runner::command::format_build_error_output(&output);
        return Err(anyhow::Error::new(TestResult::Failed {
            case,
            output: formatted_error,
            reason: FailureReason::Build,
            duration: build_duration,
        }));
    }

    let artifacts = output
        .lines()
        .filter_map(|line| serde_json::from_str::<crate::runner::models::CargoMessage>(line).ok())
        .filter_map(|msg| msg.into_artifact());

    let mut executables = Vec::new();
    for artifact in artifacts {
        if let Some(target) = artifact.target {
            if target.kind.iter().any(|k| k == "test") {
                executables.extend(artifact.filenames.into_iter());
            }
        }
    }

    Ok(BuiltTest {
        case,
        executable_path: executables.get(0).cloned().unwrap_or_default(),
        duration: build_duration,
    })
}

/// Executes a test that has already been built.
async fn run_built_test(built_test: BuiltTest, project_root: &PathBuf) -> Result<TestResult> {
    let case = built_test.case;
    let test_executable = built_test.executable_path;

    println!(
        "{}",
        t!("running_test", name = case.name).blue()
    );

    let start_time = std::time::Instant::now();
    let mut cmd = tokio::process::Command::new(&test_executable);
    cmd.kill_on_drop(true).current_dir(project_root);

    let (status_res, output) = command::spawn_and_capture(cmd).await;
    let status = status_res.context("Failed to get process status")?;
    let duration = start_time.elapsed();

    if status.success() {
        println!(
            "{}",
            t!("test_passed", name = case.name, duration = duration.as_secs()).green()
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
            t!("test_failed", name = case.name, duration = duration.as_secs()).red()
        );
        Ok(TestResult::Failed {
            case,
            output,
            reason: FailureReason::TestFailed,
            duration,
        })
    }
}

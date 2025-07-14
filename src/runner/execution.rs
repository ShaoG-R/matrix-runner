use colored::*;
use std::fs;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

use crate::runner::command::spawn_and_capture;
use crate::runner::config::TestCase;
use crate::runner::models::CargoMessage;
use crate::runner::models::{BuiltTest, FailureReason, TestResult};
use crate::runner::utils::{copy_dir_all, create_build_dir};

/// Runs a test case, including building and running it.
/// This function encapsulates the entire lifecycle of a single test configuration.
pub async fn run_test_case(
    case: TestCase,
    project_root: PathBuf,
    crate_name: String,
    stop_token: Option<CancellationToken>,
) -> Result<TestResult, TestResult> {
    // First, try to build the test case.
    // The `?` operator will propagate the `Err(TestResult)` if the build fails.
    let built_test =
        build_test_case(case.clone(), project_root, crate_name, stop_token.clone()).await?;

    // If the build is successful, run the test.
    run_built_test(built_test, stop_token).await
}

async fn build_test_case(
    case: TestCase,
    project_root: PathBuf,
    crate_name: String,
    stop_token: Option<CancellationToken>,
) -> Result<BuiltTest, TestResult> {
    println!("{}", format!("Building test: {}", case.name).blue());

    let build_ctx = create_build_dir(&case.features, case.no_default_features);

    let mut cmd = tokio::process::Command::new("cargo");
    cmd.kill_on_drop(true);
    cmd.current_dir(&project_root);
    cmd.arg("test")
        .arg("--lib")
        .arg("--no-run") // Build but don't run
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

    let command_string = format!("cargo test --lib --no-run --message-format=json-diagnostic-rendered-ansi --locked --offline --target-dir \"{}\" {} {}",
        build_ctx.target_path.display(),
        if case.no_default_features { "--no-default-features" } else { "" },
        if !case.features.is_empty() { format!("--features \"{}\"", case.features) } else { "".to_string() }
    ).split_whitespace().collect::<Vec<&str>>().join(" ");

    let (status_res, output, was_cancelled) = spawn_and_capture(cmd, stop_token).await;
    let status = status_res.expect("Error waiting for process to complete");

    if !status.success() {
        let failure_reason = if was_cancelled {
            FailureReason::Cancelled
        } else {
            FailureReason::Build
        };

        // Only preserve artifacts for genuine build failures, not cancellations.
        if failure_reason == FailureReason::Build {
            let sanitized_name = case
                .name
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                .collect::<String>();
            let error_dir_path = project_root.join("target-errors").join(sanitized_name);

            println!(
                "{}\n  Command: {}",
                format!(
                    "Build for '{}' failed. Preserving build artifacts in: {}",
                    case.name,
                    error_dir_path.display()
                )
                .yellow(),
                command_string.cyan()
            );

            if error_dir_path.exists() {
                fs::remove_dir_all(&error_dir_path)
                    .expect("Failed to clean up old error artifacts directory");
            }

            copy_dir_all(&build_ctx.target_path, &error_dir_path).unwrap_or_else(|e| {
                eprintln!("Failed to copy error artifacts for '{}': {}", case.name, e)
            });
        }

        return Err(TestResult {
            case,
            output,
            success: false,
            failure_reason: Some(failure_reason),
        });
    }

    // Find the executable from the cargo JSON output
    let executable = output
        .lines()
        .filter_map(|line| serde_json::from_str::<CargoMessage>(line).ok())
        .find_map(|msg| {
            if msg.reason == "compiler-artifact" {
                if let (Some(target), Some(executable_path)) = (msg.target, msg.executable) {
                    // Check if it's a test artifact for the main crate.
                    // When running `cargo test`, the main library is compiled as a test.
                    // We check for `target.test == true` and that the name matches our crate.
                    // Crate names with hyphens are converted to underscores.
                    if target.name == crate_name && target.test {
                        return Some(executable_path);
                    }
                }
            }
            None
        })
        .expect("Could not find test executable in cargo output");

    println!(
        "{}",
        format!("Successfully built test: {}", case.name).green()
    );

    Ok(BuiltTest {
        case,
        executable,
        build_ctx,
    })
}

async fn run_built_test(
    built_test: BuiltTest,
    stop_token: Option<CancellationToken>,
) -> Result<TestResult, TestResult> {
    let case = built_test.case;
    let executable = built_test.executable;
    let build_ctx = built_test.build_ctx; // This now holds the ownership of the temp dir
    let project_root = PathBuf::from("."); // Not ideal, but should work.

    let start_time = std::time::Instant::now();
    println!("{}", format!("Queueing test: {}", case.name).blue());

    let mut cmd = tokio::process::Command::new(&executable);
    cmd.kill_on_drop(true);
    let command_string = format!("{}", executable.display());

    let (status_res, output, was_cancelled) = spawn_and_capture(cmd, stop_token).await;
    let status = status_res.expect("Error waiting for process to complete");

    let duration = start_time.elapsed();

    println!(
        "{}",
        format!("Finished test: {} in {:.2?}", case.name, duration).blue()
    );

    let success = status.success();
    let failure_reason = if success {
        None
    } else if was_cancelled {
        Some(FailureReason::Cancelled)
    } else {
        Some(FailureReason::Test)
    };

    let result = TestResult {
        case: case.clone(),
        output,
        success,
        failure_reason,
    };

    if !result.success {
        // Only preserve artifacts for genuine test failures, not cancellations.
        if result.failure_reason == Some(FailureReason::Test) {
            let sanitized_name = case
                .name
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                .collect::<String>();
            let error_dir_path = project_root.join("target-errors").join(sanitized_name);

            println!(
                "{}\n  Command: {}",
                format!(
                    "Test '{}' failed. Preserving build artifacts in: {}",
                    case.name,
                    error_dir_path.display()
                )
                .yellow(),
                command_string.cyan()
            );

            if error_dir_path.exists() {
                fs::remove_dir_all(&error_dir_path)
                    .expect("Failed to clean up old error artifacts directory");
            }

            // The build artifacts are already in the temp dir managed by build_ctx.
            // We just need to copy them.
            copy_dir_all(&build_ctx.target_path, &error_dir_path).unwrap_or_else(|e| {
                eprintln!("Failed to copy error artifacts for '{}': {}", case.name, e)
            });
        }

        Err(result)
    } else {
        // build_ctx is dropped here, cleaning up the temp dir
        Ok(result)
    }
}

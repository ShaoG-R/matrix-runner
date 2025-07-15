//! # Run Command Module / 运行命令模块
//!
//! This module implements the `run` command for the Matrix Runner CLI,
//! which executes test cases according to the test matrix configuration.
//!
//! 此模块实现了 Matrix Runner CLI 的 `run` 命令，
//! 根据测试矩阵配置执行测试用例。

use anyhow::{Context, Result};
use colored::*;
use futures::{stream, StreamExt};
use std::{env, fs, path::PathBuf, time::Duration};
use tokio::signal;
use tokio_util::sync::CancellationToken;

use crate::{
    infra::t,
    core::{
        config::{self, TestMatrix},
        execution::run_test_case,
        models::{self, FailureReason, Manifest},
        planner,
    },
    reporting::{
        console::{print_summary, print_unexpected_failure_details},
        html::generate_html_report,
    }
};

/// Executes the run command with the provided arguments.
///
/// # Arguments
/// * `jobs` - Number of parallel jobs to run
/// * `config` - Path to the test matrix configuration file
/// * `project_dir` - Path to the project directory
/// * `total_runners` - Total number of distributed runners (for CI)
/// * `runner_index` - Index of this runner (for CI)
/// * `html` - Optional path for HTML report output
///
/// # Returns
/// A Result indicating success or failure of the command execution
pub async fn execute(
    jobs: Option<usize>,
    config: PathBuf,
    project_dir: PathBuf,
    total_runners: Option<usize>,
    runner_index: Option<usize>,
    html: Option<PathBuf>,
) -> Result<()> {
    let (test_matrix, config_path) = setup_and_parse_config(&config)?;
    let locale = test_matrix.language.clone();
    rust_i18n::set_locale(& locale);

    let (project_root, crate_name) = prepare_environment(&project_dir, &locale).await?;

    println!(
        "{}",
        t!("project_root_detected", locale = locale, path = project_root.display())
    );
    println!(
        "{}",
        t!("testing_crate", locale = locale, name = crate_name.yellow())
    );
    println!(
        "{}",
        t!("loading_test_matrix", locale = locale, path = config_path.display())
    );

    let overall_stop_token = setup_signal_handler(&locale)?;

    let plan = planner::plan_execution(test_matrix, total_runners, runner_index)?;

    if plan.filtered_arch_count > 0 {
        println!(
            "{}",
            t!(
                "filtered_arch_cases",
                locale = locale,
                filtered = plan.filtered_arch_count,
                total = plan.cases_to_run.len()
            )
            .cyan()
        );
    }

    println!(
        "{}",
        t!("current_os", locale = locale, os = env::consts::OS).cyan()
    );

    if plan.flaky_cases_count > 0 {
        println!(
            "{}",
            t!("flaky_cases_found", locale = locale, count = plan.flaky_cases_count).yellow()
        );
    }

    if let (Some(total), Some(index)) = (total_runners, runner_index) {
        println!(
            "{}",
            t!(
                "running_as_split_runner",
                locale = locale,
                index = index + 1,
                total = total,
                count = plan.cases_to_run.len()
            )
            .bold()
        );
    } else {
        println!("{}", t!("running_as_single_runner", locale = locale).bold());
    }

    if plan.cases_to_run.is_empty() {
        println!("{}", t!("no_cases_to_run", locale = locale).green());
        return Ok(());
    }

    let (final_results, has_unexpected_failures) = run_tests(
        plan.cases_to_run,
        jobs.unwrap_or(num_cpus::get() / 2 + 1),
        &project_root,
        &crate_name,
        overall_stop_token,
        &locale,
    )
    .await?;

    print_summary(&final_results, &locale);

    if let Some(report_path) = &html {
        println!("\nGenerating HTML report at: {}", report_path.display());
        if let Err(e) = generate_html_report(&final_results, report_path) {
            eprintln!("{} {}", "Failed to generate HTML report:".red(), e);
        }
    }

    if has_unexpected_failures {
        let unexpected_failures: Vec<_> = final_results
            .iter()
            .filter(|r| r.is_unexpected_failure())
            .collect();
        print_unexpected_failure_details(&unexpected_failures, &locale);
        anyhow::bail!("Matrix tests failed with unexpected errors.");
    } else {
        println!("\n{}", t!("all_tests_passed", locale = locale).green().bold());
        Ok(())
    }
}

/// Sets up and parses the test matrix configuration file.
fn setup_and_parse_config(config_path_arg: &PathBuf) -> Result<(TestMatrix, PathBuf)> {
    // For config parsing, we don't have the locale yet. Use English as a default.
    let locale = "en";
    let config_path = fs::canonicalize(config_path_arg)
        .with_context(|| t!("config_read_failed_path", locale = locale, path = config_path_arg.display()))?;

    let config_matrix = config::load_test_matrix(&config_path)
        .with_context(|| t!("config_parse_failed", locale = locale))?;

    Ok((config_matrix, config_path))
}

/// Prepares the environment for running tests.
async fn prepare_environment(project_dir: &PathBuf, locale: &str) -> Result<(PathBuf, String)> {
    let project_root = fs::canonicalize(project_dir)
        .with_context(|| t!("project_dir_not_found", locale = locale, path = project_dir.display()))?;

    let fetch_status = tokio::process::Command::new("cargo")
        .arg("fetch")
        .current_dir(&project_root)
        .status()
        .await
        .context("Failed to execute 'cargo fetch'")?;

    if !fetch_status.success() {
        anyhow::bail!(t!("cargo_fetch_failed", locale = locale));
    }

    let manifest_path = project_root.join("Cargo.toml");
    let manifest_content = fs::read_to_string(&manifest_path)
        .with_context(|| t!("manifest_read_failed", locale = locale, path = manifest_path.display()))?;
    let manifest: Manifest =
        toml::from_str(&manifest_content).context(t!("manifest_parse_failed", locale = locale))?;
    let crate_name = manifest.package.name;

    Ok((project_root, crate_name))
}

/// Sets up a signal handler for graceful shutdown.
fn setup_signal_handler(locale: &str) -> Result<CancellationToken> {
    let token = CancellationToken::new();
    let token_clone = token.clone();
    let locale = locale.to_string();

    tokio::spawn(async move {
        signal::ctrl_c().await.expect("Failed to listen for Ctrl-C");
        println!("\n{}", t!("shutdown_signal", locale = &locale).yellow());
        token_clone.cancel();
    });

    Ok(token)
}

/// Runs the test cases in parallel.
async fn run_tests(
    cases_to_run: Vec<crate::core::config::TestCase>,
    jobs: usize,
    project_root: &PathBuf,
    crate_name: &str,
    overall_stop_token: CancellationToken,
    locale: &str,
) -> Result<(
    Vec<models::TestResult>,
    bool,
)> {
    let fast_fail_token = CancellationToken::new();
    let current_os = env::consts::OS;

    let stream = stream::iter(cases_to_run.into_iter().map(|case| {
        let fast_fail_token = fast_fail_token.clone();
        let overall_stop_token = overall_stop_token.clone();
        let project_root = project_root.clone();
        let crate_name = crate_name.to_string();
        let is_flaky = case.allow_failure.iter().any(|os| os == current_os);
        let case_clone_for_error = case.clone();

        tokio::spawn(async move {
            let mut handle = tokio::spawn(async move {
                run_test_case(case, &project_root, &crate_name).await
            });

            let mut final_result;
            let mut handle_finished = false;

            tokio::select! {
                biased;
                _ = overall_stop_token.cancelled() => {
                    handle.abort();
                    final_result = models::TestResult::Skipped;
                }
                _ = &mut handle => {
                    handle_finished = true;
                    // We will process the result outside the select block.
                    // This placeholder is just to satisfy the compiler.
                    final_result = models::TestResult::Skipped;
                }
            }

            if handle_finished {
                 if fast_fail_token.is_cancelled() && !is_flaky {
                    final_result = models::TestResult::Skipped;
                } else {
                    final_result = match handle.await {
                        Ok(Ok(res)) => res,
                        Ok(Err(e)) => models::TestResult::Failed {
                            case: case_clone_for_error.clone(),
                            output: e.to_string(),
                            reason: FailureReason::TestFailed,
                            duration: Duration::default(),
                        },
                        Err(e) => models::TestResult::Failed {
                            case: case_clone_for_error.clone(),
                            output: e.to_string(),
                            reason: FailureReason::Build,
                            duration: Duration::default(),
                        },
                    };
                }

                if !is_flaky && matches!(final_result, models::TestResult::Failed { .. }) {
                    // Cancel other tasks if this is an unexpected failure
                    fast_fail_token.cancel();
                }
            }

            final_result
        })
    }))
    .buffer_unordered(jobs)
    .collect::<Vec<Result<models::TestResult, tokio::task::JoinError>>>()
    .await;

    // Process results and check for unexpected failures
    let mut has_unexpected_failures = false;
    let final_results: Vec<models::TestResult> = stream
        .into_iter()
        .map(|res| match res {
            Ok(test_result) => {
                if matches!(test_result, models::TestResult::Failed { .. }) {
                    if test_result.is_unexpected_failure() {
                        has_unexpected_failures = true;
                    }
                }
                test_result
            }
            Err(e) => {
                has_unexpected_failures = true;
                models::TestResult::Failed {
                    case: crate::core::config::TestCase::default(),
                    output: format!("Critical error during test execution: {}", e),
                    reason: FailureReason::BuildFailed,
                    duration: Duration::default(),
                }
            }
        })
        .collect();

    Ok((final_results, has_unexpected_failures))
} 
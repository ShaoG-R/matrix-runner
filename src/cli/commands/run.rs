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
use tempfile::TempDir;
use tokio::{signal, sync::mpsc};
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
/// * `lang` - Optional language code for the test matrix (e.g., "en", "zh")
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
    lang: Option<String>,
    fast_fail_cli: bool,
) -> Result<()> {
    let (test_matrix, config_path) = setup_and_parse_config(&config)?;
    let fast_fail_mode = fast_fail_cli || test_matrix.fast_fail;

    // The locale has been pre-initialized in main.rs from the system or --lang argument.
    // We only override it if the config file specifies a non-default language
    // AND no --lang argument was provided.
    if lang.is_none() && test_matrix.language != "en" {
        rust_i18n::set_locale(&test_matrix.language);
    }
    
    // Get the final, correct locale for use in this command.
    let locale = rust_i18n::locale().to_string();

    let (project_root, crate_name) = prepare_environment(&project_dir, &locale).await?;

    println!(
        "{}",
        t!("common.project_root_detected", locale = &locale, path = project_root.display())
    );
    println!(
        "{}",
        t!("common.testing_crate", locale = &locale, name = crate_name.yellow())
    );
    println!(
        "{}",
        t!("common.loading_test_matrix", locale = &locale, path = config_path.display())
    );

    let overall_stop_token = setup_signal_handler(&locale)?;

    let plan = planner::plan_execution(test_matrix, total_runners, runner_index)?;

    if plan.filtered_arch_count > 0 {
        println!(
            "{}",
            t!(
                "run.filtered_arch_cases",
                locale = &locale,
                filtered = plan.filtered_arch_count,
                total = plan.cases_to_run.len() + plan.filtered_arch_count,
            )
            .cyan()
        );
    }

    println!(
        "{}",
        t!("common.current_os", locale = &locale, os = env::consts::OS).cyan()
    );

    if plan.flaky_cases_count > 0 {
        println!(
            "{}",
            t!("common.flaky_cases_found", locale = &locale, count = plan.flaky_cases_count).yellow()
        );
    }

    if let (Some(total), Some(index)) = (total_runners, runner_index) {
        println!(
            "{}",
            t!(
                "run.running_as_split_runner",
                locale = &locale,
                index = index,
                total = total,
                count = plan.cases_to_run.len()
            )
            .bold()
        );
    } else {
        println!("{}", t!("run.running_as_single_runner", locale = &locale).bold());
    }

    if plan.cases_to_run.is_empty() {
        println!("{}", t!("common.no_cases_to_run", locale = &locale).green());
        return Ok(());
    }

    let (temp_dir_tx, mut temp_dir_rx) = mpsc::unbounded_channel::<TempDir>();
    let collector_handle = tokio::spawn(async move {
        let mut dirs = Vec::new();
        while let Some(dir) = temp_dir_rx.recv().await {
            dirs.push(dir);
        }
        dirs
    });

    let (final_results, has_unexpected_failures) = run_tests(
        plan.cases_to_run,
        jobs.unwrap_or(num_cpus::get() / 2 + 1),
        &project_root,
        &crate_name,
        overall_stop_token,
        temp_dir_tx.clone(),
        fast_fail_mode,
    )
    .await?;

    drop(temp_dir_tx);
    let _temp_dirs = collector_handle
        .await
        .context("Failed to collect temporary directories")?;

    print_summary(&final_results, &locale);

    if let Some(report_path) = &html {
        println!(
            "\n{}",
            t!(
                "run.html_report_generating",
                locale = &locale,
                path = report_path.display()
            )
        );
        if let Err(e) = generate_html_report(&final_results, report_path, &locale) {
            eprintln!(
                "{} {}",
                t!("run.html_report_failed", locale = &locale).red(),
                e
            );
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
        println!("\n{}", t!("common.all_tests_passed", locale = &locale).green().bold());
        Ok(())
    }
}

/// Sets up and parses the test matrix configuration file.
fn setup_and_parse_config(config_path_arg: &PathBuf) -> Result<(TestMatrix, PathBuf)> {
    // For config parsing, we must use the locale that has already been set in main.rs.
    let locale = rust_i18n::locale();
    let config_path = match fs::canonicalize(config_path_arg) {
        Ok(path) => path,
        Err(e) => {
            return Err(anyhow::Error::new(e).context(t!(
                "common.config_read_failed_path",
                locale = &locale,
                path = config_path_arg.display().to_string()
            )));
        }
    };

    let config_matrix = config::load_test_matrix(&config_path)
        .with_context(|| t!("common.config_parse_failed", locale = &locale))?;

    Ok((config_matrix, config_path))
}

/// Prepares the environment for running tests.
async fn prepare_environment(project_dir: &PathBuf, locale: &str) -> Result<(PathBuf, String)> {
    let project_root = match fs::canonicalize(project_dir) {
        Ok(path) => path,
        Err(e) => {
            return Err(anyhow::Error::new(e).context(t!(
                "common.project_dir_not_found",
                locale = locale,
                path = project_dir.display().to_string()
            )));
        }
    };

    let fetch_status = tokio::process::Command::new("cargo")
        .arg("fetch")
        .current_dir(&project_root)
        .status()
        .await
        .context("Failed to execute 'cargo fetch'")?;

    if !fetch_status.success() {
        anyhow::bail!(t!("common.cargo_fetch_failed", locale = locale));
    }

    let manifest_path = project_root.join("Cargo.toml");
    let manifest_content = match fs::read_to_string(&manifest_path) {
        Ok(content) => content,
        Err(e) => {
            return Err(anyhow::Error::new(e).context(t!(
                "common.manifest_read_failed",
                locale = locale,
                path = manifest_path.display().to_string()
            )));
        }
    };
    let manifest: Manifest =
        toml::from_str(&manifest_content).context(t!("common.manifest_parse_failed", locale = locale))?;
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
        println!("\n{}", t!("common.shutdown_signal", locale = &locale).yellow());
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
    temp_dir_tx: mpsc::UnboundedSender<TempDir>,
    fast_fail: bool,
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
        let temp_dir_tx = temp_dir_tx.clone();

        async move {
            let case_clone_for_error = case.clone();
            let mut handle = tokio::spawn(async move {
                run_test_case(case, &project_root, &crate_name, temp_dir_tx).await
            });

            let result = tokio::select! {
                biased;

                _ = overall_stop_token.cancelled() => {
                    handle.abort();
                    Ok(models::TestResult::Skipped)
                }

                _ = fast_fail_token.cancelled() => {
                    handle.abort();
                    Ok(models::TestResult::Skipped)
                }

                result = &mut handle => {
                    result.map(|inner_result| {
                        match inner_result {
                            Ok(res) => res,
                            Err(e) => models::TestResult::Failed {
                                case: case_clone_for_error.clone(),
                                output: e.to_string(),
                                reason: FailureReason::TestFailed,
                                duration: Duration::default(),
                            },
                        }
                    })
                }
            };
            
            let final_result = match result {
                Ok(res) => res,
                Err(e) => models::TestResult::Failed {
                    case: case_clone_for_error.clone(),
                    output: e.to_string(),
                    reason: FailureReason::TestFailed,
                    duration: Duration::default(),
                }
            };

            if fast_fail && !is_flaky && final_result.is_unexpected_failure() {
                fast_fail_token.cancel();
            }

            (case_clone_for_error, final_result)
        }
    }))
    .buffer_unordered(jobs)
    .collect::<Vec<(
        crate::core::config::TestCase,
        models::TestResult,
    )>>()
    .await;

    // Process results and check for unexpected failures
    let mut has_unexpected_failures = false;
    let final_results: Vec<models::TestResult> = stream
        .into_iter()
        .map(|(_case, test_result)| {
            if test_result.is_unexpected_failure() {
                has_unexpected_failures = true;
            }
            test_result
        })
        .collect();

    Ok((final_results, has_unexpected_failures))
} 
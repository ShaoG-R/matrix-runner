//! # Matrix Runner - Configuration-Driven Test Executor
//! # Matrix Runner - 配置驱动的测试执行器
//!
//! A powerful, configuration-driven test executor for Rust projects that enables
//! testing across a wide matrix of feature flags and environments. This tool helps
//! ensure comprehensive test coverage by automatically running tests with different
//! feature combinations in isolated environments.
//!
//! 一个强大的、配置驱动的 Rust 项目测试执行器，支持在广泛的 feature 标志和环境矩阵中进行测试。
//! 此工具通过在隔离环境中自动运行不同 feature 组合的测试来帮助确保全面的测试覆盖。
//!
//! ## Key Features / 主要功能
//!
//! - **Matrix Testing**: Run tests across multiple feature flag combinations
//! - **Parallel Execution**: Concurrent test execution with configurable job limits
//! - **Isolated Builds**: Each test runs in its own temporary directory
//! - **HTML Reports**: Generate detailed HTML reports with test results
//! - **Internationalization**: Support for multiple languages (English, Chinese)
//! - **Graceful Shutdown**: Handle interruption signals properly
//!
//! - **矩阵测试**: 在多个 feature 标志组合中运行测试
//! - **并行执行**: 具有可配置作业限制的并发测试执行
//! - **隔离构建**: 每个测试在自己的临时目录中运行
//! - **HTML 报告**: 生成包含测试结果的详细 HTML 报告
//! - **国际化**: 支持多种语言（英语、中文）
//! - **优雅关闭**: 正确处理中断信号

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use futures::{StreamExt, stream};
use runner::i18n::I18nKey;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use tokio::signal;
use tokio_util::sync::CancellationToken;

mod runner;
use runner::config::TestMatrix;
use runner::execution::run_test_case;
use runner::i18n;
use runner::init;
use runner::reporting::{print_summary, print_unexpected_failure_details};

/// Command-line interface definition for the Matrix Runner application.
/// Defines the main CLI structure and available subcommands.
///
/// Matrix Runner 应用程序的命令行界面定义。
/// 定义主要的 CLI 结构和可用的子命令。
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The subcommand to execute / 要执行的子命令
    #[command(subcommand)]
    command: Commands,
}

/// Available subcommands for the Matrix Runner CLI.
/// Defines the main operations that can be performed with the tool.
///
/// Matrix Runner CLI 的可用子命令。
/// 定义可以使用该工具执行的主要操作。
#[derive(Subcommand, Debug)]
enum Commands {
    /// Run tests based on the matrix configuration (default command).
    /// 基于矩阵配置运行测试（默认命令）。
    Run(RunArgs),
    /// Launch an interactive wizard to create a new TestMatrix.toml file.
    /// 启动交互式向导创建新的 TestMatrix.toml 文件。
    Init(InitArgs),
}

/// Defines the command-line arguments for the `run` command.
#[derive(Parser, Debug, Default)]
struct RunArgs {
    /// Number of parallel jobs, defaults to (logical CPUs / 2) + 1.
    #[arg(short, long)]
    jobs: Option<usize>,

    /// Path to the test matrix config file, relative to project dir.
    #[arg(short, long, default_value = "TestMatrix.toml")]
    config: PathBuf,

    /// Path to the project directory to test.
    #[arg(long, default_value = ".")]
    project_dir: PathBuf,

    /// Total number of parallel runners for splitting the test matrix.
    #[arg(long)]
    total_runners: Option<usize>,

    /// Index of the current runner (0-based) when splitting the test matrix.
    #[arg(long)]
    runner_index: Option<usize>,

    /// Path to write an HTML report to. If provided, a report will be generated.
    #[arg(long)]
    html: Option<PathBuf>,
}

/// Defines the command-line arguments for the `init` command.
#[derive(Parser, Debug)]
struct InitArgs {
    /// Language for the wizard interface and generated config file.
    /// If not specified, the system language will be auto-detected.
    /// 向导界面和生成的配置文件的语言。
    /// 如果未指定，将自动检测系统语言。
    #[arg(short, long)]
    language: Option<String>,
}

/// Represents the `[package]` section of a Cargo.toml file.
/// Used to extract the crate name.
/// 代表 Cargo.toml 文件中的 `[package]` 部分。
/// 用于提取 crate 的名称。
#[derive(Deserialize)]
struct Package {
    name: String,
}

/// Represents the top-level structure of a Cargo.toml manifest.
/// 代表 Cargo.toml 清单的顶层结构。
#[derive(Deserialize)]
struct Manifest {
    package: Package,
}

/// Application entry point with async runtime.
/// Initializes the Tokio runtime and handles top-level error reporting.
///
/// 具有异步运行时的应用程序入口点。
/// 初始化 Tokio 运行时并处理顶级错误报告。
#[tokio::main]
async fn main() {
    // Run the main application logic and handle any errors
    // 运行主应用程序逻辑并处理任何错误
    if let Err(e) = run_main().await {
        eprintln!("{} {}", "Error:".red(), e);
        std::process::exit(1);
    }
}

async fn run_main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run(args) => {
            run_matrix_tests(args).await?;
        }
        Commands::Init(args) => {
            // Determine the language to use: user-specified or auto-detected
            // 确定要使用的语言：用户指定的或自动检测的
            let language = args
                .language
                .clone()
                .unwrap_or_else(|| i18n::detect_system_language());

            // Initialize i18n for the init wizard
            // 为初始化向导初始化 i18n
            i18n::init(&language);

            // Show language detection message if it was auto-detected
            // 如果是自动检测的语言，显示检测消息
            if args.language.is_none() {
                println!(
                    "🌐 {}",
                    i18n::t_fmt(I18nKey::SystemLanguageDetected, &[&language])
                );
            }

            init::run_init_wizard(&language)?;
        }
    }
    Ok(())
}

async fn run_matrix_tests(args: RunArgs) -> Result<()> {
    let (test_matrix, config_path) = setup_and_parse_config(&args)?;
    i18n::init(&test_matrix.language);

    let (project_root, crate_name) = prepare_environment(&args).await?;

    println!(
        "{}",
        i18n::t_fmt(I18nKey::ProjectRootDetected, &[&project_root.display()])
    );
    println!(
        "{}",
        i18n::t_fmt(I18nKey::TestingCrate, &[&crate_name.yellow()])
    );
    println!(
        "{}",
        i18n::t_fmt(I18nKey::LoadingTestMatrix, &[&config_path.display()])
    );

    let overall_stop_token = setup_signal_handler()?;

    let cases_to_run = filter_and_distribute_cases(test_matrix, &args)?;
    if cases_to_run.is_empty() {
        println!("{}", i18n::t(I18nKey::NoCasesToRun).green());
        std::process::exit(0);
    }

    let (final_results, has_unexpected_failures) = run_tests(
        cases_to_run,
        args.jobs.unwrap_or(num_cpus::get() / 2 + 1),
        &project_root,
        &crate_name,
        overall_stop_token,
    )
    .await?;

    print_summary(&final_results);

    if let Some(report_path) = &args.html {
        println!("\nGenerating HTML report at: {}", report_path.display());
        if let Err(e) = runner::reporting::generate_html_report(&final_results, report_path) {
            eprintln!("{} {}", "Failed to generate HTML report:".red(), e);
        }
    }

    if has_unexpected_failures {
        let unexpected_failures: Vec<_> = final_results
            .iter()
            .filter(|r| r.is_unexpected_failure())
            .collect();
        print_unexpected_failure_details(&unexpected_failures);
        std::process::exit(1);
    } else {
        println!("\n{}", i18n::t(I18nKey::AllTestsPassed).green().bold());
        std::process::exit(0);
    }
}

/// Sets up command-line arguments and loads the test matrix configuration.
/// 设置命令行参数并加载测试矩阵配置。
fn setup_and_parse_config(args: &RunArgs) -> Result<(TestMatrix, PathBuf)> {
    let config_path = fs::canonicalize(&args.config)
        .with_context(|| i18n::t_fmt(I18nKey::ConfigReadFailedPath, &[&args.config.display()]))?;

    let config_content = fs::read_to_string(&config_path)
        .with_context(|| i18n::t_fmt(I18nKey::ConfigReadFailedPath, &[&config_path.display()]))?;

    let test_matrix: TestMatrix =
        toml::from_str(&config_content).with_context(|| i18n::t(I18nKey::ConfigParseFailed))?;

    Ok((test_matrix, config_path))
}

/// Prepares the testing environment by fetching dependencies and identifying the crate name.
/// 通过预取依赖和识别 crate 名称来准备测试环境。
async fn prepare_environment(args: &RunArgs) -> Result<(PathBuf, String)> {
    let project_root = fs::canonicalize(&args.project_dir).with_context(|| {
        i18n::t_fmt(I18nKey::ProjectDirNotFound, &[&args.project_dir.display()])
    })?;

    println!("\n{}", i18n::t(I18nKey::DepFetchStart).cyan());
    let mut fetch_cmd = std::process::Command::new("cargo");
    fetch_cmd.current_dir(&project_root).arg("fetch");

    let fetch_status = fetch_cmd
        .status()
        .context("Failed to execute cargo fetch command")?;
    if !fetch_status.success() {
        return Err(anyhow::anyhow!("{}", i18n::t(I18nKey::CargoFetchFailed)));
    }
    println!("{}", i18n::t(I18nKey::DepFetchSuccess).green());

    let manifest_path = project_root.join("Cargo.toml");
    let manifest_content = fs::read_to_string(&manifest_path)
        .with_context(|| i18n::t_fmt(I18nKey::ManifestReadFailed, &[&manifest_path.display()]))?;

    let manifest: Manifest =
        toml::from_str(&manifest_content).with_context(|| i18n::t(I18nKey::ManifestParseFailed))?;
    let crate_name = manifest.package.name.replace('-', "_");

    Ok((project_root, crate_name))
}

/// Sets up a Ctrl+C signal handler to gracefully shut down the application.
/// 设置 Ctrl+C 信号处理器以优雅地关闭应用程序。
fn setup_signal_handler() -> Result<CancellationToken> {
    let token = CancellationToken::new();
    let signal_token = token.clone();
    tokio::spawn(async move {
        if let Err(e) = signal::ctrl_c().await {
            eprintln!("Failed to listen for Ctrl+C signal: {}", e);
            return;
        }
        println!("\n{}", i18n::t(I18nKey::ShutdownSignal).yellow());
        signal_token.cancel();
    });
    Ok(token)
}

/// Filters test cases based on architecture and distributes them for parallel runners.
/// 根据架构筛选测试用例并为并行运行器分发它们。
fn filter_and_distribute_cases(
    test_matrix: TestMatrix,
    args: &RunArgs,
) -> Result<Vec<runner::config::TestCase>> {
    let total_cases_count = test_matrix.cases.len();
    let current_arch = std::env::consts::ARCH;
    println!(
        "{}",
        i18n::t_fmt(I18nKey::CurrentArch, &[&current_arch.yellow()])
    );

    let all_cases: Vec<_> = test_matrix
        .cases
        .into_iter()
        .filter(|case| case.arch.is_empty() || case.arch.iter().any(|a| a == current_arch))
        .collect();

    let filtered_count = total_cases_count - all_cases.len();
    if filtered_count > 0 {
        println!(
            "{}",
            i18n::t_fmt(
                I18nKey::FilteredArchCases,
                &[&filtered_count, &all_cases.len()]
            )
            .yellow()
        );
    }

    match (args.total_runners, args.runner_index) {
        (Some(total), Some(index)) => {
            if index >= total {
                return Err(anyhow::anyhow!("{}", i18n::t(I18nKey::RunnerIndexInvalid)));
            }
            let cases_for_this_runner = all_cases
                .into_iter()
                .enumerate()
                .filter_map(|(i, case)| if i % total == index { Some(case) } else { None })
                .collect::<Vec<_>>();
            println!(
                "{}",
                i18n::t_fmt(
                    I18nKey::RunningAsSplitRunner,
                    &[&(index + 1), &total, &cases_for_this_runner.len()]
                )
                .yellow()
            );
            Ok(cases_for_this_runner)
        }
        (None, None) => {
            println!("{}", i18n::t(I18nKey::RunningAsSingleRunner).yellow());
            Ok(all_cases)
        }
        _ => Err(anyhow::anyhow!(
            "{}",
            i18n::t(I18nKey::RunnerFlagsInconsistent)
        )),
    }
}

/// Runs the selected test cases and reports the summary.
/// 运行选定的测试用例并报告摘要。
async fn run_tests(
    cases_to_run: Vec<runner::config::TestCase>,
    jobs: usize,
    project_root: &PathBuf,
    crate_name: &str,
    overall_stop_token: CancellationToken,
) -> Result<(
    Vec<runner::models::TestResult>, // final_results
    bool,                            // has_unexpected_failures
)> {
    println!("{}", i18n::t(I18nKey::TempDirCleanupInfo).green());
    println!("{}", i18n::t(I18nKey::FailureArtifactInfo).yellow());

    let current_os = std::env::consts::OS;
    println!(
        "{}",
        i18n::t_fmt(I18nKey::CurrentOs, &[&current_os.yellow()])
    );

    let (flaky_cases, safe_cases): (Vec<_>, Vec<_>) = cases_to_run
        .into_iter()
        .partition(|case| case.allow_failure.iter().any(|s| s == current_os));

    let safe_cases_count = safe_cases.len();
    let flaky_cases_count = flaky_cases.len();
    if flaky_cases_count > 0 {
        println!(
            "{}",
            i18n::t_fmt(I18nKey::FlakyCasesFound, &[&flaky_cases_count]).yellow()
        );
    }

    let mut results = Vec::new();
    let fast_fail_token = CancellationToken::new();

    println!(
        "\n{}",
        i18n::t_fmt(I18nKey::RunningSafeCases, &[&safe_cases_count, &jobs]).green()
    );
    let safe_stream = stream::iter(safe_cases.into_iter().map(|case| {
        let case_stop_token = fast_fail_token.clone();
        let global_stop_token = overall_stop_token.clone();
        let root = project_root.clone();
        let name = crate_name.to_string();
        tokio::spawn(async move {
            tokio::select! {
                res = run_test_case(case, &root, &name) => {
                    res.unwrap_or_else(|e| {
                            eprintln!("Task failed: {e:?}");
                            runner::models::TestResult::Skipped
                        })
                },
                _ = case_stop_token.cancelled() => runner::models::TestResult::Skipped,
                _ = global_stop_token.cancelled() => runner::models::TestResult::Skipped,
            }
        })
    }));

    let mut safe_processed_stream = safe_stream.buffer_unordered(jobs);
    while let Some(result) = safe_processed_stream.next().await {
        let test_result = result.unwrap_or_else(|e| {
            eprintln!("Task join error: {e:?}");
            runner::models::TestResult::Skipped
        });
        if test_result.is_unexpected_failure() && !fast_fail_token.is_cancelled() {
            println!("\n{}", i18n::t(I18nKey::FastFailTriggered).red().bold());
            fast_fail_token.cancel();
        }
        results.push(test_result);
    }

    if !overall_stop_token.is_cancelled() && flaky_cases_count > 0 {
        println!(
            "\n{}",
            i18n::t_fmt(I18nKey::RunningFlakyCases, &[&flaky_cases_count, &jobs]).green()
        );
        let flaky_stream = stream::iter(flaky_cases.into_iter().map(|case| {
            let global_stop_token = overall_stop_token.clone();
            let root = project_root.clone();
            let name = crate_name.to_string();
            tokio::spawn(async move {
                tokio::select! {
                    res = run_test_case(case, &root, &name) => {
                        res.unwrap_or_else(|e| {
                                eprintln!("Task failed: {e:?}");
                                runner::models::TestResult::Skipped
                            })
                    },
                    _ = global_stop_token.cancelled() => runner::models::TestResult::Skipped,
                }
            })
        }));

        let mut flaky_processed_stream = flaky_stream.buffer_unordered(jobs);
        while let Some(result) = flaky_processed_stream.next().await {
            let test_result = result.unwrap_or_else(|e| {
                eprintln!("Task join error: {e:?}");
                runner::models::TestResult::Skipped
            });
            results.push(test_result);
        }
    }

    let has_unexpected_failures = results.iter().any(|r| r.is_unexpected_failure());
    Ok((results, has_unexpected_failures))
}

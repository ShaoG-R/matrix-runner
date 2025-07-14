use clap::Parser;
use colored::*;
use futures::{StreamExt, stream};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use runner::i18n::I18nKey;

mod runner;
use runner::config::TestMatrix;
use runner::execution::run_test_case;
use runner::i18n;
use runner::reporting::{print_summary, print_unexpected_failure_details};

/// Defines the command-line arguments for the application.
/// 定义了应用程序的命令行参数。
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of parallel jobs, defaults to (logical CPUs / 2) + 1.
    /// 并行作业的数量，默认为 (逻辑 CPU / 2) + 1。
    #[arg(short, long)]
    jobs: Option<usize>,

    /// Path to the test matrix config file, relative to project dir.
    /// 测试矩阵配置文件的路径，相对于项目目录。
    #[arg(short, long, default_value = "TestMatrix.toml")]
    config: PathBuf,

    /// Path to the project directory to test.
    /// 要测试的项目目录的路径。
    #[arg(long, default_value = ".")]
    project_dir: PathBuf,

    /// Total number of parallel runners for splitting the test matrix.
    /// 用于拆分测试矩阵的并行运行器总数。
    #[arg(long)]
    total_runners: Option<usize>,

    /// Index of the current runner (0-based) when splitting the test matrix.
    /// 拆分测试矩阵时当前运行器的索引（从 0 开始）。
    #[arg(long)]
    runner_index: Option<usize>,
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

#[tokio::main]
async fn main() {
    let (args, test_matrix, config_path) = setup_and_parse_args();
    i18n::init(&test_matrix.language);

    let (project_root, crate_name) = prepare_environment(&args).await;

    println!("{}", i18n::t_fmt(I18nKey::ProjectRootDetected, &[&project_root.display()]));
    println!("{}", i18n::t_fmt(I18nKey::TestingCrate, &[&crate_name.yellow()]));
    println!("{}", i18n::t_fmt(I18nKey::LoadingTestMatrix, &[&config_path.display()]));

    let overall_stop_token = setup_signal_handler();

    let cases_to_run = filter_and_distribute_cases(test_matrix, &args);
    if cases_to_run.is_empty() {
        println!("{}", i18n::t(I18nKey::NoCasesToRun).green());
        std::process::exit(0);
    }

    run_tests(cases_to_run, args.jobs.unwrap_or(num_cpus::get() / 2 + 1), &project_root, &crate_name, overall_stop_token).await;
}

/// Sets up command-line arguments and loads the test matrix configuration.
/// 设置命令行参数并加载测试矩阵配置。
fn setup_and_parse_args() -> (Args, TestMatrix, PathBuf) {
    let args = Args::parse();
    let config_path = fs::canonicalize(&args.config).unwrap_or_else(|e| {
        panic!(
            "{}: {}",
            i18n::t_fmt(I18nKey::ConfigReadFailedPath, &[&args.config.display()]),
            e
        )
    });
    let config_content = fs::read_to_string(&config_path)
        .unwrap_or_else(|_| panic!("{}", i18n::t_fmt(I18nKey::ConfigReadFailedPath, &[&config_path.display()])));
    let test_matrix: TestMatrix =
        toml::from_str(&config_content).expect(i18n::t(I18nKey::ConfigParseFailed).as_str());
    (args, test_matrix, config_path)
}

/// Prepares the testing environment by fetching dependencies and identifying the crate name.
/// 通过预取依赖和识别 crate 名称来准备测试环境。
async fn prepare_environment(args: &Args) -> (PathBuf, String) {
    let project_root = fs::canonicalize(&args.project_dir).unwrap_or_else(|_| {
        panic!("{}", i18n::t_fmt(I18nKey::ProjectDirNotFound, &[&args.project_dir.display()]))
    });

    println!("\n{}", i18n::t(I18nKey::DepFetchStart).cyan());
    let mut fetch_cmd = std::process::Command::new("cargo");
    fetch_cmd.current_dir(&project_root).arg("fetch");

    let fetch_status = fetch_cmd.status().expect("Failed to execute cargo fetch command");
    if !fetch_status.success() {
        panic!("{}", i18n::t(I18nKey::CargoFetchFailed));
    }
    println!("{}", i18n::t(I18nKey::DepFetchSuccess).green());

    let manifest_path = project_root.join("Cargo.toml");
    let manifest_content = fs::read_to_string(&manifest_path).unwrap_or_else(|_| {
        panic!("{}", i18n::t_fmt(I18nKey::ManifestReadFailed, &[&manifest_path.display()]))
    });
    let manifest: Manifest = toml::from_str(&manifest_content).expect(i18n::t(I18nKey::ManifestParseFailed).as_str());
    let crate_name = manifest.package.name.replace('-', "_");

    (project_root, crate_name)
}

/// Sets up a Ctrl+C signal handler to gracefully shut down the application.
/// 设置 Ctrl+C 信号处理器以优雅地关闭应用程序。
fn setup_signal_handler() -> CancellationToken {
    let token = CancellationToken::new();
    let signal_token = token.clone();
    tokio::spawn(async move {
        signal::ctrl_c().await.expect("Failed to listen for Ctrl+C signal");
        println!("\n{}", i18n::t(I18nKey::ShutdownSignal).yellow());
        signal_token.cancel();
    });
    token
}

/// Filters test cases based on architecture and distributes them for parallel runners.
/// 根据架构筛选测试用例并为并行运行器分发它们。
fn filter_and_distribute_cases(test_matrix: TestMatrix, args: &Args) -> Vec<runner::config::TestCase> {
    let total_cases_count = test_matrix.cases.len();
    let current_arch = std::env::consts::ARCH;
    println!("{}", i18n::t_fmt(I18nKey::CurrentArch, &[&current_arch.yellow()]));

    let all_cases: Vec<_> = test_matrix
        .cases
        .into_iter()
        .filter(|case| case.arch.is_empty() || case.arch.iter().any(|a| a == current_arch))
        .collect();

    let filtered_count = total_cases_count - all_cases.len();
    if filtered_count > 0 {
        println!("{}", i18n::t_fmt(I18nKey::FilteredArchCases, &[&filtered_count, &all_cases.len()]).yellow());
    }

    match (args.total_runners, args.runner_index) {
        (Some(total), Some(index)) => {
            if index >= total {
                panic!("{}", i18n::t(I18nKey::RunnerIndexInvalid));
            }
            let cases_for_this_runner = all_cases
                .into_iter()
                .enumerate()
                .filter_map(|(i, case)| if i % total == index { Some(case) } else { None })
                .collect::<Vec<_>>();
            println!("{}", i18n::t_fmt(I18nKey::RunningAsSplitRunner, &[&(index + 1), &total, &cases_for_this_runner.len()]).yellow());
            cases_for_this_runner
        }
        (None, None) => {
            println!("{}", i18n::t(I18nKey::RunningAsSingleRunner).yellow());
            all_cases
        }
        _ => {
            panic!("{}", i18n::t(I18nKey::RunnerFlagsInconsistent));
        }
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
) {
    println!("{}", i18n::t(I18nKey::TempDirCleanupInfo).green());
    println!("{}", i18n::t(I18nKey::FailureArtifactInfo).yellow());

    let current_os = std::env::consts::OS;
    println!("{}", i18n::t_fmt(I18nKey::CurrentOs, &[&current_os.yellow()]));

    let (flaky_cases, safe_cases): (Vec<_>, Vec<_>) = cases_to_run
        .into_iter()
        .partition(|case| case.allow_failure.iter().any(|s| s == current_os));

    let safe_cases_count = safe_cases.len();
    let flaky_cases_count = flaky_cases.len();
    if flaky_cases_count > 0 {
        println!("{}", i18n::t_fmt(I18nKey::FlakyCasesFound, &[&flaky_cases_count]).yellow());
    }

    let mut results = Vec::new();
    let fast_fail_token = CancellationToken::new();

    println!("\n{}", i18n::t_fmt(I18nKey::RunningSafeCases, &[&safe_cases_count, &jobs]).green());
    let safe_stream = stream::iter(safe_cases.into_iter().map(|case| {
        let case_stop_token = fast_fail_token.clone();
        let global_stop_token = overall_stop_token.clone();
        let root = project_root.clone();
        let name = crate_name.to_string();
        tokio::spawn(async move {
            tokio::select! {
                res = run_test_case(case, &root, &name) => res.unwrap_or_else(|e| panic!("Task failed: {:?}", e)),
                _ = case_stop_token.cancelled() => runner::models::TestResult::Skipped,
                _ = global_stop_token.cancelled() => runner::models::TestResult::Skipped,
            }
        })
    }));

    let mut safe_processed_stream = safe_stream.buffer_unordered(jobs);
    while let Some(result) = safe_processed_stream.next().await {
        let test_result = result.unwrap();
        if let runner::models::TestResult::Failed { .. } = &test_result {
            if !fast_fail_token.is_cancelled() {
                println!("\n{}", i18n::t(I18nKey::FastFailTriggered).red().bold());
                fast_fail_token.cancel();
            }
        }
        results.push(test_result);
    }

    if !overall_stop_token.is_cancelled() && flaky_cases_count > 0 {
        println!("\n{}", i18n::t_fmt(I18nKey::RunningFlakyCases, &[&flaky_cases_count, &jobs]).green());
        let flaky_stream = stream::iter(flaky_cases.into_iter().map(|case| {
            let global_stop_token = overall_stop_token.clone();
            let root = project_root.clone();
            let name = crate_name.to_string();
            tokio::spawn(async move {
                tokio::select! {
                    res = run_test_case(case, &root, &name) => res.unwrap_or_else(|e| panic!("Task failed: {:?}", e)),
                    _ = global_stop_token.cancelled() => runner::models::TestResult::Skipped,
                }
            })
        }));

        let mut flaky_processed_stream = flaky_stream.buffer_unordered(jobs);
        while let Some(result) = flaky_processed_stream.next().await {
            results.push(result.unwrap());
        }
    }

    let unexpected_failures = print_summary(&results);
    if !unexpected_failures.is_empty() {
        print_unexpected_failure_details(&unexpected_failures);
        std::process::exit(1);
    } else {
        println!("\n{}", i18n::t(I18nKey::AllTestsPassed).green().bold());
    }
}

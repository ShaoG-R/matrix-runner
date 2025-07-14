use clap::Parser;
use colored::*;
use futures::{StreamExt, stream};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use tokio::signal;
use tokio_util::sync::CancellationToken;

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
    // Parse command-line arguments.
    // 解析命令行参数。
    let args = Args::parse();

    // Determine the number of parallel jobs.
    // Defaults to half the number of logical CPUs plus one if not specified.
    // 确定并行作业的数量。
    // 如果未指定，则默认为逻辑 CPU 数量的一半加一。
    let num_cpus = num_cpus::get();
    let jobs = args.jobs.unwrap_or(num_cpus / 2 + 1);

    // Determine the project root from the command-line argument and canonicalize it.
    // 从命令行参数确定项目根目录并将其规范化。
    let project_root = fs::canonicalize(&args.project_dir).unwrap_or_else(|_| {
        panic!("{}", i18n::t_fmt("project_dir_not_found", &[&args.project_dir.display()]))
    });

    // --- Pre-fetch all dependencies ---
    // --- 预取所有依赖 ---
    println!(
        "\n{}",
        i18n::t("dep_fetch_start").cyan()
    );
    let mut fetch_cmd = std::process::Command::new("cargo");
    fetch_cmd.current_dir(&project_root);
    fetch_cmd.arg("fetch");

    let fetch_status = fetch_cmd
        .status()
        .expect("Failed to execute cargo fetch command");

    if !fetch_status.success() {
        panic!("{}", i18n::t("cargo_fetch_failed"));
    }
    println!("{}", i18n::t("dep_fetch_success").green());

    // --- Read crate name from Cargo.toml ---
    // --- 从 Cargo.toml 读取 crate 名称 ---
    let manifest_path = project_root.join("Cargo.toml");
    let manifest_content = fs::read_to_string(&manifest_path).unwrap_or_else(|_| {
        panic!("{}", i18n::t_fmt("manifest_read_failed", &[&manifest_path.display()]))
    });
    let manifest: Manifest =
        toml::from_str(&manifest_content).expect(i18n::t("manifest_parse_failed").as_str());
    // Cargo converts hyphens in crate names to underscores for symbol names.
    // Cargo 会将 crate 名称中的连字符转换成下划线作为符号名称。
    let crate_name = manifest.package.name.replace('-', "_");

    // The config file path is relative to the project root.
    // 配置文件路径是相对于项目根目录的。
    let config_path = project_root.join(&args.config);

    let config_content = fs::read_to_string(&config_path)
        .unwrap_or_else(|_| panic!("{}", i18n::t_fmt("config_read_failed", &[&config_path.display()])));

    let test_matrix: TestMatrix =
        toml::from_str(&config_content).expect(i18n::t("config_parse_failed").as_str());

    // Initialize the i18n system based on the language specified in the test matrix config.
    // 根据测试矩阵配置中指定的语言初始化 i18n 系统。
    i18n::init(&test_matrix.language);

    println!("{}", i18n::t_fmt("project_root_detected", &[&project_root.display()]));
    println!("{}", i18n::t_fmt("testing_crate", &[&crate_name.yellow()]));
    println!("{}", i18n::t_fmt("loading_test_matrix", &[&config_path.display()]));

    // Set up a global cancellation token for graceful shutdown on Ctrl+C.
    // 设置一个全局取消令牌，以便在 Ctrl+C 时优雅地关闭。
    let overall_stop_token = CancellationToken::new();
    let signal_token = overall_stop_token.clone();
    tokio::spawn(async move {
        signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C signal");
        println!(
            "\n{}",
            i18n::t("shutdown_signal").yellow()
        );
        // When Ctrl+C is pressed, cancel the token.
        // 当按下 Ctrl+C 时，取消令牌。
        signal_token.cancel();
    });

    println!(
        "{}",
        i18n::t("temp_dir_cleanup_info").green()
    );
    println!(
        "{}",
        i18n::t("failure_artifact_info").yellow()
    );

    // --- Filter and Prepare Test Cases ---
    // --- 筛选和准备测试用例 ---
    let total_cases_count = test_matrix.cases.len();
    let current_arch = std::env::consts::ARCH;
    println!("{}", i18n::t_fmt("current_arch", &[&current_arch.yellow()]));

    // Filter cases based on the current machine's architecture.
    // A case runs if its `arch` field is empty or contains the current architecture.
    // 根据当前机器的架构筛选测试用例。
    // 如果一个用例的 `arch` 字段为空或包含当前架构，则该用例会运行。
    let all_cases: Vec<_> = test_matrix
        .cases
        .into_iter()
        .filter(|case| case.arch.is_empty() || case.arch.iter().any(|a| a == current_arch))
        .collect();

    let filtered_count = total_cases_count - all_cases.len();
    if filtered_count > 0 {
        println!(
            "{}",
            i18n::t_fmt("filtered_arch_cases", &[&filtered_count, &all_cases.len()]).yellow()
        );
    }

    // Distribute test cases if running in a distributed environment.
    // 如果在分布式环境中运行，则分发测试用例。
    let cases_to_run = match (args.total_runners, args.runner_index) {
        (Some(total), Some(index)) => {
            if index >= total {
                panic!("{}", i18n::t("runner_index_invalid"));
            }
            // Assign cases to this runner based on its index using simple round-robin.
            // 使用简单的轮询方式，根据索引将用例分配给此运行器。
            let cases_for_this_runner = all_cases
                .into_iter()
                .enumerate()
                .filter_map(|(i, case)| if i % total == index { Some(case) } else { None })
                .collect::<Vec<_>>();

            println!(
                "{}",
                i18n::t_fmt("running_as_split_runner", &[&(index + 1), &total, &cases_for_this_runner.len()]).yellow()
            );
            cases_for_this_runner
        }
        (None, None) => {
            // Run all filtered cases if not in a distributed environment.
            // 如果不是在分布式环境中，则运行所有筛选后的用例。
            println!("{}", i18n::t("running_as_single_runner").yellow());
            all_cases
        }
        _ => {
            // The `total_runners` and `runner_index` flags must be used together.
            // `total_runners` 和 `runner_index` 标志必须一起使用。
            panic!("{}", i18n::t("runner_flags_inconsistent"));
        }
    };

    if cases_to_run.is_empty() {
        println!(
            "{}",
            i18n::t("no_cases_to_run").green()
        );
        std::process::exit(0);
    }

    let current_os = std::env::consts::OS;
    println!("{}", i18n::t_fmt("current_os", &[&current_os.yellow()]));

    // Partition cases into "flaky" (allowed to fail on the current OS) and "safe".
    // "Flaky" cases are those where `allow_failure` contains the current OS.
    // 将用例分为 "flaky"（允许在当前操作系统上失败）和 "safe" 两类。
    // "Flaky" 用例是指其 `allow_failure` 字段包含当前操作系统的用例。
    let (flaky_cases, safe_cases): (Vec<_>, Vec<_>) = cases_to_run
        .into_iter()
        .partition(|c| c.allow_failure.iter().any(|os| os == current_os));

    let mut results = Vec::new();

    // --- Run safe cases in parallel ---
    // --- 并行运行 safe 用例 ---
    println!(
        "\n{}",
        i18n::t_fmt("running_safe_cases", &[&safe_cases.len(), &jobs]).cyan()
    );

    // Create a stream of test execution futures.
    // `buffer_unordered` runs up to `jobs` futures concurrently.
    // 创建一个测试执行 future 的流。
    // `buffer_unordered` 会并发运行最多 `jobs` 个 future。
    let mut safe_tests_stream = stream::iter(safe_cases)
        .map(|case| {
            let project_root = project_root.clone();
            let crate_name = crate_name.clone();
            let stop_token = overall_stop_token.clone();
            tokio::spawn(
                async move { run_test_case(case, project_root, crate_name, Some(stop_token)).await },
            )
        })
        .buffer_unordered(jobs);

    // This flag tracks if an unexpected failure has occurred in any of the safe tests.
    // 此标志跟踪在任何 safe 测试中是否发生了意外失败。
    let mut unexpected_failure_observed = false;
    while let Some(res) = safe_tests_stream.next().await {
        let result = res.unwrap(); // Unwrap the JoinHandle result
        match result {
            Ok(test_result) => {
                results.push(test_result);
            }
            Err(test_result) => {
                // Only treat genuine failures as "unexpected".
                // Failures due to cancellation are handled separately.
                // 仅将真正的失败视为“意外”。
                // 由取消导致的失败会分开处理。
                if test_result.failure_reason != Some(runner::models::FailureReason::Cancelled) {
                    if !unexpected_failure_observed {
                        // This is the first unexpected failure.
                        // Signal all other tests to stop and print failure details immediately.
                        // 这是第一个意外失败。
                        // 发信号通知所有其他测试停止，并立即打印失败详情。
                        unexpected_failure_observed = true;
                        overall_stop_token.cancel(); // Signal all other tests to stop.
                        print_unexpected_failure_details(&test_result);
                    }
                }
                results.push(test_result);
            }
        }
    }

    // --- Run flaky cases sequentially ---
    // --- 串行运行 flaky 用例 ---
    if !flaky_cases.is_empty() {
        println!(
            "\n{}",
            i18n::t_fmt("running_flaky_cases", &[&flaky_cases.len()]).cyan()
        );
        for case in flaky_cases {
            // If the stop token is cancelled (due to an earlier unexpected failure or Ctrl+C),
            // skip the remaining flaky tests.
            // 如果停止令牌被取消（由于早期的意外失败或 Ctrl+C），
            // 则跳过剩余的 flaky 测试。
            if overall_stop_token.is_cancelled() {
                results.push(runner::models::TestResult {
                    case,
                    output: i18n::t("test_skipped_due_to_cancellation"),
                    success: false,
                    failure_reason: Some(runner::models::FailureReason::Cancelled),
                });
                continue;
            }
            // Flaky tests run sequentially. Their failure is allowed and should not stop other tests,
            // so we don't pass the overall stop token to them.
            // Flaky 测试是串行运行的。它们的失败是允许的，并且不应停止其他测试，
            // 因此我们不将全局停止令牌传递给它们。
            let result = run_test_case(case, project_root.clone(), crate_name.clone(), None).await;
            match result {
                Ok(res) | Err(res) => {
                    results.push(res);
                }
            }
        }
    }

    // --- Final Summary ---
    // --- 最终总结 ---
    println!("\n{}", i18n::t("all_tests_completed").cyan());
    let unexpected_failures_exist = print_summary(&results);

    // Final status message about directories.
    // 关于目录的最终状态消息。
    println!(
        "\n{}",
        i18n::t("temp_dir_cleanup_end_success").green()
    );
    // If any test failed, remind the user where to find failure artifacts.
    // 如果有任何测试失败，提醒用户在哪里可以找到失败的产物。
    if results.iter().any(|r| !r.success) {
        println!(
            "{}",
            i18n::t("failure_artifact_info").yellow()
        );
    }

    // Exit with a non-zero status code if there were unexpected failures.
    // This is useful for CI/CD environments.
    // 如果存在意外失败，则以非零状态码退出。
    // 这对于 CI/CD 环境很有用。
    if unexpected_failures_exist {
        std::process::exit(1);
    }
}

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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of parallel jobs, defaults to (logical CPUs / 2) + 1
    #[arg(short, long)]
    jobs: Option<usize>,

    /// Path to the test matrix config file, relative to project dir
    #[arg(short, long, default_value = "test-runner/TestMatrix.toml")]
    config: PathBuf,

    /// Path to the project directory to test
    #[arg(long, default_value = ".")]
    project_dir: PathBuf,

    /// Total number of parallel runners for splitting the test matrix
    #[arg(long)]
    total_runners: Option<usize>,

    /// Index of the current runner (0-based) when splitting the test matrix
    #[arg(long)]
    runner_index: Option<usize>,
}

// Structs for parsing Cargo.toml
#[derive(Deserialize)]
struct Package {
    name: String,
}

#[derive(Deserialize)]
struct Manifest {
    package: Package,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let num_cpus = num_cpus::get();
    let jobs = args.jobs.unwrap_or(num_cpus / 2 + 1);

    // Determine the project root from the command-line argument
    let project_root = fs::canonicalize(&args.project_dir).unwrap_or_else(|_| {
        panic!("{}", i18n::t_fmt("project_dir_not_found", &[&args.project_dir.display()]))
    });

    // --- Pre-fetch all dependencies ---
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
    let manifest_path = project_root.join("Cargo.toml");
    let manifest_content = fs::read_to_string(&manifest_path).unwrap_or_else(|_| {
        panic!("{}", i18n::t_fmt("manifest_read_failed", &[&manifest_path.display()]))
    });
    let manifest: Manifest =
        toml::from_str(&manifest_content).expect(i18n::t("manifest_parse_failed").as_str());
    // Cargo converts hyphens in crate names to underscores for symbol names.
    let crate_name = manifest.package.name.replace('-', "_");

    // The config file path is relative to the project root
    let config_path = project_root.join(&args.config);

    let config_content = fs::read_to_string(&config_path)
        .unwrap_or_else(|_| panic!("{}", i18n::t_fmt("config_read_failed", &[&config_path.display()])));

    let test_matrix: TestMatrix =
        toml::from_str(&config_content).expect(i18n::t("config_parse_failed").as_str());

    // Initialize the i18n system
    i18n::init(&test_matrix.language);

    println!("{}", i18n::t_fmt("project_root_detected", &[&project_root.display()]));
    println!("{}", i18n::t_fmt("testing_crate", &[&crate_name.yellow()]));
    println!("{}", i18n::t_fmt("loading_test_matrix", &[&config_path.display()]));

    // Setup a global cancellation token for graceful shutdown
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

    // Move i18n initialization to after parsing test_matrix
    let total_cases_count = test_matrix.cases.len();
    let current_arch = std::env::consts::ARCH;
    println!("{}", i18n::t_fmt("current_arch", &[&current_arch.yellow()]));

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

    let cases_to_run = match (args.total_runners, args.runner_index) {
        (Some(total), Some(index)) => {
            if index >= total {
                panic!("{}", i18n::t("runner_index_invalid"));
            }
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
            println!("{}", i18n::t("running_as_single_runner").yellow());
            all_cases
        }
        _ => {
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

    let (flaky_cases, safe_cases): (Vec<_>, Vec<_>) = cases_to_run
        .into_iter()
        .partition(|c| c.allow_failure.iter().any(|os| os == current_os));

    let mut results = Vec::new();

    // --- Run safe cases in parallel ---
    println!(
        "\n{}",
        i18n::t_fmt("running_safe_cases", &[&safe_cases.len(), &jobs]).cyan()
    );

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

    let mut unexpected_failure_observed = false;
    while let Some(res) = safe_tests_stream.next().await {
        let result = res.unwrap(); // Unwrap the JoinHandle result
        match result {
            Ok(test_result) => {
                results.push(test_result);
            }
            Err(test_result) => {
                // Only treat genuine failures as "unexpected"
                if test_result.failure_reason != Some(runner::models::FailureReason::Cancelled) {
                    if !unexpected_failure_observed {
                        // This is the first unexpected failure.
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
    if !flaky_cases.is_empty() {
        println!(
            "\n{}",
            i18n::t_fmt("running_flaky_cases", &[&flaky_cases.len()]).cyan()
        );
        for case in flaky_cases {
            if overall_stop_token.is_cancelled() {
                results.push(runner::models::TestResult {
                    case,
                    output: i18n::t("test_skipped_due_to_cancellation"),
                    success: false,
                    failure_reason: Some(runner::models::FailureReason::Cancelled),
                });
                continue;
            }
            // Flaky tests run sequentially, so we don't pass the overall stop token.
            // Their failure is allowed and should not stop other tests.
            let result = run_test_case(case, project_root.clone(), crate_name.clone(), None).await;
            match result {
                Ok(res) | Err(res) => {
                    results.push(res);
                }
            }
        }
    }

    println!("\n{}", i18n::t("all_tests_completed").cyan());
    let unexpected_failures_exist = print_summary(&results);

    // Final status message about directories.
    println!(
        "\n{}",
        i18n::t("temp_dir_cleanup_end_success").green()
    );
    if results.iter().any(|r| !r.success) {
        println!(
            "{}",
            i18n::t("failure_artifact_info").yellow()
        );
    }

    if unexpected_failures_exist {
        std::process::exit(1);
    }
}

use clap::Parser;
use colored::*;
use futures::{StreamExt, stream};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::signal;
use tokio_util::sync::CancellationToken;

mod runner;
use runner::config::TestMatrix;
use runner::execution::run_test_case;
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

    // Setup a global cancellation token for graceful shutdown
    let overall_stop_token = CancellationToken::new();
    let signal_token = overall_stop_token.clone();
    tokio::spawn(async move {
        signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C signal");
        println!(
            "\n{}",
            "Ctrl+C received, initiating graceful shutdown...".yellow()
        );
        signal_token.cancel();
    });

    println!(
        "{}",
        "Temporary directories will be auto-cleaned for successful tests.".green()
    );
    println!(
        "{}",
        "Artifacts for failed tests will be preserved in './target-errors'.".yellow()
    );

    // Determine the project root from the command-line argument
    let project_root = fs::canonicalize(&args.project_dir).unwrap_or_else(|_| {
        panic!(
            "Failed to find project directory at: {}",
            args.project_dir.display()
        )
    });

    // --- Pre-fetch all dependencies ---
    println!(
        "\n{}",
        "Fetching all dependencies to avoid lock contention...".cyan()
    );
    let mut fetch_cmd = std::process::Command::new("cargo");
    fetch_cmd.current_dir(&project_root);
    fetch_cmd.arg("fetch");

    let fetch_status = fetch_cmd
        .status()
        .expect("Failed to execute cargo fetch command");

    if !fetch_status.success() {
        panic!("'cargo fetch' failed. Please check your network and Cargo.toml file.");
    }
    println!("{}", "Dependency fetching successful.".green());

    // --- Read crate name from Cargo.toml ---
    let manifest_path = project_root.join("Cargo.toml");
    let manifest_content = fs::read_to_string(&manifest_path).unwrap_or_else(|_| {
        panic!(
            "Failed to read Cargo.toml at: {}",
            manifest_path.display()
        )
    });
    let manifest: Manifest =
        toml::from_str(&manifest_content).expect("Failed to parse Cargo.toml");
    // Cargo converts hyphens in crate names to underscores for symbol names.
    let crate_name = manifest.package.name.replace('-', "_");

    // The config file path is relative to the project root
    let config_path = project_root.join(&args.config);

    println!("Project root detected at: {}", project_root.display());
    println!("Testing crate: {}", crate_name.yellow());
    println!("Loading test matrix from: {}", config_path.display());
    let config_content = fs::read_to_string(&config_path)
        .unwrap_or_else(|_| panic!("Failed to read config file: {}", config_path.display()));

    let test_matrix: TestMatrix =
        toml::from_str(&config_content).expect("Failed to parse TOML config file");

    let total_cases_count = test_matrix.cases.len();
    let current_arch = std::env::consts::ARCH;
    println!("Current architecture detected: {}", current_arch.yellow());

    let all_cases: Vec<_> = test_matrix
        .cases
        .into_iter()
        .filter(|case| case.arch.is_empty() || case.arch.iter().any(|a| a == current_arch))
        .collect();

    let filtered_count = total_cases_count - all_cases.len();
    if filtered_count > 0 {
        println!(
            "{}",
            format!(
                "Filtered out {} test case(s) based on current architecture. {} case(s) remaining.",
                filtered_count,
                all_cases.len()
            )
            .yellow()
        );
    }

    let cases_to_run = match (args.total_runners, args.runner_index) {
        (Some(total), Some(index)) => {
            if index >= total {
                panic!("--runner-index must be less than --total-runners.");
            }
            let cases_for_this_runner = all_cases
                .into_iter()
                .enumerate()
                .filter_map(|(i, case)| if i % total == index { Some(case) } else { None })
                .collect::<Vec<_>>();

            println!(
                "{}",
                format!(
                    "Running as runner {}/{} ({} cases assigned)",
                    index + 1,
                    total,
                    cases_for_this_runner.len()
                )
                .yellow()
            );
            cases_for_this_runner
        }
        (None, None) => {
            println!("{}", "Running all test cases as a single runner.".yellow());
            all_cases
        }
        _ => {
            panic!("--total-runners and --runner-index must be provided together.");
        }
    };

    if cases_to_run.is_empty() {
        println!(
            "{}",
            "No test cases to run for this runner, exiting successfully.".green()
        );
        std::process::exit(0);
    }

    let current_os = std::env::consts::OS;
    println!("Current OS detected: {}", current_os.yellow());

    let (flaky_cases, safe_cases): (Vec<_>, Vec<_>) = cases_to_run
        .into_iter()
        .partition(|c| c.allow_failure.iter().any(|os| os == current_os));

    let mut results = Vec::new();

    // --- Run safe cases in parallel ---
    println!(
        "\n{}",
        format!(
            "Running {} safe configurations with up to {} parallel jobs...",
            safe_cases.len(),
            jobs
        )
        .cyan()
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
            format!(
                "Running {} platform-specific (may fail) configurations sequentially...",
                flaky_cases.len()
            )
            .yellow()
        );
        for case in flaky_cases {
            if overall_stop_token.is_cancelled() {
                println!(
                    "{}",
                    format!("Shutdown triggered, skipping test: {}", case.name).yellow()
                );
                results.push(runner::models::TestResult {
                    case,
                    output: "Test skipped due to cancellation.".to_string(),
                    success: false,
                    failure_reason: Some(runner::models::FailureReason::Cancelled),
                });
            } else {
                let result = run_test_case(
                    case,
                    project_root.clone(),
                    crate_name.clone(),
                    Some(overall_stop_token.clone()),
                )
                .await;
                match result {
                    Ok(res) => {
                        results.push(res);
                    }
                    Err(res) => {
                        // Only treat genuine failures as "unexpected"
                        if res.failure_reason != Some(runner::models::FailureReason::Cancelled) {
                            let failure_allowed =
                                res.case.allow_failure.iter().any(|os| os == current_os);
                            if !failure_allowed {
                                if !unexpected_failure_observed {
                                    unexpected_failure_observed = true;
                                    overall_stop_token.cancel();
                                    print_unexpected_failure_details(&res);
                                }
                            }
                        }
                        results.push(res);
                    }
                }
            }
        }
    }

    let has_unexpected_failures = print_summary(&results);

    // Final status message about directories.
    println!(
        "{}",
        "\nTemporary build directories for successful tests have been cleaned up automatically."
            .green()
    );
    if results.iter().any(|r| !r.success) {
        println!(
            "{}",
            "Build artifacts for any failed tests have been preserved in './target-errors'."
                .yellow()
        );
    }

    if has_unexpected_failures {
        println!("{}", "TEST MATRIX FAILED".red());
        std::process::exit(1);
    } else {
        println!("{}", "TEST MATRIX PASSED SUCCESSFULLY".green());
        std::process::exit(0);
    }
}

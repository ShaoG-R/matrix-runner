use crate::runner::command::format_build_error_output;
use crate::runner::i18n;
use crate::runner::models::{FailureReason, TestResult};
use colored::*;

/// Prints details for an unexpected test failure.
/// This function is called when a non-ignored test fails, to provide immediate feedback.
pub fn print_unexpected_failure_details(result: &TestResult) {
    println!(
        "{}",
        "=================================================================".cyan()
    );
    println!("{}", i18n::t("unexpected_failure_banner").red().bold());
    println!(
        "{}",
        i18n::t_fmt("failure_details_for", &[&result.case.name]).cyan()
    );
    println!(
        "{}",
        "-----------------------------------------------------------------".cyan()
    );

    let output_to_print = match &result.failure_reason {
        Some(FailureReason::Build) => format_build_error_output(&result.output),
        _ => result.output.clone(), // For Test failures or unknown, print raw output
    };
    println!("{}", output_to_print);

    println!(
        "{}",
        "-----------------------------------------------------------------".cyan()
    );
    println!(
        "{}",
        i18n::t("stopping_other_tests").yellow()
    );
}

/// Prints the final summary of all test results.
/// Returns true if there were any unexpected failures or cancellations.
pub fn print_summary(results: &[TestResult]) -> bool {
    let mut successes = Vec::new();
    let mut allowed_failures = Vec::new();
    let mut unexpected_failures = Vec::new();
    let mut cancelled_tests = Vec::new();

    let current_os = std::env::consts::OS;

    for result in results {
        if result.success {
            successes.push(result);
        } else {
            if result.failure_reason == Some(FailureReason::Cancelled) {
                cancelled_tests.push(result);
                continue;
            }

            let failure_allowed = result.case.allow_failure.iter().any(|os| os == current_os);

            if failure_allowed {
                allowed_failures.push(result);
            } else {
                unexpected_failures.push(result);
            }
        }
    }

    println!(
        "\n{}",
        i18n::t("final_summary_banner").cyan()
    );

    if !successes.is_empty() {
        println!("\n{}", i18n::t("summary_successful_tests").green());
        for result in successes {
            println!("  - {}", result.case.name.green());
        }
    }

    if !allowed_failures.is_empty() {
        println!("\n{}", i18n::t("summary_allowed_failures").yellow());
        for result in allowed_failures {
            println!(
                "  - {}",
                i18n::t_fmt(
                    "summary_failed_as_expected",
                    &[&result.case.name.yellow(), &current_os]
                )
            );
        }
    }

    if !cancelled_tests.is_empty() {
        println!("\n{}", i18n::t("summary_cancelled_tests").yellow());
        for result in &cancelled_tests {
            println!(
                "  - {}",
                i18n::t_fmt("summary_cancelled_test_case", &[&result.case.name.yellow()])
            );
        }
    }

    if !unexpected_failures.is_empty() {
        println!("\n{}", i18n::t("summary_unexpected_failures").red().bold());
        for result in &unexpected_failures {
            let failure_type = match result.failure_reason {
                Some(FailureReason::Build) => i18n::t("build_failure"),
                Some(FailureReason::Test) => i18n::t("test_failure"),
                _ => i18n::t("unhandled_error"),
            };
            println!(
                "  - {}",
                i18n::t_fmt(
                    "summary_unexpected_failure_case",
                    &[&result.case.name.red(), &failure_type]
                )
            );
        }
    }

    println!(); // Add a blank line for spacing

    if !unexpected_failures.is_empty() {
        println!("{}", i18n::t("overall_failure").red().bold());
        true
    } else if !cancelled_tests.is_empty() {
        println!("{}", i18n::t("overall_cancelled").yellow().bold());
        true
    } else {
        println!("{}", i18n::t("overall_success").green().bold());
        false
    }
}

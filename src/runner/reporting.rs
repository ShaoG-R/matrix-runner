use crate::runner::command::format_build_error_output;
use crate::runner::models::{FailureReason, TestResult};
use colored::*;

/// Prints details for an unexpected test failure.
/// This function is called when a non-ignored test fails, to provide immediate feedback.
pub fn print_unexpected_failure_details(result: &TestResult) {
    println!(
        "{}",
        "=================================================================".cyan()
    );
    println!("{}", "UNEXPECTED FAILURE DETECTED".red().bold());
    println!(
        "{}",
        format!("  Failure details for: {}", result.case.name).cyan()
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
        "Signaling all other running tests to stop. A final summary will be printed at the end."
            .yellow()
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
            // Distinguish between genuine failures and cancellations
            if result.failure_reason == Some(FailureReason::Cancelled) {
                cancelled_tests.push(result);
                continue;
            }

            // Check if the failure was allowed for the current OS
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
        "==================== FINAL SUMMARY ====================".cyan()
    );

    if !successes.is_empty() {
        println!("\n{}", "--- Successful Tests ---".green());
        for result in successes {
            println!("  - {}", result.case.name.green());
        }
    }

    if !allowed_failures.is_empty() {
        println!("\n{}", "--- Allowed Failures ---".yellow());
        for result in allowed_failures {
            println!(
                "  - {} (failed as expected on {})",
                result.case.name.yellow(),
                current_os
            );
        }
    }

    if !cancelled_tests.is_empty() {
        println!("\n{}", "--- Cancelled Tests ---".yellow());
        for result in &cancelled_tests {
            println!("  - {} (Cancelled)", result.case.name.yellow());
        }
    }

    if !unexpected_failures.is_empty() {
        println!("\n{}", "--- Unexpected Failures ---".red().bold());
        for result in &unexpected_failures {
            let failure_type = match result.failure_reason {
                Some(FailureReason::Build) => "Build",
                Some(FailureReason::Test) => "Test",
                _ => "Unknown", // Should not happen with current logic
            };
            println!("  - {} ({} Failure)", result.case.name.red(), failure_type);
        }
    }

    println!(); // Add a blank line for spacing

    if !unexpected_failures.is_empty() {
        println!("{}", "TEST MATRIX FAILED".red().bold());
        true
    } else if !cancelled_tests.is_empty() {
        println!("{}", "TEST MATRIX EXECUTION CANCELLED".yellow().bold());
        true
    } else {
        println!("{}", "TEST MATRIX PASSED SUCCESSFULLY".green().bold());
        false
    }
}

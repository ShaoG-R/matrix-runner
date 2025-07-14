use crate::runner::command::format_build_error_output;
use crate::runner::i18n;
use crate::runner::i18n::I18nKey;
use crate::runner::models::{FailureReason, TestResult};
use colored::*;

pub fn print_unexpected_failure_details(unexpected_failures: &[&TestResult]) {
    if unexpected_failures.is_empty() {
        return;
    }

    println!(
        "\n{}",
        "=================================================================".cyan()
    );
    println!("{}", i18n::t(I18nKey::UnexpectedFailureBanner).red().bold());

    for result in unexpected_failures {
        if let TestResult::Failed { case, output, reason } = result {
            println!(
                "\n{}",
                i18n::t_fmt(I18nKey::FailureDetailsFor, &[&case.name]).cyan()
            );
            println!(
                "{}",
                "-----------------------------------------------------------------".cyan()
            );

            let output_to_print = match reason {
                FailureReason::Build => format_build_error_output(output),
                FailureReason::Test => output.clone(),
            };
            println!("{}", output_to_print);
        }
    }

    println!(
        "{}",
        "-----------------------------------------------------------------".cyan()
    );
}

pub fn print_summary<'a>(results: &'a [TestResult]) -> Vec<&'a TestResult> {
    let mut successes = Vec::new();
    let mut allowed_failures = Vec::new();
    let mut unexpected_failures = Vec::new();
    let mut skipped_tests = Vec::new();

    let current_os = std::env::consts::OS;

    for result in results {
        match result {
            TestResult::Passed { .. } => successes.push(result),
            TestResult::Skipped => skipped_tests.push(result),
            TestResult::Failed { case, .. } => {
                let failure_allowed = case.allow_failure.iter().any(|os| os == current_os);
                if failure_allowed {
                    allowed_failures.push(result);
                } else {
                    unexpected_failures.push(result);
                }
            }
        }
    }

    println!(
        "\n{}",
        i18n::t(I18nKey::FinalSummaryBanner).cyan()
    );

    if !successes.is_empty() {
        println!("\n{}", i18n::t(I18nKey::SummarySuccessfulTests).green());
        for result in successes {
            if let TestResult::Passed { case, .. } = result {
                println!("  - {}", case.name.green());
            }
        }
    }

    if !allowed_failures.is_empty() {
        println!("\n{}", i18n::t(I18nKey::SummaryAllowedFailures).yellow());
        for result in allowed_failures {
            if let TestResult::Failed { case, .. } = result {
                println!(
                    "  - {}",
                    i18n::t_fmt(
                        I18nKey::SummaryFailedAsExpected,
                        &[&case.name.yellow(), &current_os]
                    )
                );
            }
        }
    }

    if !skipped_tests.is_empty() {
        println!("\n{}", i18n::t(I18nKey::SummarySkippedTests).yellow());
        println!("  - {}", i18n::t_fmt(I18nKey::SummarySkippedCount, &[&skipped_tests.len()]));
    }

    if !unexpected_failures.is_empty() {
        println!("\n{}", i18n::t(I18nKey::SummaryUnexpectedFailures).red().bold());
        for result in &unexpected_failures {
            if let TestResult::Failed { case, reason, .. } = result {
                let failure_type = match reason {
                    FailureReason::Build => i18n::t(I18nKey::BuildFailure),
                    FailureReason::Test => i18n::t(I18nKey::TestFailure),
                };
                println!(
                    "  - {}",
                    format!("{} ({})", case.name.red(), failure_type)
                );
            }
        }
    }

    println!();

    if !unexpected_failures.is_empty() {
        println!("{}", i18n::t(I18nKey::OverallFailure).red().bold());
    } else {
        println!("{}", i18n::t(I18nKey::OverallSuccess).green().bold());
    }
    
    unexpected_failures
}

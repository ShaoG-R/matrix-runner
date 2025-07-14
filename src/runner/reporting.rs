use crate::runner::command::format_build_error_output;
use crate::runner::i18n;
use crate::runner::models::{FailureReason, TestResult};
use colored::*;

/// Prints a detailed, formatted report for an unexpected test failure.
/// This function is called immediately when a "safe" (non-ignored) test fails,
/// providing instant feedback without waiting for all other tests to complete.
///
/// # Arguments
/// * `result` - The `TestResult` of the failed test case.
///
/// 打印一个详细、格式化的非预期测试失败报告。
/// 当一个“安全”（非忽略）的测试失败时，会立即调用此函数，
/// 提供即时反馈，而无需等待所有其他测试完成。
///
/// # Arguments
/// * `result` - 失败测试用例的 `TestResult`。
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

    // Format the output differently depending on the failure reason.
    // For build failures, we parse and show only the relevant compiler errors.
    // For test failures, we show the full raw output from the test executable.
    // 根据失败原因以不同方式格式化输出。
    // 对于构建失败，我们解析并仅显示相关的编译器错误。
    // 对于测试失败，我们显示测试可执行文件的完整原始输出。
    let output_to_print = match &result.failure_reason {
        Some(FailureReason::Build) => format_build_error_output(&result.output),
        _ => result.output.clone(),
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

/// Prints the final summary of all test results after all executions are complete.
/// It categorizes results into successes, allowed failures, unexpected failures, and
/// cancelled tests, then prints each category in a formatted list.
///
/// # Arguments
/// * `results` - A slice of `TestResult` from all completed test cases.
///
/// # Returns
/// Returns `true` if there were any unexpected failures or cancellations, which is
/// used to set the process exit code. Otherwise, returns `false`.
///
/// 在所有测试执行完毕后，打印最终的全部测试结果摘要。
/// 它将结果分为成功、允许的失败、意外的失败和已取消的测试，
/// 然后以格式化列表的形式打印每个类别。
///
/// # Arguments
/// * `results` - 所有已完成测试用例的 `TestResult` 切片。
///
/// # Returns
/// 如果存在任何意外失败或取消，则返回 `true`，用于设置进程退出码。
/// 否则返回 `false`。
pub fn print_summary(results: &[TestResult]) -> bool {
    let mut successes = Vec::new();
    let mut allowed_failures = Vec::new();
    let mut unexpected_failures = Vec::new();
    let mut cancelled_tests = Vec::new();

    let current_os = std::env::consts::OS;

    // Categorize each result.
    // 将每个结果分类。
    for result in results {
        if result.success {
            successes.push(result);
        } else {
            // A failure due to cancellation is a special category.
            // 因取消而导致的失败是一个特殊的类别。
            if result.failure_reason == Some(FailureReason::Cancelled) {
                cancelled_tests.push(result);
                continue;
            }

            // Check if the failure was expected on the current operating system.
            // 检查当前操作系统上是否预期会发生此失败。
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

    // Print each category if it's not empty.
    // 如果每个类别不为空，则打印它。
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
                // Should not happen, but included for completeness.
                // 理论上不应发生，但为完整性而包含。
                _ => i18n::t("unhandled_error"),
            };
            println!(
                "  - {}",
                format!("{} ({})", result.case.name.red(), failure_type)
            );
        }
    }

    println!(); // Add a blank line for spacing.

    // Determine the overall status of the test run.
    // An exit code of 1 will be triggered for unexpected failures or cancellations.
    // 确定测试运行的总体状态。
    // 对于意外失败或取消，将触发退出码 1。
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

//! # Console Reporting Module / 控制台报告模块
//!
//! This module handles the generation and display of test reports in the console.
//! It provides functionality for printing colorful, formatted summaries with
//! internationalization support.
//!
//! 此模块处理控制台中测试报告的生成和显示。
//! 它提供打印彩色格式化摘要的功能，支持国际化。

use colored::*;
use crate::core::models::{FailureReason, TestResult};
use crate::infra::t;
use crate::infra::command::format_build_error_output;

/// Prints a formatted summary of test results to the console.
/// Displays a table with test status, name, duration, and retry information,
/// using color coding to highlight different statuses.
///
/// 在控制台打印格式化的测试结果摘要。
/// 显示一个包含测试状态、名称、持续时间和重试信息的表格，
/// 使用颜色编码突出显示不同的状态。
///
/// # Arguments / 参数
/// * `results` - A slice of test results to summarize
///               要总结的测试结果切片
/// * `locale` - The language locale to use for messages
///              用于消息的语言区域设置
///
/// # Output Format / 输出格式
/// ```text
/// --- Test Summary ---
///   - Status           | Test Name                               | Duration   Retries
///   - Passed           | test_case_1                             |     1.23s
///   - Failed           | test_case_2                             |     0.45s  (2 retries)
///   - Allowed Failure  | test_case_3                             |     2.10s
///   - Skipped          | test_case_4                             |       N/A
/// ```
pub fn print_summary(results: &[TestResult], locale: &str) {
    println!("\n{}", t!("test_summary_banner", locale = locale).bold());

    for result in results {
        let status_str = result.get_status_str(locale);
        let duration_str = result
            .get_duration()
            .map(|d| format!("{:.2?}", d))
            .unwrap_or_else(|| "N/A".to_string());

        let name = result.case_name();
        let retries_str = {
            let retries = result.get_retries();
            if retries > 1 {
                format!(" ({} retries)", retries - 1)
            } else {
                String::new()
            }
        };

        let status_colored = match result {
            TestResult::Passed { .. } => status_str.green(),
            TestResult::Failed { case, .. } => {
                let current_os = std::env::consts::OS;
                if case.allow_failure.iter().any(|os| os == current_os) {
                    status_str.yellow()
                } else {
                    status_str.red()
                }
            }
            TestResult::Skipped => status_str.dimmed(),
        };

        println!(
            "  - {:<18} | {:<40} | {:>10} {}",
            status_colored, name, duration_str, retries_str
        );
    }
}

/// Prints detailed information about unexpected test failures.
/// Shows the full output and error details for each test that failed unexpectedly,
/// helping developers debug issues. Only displays failures that were not marked
/// as allowed failures for the current platform.
///
/// 打印意外测试失败的详细信息。
/// 显示每个意外失败测试的完整输出和错误详情，
/// 帮助开发者调试问题。仅显示在当前平台上未标记为允许失败的失败测试。
///
/// # Arguments / 参数
/// * `unexpected_failures` - A slice of test results that failed unexpectedly
///                           意外失败的测试结果切片
/// * `locale` - The language locale to use for messages
///              用于消息的语言区域设置
///
/// # Behavior / 行为
/// - Returns early if no unexpected failures are found
/// - Formats build errors differently from test execution errors
/// - Uses colored output to improve readability
/// - Includes separator lines for visual clarity
///
/// - 如果没有发现意外失败则提前返回
/// - 构建错误和测试执行错误的格式不同
/// - 使用彩色输出提高可读性
/// - 包含分隔线以提高视觉清晰度
pub fn print_unexpected_failure_details(unexpected_failures: &[&TestResult], locale: &str) {
    if unexpected_failures.is_empty() {
        return;
    }

    println!("\n{}", t!("unexpected_failure_banner", locale = locale).red().bold());
    println!("{}", "-".repeat(80));

    for (i, result) in unexpected_failures.iter().enumerate() {
        println!(
            "[{}/{}] {} '{}'",
            i + 1,
            unexpected_failures.len(),
            t!("report_header_failure", locale = locale).red(),
            result.case_name().cyan()
        );

        if let TestResult::Failed { output, reason, .. } = result {
            let log_header = match reason {
                FailureReason::Build | FailureReason::BuildFailed => t!("build_log", locale = locale),
                _ => t!("test_log", locale = locale),
            };
            println!("\n--- {} ---\n", log_header.yellow());
            println!("{}", output);
            println!("\n{}", "-".repeat(80));
        }
    }
}

/// Gets the error output from a test result for display.
///
/// 获取测试结果的错误输出以供显示。
///
/// # Arguments
/// * `result` - The test result to extract error output from
/// * `locale` - The language locale to use for messages
///
/// # Returns
/// A formatted string containing the error output
pub fn get_error_output_from_result(result: &TestResult, locale: &str) -> String {
    match result {
        TestResult::Failed { output, reason, .. } => {
            // For build errors, try to format the output as Cargo JSON messages
            if *reason == FailureReason::Build || *reason == FailureReason::BuildFailed {
                format_build_error_output(output)
            } else {
                output.clone()
            }
        }
        _ => t!("no_error_output", locale = locale).to_string(),
    }
} 
//! # Test Reporting Module / 测试报告模块
//!
//! This module handles the generation and display of test reports in multiple formats.
//! It provides functionality for creating styled HTML reports and printing colorful,
//! formatted summaries to the console with internationalization support.
//!
//! 此模块处理多种格式的测试报告生成和显示。
//! 它提供创建样式化 HTML 报告和在控制台打印彩色格式化摘要的功能，支持国际化。
//!
//! ## Features / 功能特性
//!
//! - **HTML Report Generation**: Creates comprehensive HTML reports with embedded CSS and JavaScript
//! - **Console Summary**: Displays colorful test summaries in the terminal
//! - **Failure Details**: Shows detailed failure information for debugging
//! - **Internationalization**: All output text supports multiple languages
//! - **Statistics**: Provides detailed test execution statistics
//!
//! - **HTML 报告生成**: 创建包含嵌入式 CSS 和 JavaScript 的综合 HTML 报告
//! - **控制台摘要**: 在终端显示彩色测试摘要
//! - **失败详情**: 显示详细的失败信息用于调试
//! - **国际化**: 所有输出文本支持多种语言
//! - **统计信息**: 提供详细的测试执行统计

use crate::t;
use crate::runner::command::format_build_error_output;
use crate::runner::models::{self, FailureReason, TestResult};
use anyhow::{Context, Result};
use colored::*;
use maud::{DOCTYPE, PreEscaped, html};
use std::fs;
use std::path::Path;

/// Embedded CSS styles for HTML reports / HTML 报告的嵌入式 CSS 样式
const HTML_STYLE: &str = include_str!("assets/report.css");

/// Embedded JavaScript for HTML report interactivity / HTML 报告交互性的嵌入式 JavaScript
const HTML_SCRIPT: &str = include_str!("assets/report.js");

/// Generates a comprehensive HTML report from test results.
/// Creates a styled HTML file with test statistics, detailed results table,
/// and interactive features for viewing test output.
///
/// 从测试结果生成综合的 HTML 报告。
/// 创建一个样式化的 HTML 文件，包含测试统计、详细结果表格和查看测试输出的交互功能。
///
/// # Arguments / 参数
/// * `results` - A slice of test results to include in the report
///               要包含在报告中的测试结果切片
/// * `output_path` - The file path where the HTML report will be saved
///                   保存 HTML 报告的文件路径
///
/// # Returns / 返回值
/// * `Result<()>` - Success or error information
///                  成功或错误信息
///
/// # Errors / 错误
/// This function will return an error if:
/// - The output file cannot be written to the specified path
/// - File system permissions prevent writing
///
/// 此函数在以下情况下会返回错误：
/// - 无法将输出文件写入指定路径
/// - 文件系统权限阻止写入
pub fn generate_html_report(
    results: &[TestResult],
    output_path: &std::path::Path,
) -> Result<(), anyhow::Error> {
    let mut html = String::new();
    html.push_str("<!DOCTYPE html><html><head><title>Test Report</title>");
    let css_content = include_str!("assets/report.css");
    html.push_str("<style>");
    html.push_str(css_content);
    html.push_str("</style>");
    html.push_str("</head><body><h1>Test Matrix Report</h1>");
    html.push_str("<table><tr><th>#</th><th>Name</th><th>Result</th><th>Time (s)</th></tr>");

    for (i, result) in results.iter().enumerate() {
        let (status_class, status_text, duration_text, error_details) = match result {
            models::TestResult::Passed { duration, .. } => (
                "passed",
                "Passed",
                format!("{:.2}", duration.as_secs_f64()),
                String::new(),
            ),
            models::TestResult::Failed {
                reason,
                duration,
                output,
                ..
            } => {
                let reason_text = match reason {
                    models::FailureReason::Build | models::FailureReason::BuildFailed => "Build",
                    models::FailureReason::TestFailed => "Test",
                    models::FailureReason::Timeout => "Timeout",
                    models::FailureReason::CustomCommand => "Command",
                };
                let error_output = get_error_output_from_result(result, "en"); // Assuming English for now
                (
                    "failed",
                    "Failed",
                    format!("{:.2}", duration.as_secs_f64()),
                    format!(
                        "<div class='error-details'><strong>Reason: {}</strong><pre>{}</pre></div>",
                        reason_text, error_output
                    ),
                )
            }
            models::TestResult::Skipped => ("skipped", "Skipped", "N/A".to_string(), String::new()),
        };

        html.push_str(&format!(
            "<tr class='{}' onclick='toggleError(this)'><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            status_class,
            i + 1,
            result.case_name(),
            status_text,
            duration_text
        ));
        if !error_details.is_empty() {
            html.push_str(&format!(
                "<tr class='error-row'><td colspan='4'>{}</td></tr>",
                error_details
            ));
        }
    }

    html.push_str("</table>");
    let js_content = include_str!("assets/report.js");
    html.push_str("<script>");
    html.push_str(js_content);
    html.push_str("</script>");
    html.push_str("</body></html>");

    fs::write(output_path, html)?;
    Ok(())
}

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

        if let models::TestResult::Failed { output, reason, .. } = result {
            let log_header = match reason {
                models::FailureReason::Build | models::FailureReason::BuildFailed => t!("build_log", locale = locale),
                _ => t!("test_log", locale = locale),
            };
            println!("\n--- {} ---\n", log_header.yellow());
            println!("{}", output);
            println!("\n{}", "-".repeat(80));
        }
    }
}

fn get_error_output_from_result(result: &models::TestResult, locale: &str) -> String {
    match result {
        models::TestResult::Failed { reason, output, .. } => match reason {
            models::FailureReason::Build | models::FailureReason::BuildFailed => {
                format!("{}\n\n{}", t!("build_log", locale = locale), output)
            }
            _ => output.clone(),
        },
        _ => String::new(),
    }
}

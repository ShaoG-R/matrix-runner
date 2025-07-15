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

use crate::runner::command::format_build_error_output;
use crate::runner::i18n::{self, I18nKey};
use crate::runner::models::{FailureReason, TestResult};
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
pub fn generate_html_report(results: &[TestResult], output_path: &Path) -> Result<()> {
    let passed_count = results
        .iter()
        .filter(|r| matches!(r, TestResult::Passed { .. }))
        .count();
    let allowed_failures_count = results
        .iter()
        .filter(|r| matches!(r, TestResult::Failed { .. }) && !r.is_unexpected_failure())
        .count();
    let skipped_count = results
        .iter()
        .filter(|r| matches!(r, TestResult::Skipped))
        .count();
    let timeout_count = results
        .iter()
        .filter(|r| {
            matches!(
                r,
                TestResult::Failed {
                    reason: FailureReason::Timeout,
                    ..
                }
            )
        })
        .count();
    let total_unexpected_failures = results.iter().filter(|r| r.is_unexpected_failure()).count();
    let failed_count = total_unexpected_failures - timeout_count;
    let total_count = results.len();
    let total_duration: std::time::Duration = results.iter().filter_map(|r| r.get_duration()).sum();

    let formatted_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let markup = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { (i18n::t(I18nKey::HtmlReportTitle)) }
                style { (PreEscaped(HTML_STYLE)) }
                script { (PreEscaped(HTML_SCRIPT)) }
            }
            body {
                div class="container" {
                    h1 { (i18n::t(I18nKey::HtmlReportTitle)) }
                    p { (i18n::t(I18nKey::HtmlReportGeneratedAt)) ": " (formatted_time) }

                    h2 { (i18n::t(I18nKey::HtmlReportSummary)) }
                    div class="summary-container" {
                        div class="summary-item" {
                            span class="count" { (total_count) }
                            span class="label" { (i18n::t(I18nKey::HtmlReportTotal)) }
                        }
                        div class="summary-item" {
                            span class="count" style=(format!("color: {};", "var(--color-passed)")) { (passed_count) }
                            span class="label" { (i18n::t(I18nKey::HtmlReportPassed)) }
                        }
                        div class="summary-item" {
                            span class="count" style=(format!("color: {};", "var(--color-failed)")) { (failed_count) }
                            span class="label" { (i18n::t(I18nKey::HtmlReportFailed)) }
                        }
                        div class="summary-item" {
                            span class="count" style=(format!("color: {};", "var(--color-timeout)")) { (timeout_count) }
                            span class="label" { (i18n::t(I18nKey::TimeoutFailure)) }
                        }
                        div class="summary-item" {
                            span class="count" style=(format!("color: {};", "var(--color-allowed-failure)")) { (allowed_failures_count) }
                            span class="label" { (i18n::t(I18nKey::HtmlReportAllowedFailures)) }
                        }
                        div class="summary-item" {
                            span class="count" style=(format!("color: {};", "var(--color-skipped)")) { (skipped_count) }
                            span class="label" { (i18n::t(I18nKey::HtmlReportSkipped)) }
                        }
                        div class="summary-item" {
                            span class="count" { (format!("{:.2?}", total_duration)) }
                            span class="label" { (i18n::t(I18nKey::HtmlReportTotalDuration)) }
                        }
                    }

                    table {
                        thead {
                            tr {
                                th class="status-col" { (i18n::t(I18nKey::HtmlReportStatus)) }
                                th { (i18n::t(I18nKey::HtmlReportName)) }
                                th { (i18n::t(I18nKey::HtmlReportFeatures)) }
                                th class="duration-cell" { (i18n::t(I18nKey::HtmlReportDuration)) }
                                th class="retries-cell" { (i18n::t(I18nKey::HtmlReportRetries)) }
                                th { (i18n::t(I18nKey::HtmlReportOutput)) }
                            }
                        }
                        tbody {
                            @for (i, result) in results.iter().enumerate() {
                                tr {
                                    td class="status-col" {
                                        span class=(format!("status-cell status-{}", result.get_status_str().replace(' ', "-"))) {
                                            (result.get_status_str())
                                        }
                                    }
                                    td { (result.get_name()) }
                                    td { (result.get_features()) }
                                    td class="duration-cell" {
                                        @if let Some(duration) = result.get_duration() {
                                            (format!("{:.2?}", duration))
                                        } @else {
                                            "N/A"
                                        }
                                    }
                                    td class="retries-cell" {
                                        @let retries = result.get_retries();
                                        @if retries > 1 {
                                            (format!("{}", retries - 1))
                                        } @else {
                                            "0"
                                        }
                                    }
                                    td {
                                        @let output = result.get_output();
                                        @if !output.is_empty() {
                                            div {
                                                span class="output-toggle" onclick=(format!("toggleOutput('output-{}')", i)) {
                                                    (i18n::t(I18nKey::HtmlReportShowOutput))
                                                }
                                                pre id=(format!("output-{}", i)) class="output-content" style="display: none;" {
                                                    (output)
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    .into_string();

    fs::write(output_path, markup)
        .with_context(|| i18n::t_fmt(I18nKey::HtmlReportWriteFailed, &[&output_path.display()]))
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
pub fn print_summary(results: &[TestResult]) {
    println!("\n{}", "--- Test Summary ---".bold());

    for result in results {
        let status_str = result.get_status_str();
        let duration_str = result
            .get_duration()
            .map(|d| format!("{:.2?}", d))
            .unwrap_or_else(|| "N/A".to_string());

        let name = result.get_name();
        let retries_str = {
            let retries = result.get_retries();
            if retries > 1 {
                format!(" ({} retries)", retries - 1)
            } else {
                String::new()
            }
        };

        let status_colored = match status_str {
            "Passed" => status_str.green(),
            "Failed" | "Timeout" => status_str.red(),
            "Allowed Failure" => status_str.yellow(),
            "Skipped" => status_str.dimmed(),
            _ => status_str.normal(),
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
pub fn print_unexpected_failure_details(unexpected_failures: &[&TestResult]) {
    // Early return if no unexpected failures / 如果没有意外失败则提前返回
    if unexpected_failures.is_empty() {
        return;
    }

    println!(
        "\n{}",
        "=================================================================".cyan()
    );
    println!("{}", i18n::t(I18nKey::UnexpectedFailureBanner).red().bold());

    for result in unexpected_failures {
        if let TestResult::Failed {
            case,
            output,
            reason,
            duration: _,
        } = result
        {
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
                FailureReason::Timeout => output.clone(),
                FailureReason::CustomCommand => output.clone(),
            };
            println!("{output_to_print}");
        }
    }

    println!(
        "{}",
        "-----------------------------------------------------------------".cyan()
    );
}

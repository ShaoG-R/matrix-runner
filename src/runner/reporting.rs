//! # Test Reporting Module / 测试报告模块
//!
//! This module provides functionality for generating test reports and displaying test results.
//! It supports both console output with colored formatting and HTML report generation.
//!
//! 此模块提供生成测试报告和显示测试结果的功能。
//! 它支持带有彩色格式的控制台输出和 HTML 报告生成。
//!
//! ## Features / 功能特性
//!
//! - **Console Reporting**: Colored output for test summaries and failure details
//! - **HTML Report Generation**: Interactive HTML reports with expandable output sections
//! - **Internationalization**: Support for multiple languages through the i18n module
//! - **Failure Categorization**: Distinguishes between expected and unexpected failures
//!
//! - **控制台报告**: 为测试摘要和失败详情提供彩色输出
//! - **HTML 报告生成**: 带有可展开输出部分的交互式 HTML 报告
//! - **国际化**: 通过 i18n 模块支持多种语言
//! - **失败分类**: 区分预期失败和意外失败

use crate::runner::command::format_build_error_output;
use crate::runner::i18n;
use crate::runner::i18n::I18nKey;
use crate::runner::models::{FailureReason, TestResult};
use anyhow::{Context, Result};
use chrono::Local;
use colored::*;
use maud::{PreEscaped, html};
use std::fs;
use std::path::Path;

/// Prints detailed information about unexpected test failures to the console.
/// This function displays failure details in a formatted, colored output for better readability.
///
/// # Arguments / 参数
/// * `unexpected_failures` - A slice of references to `TestResult` instances that represent unexpected failures
///                          一个包含代表意外失败的 `TestResult` 实例引用的切片
///
/// # Behavior / 行为
/// - If no unexpected failures are provided, the function returns early without output
/// - Displays a banner to separate failure details from other output
/// - For each failure, shows the test case name and formatted output
/// - Distinguishes between build failures and test execution failures
/// - Uses colored output for better visual distinction
///
/// - 如果没有提供意外失败，函数会提前返回而不输出任何内容
/// - 显示横幅以将失败详情与其他输出分开
/// - 对于每个失败，显示测试用例名称和格式化的输出
/// - 区分构建失败和测试执行失败
/// - 使用彩色输出以获得更好的视觉区分
///
/// 打印意外测试失败的详细信息到控制台。
/// 此函数以格式化的彩色输出显示失败详情，以提高可读性。
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
        if let TestResult::Failed {
            case,
            output,
            reason,
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

            // Format the output based on the failure reason
            // 根据失败原因格式化输出
            let output_to_print = match reason {
                FailureReason::Build => format_build_error_output(output),
                FailureReason::Test => output.clone(),
            };
            println!("{output_to_print}");
        }
    }

    println!(
        "{}",
        "-----------------------------------------------------------------".cyan()
    );
}

/// CSS styles for the HTML test report.
/// This constant contains all the styling rules for the generated HTML report,
/// including responsive design, color schemes, and interactive elements.
///
/// HTML 测试报告的 CSS 样式。
/// 此常量包含生成的 HTML 报告的所有样式规则，
/// 包括响应式设计、配色方案和交互元素。
///
/// # Features / 功能特性
/// - CSS custom properties for consistent theming / 用于一致主题的 CSS 自定义属性
/// - Responsive design for different screen sizes / 适应不同屏幕尺寸的响应式设计
/// - Color-coded status badges for test results / 测试结果的彩色状态徽章
/// - Interactive output sections with show/hide functionality / 带有显示/隐藏功能的交互式输出部分
const HTML_STYLE: &str = r#"
:root {
    --color-bg: #f8f9fa;
    --color-text: #212529;
    --color-border: #dee2e6;
    --color-header-bg: #ffffff;
    --color-passed: #28a745;
    --color-passed-bg: #e9f5ec;
    --color-failed: #dc3545;
    --color-failed-bg: #f8d7da;
    --color-allowed-failure: #ffc107;
    --color-allowed-failure-bg: #fff8e1;
    --color-skipped: #6c757d;
    --color-skipped-bg: #f1f3f5;
    --font-family-sans-serif: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
    --font-family-monospace: SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
}
body {
    font-family: var(--font-family-sans-serif);
    background-color: var(--color-bg);
    color: var(--color-text);
    margin: 0;
    padding: 2rem;
}
.container {
    max-width: 1200px;
    margin: 0 auto;
    background-color: var(--color-header-bg);
    border-radius: 8px;
    box-shadow: 0 4px 6px rgba(0,0,0,0.1);
    overflow: hidden;
}
header {
    padding: 2rem;
    border-bottom: 1px solid var(--color-border);
}
h1 {
    margin: 0;
    font-size: 2rem;
}
.summary {
    display: flex;
    justify-content: space-around;
    padding: 1.5rem;
    background-color: var(--color-bg);
    border-bottom: 1px solid var(--color-border);
}
.summary-item {
    text-align: center;
}
.summary-item .count {
    font-size: 2.5rem;
    font-weight: bold;
    display: block;
}
.summary-item .label {
    font-size: 1rem;
    color: #6c757d;
}
#results-table {
    width: 100%;
    border-collapse: collapse;
}
#results-table th, #results-table td {
    padding: 1rem 1.5rem;
    text-align: left;
    border-bottom: 1px solid var(--color-border);
}
#results-table th {
    background-color: var(--color-bg);
    font-weight: 600;
}
.status-badge {
    padding: 0.3em 0.6em;
    border-radius: 100px;
    font-weight: 600;
    font-size: 0.8rem;
    white-space: nowrap;
}
.status-Passed {
    color: var(--color-passed);
    background-color: var(--color-passed-bg);
}
.status-Failed {
    color: var(--color-failed);
    background-color: var(--color-failed-bg);
}
.status-Allowed-Failure {
    color: var(--color-allowed-failure);
    background-color: var(--color-allowed-failure-bg);
}
.status-Skipped {
    color: var(--color-skipped);
    background-color: var(--color-skipped-bg);
}
.output-toggle {
    cursor: pointer;
    color: #007bff;
    font-weight: 500;
}
.output-content {
    display: none;
    margin-top: 1rem;
    padding: 1rem;
    background-color: #212529;
    color: #f8f9fa;
    border-radius: 4px;
    font-family: var(--font-family-monospace);
    white-space: pre-wrap;
    word-break: break-all;
}
"#;

/// JavaScript code for the HTML test report.
/// This constant contains the client-side JavaScript functionality for the HTML report,
/// primarily for toggling the visibility of test output sections.
///
/// HTML 测试报告的 JavaScript 代码。
/// 此常量包含 HTML 报告的客户端 JavaScript 功能，
/// 主要用于切换测试输出部分的可见性。
///
/// # Functions / 函数
/// - `toggleOutput(id)`: Toggles the display state of an element with the given ID
///                      切换具有给定 ID 的元素的显示状态
const HTML_SCRIPT: &str = r#"
function toggleOutput(id) {
    const el = document.getElementById(id);
    if (el.style.display === 'block') {
        el.style.display = 'none';
    } else {
        el.style.display = 'block';
    }
}
"#;

/// Generates an interactive HTML report from test results and saves it to the specified path.
/// The report includes a summary of test statistics, a detailed table of all test results,
/// and interactive features for viewing test output.
///
/// # Arguments / 参数
/// * `results` - A slice of `TestResult` instances containing all test execution results
///              包含所有测试执行结果的 `TestResult` 实例切片
/// * `output_path` - The file path where the HTML report should be saved
///                  应保存 HTML 报告的文件路径
///
/// # Returns / 返回值
/// * `Result<()>` - Success if the report was generated and saved successfully, or an error
///                 如果报告成功生成并保存则返回成功，否则返回错误
///
/// # Features / 功能特性
/// - Categorizes test results into passed, failed, allowed failures, and skipped
/// - Generates a responsive HTML layout with CSS styling
/// - Includes interactive JavaScript for toggling test output visibility
/// - Displays timestamp of report generation
///
/// - 将测试结果分类为通过、失败、允许失败和跳过
/// - 生成带有 CSS 样式的响应式 HTML 布局
/// - 包含用于切换测试输出可见性的交互式 JavaScript
/// - 显示报告生成的时间戳
///
/// 从测试结果生成交互式 HTML 报告并将其保存到指定路径。
/// 报告包括测试统计摘要、所有测试结果的详细表格以及查看测试输出的交互功能。
pub fn generate_html_report(results: &[TestResult], output_path: &Path) -> Result<()> {
    // Calculate statistics for different test result categories
    // 计算不同测试结果类别的统计信息
    let passed_count = results
        .iter()
        .filter(|r| matches!(r, TestResult::Passed { .. }))
        .count();
    let failed_count = results.iter().filter(|r| r.is_unexpected_failure()).count();
    let allowed_failures_count = results
        .iter()
        .filter(|r| matches!(r, TestResult::Failed { .. }) && !r.is_unexpected_failure())
        .count();
    let skipped_count = results
        .iter()
        .filter(|r| matches!(r, TestResult::Skipped))
        .count();
    let total_count = results.len();

    // Generate timestamp for the report
    // 为报告生成时间戳
    let now = Local::now();
    let report_date = now.format("%Y-%m-%d %H:%M:%S").to_string();

    let markup = html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { (i18n::t(I18nKey::HtmlReportTitle)) }
                style { (PreEscaped(HTML_STYLE)) }
            }
            body {
                div class="container" {
                    header {
                        h1 { (i18n::t(I18nKey::HtmlReportTitle)) }
                        p { (i18n::t_fmt(I18nKey::HtmlReportGeneratedOn, &[&report_date])) }
                    }

                    div class="summary" {
                        div class="summary-item" {
                            span class="count" style=(format!("color: {};", "var(--color-passed)")) { (passed_count) }
                            span class="label" { (i18n::t(I18nKey::HtmlReportPassed)) }
                        }
                        div class="summary-item" {
                            span class="count" style=(format!("color: {};", "var(--color-failed)")) { (failed_count) }
                            span class="label" { (i18n::t(I18nKey::HtmlReportFailed)) }
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
                            span class="count" { (total_count) }
                            span class="label" { (i18n::t(I18nKey::HtmlReportTotal)) }
                        }
                    }

                    table id="results-table" {
                        thead {
                            tr {
                                th { (i18n::t(I18nKey::HtmlReportNameColumn)) }
                                th { (i18n::t(I18nKey::HtmlReportStatusColumn)) }
                                th { (i18n::t(I18nKey::HtmlReportOutputColumn)) }
                            }
                        }
                        tbody {
                            @for (i, result) in results.iter().enumerate() {
                                tr {
                                    td { (result.get_name()) }
                                    td {
                                        span class=(format!("status-badge status-{}", result.get_status_str().replace(' ', "-"))) {
                                            (result.get_status_str())
                                        }
                                    }
                                    td {
                                        @if !result.get_output().is_empty() {
                                            a class="output-toggle" onclick=(format!("toggleOutput('output-{}')", i)) { (i18n::t(I18nKey::HtmlReportShowHide)) }
                                        }
                                    }
                                }
                                @if !result.get_output().is_empty() {
                                    tr class="output-row" {
                                        td colspan="3" {
                                            pre class="output-content" id=(format!("output-{}", i)) { (result.get_output()) }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                script { (PreEscaped(HTML_SCRIPT)) }
            }
        }
    };

    // Write the generated HTML to the specified file
    // 将生成的 HTML 写入指定文件
    fs::write(output_path, markup.into_string())
        .with_context(|| format!("Failed to write HTML report to {}", output_path.display()))?;

    Ok(())
}

/// Prints a comprehensive summary of test results to the console and returns unexpected failures.
/// This function categorizes all test results and displays them in a formatted, colored output.
/// It also determines the overall test run status and returns any unexpected failures for further processing.
///
/// # Arguments / 参数
/// * `results` - A slice of `TestResult` instances containing all test execution results
///              包含所有测试执行结果的 `TestResult` 实例切片
///
/// # Returns / 返回值
/// * `Vec<&TestResult>` - A vector of references to unexpected failure results
///                       指向意外失败结果的引用向量
///
/// # Behavior / 行为
/// - Categorizes results into successes, allowed failures, unexpected failures, and skipped tests
/// - Displays each category with appropriate colored formatting
/// - Shows the overall test run status (success or failure)
/// - Temporarily disables colored output for subsequent report generation
/// - Returns unexpected failures for detailed reporting
///
/// - 将结果分类为成功、允许失败、意外失败和跳过的测试
/// - 使用适当的彩色格式显示每个类别
/// - 显示整体测试运行状态（成功或失败）
/// - 临时禁用彩色输出以便后续报告生成
/// - 返回意外失败以进行详细报告
///
/// 打印测试结果的综合摘要到控制台并返回意外失败。
/// 此函数对所有测试结果进行分类，并以格式化的彩色输出显示它们。
/// 它还确定整体测试运行状态并返回任何意外失败以供进一步处理。
pub fn print_summary(results: &[TestResult]) -> Vec<&TestResult> {
    // Initialize vectors to categorize test results
    // 初始化向量以对测试结果进行分类
    let mut successes = Vec::new();
    let mut allowed_failures = Vec::new();
    let mut unexpected_failures = Vec::new();
    let mut skipped_tests = Vec::new();

    let current_os = std::env::consts::OS;

    // Categorize each test result based on its type and configuration
    // 根据类型和配置对每个测试结果进行分类
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

    // Display the summary banner
    // 显示摘要横幅
    println!("\n{}", i18n::t(I18nKey::FinalSummaryBanner).cyan());

    // Display successful tests if any
    // 如果有成功的测试则显示
    if !successes.is_empty() {
        println!("\n{}", i18n::t(I18nKey::SummarySuccessfulTests).green());
        for result in successes {
            if let TestResult::Passed { case, .. } = result {
                println!("  - {}", case.name.green());
            }
        }
    }

    // Display allowed failures if any
    // 如果有允许的失败则显示
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

    // Display skipped tests if any
    // 如果有跳过的测试则显示
    if !skipped_tests.is_empty() {
        println!("\n{}", i18n::t(I18nKey::SummarySkippedTests).yellow());
        println!(
            "  - {}",
            i18n::t_fmt(I18nKey::SummarySkippedCount, &[&skipped_tests.len()])
        );
    }

    // Display unexpected failures if any
    // 如果有意外失败则显示
    if !unexpected_failures.is_empty() {
        println!(
            "\n{}",
            i18n::t(I18nKey::SummaryUnexpectedFailures).red().bold()
        );
        for result in &unexpected_failures {
            if let TestResult::Failed { case, reason, .. } = result {
                let failure_type = match reason {
                    FailureReason::Build => i18n::t(I18nKey::BuildFailure),
                    FailureReason::Test => i18n::t(I18nKey::TestFailure),
                };
                println!("  - {}", format!("{} ({})", case.name.red(), failure_type));
            }
        }
    }

    println!();

    // Display overall test run status
    // 显示整体测试运行状态
    if !unexpected_failures.is_empty() {
        println!("{}", i18n::t(I18nKey::OverallFailure).red().bold());
    } else {
        println!("{}", i18n::t(I18nKey::OverallSuccess).green().bold());
    }

    // Temporarily disable colors before generating the report to test a hypothesis
    // 在生成报告之前临时禁用颜色以测试假设
    colored::control::set_override(false);

    unexpected_failures
}

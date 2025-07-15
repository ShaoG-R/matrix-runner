//! # HTML Reporting Module / HTML 报告模块
//!
//! This module handles the generation of HTML test reports.
//! It creates styled HTML files with test statistics, detailed results tables,
//! and interactive features for viewing test output.
//!
//! 此模块处理 HTML 测试报告的生成。
//! 它创建带有测试统计、详细结果表格和查看测试输出的交互功能的样式化 HTML 文件。

use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::core::models::{self, TestResult};
use crate::reporting::console::get_error_output_from_result;

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
    output_path: &Path,
) -> Result<()> {
    let mut html = String::new();
    html.push_str("<!DOCTYPE html><html><head><title>Test Report</title>");
    html.push_str("<style>");
    html.push_str(HTML_STYLE);
    html.push_str("</style>");
    html.push_str("</head><body><h1>Test Matrix Report</h1>");
    
    // Add summary statistics
    let total = results.len();
    let passed = results.iter().filter(|r| matches!(r, TestResult::Passed { .. })).count();
    let failed = results.iter().filter(|r| matches!(r, TestResult::Failed { .. })).count();
    let skipped = results.iter().filter(|r| matches!(r, TestResult::Skipped)).count();
    
    html.push_str("<div class='summary'>");
    html.push_str(&format!("<div class='stat'>Total: <span>{}</span></div>", total));
    html.push_str(&format!("<div class='stat passed'>Passed: <span>{}</span></div>", passed));
    html.push_str(&format!("<div class='stat failed'>Failed: <span>{}</span></div>", failed));
    html.push_str(&format!("<div class='stat skipped'>Skipped: <span>{}</span></div>", skipped));
    html.push_str("</div>");

    // Add results table
    html.push_str("<table><thead><tr><th>#</th><th>Name</th><th>Result</th><th>Time (s)</th></tr></thead><tbody>");

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
                ..
            } => {
                let reason_text = match reason {
                    models::FailureReason::Build | models::FailureReason::BuildFailed => "Build",
                    models::FailureReason::TestFailed => "Test",
                    models::FailureReason::Timeout => "Timeout",
                    models::FailureReason::CustomCommand => "Command",
                };
                let error_output = get_error_output_from_result(result, "en"); // Assuming English for now
                let escaped_output = escape_html(&error_output);
                (
                    "failed",
                    "Failed",
                    format!("{:.2}", duration.as_secs_f64()),
                    format!(
                        "<div class='error-details'><strong>Reason: {}</strong><pre>{}</pre></div>",
                        reason_text, escaped_output
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

    html.push_str("</tbody></table>");
    html.push_str("<script>");
    html.push_str(HTML_SCRIPT);
    html.push_str("</script>");
    html.push_str("</body></html>");

    fs::write(output_path, html)?;
    Ok(())
}

/// Simple HTML escape function to replace special characters with their HTML entities
/// 简单的 HTML 转义函数，用 HTML 实体替换特殊字符
fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
} 
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

use crate::core::models::TestResult;
use crate::infra::t;
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
/// * `locale` - The locale to use for internationalization
///              用于国际化使用的语言环境
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
    locale: &str,
) -> Result<()> {
    let mut html = String::new();
    html.push_str(&format!(
        "<!DOCTYPE html><html><head><title>{}</title>",
        t!("html_report.title", locale = locale)
    ));
    html.push_str("<style>");
    html.push_str(HTML_STYLE);
    html.push_str("</style>");
    html.push_str("</head><body>");
    html.push_str(&format!(
        "<h1>{}</h1>",
        t!("html_report.main_header", locale = locale)
    ));
    
    // Add summary statistics
    let total = results.len();
    let passed = results
        .iter()
        .filter(|r| matches!(r, TestResult::Passed { .. }))
        .count();
    let failed = results
        .iter()
        .filter(|r| r.is_failure())
        .count();
    let skipped = results
        .iter()
        .filter(|r| matches!(r, TestResult::Skipped))
        .count();

    html.push_str("<div class='summary-container'>");
    html.push_str(&format!(
        "<div class='summary-item'><span class='count'>{}</span><span class='label'>{}</span></div>",
        total,
        t!("html_report.summary.total", locale = locale)
    ));
    html.push_str(&format!(
        "<div class='summary-item'><span class='count passed-text'>{}</span><span class='label'>{}</span></div>",
        passed,
        t!("html_report.summary.passed", locale = locale)
    ));
    html.push_str(&format!(
        "<div class='summary-item'><span class='count failed-text'>{}</span><span class='label'>{}</span></div>",
        failed,
        t!("html_report.summary.failed", locale = locale)
    ));
    html.push_str(&format!(
        "<div class='summary-item'><span class='count skipped-text'>{}</span><span class='label'>{}</span></div>",
        skipped,
        t!("html_report.summary.skipped", locale = locale)
    ));
    html.push_str("</div>");


    // Add results table
    html.push_str("<table><thead><tr>");
    html.push_str(&format!(
        "<th>{}</th>",
        t!("html_report.table.header.name", locale = locale)
    ));
    html.push_str(&format!(
        "<th class='status-col'>{}</th>",
        t!("html_report.table.header.status", locale = locale)
    ));
    html.push_str(&format!(
        "<th class='duration-cell'>{}</th>",
        t!("html_report.table.header.duration", locale = locale)
    ));
    html.push_str(&format!(
        "<th class='retries-cell'>{}</th>",
        t!("html_report.table.header.retries", locale = locale)
    ));
    html.push_str("</tr></thead><tbody>");


    for (i, result) in results.iter().enumerate() {
        let status_str = result.get_status_str(locale);
        let status_class = result.get_status_class();
        let duration_str = result
            .get_duration()
            .map(|d| format!("{:.2}s", d.as_secs_f64()))
            .unwrap_or_else(|| "N/A".to_string());
        
        let retries_str = {
            let retries = result.get_retries();
            if retries > 1 {
                format!("{}", retries - 1)
            } else {
                String::new()
            }
        };

        let output_id = format!("output-{}", i);
        let error_details = if let TestResult::Failed { .. } = result {
            let error_output = get_error_output_from_result(result, locale);
            let escaped_output = escape_html(&error_output);
            format!(
                "<tr id='{}' style='display:none;'><td colspan='4'><pre class='output-content'>{}</pre></td></tr>",
                output_id,
                escaped_output
            )
        } else {
            String::new()
        };

        let output_toggle = if result.is_failure() {
            format!("<div class='output-toggle' onclick=\"toggleOutput('{}')\">{}</div>", output_id, t!("html_report.toggle_output", locale=locale))
        } else {
            String::new()
        };
        
        html.push_str("<tr>");
        html.push_str(&format!("<td>{}</td>", result.case_name()));
        html.push_str(&format!(
            "<td class='status-col'><div class='status-cell {}'>{}</div>{}</td>",
            status_class, status_str, output_toggle
        ));
        html.push_str(&format!(
            "<td class='duration-cell'>{}</td>",
            duration_str
        ));
        html.push_str(&format!("<td class='retries-cell'>{}</td>", retries_str));
        html.push_str("</tr>");
        html.push_str(&error_details);
    }

    html.push_str("</tbody></table>");
    html.push_str("<script>");
    html.push_str(HTML_SCRIPT);
    html.push_str("</script></body></html>");

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
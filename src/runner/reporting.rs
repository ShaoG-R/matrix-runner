//! This module handles the generation and display of test reports.
//! It includes functions for creating styled HTML reports and for printing
//! colorful, formatted summaries to the console.

use crate::runner::command::format_build_error_output;
use crate::runner::i18n::{self, I18nKey};
use crate::runner::models::{FailureReason, TestResult};
use anyhow::{Context, Result};
use colored::*;
use maud::{html, PreEscaped, DOCTYPE};
use std::fs;
use std::path::Path;

const HTML_STYLE: &str = r#"
:root {
    --color-passed: #28a745;
    --color-passed-bg: #e9f5ec;
    --color-failed: #dc3545;
    --color-failed-bg: #f8d7da;
    --color-allowed-failure: #ffc107;
    --color-allowed-failure-bg: #fff8e1;
    --color-skipped: #6c757d;
    --color-skipped-bg: #f1f3f5;
    --color-timeout: #fd7e14;
    --color-timeout-bg: #fff3e0;
    --font-family-sans-serif: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
    --font-family-monospace: SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
    --table-border-color: #dee2e6;
}
body {
    font-family: var(--font-family-sans-serif);
    line-height: 1.6;
    color: #333;
    margin: 0;
    padding: 20px;
    background-color: #fdfdff;
}
.container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 20px;
    background: #fff;
    border-radius: 8px;
    box-shadow: 0 2px 10px rgba(0,0,0,0.05);
}
h1, h2 {
    color: #333;
    border-bottom: 2px solid #eee;
    padding-bottom: 10px;
    margin-top: 0;
}
.summary-container {
    display: flex;
    flex-wrap: wrap;
    gap: 15px;
    margin-bottom: 20px;
}
.summary-item {
    background-color: #f8f9fa;
    border-radius: 6px;
    padding: 15px;
    flex-grow: 1;
    text-align: center;
    border: 1px solid var(--table-border-color);
}
.summary-item .count {
    display: block;
    font-size: 2em;
    font-weight: 700;
}
.summary-item .label {
    font-size: 0.9em;
    color: #555;
}
.status-cell {
    font-weight: 600;
    text-align: center;
    border-radius: 4px;
    padding: 5px 10px;
    display: inline-block;
    min-width: 80px;
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
.status-Timeout {
    color: var(--color-timeout);
    background-color: var(--color-timeout-bg);
}
.output-toggle {
    cursor: pointer;
    color: #007bff;
    text-decoration: underline;
    font-size: 0.9em;
}
.output-toggle:hover {
    color: #0056b3;
}
table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 20px;
}
th, td {
    border: 1px solid var(--table-border-color);
    padding: 10px;
    text-align: left;
    vertical-align: middle;
}
th {
    background-color: #f8f9fa;
    font-weight: 600;
}
pre.output-content {
    background: #222;
    color: #eee;
    padding: 15px;
    border-radius: 5px;
    white-space: pre-wrap;
    word-wrap: break-word;
    font-family: var(--font-family-monospace);
    margin-top: 10px;
}
.duration-cell, .retries-cell {
    text-align: right;
    width: 80px;
}
.status-col {
    width: 150px;
    text-align: center;
}
"#;

const HTML_SCRIPT: &str = r#"
function toggleOutput(id) {
    const element = document.getElementById(id);
    if (element.style.display === 'none') {
        element.style.display = 'block';
    } else {
        element.style.display = 'none';
    }
}
"#;

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
        .filter(|r| matches!(r, TestResult::Failed { reason: FailureReason::Timeout, .. }))
        .count();
    let total_unexpected_failures = results.iter().filter(|r| r.is_unexpected_failure()).count();
    let failed_count = total_unexpected_failures - timeout_count;
    let total_count = results.len();
    let total_duration: std::time::Duration =
        results.iter().filter_map(|r| r.get_duration()).sum();

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

    fs::write(output_path, markup).with_context(|| {
        i18n::t_fmt(
            I18nKey::HtmlReportWriteFailed,
            &[&output_path.display()],
        )
    })
}

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
            };
            println!("{output_to_print}");
        }
    }

    println!(
        "{}",
        "-----------------------------------------------------------------".cyan()
    );
}

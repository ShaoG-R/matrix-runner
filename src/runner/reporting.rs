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

const HTML_STYLE: &str = include_str!("assets/report.css");
const HTML_SCRIPT: &str = include_str!("assets/report.js");

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

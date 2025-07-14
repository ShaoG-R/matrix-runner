use crate::runner::command::format_build_error_output;
use crate::runner::i18n;
use crate::runner::i18n::I18nKey;
use crate::runner::models::{FailureReason, TestResult};
use anyhow::{Context, Result};
use chrono::Local;
use colored::*;
use maud::{html, PreEscaped};
use std::fs;
use std::path::Path;

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

pub fn generate_html_report(results: &[TestResult], output_path: &Path) -> Result<()> {
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

    let now = Local::now();
    let report_date = now.format("%Y-%m-%d %H:%M:%S").to_string();

    let markup = html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { "Test Report" }
                style { (PreEscaped(HTML_STYLE)) }
            }
            body {
                div class="container" {
                    header {
                        h1 { "Test Matrix Report" }
                        p { "Generated on: " (report_date) }
                    }

                    div class="summary" {
                        div class="summary-item" {
                            span class="count" style=(format!("color: {};", "var(--color-passed)")) { (passed_count) }
                            span class="label" { "Passed" }
                        }
                        div class="summary-item" {
                            span class="count" style=(format!("color: {};", "var(--color-failed)")) { (failed_count) }
                            span class="label" { "Failed" }
                        }
                        div class="summary-item" {
                            span class="count" style=(format!("color: {};", "var(--color-allowed-failure)")) { (allowed_failures_count) }
                            span class="label" { "Allowed Failures" }
                        }
                        div class="summary-item" {
                            span class="count" style=(format!("color: {};", "var(--color-skipped)")) { (skipped_count) }
                            span class="label" { "Skipped" }
                        }
                        div class="summary-item" {
                            span class="count" { (total_count) }
                            span class="label" { "Total" }
                        }
                    }

                    table id="results-table" {
                        thead {
                            tr {
                                th { "Name" }
                                th { "Status" }
                                th { "Output" }
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
                                            a class="output-toggle" onclick=(format!("toggleOutput('output-{}')", i)) { "Show/Hide" }
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

    fs::write(output_path, markup.into_string())
        .with_context(|| format!("Failed to write HTML report to {}", output_path.display()))?;

    Ok(())
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
    
    // Temporarily disable colors before generating the report to test a hypothesis
    colored::control::set_override(false);
    
    unexpected_failures
}

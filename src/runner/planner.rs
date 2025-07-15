// src/runner/planner.rs
use crate::runner::config::{TestCase, TestMatrix};
use anyhow::Result;
use std::env;

#[derive(Debug)]
pub struct ExecutionPlan {
    pub cases_to_run: Vec<TestCase>,
    pub filtered_arch_count: usize,
    pub flaky_cases_count: usize,
    pub is_distributed: bool,
}

pub fn plan_execution(
    test_matrix: TestMatrix,
    total_runners: Option<usize>,
    runner_index: Option<usize>,
) -> Result<ExecutionPlan> {
    let cases = test_matrix.cases;

    // Filter by architecture
    let current_arch = env::consts::ARCH;
    let (arch_cases, filtered_arch_cases): (Vec<_>, Vec<_>) = cases
        .into_iter()
        .partition(|case| case.arch.is_empty() || case.arch.iter().any(|a| a == current_arch));

    // Separate flaky cases
    let current_os = env::consts::OS;
    let (mut safe_cases, flaky_cases): (Vec<_>, Vec<_>) = arch_cases
        .into_iter()
        .partition(|case| !case.allow_failure.iter().any(|os| os == current_os));

    safe_cases.sort_by(|a, b| a.name.cmp(&b.name));

    let mut combined_cases = safe_cases;
    combined_cases.extend(flaky_cases.clone());

    // Distribute cases if running in CI
    let (cases_to_run, is_distributed) =
        if let (Some(total), Some(index)) = (total_runners, runner_index) {
            if index >= total {
                anyhow::bail!("Runner index must be less than total runners.");
            }
            let distributed_cases: Vec<_> = combined_cases
                .into_iter()
                .enumerate()
                .filter(|(i, _)| i % total == index)
                .map(|(_, case)| case)
                .collect();
            (distributed_cases, true)
        } else {
            if total_runners.is_some() || runner_index.is_some() {
                anyhow::bail!("Both --total-runners and --runner-index must be provided.");
            }
            (combined_cases, false)
        };

    Ok(ExecutionPlan {
        cases_to_run,
        filtered_arch_count: filtered_arch_cases.len(),
        flaky_cases_count: flaky_cases.len(),
        is_distributed,
    })
} 
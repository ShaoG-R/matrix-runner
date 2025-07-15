//! # Test Execution Planner Module / 测试执行计划模块
//!
//! This module handles planning and organizing the execution of test cases,
//! including filtering by architecture, handling distributed execution,
//! and prioritizing test cases.
//!
//! 此模块处理测试用例的执行计划和组织，
//! 包括按架构过滤、处理分布式执行和优先排序测试用例。

use crate::core::config::TestCase;
use anyhow::{bail, Result};
use std::env;

/// Represents a complete execution plan for a test matrix.
/// 表示测试矩阵的完整执行计划。
#[derive(Debug)]
pub struct ExecutionPlan {
    /// The list of test cases to be executed, filtered by architecture and possibly distributed.
    /// 要执行的测试用例列表，按架构过滤并可能分布式执行。
    pub cases_to_run: Vec<TestCase>,
    /// The number of cases filtered out due to architecture constraints.
    /// 由于架构约束而被过滤掉的用例数量。
    pub filtered_arch_count: usize,
    /// The number of cases that are allowed to fail on the current platform.
    /// 在当前平台上允许失败的用例数量。
    pub flaky_cases_count: usize,
    /// Whether the cases are distributed across multiple runners (CI environment).
    /// 用例是否分布在多个运行器上（CI 环境）。
    pub is_distributed: bool,
}

/// Creates an execution plan for the given test matrix.
/// This involves filtering test cases by architecture, separating flaky cases,
/// and potentially distributing cases across multiple runners.
///
/// 为给定的测试矩阵创建执行计划。
/// 这涉及按架构过滤测试用例、分离不稳定的用例，
/// 并可能在多个运行器之间分配用例。
///
/// # Arguments
/// * `test_matrix` - The complete test matrix configuration
/// * `total_runners` - Optional total number of runners for distributed execution
/// * `runner_index` - Optional index of this runner (0-based)
///
/// # Returns
/// An `ExecutionPlan` with the filtered and potentially distributed test cases
pub fn plan_execution(
    test_matrix: crate::core::config::TestMatrix,
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

    // Sort cases by name for deterministic execution order
    safe_cases.sort_by(|a, b| a.name.cmp(&b.name));

    let mut combined_cases = safe_cases;
    combined_cases.extend(flaky_cases.clone());

    // Distribute cases if running in CI
    let (cases_to_run, is_distributed) =
        if let (Some(total), Some(index)) = (total_runners, runner_index) {
            if index >= total {
                bail!("Runner index must be less than total runners.");
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
                bail!("Both --total-runners and --runner-index must be provided.");
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
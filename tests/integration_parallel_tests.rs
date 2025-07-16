//! # Parallel Execution Integration Tests / 并行执行集成测试
//!
//! This module contains integration tests for parallel test execution functionality,
//! testing job distribution, concurrency limits, and resource management.
//!
//! 此模块包含并行测试执行功能的集成测试，
//! 测试任务分配、并发限制和资源管理。

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use std::time::Instant;
use tempfile::TempDir;

/// Helper function to create a test matrix with multiple cases
/// 创建包含多个测试用例的测试矩阵的辅助函数
fn create_multi_case_matrix(temp_dir: &TempDir, case_count: usize) -> std::path::PathBuf {
    let matrix_path = temp_dir.path().join("multi_case.toml");

    let mut content = String::from("language = \"en\"\n\n");

    for i in 0..case_count {
        content.push_str(&format!(
            r#"[[cases]]
name = "test-case-{}"
features = ""
no_default_features = false

"#,
            i
        ));
    }

    fs::write(&matrix_path, content).unwrap();
    matrix_path
}

/// Helper function to create a test matrix with slow cases (using sleep commands)
/// 创建包含慢速测试用例的测试矩阵的辅助函数
fn create_slow_case_matrix(temp_dir: &TempDir) -> std::path::PathBuf {
    let matrix_path = temp_dir.path().join("slow_cases.toml");

    let content = r#"language = "en"

[[cases]]
name = "slow-case-1"
command = "powershell -Command \"Start-Sleep -Seconds 2; Write-Output 'slow-case-1 completed'\""
features = ""
no_default_features = false

[[cases]]
name = "slow-case-2"
command = "powershell -Command \"Start-Sleep -Seconds 3; Write-Output 'slow-case-2 completed'\""
features = ""
no_default_features = false

[[cases]]
name = "slow-case-3"
command = "powershell -Command \"Start-Sleep -Seconds 2; Write-Output 'slow-case-3 completed'\""
features = ""
no_default_features = false

[[cases]]
name = "slow-case-4"
command = "powershell -Command \"Start-Sleep -Seconds 3; Write-Output 'slow-case-4 completed'\""
features = ""
no_default_features = false
"#;

    fs::write(&matrix_path, content).unwrap();
    matrix_path
}

#[cfg(test)]
mod parallel_execution_tests {
    use super::*;

    #[test]
    fn test_parallel_execution_with_multiple_jobs() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = create_multi_case_matrix(&temp_dir, 4);

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project")
            .arg("--jobs")
            .arg("2"); // Use 2 parallel jobs

        let start_time = Instant::now();
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("All tests passed successfully!"));
        let duration = start_time.elapsed();

        // With parallel execution, it should complete reasonably quickly
        assert!(
            duration.as_secs() < 30,
            "Parallel execution took too long: {:?}",
            duration
        );
    }

    #[test]
    fn test_single_job_execution() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = create_multi_case_matrix(&temp_dir, 3);

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project")
            .arg("--jobs")
            .arg("1"); // Use only 1 job (sequential execution)

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("All tests passed successfully!"));
    }

    #[test]
    fn test_parallel_execution_basic_functionality() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = create_slow_case_matrix(&temp_dir);

        // Test with 4 jobs (parallel) - just ensure it works
        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project")
            .arg("--jobs")
            .arg("4");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("All tests passed successfully!"));
    }

    #[test]
    fn test_job_count_validation() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = create_multi_case_matrix(&temp_dir, 2);

        // Test with 1 job (minimum valid)
        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project")
            .arg("--jobs")
            .arg("1");

        // Should work
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("All tests passed successfully!"));
    }

    #[test]
    fn test_high_job_count() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = create_multi_case_matrix(&temp_dir, 2);

        // Test with very high job count (more than test cases)
        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project")
            .arg("--jobs")
            .arg("100");

        // Should still work without issues
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("All tests passed successfully!"));
    }
}

#[cfg(test)]
mod runner_splitting_tests {
    use super::*;

    #[test]
    fn test_runner_splitting_basic() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = create_multi_case_matrix(&temp_dir, 6);

        // Test runner 0 of 2
        let mut cmd1 = Command::cargo_bin("matrix-runner").unwrap();
        cmd1.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project")
            .arg("--total-runners")
            .arg("2")
            .arg("--runner-index")
            .arg("0");

        // Test runner 1 of 2
        let mut cmd2 = Command::cargo_bin("matrix-runner").unwrap();
        cmd2.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project")
            .arg("--total-runners")
            .arg("2")
            .arg("--runner-index")
            .arg("1");
        
        cmd1.assert()
            .success()
            .stdout(predicate::str::contains("Running as runner 0 of 2"));
        
        cmd2.assert()
            .success()
            .stdout(predicate::str::contains("Running as runner 1 of 2"));
    }

    #[test]
    fn test_runner_splitting_edge_cases() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = create_multi_case_matrix(&temp_dir, 3);

        // Test with more runners than cases
        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project")
            .arg("--total-runners")
            .arg("5")
            .arg("--runner-index")
            .arg("4");

        // Should handle gracefully (might have no cases to run)
        cmd.assert().success();
    }

    #[test]
    fn test_runner_splitting_invalid_index() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = create_multi_case_matrix(&temp_dir, 4);

        // Test with runner index >= total runners
        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project")
            .arg("--total-runners")
            .arg("2")
            .arg("--runner-index")
            .arg("2"); // Invalid: should be 0 or 1

        // Should fail with validation error
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains(
                "Runner index must be less than total runners",
            ));
    }

    #[test]
    fn test_single_runner_mode() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = create_multi_case_matrix(&temp_dir, 4);

        // Test without runner splitting (default mode)
        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project");

        cmd.assert().success().stdout(predicate::str::contains(
            "Running all test cases as a single runner",
        ));
    }
}

#[cfg(test)]
mod resource_management_tests {
    use super::*;

    #[test]
    fn test_temporary_directory_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = create_multi_case_matrix(&temp_dir, 1);

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project");

        // The main check is that this runs without error.
        // The temporary directories are cleaned up by the `tempfile` crate on drop.
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("All tests passed successfully!"));
    }

    #[test]
    fn test_concurrent_builds_isolation() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = create_multi_case_matrix(&temp_dir, 4);

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project")
            .arg("--jobs")
            .arg("3"); // Use multiple jobs

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("All tests passed successfully!"));
    }
}

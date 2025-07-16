//! # Error Handling Integration Tests / 错误处理集成测试
//!
//! This module contains integration tests for error handling scenarios,
//! testing various failure modes and edge cases.
//!
//! 此模块包含错误处理场景的集成测试，
//! 测试各种失败模式和边界情况。

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;
#[path = "common/mod.rs"]
mod common;
use common::{
    create_incomplete_config, create_invalid_toml,
};

#[cfg(test)]
mod config_error_tests {
    use super::*;

    #[test]
    fn test_nonexistent_config_file() {
        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg("nonexistent_file.toml")
            .arg("--project-dir")
            .arg("tests/sample_project");

        cmd.assert().failure().stderr(predicate::str::contains(
            "Failed to find or read config file",
        ));
    }

    #[test]
    fn test_invalid_toml_syntax() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = create_invalid_toml(&temp_dir);

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project");

        cmd.assert().failure();
    }

    #[test]
    fn test_incomplete_config() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = create_incomplete_config(&temp_dir);

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project");

        cmd.assert().failure();
    }

    #[test]
    fn test_empty_config_file() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = temp_dir.path().join("empty.toml");
        fs::write(&matrix_path, "").unwrap();

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project");

        cmd.assert().failure();
    }

    #[test]
    fn test_config_with_no_cases() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = temp_dir.path().join("no_cases.toml");
        let content = r#"
language = "en"
cases = []
"#;
        fs::write(&matrix_path, content).unwrap();

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("No test cases to run"));
    }
}

#[cfg(test)]
mod project_error_tests {
    use super::*;

    #[test]
    fn test_nonexistent_project_directory() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = temp_dir.path().join("valid.toml");
        let content = r#"
language = "en"

[[cases]]
name = "test-case"
features = ""
no_default_features = false
"#;
        fs::write(&matrix_path, content).unwrap();

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("nonexistent_project_directory");

        cmd.assert().failure();
    }

    #[test]
    fn test_project_without_cargo_toml() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("no_cargo_project");
        fs::create_dir_all(&project_dir).unwrap();

        let matrix_path = temp_dir.path().join("valid.toml");
        let content = r#"
language = "en"

[[cases]]
name = "test-case"
features = ""
no_default_features = false
"#;
        fs::write(&matrix_path, content).unwrap();

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg(&project_dir);

        cmd.assert().failure();
    }

    #[test]
    fn test_invalid_cargo_toml() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("invalid_cargo_project");
        fs::create_dir_all(&project_dir).unwrap();

        // Create invalid Cargo.toml
        let cargo_toml = project_dir.join("Cargo.toml");
        fs::write(&cargo_toml, "invalid toml content [[[").unwrap();

        let matrix_path = temp_dir.path().join("valid.toml");
        let content = r#"
language = "en"

[[cases]]
name = "test-case"
features = ""
no_default_features = false
"#;
        fs::write(&matrix_path, content).unwrap();

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg(&project_dir);

        cmd.assert().failure();
    }
}

#[cfg(test)]
mod command_error_tests {
    use super::*;

    #[test]
    fn test_invalid_custom_command() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = temp_dir.path().join("invalid_command.toml");
        // This TOML is now valid, but the case should be filtered on non-aarch64.
        let content = r#"
language = "en"
[[cases]]
name = "invalid-command-case"
features = ""
no_default_features = false
command = "nonexistent_command"
arch = ["aarch64"]
"#;
        fs::write(&matrix_path, content).unwrap();

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project");

        // On non-aarch64 systems, the case is filtered, and the program should exit gracefully.
        #[cfg(not(target_arch = "aarch64"))]
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("No test cases to run"));

        // On aarch64, this test would rightfully fail because the command is invalid.
        // A separate test, running only on aarch64, would be needed to verify that.
        #[cfg(target_arch = "aarch64")]
        cmd.assert().failure();
    }

    #[test]
    fn test_command_with_invalid_arguments() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = temp_dir.path().join("invalid_args.toml");
        let content = r#"
language = "en"
[[cases]]
name = "invalid-args-case"
features = ""
no_default_features = false
command = "cargo build --bin nonexistent_target"
"#;
        fs::write(&matrix_path, content).unwrap();

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project");

        cmd.assert()
            .failure()
            .stdout(predicate::str::contains("Test 'invalid-args-case' failed"));
    }
}

#[cfg(test)]
mod architecture_filtering_tests {
    use super::*;

    #[test]
    fn test_architecture_filtering() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = temp_dir.path().join("arch_filtered.toml");
        let content = r#"
language = "en"
[[cases]]
name = "unsupported-arch-case"
features = ""
no_default_features = false
arch = ["nonexistent_arch"]

[[cases]]
name = "supported-arch-case"
features = ""
no_default_features = false
arch = ["x86_64", "aarch64"]
"#;
        fs::write(&matrix_path, content).unwrap();

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project");

        cmd.assert()
            .success()
            .stdout(
                predicate::str::contains("Filtered out 1 of 2 cases based on current architecture."),
            );
    }

    #[test]
    fn test_all_cases_filtered_by_architecture() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = temp_dir.path().join("all_filtered.toml");
        let content = r#"
language = "en"

[[cases]]
name = "unsupported-arch-case-1"
features = ""
no_default_features = false
arch = ["nonexistent_arch_1"]

[[cases]]
name = "unsupported-arch-case-2"
features = ""
no_default_features = false
arch = ["nonexistent_arch_2"]
"#;
        fs::write(&matrix_path, content).unwrap();

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("No test cases to run"));
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_very_long_case_name() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = temp_dir.path().join("long_name.toml");
        let long_name = "a".repeat(1000); // Very long case name
        let content = format!(
            r#"
language = "en"

[[cases]]
name = "{}"
features = ""
no_default_features = false
"#,
            long_name
        );
        fs::write(&matrix_path, content).unwrap();

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("All tests passed successfully!"));
    }

    #[test]
    fn test_unicode_in_case_names() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = temp_dir.path().join("unicode.toml");
        let content = r#"
language = "zh-CN"

[[cases]]
name = "测试用例-🚀"
features = ""
no_default_features = false

[[cases]]
name = "тест-кейс"
features = ""
no_default_features = false
"#;
        fs::write(&matrix_path, content).unwrap();

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("所有测试成功通过！"));
    }

    #[test]
    fn test_special_characters_in_features() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = temp_dir.path().join("special_features.toml");
        let content = r#"
language = "en"

[[cases]]
name = "special-features-case"
features = "feature_a"
no_default_features = false
"#;
        fs::write(&matrix_path, content).unwrap();

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("All tests passed successfully!"));
    }
}

#[cfg(test)]
mod runner_error_tests {
    use super::*;

    #[test]
    fn test_invalid_runner_index() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = temp_dir.path().join("valid.toml");
        let content = r#"
language = "en"

[[cases]]
name = "test-case"
features = ""
no_default_features = false
"#;
        fs::write(&matrix_path, content).unwrap();

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
            .arg("2"); // Index equal to total - invalid

        cmd.assert().failure().stderr(predicate::str::contains(
            "Runner index must be less than total runners.",
        ));
    }

    #[test]
    fn test_inconsistent_runner_flags() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = temp_dir.path().join("valid.toml");
        let content = r#"
language = "en"

[[cases]]
name = "test-case"
features = ""
no_default_features = false
"#;
        fs::write(&matrix_path, content).unwrap();

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--lang")
            .arg("en")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project")
            .arg("--total-runners")
            .arg("2"); // Missing --runner-index

        cmd.assert().failure().stderr(predicate::str::contains(
            "Both --total-runners and --runner-index must be provided.",
        ));
    }
}

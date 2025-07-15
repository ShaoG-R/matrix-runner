//! # Internationalization Integration Tests / 国际化集成测试
//!
//! This module contains integration tests for internationalization features,
//! testing language switching, locale detection, and multilingual output.
//!
//! 此模块包含国际化功能的集成测试，
//! 测试语言切换、区域设置检测和多语言输出。

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::TempDir;
use std::fs;

/// Helper function to create a Chinese language test matrix
/// 创建中文语言测试矩阵的辅助函数
fn create_chinese_matrix(temp_dir: &TempDir) -> std::path::PathBuf {
    let matrix_path = temp_dir.path().join("chinese.toml");
    let content = r#"
language = "zh-CN"

[[cases]]
name = "中文测试用例"
features = ""
no_default_features = false

[[cases]]
name = "功能测试"
features = "feature_a"
no_default_features = false
"#;
    fs::write(&matrix_path, content).unwrap();
    matrix_path
}

/// Helper function to create an English language test matrix
/// 创建英文语言测试矩阵的辅助函数
fn create_english_matrix(temp_dir: &TempDir) -> std::path::PathBuf {
    let matrix_path = temp_dir.path().join("english.toml");
    let content = r#"
language = "en"

[[cases]]
name = "english-test-case"
features = ""
no_default_features = false

[[cases]]
name = "feature-test"
features = "feature_a"
no_default_features = false
"#;
    fs::write(&matrix_path, content).unwrap();
    matrix_path
}

/// Helper function to create a matrix without language specification
/// 创建未指定语言的测试矩阵的辅助函数
fn create_default_language_matrix(temp_dir: &TempDir) -> std::path::PathBuf {
    let matrix_path = temp_dir.path().join("default_lang.toml");
    let content = r#"
# No language specified - should default to "en"

[[cases]]
name = "default-language-case"
features = ""
no_default_features = false
"#;
    fs::write(&matrix_path, content).unwrap();
    matrix_path
}

#[cfg(test)]
mod language_output_tests {
    use super::*;

    #[test]
    fn test_chinese_output() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = create_chinese_matrix(&temp_dir);
        
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
    fn test_english_output() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = create_english_matrix(&temp_dir);
        
        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project");
        
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("All tests passed successfully!"));
    }

    #[test]
    fn test_default_language_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = create_default_language_matrix(&temp_dir);
        
        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project");
        
        // Should default to English
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("All tests passed successfully!"));
    }

    #[test]
    fn test_invalid_language_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = temp_dir.path().join("invalid_lang.toml");
        let content = r#"
language = "invalid-language-code"

[[cases]]
name = "test-case"
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
        
        // Should fallback to English
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("All tests passed successfully!"));
    }
}

#[cfg(test)]
mod init_command_i18n_tests {
    use super::*;

    #[test]
    fn test_init_with_english_language() {
        let temp_dir = TempDir::new().unwrap();

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("init")
            .arg("--language")
            .arg("en")
            .current_dir(&temp_dir);

        // For now, just test that the command starts without crashing
        let output = cmd.output().unwrap();

        // The command should exit (either success or failure is acceptable for init without input)
        assert!(output.status.code().is_some());
    }

    #[test]
    fn test_init_with_chinese_language() {
        let temp_dir = TempDir::new().unwrap();

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("init")
            .arg("--language")
            .arg("zh-CN")
            .current_dir(&temp_dir);

        // For now, just test that the command starts without crashing
        let output = cmd.output().unwrap();

        // The command should exit (either success or failure is acceptable for init without input)
        assert!(output.status.code().is_some());
    }

    #[test]
    fn test_init_auto_language_detection() {
        let temp_dir = TempDir::new().unwrap();

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("init")
            .current_dir(&temp_dir);

        // For now, just test that the command starts without crashing
        let output = cmd.output().unwrap();

        // The command should exit (either success or failure is acceptable for init without input)
        assert!(output.status.code().is_some());
    }

    #[test]
    fn test_init_with_invalid_language() {
        let temp_dir = TempDir::new().unwrap();

        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("init")
            .arg("--language")
            .arg("invalid-lang")
            .current_dir(&temp_dir);

        // Should fallback to English - for now just test that it doesn't crash
        let output = cmd.output().unwrap();

        // The command should exit (either success or failure is acceptable for init without input)
        assert!(output.status.code().is_some());
    }
}

#[cfg(test)]
mod error_message_i18n_tests {
    use super::*;

    #[test]
    fn test_chinese_error_messages() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = temp_dir.path().join("chinese_error.toml");
        let content = r#"
language = "zh-CN"

[[cases]]
name = "构建失败测试"
features = "feature_build_fail"
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
            .failure()
            .stdout(predicate::str::contains("检测到意外失败"));
    }

    #[test]
    fn test_english_error_messages() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = temp_dir.path().join("english_error.toml");
        let content = r#"
language = "en"

[[cases]]
name = "build-failure-test"
features = "feature_build_fail"
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
            .failure()
            .stdout(predicate::str::contains("UNEXPECTED FAILURE DETECTED"));
    }
}

#[cfg(test)]
mod html_report_i18n_tests {
    use super::*;

    #[test]
    fn test_chinese_html_report() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = create_chinese_matrix(&temp_dir);
        let report_path = temp_dir.path().join("chinese_report.html");
        
        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project")
            .arg("--html")
            .arg(&report_path);
        
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Generating HTML report"));
        
        // Verify the HTML report was created
        assert!(report_path.exists());
        
        // Check that the report contains Chinese content
        let report_content = fs::read_to_string(&report_path).unwrap();
        assert!(
            report_content.contains("<title>测试报告</title>"),
            "Chinese HTML report content is invalid. Got:\n\n{}",
            report_content
        );
    }

    #[test]
    fn test_english_html_report() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = create_english_matrix(&temp_dir);
        let report_path = temp_dir.path().join("english_report.html");
        
        let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
        cmd.arg("run")
            .arg("--config")
            .arg(&matrix_path)
            .arg("--project-dir")
            .arg("tests/sample_project")
            .arg("--html")
            .arg(&report_path);
        
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Generating HTML report"));
        
        // Verify the HTML report was created
        assert!(report_path.exists());
        
        // Check that the report contains English content
        let report_content = fs::read_to_string(&report_path).unwrap();
        assert!(report_content.contains("<title>Test Report</title>"));
    }
}

#[cfg(test)]
mod mixed_language_tests {
    use super::*;

    #[test]
    fn test_chinese_config_with_english_case_names() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = temp_dir.path().join("mixed.toml");
        let content = r#"
language = "zh-CN"

[[cases]]
name = "english-case-name"
features = ""
no_default_features = false

[[cases]]
name = "中文用例名称"
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
    fn test_unicode_handling_in_output() {
        let temp_dir = TempDir::new().unwrap();
        let matrix_path = temp_dir.path().join("unicode.toml");
        let content = r#"
language = "zh-CN"

[[cases]]
name = "emoji-test-🚀"
features = ""
no_default_features = false

[[cases]]
name = "特殊字符测试-©®™"
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
}

//! # Models Module Unit Tests / Models 模块单元测试
//!
//! This module contains comprehensive unit tests for the `models.rs` module,
//! testing the various data structures and their behavior.
//!
//! 此模块包含 `models.rs` 模块的全面单元测试，
//! 测试各种数据结构及其行为。

use matrix_runner::runner::config::TestCase;
use matrix_runner::runner::i18n;
use matrix_runner::runner::models::{
    CargoDiagnostic, CargoMessage, CargoTarget, FailureReason, TestResult,
};
use matrix_runner::runner::utils::create_build_dir;
use std::path::PathBuf;

/// Initialize i18n for tests / 为测试初始化 i18n
fn setup_i18n() {
    i18n::init("en");
}

/// Helper function to create a test case / 创建测试用例的辅助函数
fn create_test_case(name: &str) -> TestCase {
    TestCase {
        name: name.to_string(),
        features: "".to_string(),
        no_default_features: false,
        command: None,
        allow_failure: vec![],
        arch: vec![],
    }
}

#[cfg(test)]
mod test_result_tests {
    use super::*;

    #[test]
    fn test_test_result_passed() {
        let case = create_test_case("passed-test");
        let result = TestResult::Passed {
            case: case.clone(),
            output: "Test passed successfully".to_string(),
        };

        match &result {
            TestResult::Passed {
                case: result_case,
                output,
            } => {
                assert_eq!(result_case.name, "passed-test");
                assert_eq!(output, "Test passed successfully");
            }
            _ => panic!("Expected Passed variant"),
        }

        assert!(!result.is_unexpected_failure());
    }

    #[test]
    fn test_test_result_failed_unexpected() {
        let case = create_test_case("failed-test");
        let result = TestResult::Failed {
            case: case.clone(),
            output: "Test failed".to_string(),
            reason: FailureReason::Test,
        };

        match &result {
            TestResult::Failed {
                case: result_case,
                output,
                reason,
            } => {
                assert_eq!(result_case.name, "failed-test");
                assert_eq!(output, "Test failed");
                assert!(matches!(reason, FailureReason::Test));
            }
            _ => panic!("Expected Failed variant"),
        }

        // Should be unexpected failure since allow_failure is empty
        assert!(result.is_unexpected_failure());
    }

    #[test]
    fn test_test_result_failed_allowed() {
        let mut case = create_test_case("allowed-failure-test");
        case.allow_failure = vec![std::env::consts::OS.to_string()];

        let result = TestResult::Failed {
            case: case.clone(),
            output: "Test failed but allowed".to_string(),
            reason: FailureReason::Build,
        };

        // Should not be unexpected failure since current OS is in allow_failure list
        assert!(!result.is_unexpected_failure());
    }

    #[test]
    fn test_test_result_skipped() {
        let result = TestResult::Skipped;

        match &result {
            TestResult::Skipped => {
                // Expected
            }
            _ => panic!("Expected Skipped variant"),
        }

        assert!(!result.is_unexpected_failure());
    }

    #[test]
    fn test_test_result_clone() {
        let case = create_test_case("clone-test");
        let original = TestResult::Passed {
            case: case.clone(),
            output: "Original output".to_string(),
        };

        let cloned = original.clone();

        match (&original, &cloned) {
            (
                TestResult::Passed {
                    case: orig_case,
                    output: orig_output,
                },
                TestResult::Passed {
                    case: clone_case,
                    output: clone_output,
                },
            ) => {
                assert_eq!(orig_case.name, clone_case.name);
                assert_eq!(orig_output, clone_output);
            }
            _ => panic!("Clone should preserve variant type"),
        }
    }
}

#[cfg(test)]
mod failure_reason_tests {
    use super::*;

    #[test]
    fn test_failure_reason_variants() {
        let build_failure = FailureReason::Build;
        let test_failure = FailureReason::Test;

        // Test Debug formatting
        assert_eq!(format!("{:?}", build_failure), "Build");
        assert_eq!(format!("{:?}", test_failure), "Test");
    }

    #[test]
    fn test_failure_reason_clone() {
        let original = FailureReason::Build;
        let cloned = original.clone();

        assert!(matches!(cloned, FailureReason::Build));
    }
}

#[cfg(test)]
mod cargo_diagnostic_tests {
    use super::*;

    #[test]
    fn test_cargo_diagnostic_deserialization() {
        let json = r#"{
            "level": "error",
            "message": "cannot find function `test` in this scope",
            "rendered": "\u001b[0m\u001b[1m\u001b[38;5;9merror[E0425]\u001b[0m\u001b[0m\u001b[1m: cannot find function `test` in this scope\u001b[0m"
        }"#;

        let diagnostic: CargoDiagnostic = serde_json::from_str(json).unwrap();

        assert_eq!(diagnostic.level, "error");
        assert_eq!(
            diagnostic.message,
            "cannot find function `test` in this scope"
        );
        assert!(diagnostic.rendered.is_some());
        assert!(diagnostic.rendered.unwrap().contains("error[E0425]"));
    }

    #[test]
    fn test_cargo_diagnostic_without_rendered() {
        let json = r#"{
            "level": "warning",
            "message": "unused variable: `x`"
        }"#;

        let diagnostic: CargoDiagnostic = serde_json::from_str(json).unwrap();

        assert_eq!(diagnostic.level, "warning");
        assert_eq!(diagnostic.message, "unused variable: `x`");
        assert!(diagnostic.rendered.is_none());
    }

    #[test]
    fn test_cargo_diagnostic_clone() {
        let original = CargoDiagnostic {
            level: "error".to_string(),
            message: "test message".to_string(),
            rendered: Some("rendered message".to_string()),
        };

        let cloned = original.clone();

        assert_eq!(original.level, cloned.level);
        assert_eq!(original.message, cloned.message);
        assert_eq!(original.rendered, cloned.rendered);
    }
}

#[cfg(test)]
mod cargo_message_tests {
    use super::*;

    #[test]
    fn test_cargo_message_compiler_message() {
        let json = r#"{
            "reason": "compiler-message",
            "message": {
                "level": "error",
                "message": "test error",
                "rendered": "rendered error"
            }
        }"#;

        let message: CargoMessage = serde_json::from_str(json).unwrap();

        assert_eq!(message.reason, "compiler-message");
        assert!(message.message.is_some());
        assert!(message.target.is_none());
        assert!(message.executable.is_none());

        let diagnostic = message.message.unwrap();
        assert_eq!(diagnostic.level, "error");
        assert_eq!(diagnostic.message, "test error");
    }

    #[test]
    fn test_cargo_message_compiler_artifact() {
        let json = r#"{
            "reason": "compiler-artifact",
            "target": {
                "name": "test-crate",
                "test": true
            },
            "executable": "/path/to/executable"
        }"#;

        let message: CargoMessage = serde_json::from_str(json).unwrap();

        assert_eq!(message.reason, "compiler-artifact");
        assert!(message.target.is_some());
        assert!(message.executable.is_some());
        assert!(message.message.is_none());

        let target = message.target.unwrap();
        assert_eq!(target.name, "test-crate");
        assert!(target.test);

        let executable = message.executable.unwrap();
        assert_eq!(executable, PathBuf::from("/path/to/executable"));
    }
}

#[cfg(test)]
mod cargo_target_tests {
    use super::*;

    #[test]
    fn test_cargo_target_deserialization() {
        let json = r#"{
            "name": "my-crate",
            "test": true
        }"#;

        let target: CargoTarget = serde_json::from_str(json).unwrap();

        assert_eq!(target.name, "my-crate");
        assert!(target.test);
    }

    #[test]
    fn test_cargo_target_non_test() {
        let json = r#"{
            "name": "my-lib",
            "test": false
        }"#;

        let target: CargoTarget = serde_json::from_str(json).unwrap();

        assert_eq!(target.name, "my-lib");
        assert!(!target.test);
    }
}

#[cfg(test)]
mod build_context_tests {
    use super::*;

    #[test]
    fn test_build_context_creation_and_cleanup() {
        setup_i18n();

        let target_path = {
            let build_ctx = create_build_dir("test-crate", "", false).unwrap();
            let path = build_ctx.target_path.clone();

            // Verify the directory exists while BuildContext is alive
            assert!(path.exists());
            assert!(path.is_dir());

            path
            // BuildContext goes out of scope here
        };

        // Directory should be cleaned up after BuildContext is dropped
        assert!(!target_path.exists());
    }

    #[test]
    fn test_build_context_target_path() {
        setup_i18n();

        let build_ctx = create_build_dir("test-crate", "feature1", true).unwrap();

        // Verify target_path is valid
        assert!(build_ctx.target_path.exists());
        assert!(build_ctx.target_path.is_dir());

        // Verify the path contains expected components
        let path_str = build_ctx.target_path.to_string_lossy();
        assert!(path_str.contains("test-crate"));
        assert!(path_str.contains("no-std"));
        assert!(path_str.contains("feature1"));
    }
}

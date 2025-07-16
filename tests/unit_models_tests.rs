//! # Models Module Unit Tests / Models 模块单元测试
//!
//! This module contains comprehensive unit tests for the `models.rs` module,
//! testing the various data structures and their behavior.
//!
//! 此模块包含 `models.rs` 模块的全面单元测试，
//! 测试各种数据结构及其行为。

use matrix_runner::core::config::TestCase;
use matrix_runner::core::models::{
    CargoDiagnostic, CargoMessage, CargoTarget, FailureReason, TestResult,
};
use std::time::Duration;

/// Helper function to create a test case / 创建测试用例的辅助函数
fn create_test_case(name: &str) -> TestCase {
    TestCase {
        name: name.to_string(),
        features: "".to_string(),
        no_default_features: false,
        command: None,
        allow_failure: vec![],
        arch: vec![],
        retries: None,
        timeout_secs: None,
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
            duration: Duration::from_secs(1),
            retries: 1,
        };

        match &result {
            TestResult::Passed {
                case: result_case,
                output,
                ..
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
            reason: FailureReason::TestFailed,
            duration: Duration::from_secs(1),
        };

        match &result {
            TestResult::Failed {
                case: result_case,
                output,
                reason,
                ..
            } => {
                assert_eq!(result_case.name, "failed-test");
                assert_eq!(output, "Test failed");
                assert!(matches!(reason, FailureReason::TestFailed));
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
            duration: Duration::from_secs(1),
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
            duration: Duration::from_secs(5),
            retries: 2,
        };

        let cloned = original.clone();

        match (&original, &cloned) {
            (
                TestResult::Passed {
                    case: orig_case,
                    output: orig_output,
                    ..
                },
                TestResult::Passed {
                    case: clone_case,
                    output: clone_output,
                    ..
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
        let test_failure = FailureReason::TestFailed;

        // Test Debug formatting
        assert_eq!(format!("{:?}", build_failure), "Build");
        assert_eq!(format!("{:?}", test_failure), "TestFailed");
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
                "test": true,
                "kind": ["bin"]
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
        assert_eq!(executable, std::path::PathBuf::from("/path/to/executable"));
    }
}

#[cfg(test)]
mod cargo_target_tests {
    use super::*;

    #[test]
    fn test_cargo_target_deserialization() {
        let json = r#"{
            "name": "my-crate",
            "test": true,
            "kind": ["bin"]
        }"#;

        let target: CargoTarget = serde_json::from_str(json).unwrap();

        assert_eq!(target.name, "my-crate");
        assert!(target.test);
    }

    #[test]
    fn test_cargo_target_non_test() {
        let json = r#"{
            "name": "my-lib",
            "test": false,
            "kind": ["lib"]
        }"#;

        let target: CargoTarget = serde_json::from_str(json).unwrap();

        assert_eq!(target.name, "my-lib");
        assert!(!target.test);
    }
}

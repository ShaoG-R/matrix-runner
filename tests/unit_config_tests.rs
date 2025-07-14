//! # Config Module Unit Tests / Config 模块单元测试
//!
//! This module contains comprehensive unit tests for the `config.rs` module,
//! testing the `TestCase` and `TestMatrix` structures and their serialization/deserialization.
//!
//! 此模块包含 `config.rs` 模块的全面单元测试，
//! 测试 `TestCase` 和 `TestMatrix` 结构体及其序列化/反序列化。

use matrix_runner::runner::config::{TestCase, TestMatrix};

#[cfg(test)]
mod test_case_tests {
    use super::*;

    #[test]
    fn test_test_case_basic_serialization() {
        let test_case = TestCase {
            name: "basic-test".to_string(),
            features: "feature1,feature2".to_string(),
            no_default_features: false,
            command: None,
            allow_failure: vec![],
            arch: vec![],
        };

        let toml_str = toml::to_string(&test_case).unwrap();

        assert!(toml_str.contains("name = \"basic-test\""));
        assert!(toml_str.contains("features = \"feature1,feature2\""));
        assert!(toml_str.contains("no_default_features = false"));
    }

    #[test]
    fn test_test_case_with_custom_command() {
        let test_case = TestCase {
            name: "custom-command-test".to_string(),
            features: "".to_string(),
            no_default_features: true,
            command: Some("cargo test --release".to_string()),
            allow_failure: vec!["windows".to_string()],
            arch: vec!["x86_64".to_string(), "aarch64".to_string()],
        };

        let toml_str = toml::to_string(&test_case).unwrap();

        assert!(toml_str.contains("name = \"custom-command-test\""));
        assert!(toml_str.contains("command = \"cargo test --release\""));
        assert!(toml_str.contains("no_default_features = true"));
        assert!(toml_str.contains("allow_failure = [\"windows\"]"));
        assert!(toml_str.contains("arch = [\"x86_64\", \"aarch64\"]"));
    }

    #[test]
    fn test_test_case_deserialization_minimal() {
        let toml_str = r#"
            name = "minimal-test"
            features = ""
            no_default_features = false
        "#;

        let test_case: TestCase = toml::from_str(toml_str).unwrap();

        assert_eq!(test_case.name, "minimal-test");
        assert_eq!(test_case.features, "");
        assert!(!test_case.no_default_features);
        assert!(test_case.command.is_none());
        assert!(test_case.allow_failure.is_empty());
        assert!(test_case.arch.is_empty());
    }

    #[test]
    fn test_test_case_deserialization_full() {
        let toml_str = r#"
            name = "full-test"
            features = "feature1,feature2"
            no_default_features = true
            command = "custom command"
            allow_failure = ["linux", "macos"]
            arch = ["x86_64"]
        "#;

        let test_case: TestCase = toml::from_str(toml_str).unwrap();

        assert_eq!(test_case.name, "full-test");
        assert_eq!(test_case.features, "feature1,feature2");
        assert!(test_case.no_default_features);
        assert_eq!(test_case.command, Some("custom command".to_string()));
        assert_eq!(test_case.allow_failure, vec!["linux", "macos"]);
        assert_eq!(test_case.arch, vec!["x86_64"]);
    }

    #[test]
    fn test_test_case_clone() {
        let original = TestCase {
            name: "clone-test".to_string(),
            features: "feature1".to_string(),
            no_default_features: true,
            command: Some("test command".to_string()),
            allow_failure: vec!["windows".to_string()],
            arch: vec!["x86_64".to_string()],
        };

        let cloned = original.clone();

        assert_eq!(original.name, cloned.name);
        assert_eq!(original.features, cloned.features);
        assert_eq!(original.no_default_features, cloned.no_default_features);
        assert_eq!(original.command, cloned.command);
        assert_eq!(original.allow_failure, cloned.allow_failure);
        assert_eq!(original.arch, cloned.arch);
    }
}

#[cfg(test)]
mod test_matrix_tests {
    use super::*;

    #[test]
    fn test_test_matrix_default_language() {
        let toml_str = r#"
            [[cases]]
            name = "test1"
            features = ""
            no_default_features = false
        "#;

        let matrix: TestMatrix = toml::from_str(toml_str).unwrap();

        // Should default to "en" when language is not specified
        assert_eq!(matrix.language, "en");
        assert_eq!(matrix.cases.len(), 1);
        assert_eq!(matrix.cases[0].name, "test1");
    }

    #[test]
    fn test_test_matrix_explicit_language() {
        let toml_str = r#"
            language = "zh-CN"
            
            [[cases]]
            name = "test1"
            features = ""
            no_default_features = false
        "#;

        let matrix: TestMatrix = toml::from_str(toml_str).unwrap();

        assert_eq!(matrix.language, "zh-CN");
        assert_eq!(matrix.cases.len(), 1);
    }

    #[test]
    fn test_test_matrix_multiple_cases() {
        let toml_str = r#"
            language = "en"
            
            [[cases]]
            name = "std-test"
            features = "std"
            no_default_features = false
            
            [[cases]]
            name = "no-std-test"
            features = "core"
            no_default_features = true
            
            [[cases]]
            name = "custom-test"
            features = ""
            no_default_features = false
            command = "echo test"
            allow_failure = ["windows"]
            arch = ["x86_64", "aarch64"]
        "#;

        let matrix: TestMatrix = toml::from_str(toml_str).unwrap();

        assert_eq!(matrix.language, "en");
        assert_eq!(matrix.cases.len(), 3);

        // Verify first case
        assert_eq!(matrix.cases[0].name, "std-test");
        assert_eq!(matrix.cases[0].features, "std");
        assert!(!matrix.cases[0].no_default_features);

        // Verify second case
        assert_eq!(matrix.cases[1].name, "no-std-test");
        assert_eq!(matrix.cases[1].features, "core");
        assert!(matrix.cases[1].no_default_features);

        // Verify third case
        assert_eq!(matrix.cases[2].name, "custom-test");
        assert_eq!(matrix.cases[2].command, Some("echo test".to_string()));
        assert_eq!(matrix.cases[2].allow_failure, vec!["windows"]);
        assert_eq!(matrix.cases[2].arch, vec!["x86_64", "aarch64"]);
    }

    #[test]
    fn test_test_matrix_serialization() {
        let matrix = TestMatrix {
            language: "zh-CN".to_string(),
            cases: vec![
                TestCase {
                    name: "test1".to_string(),
                    features: "feature1".to_string(),
                    no_default_features: false,
                    command: None,
                    allow_failure: vec![],
                    arch: vec![],
                },
                TestCase {
                    name: "test2".to_string(),
                    features: "".to_string(),
                    no_default_features: true,
                    command: Some("custom command".to_string()),
                    allow_failure: vec!["linux".to_string()],
                    arch: vec!["x86_64".to_string()],
                },
            ],
        };

        let toml_str = toml::to_string_pretty(&matrix).unwrap();

        assert!(toml_str.contains("language = \"zh-CN\""));
        assert!(toml_str.contains("name = \"test1\""));
        assert!(toml_str.contains("name = \"test2\""));
        assert!(toml_str.contains("command = \"custom command\""));
        assert!(toml_str.contains("allow_failure = [\"linux\"]"));
    }

    #[test]
    fn test_test_matrix_empty_cases() {
        let toml_str = r#"
            language = "en"
            cases = []
        "#;

        let matrix: TestMatrix = toml::from_str(toml_str).unwrap();

        assert_eq!(matrix.language, "en");
        assert!(matrix.cases.is_empty());
    }

    #[test]
    fn test_test_matrix_roundtrip_serialization() {
        let original = TestMatrix {
            language: "en".to_string(),
            cases: vec![TestCase {
                name: "roundtrip-test".to_string(),
                features: "feature1,feature2".to_string(),
                no_default_features: true,
                command: Some("test command".to_string()),
                allow_failure: vec!["windows".to_string(), "linux".to_string()],
                arch: vec!["x86_64".to_string()],
            }],
        };

        // Serialize to TOML
        let toml_str = toml::to_string_pretty(&original).unwrap();

        // Deserialize back
        let deserialized: TestMatrix = toml::from_str(&toml_str).unwrap();

        // Verify they match
        assert_eq!(original.language, deserialized.language);
        assert_eq!(original.cases.len(), deserialized.cases.len());

        let orig_case = &original.cases[0];
        let deser_case = &deserialized.cases[0];

        assert_eq!(orig_case.name, deser_case.name);
        assert_eq!(orig_case.features, deser_case.features);
        assert_eq!(
            orig_case.no_default_features,
            deser_case.no_default_features
        );
        assert_eq!(orig_case.command, deser_case.command);
        assert_eq!(orig_case.allow_failure, deser_case.allow_failure);
        assert_eq!(orig_case.arch, deser_case.arch);
    }

    #[test]
    fn test_test_matrix_invalid_toml() {
        let invalid_toml = r#"
            language = "en"
            [[cases]]
            # Missing required fields
            name = "incomplete-test"
        "#;

        let result: Result<TestMatrix, _> = toml::from_str(invalid_toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_test_matrix_with_chinese_content() {
        let toml_str = r#"
            language = "zh-CN"
            
            [[cases]]
            name = "中文测试"
            features = "功能1,功能2"
            no_default_features = false
        "#;

        let matrix: TestMatrix = toml::from_str(toml_str).unwrap();

        assert_eq!(matrix.language, "zh-CN");
        assert_eq!(matrix.cases[0].name, "中文测试");
        assert_eq!(matrix.cases[0].features, "功能1,功能2");
    }
}

//! # I18n Module Unit Tests / I18n 模块单元测试
//!
//! This module contains comprehensive unit tests for the `i18n.rs` module,
//! testing internationalization functionality, language detection, and formatting.
//!
//! 此模块包含 `i18n.rs` 模块的全面单元测试，
//! 测试国际化功能、语言检测和格式化。

use matrix_runner::runner::i18n;

#[cfg(test)]
mod i18n_init_tests {
    use super::*;

    #[test]
    fn test_init_with_english() {
        i18n::init("en");
        // Test that a basic translation works
        let result = i18n::t(i18n::I18nKey::ProjectRootDetected);
        assert!(!result.is_empty());
        // Should contain English text
        assert!(result.contains("Project root detected"));
    }

    #[test]
    fn test_init_with_chinese() {
        i18n::init("zh-CN");
        // Test that a basic translation works
        let result = i18n::t(i18n::I18nKey::ProjectRootDetected);
        assert!(!result.is_empty());
        // Should contain Chinese text
        assert!(result.contains("检测到项目根目录"));
    }

    #[test]
    fn test_init_with_invalid_language() {
        i18n::init("invalid-lang");
        // Should fallback to English
        let result = i18n::t(i18n::I18nKey::ProjectRootDetected);
        assert!(!result.is_empty());
        assert!(result.contains("Project root detected"));
    }

    #[test]
    fn test_init_with_empty_string() {
        i18n::init("");
        // Should fallback to English
        let result = i18n::t(i18n::I18nKey::ProjectRootDetected);
        assert!(!result.is_empty());
        assert!(result.contains("Project root detected"));
    }

    #[test]
    fn test_multiple_init_calls() {
        // Initialize with Chinese
        i18n::init("zh-CN");
        let chinese_result = i18n::t(i18n::I18nKey::ProjectRootDetected);
        assert!(chinese_result.contains("检测到项目根目录"));

        // Switch to English
        i18n::init("en");
        let english_result = i18n::t(i18n::I18nKey::ProjectRootDetected);
        assert!(english_result.contains("Project root detected"));

        // Results should be different
        assert_ne!(chinese_result, english_result);
    }
}

#[cfg(test)]
mod translation_tests {
    use super::*;

    #[test]
    fn test_basic_translation_english() {
        i18n::init("en");
        
        let result = i18n::t(i18n::I18nKey::TestingCrate);
        assert!(!result.is_empty());
        assert!(result.contains("Testing crate:"));
    }

    #[test]
    fn test_basic_translation_chinese() {
        i18n::init("zh-CN");
        
        let result = i18n::t(i18n::I18nKey::TestingCrate);
        assert!(!result.is_empty());
        assert!(result.contains("正在测试的 Crate:"));
    }

    #[test]
    fn test_formatted_translation_english() {
        i18n::init("en");
        
        let crate_name = "test-crate";
        let result = i18n::t_fmt(i18n::I18nKey::TestingCrate, &[&crate_name]);
        assert!(!result.is_empty());
        assert!(result.contains("test-crate"));
    }

    #[test]
    fn test_formatted_translation_chinese() {
        i18n::init("zh-CN");
        
        let crate_name = "测试包";
        let result = i18n::t_fmt(i18n::I18nKey::TestingCrate, &[&crate_name]);
        assert!(!result.is_empty());
        assert!(result.contains("测试包"));
    }

    #[test]
    fn test_multiple_arguments_formatting() {
        i18n::init("en");
        
        let arg1 = "first";
        let arg2 = "second";
        let result = i18n::t_fmt(i18n::I18nKey::SystemLanguageDetected, &[&arg1, &arg2]);
        assert!(!result.is_empty());
        // Should contain at least one of the arguments
        assert!(result.contains("first") || result.contains("second"));
    }

    #[test]
    fn test_numeric_arguments_formatting() {
        i18n::init("en");
        
        let number = 42;
        let result = i18n::t_fmt(i18n::I18nKey::SystemLanguageDetected, &[&number]);
        assert!(!result.is_empty());
        assert!(result.contains("42"));
    }
}

#[cfg(test)]
mod language_detection_tests {
    use super::*;

    #[test]
    fn test_detect_system_language_returns_supported() {
        let detected = i18n::detect_system_language();
        // Should return either "en" or "zh-CN"
        assert!(detected == "en" || detected == "zh-CN");
    }

    #[test]
    fn test_detect_system_language_consistency() {
        let first_call = i18n::detect_system_language();
        let second_call = i18n::detect_system_language();
        // Should be consistent across calls
        assert_eq!(first_call, second_call);
    }

    #[test]
    fn test_detect_system_language_integration() {
        let detected = i18n::detect_system_language();
        
        // Initialize with detected language
        i18n::init(&detected);
        
        // Should be able to get translations
        let result = i18n::t(i18n::I18nKey::ProjectRootDetected);
        assert!(!result.is_empty());
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_translation_with_uninitialized_state() {
        // Even without explicit init, should work with default language
        let result = i18n::t(i18n::I18nKey::ProjectRootDetected);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_formatting_with_empty_args() {
        i18n::init("en");
        
        let result = i18n::t_fmt(i18n::I18nKey::ProjectRootDetected, &[]);
        assert!(!result.is_empty());
        // Should still work even with no arguments
    }

    #[test]
    fn test_formatting_with_mismatched_args() {
        i18n::init("en");
        
        // Provide more arguments than placeholders
        let extra_arg = "extra";
        let result = i18n::t_fmt(i18n::I18nKey::ProjectRootDetected, &[&extra_arg, &extra_arg]);
        assert!(!result.is_empty());
        // Should not crash
    }
}

#[cfg(test)]
mod concurrent_access_tests {
    use super::*;
    use std::thread;
    use std::sync::Arc;

    #[test]
    fn test_concurrent_init_and_translate() {
        let handles: Vec<_> = (0..10)
            .map(|i| {
                thread::spawn(move || {
                    let lang = if i % 2 == 0 { "en" } else { "zh-CN" };
                    i18n::init(lang);
                    i18n::t(i18n::I18nKey::ProjectRootDetected)
                })
            })
            .collect();

        for handle in handles {
            let result = handle.join().unwrap();
            assert!(!result.is_empty());
        }
    }

    #[test]
    fn test_concurrent_translations() {
        i18n::init("en");
        
        let handles: Vec<_> = (0..10)
            .map(|_| {
                thread::spawn(|| {
                    i18n::t(i18n::I18nKey::TestingCrate)
                })
            })
            .collect();

        for handle in handles {
            let result = handle.join().unwrap();
            assert!(!result.is_empty());
            assert!(result.contains("Testing crate:") || result.contains("正在测试的 Crate:"));
        }
    }

    #[test]
    fn test_concurrent_formatted_translations() {
        i18n::init("en");
        
        let shared_arg = Arc::new("shared-value".to_string());
        
        let handles: Vec<_> = (0..5)
            .map(|i| {
                let arg = Arc::clone(&shared_arg);
                thread::spawn(move || {
                    let local_arg = format!("arg-{}", i);
                    i18n::t_fmt(i18n::I18nKey::TestingCrate, &[&*arg, &local_arg])
                })
            })
            .collect();

        for handle in handles {
            let result = handle.join().unwrap();
            assert!(!result.is_empty());
            assert!(result.contains("shared-value"));
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_language_switching_workflow() {
        // Start with English
        i18n::init("en");
        let en_result = i18n::t(i18n::I18nKey::ProjectRootDetected);
        assert!(en_result.contains("Project root detected"));

        // Switch to Chinese
        i18n::init("zh-CN");
        let zh_result = i18n::t(i18n::I18nKey::ProjectRootDetected);
        assert!(zh_result.contains("检测到项目根目录"));

        // Switch back to English
        i18n::init("en");
        let en_result2 = i18n::t(i18n::I18nKey::ProjectRootDetected);
        assert_eq!(en_result, en_result2);
    }

    #[test]
    fn test_auto_detection_and_manual_override() {
        // Auto-detect system language
        let detected = i18n::detect_system_language();
        i18n::init(&detected);
        let auto_result = i18n::t(i18n::I18nKey::ProjectRootDetected);

        // Manually override to English
        i18n::init("en");
        let manual_result = i18n::t(i18n::I18nKey::ProjectRootDetected);

        // Both should work
        assert!(!auto_result.is_empty());
        assert!(!manual_result.is_empty());
        assert!(manual_result.contains("Project root detected"));
    }

    #[test]
    fn test_formatting_with_different_languages() {
        let test_arg = "test-value";

        // Test English formatting
        i18n::init("en");
        let en_formatted = i18n::t_fmt(i18n::I18nKey::TestingCrate, &[&test_arg]);
        assert!(en_formatted.contains("test-value"));

        // Test Chinese formatting
        i18n::init("zh-CN");
        let zh_formatted = i18n::t_fmt(i18n::I18nKey::TestingCrate, &[&test_arg]);
        assert!(zh_formatted.contains("test-value"));

        // Both should contain the argument but have different base text
        assert_ne!(en_formatted, zh_formatted);
    }
}

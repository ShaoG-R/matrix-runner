//! # I18n Module Unit Tests / I18n 模块单元测试
//!
//! This module contains tests for the internationalization functionality
//! provided by the `rust_i18n` macro.
//!
//! 此模块包含由 `rust_i18n` 宏提供的国际化功能的测试。

use rust_i18n::t;
use std::sync::Mutex;

// Load I18n macro, an alternative to `rust_i18n::i18n!("locales")`
rust_i18n::i18n!("locales", fallback = "en");

lazy_static::lazy_static! {
    static ref I18N_TEST_MUTEX: Mutex<()> = Mutex::new(());
}

#[cfg(test)]
mod i18n_tests {
    use super::*;

    #[test]
    fn test_set_locale_and_translate() {
        let _guard = I18N_TEST_MUTEX.lock().unwrap();

        // Set to English
        rust_i18n::set_locale("en");
        let en_text = t!("common.project_root_detected", path = "test/path");
        assert!(en_text.contains("Project root detected at: test/path"));

        // Switch to Chinese
        rust_i18n::set_locale("zh-CN");
        let zh_text = t!("common.project_root_detected", path = "测试/路径");
        assert!(zh_text.contains("检测到项目根目录于: 测试/路径"));

        // Ensure they are different
        assert_ne!(en_text, zh_text);
    }

    #[test]
    fn test_fallback_to_default_locale() {
        let _guard = I18N_TEST_MUTEX.lock().unwrap();

        // Set an unsupported locale
        rust_i18n::set_locale("fr"); // "fr" is not a supported language in this project

        // It should fall back to the default locale specified in `Cargo.toml` or "en"
        let fallback_text = t!("common.all_tests_passed");
        assert!(
            fallback_text.contains("All tests passed successfully!")
                || fallback_text.contains("所有测试成功通过！")
        );
    }

    #[test]
    fn test_translation_with_interpolation() {
        let _guard = I18N_TEST_MUTEX.lock().unwrap();

        rust_i18n::set_locale("en");
        let text = t!("common.testing_crate", name = "my-crate");
        assert_eq!(text, "Testing crate: my-crate");

        rust_i18n::set_locale("zh-CN");
        let text_zh = t!("common.testing_crate", name = "我的包");
        assert_eq!(text_zh, "正在测试的 Crate: 我的包");
    }

    #[test]
    fn test_sequential_locale_switching() {
        let _guard = I18N_TEST_MUTEX.lock().unwrap();

        // Set to English first
        rust_i18n::set_locale("en");
        let en_text = t!("common.all_tests_passed");
        assert_eq!(en_text, "All tests passed successfully!");

        // Then switch to Chinese
        rust_i18n::set_locale("zh-CN");
        let zh_text = t!("common.all_tests_passed");
        assert_eq!(zh_text, "所有测试成功通过！");
    }
}

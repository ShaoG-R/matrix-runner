//! # Matrix Runner Library / Matrix Runner 库
//!
//! This library provides the core functionality for the Matrix Runner tool,
//! a powerful, configuration-driven test executor for Rust projects.
//!
//! 此库为 Matrix Runner 工具提供核心功能，
//! 这是一个强大的、配置驱动的 Rust 项目测试执行器。
//!
//! ## Modules / 模块
//!
//! - `core` - Core data models and test execution engine
//! - `infra` - Infrastructure services like command execution and file system operations
//! - `reporting` - Test result reporting and visualization
//! - `cli` - Command-line interface and commands
//!
//! - `core` - 核心数据模型和测试执行引擎
//! - `infra` - 基础设施服务，如命令执行和文件系统操作
//! - `reporting` - 测试结果报告和可视化
//! - `cli` - 命令行接口和命令

pub mod core;
pub mod infra;
pub mod reporting;
pub mod cli;

// Re-export commonly used items
pub use core::models;
pub use core::config;
pub use core::execution;

/// Initializes the application's internationalization (i18n) based on the system locale.
///
/// This function detects the user's system locale and sets the appropriate
/// language for the application's user interface. It attempts to match the full
/// locale (e.g., "zh-CN"), then just the language code (e.g., "en"), and
/// finally falls back to the default language ("en").
pub fn init() {
    // Detect system locale and set it for i18n.
    // Fallback to "en" if detection fails.
    let locale = sys_locale::get_locale().unwrap_or_else(|| "en".to_string());
    let available_locales = rust_i18n::available_locales!();

    // Try to match the full locale first (e.g., "zh-CN")
    // Then try to match the language part only (e.g., "en" from "en-US")
    // Finally, fall back to "en"
    let lang = if available_locales.contains(&locale.as_str()) {
        &locale
    } else {
        locale
            .split('-')
            .next()
            .filter(|lang_code| available_locales.contains(lang_code))
            .unwrap_or("en")
    };

    rust_i18n::set_locale(lang);
}

// Initialize i18n
rust_i18n::i18n!("locales", fallback = "en");
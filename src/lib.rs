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

// Initialize i18n
rust_i18n::i18n!("locales");
//! # Core Module / 核心模块
//!
//! This module contains the core functionality of Matrix Runner,
//! including data models, configuration, and test execution logic.
//!
//! 此模块包含 Matrix Runner 的核心功能，
//! 包括数据模型、配置和测试执行逻辑。

pub mod models;
pub mod config;
pub mod execution;
pub mod planner;

// Re-exports
pub use models::TestResult;
pub use config::TestMatrix;
pub use execution::run_test_case; 
//! # Infrastructure Module / 基础设施模块
//!
//! This module provides infrastructure services for Matrix Runner,
//! including command execution, file system operations, and i18n support.
//!
//! 此模块为 Matrix Runner 提供基础设施服务，
//! 包括命令执行、文件系统操作和国际化支持。

pub mod command;
pub mod fs;

// Re-export i18n functions for easier access
pub use rust_i18n::t; 
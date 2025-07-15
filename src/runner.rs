//! # Runner Module / 运行器模块
//!
//! This module contains the core functionality of the Matrix Runner application.
//! It is organized into several submodules, each responsible for a specific
//! aspect of the test execution pipeline.
//!
//! 此模块包含 Matrix Runner 应用程序的核心功能。
//! 它被组织成几个子模块，每个子模块负责测试执行管道的特定方面。
//!
//! ## Module Organization / 模块组织
//!
//! - `command` - Command execution and output capture utilities
//! - `config` - Configuration file parsing and test case definitions
//! - `execution` - Core test execution logic and orchestration
//! - `models` - Data structures and type definitions
//! - `reporting` - Test result reporting and HTML generation
//! - `utils` - Utility functions for file operations and build management
//!
//! - `command` - 命令执行和输出捕获工具
//! - `config` - 配置文件解析和测试用例定义
//! - `execution` - 核心测试执行逻辑和编排
//! - `models` - 数据结构和类型定义
//! - `reporting` - 测试结果报告和 HTML 生成
//! - `utils` - 文件操作和构建管理的工具函数

/// Command execution and output capture utilities / 命令执行和输出捕获工具
pub mod command;
/// Configuration file parsing and test case definitions / 配置文件解析和测试用例定义
pub mod config;
/// Core test execution logic and orchestration / 核心测试执行逻辑和编排
pub mod execution;
/// Data structures and type definitions / 数据结构和类型定义
pub mod models;
/// Test execution planning logic / 测试执行计划逻辑
pub mod planner;
/// Test result reporting and HTML generation / 测试结果报告和 HTML 生成
pub mod reporting;
/// Utility functions for file operations and build management / 文件操作和构建管理的工具函数
pub mod utils;

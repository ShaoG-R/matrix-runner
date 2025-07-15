//! # Matrix Runner - Configuration-Driven Test Executor
//! # Matrix Runner - 配置驱动的测试执行器
//!
//! A powerful, configuration-driven test executor for Rust projects that enables
//! testing across a wide matrix of feature flags and environments. This tool helps
//! ensure comprehensive test coverage by automatically running tests with different
//! feature combinations in isolated environments.
//!
//! 一个强大的、配置驱动的 Rust 项目测试执行器，支持在广泛的 feature 标志和环境矩阵中进行测试。
//! 此工具通过在隔离环境中自动运行不同 feature 组合的测试来帮助确保全面的测试覆盖。
//!
//! ## Key Features / 主要功能
//!
//! - **Matrix Testing**: Run tests across multiple feature flag combinations
//! - **Parallel Execution**: Concurrent test execution with configurable job limits
//! - **Isolated Builds**: Each test runs in its own temporary directory
//! - **HTML Reports**: Generate detailed HTML reports with test results
//! - **Internationalization**: Support for multiple languages (English, Chinese)
//! - **Graceful Shutdown**: Handle interruption signals properly
//!
//! - **矩阵测试**: 在多个 feature 标志组合中运行测试
//! - **并行执行**: 具有可配置作业限制的并发测试执行
//! - **隔离构建**: 每个测试在自己的临时目录中运行
//! - **HTML 报告**: 生成包含测试结果的详细 HTML 报告
//! - **国际化**: 支持多种语言（英语、中文）
//! - **优雅关闭**: 正确处理中断信号


#[tokio::main]
async fn main() {
    if let Err(e) = matrix_runner::cli::run().await {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
    }
}

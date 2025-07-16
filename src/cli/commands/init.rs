//! # Init Command Module / 初始化命令模块
//!
//! This module implements the `init` command for the Matrix Runner CLI,
//! which creates a new test matrix configuration file.
//!
//! 此模块实现了 Matrix Runner CLI 的 `init` 命令，
//! 用于创建新的测试矩阵配置文件。

use anyhow::{Context, Result};
use colored::*;
use std::{fs, path::PathBuf};
use crate::infra::t;

const DEFAULT_CONFIG: &str = r#"# Test Matrix Configuration / 测试矩阵配置
# Documentation: https://github.com/yourusername/matrix-runner

# Language for error messages / 错误消息的语言
language = "en"

# Test Cases / 测试用例
[[cases]]
name = "no-features" # Name of the test case / 测试用例名称
features = "" # Features to enable / 启用的特性
no_default_features = false # Disable default features? / 禁用默认特性？

[[cases]]
name = "all-features"
features = "full,extra"
no_default_features = false

[[cases]]
name = "no-default-features"
features = ""
no_default_features = true
# Optional timeout in seconds / 可选的超时时间（秒）
timeout_secs = 60
# Optional number of retries for flaky tests / 对于不稳定测试的可选重试次数
retries = 1
# Allow failure on specific platforms / 允许在特定平台上失败
allow_failure = ["windows"]
# Only run on specific architectures / 仅在特定架构上运行
arch = ["x86_64", "aarch64"]

# Custom command example / 自定义命令示例
[[cases]]
name = "custom-command"
features = ""
no_default_features = false
# Custom command to run / 要运行的自定义命令
command = "cargo run --example demo"
"#;

/// Executes the init command with the provided arguments.
///
/// # Arguments
/// * `output` - Path for the new configuration file
/// * `force` - Whether to overwrite an existing file
/// * `lang` - Language for error messages
///
/// # Returns
/// A Result indicating success or failure of the command execution
pub async fn execute(output: PathBuf, force: bool, lang: String) -> Result<()> {
    rust_i18n::set_locale(&lang);
    
    // Check if file already exists
    if output.exists() && !force {
        println!(
            "{}",
            t!("init.file_exists", path = output.display()).red()
        );
        println!("{}", t!("init.use_force").yellow());
        return Ok(());
    }

    // Create parent directories if needed
    if let Some(parent) = output.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "{}",
                    t!(
                        "init.create_parent_dir_failed",
                        path = parent.display()
                    )
                )
            })?;
        }
    }

    // Write the default configuration to the output file
    fs::write(&output, DEFAULT_CONFIG).with_context(|| {
        format!(
            "{}",
            t!("init.write_failed", path = output.display())
        )
    })?;

    println!(
        "{}",
        t!("init.success", path = output.display()).green()
    );
    println!("{}", t!("init.next_steps"));

    Ok(())
} 
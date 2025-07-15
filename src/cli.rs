//! # CLI Module / CLI 模块
//!
//! This module defines the command-line interface for Matrix Runner.
//! It processes command line arguments and dispatches to the appropriate commands.
//!
//! 此模块定义了 Matrix Runner 的命令行界面。
//! 它处理命令行参数并分派到适当的命令。

pub mod commands;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// A powerful, configuration-driven test executor for Rust projects.
/// 一个强大的、配置驱动的 Rust 项目测试执行器。
#[derive(Parser)]
#[clap(version, about, long_about = None)]
pub struct Cli {
    /// The subcommand to run.
    /// 要运行的子命令。
    #[clap(subcommand)]
    pub command: Commands,
}

/// The available commands.
/// 可用的命令。
#[derive(Subcommand)]
pub enum Commands {
    /// Run tests according to the test matrix configuration.
    /// 根据测试矩阵配置运行测试。
    Run {
        /// Number of parallel jobs to run. Defaults to half of the CPU cores + 1.
        /// 要运行的并行任务数量。默认为 CPU 核心数的一半 + 1。
        #[clap(short, long)]
        jobs: Option<usize>,

        /// Path to the test matrix configuration file.
        /// 测试矩阵配置文件的路径。
        #[clap(short, long, default_value = "TestMatrix.toml")]
        config: PathBuf,

        /// Path to the project directory.
        /// 项目目录的路径。
        #[clap(short, long, default_value = ".")]
        project_dir: PathBuf,

        /// Total number of distributed runners (for CI).
        /// 分布式运行器的总数（用于 CI）。
        #[clap(long)]
        total_runners: Option<usize>,

        /// Index of this runner (0-based, for CI).
        /// 此运行器的索引（从 0 开始，用于 CI）。
        #[clap(long)]
        runner_index: Option<usize>,

        /// Path for HTML report output.
        /// HTML 报告输出的路径。
        #[clap(long)]
        html: Option<PathBuf>,
    },

    /// Initialize a new test matrix configuration.
    /// 初始化一个新的测试矩阵配置。
    Init {
        /// Path for the new configuration file.
        /// 新配置文件的路径。
        #[clap(short, long, default_value = "TestMatrix.toml")]
        output: PathBuf,

        /// Force overwrite if the file exists.
        /// 如果文件存在，则强制覆盖。
        #[clap(short, long)]
        force: bool,

        /// Specify the language for error messages.
        /// 指定错误消息的语言。
        #[clap(long, default_value = "en")]
        lang: String,
    },
}

/// Parses the command line arguments and returns the CLI structure.
/// 解析命令行参数并返回 CLI 结构。
pub fn parse_args() -> Cli {
    Cli::parse()
}

/// Process the parsed CLI command and dispatch to the appropriate handler.
/// 处理解析后的 CLI 命令并分派到适当的处理程序。
pub async fn process_command(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Run {
            jobs,
            config,
            project_dir,
            total_runners,
            runner_index,
            html,
        } => {
            crate::cli::commands::run::execute(
                jobs,
                config,
                project_dir,
                total_runners,
                runner_index,
                html,
            )
            .await
        }
        Commands::Init {
            output,
            force,
            lang,
        } => crate::cli::commands::init::execute(output, force, lang).await,
    }
} 
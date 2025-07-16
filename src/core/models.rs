//! # Data Models Module / 数据模型模块
//!
//! This module defines the core data structures used throughout the matrix runner.
//! It includes models for test results, build contexts, failure reasons, and
//! cargo-specific message formats.
//!
//! 此模块定义了整个矩阵运行器中使用的核心数据结构。
//! 它包括测试结果、构建上下文、失败原因和 cargo 特定消息格式的模型。

use crate::core::config::TestCase;
use crate::infra::t;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use std::fmt;
use anyhow::Result;

/// Enumerates the possible reasons for a test case failure.
/// This helps in categorizing errors for reporting and handling.
/// 枚举测试用例失败的可能原因。
/// 这有助于对错误进行分类，以便报告和处理。
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum FailureReason {
    /// The test case failed during the `cargo build` or `cargo test --no-run` phase.
    /// 测试用例在 `cargo build` 或 `cargo test --no-run` 阶段失败。
    Build,
    /// The test case built successfully but failed when the test executable was run.
    /// 测试用例构建成功，但在运行测试可执行文件时失败。
    TestFailed,
    /// The test case exceeded its configured timeout.
    /// 测试用例超出了其配置的超时时间。
    Timeout,
    /// A custom command defined in the test case failed.
    /// 测试用例中定义的自定义命令执行失败。
    CustomCommand,
    /// The `cargo build` phase itself failed.
    /// `cargo build` 阶段本身失败。
    BuildFailed,
}

/// Represents the final result of a single test case execution.
/// This enum captures all possible outcomes of running a test case,
/// including success, various types of failures, and skipped tests.
///
/// 表示单个测试用例执行的最终结果。
/// 此枚举捕获运行测试用例的所有可能结果，
/// 包括成功、各种类型的失败和跳过的测试。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestResult {
    /// The test case passed successfully.
    /// 测试用例成功通过。
    Passed {
        /// The test case configuration that was executed / 执行的测试用例配置
        case: TestCase,
        /// The complete output from the test execution / 测试执行的完整输出
        #[allow(dead_code)]
        output: String,
        /// The time taken to execute the test / 执行测试所花费的时间
        duration: Duration,
        /// The number of attempts it took to pass the test (1 means it passed on the first try).
        /// 通过测试所需的尝试次数（1 表示第一次尝试就通过）。
        retries: u8,
    },
    /// The test case failed for various reasons.
    /// 测试用例因各种原因失败。
    Failed {
        /// The test case configuration that failed / 失败的测试用例配置
        case: TestCase,
        /// The complete output from the failed execution / 失败执行的完整输出
        output: String,
        /// The specific reason for the failure / 失败的具体原因
        reason: FailureReason,
        /// The time taken before the failure occurred / 失败发生前所花费的时间
        duration: Duration,
    },
    /// The test case was skipped due to platform or architecture constraints.
    /// 由于平台或架构约束，测试用例被跳过。
    Skipped,
}

impl TestResult {
    /// Checks if a test result is a failure that was not explicitly allowed.
    /// A failure is "unexpected" if it's a `Failed` variant and the current OS
    /// is not in the test case's `allow_failure` list.
    pub fn is_unexpected_failure(&self) -> bool {
        match self {
            TestResult::Failed { case, .. } => {
                !case.allow_failure.iter().any(|s| s == std::env::consts::OS)
            }
            _ => false, // Only failures can be unexpected.
        }
    }

    /// Checks if the test result is a failure that was explicitly allowed for the current platform.
    pub fn is_allowed_failure(&self) -> bool {
        match self {
            TestResult::Failed { case, .. } => {
                case.allow_failure.iter().any(|s| s == std::env::consts::OS)
            }
            _ => false,
        }
    }

    /// Checks if the test result is any kind of failure.
    pub fn is_failure(&self) -> bool {
        matches!(self, TestResult::Failed { .. })
    }

    /// Gets the appropriate CSS class for the test status.
    pub fn get_status_class(&self) -> &str {
        match self {
            TestResult::Passed { .. } => "status-Passed",
            TestResult::Failed { reason, .. } => {
                if self.is_allowed_failure() {
                    "status-Allowed-Failure"
                } else if *reason == FailureReason::Timeout {
                    "status-Timeout"
                } else {
                    "status-Failed"
                }
            }
            TestResult::Skipped => "status-Skipped",
        }
    }

    /// Gets the name of the test case. Returns "Skipped" for skipped tests.
    /// 获取测试用例的名称。对于跳过的测试，返回 "Skipped"。
    pub fn case_name(&self) -> &str {
        match self {
            TestResult::Passed { case, .. } => &case.name,
            TestResult::Failed { case, .. } => &case.name,
            TestResult::Skipped => "Skipped",
        }
    }

    /// Gets the status of the test result as a string for display.
    /// 以字符串形式获取测试结果的状态以供显示。
    pub fn get_status_str(&self, locale: &str) -> String {
        match self {
            TestResult::Passed { .. } => t!("report.status_passed", locale = locale).to_string(),
            TestResult::Failed { case, reason, .. } => {
                if *reason == FailureReason::Timeout {
                    t!("report.status_timeout", locale = locale).to_string()
                } else if case.allow_failure.iter().any(|s| s == std::env::consts::OS) {
                    t!("report.status_allowed_failure", locale = locale).to_string()
                } else {
                    t!("report.status_failed", locale = locale).to_string()
                }
            }
            TestResult::Skipped => t!("report.status_skipped", locale = locale).to_string(),
        }
    }

    /// Gets the output of the test case. Returns an empty string if there's no output.
    /// 获取测试用例的输出。如果没有输出，则返回空字符串。
    pub fn get_output(&self) -> String {
        match self {
            TestResult::Passed { output, .. } => output.clone(),
            TestResult::Failed { output, .. } => output.clone(),
            TestResult::Skipped => String::new(),
        }
    }

    /// Gets the features associated with the test case.
    /// 获取与测试用例关联的 features。
    pub fn get_features(&self) -> &str {
        match self {
            TestResult::Passed { case, .. } => &case.features,
            TestResult::Failed { case, .. } => &case.features,
            TestResult::Skipped => "",
        }
    }

    /// Gets the duration of the test case. Returns None if not applicable.
    /// 获取测试用例的持续时间。如果不适用，则返回 None。
    pub fn get_duration(&self) -> Option<Duration> {
        match self {
            TestResult::Passed { duration, .. } => Some(*duration),
            TestResult::Failed { duration, .. } => Some(*duration),
            TestResult::Skipped => None,
        }
    }

    /// Gets the number of attempts for a passed test. Returns 0 for other states.
    /// 获取通过测试的尝试次数。对于其他状态返回 0。
    pub fn get_retries(&self) -> u8 {
        match self {
            TestResult::Passed { retries, .. } => *retries,
            _ => 0,
        }
    }

    pub fn is_timeout(&self) -> bool {
        matches!(self, TestResult::Failed { reason, .. } if *reason == FailureReason::Timeout)
    }
}

impl fmt::Display for TestResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for TestResult {}

/// A context for a single build, managing its isolated temporary directory.
/// The temporary directory is automatically deleted when this struct is dropped,
/// ensuring cleanup.
/// 单个构建的上下文，管理其隔离的临时目录。
/// 当此结构体被丢弃时，临时目录会自动删除，以确保清理。
pub struct BuildContext {
    /// The `TempDir` guard. When this goes out of scope, the directory on disk is deleted.
    /// `TempDir` 的 guard。当它超出作用域时，磁盘上的目录将被删除。
    pub _temp_root: TempDir,
    /// The absolute path to the target directory within the temporary directory.
    /// This is where `cargo` will place build artifacts.
    /// 临时目录中 target 目录的绝对路径。
    /// `cargo` 会将构建产物放在这里。
    pub path: PathBuf,
}

impl BuildContext {
    pub fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let path = temp_dir.path().to_path_buf();
        Ok(Self {
            _temp_root: temp_dir,
            path,
        })
    }
}

impl fmt::Debug for BuildContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BuildContext")
         .field("target_path", &self.path)
         .finish_non_exhaustive()
    }
}

/// Represents a successfully built test case, ready to be executed.
/// Contains all necessary information to run the test, including the path
/// to the compiled executable and the build context for proper cleanup.
///
/// 表示成功构建的测试用例，准备执行。
/// 包含运行测试所需的所有信息，包括编译可执行文件的路径和用于正确清理的构建上下文。
#[derive(Debug)]
pub struct BuiltTest {
    /// The `TestCase` configuration that was built.
    /// 已构建的 `TestCase` 配置。
    pub case: TestCase,
    /// The path to the compiled test executable file.
    /// 指向已编译的测试可执行文件的路径。
    pub executable_path: PathBuf,
    /// Duration of the build process
    /// 构建过程的持续时间
    pub duration: Duration,
    /// The build context that holds the temporary directory.
    /// This must be kept alive until the test is finished.
    /// 保存临时目录的构建上下文。
    /// 必须保持活动状态直到测试结束。
    _build_ctx: BuildContext,
}

impl BuiltTest {
    /// Creates a new `BuiltTest`.
    pub fn new(
        case: TestCase,
        executable_path: PathBuf,
        duration: Duration,
        build_ctx: BuildContext,
    ) -> Self {
        Self {
            case,
            executable_path,
            duration,
            _build_ctx: build_ctx,
        }
    }

    /// Checks if the test case has an associated executable.
    pub fn is_empty(&self) -> bool {
        // This is a placeholder. A real implementation might check
        // if the executable_path is a dummy or special value.
        // For now, we assume a path is always valid if present.
        false
    }
}

/// Diagnostic information for cargo compiler messages.
/// 诊断信息，用于cargo编译器消息。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CargoDiagnostic {
    /// The severity level of the diagnostic (e.g., "error", "warning").
    /// 诊断的严重级别（例如 "error", "warning"）。
    pub level: String,
    /// The raw diagnostic message.
    /// 原始的诊断消息。
    pub message: String,
    /// The ANSI color-coded, formatted message, if available.
    /// 带有 ANSI 颜色代码的格式化消息（如果可用）。
    pub rendered: Option<String>,
}

/// A structured representation of a message from Cargo's JSON output format.
/// 来自Cargo的JSON输出格式的消息的结构化表示。
#[derive(Debug, Clone, Deserialize)]
pub struct CargoMessage {
    /// The reason for the message (e.g., "compiler-artifact", "compiler-message").
    /// 消息的原因（例如 "compiler-artifact", "compiler-message"）。
    pub reason: String,
    /// Information about the compilation target, present for artifact messages.
    /// 关于编译目标的信息，存在于产物消息中。
    pub target: Option<CargoTarget>,
    /// The path to the compiled executable, present for artifact messages.
    /// 指向已编译可执行文件的路径，存在于产物消息中。
    pub executable: Option<PathBuf>,
    /// The diagnostic message, present for compiler messages.
    /// 诊断消息，存在于编译器消息中。
    pub message: Option<CargoDiagnostic>,
    #[serde(default)]
    pub filenames: Vec<PathBuf>,
}

impl CargoMessage {
    /// Returns the message if it's an artifact message, otherwise None.
    /// 如果是产物消息则返回该消息，否则返回None。
    pub fn into_artifact(self) -> Option<Self> {
        if self.reason == "compiler-artifact" {
            Some(self)
        } else {
            None
        }
    }
}

/// Information about a compilation target from Cargo.
/// 来自Cargo的编译目标信息。
#[derive(Debug, Clone, Deserialize)]
pub struct CargoTarget {
    /// The name of the crate being compiled.
    /// 正在编译的 crate 的名称。
    pub name: String,
    /// `true` if the artifact is a test executable.
    /// 如果产物是测试可执行文件，则为 `true`。
    pub test: bool,
    pub kind: Vec<String>,
}

/// Package information from Cargo.toml
/// 来自Cargo.toml的包信息
#[derive(Debug, Clone, Deserialize)]
pub struct Package {
    pub name: String,
}

/// Cargo.toml manifest structure
/// Cargo.toml清单结构
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub package: Package,
} 
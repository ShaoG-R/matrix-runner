use crate::runner::config::TestCase;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

/// Enumerates the possible reasons for a test case failure.
/// This helps in categorizing errors for reporting and handling.
/// 枚举测试用例失败的可能原因。
/// 这有助于对错误进行分类，以便报告和处理。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum FailureReason {
    /// The test case failed during the `cargo build` or `cargo test --no-run` phase.
    /// 测试用例在 `cargo build` 或 `cargo test --no-run` 阶段失败。
    Build,
    /// The test case built successfully but failed when the test executable was run.
    /// 测试用例构建成功，但在运行测试可执行文件时失败。
    Test,
    /// The test case exceeded its configured timeout.
    /// 测试用例超出了其配置的超时时间。
    Timeout,
    /// A custom command defined in the test case failed.
    /// 测试用例中定义的自定义命令执行失败。
    CustomCommand,
}

/// Represents the final result of a single test case execution.
#[derive(Debug, Clone)]
pub enum TestResult {
    /// The test case passed successfully.
    Passed {
        case: TestCase,
        #[allow(dead_code)]
        output: String,
        duration: Duration,
        /// The number of attempts it took to pass the test (1 means it passed on the first try).
        retries: u8,
    },
    /// The test case failed.
    Failed {
        case: TestCase,
        output: String,
        reason: FailureReason,
        duration: Duration,
    },
    /// The test case was skipped.
    Skipped,
}

impl TestResult {
    /// Checks if a test result is a failure that was not explicitly allowed.
    /// A failure is "unexpected" if it's a `Failed` variant and the current OS
    /// is not in the test case's `allow_failure` list.
    pub fn is_unexpected_failure(&self) -> bool {
        match self {
            TestResult::Failed { case, reason, .. } => {
                // Timeouts are always unexpected failures.
                if *reason == FailureReason::Timeout {
                    return true;
                }
                !case
                    .allow_failure
                    .iter()
                    .any(|s| s == std::env::consts::OS)
            }
            _ => false,
        }
    }

    /// Gets the name of the test case. Returns "Skipped" for skipped tests.
    /// 获取测试用例的名称。对于跳过的测试，返回 "Skipped"。
    pub fn get_name(&self) -> &str {
        match self {
            TestResult::Passed { case, .. } => &case.name,
            TestResult::Failed { case, .. } => &case.name,
            TestResult::Skipped => "Skipped",
        }
    }

    /// Gets the status of the test result as a string for display.
    /// 以字符串形式获取测试结果的状态以供显示。
    pub fn get_status_str(&self) -> &str {
        match self {
            TestResult::Passed { .. } => "Passed",
            TestResult::Failed { case, reason, .. } => {
                if *reason == FailureReason::Timeout {
                    "Timeout"
                } else if case
                    .allow_failure
                    .iter()
                    .any(|s| s == std::env::consts::OS)
                {
                    "Allowed Failure"
                } else {
                    "Failed"
                }
            }
            TestResult::Skipped => "Skipped",
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
    /// 获取测试用E例的持续时间。如果不适用，则返回 None。
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
}

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
    pub target_path: PathBuf,
}

/// Represents a successfully built test case, ready to be executed.
/// It holds the necessary information to run the test, including the path
/// to the compiled executable and the build context (for cleanup).
/// 代表一个成功构建的、准备好执行的测试用例。
/// 它持有运行测试所需的信息，包括已编译可执行文件的路径和构建上下文（用于清理）。
pub struct BuiltTest {
    /// The `TestCase` configuration that was built.
    /// 已构建的 `TestCase` 配置。
    pub case: TestCase,
    /// The path to the compiled test executable file.
    /// 指向已编译的测试可执行文件的路径。
    pub executable: PathBuf,
    /// The build context, which manages the temporary directory for this test's artifacts.
    /// 构建上下文，管理此测试产物的临时目录。
    pub build_ctx: BuildContext,
}

/// Represents a single diagnostic message from the compiler, part of a `CargoMessage`.
/// This is used to parse JSON output from `cargo build`.
/// 代表来自编译器的单个诊断消息，是 `CargoMessage` 的一部分。
/// 用于解析来自 `cargo build` 的 JSON 输出。
#[derive(Debug, Clone, Deserialize)]
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

/// Represents a single message from `cargo build --message-format=json`.
/// These messages can be compiler artifacts, build scripts, or diagnostics.
/// 代表来自 `cargo build --message-format=json` 的单条消息。
/// 这些消息可以是编译器产物、构建脚本或诊断信息。
#[derive(Deserialize)]
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
}

/// Represents the "target" field within a `CargoMessage`, identifying the crate and type of artifact.
/// 代表 `CargoMessage` 中的 "target" 字段，用于标识 crate 和产物类型。
#[derive(Deserialize)]
pub struct CargoTarget {
    /// The name of the crate being compiled.
    /// 正在编译的 crate 的名称。
    pub name: String,
    /// `true` if the artifact is a test executable.
    /// 如果产物是测试可执行文件，则为 `true`。
    pub test: bool,
}

use crate::runner::config::TestCase;
use serde::Deserialize;
use std::path::PathBuf;
use tempfile::TempDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureReason {
    Build,
    Test,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub case: TestCase,
    pub output: String,
    pub success: bool,
    pub failure_reason: Option<FailureReason>,
}

/// A context for a build, managing the temporary directory.
pub struct BuildContext {
    /// The temporary directory that will be auto-deleted when this struct is dropped.
    pub _temp_root: TempDir,
    /// Path to the target directory for this build.
    pub target_path: PathBuf,
}

/// Holds the result of a successful build.
pub struct BuiltTest {
    pub case: TestCase,
    pub executable: PathBuf,
    pub build_ctx: BuildContext,
}

/// Represents a diagnostic message from the compiler.
#[derive(Debug, Clone, Deserialize)]
pub struct CargoDiagnostic {
    pub level: String,
    pub message: String,
    pub rendered: Option<String>,
}

/// Represents a message from `cargo build --message-format=json`.
#[derive(Deserialize)]
pub struct CargoMessage {
    pub reason: String,
    pub target: Option<CargoTarget>,
    pub executable: Option<PathBuf>,
    pub message: Option<CargoDiagnostic>,
}

/// Represents the "target" field in a `CargoMessage`.
#[derive(Deserialize)]
pub struct CargoTarget {
    pub name: String,
    pub test: bool,
}

use serde::{Deserialize, Serialize};

/// Represents a single test case defined in the test matrix configuration.
/// Each `TestCase` corresponds to a specific build and test configuration.
/// 代表测试矩阵配置中定义的单个测试用例。
/// 每个 `TestCase` 对应一个特定的构建和测试配置。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestCase {
    /// The unique name for the test case, used for identification in logs.
    /// 测试用例的唯一名称，用于在日志中进行识别。
    pub name: String,
    /// A string of comma-separated features to enable for this test case.
    /// 为此测试用例启用的一系列以逗号分隔的 features。
    pub features: String,
    /// If `true`, the `--no-default-features` flag will be used during the build.
    /// 如果为 `true`，则在构建期间将使用 `--no-default-features` 标志。
    pub no_default_features: bool,
    /// An optional custom command to run for this test case. If not provided,
    /// a default `cargo test` command will be constructed.
    /// 为此测试用例运行的可选自定义命令。如果未提供，
    /// 则会构建一个默认的 `cargo test` 命令。
    #[serde(default)]
    pub command: Option<String>,
    /// A list of operating systems (e.g., "windows", "linux") on which this
    /// test case is allowed to fail without causing the overall run to fail.
    /// 一个操作系统列表（例如 "windows", "linux"），在此列表中的系统上，
    /// 该测试用例允许失败，而不会导致整个运行失败。
    #[serde(default)]
    pub allow_failure: Vec<String>,
    /// A list of CPU architectures (e.g., "x86_64", "aarch64") on which this
    /// test case should be run. If empty, the case runs on all architectures.
    /// 一个 CPU 架构列表（例如 "x86_64", "aarch64"），此测试用例应在这些架构上运行。
    /// 如果为空，则该用例在所有架构上运行。
    #[serde(default)]
    pub arch: Vec<String>,
}

/// Represents the entire test matrix configuration, loaded from a TOML file.
/// It contains global settings and a list of all test cases.
/// 代表从 TOML 文件加载的整个测试矩阵配置。
/// 它包含全局设置和所有测试用例的列表。
#[derive(Debug, Deserialize, Serialize)]
pub struct TestMatrix {
    /// The language for the runner's output messages (e.g., "en", "zh-CN").
    /// Defaults to "en" if not specified.
    ///
    /// 运行器输出消息的语言（例如 "en", "zh-CN"）。
    /// 如果未指定，则默认为 "en"。
    #[serde(default = "default_language")]
    pub language: String,
    
    /// A vector containing all the test cases to be potentially executed.
    /// 一个包含所有可能被执行的测试用例的向量。
    pub cases: Vec<TestCase>,
}

fn default_language() -> String {
    "en".to_string()
}

use colored::*;
use std::fs;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

use crate::runner::command::spawn_and_capture;
use crate::runner::config::TestCase;
use crate::runner::i18n;
use crate::runner::models::CargoMessage;
use crate::runner::models::{BuiltTest, FailureReason, TestResult};
use crate::runner::utils::{copy_dir_all, create_build_dir};

/// Runs a complete test case, which involves two main phases: building and running.
/// This function orchestrates the entire lifecycle for a single test configuration.
/// If the build phase fails, the function will return early with a build failure.
///
/// # Arguments
/// * `case` - The `TestCase` configuration to run.
/// * `project_root` - The absolute path to the root of the project being tested.
/// * `crate_name` - The name of the crate being tested.
/// * `stop_token` - An optional `CancellationToken` to gracefully stop the execution.
///
/// # Returns
/// A `Result` containing either a successful `TestResult` or an error `TestResult`.
/// The `Err` variant is used to signal failure to the caller, which can then decide
/// whether to stop other parallel tests.
///
/// 运行一个完整的测试用例，包括两个主要阶段：构建和运行。
/// 此函数协调单个测试配置的整个生命周期。
/// 如果构建阶段失败，函数将提前返回一个构建失败结果。
///
/// # Arguments
/// * `case` - 要运行的 `TestCase` 配置。
/// * `project_root` - 被测试项目根目录的绝对路径。
/// * `crate_name` - 被测试的 crate 名称。
/// * `stop_token` - 一个可选的 `CancellationToken`，用于优雅地停止执行。
///
/// # Returns
/// 一个 `Result`，包含成功的 `TestResult` 或错误的 `TestResult`。
/// `Err` 变体用于向调用者发信号表示失败，调用者可以据此决定是否停止其他并行测试。
pub async fn run_test_case(
    case: TestCase,
    project_root: PathBuf,
    crate_name: String,
    stop_token: Option<CancellationToken>,
) -> Result<TestResult, TestResult> {
    // First, try to build the test case.
    // The `?` operator will propagate the `Err(TestResult)` if the build fails,
    // immediately stopping execution for this case.
    // 首先，尝试构建测试用例。
    // 如果构建失败，`?` 操作符将传播 `Err(TestResult)`，立即停止此用例的执行。
    let built_test = build_test_case(
        case.clone(),
        project_root,
        crate_name,
        stop_token.clone(),
    )
    .await?;

    // If the build is successful, run the compiled test executable.
    // 如果构建成功，则运行已编译的测试可执行文件。
    run_built_test(built_test, stop_token).await
}

/// Builds a single test case using `cargo test --no-run`.
/// It creates a temporary, isolated directory for the build artifacts to prevent
/// interference between parallel test runs.
///
/// # Returns
/// On success, returns a `BuiltTest` struct containing the path to the executable
/// and the build context. On failure, returns a `TestResult` with `FailureReason::Build`.
///
/// 使用 `cargo test --no-run` 构建单个测试用例。
/// 它为构建产物创建一个临时的、隔离的目录，以防止并行测试运行之间发生干扰。
///
/// # Returns
/// 成功时，返回一个 `BuiltTest` 结构体，其中包含可执行文件的路径和构建上下文。
/// 失败时，返回一个带有 `FailureReason::Build` 的 `TestResult`。
async fn build_test_case(
    case: TestCase,
    project_root: PathBuf,
    crate_name: String,
    stop_token: Option<CancellationToken>,
) -> Result<BuiltTest, TestResult> {
    println!(
        "{}",
        i18n::t_fmt("building_test", &[&case.name]).blue()
    );

    // Create a unique temporary directory for this build.
    // 为此构建创建一个唯一的临时目录。
    let build_ctx = create_build_dir(&crate_name, &case.features, case.no_default_features);

    // Set up the `cargo test --no-run` command.
    // 设置 `cargo test --no-run` 命令。
    let mut cmd = tokio::process::Command::new("cargo");
    cmd.kill_on_drop(true);
    cmd.current_dir(&project_root);
    cmd.arg("test")
        .arg("--lib")
        .arg("--no-run") // Build but don't run.
        .arg("--message-format=json-diagnostic-rendered-ansi")
        .arg("--locked") // Use the locked version of dependencies.
        .arg("--offline") // Run without accessing the network.
        .arg("--target-dir")
        .arg(&build_ctx.target_path);

    if case.no_default_features {
        cmd.arg("--no-default-features");
    }

    if !case.features.is_empty() {
        cmd.arg("--features").arg(&case.features);
    }

    // For logging purposes, construct a string representation of the command.
    // 为记录目的，构造命令的字符串表示。
    let command_string = format!("cargo test --lib --no-run --message-format=json-diagnostic-rendered-ansi --locked --offline --target-dir \"{}\" {} {}",
        build_ctx.target_path.display(),
        if case.no_default_features { "--no-default-features" } else { "" },
        if !case.features.is_empty() { format!("--features \"{}\"", case.features) } else { "".to_string() }
    ).split_whitespace().collect::<Vec<&str>>().join(" ");

    let (status_res, output, was_cancelled) = spawn_and_capture(cmd, stop_token).await;
    let status = status_res.expect(&i18n::t("error_waiting_process_complete"));

    if !status.success() {
        let failure_reason = if was_cancelled {
            FailureReason::Cancelled
        } else {
            FailureReason::Build
        };

        // Only preserve artifacts for genuine build failures, not cancellations.
        // If a build fails, copy the entire temporary target directory to `target-errors/`
        // for later inspection.
        // 仅为真正的构建失败保留产物，而非取消操作。
        // 如果构建失败，将整个临时 target 目录复制到 `target-errors/` 以供后续检查。
        if failure_reason == FailureReason::Build {
            let sanitized_name = case
                .name
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                .collect::<String>();
            let error_dir_path = project_root.join("target-errors").join(sanitized_name);

            println!(
                "{}\n  Command: {}",
                i18n::t_fmt(
                    "build_failed_preserving",
                    &[&case.name, &error_dir_path.display()]
                )
                .yellow(),
                command_string.cyan()
            );

            if error_dir_path.exists() {
                fs::remove_dir_all(&error_dir_path)
                    .expect(&i18n::t("cleanup_old_artifacts_failed"));
            }

            copy_dir_all(&build_ctx.target_path, &error_dir_path).unwrap_or_else(|e| {
                eprintln!(
                    "{}",
                    i18n::t_fmt("copy_artifacts_failed", &[&case.name, &e])
                )
            });
        }

        return Err(TestResult {
            case,
            output,
            success: false,
            failure_reason: Some(failure_reason),
        });
    }

    // Find the path to the compiled test executable from cargo's JSON output.
    // 从 cargo 的 JSON 输出中找到已编译测试可执行文件的路径。
    let executable = output
        .lines()
        .filter_map(|line| serde_json::from_str::<CargoMessage>(line).ok())
        .find_map(|msg| {
            if msg.reason == "compiler-artifact" {
                if let (Some(target), Some(executable_path)) = (msg.target, msg.executable) {
                    // Check if it's a test artifact for the main crate being tested.
                    // Cargo converts hyphens in crate names to underscores for library targets.
                    // 检查它是否是正在测试的主 crate 的测试产物。
                    // Cargo 会将 crate 名称中的连字符转换成下划线用于库目标。
                    if target.name == crate_name && target.test {
                        return Some(executable_path);
                    }
                }
            }
            None
        })
        .expect(&i18n::t("find_executable_failed"));

    println!(
        "{}",
        i18n::t_fmt("build_successful", &[&case.name]).green()
    );

    Ok(BuiltTest {
        case,
        executable,
        build_ctx,
    })
}

/// Runs a test executable that has already been built.
/// It captures the output and status of the executable. If the test fails,
/// its build artifacts are preserved for debugging.
///
/// # Returns
/// A `Result<TestResult, TestResult>` indicating the outcome. On success, the temporary
/// build directory is cleaned up. On failure, it is preserved.
///
/// 运行一个已经构建好的测试可执行文件。
/// 它会捕获可执行文件的输出和状态。如果测试失败，其构建产物将被保留以供调试。
///
/// # Returns
/// 一个 `Result<TestResult, TestResult>`，指示执行结果。成功时，临时构建目录会被清理。
/// 失败时，它将被保留。
async fn run_built_test(
    built_test: BuiltTest,
    stop_token: Option<CancellationToken>,
) -> Result<TestResult, TestResult> {
    let case = built_test.case;
    let executable = built_test.executable;
    // `build_ctx` is moved here. Its `_temp_root` field (a `TempDir`) will be
    // dropped at the end of this function's scope, cleaning up the directory,
    // unless the test fails and we preserve it.
    // `build_ctx` 被移动到这里。其 `_temp_root` 字段（一个 `TempDir`）将在此函数
    // 作用域结束时被 drop，从而清理目录，除非测试失败需要保留它。
    let build_ctx = built_test.build_ctx;
    let project_root = PathBuf::from("."); // Relative path for artifact preservation.

    let start_time = std::time::Instant::now();
    println!("{}", i18n::t_fmt("running_test", &[&case.name]).blue());

    let mut cmd = tokio::process::Command::new(&executable);
    cmd.kill_on_drop(true);
    let command_string = format!("{}", executable.display());

    let (status_res, output, was_cancelled) = spawn_and_capture(cmd, stop_token).await;
    let status = status_res.expect(&i18n::t("error_waiting_process_complete"));

    let duration = start_time.elapsed();

    println!(
        "{}",
        i18n::t_fmt(
            "finished_test",
            &[&case.name, &format_args!("{:.2?}", duration)]
        )
        .blue()
    );

    let success = status.success();
    let failure_reason = if success {
        None
    } else if was_cancelled {
        Some(FailureReason::Cancelled)
    } else {
        Some(FailureReason::Test)
    };

    let result = TestResult {
        case: case.clone(),
        output,
        success,
        failure_reason,
    };

    if !result.success {
        // Only preserve artifacts for genuine test failures, not cancellations.
        // 仅为真正的测试失败保留产物，而非取消操作。
        if result.failure_reason == Some(FailureReason::Test) {
            let sanitized_name = case
                .name
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                .collect::<String>();
            let error_dir_path = project_root.join("target-errors").join(sanitized_name);

            println!(
                "{}\n  Command: {}",
                i18n::t_fmt(
                    "test_failed_preserving",
                    &[&case.name, &error_dir_path.display()]
                )
                .yellow(),
                command_string.cyan()
            );

            if error_dir_path.exists() {
                fs::remove_dir_all(&error_dir_path)
                    .expect(&i18n::t("cleanup_old_artifacts_failed"));
            }

            // The build artifacts are in the temp dir managed by build_ctx.
            // We just need to copy them to the persistent `target-errors` directory.
            // 构建产物位于由 build_ctx 管理的临时目录中。
            // 我们只需将它们复制到持久的 `target-errors` 目录。
            copy_dir_all(&build_ctx.target_path, &error_dir_path).unwrap_or_else(|e| {
                eprintln!(
                    "{}",
                    i18n::t_fmt("copy_artifacts_failed", &[&case.name, &e])
                )
            });
        }

        Err(result)
    } else {
        // If the test was successful, the `build_ctx` is dropped here, and its
        // associated temporary directory is automatically removed from the disk.
        // 如果测试成功，`build_ctx` 会在这里被 drop，其关联的临时目录会
        // 自动从磁盘中删除。
        Ok(result)
    }
}

mod common;
use crate::common::setup_test_environment;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;

/// This test runs the `matrix-runner` against the `sample_project`
/// using the `success.toml` fixture. It asserts that the command
/// executes successfully (exit code 0) and that the final summary
/// reports overall success.
///
/// 这个测试使用 `success.toml` 配置针对 `sample_project` 运行 `matrix-runner`。
/// 它断言命令成功执行（退出码为 0），并且最终的摘要报告了总体成功。
#[test]
fn test_successful_run() {
    let temp_dir = setup_test_environment();
    let config_path = temp_dir.path().join("success.toml");
    fs::write(&config_path, r#"
language = "en"
cases = [
    { name = "test-success-case", features = "feature_test_success", no_default_features = false },
]
"#).unwrap();

    let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
    cmd.arg("run")
        .arg("--config")
        .arg(&config_path)
        .arg("--project-dir")
        .arg(temp_dir.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("All tests passed successfully!"));
}

/// This test checks the build failure scenario.
/// It asserts that the command fails (non-zero exit code) and that
/// the output contains keywords indicating a build failure.
///
/// 这个测试检查构建失败的场景。
/// 它断言命令失败（非零退出码），并且输出包含指示构建失败的关键字。
#[test]
fn test_build_failure() {
    let temp_dir = setup_test_environment();
    let config_path = temp_dir.path().join("build_fail.toml");
    fs::write(&config_path, r#"
language = "en"
cases = [
    { name = "build-failure-case", features = "feature_build_fail", no_default_features = false },
]
"#).unwrap();

    let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
    cmd.arg("run")
        .arg("--config")
        .arg(&config_path)
        .arg("--project-dir")
        .arg(temp_dir.path());
    
    let output = cmd.output().expect("Failed to run");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "Command unexpectedly succeeded");
    assert!(stderr.contains("Matrix tests failed with unexpected errors."), "stderr does not contain expected error message. stderr: {}", stderr);
}

/// This test checks the test failure scenario.
/// It asserts that the command fails and that the output contains
/// keywords indicating a test failure.
///
/// 这个测试检查测试失败的场景。
/// 它断言命令失败，并且输出包含指示测试失败的关键字。
#[test]
fn test_test_failure() {
    let temp_dir = setup_test_environment();
    let config_path = temp_dir.path().join("test_fail.toml");
    fs::write(&config_path, r#"
language = "en"
cases = [
    { name = "test-failure-case", features = "feature_test_fail", no_default_features = false },
]
"#).unwrap();

    let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
    cmd.arg("run")
        .arg("--config")
        .arg(&config_path)
        .arg("--project-dir")
        .arg(temp_dir.path());

    let output = cmd.output().expect("Failed to run");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "Command unexpectedly succeeded");
    assert!(stderr.contains("Matrix tests failed with unexpected errors."), "stderr does not contain expected error message. stderr: {}", stderr);
}

/// This test checks the custom command feature.
/// It runs a matrix with a command that just echoes a string.
///
/// 这个测试检查自定义命令功能。
/// 它运行一个矩阵，其中的命令只回显一个字符串。
#[test]
fn test_custom_command() {
    let temp_dir = setup_test_environment();
    let config_path = temp_dir.path().join("custom_command.toml");
    fs::write(&config_path, r#"
language = "en"
cases = [
    { name = "custom-command-case", features = "feature_custom_command", no_default_features = false },
]
"#).unwrap();

    let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
    cmd.arg("run")
        .arg("--config")
        .arg(&config_path)
        .arg("--project-dir")
        .arg(temp_dir.path());

    cmd.assert().success().stdout(predicate::str::contains(
        "Custom command executed successfully!",
    ));
}

/// This test checks the HTML report generation feature.
/// It runs a successful matrix and asserts that an HTML file is created.
///
/// 这个测试检查 HTML 报告生成功能。
/// 它运行一个成功的矩阵，并断言 HTML 文件被创建。
#[test]
fn test_html_report_generation() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = setup_test_environment();
    let config_path = temp_dir.path().join("success.toml");
    fs::write(&config_path, r#"
language = "en"
cases = [
    { name = "test-success-case", features = "feature_test_success", no_default_features = false },
]
"#).unwrap();

    let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
    cmd.arg("run")
        .arg("--config")
        .arg(&config_path)
        .arg("--project-dir")
        .arg(temp_dir.path())
        .arg("--html")
        .arg("report.html");

    // Get output instead of just asserting success, for better debugging.
    let output = cmd.output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Print stderr for debugging purposes, even on success.
    if !stderr.is_empty() {
        eprintln!("matrix-runner stderr:\n---\n{stderr}\n---");
    }

    // Assert that the command was successful and that it mentioned generating a report.
    assert!(
        output.status.success(),
        "Command did not run successfully. Stderr: {stderr}"
    );
    assert!(
        stdout.contains("Generating HTML report"),
        "Stdout does not confirm report generation."
    );

    assert!(
        fs::metadata("report.html").is_ok(),
        "HTML report file was not created"
    );
    let report_content = fs::read_to_string("report.html")?;
    assert!(
        report_content.contains("<title>Test Report</title>"),
        "HTML report content is invalid. Got:\n\n{}",
        report_content
    );

    // Cleanup the created report file
    fs::remove_file("report.html")?;

    Ok(())
}

/// This test checks the init command with default language.
/// It verifies that the command runs and creates a TestMatrix.toml file.
///
/// 这个测试检查默认语言的 init 命令。
/// 它验证命令运行并创建 TestMatrix.toml 文件。
#[test]
fn test_init_command_default() {
    let temp_dir = setup_test_environment();
    let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("init")
        .arg("--non-interactive")
        .arg("--lang")
        .arg("en");

    let output = cmd.output().expect("Failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(
        stdout.contains("Successfully created `TestMatrix.toml`"),
        "stdout does not contain expected success message. stdout: {}",
        stdout
    );
    assert!(temp_dir.path().join("TestMatrix.toml").exists());
}

/// This test checks the init command with specified language.
/// It verifies the language detection message and basic execution.
///
/// 这个测试检查指定语言的 init 命令。
/// 它验证语言检测消息和基本执行。
#[test]
fn test_init_command_with_language() {
    let temp_dir = setup_test_environment();
    let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("init")
        .arg("--non-interactive")
        .arg("--lang")
        .arg("zh-CN");

    let output = cmd.output().expect("Failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(
        stdout.contains("成功创建了 `TestMatrix.toml`"),
        "stdout does not contain expected success message in Chinese. stdout: {}",
        stdout
    );
    assert!(temp_dir.path().join("TestMatrix.toml").exists());
}

/// This test checks running with invalid arguments.
/// It asserts that the command fails with appropriate error message.
///
/// 这个测试检查使用无效参数运行。
/// 它断言命令失败并显示适当的错误消息。
#[test]
fn test_invalid_arguments() {
    let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
    cmd.arg("run").arg("--invalid-flag");

    cmd.assert().failure().stderr(predicate::str::contains(
        "error: unexpected argument '--invalid-flag' found",
    ));
}

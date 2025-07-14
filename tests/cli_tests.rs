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
    let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
    cmd.arg("run")
        .arg("--config")
        .arg("tests/fixtures/success.toml")
        .arg("--project-dir")
        .arg("tests/sample_project");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("TEST MATRIX PASSED SUCCESSFULLY"));
}

/// This test checks the build failure scenario.
/// It asserts that the command fails (non-zero exit code) and that
/// the output contains keywords indicating a build failure.
///
/// 这个测试检查构建失败的场景。
/// 它断言命令失败（非零退出码），并且输出包含指示构建失败的关键字。
#[test]
fn test_build_failure() {
    let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
    cmd.arg("run")
        .arg("--config")
        .arg("tests/fixtures/build_fail.toml")
        .arg("--project-dir")
        .arg("tests/sample_project");

    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("UNEXPECTED FAILURE DETECTED"))
        .stdout(predicate::str::contains("(Build Failure)"));
}

/// This test checks the test failure scenario.
/// It asserts that the command fails and that the output contains
/// keywords indicating a test failure.
///
/// 这个测试检查测试失败的场景。
/// 它断言命令失败，并且输出包含指示测试失败的关键字。
#[test]
fn test_test_failure() {
    let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
    cmd.arg("run")
        .arg("--config")
        .arg("tests/fixtures/test_fail.toml")
        .arg("--project-dir")
        .arg("tests/sample_project");

    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("UNEXPECTED FAILURE DETECTED"))
        .stdout(predicate::str::contains("(Test Failure)"));
}

/// This test checks the custom command feature.
/// It runs a matrix with a command that just echoes a string.
///
/// 这个测试检查自定义命令功能。
/// 它运行一个矩阵，其中的命令只回显一个字符串。
#[test]
fn test_custom_command() {
    let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
    cmd.arg("run")
        .arg("--config")
        .arg("tests/fixtures/custom_command.toml");

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
    // Instead of a temp dir, we'll just use a path relative to the test's CWD.
    // `assert_cmd` runs tests in a temporary directory per-test.
    let report_path = "report.html";

    let mut cmd = Command::cargo_bin("matrix-runner").unwrap();
    cmd.arg("run")
        .arg("--config")
        .arg("tests/fixtures/success.toml")
        .arg("--project-dir")
        .arg("tests/sample_project")
        .arg("--html")
        .arg(report_path);

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
        fs::metadata(report_path).is_ok(),
        "HTML report file was not created"
    );
    let report_content = fs::read_to_string(report_path)?;
    assert!(
        report_content.contains("<title>Test Matrix Report</title>"),
        "HTML report content is invalid"
    );

    // Cleanup the created report file
    fs::remove_file(report_path)?;

    Ok(())
}

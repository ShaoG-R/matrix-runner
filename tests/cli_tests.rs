use assert_cmd::prelude::*;
use predicates::prelude::*;
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
    cmd.arg("--config")
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
    cmd.arg("--config")
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
    cmd.arg("--config")
        .arg("tests/fixtures/test_fail.toml")
        .arg("--project-dir")
        .arg("tests/sample_project");

    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("UNEXPECTED FAILURE DETECTED"))
        .stdout(predicate::str::contains("(Test Failure)"));
} 
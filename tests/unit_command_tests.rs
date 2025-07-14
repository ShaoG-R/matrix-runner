//! # Command Module Unit Tests / Command 模块单元测试
//!
//! This module contains comprehensive unit tests for the `command.rs` module,
//! testing both the `format_build_error_output` and `spawn_and_capture` functions.
//!
//! 此模块包含 `command.rs` 模块的全面单元测试，
//! 测试 `format_build_error_output` 和 `spawn_and_capture` 函数。

use matrix_runner::runner::command::{format_build_error_output, spawn_and_capture};
use matrix_runner::runner::i18n;
use tokio::process::Command;

/// Initialize i18n for tests / 为测试初始化 i18n
fn setup_i18n() {
    i18n::init("en");
}

#[cfg(test)]
mod format_build_error_output_tests {
    use super::*;

    #[test]
    fn test_format_build_error_output_with_valid_json_error() {
        setup_i18n();
        
        // 模拟包含编译器错误的 JSON 输出
        let json_output = r#"{"reason":"compiler-message","package_id":"test 0.1.0 (path+file:///test)","manifest_path":"/test/Cargo.toml","target":{"kind":["lib"],"crate_types":["lib"],"name":"test","src_path":"/test/src/lib.rs","edition":"2021","doc":true,"doctest":true,"test":true},"message":{"message":"cannot find function `non_existent_function` in this scope","code":{"code":"E0425","explanation":"An unresolved name was used.\n\nErroneous code examples:\n\n```compile_fail,E0425\nfoo::bar(); // error: unresolved name `foo::bar`\n// or\nbaz(); // error: unresolved name `baz`\n```\n\nPlease verify that the name wasn't misspelled and ensure that the identifier\nis declared. Example:\n\n```\nfn bar() {}\nfoo::bar(); // ok!\n// or\nfn baz() {}\nbaz(); // ok!\n```\n\nShadowing and scoping rules can sometimes make it look like a path or a name\nis valid, but actually isn't. Example:\n\n```compile_fail,E0425\nstruct Foo;\nlet binding = Foo;\nlet another_binding = binding();\n// error: unresolved name `binding`\n```\n\nIn this example, `binding` is created as a variable, not a function. So when\nwe try to call it as a function, we get this error. Let's fix it:\n\n```\nstruct Foo;\nlet binding = Foo;\nlet another_binding = binding;\n// ok!\n```\n"},"level":"error","spans":[{"file_name":"src/lib.rs","byte_start":123,"byte_end":144,"line_start":10,"line_end":10,"column_start":5,"column_end":26,"is_primary":true,"text":[{"text":"    non_existent_function();","highlight_start":5,"highlight_end":26}],"label":"not found in this scope","suggested_replacement":null,"suggestion_applicability":null,"expansion":null}],"children":[],"rendered":"\u001b[0m\u001b[1m\u001b[38;5;9merror[E0425]\u001b[0m\u001b[0m\u001b[1m: cannot find function `non_existent_function` in this scope\u001b[0m\n\u001b[0m  \u001b[0m\u001b[0m\u001b[1m\u001b[38;5;12m--> \u001b[0m\u001b[0msrc/lib.rs:10:5\u001b[0m\n\u001b[0m   \u001b[0m\u001b[0m\u001b[1m\u001b[38;5;12m|\u001b[0m\n\u001b[0m\u001b[1m\u001b[38;5;12m10\u001b[0m\u001b[0m \u001b[0m\u001b[0m\u001b[1m\u001b[38;5;12m|\u001b[0m\u001b[0m \u001b[0m\u001b[0m    non_existent_function();\u001b[0m\n\u001b[0m   \u001b[0m\u001b[0m\u001b[1m\u001b[38;5;12m|\u001b[0m\u001b[0m \u001b[0m\u001b[0m\u001b[1m\u001b[38;5;9m    ^^^^^^^^^^^^^^^^^^^^^\u001b[0m\u001b[0m \u001b[0m\u001b[0m\u001b[1m\u001b[38;5;9mnot found in this scope\u001b[0m\n\n"}}"#;

        let result = format_build_error_output(json_output);
        
        // 应该返回带颜色的渲染输出
        assert!(result.contains("error[E0425]"));
        assert!(result.contains("cannot find function `non_existent_function` in this scope"));
        assert!(result.contains("src/lib.rs:10:5"));
    }

    #[test]
    fn test_format_build_error_output_with_multiple_errors() {
        setup_i18n();
        
        // 模拟包含多个编译器错误的 JSON 输出
        let json_output = r#"{"reason":"compiler-message","message":{"message":"first error","level":"error","rendered":"First Error Message"}}
{"reason":"compiler-message","message":{"message":"second error","level":"error","rendered":"Second Error Message"}}"#;

        let result = format_build_error_output(json_output);
        
        // 应该包含两个错误消息
        assert!(result.contains("First Error Message"));
        assert!(result.contains("Second Error Message"));
        assert!(result.contains("\n")); // 错误消息应该用换行符分隔
    }

    #[test]
    fn test_format_build_error_output_with_warnings_only() {
        setup_i18n();
        
        // 模拟只包含警告的 JSON 输出
        let json_output = r#"{"reason":"compiler-message","message":{"message":"unused variable","level":"warning","rendered":"Warning Message"}}"#;

        let result = format_build_error_output(json_output);
        
        // 应该返回原始输出的摘要，因为没有错误级别的消息
        assert!(result.contains("Could not parse specific compiler errors"));
        assert!(result.contains("unused variable"));
    }

    #[test]
    fn test_format_build_error_output_with_no_rendered_output() {
        setup_i18n();
        
        // 模拟没有 rendered 字段的错误消息
        let json_output = r#"{"reason":"compiler-message","message":{"message":"raw error message","level":"error"}}"#;

        let result = format_build_error_output(json_output);
        
        // 应该返回原始的错误消息
        assert_eq!(result, "raw error message");
    }

    #[test]
    fn test_format_build_error_output_with_invalid_json() {
        setup_i18n();
        
        // 模拟无效的 JSON 输出
        let invalid_json = "This is not valid JSON\nSome error occurred\nAnother line";

        let result = format_build_error_output(invalid_json);
        
        // 应该返回原始输出的摘要
        assert!(result.contains("Could not parse specific compiler errors"));
        assert!(result.contains("This is not valid JSON"));
        assert!(result.contains("Some error occurred"));
    }

    #[test]
    fn test_format_build_error_output_with_empty_input() {
        setup_i18n();
        
        let result = format_build_error_output("");
        
        // 应该返回解析失败的消息
        assert!(result.contains("Could not parse specific compiler errors"));
    }

    #[test]
    fn test_format_build_error_output_with_non_compiler_messages() {
        setup_i18n();
        
        // 模拟包含非编译器消息的 JSON 输出
        let json_output = r#"{"reason":"compiler-artifact","target":{"name":"test","kind":["bin"]}}
{"reason":"build-script-executed","package_id":"test 0.1.0"}"#;

        let result = format_build_error_output(json_output);
        
        // 应该返回原始输出的摘要，因为没有编译器错误消息
        assert!(result.contains("Could not parse specific compiler errors"));
    }

    #[test]
    fn test_format_build_error_output_truncates_long_output() {
        setup_i18n();
        
        // 创建超过 50 行的输出
        let long_output = (0..60)
            .map(|i| format!("Line {}", i))
            .collect::<Vec<_>>()
            .join("\n");

        let result = format_build_error_output(&long_output);
        
        // 应该只包含前 50 行
        assert!(result.contains("Line 0"));
        assert!(result.contains("Line 49"));
        assert!(!result.contains("Line 50"));
        assert!(!result.contains("Line 59"));
    }
}

#[cfg(test)]
mod spawn_and_capture_tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn_and_capture_successful_command() {
        // 测试成功执行的命令
        let mut cmd = Command::new("echo");
        cmd.arg("Hello, World!");

        let (status_result, output) = spawn_and_capture(cmd).await;
        
        assert!(status_result.is_ok());
        let status = status_result.unwrap();
        assert!(status.success());
        assert!(output.contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_spawn_and_capture_command_with_stderr() {
        // 在 Windows 上使用 PowerShell 命令来生成 stderr 输出
        #[cfg(target_os = "windows")]
        let cmd = {
            let mut cmd = Command::new("powershell");
            cmd.args(["-Command", "Write-Error 'Test error' -ErrorAction Continue; Write-Output 'Test output'"]);
            cmd
        };

        // 在 Unix 系统上使用 sh 命令
        #[cfg(not(target_os = "windows"))]
        let cmd = {
            let mut cmd = Command::new("sh");
            cmd.args(["-c", "echo 'Test output'; echo 'Test error' >&2"]);
            cmd
        };

        let (status_result, output) = spawn_and_capture(cmd).await;
        
        assert!(status_result.is_ok());
        // 输出应该包含 stdout 和 stderr 的内容
        assert!(output.contains("Test output") || output.contains("Test error"));
    }

    #[tokio::test]
    async fn test_spawn_and_capture_nonexistent_command() {
        // 测试不存在的命令
        let cmd = Command::new("this_command_does_not_exist_12345");

        let (status_result, output) = spawn_and_capture(cmd).await;
        
        // 应该返回错误
        assert!(status_result.is_err());
        // 输出应该为空
        assert!(output.is_empty());
    }

    #[tokio::test]
    async fn test_spawn_and_capture_failing_command() {
        // 测试失败的命令（非零退出码）
        #[cfg(target_os = "windows")]
        let cmd = {
            let mut cmd = Command::new("cmd");
            cmd.args(["/C", "exit 1"]);
            cmd
        };

        #[cfg(not(target_os = "windows"))]
        let cmd = {
            let mut cmd = Command::new("sh");
            cmd.args(["-c", "exit 1"]);
            cmd
        };

        let (status_result, _output) = spawn_and_capture(cmd).await;
        
        assert!(status_result.is_ok());
        let status = status_result.unwrap();
        assert!(!status.success());
        assert_eq!(status.code(), Some(1));
    }

    #[tokio::test]
    async fn test_spawn_and_capture_empty_output() {
        // 测试没有输出的命令
        #[cfg(target_os = "windows")]
        let cmd = {
            let mut cmd = Command::new("cmd");
            cmd.args(["/C", ""]);
            cmd
        };

        #[cfg(not(target_os = "windows"))]
        let cmd = {
            let mut cmd = Command::new("true");
            cmd
        };

        let (status_result, output) = spawn_and_capture(cmd).await;
        
        assert!(status_result.is_ok());
        let status = status_result.unwrap();
        assert!(status.success());
        // 输出可能为空或只包含换行符
        assert!(output.is_empty() || output.trim().is_empty());
    }
}

//! # Test Matrix Initialization Module / 测试矩阵初始化模块
//!
//! This module provides functionality for initializing a new test matrix configuration
//! through an interactive command-line wizard. It helps users create a `TestMatrix.toml`
//! file with common test case templates and configurations.
//!
//! 此模块通过交互式命令行向导提供初始化新测试矩阵配置的功能。
//! 它帮助用户创建带有常见测试用例模板和配置的 `TestMatrix.toml` 文件。
//!
//! ## Features / 功能特性
//!
//! - **Interactive Wizard**: Step-by-step guidance for configuration setup
//! - **Template Selection**: Pre-defined test case templates for common scenarios
//! - **Crate Detection**: Automatic detection of the current Rust crate name
//! - **Overwrite Protection**: Confirmation prompts before overwriting existing configurations
//!
//! - **交互式向导**: 配置设置的逐步指导
//! - **模板选择**: 常见场景的预定义测试用例模板
//! - **Crate 检测**: 自动检测当前 Rust crate 名称
//! - **覆盖保护**: 覆盖现有配置前的确认提示

use anyhow::{Context, Result};
use colored::*;
use dialoguer::{Confirm, Input, MultiSelect, theme::ColorfulTheme};
use serde::Deserialize;
use std::fs;
use std::path::Path;

use crate::runner::config::{TestCase, TestMatrix};
use crate::t;

/// Represents the package section of a Cargo.toml manifest file.
/// This struct is used for deserializing the package information from Cargo.toml
/// to automatically detect the crate name during initialization.
///
/// 代表 Cargo.toml 清单文件的 package 部分。
/// 此结构体用于从 Cargo.toml 反序列化包信息，
/// 以在初始化期间自动检测 crate 名称。
#[derive(Deserialize)]
struct Package {
    /// The name of the Rust crate / Rust crate 的名称
    name: String,
}

/// Represents the top-level structure of a Cargo.toml manifest file.
/// This struct is used for parsing Cargo.toml files to extract package information.
///
/// 代表 Cargo.toml 清单文件的顶级结构。
/// 此结构体用于解析 Cargo.toml 文件以提取包信息。
#[derive(Deserialize)]
struct Manifest {
    /// The package section containing crate metadata / 包含 crate 元数据的 package 部分
    package: Package,
}

/// Runs the interactive wizard to generate a `TestMatrix.toml` file.
///
/// This function provides a step-by-step guided process for creating a new test matrix
/// configuration file with user-selected templates for test cases.
///
/// 运行交互式向导以生成 `TestMatrix.toml` 文件。
///
/// 此函数提供逐步指导过程，用于创建带有用户选择的测试用例模板的新测试矩阵配置文件。
pub fn run_init_wizard(language: &str, non_interactive: bool) -> Result<()> {
    let config_path = Path::new("TestMatrix.toml");
    let theme = ColorfulTheme::default();

    if !non_interactive {
        println!("\n{}", t!("init_wizard_welcome", locale = language).cyan().bold());
        println!("{}", t!("init_wizard_description", locale = language));
    }

    if config_path.exists() && !non_interactive {
        let confirmation = Confirm::with_theme(&theme)
            .with_prompt(t!("init_overwrite_prompt", locale = language, path = config_path.to_str().unwrap()))
            .default(false)
            .interact()
            .context(t!("init_user_confirmation_failed", locale = language).to_string())?;
        if !confirmation {
            println!("{}", t!("init_aborted", locale = language));
            return Ok(());
        }
    }

    let default_matrix = generate_default_matrix()?;

    if non_interactive {
        write_config(config_path, &default_matrix, language)?;
        return Ok(());
    }

    // Interactive part starts here
    let crate_name = match detect_crate_name() {
        Ok(name) => {
            println!("{}", t!("init_detected_crate_name", locale = language, name = name.green()));
            name
        }
        Err(_) => String::new(),
    };

    let options = vec![
        ("default_features", t!("init_template_default_features", locale = language)),
        ("no_default_features", t!("init_template_no_default_features", locale = language)),
        ("all_features", t!("init_template_all_features", locale = language)),
        ("custom_command", t!("init_template_custom_command", locale = language)),
    ];

    let selections = MultiSelect::with_theme(&theme)
        .with_prompt(t!("init_case_selection_prompt", locale = language))
        .items(&options.iter().map(|o| o.1.clone()).collect::<Vec<_>>())
        .interact()
        .context(t!("init_user_confirmation_failed", locale = language).to_string())?;

    if selections.is_empty() {
        println!("{}", t!("init_no_cases_selected", locale = language).yellow());
    }

    let mut selected_cases = Vec::new();

    for i in selections {
        let selection_key = options[i].0;
        let case = match selection_key {
            "default_features" => TestCase {
                name: "default-features".to_string(),
                features: "".to_string(),
                no_default_features: false,
                command: None,
                allow_failure: vec![],
                arch: vec![],
                timeout_secs: Some(10),
                retries: None,
            },
            "no_default_features" => {
                let features = Input::with_theme(&theme)
                    .with_prompt(t!("init_no_std_features_prompt", locale = language))
                    .interact_text()?;
                TestCase {
                    name: "no-default-features".to_string(),
                    features,
                    no_default_features: true,
                    command: None,
                    allow_failure: vec![],
                    arch: vec![],
                    timeout_secs: Some(10),
                    retries: None,
                }
            },
            "all_features" => {
                 let features = Input::with_theme(&theme)
                    .with_prompt(t!("init_all_features_prompt", locale = language))
                    .interact_text()?;
                TestCase {
                    name: "all-features".to_string(),
                    features,
                    no_default_features: false,
                    command: None,
                    allow_failure: vec![],
                    arch: vec![],
                    timeout_secs: Some(10),
                    retries: None,
                }
            },
            "custom_command" => {
                let command = Input::with_theme(&theme)
                    .with_prompt(t!("init_custom_command_prompt", locale = language))
                    .interact_text()?;
                TestCase {
                    name: "custom-command".to_string(),
                    features: "".to_string(), // Ignored for custom commands / 自定义命令忽略此字段
                    no_default_features: false, // Ignored for custom commands / 自定义命令忽略此字段
                    command: Some(command),
                    allow_failure: vec![],
                    arch: vec![],
                    timeout_secs: Some(10),
                    retries: None,
                }
            },
            _ => continue,
        };
        selected_cases.push(case);
    }
    
    let final_matrix = if selected_cases.is_empty() {
        default_matrix
    } else {
        TestMatrix {
            language: language.to_string(),
            cases: selected_cases,
        }
    };
    
    write_config(config_path, &final_matrix, language)
}

fn generate_default_matrix() -> Result<TestMatrix> {
    Ok(TestMatrix {
        language: "en".to_string(),
        cases: vec![
            TestCase {
                name: "default-features".to_string(),
                features: "".to_string(),
                no_default_features: false,
                command: None,
                allow_failure: vec![],
                arch: vec![],
                timeout_secs: Some(10),
                retries: None,
            },
            TestCase {
                name: "no-default-features".to_string(),
                features: "".to_string(),
                no_default_features: true,
                command: None,
                allow_failure: vec![],
                arch: vec![],
                timeout_secs: Some(10),
                retries: None,
            },
        ],
    })
}


fn write_config(path: &Path, matrix: &TestMatrix, language: &str) -> Result<()> {
    let toml_string = toml::to_string_pretty(matrix)
        .context(t!("init_serialize_failed", locale = language).to_string())?;

    fs::write(path, toml_string)
        .with_context(|| t!("init_write_failed", locale = language, path = path.to_str().unwrap()))?;

    println!(
        "\n{} {}",
        "✔".green(),
        t!("init_success_created", locale = language, path = path.to_str().unwrap()).bold()
    );
    println!("{}", t!("init_usage_hint", locale = language));

    Ok(())
}

/// Tries to detect the crate name from the local `Cargo.toml`.
/// This function attempts to automatically determine the current Rust crate's name
/// by parsing the Cargo.toml file in the current directory.
///
/// # Returns / 返回值
/// * `Result<String>` - The detected crate name on success, or an error if detection fails
///                     成功时返回检测到的 crate 名称，失败时返回错误
///
/// # Error Conditions / 错误条件
/// - `Cargo.toml` file is not found in the current directory
/// - `Cargo.toml` file cannot be read (permissions, etc.)
/// - `Cargo.toml` file contains invalid TOML syntax
/// - `Cargo.toml` file is missing the required `[package]` section or `name` field
///
/// - 当前目录中未找到 `Cargo.toml` 文件
/// - 无法读取 `Cargo.toml` 文件（权限等）
/// - `Cargo.toml` 文件包含无效的 TOML 语法
/// - `Cargo.toml` 文件缺少必需的 `[package]` 部分或 `name` 字段
///
/// # Usage / 使用方法
/// This function is typically used during initialization to provide a sensible default
/// for test case names and configurations based on the current project.
///
/// 此函数通常在初始化期间使用，以基于当前项目为测试用例名称和配置提供合理的默认值。
///
/// 尝试从本地 `Cargo.toml` 检测 crate 名称。
/// 此函数通过解析当前目录中的 Cargo.toml 文件来自动确定当前 Rust crate 的名称。
fn detect_crate_name() -> Result<String> {
    let manifest_path = "Cargo.toml";
    let manifest_content = fs::read_to_string(manifest_path)
        .context(t!("init_cargo_toml_not_found", locale = "en").to_string())?;
    let manifest: Manifest = toml::from_str(&manifest_content)
        .context(t!("init_cargo_toml_parse_failed", locale = "en").to_string())?;
    Ok(manifest.package.name)
}


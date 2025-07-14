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
use crate::runner::i18n;
use crate::runner::i18n::I18nKey;

/// The default name for the test matrix configuration file.
/// This constant defines the standard filename used for test matrix configurations.
///
/// 测试矩阵配置文件的默认名称。
/// 此常量定义用于测试矩阵配置的标准文件名。
const CONFIG_FILE_NAME: &str = "TestMatrix.toml";

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
/// This function provides a step-by-step guided process for creating a new test matrix
/// configuration file with user-selected test case templates.
///
/// # Returns / 返回值
/// * `Result<()>` - Success if the configuration file was created successfully, or an error
///                 如果配置文件成功创建则返回成功，否则返回错误
///
/// # Process Flow / 处理流程
/// 1. Display welcome message and instructions / 显示欢迎消息和说明
/// 2. Check for existing configuration and confirm overwrite if needed / 检查现有配置并在需要时确认覆盖
/// 3. Detect the current crate name from Cargo.toml / 从 Cargo.toml 检测当前 crate 名称
/// 4. Prompt user to select test case templates / 提示用户选择测试用例模板
/// 5. Generate and save the configuration file / 生成并保存配置文件
/// 6. Display success message with next steps / 显示成功消息和后续步骤
///
/// # Error Handling / 错误处理
/// - Gracefully handles missing Cargo.toml files by using a default crate name
/// - Provides context for file I/O errors
/// - Allows user to abort the process at any time
///
/// - 通过使用默认 crate 名称优雅地处理缺失的 Cargo.toml 文件
/// - 为文件 I/O 错误提供上下文
/// - 允许用户随时中止过程
///
/// 运行交互式向导以生成 `TestMatrix.toml` 文件。
/// 此函数提供逐步指导过程，用于创建带有用户选择的测试用例模板的新测试矩阵配置文件。
pub fn run_init_wizard(language: &str) -> Result<()> {
    let theme = ColorfulTheme::default();
    println!("\n{}", i18n::t(I18nKey::InitWizardWelcome).bold().cyan());
    println!("{}\n", i18n::t(I18nKey::InitWizardDescription));

    // Check if configuration file already exists and get user confirmation
    // 检查配置文件是否已存在并获取用户确认
    if !confirm_overwrite(&theme)? {
        println!("{}", i18n::t(I18nKey::InitAborted).yellow());
        return Ok(());
    }

    // Attempt to detect the crate name, fallback to default if detection fails
    // 尝试检测 crate 名称，如果检测失败则回退到默认值
    let detected_crate_name = detect_crate_name().unwrap_or_else(|_| "my-crate".to_string());
    let formatted_message = i18n::t_fmt(I18nKey::InitDetectedCrateName, &[&detected_crate_name]);
    println!("{}", formatted_message);

    // Prompt user to select and configure test cases
    // 提示用户选择和配置测试用例
    let cases = prompt_for_cases(&theme)?;

    // Create the test matrix configuration
    // 创建测试矩阵配置
    let config = TestMatrix {
        language: language.to_string(),
        cases,
    };

    // Serialize the configuration to TOML format
    // 将配置序列化为 TOML 格式
    let toml_string = toml::to_string_pretty(&config)
        .context(i18n::t(I18nKey::InitSerializeFailed).to_string())?;

    // Write the configuration to file
    // 将配置写入文件
    fs::write(CONFIG_FILE_NAME, toml_string)
        .with_context(|| i18n::t_fmt(I18nKey::InitWriteFailed, &[&CONFIG_FILE_NAME]))?;

    // Display success message and next steps
    // 显示成功消息和后续步骤
    println!(
        "\n{} {}",
        "✔".green(),
        i18n::t_fmt(I18nKey::InitSuccessCreated, &[&CONFIG_FILE_NAME]).bold()
    );
    println!("{}", i18n::t(I18nKey::InitUsageHint));

    Ok(())
}

/// Checks if `TestMatrix.toml` exists and asks the user for confirmation to overwrite.
/// This function provides a safety mechanism to prevent accidental overwriting of existing
/// configuration files by prompting the user for explicit confirmation.
///
/// # Arguments / 参数
/// * `theme` - The dialoguer theme to use for consistent styling of prompts
///            用于提示一致样式的 dialoguer 主题
///
/// # Returns / 返回值
/// * `Result<bool>` - `true` if the user confirms overwrite or if no file exists,
///                   `false` if the user chooses not to overwrite
///                   如果用户确认覆盖或文件不存在则返回 `true`，
///                   如果用户选择不覆盖则返回 `false`
///
/// # Behavior / 行为
/// - If no configuration file exists, returns `true` immediately
/// - If a configuration file exists, prompts the user for confirmation
/// - Uses the provided theme for consistent UI styling
///
/// - 如果不存在配置文件，立即返回 `true`
/// - 如果存在配置文件，提示用户确认
/// - 使用提供的主题保持 UI 样式一致
///
/// 检查 `TestMatrix.toml` 是否存在并询问用户确认覆盖。
/// 此函数提供安全机制，通过提示用户明确确认来防止意外覆盖现有配置文件。
fn confirm_overwrite(theme: &ColorfulTheme) -> Result<bool> {
    if Path::new(CONFIG_FILE_NAME).exists() {
        Confirm::with_theme(theme)
            .with_prompt(i18n::t_fmt(
                I18nKey::InitOverwritePrompt,
                &[&CONFIG_FILE_NAME],
            ))
            .interact()
            .context(i18n::t(I18nKey::InitUserConfirmationFailed).to_string())
    } else {
        Ok(true)
    }
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
        .context(i18n::t(I18nKey::InitCargoTomlNotFound).to_string())?;
    let manifest: Manifest = toml::from_str(&manifest_content)
        .context(i18n::t(I18nKey::InitCargoTomlParseFailed).to_string())?;
    Ok(manifest.package.name)
}

/// Prompts the user to select and configure common test cases.
/// This function presents a multi-selection interface for choosing from predefined
/// test case templates and then configures each selected template with user input.
///
/// # Arguments / 参数
/// * `theme` - The dialoguer theme to use for consistent styling of prompts
///            用于提示一致样式的 dialoguer 主题
///
/// # Returns / 返回值
/// * `Result<Vec<TestCase>>` - A vector of configured test cases based on user selections
///                            基于用户选择的已配置测试用例向量
///
/// # Available Templates / 可用模板
/// 1. **Default features on stable Rust**: Basic test case with default feature set
/// 2. **No default features (`no_std` setup)**: Test case for `no_std` environments
/// 3. **All features enabled**: Test case with all available features
/// 4. **Custom command**: Test case with user-defined command (e.g., MIRI, wasm-pack)
///
/// 1. **稳定 Rust 上的默认功能**: 具有默认功能集的基本测试用例
/// 2. **无默认功能（`no_std` 设置）**: 用于 `no_std` 环境的测试用例
/// 3. **启用所有功能**: 具有所有可用功能的测试用例
/// 4. **自定义命令**: 具有用户定义命令的测试用例（例如 MIRI、wasm-pack）
///
/// # Interactive Process / 交互过程
/// - Displays a multi-select menu with pre-selected common options
/// - Allows users to customize feature sets for selected templates
/// - Prompts for custom commands when the custom command template is selected
/// - Provides helpful defaults and guidance throughout the process
///
/// - 显示带有预选常见选项的多选菜单
/// - 允许用户为选定模板自定义功能集
/// - 在选择自定义命令模板时提示输入自定义命令
/// - 在整个过程中提供有用的默认值和指导
///
/// 提示用户选择和配置常见测试用例。
/// 此函数提供多选界面，用于从预定义的测试用例模板中选择，然后根据用户输入配置每个选定的模板。
fn prompt_for_cases(theme: &ColorfulTheme) -> Result<Vec<TestCase>> {
    let mut cases = Vec::new();

    // Define available test case templates
    // 定义可用的测试用例模板
    let case_templates = vec![
        i18n::t(I18nKey::InitTemplateDefaultFeatures),
        i18n::t(I18nKey::InitTemplateNoDefaultFeatures),
        i18n::t(I18nKey::InitTemplateAllFeatures),
        i18n::t(I18nKey::InitTemplateCustomCommand),
    ];

    // Present multi-selection interface to user
    // 向用户展示多选界面
    let selections = MultiSelect::with_theme(theme)
        .with_prompt(i18n::t(I18nKey::InitCaseSelectionPrompt))
        .items(&case_templates)
        .defaults(&[true, true, false, false]) // Pre-select first two / 预选前两个
        .interact()?;

    if selections.is_empty() {
        println!("{}", i18n::t(I18nKey::InitNoCasesSelected).yellow());
    }

    // Configure test cases based on user selections
    // 根据用户选择配置测试用例

    // Template 0: Default features on stable Rust
    // 模板 0: 稳定 Rust 上的默认功能
    if selections.contains(&0) {
        cases.push(TestCase {
            name: "stable-default".to_string(),
            features: "".to_string(),
            no_default_features: false,
            command: None,
            allow_failure: vec![],
            arch: vec![],
        });
    }

    // Template 1: No default features (`no_std` setup)
    // 模板 1: 无默认功能（`no_std` 设置）
    if selections.contains(&1) {
        cases.push(TestCase {
            name: "stable-no-default-features".to_string(),
            features: Input::with_theme(theme)
                .with_prompt(i18n::t(I18nKey::InitNoStdFeaturesPrompt))
                .default("".into())
                .interact_text()?,
            no_default_features: true,
            command: None,
            allow_failure: vec![],
            arch: vec![],
        });
    }

    // Template 2: All features enabled
    // 模板 2: 启用所有功能
    if selections.contains(&2) {
        cases.push(TestCase {
            name: "stable-all-features".to_string(),
            features: Input::with_theme(theme)
                .with_prompt(i18n::t(I18nKey::InitAllFeaturesPrompt))
                .interact_text()?,
            no_default_features: false,
            command: None,
            allow_failure: vec![],
            arch: vec![],
        });
    }

    // Template 3: Custom command (e.g., for MIRI or wasm-pack)
    // 模板 3: 自定义命令（例如用于 MIRI 或 wasm-pack）
    if selections.contains(&3) {
        let cmd_input = Input::with_theme(theme)
            .with_prompt(i18n::t(I18nKey::InitCustomCommandPrompt))
            .default("cargo miri test".into())
            .interact_text()?;
        cases.push(TestCase {
            name: "custom-command".to_string(),
            features: "".to_string(), // Ignored for custom commands / 自定义命令忽略此字段
            no_default_features: false, // Ignored for custom commands / 自定义命令忽略此字段
            command: Some(cmd_input),
            allow_failure: vec![],
            arch: vec![],
        });
    }

    Ok(cases)
}

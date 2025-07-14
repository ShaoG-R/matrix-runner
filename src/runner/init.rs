
use anyhow::{Context, Result};
use colored::*;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect};
use serde::Deserialize;
use std::fs;
use std::path::Path;

use crate::runner::config::{TestCase, TestMatrix};

const CONFIG_FILE_NAME: &str = "TestMatrix.toml";

#[derive(Deserialize)]
struct Package {
    name: String,
}

#[derive(Deserialize)]
struct Manifest {
    package: Package,
}

/// Runs the interactive wizard to generate a `TestMatrix.toml` file.
pub fn run_init_wizard() -> Result<()> {
    let theme = ColorfulTheme::default();
    println!(
        "\n{}",
        "Welcome to the matrix-runner setup wizard!".bold().cyan()
    );
    println!("This will help you create a `TestMatrix.toml` file to configure your tests.\n");

    if !confirm_overwrite(&theme)? {
        println!("{}", "Aborted.".yellow());
        return Ok(());
    }

    let detected_crate_name = detect_crate_name().unwrap_or_else(|_| "my-crate".to_string());
    println!(
        "Detected crate name: {}",
        detected_crate_name.clone().green()
    );

    let cases = prompt_for_cases(&theme)?;

    let config = TestMatrix {
        language: "en".to_string(),
        cases,
    };

    let toml_string = toml::to_string_pretty(&config)
        .context("Failed to serialize configuration to TOML.")?;

    fs::write(CONFIG_FILE_NAME, toml_string)
        .with_context(|| format!("Failed to write to {}", CONFIG_FILE_NAME))?;

    println!(
        "\n{} {}",
        "✔".green(),
        format!("Successfully created `{}`.", CONFIG_FILE_NAME).bold()
    );
    println!("You can now run `matrix-runner` to execute your test matrix.");

    Ok(())
}

/// Checks if `TestMatrix.toml` exists and asks the user for confirmation to overwrite.
fn confirm_overwrite(theme: &ColorfulTheme) -> Result<bool> {
    if Path::new(CONFIG_FILE_NAME).exists() {
        Confirm::with_theme(theme)
            .with_prompt(format!(
                "`{}` already exists. Do you want to overwrite it?",
                CONFIG_FILE_NAME
            ))
            .interact()
            .context("Failed to get user confirmation.")
    } else {
        Ok(true)
    }
}

/// Tries to detect the crate name from the local `Cargo.toml`.
fn detect_crate_name() -> Result<String> {
    let manifest_path = "Cargo.toml";
    let manifest_content =
        fs::read_to_string(manifest_path).context("Could not find or read Cargo.toml.")?;
    let manifest: Manifest =
        toml::from_str(&manifest_content).context("Failed to parse Cargo.toml.")?;
    Ok(manifest.package.name)
}

/// Prompts the user to select and configure common test cases.
fn prompt_for_cases(theme: &ColorfulTheme) -> Result<Vec<TestCase>> {
    let mut cases = Vec::new();

    let case_templates = &[
        "Default features on stable Rust",
        "No default features (`no_std` setup)",
        "All features enabled",
        "A custom command (e.g., for MIRI or wasm-pack)",
    ];

    let selections = MultiSelect::with_theme(theme)
        .with_prompt("Choose which test cases to generate (use space to select, enter to confirm)")
        .items(&case_templates[..])
        .defaults(&[true, true, false, false]) // Pre-select first two
        .interact()?;

    if selections.is_empty() {
        println!("{}", "No test cases selected. Your config file will be minimal.".yellow());
    }

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

    if selections.contains(&1) {
        cases.push(TestCase {
            name: "stable-no-default-features".to_string(),
            features: Input::with_theme(theme)
                .with_prompt("Enter comma-separated features for the `no_std` case (if any)")
                .default("".into())
                .interact_text()?,
            no_default_features: true,
            command: None,
            allow_failure: vec![],
            arch: vec![],
        });
    }

    if selections.contains(&2) {
        cases.push(TestCase {
            name: "stable-all-features".to_string(),
            features: Input::with_theme(theme)
                .with_prompt("Enter comma-separated list of ALL your crate's features")
                .interact_text()?,
            no_default_features: false,
            command: None,
            allow_failure: vec![],
            arch: vec![],
        });
    }

    if selections.contains(&3) {
        let cmd_input = Input::with_theme(theme)
            .with_prompt("Enter the custom command to run")
            .default("cargo miri test".into())
            .interact_text()?;
        cases.push(TestCase {
            name: "custom-command".to_string(),
            features: "".to_string(), // Ignored
            no_default_features: false, // Ignored
            command: Some(cmd_input),
            allow_failure: vec![],
            arch: vec![],
        });
    }

    Ok(cases)
} 
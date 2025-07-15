// Shared test helpers for integration tests
use std::fs;
use std::path::PathBuf;
use tempfile::{tempdir, TempDir};

pub fn setup_test_environment() -> TempDir {
    let temp_dir = tempdir().expect("Failed to create temporary directory");
    let sample_project_path = temp_dir.path();
    let src_path = sample_project_path.join("src");
    fs::create_dir_all(&src_path).expect("Failed to create src directory");

    let cargo_toml_content = r#"[package]
name = "sample_project"
version = "0.1.0"
edition = "2021"

[lib]
name = "sample_project"
path = "src/lib.rs"

[features]
feature_build_fail = []
feature_test_fail = []
"#;
    fs::write(sample_project_path.join("Cargo.toml"), cargo_toml_content).expect("Failed to write Cargo.toml");

    let lib_rs_content = r#"
#[cfg(feature = "feature_build_fail")]
compile_error!("This is a deliberate build failure for testing purposes.");

#[cfg(feature = "feature_test_fail")]
#[test]
fn it_fails() {
    panic!("This is a deliberate test failure for testing purposes.");
}
"#;
    fs::write(src_path.join("lib.rs"), lib_rs_content).expect("Failed to write lib.rs");

    temp_dir
}

/// Helper function to create an invalid TOML configuration
pub fn create_invalid_toml(temp_dir: &TempDir) -> PathBuf {
    let matrix_path = temp_dir.path().join("invalid.toml");
    let content = r#"
language = "en"
# Invalid TOML - missing closing bracket
[[cases]
name = "invalid-case"
features = ""
no_default_features = false
"#;
    fs::write(&matrix_path, content).unwrap();
    matrix_path
}

/// Helper function to create a configuration with missing required fields
pub fn create_incomplete_config(temp_dir: &TempDir) -> PathBuf {
    let matrix_path = temp_dir.path().join("incomplete.toml");
    let content = r#"
language = "en"

[[cases]]
name = "incomplete-case"
# Missing required fields: features, no_default_features
"#;
    fs::write(&matrix_path, content).unwrap();
    matrix_path
}

/// Helper function to create a configuration with invalid commands
pub fn create_invalid_command_config(temp_dir: &TempDir) -> PathBuf {
    let matrix_path = temp_dir.path().join("invalid_command.toml");
    let content = r#"
language = "en"

[[cases]]
name = "invalid-command-case"
command = "this_command_definitely_does_not_exist_12345"
features = ""
no_default_features = false
"#;
    fs::write(&matrix_path, content).unwrap();
    matrix_path
}

/// Helper function to create a configuration with architecture filtering
pub fn create_arch_filtered_config(temp_dir: &TempDir) -> PathBuf {
    let matrix_path = temp_dir.path().join("arch_filtered.toml");
    let content = r#"
language = "en"

[[cases]]
name = "unsupported-arch-case"
features = ""
no_default_features = false
arch = ["nonexistent_architecture"]

[[cases]]
name = "supported-arch-case"
features = ""
no_default_features = false
arch = ["x86_64", "aarch64"]
"#;
    fs::write(&matrix_path, content).unwrap();
    matrix_path
}

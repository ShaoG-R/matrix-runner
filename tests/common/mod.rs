// Shared test helpers for integration tests
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

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

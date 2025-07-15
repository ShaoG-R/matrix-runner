use anyhow::{Context, Result};
use fs_extra::dir::{copy, CopyOptions};
use std::fs;
use std::path::Path;
use tempfile::{tempdir, TempDir};

/// Creates a unique, temporary build directory for a test case.
pub fn create_build_dir(project_root: &Path, case_name: &str) -> Result<TempDir> {
    let sanitized_name = case_name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>();
    let temp_dir_name = format!("matrix_runner_{}", sanitized_name);
    let target_dir = project_root.join("target");
    let temp_dir_path = target_dir.join(&temp_dir_name);

    // Clean up old directory if it exists, to ensure a fresh build environment.
    if temp_dir_path.exists() {
        fs::remove_dir_all(&temp_dir_path).with_context(|| {
            format!(
                "Failed to clean up old build directory: {}",
                temp_dir_path.display()
            )
        })?;
    }

    tempdir()
        .or_else(|_| tempdir_in(target_dir))
        .with_context(|| "Failed to create temporary build directory".to_string())
}

/// A wrapper around `tempfile::tempdir_in` to provide more context on failure.
fn tempdir_in<P: AsRef<Path>>(dir: P) -> std::io::Result<TempDir> {
    tempfile::Builder::new()
        .prefix("matrix_runner_")
        .tempdir_in(dir)
}

/// Copies the entire content of a source directory to a destination directory.
pub fn copy_dir_all(from: &Path, to: &Path) -> Result<()> {
    let mut options = CopyOptions::new();
    options.overwrite = true;
    options.copy_inside = true;
    copy(from, to, &options)?;
    Ok(())
}

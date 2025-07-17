//! # File System Operations Module / 文件系统操作模块
//!
//! This module provides utilities for file system operations,
//! such as creating temporary build directories and copying files.
//!
//! 此模块提供文件系统操作的实用功能，
//! 如创建临时构建目录和复制文件。

use anyhow::{Context, Result};
use fs_extra::dir::{copy, CopyOptions};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

/// Creates a unique, temporary build directory for a test case.
///
/// # Arguments
/// * `project_root` - Path to the project root directory
/// * `case_name` - Name of the test case, used to create a unique directory name
///
/// # Returns
/// A `BuildContext` containing the temporary directory information
pub fn create_build_dir(project_root: &Path, case_name: &str) -> Result<(PathBuf, TempDir)> {
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

    let temp_dir = tempdir()
        .or_else(|_| tempdir_in(target_dir))
        .with_context(|| "Failed to create temporary build directory".to_string())?;
        
    let path = temp_dir.path().to_path_buf();
    
    Ok((path, temp_dir))
}

/// A wrapper around `tempfile::tempdir_in` to provide more context on failure.
fn tempdir_in<P: AsRef<Path>>(dir: P) -> std::io::Result<TempDir> {
    tempfile::Builder::new()
        .prefix("matrix_runner_")
        .tempdir_in(dir)
}

/// Copies the entire content of a source directory to a destination directory.
///
/// # Arguments
/// * `from` - Source directory path
/// * `to` - Destination directory path
///
/// # Returns
/// A `Result` indicating success or failure
pub fn copy_dir_all(from: &Path, to: &Path) -> Result<()> {
    let mut options = CopyOptions::new();
    options.overwrite = true;
    options.copy_inside = true;
    copy(from, to, &options)?;
    Ok(())
}

/// Checks if a path exists and is a directory.
///
/// # Arguments
/// * `path` - Path to check
///
/// # Returns
/// `true` if the path exists and is a directory, `false` otherwise
pub fn is_directory(path: &Path) -> bool {
    path.exists() && path.is_dir()
}

/// Gets the absolute path from a potentially relative path.
///
/// # Arguments
/// * `path` - Path to canonicalize
///
/// # Returns
/// Canonicalized absolute path, or an error if the path doesn't exist
pub fn absolute_path(path: &Path) -> Result<PathBuf> {
    fs::canonicalize(path).with_context(|| format!("Failed to resolve path: {}", path.display()))
} 
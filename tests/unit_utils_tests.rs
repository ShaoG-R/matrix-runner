//! # Utils Module Unit Tests / Utils 模块单元测试
//!
//! This module contains comprehensive unit tests for the `utils.rs` module,
//! testing both the `copy_dir_all` and `create_build_dir` functions.
//!
//! 此模块包含 `utils.rs` 模块的全面单元测试，
//! 测试 `copy_dir_all` 和 `create_build_dir` 函数。

use matrix_runner::runner::i18n;
use matrix_runner::runner::utils::{copy_dir_all, create_build_dir};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Initialize i18n for tests / 为测试初始化 i18n
fn setup_i18n() {
    i18n::init("en");
}

/// Helper function to create a test directory structure
/// 创建测试目录结构的辅助函数
fn create_test_dir_structure(base_path: &Path) -> std::io::Result<()> {
    // Create directory structure:
    // base_path/
    // ├── file1.txt
    // ├── file2.txt
    // └── subdir/
    //     ├── file3.txt
    //     └── nested/
    //         └── file4.txt

    fs::create_dir_all(base_path.join("subdir").join("nested"))?;

    fs::write(base_path.join("file1.txt"), "content1")?;
    fs::write(base_path.join("file2.txt"), "content2")?;
    fs::write(base_path.join("subdir").join("file3.txt"), "content3")?;
    fs::write(
        base_path.join("subdir").join("nested").join("file4.txt"),
        "content4",
    )?;

    Ok(())
}

/// Helper function to verify directory structure
/// 验证目录结构的辅助函数
fn verify_dir_structure(base_path: &Path) -> std::io::Result<bool> {
    let file1_exists = base_path.join("file1.txt").exists();
    let file2_exists = base_path.join("file2.txt").exists();
    let file3_exists = base_path.join("subdir").join("file3.txt").exists();
    let file4_exists = base_path
        .join("subdir")
        .join("nested")
        .join("file4.txt")
        .exists();

    if !file1_exists || !file2_exists || !file3_exists || !file4_exists {
        return Ok(false);
    }

    // Verify file contents
    let content1 = fs::read_to_string(base_path.join("file1.txt"))?;
    let content2 = fs::read_to_string(base_path.join("file2.txt"))?;
    let content3 = fs::read_to_string(base_path.join("subdir").join("file3.txt"))?;
    let content4 = fs::read_to_string(base_path.join("subdir").join("nested").join("file4.txt"))?;

    Ok(content1 == "content1"
        && content2 == "content2"
        && content3 == "content3"
        && content4 == "content4")
}

#[cfg(test)]
mod copy_dir_all_tests {
    use super::*;

    #[test]
    fn test_copy_dir_all_successful_copy() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        let dst_dir = temp_dir.path().join("dst");

        // Create source directory structure
        create_test_dir_structure(&src_dir).unwrap();

        // Copy directory
        let result = copy_dir_all(&src_dir, &dst_dir);

        assert!(result.is_ok());
        assert!(verify_dir_structure(&dst_dir).unwrap());
    }

    #[test]
    fn test_copy_dir_all_creates_destination_directory() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        let dst_dir = temp_dir.path().join("non_existent").join("dst");

        // Create source directory with a simple file
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("test.txt"), "test content").unwrap();

        // Destination directory doesn't exist yet
        assert!(!dst_dir.exists());

        // Copy directory
        let result = copy_dir_all(&src_dir, &dst_dir);

        assert!(result.is_ok());
        assert!(dst_dir.exists());
        assert!(dst_dir.join("test.txt").exists());

        let content = fs::read_to_string(dst_dir.join("test.txt")).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_copy_dir_all_overwrites_existing_files() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        let dst_dir = temp_dir.path().join("dst");

        // Create source directory
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("test.txt"), "new content").unwrap();

        // Create destination directory with existing file
        fs::create_dir_all(&dst_dir).unwrap();
        fs::write(dst_dir.join("test.txt"), "old content").unwrap();

        // Copy directory (should overwrite)
        let result = copy_dir_all(&src_dir, &dst_dir);

        assert!(result.is_ok());

        let content = fs::read_to_string(dst_dir.join("test.txt")).unwrap();
        assert_eq!(content, "new content");
    }

    #[test]
    fn test_copy_dir_all_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("empty_src");
        let dst_dir = temp_dir.path().join("empty_dst");

        // Create empty source directory
        fs::create_dir_all(&src_dir).unwrap();

        // Copy empty directory
        let result = copy_dir_all(&src_dir, &dst_dir);

        assert!(result.is_ok());
        assert!(dst_dir.exists());
        assert!(dst_dir.is_dir());

        // Verify destination is empty
        let entries: Vec<_> = fs::read_dir(&dst_dir).unwrap().collect();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_copy_dir_all_nonexistent_source() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("nonexistent");
        let dst_dir = temp_dir.path().join("dst");

        // Try to copy from non-existent source
        let result = copy_dir_all(&src_dir, &dst_dir);

        assert!(result.is_err());
    }

    #[test]
    fn test_copy_dir_all_preserves_directory_structure() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        let dst_dir = temp_dir.path().join("dst");

        // Create complex directory structure
        fs::create_dir_all(src_dir.join("a").join("b").join("c")).unwrap();
        fs::create_dir_all(src_dir.join("x").join("y")).unwrap();
        fs::write(src_dir.join("a").join("file_a.txt"), "a").unwrap();
        fs::write(src_dir.join("a").join("b").join("file_b.txt"), "b").unwrap();
        fs::write(src_dir.join("x").join("file_x.txt"), "x").unwrap();

        // Copy directory
        let result = copy_dir_all(&src_dir, &dst_dir);

        assert!(result.is_ok());

        // Verify structure is preserved
        assert!(dst_dir.join("a").join("b").join("c").exists());
        assert!(dst_dir.join("x").join("y").exists());
        assert!(dst_dir.join("a").join("file_a.txt").exists());
        assert!(dst_dir.join("a").join("b").join("file_b.txt").exists());
        assert!(dst_dir.join("x").join("file_x.txt").exists());

        // Verify file contents
        assert_eq!(
            fs::read_to_string(dst_dir.join("a").join("file_a.txt")).unwrap(),
            "a"
        );
        assert_eq!(
            fs::read_to_string(dst_dir.join("a").join("b").join("file_b.txt")).unwrap(),
            "b"
        );
        assert_eq!(
            fs::read_to_string(dst_dir.join("x").join("file_x.txt")).unwrap(),
            "x"
        );
    }
}

#[cfg(test)]
mod create_build_dir_tests {
    use super::*;

    #[test]
    fn test_create_build_dir_std_build() {
        setup_i18n();

        let result = create_build_dir("test-crate", "", false);

        assert!(result.is_ok());
        let build_ctx = result.unwrap();

        // Verify the directory exists
        assert!(build_ctx.target_path.exists());
        assert!(build_ctx.target_path.is_dir());

        // Verify the directory name contains expected components
        let dir_name = build_ctx.target_path.file_name().unwrap().to_string_lossy();
        assert!(dir_name.contains("test-crate"));
        assert!(dir_name.contains("std"));
    }

    #[test]
    fn test_create_build_dir_no_std_build() {
        setup_i18n();

        let result = create_build_dir("test-crate", "", true);

        assert!(result.is_ok());
        let build_ctx = result.unwrap();

        // Verify the directory exists
        assert!(build_ctx.target_path.exists());
        assert!(build_ctx.target_path.is_dir());

        // Verify the directory name contains expected components
        let dir_name = build_ctx.target_path.file_name().unwrap().to_string_lossy();
        assert!(dir_name.contains("test-crate"));
        assert!(dir_name.contains("no-std"));
    }

    #[test]
    fn test_create_build_dir_with_features() {
        setup_i18n();

        let result = create_build_dir("test-crate", "feature1,feature2", false);

        assert!(result.is_ok());
        let build_ctx = result.unwrap();

        // Verify the directory exists
        assert!(build_ctx.target_path.exists());
        assert!(build_ctx.target_path.is_dir());

        // Verify the directory name contains sanitized features
        let dir_name = build_ctx.target_path.file_name().unwrap().to_string_lossy();
        assert!(dir_name.contains("test-crate"));
        assert!(dir_name.contains("std"));
        assert!(dir_name.contains("feature1_feature2"));
    }

    #[test]
    fn test_create_build_dir_sanitizes_special_characters() {
        setup_i18n();

        let result = create_build_dir("test-crate", "feature@1,feature#2", false);

        assert!(result.is_ok());
        let build_ctx = result.unwrap();

        // Verify the directory name has sanitized special characters
        let dir_name = build_ctx.target_path.file_name().unwrap().to_string_lossy();
        assert!(dir_name.contains("feature_1_feature_2"));
        assert!(!dir_name.contains("@"));
        assert!(!dir_name.contains("#"));
    }

    #[test]
    fn test_create_build_dir_unique_directories() {
        setup_i18n();

        let result1 = create_build_dir("test-crate", "feature1", false);
        let result2 = create_build_dir("test-crate", "feature1", false);

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let build_ctx1 = result1.unwrap();
        let build_ctx2 = result2.unwrap();

        // Verify both directories exist and are different
        assert!(build_ctx1.target_path.exists());
        assert!(build_ctx2.target_path.exists());
        assert_ne!(build_ctx1.target_path, build_ctx2.target_path);
    }

    #[test]
    fn test_create_build_dir_cleanup_on_drop() {
        setup_i18n();

        let target_path = {
            let result = create_build_dir("test-crate", "", false);
            assert!(result.is_ok());
            let build_ctx = result.unwrap();
            let path = build_ctx.target_path.clone();
            assert!(path.exists());
            path
            // build_ctx goes out of scope here, should trigger cleanup
        };

        // Directory should be cleaned up after BuildContext is dropped
        assert!(!target_path.exists());
    }
}

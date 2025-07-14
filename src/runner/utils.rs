use crate::runner::i18n;
use crate::runner::models::BuildContext;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Recursively copies a directory from a source path to a destination path.
/// It creates the destination directory if it doesn't exist.
///
/// # Arguments
/// * `src` - The source path, which must be a directory.
/// * `dst` - The destination path.
///
/// # Returns
/// An `std::io::Result` indicating the outcome of the operation.
///
/// 递归地将目录从源路径复制到目标路径。
/// 如果目标目录不存在，则会创建它。
///
/// # Arguments
/// * `src` - 源路径，必须是一个目录。
/// * `dst` - 目标路径。
///
/// # Returns
/// 一个 `std::io::Result`，指示操作的结果。
pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.as_ref().join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(entry.path(), &dst_path)?;
        } else {
            // `fs::copy` will overwrite the destination file if it already exists.
            // 如果目标文件已存在，`fs::copy` 会覆盖它。
            fs::copy(entry.path(), &dst_path)?;
        }
    }
    Ok(())
}

/// Creates a unique, temporary build directory for a given test case configuration.
/// The directory name is generated based on the features and build type to be
/// easily identifiable and to avoid conflicts between different test runs.
/// The `BuildContext` returned manages the lifetime of this directory.
///
/// # Arguments
/// * `crate_name` - The name of the crate being tested, used in the prefix.
/// * `features` - The feature string for the test case.
/// * `no_default_features` - A boolean indicating if default features are disabled.
///
/// # Returns
/// A `BuildContext` that holds the `TempDir` guard and the path to the `target`
/// directory within it.
///
/// 为给定的测试用例配置创建一个唯一的临时构建目录。
/// 目录名称基于 features 和构建类型生成，以便于识别并避免不同测试运行之间的冲突。
/// 返回的 `BuildContext` 管理此目录的生命周期。
///
/// # Arguments
/// * `crate_name` - 被测试的 crate 名称，用于前缀。
/// * `features` - 测试用例的 feature 字符串。
/// * `no_default_features` - 一个布尔值，指示是否禁用默认 features。
///
/// # Returns
/// 一个 `BuildContext`，它持有 `TempDir` guard 以及其中 `target` 目录的路径。
pub fn create_build_dir(
    crate_name: &str,
    features: &str,
    no_default_features: bool,
) -> BuildContext {
    let build_type = if no_default_features { "no-std" } else { "std" };

    // Create a descriptive prefix for the temp directory to make it easier to identify
    // when debugging. e.g., "my-crate-std" or "my-crate-no-std-feature-x"
    // 为临时目录创建一个描述性的前缀，以便在调试时更容易识别。
    // 例如："my-crate-std" 或 "my-crate-no-std-feature-x"
    let mut prefix = format!("{}-{}", crate_name, build_type);
    if !features.is_empty() {
        // Sanitize features for the directory name.
        // This is simpler than hashing and more readable.
        // 为目录名清理 features 字符串。
        // 这比哈希更简单且更具可读性。
        let sanitized_features = features.replace(|c: char| !c.is_alphanumeric(), "_");
        prefix = format!("{}-{}", prefix, sanitized_features);
    }

    let temp_dir = TempDir::with_prefix(&prefix).expect(&i18n::t("create_temp_dir_failed"));
    let target_path = temp_dir.path().to_path_buf();

    BuildContext {
        _temp_root: temp_dir,
        target_path,
    }
}

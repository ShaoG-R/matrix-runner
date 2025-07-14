use crate::runner::models::BuildContext;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Recursively copies a directory.
pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.as_ref().join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(entry.path(), &dst_path)?;
        } else {
            // The `copy` function will overwrite the destination file if it exists.
            fs::copy(entry.path(), &dst_path)?;
        }
    }
    Ok(())
}

/// Generate a temporary build directory for a given build configuration
pub fn create_build_dir(features: &str, no_default_features: bool) -> BuildContext {
    // Create a base temp directory with a meaningful prefix
    let build_type = if no_default_features { "no-std" } else { "std" };

    // Create descriptive prefix for the temp directory
    let mut prefix = format!("seal-crypto-{}", build_type);
    if !features.is_empty() {
        // Add a short hash of features to keep the name reasonably sized
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        features.hash(&mut hasher);
        let feature_hash = hasher.finish() % 10000; // Get last 4 digits
        prefix = format!("{}-{:04}", prefix, feature_hash);
    }

    let temp_dir = TempDir::with_prefix(&prefix).expect("Failed to create temporary directory");
    let target_path = temp_dir.path().to_path_buf();

    BuildContext {
        _temp_root: temp_dir,
        target_path,
    }
}

//! File filtering utilities for repository processing

use std::path::Path;

/// Check if a file should be included in processing
pub fn should_include_file<P: AsRef<Path>>(file_path: P) -> bool {
    let path = file_path.as_ref();

    // Skip hidden files and directories
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
    {
        return false;
    }

    // TODO: Implement more sophisticated filtering
    true
}

/// Check if a directory should be traversed
pub fn should_traverse_directory<P: AsRef<Path>>(dir_path: P) -> bool {
    let path = dir_path.as_ref();

    // Skip common build/cache directories
    if let Some(".git" | "node_modules" | "target" | "build" | "dist" | ".venv" | "venv") =
        path.file_name().and_then(|name| name.to_str())
    {
        return false;
    }

    true
}

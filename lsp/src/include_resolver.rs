use std::path::PathBuf;

/// Resolves include/jump file targets across the workspace.
pub struct IncludeResolver {
    /// Max recursion depth for include chains
    max_depth: usize,
}

impl IncludeResolver {
    pub fn new() -> Self {
        Self {
            max_depth: 10,
        }
    }

    /// Resolve a file path relative to a source file directory.
    pub fn resolve(
        &mut self,
        file_path: &str,
        source_dir: &PathBuf,
        depth: usize,
    ) -> Option<String> {
        if depth > self.max_depth {
            log::warn!("Max include depth ({}) exceeded for: {}", self.max_depth, file_path);
            return None;
        }

        // Resolve the path
        let resolved = source_dir.join(file_path);

        // Try the exact path first
        if resolved.exists() {
            return Some(resolved.to_string_lossy().to_string());
        }

        // Try appending .in extension
        let with_ext = source_dir.join(format!("{}.in", file_path));
        if with_ext.exists() {
            return Some(with_ext.to_string_lossy().to_string());
        }

        None
    }

    /// Check if an include path resolves to an existing file.
    pub fn file_exists(&self, file_path: &str, source_dir: &PathBuf) -> bool {
        let resolved = source_dir.join(file_path);
        if resolved.exists() {
            return true;
        }
        let with_ext = source_dir.join(format!("{}.in", file_path));
        with_ext.exists()
    }
}

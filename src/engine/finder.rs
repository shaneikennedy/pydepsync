use std::{fs, io, path::PathBuf};
use walkdir::WalkDir;

pub struct PythonFileFinder {
    excluded_dirs: Vec<String>,
}

impl PythonFileFinder {
    pub fn new() -> Self {
        Self {
            excluded_dirs: vec!["venv".to_string(), ".git".to_string()],
        }
    }

    /// Add directories to exclude from the search
    pub fn exclude_dirs(mut self, dirs: Vec<String>) -> Self {
        self.excluded_dirs.extend(dirs);
        self
    }

    /// Similar to find files but this returns dir names too
    /// Because imports can reference just a dir if code is in the
    /// __init__.py file
    pub fn find_local_packages(&self, start_path: &PathBuf) -> Result<Vec<PathBuf>, io::Error> {
        let root_package = fs::canonicalize(PathBuf::from(start_path)).unwrap();
        let mut local_packages = vec![root_package];
        for entry in WalkDir::new(start_path)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| {
                // Skip excluded directories
                if e.file_type().is_dir() {
                    if let Some(dir_name) = e.file_name().to_str() {
                        return !self.excluded_dirs.contains(&dir_name.to_string());
                    }
                }
                true
            })
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            local_packages.push(path.to_path_buf());
        }
        Ok(local_packages)
    }

    /// Find all Python files with the configured settings
    pub fn find_files(&self, start_path: &PathBuf) -> Result<Vec<PathBuf>, io::Error> {
        let mut python_files = Vec::new();

        for entry in WalkDir::new(start_path)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| {
                // Skip excluded directories
                if e.file_type().is_dir() {
                    if let Some(dir_name) = e.file_name().to_str() {
                        return !self.excluded_dirs.contains(&dir_name.to_string());
                    }
                }
                true
            })
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if path.is_file() {
                if let Some(extension) = path.extension() {
                    let ext = extension.to_str().unwrap_or("");
                    if ext == "py" {
                        python_files.push(path.to_path_buf());
                    }
                }
            }
        }

        Ok(python_files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::tempdir;

    #[test]
    fn test_python_file_finder() -> Result<(), io::Error> {
        let temp_dir = tempdir()?;

        // Create test files
        File::create(temp_dir.path().join("test1.py"))?;

        // Create excluded directory
        let venv_dir = temp_dir.path().join("venv");
        fs::create_dir(&venv_dir)?;
        File::create(venv_dir.join("test2.py"))?;

        // Test with default settings
        let finder = PythonFileFinder::new();
        let files = finder.find_files(&PathBuf::from(temp_dir.path()))?;
        assert_eq!(files.len(), 1); // Should only find test1.py

        Ok(())
    }
}

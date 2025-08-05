use std::collections::HashSet;
use std::fs::{self};
use std::io;
use std::path::PathBuf;

use log::{debug, info};
use taplo::formatter::{format, Options};
use toml_edit::{value, Array, DocumentMut, Item};

use crate::dependency::Dependency;

#[derive(Debug, Clone)]
pub struct PyProject {
    deps: HashSet<Dependency>,
    optional_deps: HashSet<Dependency>,
    toml_document: DocumentMut,
}

impl PyProject {
    pub fn all_deps(&self) -> HashSet<Dependency> {
        let mut all_deps = HashSet::new();
        for dep in self.deps.clone() {
            all_deps.insert(dep);
        }
        for dep in self.optional_deps.clone() {
            all_deps.insert(dep);
        }
        all_deps
    }
}

pub fn read(path: &PathBuf) -> Result<PyProject, io::Error> {
    let content = fs::read_to_string(path)?;
    let doc = content.parse::<DocumentMut>().unwrap();

    // get existing deps
    let mut existing_deps = Array::new();
    if let Some(project) = doc.get("project") {
        if let Some(deps) = project.get("dependencies").and_then(|d| d.as_array()) {
            existing_deps = deps.clone();
        }
    }

    // Access the "dependency-groups" table
    let mut optional_dependencies: HashSet<Dependency> = HashSet::new();
    if let Some(Item::Table(table)) = doc.get("dependency-groups") {
        // Iterate through each group in dependency-groups
        for (_group_name, group_value) in table.iter() {
            if let Item::Value(value) = group_value {
                // If the value is an array, process each dependency
                if let Some(array) = value.as_array() {
                    for dep in array {
                        if let Some(dep_str) = dep.as_str() {
                            optional_dependencies.insert(Dependency::parse(dep_str).unwrap());
                        }
                    }
                }
            }
        }
    }

    // Parse project.optional-dependencies
    if let Some(Item::Table(project_table)) = doc.get("project") {
        if let Some(Item::Table(opt_deps_table)) = project_table.get("optional-dependencies") {
            for (_group_name, group_value) in opt_deps_table.iter() {
                if let Item::Value(value) = group_value {
                    if let Some(array) = value.as_array() {
                        for dep in array {
                            if let Some(dep_str) = dep.as_str() {
                                optional_dependencies.insert(Dependency::parse(dep_str).unwrap());
                            }
                        }
                    }
                }
            }
        }
    }

    let existing_deps: HashSet<Dependency> = existing_deps
        .iter()
        .map(|v| Dependency::parse(v.as_str().unwrap()).unwrap())
        .collect();
    debug!(
        "Found existing deps: {}",
        existing_deps
            .iter()
            .map(|d| format!("{d}"))
            .collect::<Vec<_>>()
            .join(",")
    );
    Ok(PyProject {
        deps: existing_deps,
        optional_deps: optional_dependencies,
        toml_document: doc,
    })
}

pub fn write(
    path: &PathBuf,
    mut pyproject: PyProject,
    new_deps: HashSet<Dependency>,
) -> Result<(), io::Error> {
    // Constrcuct a new dependency set that we will write back to pyproject
    // that contains the existing ones and anything new
    let mut arr = Array::new();
    for dep in new_deps {
        info!("Adding: {dep}");
        arr.push(dep.to_dependency_repr());
    }
    for dep in pyproject.deps {
        arr.push(dep.to_dependency_repr());
    }
    // Insert into project table
    if let Some(project) = pyproject.toml_document.get_mut("project") {
        if let Some(table) = project.as_table_mut() {
            table.insert("dependencies", value(arr));
        }
    }
    let updated_contents = format(
        &pyproject.toml_document.to_string(),
        Options {
            align_entries: true,
            align_comments: true,
            align_single_comments: true,
            array_trailing_comma: true,
            array_auto_expand: true,
            inline_table_expand: true,
            array_auto_collapse: false,
            compact_arrays: false,
            compact_inline_tables: false,
            compact_entries: false,
            column_width: 30,
            indent_tables: false,
            indent_entries: false,
            indent_string: "    ".into(),
            trailing_newline: false,
            reorder_keys: false,
            reorder_arrays: true,
            allowed_blank_lines: 2,
            crlf: false,
        },
    );
    // Write back to file
    fs::write(path, updated_contents).unwrap();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn setup_toml_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{content}").unwrap();
        file
    }

    #[test]
    fn test_all_deps_empty() {
        let pyproject = PyProject {
            deps: HashSet::new(),
            optional_deps: HashSet::new(),
            toml_document: DocumentMut::new(),
        };
        let all_deps = pyproject.all_deps();
        assert_eq!(all_deps.len(), 0, "Empty deps should return empty set");
    }

    #[test]
    fn test_all_deps_with_deps() {
        let mut deps = HashSet::new();
        deps.insert(Dependency::parse("dep1").unwrap());
        let mut optional_deps = HashSet::new();
        optional_deps.insert(Dependency::parse("dep1").unwrap());

        let pyproject = PyProject {
            deps,
            optional_deps,
            toml_document: DocumentMut::new(),
        };
        let all_deps = pyproject.all_deps();

        assert_eq!(all_deps.len(), 1, "Should combine deps and optional_deps");
        assert!(all_deps.contains(&Dependency::parse("dep1").unwrap()));
    }

    #[test]
    fn test_read_empty_toml() {
        let toml_content = "";
        let file = setup_toml_file(toml_content);
        let path = file.path().to_path_buf();
        let result = read(&path);
        assert!(result.is_ok(), "Reading empty TOML should succeed");
        let pyproject = result.unwrap();
        assert_eq!(pyproject.deps.len(), 0, "Empty TOML should have no deps");
        assert_eq!(
            pyproject.optional_deps.len(),
            0,
            "Empty TOML should have no optional deps"
        );
    }

    #[test]
    fn test_read_basic_deps() {
        let toml_content = r#"
            [project]
            dependencies = ["dep1", "dep2"]
        "#;
        let file = setup_toml_file(toml_content);
        let path = file.path().to_path_buf();
        let result = read(&path);
        assert!(result.is_ok(), "Reading basic TOML should succeed");
        let pyproject = result.unwrap();

        let mut expected_deps = HashSet::new();
        expected_deps.insert(Dependency::parse("dep1").unwrap());
        expected_deps.insert(Dependency::parse("dep2").unwrap());

        assert_eq!(pyproject.deps, expected_deps, "Deps should match");
        assert_eq!(
            pyproject.optional_deps.len(),
            0,
            "No optional deps expected"
        );
    }

    #[test]
    fn test_read_optional_deps() {
        let toml_content = r#"
            [project]
            dependencies = ["dep1"]

            [project.optional-dependencies]
            test = ["pytest", "coverage"]
            dev = ["flake8"]
        "#;
        let file = setup_toml_file(toml_content);
        let path = file.path().to_path_buf();
        let result = read(&path);
        assert!(result.is_ok(), "Reading optional deps TOML should succeed");
        let pyproject = result.unwrap();

        let mut expected_deps = HashSet::new();
        expected_deps.insert(Dependency::parse("dep1").unwrap());

        let mut expected_optional_deps = HashSet::new();
        expected_optional_deps.insert(Dependency::parse("pytest").unwrap());
        expected_optional_deps.insert(Dependency::parse("coverage").unwrap());
        expected_optional_deps.insert(Dependency::parse("flake8").unwrap());

        assert_eq!(pyproject.deps, expected_deps, "Deps should match");
        assert_eq!(
            pyproject.optional_deps, expected_optional_deps,
            "Optional deps should match"
        );
    }

    #[test]
    fn test_read_dependency_groups() {
        let toml_content = r#"
            [project]
            dependencies = ["dep1"]

            [dependency-groups]
            lint = ["ruff", "mypy"]
        "#;
        let file = setup_toml_file(toml_content);
        let path = file.path().to_path_buf();
        let result = read(&path);
        assert!(
            result.is_ok(),
            "Reading dependency-groups TOML should succeed"
        );
        let pyproject = result.unwrap();

        let mut expected_deps = HashSet::new();
        expected_deps.insert(Dependency::parse("dep1").unwrap());

        let mut expected_optional_deps = HashSet::new();
        expected_optional_deps.insert(Dependency::parse("ruff").unwrap());
        expected_optional_deps.insert(Dependency::parse("mypy").unwrap());

        assert_eq!(pyproject.deps, expected_deps, "Deps should match");
        assert_eq!(
            pyproject.optional_deps, expected_optional_deps,
            "Dependency-groups should be treated as optional deps"
        );
    }

    #[test]
    fn test_read_file_not_found() {
        let path = PathBuf::from("nonexistent.toml");
        let result = read(&path);
        assert!(result.is_err(), "Reading nonexistent file should fail");
        assert_eq!(
            result.unwrap_err().kind(),
            io::ErrorKind::NotFound,
            "Error should be NotFound"
        );
    }
}

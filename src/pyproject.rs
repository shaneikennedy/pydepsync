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
    let content = fs::read_to_string(path).unwrap();
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
    if let Some(dep_groups) = doc.get("dependency-groups") {
        if let Item::Table(table) = dep_groups {
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
    }

    // Parse project.optional-dependencies
    if let Some(project) = doc.get("project") {
        if let Item::Table(project_table) = project {
            if let Some(opt_deps) = project_table.get("optional-dependencies") {
                if let Item::Table(opt_deps_table) = opt_deps {
                    for (_group_name, group_value) in opt_deps_table.iter() {
                        if let Item::Value(value) = group_value {
                            if let Some(array) = value.as_array() {
                                for dep in array {
                                    if let Some(dep_str) = dep.as_str() {
                                        optional_dependencies
                                            .insert(Dependency::parse(dep_str).unwrap());
                                    }
                                }
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
            .map(|d| d.to_string())
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
        info!("Adding: {}", dep.to_string());
        arr.push(dep.to_string());
    }
    for dep in pyproject.deps {
        arr.push(dep.to_string());
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

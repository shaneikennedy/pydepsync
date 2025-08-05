use std::collections::{HashMap, HashSet};
use std::fs::read;
use std::path::PathBuf;
use std::str::from_utf8;
use std::{io, thread};

use evaluator::DependencyEvaluator;
use finder::PythonFileFinder;
use log::{debug, info};
use parser::extract_dependencies;
use resolver::PackageResolver;
use thiserror::Error;

use crate::dependency::Dependency;
use crate::pyproject::PyProject;

mod evaluator;
mod finder;
mod irregulars;
mod parser;
mod resolver;
mod stdlib;

type ImportParser = fn(&str) -> Result<Vec<String>, io::Error>;

#[derive(Clone, Debug, PartialEq)]
pub struct EngineOptions {
    pub exclude_dirs: Vec<String>,
    pub extra_indexes: Vec<String>,
    pub preferred_index: Option<String>,
    pub extras_to_remap: HashMap<String, String>,
}

pub struct DetectEngine<'a> {
    pyproject: PyProject,
    finder: PythonFileFinder,
    parser: ImportParser,
    evaluator: DependencyEvaluator<'a>,
    resolver: PackageResolver,
}

#[derive(Debug, Error)]
pub enum DetectEngineError {
    #[allow(dead_code)]
    #[error("problem evaluating imports")]
    Evaluation,
    #[error("problem reading packages")]
    FileFinding,
    #[error("problem reading python file")]
    FileReading,
    #[error("problem parsing python code")]
    Parsing,
    #[allow(dead_code)]
    #[error("problem resolving packages on package index")]
    Resolver,
}

impl DetectEngine<'_> {
    pub fn new(pyproject: PyProject, options: EngineOptions) -> Self {
        let mut exclude_dirs = vec![
            ".venv".to_string(),
            ".git".to_string(),
            "target".to_string(),
        ];
        for dir in &options.exclude_dirs {
            exclude_dirs.push(dir.clone());
        }
        let resolver = PackageResolver::new(
            options.extra_indexes.clone(),
            options.preferred_index.clone(),
        );
        let evaluator = DependencyEvaluator::new(options.extras_to_remap);
        DetectEngine {
            pyproject,
            finder: finder::PythonFileFinder::new().exclude_dirs(exclude_dirs),
            parser: extract_dependencies,
            evaluator,
            resolver,
        }
    }

    pub fn detect_dependencies(
        &self,
        path: PathBuf,
    ) -> Result<HashSet<Dependency>, DetectEngineError> {
        // Find python modules
        info!("Reading your code...");
        let files = self.finder.find_files(&path);
        if files.is_err() {
            return Err(DetectEngineError::FileFinding);
        }

        // Parse imports
        info!("Parsing imports...");
        let mut candidates: HashSet<String> = HashSet::new();
        for file in &files.unwrap() {
            let contents = read(file);
            if contents.is_err() {
                return Err(DetectEngineError::FileReading);
            }
            let contents = contents.unwrap();

            // Guaranteed to be utf8 from match read(&file) above
            let content_str = from_utf8(&contents).unwrap();
            let imports = (self.parser)(content_str);
            if imports.is_err() {
                return Err(DetectEngineError::Parsing);
            }
            for i in imports.unwrap() {
                // filter out mod.sub.subsub  we only want mod here
                candidates.insert(i.split(".").take(1).collect::<String>());
            }
        }

        debug!(
            "Candidates: {}",
            candidates
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(",")
        );
        let local_packages = self.get_local_packages(&path)?;

        // Evaluate the imports, i.e filtering and remapping
        info!("Evaluating candidates...");
        let deps = self
            .evaluator
            .evaluate(candidates, self.pyproject.all_deps(), local_packages);

        // Resolve each candidate in their own thread, join the threads
        // collect the resolved deps back into a hashset
        info!("Resolving packages...");
        let resolved_deps: HashSet<Dependency> = deps
            .into_iter()
            .map(|dep| {
                thread::spawn({
                    let resolver = self.resolver.clone();
                    move || resolver.resolve(&dep)
                })
            })
            .filter_map(|h| h.join().ok())
            .filter_map(|result| result.ok())
            .collect();

        debug!(
            "Resolved deps: {}",
            resolved_deps
                .iter()
                .map(|d| format!("{d}"))
                .collect::<Vec<_>>()
                .join(",")
        );

        Ok(resolved_deps)
    }

    // Get the local packages in the file tree and parse as a list of Strings that are "local packages"
    fn get_local_packages(&self, path: &PathBuf) -> Result<HashSet<String>, DetectEngineError> {
        let local_packages = self.finder.find_local_packages(path);
        if local_packages.is_err() {
            return Err(DetectEngineError::FileFinding);
        }
        let local_packages: HashSet<String> = local_packages
            .unwrap()
            .iter()
            .filter_map(|pb| pb.file_stem())
            .filter_map(|package_name| package_name.to_str())
            .map(String::from)
            .collect();
        debug!(
            "Found local packages: {}",
            local_packages
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(",")
        );
        Ok(local_packages)
    }
}

#[cfg(test)]
mod tests {
    use crate::pyproject;

    use super::*;

    #[test]
    fn test_example_django_rest_app() -> Result<(), io::Error> {
        let pyproject = pyproject::read(&PathBuf::from("./example_app/pyproject.toml")).unwrap();
        let options = EngineOptions {
            exclude_dirs: Vec::new(),
            extra_indexes: Vec::new(),
            preferred_index: None,
            extras_to_remap: HashMap::new(),
        };
        let engine = DetectEngine::new(pyproject, options);
        let deps = engine
            .detect_dependencies(PathBuf::from("./example_app"))
            .unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&Dependency::parse("Django").unwrap()));
        assert!(deps.contains(&Dependency::parse("djangorestframework").unwrap()));
        Ok(())
    }
}

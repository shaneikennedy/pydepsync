use std::collections::{HashMap, HashSet};
use std::fs::read;
use std::path::PathBuf;
use std::str::from_utf8;
use std::{io, thread};

use evaluator::evaluate_dependencies;
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
type DependencyEvaluator = fn(
    HashSet<String>,
    HashSet<Dependency>,
    HashSet<String>,
    HashSet<&str>,
    HashMap<&str, &str>,
) -> Result<HashSet<Dependency>, io::Error>;

pub struct EngineOptions {
    pub exclude_dirs: Vec<String>,
    pub extra_indexes: Vec<String>,
    pub preferred_index: Option<String>,
}

pub struct DetectEngine {
    pyproject: PyProject,
    finder: PythonFileFinder,
    parser: ImportParser,
    evaluator: DependencyEvaluator,
    resolver: PackageResolver,
}

#[derive(Debug, Error)]
pub enum DetectEngineError {
    #[error("problem evaluating imports")]
    EvaluationError,
    #[error("problem reading packages")]
    FileFindingError,
    #[error("problem reading python file")]
    FileReadingError,
    #[error("problem parsing python code")]
    ParsingError,
    #[allow(dead_code)]
    #[error("problem resolving packages on package index")]
    ResolverError,
}

impl DetectEngine {
    pub fn new(pyproject: PyProject, options: EngineOptions) -> Self {
        let mut exclude_dirs = vec![
            ".venv".to_string(),
            ".git".to_string(),
            "target".to_string(),
        ];
        exclude_dirs.extend(options.exclude_dirs);
        let resolver = PackageResolver::new(options.extra_indexes, options.preferred_index);
        return DetectEngine {
            pyproject,
            finder: finder::PythonFileFinder::new().exclude_dirs(exclude_dirs),
            parser: extract_dependencies,
            evaluator: evaluate_dependencies,
            resolver,
        };
    }

    pub fn detect_dependencies(
        &self,
        path: PathBuf,
    ) -> Result<HashSet<Dependency>, DetectEngineError> {
        // Find python modules
        info!("Reading your code...");
        let files = self.finder.find_files(&path);
        if files.is_err() {
            return Err(DetectEngineError::FileFindingError);
        }

        // Parse imports
        info!("Parsing imports...");
        let mut candidates: HashSet<String> = HashSet::new();
        for file in &files.unwrap() {
            let contents = read(&file);
            if contents.is_err() {
                return Err(DetectEngineError::FileReadingError);
            }
            let contents = contents.unwrap();

            // Guaranteed to be utf8 from match read(&file) above
            let content_str = from_utf8(&contents).unwrap();
            let imports = (self.parser)(content_str);
            if imports.is_err() {
                return Err(DetectEngineError::ParsingError);
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
        let stdlib = stdlib::get_python_stdlib_modules();
        let irregulars = irregulars::get_python_irregulars();

        // Evaluate the imports, i.e filtering and remapping
        info!("Evaluating candidates...");
        let deps = (self.evaluator)(
            candidates,
            self.pyproject.all_deps(),
            local_packages,
            stdlib,
            irregulars,
        );
        if deps.is_err() {
            return Err(DetectEngineError::EvaluationError);
        }

        // Resolve each candidate in their own thread, join the threads
        // collect the resolved deps back into a hashset
        info!("Resolving packages...");
        let resolved_deps: HashSet<Dependency> = deps
            .unwrap()
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
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        Ok(resolved_deps)
    }

    // Get the local packages in the file tree and parse as a list of Strings that are "local packages"
    fn get_local_packages(&self, path: &PathBuf) -> Result<HashSet<String>, DetectEngineError> {
        let local_packages = self.finder.find_local_packages(&path);
        if local_packages.is_err() {
            return Err(DetectEngineError::FileFindingError);
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
        return Ok(local_packages);
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

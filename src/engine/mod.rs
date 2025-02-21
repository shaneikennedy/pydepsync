use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs::read;
use std::path::PathBuf;
use std::str::from_utf8;
use std::{io, thread};

use evaluator::evaluate_dependencies;
use finder::PythonFileFinder;
use log::{debug, info};
use parser::extract_dependencies;
use resolver::PackageResolver;

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

    pub fn detect_dependencies(&self, path: PathBuf) -> Result<HashSet<Dependency>, io::Error> {
        // Find python modules
        info!("Reading your code...");
        let files = match self.finder.find_files(&path) {
            Ok(f) => f,
            Err(e) => panic!("Problem parsing directory: {:?}", e),
        };

        // Parse imports
        info!("Parsing imports...");
        let mut candidates: HashSet<String> = HashSet::new();
        for file in &files {
            let contents = match read(&file) {
                Ok(c) => c,
                Err(e) => panic!("Problem opening file: {:?}", e),
            };

            match (self.parser)(from_utf8(&contents).unwrap()) {
                Ok(imports) => {
                    for i in imports {
                        // filter out mod.sub.subsub  we only want mod here
                        candidates.insert(i.split(".").take(1).collect::<String>());
                    }
                }
                Err(_) => panic!("problem extracting deps for {:?}", file.to_str().unwrap()),
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

        let local_packages = match self.get_local_packages(&path) {
            Ok(packages) => packages,
            Err(e) => panic!("Problem finding local packages: {:?}", e),
        };
        let stdlib = stdlib::get_python_stdlib_modules();
        let irregulars = irregulars::get_python_irregulars();
        // Evaluate the imports, i.e filtering and remapping
        info!("Evaluating candidates...");
        let deps = match (self.evaluator)(
            candidates,
            self.pyproject.all_deps(),
            local_packages,
            stdlib,
            irregulars,
        ) {
            Ok(d) => d,
            Err(e) => panic!("Problem evaluating candidates: {:?}", e),
        };

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
            .map(|h| h.join().unwrap().unwrap())
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
    fn get_local_packages(&self, path: &PathBuf) -> Result<HashSet<String>, io::Error> {
        let local_packages = match self.finder.find_local_packages(&path) {
            Ok(packages) => packages,
            Err(e) => panic!("Problem finding local packages: {:?}", e),
        };
        let local_packages: HashSet<OsString> = local_packages
            .iter()
            .map(|pb| pb.file_stem())
            .filter(|stem| stem.is_some())
            .map(|stem| stem.unwrap().to_os_string())
            .collect();
        let local_packages: HashSet<String> = local_packages
            .iter()
            .filter_map(|os_str| os_str.to_str())
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

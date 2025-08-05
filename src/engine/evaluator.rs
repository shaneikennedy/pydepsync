use std::collections::{HashMap, HashSet};

use crate::dependency::Dependency;

use super::{irregulars, stdlib};

#[derive(Clone)]
pub struct DependencyEvaluator<'a> {
    stdlib_pakages: HashSet<&'a str>,
    irregulars_to_remap: HashMap<String, String>,
}

impl DependencyEvaluator<'_> {
    pub fn new(extras_to_remap: HashMap<String, String>) -> Self {
        let mut irregulars = extras_to_remap;
        for (key, val) in irregulars::get_python_irregulars() {
            irregulars.insert(key.to_string(), val.to_string());
        }
        DependencyEvaluator {
            stdlib_pakages: stdlib::get_python_stdlib_modules(),
            irregulars_to_remap: irregulars,
        }
    }

    pub fn evaluate(
        &self,
        candidates: HashSet<String>,
        existing_deps: HashSet<Dependency>,
        local_packages: HashSet<String>,
    ) -> HashSet<Dependency> {
        // Filter out any imports that are in the stdlib,
        // And convert anything that matches one of the "irregulars"
        // i.e python packages that are called something but to import code
        // from that package is called something else
        let deps: HashSet<String> = candidates
            .iter()
            .filter(|c| !self.stdlib_pakages.contains(&c.as_str()))
            .filter(|&c| !local_packages.clone().contains(c))
            .cloned()
            .map(|c| {
                let hit = self.irregulars_to_remap.get(c.as_str());
                match hit {
                    Some(m) => m.to_string(),
                    None => c,
                }
            })
            // filter on existing needs to come last
            .filter(|c| !existing_deps.contains(&Dependency::parse(c).unwrap()))
            .collect();

        deps.iter()
            .map(|d| Dependency::parse(d.as_str()).unwrap())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_excludes_stdlib() {
        let evaluator = DependencyEvaluator::new(HashMap::new());
        let candidates = HashSet::from(["os".to_string()]);
        let res = evaluator.evaluate(candidates, HashSet::new(), HashSet::new());
        assert_eq!(res.len(), 0);
    }

    #[test]
    fn test_excludes_local_package() {
        let evaluator = DependencyEvaluator::new(HashMap::new());
        let candidates = HashSet::from(["mymod".to_string()]);
        let res = evaluator.evaluate(
            candidates,
            HashSet::new(),
            HashSet::from(["mymod".to_string()]),
        );
        assert_eq!(res.len(), 0);
    }

    #[test]
    fn test_excludes_existing_packages() {
        let evaluator = DependencyEvaluator::new(HashMap::new());
        let candidates = HashSet::from(["django".to_string()]);
        let res = evaluator.evaluate(
            candidates,
            HashSet::from([Dependency::parse("Django").unwrap()]),
            HashSet::new(),
        );
        assert_eq!(res.len(), 0);
    }

    #[test]
    fn test_remaps_irregular() {
        let evaluator = DependencyEvaluator::new(HashMap::new());
        let candidates = HashSet::from(["AFQ".to_string()]);
        let res = evaluator.evaluate(candidates, HashSet::new(), HashSet::new());
        assert_eq!(res.len(), 1);
        assert!(res.contains(&Dependency::parse("pyAFQ").unwrap()));
    }

    #[test]
    fn test_remaps_extra_irregulars() {
        let evaluator = DependencyEvaluator::new(HashMap::from([(
            "thingtoremap".to_string(),
            "ThingToRemap".to_string(),
        )]));
        let candidates = HashSet::from(["thingtoremap".to_string()]);
        let res = evaluator.evaluate(candidates, HashSet::new(), HashSet::new());
        assert_eq!(res.len(), 1);
        assert!(res.contains(&Dependency::parse("ThingToRemap").unwrap()));
    }
}

use std::{
    collections::{HashMap, HashSet},
    io,
};

use crate::dependency::Dependency;

pub fn evaluate_dependencies(
    candidates: HashSet<String>,
    existing_deps: HashSet<Dependency>,
    local_packages: HashSet<String>,
    stdlib_packages: HashSet<&str>,
    irregulars_to_remap: HashMap<&str, &str>,
) -> Result<HashSet<Dependency>, io::Error> {
    // Filter out any imports that are in the stdlib,
    // And convert anything that matches one of the "irregulars"
    // i.e python packages that are called something but to import code
    // from that package is called something else
    let deps: HashSet<String> = candidates
        .iter()
        .filter(|c| !stdlib_packages.contains(&c.as_str()))
        .cloned()
        .filter(|c| !local_packages.clone().contains(c))
        .map(|c| {
            let hit = irregulars_to_remap.get(c.as_str());
            match hit {
                Some(m) => m.to_string(),
                None => c,
            }
        })
        // filter on existing needs to come last
        .filter(|c| !existing_deps.contains(&Dependency::parse(c).unwrap()))
        .collect();

    Ok(deps
        .iter()
        .map(|d| Dependency::parse(d.as_str()).unwrap())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_excludes_stdlib() -> Result<(), io::Error> {
        let candidates = HashSet::from(["os".to_string()]);
        let res = evaluate_dependencies(
            candidates,
            HashSet::new(),
            HashSet::new(),
            HashSet::from(["os"]),
            HashMap::new(),
        )
        .unwrap();
        assert_eq!(res.len(), 0);
        Ok(())
    }

    #[test]
    fn test_excludes_local_package() -> Result<(), io::Error> {
        let candidates = HashSet::from(["mymod".to_string()]);
        let res = evaluate_dependencies(
            candidates,
            HashSet::new(),
            HashSet::from(["mymod".to_string()]),
            HashSet::new(),
            HashMap::new(),
        )
        .unwrap();
        assert_eq!(res.len(), 0);
        Ok(())
    }

    #[test]
    fn test_excludes_existing_packages() -> Result<(), io::Error> {
        let candidates = HashSet::from(["django".to_string()]);
        let res = evaluate_dependencies(
            candidates,
            HashSet::from([Dependency::parse("Django").unwrap()]),
            HashSet::new(),
            HashSet::new(),
            HashMap::new(),
        )
        .unwrap();
        assert_eq!(res.len(), 0);
        Ok(())
    }

    #[test]
    fn test_remaps_irregular() -> Result<(), io::Error> {
        let candidates = HashSet::from(["AFQ".to_string()]);
        let res = evaluate_dependencies(
            candidates,
            HashSet::new(),
            HashSet::new(),
            HashSet::new(),
            HashMap::from([("AFQ", "pyAFQ")]),
        )
        .unwrap();
        assert_eq!(res.len(), 1);
        assert!(res.contains(&Dependency::parse("pyAFQ").unwrap()));
        Ok(())
    }
}

use std::io;

use crate::dependency::Dependency;

use log::{debug, warn};
use serde_json::Value;

#[derive(Clone)]
pub struct PackageResolver {
    indexes: Vec<String>,
}

impl PackageResolver {
    pub fn new(extra_indexes: Vec<String>, preferred_index: Option<String>) -> Self {
        let pref_index = match preferred_index {
            Some(i) => vec![i],
            None => Vec::new(),
        };
        let default_indexes = vec!["https://pypi.org/pypi".to_string()];
        return PackageResolver {
            indexes: pref_index
                .into_iter()
                .chain(default_indexes)
                .chain(extra_indexes)
                .collect(),
        };
    }

    pub fn resolve(&self, dep: &Dependency) -> Result<Dependency, io::Error> {
        let found = self
            .indexes
            .iter()
            .find_map(|index| self.clone().resolve_on_index(dep, index));
        match found {
            Some(d) => Ok(d),
            None => Ok(dep.clone()),
        }
    }

    // TODO make this a much better http client, retries, backoff, error handling
    fn resolve_on_index(self, dep: &Dependency, index: &str) -> Option<Dependency> {
        let url = format!("{}/{}/json", index, dep.name());
        let response = ureq::get(url.as_str()).call();
        if response.is_err() {
            warn!(
                "Problem resolving package {} on index {}.",
                dep.name(),
                index,
            );
            debug!("Error {}", response.unwrap_err());
            return None;
        }
        let json: Value = response.unwrap().body_mut().read_json().unwrap();
        if let Some(version) = json
            .get("info")
            .and_then(|info| info.get("version"))
            .and_then(|version| version.as_str())
        {
            debug!("Found version: {} for {}", version, dep.name());
            let dep_str = format!("{}~={}", dep.name(), version);
            Some(Dependency::parse(dep_str.as_str()).unwrap())
        } else {
            warn!(
                "Could not resolve package {} on index: {}",
                dep.name(),
                index,
            );
            None
        }
    }
}

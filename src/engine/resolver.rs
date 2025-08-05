use std::io;

use crate::dependency::Dependency;

use log::{debug, warn};
use scraper::{Html, Selector};

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
        let default_indexes = vec!["https://pypi.org/simple".to_string()];
        PackageResolver {
            indexes: pref_index
                .into_iter()
                .chain(default_indexes)
                .chain(extra_indexes)
                .collect(),
        }
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
        let url = format!("{}/{}", index, dep.name());
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
        let mut response = response.unwrap();
        let html = response.body_mut().read_to_string();
        if html.is_err() {
            warn!(
                "Problem reading package info for package {} on index {}",
                dep.name(),
                index
            );
            debug!("Error {}", html.unwrap_err());
            return None;
        }
        let html = html.unwrap();
        let versions = Self::parse_versions_on_index(dep, index, html.as_str());
        let versions = versions.unwrap_or_default();

        let lastest_version = Self::get_latest_version_from_version_str(versions);

        match lastest_version {
            Some(v) => {
                debug!("Found version: {} for {}", v, dep.name());
                let dep_str = format!("{}~={}", dep.name(), v);
                Some(Dependency::parse(dep_str.as_str()).unwrap())
            }
            None => {
                warn!(
                    "Could not resolve package {} on index: {}",
                    dep.name(),
                    index,
                );
                None
            }
        }
    }

    fn parse_versions_on_index(dep: &Dependency, index: &str, html: &str) -> Option<Vec<String>> {
        let document = Html::parse_document(html);
        let selector = Selector::parse("a");
        if selector.is_err() {
            warn!(
                "Problem versions for package {} on index {}",
                dep.name(),
                index
            );
            debug!("Error {}", selector.unwrap_err());
            return None;
        }
        let selector = selector.unwrap();

        let mut versions = Vec::new();

        // Extract all version links, excluding beta, alpha, and release candidates
        for element in document.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                // Extract version from the filename
                let parts: Vec<&str> = href.split('/').collect();
                if let Some(filename) = parts.last() {
                    if let Some(start) = filename.find(format!("{}-", dep.name()).as_str()) {
                        let rest = &filename[start + format!("{}-", dep.name()).as_str().len()..];
                        if let Some(end) = rest.find(".tar.gz") {
                            let version = &rest[..end];
                            // Verify it only contains numbers and dots
                            if version.chars().all(|c| c.is_ascii_digit() || c == '.') {
                                versions.push(version.to_string());
                            }
                        }
                    }
                }
            }
        }
        Some(versions)
    }

    fn get_latest_version_from_version_str(versions: Vec<String>) -> Option<String> {
        let mut versions = versions.clone();
        versions.sort_by(|a, b| {
            let a_parts: Vec<&str> = a.split('.').collect();
            let b_parts: Vec<&str> = b.split('.').collect();

            for (a_part, b_part) in a_parts.iter().zip(b_parts.iter()) {
                match (a_part.parse::<i32>(), b_part.parse::<i32>()) {
                    (Ok(a_num), Ok(b_num)) => {
                        if a_num != b_num {
                            return b_num.cmp(&a_num);
                        }
                    }
                    _ => return b.cmp(a), // Fallback to string comparison
                }
            }
            b.len().cmp(&a.len())
        });

        versions.first().cloned()
    }
}

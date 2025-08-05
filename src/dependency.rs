use regex::Regex;
use std::collections::HashSet;
use std::fmt::Display;
use std::hash::Hash;

#[derive(Debug, Eq, Clone)]
pub struct Dependency {
    name: String,
    extras: HashSet<String>,
    version_spec: Option<(String, String)>, // (specifier, version)
    markers: Option<String>,
}

impl Display for Dependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dep = String::new();
        dep += self.name.as_str();
        if !&self.extras.is_empty() {
            dep += "[";
            let mut extras = self
                .extras
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>();

            extras.sort(); // Needs to be sorted because iterating over a hashset is not deterministic
            let extras_str = extras.join(",");
            dep += extras_str.as_str();
            dep += "]";
        }
        dep = match &self.version_spec {
            Some(v) => dep + format!("{}{}", v.0, v.1).as_str(),
            None => dep,
        };
        dep = match &self.markers {
            Some(m) => dep + format!("; {m}").as_str(),
            None => dep,
        };
        write!(f, "{dep}")
    }
}

impl PartialEq for Dependency {
    fn eq(&self, other: &Self) -> bool {
        self.name.to_lowercase() == other.name.to_lowercase()
    }
}

impl Hash for Dependency {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.to_lowercase().hash(state);
    }
}

impl Dependency {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn to_dependency_repr(&self) -> String {
        let mut dep = String::new();
        dep += self.name.as_str();
        if !&self.extras.is_empty() {
            dep += "[";
            let mut extras = self
                .extras
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>();

            extras.sort(); // Needs to be sorted because iterating over a hashset is not deterministic
            let extras_str = extras.join(",");
            dep += extras_str.as_str();
            dep += "]";
        }
        dep = match &self.version_spec {
            Some(v) => dep + format!("{}{}", v.0, v.1).as_str(),
            None => dep,
        };
        dep = match &self.markers {
            Some(m) => dep + format!("; {m}").as_str(),
            None => dep,
        };
        dep
    }
    pub fn parse(input: &str) -> Option<Self> {
        let re = Regex::new(r#"^([A-Za-z0-9\-_.]+)(?:\[(.*?)\])?(?:\s*([~=<>!]={1,2}|[<>]|\^)\s*([\d\w\-.]+))?\s*(?:;\s*(.+))?"#).unwrap();

        let caps = re.captures(input)?;

        let name = caps.get(1)?.as_str().to_string();

        let extras = caps
            .get(2)
            .map(|m| {
                m.as_str()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect()
            })
            .unwrap_or_default();

        let version_spec = match (caps.get(3), caps.get(4)) {
            (Some(spec), Some(ver)) => Some((spec.as_str().to_string(), ver.as_str().to_string())),
            _ => None,
        };

        let markers = caps.get(5).map(|m| m.as_str().to_string());

        Some(Dependency {
            name,
            extras,
            version_spec,
            markers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_parse_versionless() {
        let candidate = "Django";
        let res = Dependency::parse(candidate).unwrap();
        assert_eq!(res.name, "Django");
    }

    #[test]
    fn test_can_parse_versioned() {
        let candidate = "Django~=3.2";
        let res = Dependency::parse(candidate).unwrap();
        assert_eq!(res.name, "Django");
        assert_eq!(res.version_spec, Some(("~=".into(), "3.2".into())));
    }

    #[test]
    fn test_can_parse_extras() {
        let candidate = "Django[mysql,redis]";
        let res = Dependency::parse(candidate).unwrap();
        assert_eq!(res.name, "Django");
        assert!(res.extras.contains("mysql"));
        assert!(res.extras.contains("redis"));
    }

    #[test]
    fn test_can_parse_markers() {
        let candidate = "pandas; platform_system != 'Windows'";
        let res = Dependency::parse(candidate).unwrap();
        assert_eq!(res.name, "pandas");
        assert_eq!(res.markers, Some("platform_system != 'Windows'".into()));
    }

    #[test]
    fn test_can_parse_full_declaration() {
        let candidate = "pandas[excel,postgres]>=1.3.0; platform_system != 'Windows'";
        let res = Dependency::parse(candidate).unwrap();
        assert_eq!(res.name, "pandas");
        assert!(res.extras.contains("excel"));
        assert!(res.extras.contains("postgres"));
        assert_eq!(res.version_spec, Some((">=".into(), "1.3.0".into())));
        assert_eq!(res.markers, Some("platform_system != 'Windows'".into()));
    }

    #[test]
    fn test_display() {
        let candidate = "pandas[excel,postgres]>=1.3.0; platform_system != 'Windows'";
        let dep = Dependency::parse(candidate).unwrap();
        assert_eq!(format!("{dep}"), candidate);
    }
}

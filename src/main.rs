use clap::Parser;
use std::path::PathBuf;

use cli::Args;
use config::{load_config, Config};
use engine::{DetectEngineError, EngineOptions};
use log::info;
use simple_logger::SimpleLogger;

mod cli;
mod config;
mod dependency;
mod engine;
mod pyproject;

fn merge_args_and_config(args: Args, config: Config) -> EngineOptions {
    EngineOptions {
        // CLI args take precedence; append config defaults if not provided
        exclude_dirs: if !args.exclude_dirs.is_empty() {
            args.exclude_dirs
        } else {
            config.exclude_dirs.unwrap_or_default()
        },
        extra_indexes: if !args.extra_indexes.is_empty() {
            args.extra_indexes
        } else {
            config.extra_indexes.unwrap_or_default()
        },
        preferred_index: args.preferred_index.or(config.preferred_index),
        extras_to_remap: if !args.remap.is_empty() {
            args.remap.into_iter().collect()
        } else {
            config.remap.unwrap_or_default()
        },
    }
}

fn main() -> Result<(), DetectEngineError> {
    SimpleLogger::new()
        .env()
        .with_level(log::LevelFilter::Info)
        .without_timestamps()
        .init()
        .unwrap();

    let args = Args::parse();
    let config = load_config();
    let options = merge_args_and_config(args, config);

    let pyproject_path = PathBuf::from("./pyproject.toml");
    let pyproject = pyproject::read(&pyproject_path).unwrap();
    let engine = engine::DetectEngine::new(pyproject.clone(), options);
    let deps = engine.detect_dependencies(PathBuf::from("."))?;

    if deps.is_empty() {
        info!("No new dependencies detected, nothing to do");
        return Ok(());
    }

    match pyproject::write(&pyproject_path, pyproject, deps) {
        Ok(_) => info!("Updated pyproject.toml"),
        Err(e) => panic!("Failed to write deps to pyproject.toml: {e:?}"),
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn default_args() -> Args {
        Args {
            exclude_dirs: Vec::new(),
            extra_indexes: Vec::new(),
            preferred_index: None,
            remap: Vec::new(),
        }
    }

    fn default_config() -> Config {
        Config {
            exclude_dirs: None,
            extra_indexes: None,
            preferred_index: None,
            remap: None,
        }
    }

    #[test]
    fn test_empty_args_and_config() {
        let args = default_args();
        let config = default_config();
        let options = merge_args_and_config(args, config);

        assert_eq!(
            options,
            EngineOptions {
                exclude_dirs: Vec::new(),
                extra_indexes: Vec::new(),
                preferred_index: None,
                extras_to_remap: HashMap::new(),
            },
            "Empty args and config should return empty options"
        );
    }

    #[test]
    fn test_args_only() {
        let mut args = default_args();
        args.exclude_dirs = vec!["build".to_string(), "dist".to_string()];
        args.extra_indexes = vec!["https://test.pypi.org/simple/".to_string()];
        args.preferred_index = Some("https://pypi.org/simple/".to_string());
        args.remap = vec![("old".to_string(), "new".to_string())];

        let config = default_config();
        let options = merge_args_and_config(args, config);

        let mut expected_remap = HashMap::new();
        expected_remap.insert("old".to_string(), "new".to_string());

        assert_eq!(
            options,
            EngineOptions {
                exclude_dirs: vec!["build".to_string(), "dist".to_string()],
                extra_indexes: vec!["https://test.pypi.org/simple/".to_string()],
                preferred_index: Some("https://pypi.org/simple/".to_string()),
                extras_to_remap: expected_remap,
            },
            "Args should take precedence when config is empty"
        );
    }

    #[test]
    fn test_config_only() {
        let args = default_args();

        let mut config = default_config();
        config.exclude_dirs = Some(vec![".venv".to_string(), ".git".to_string()]);
        config.extra_indexes = Some(vec!["https://company.pypi.org/simple/".to_string()]);
        config.preferred_index = Some("https://custom.pypi.org/simple/".to_string());
        let mut remap = HashMap::new();
        remap.insert(
            "rest_framework".to_string(),
            "djangorestframework".to_string(),
        );
        config.remap = Some(remap.clone());

        let options = merge_args_and_config(args, config);

        assert_eq!(
            options,
            EngineOptions {
                exclude_dirs: vec![".venv".to_string(), ".git".to_string()],
                extra_indexes: vec!["https://company.pypi.org/simple/".to_string()],
                preferred_index: Some("https://custom.pypi.org/simple/".to_string()),
                extras_to_remap: remap,
            },
            "Config should be used when args are empty"
        );
    }

    #[test]
    fn test_args_override_config() {
        let mut args = default_args();
        args.exclude_dirs = vec!["dist".to_string()];
        args.preferred_index = Some("https://override.pypi.org/simple/".to_string());
        args.remap = vec![("old".to_string(), "new".to_string())];

        let mut config = default_config();
        config.exclude_dirs = Some(vec![".venv".to_string(), ".git".to_string()]);
        config.extra_indexes = Some(vec!["https://company.pypi.org/simple/".to_string()]);
        config.preferred_index = Some("https://custom.pypi.org/simple/".to_string());
        let mut config_remap = HashMap::new();
        config_remap.insert(
            "rest_framework".to_string(),
            "djangorestframework".to_string(),
        );
        config.remap = Some(config_remap);

        let options = merge_args_and_config(args, config);

        let mut expected_remap = HashMap::new();
        expected_remap.insert("old".to_string(), "new".to_string());

        assert_eq!(
            options,
            EngineOptions {
                exclude_dirs: vec!["dist".to_string()],
                extra_indexes: vec!["https://company.pypi.org/simple/".to_string()],
                preferred_index: Some("https://override.pypi.org/simple/".to_string()),
                extras_to_remap: expected_remap,
            },
            "Args should override config where provided"
        );
    }

    #[test]
    fn test_partial_args_with_config() {
        let mut args = default_args();
        args.extra_indexes = vec!["https://test.pypi.org/simple/".to_string()];

        let mut config = default_config();
        config.exclude_dirs = Some(vec!["build".to_string()]);
        config.preferred_index = Some("https://custom.pypi.org/simple/".to_string());
        let mut remap = HashMap::new();
        remap.insert("key".to_string(), "value".to_string());
        config.remap = Some(remap.clone());

        let options = merge_args_and_config(args, config);

        assert_eq!(
            options,
            EngineOptions {
                exclude_dirs: vec!["build".to_string()],
                extra_indexes: vec!["https://test.pypi.org/simple/".to_string()],
                preferred_index: Some("https://custom.pypi.org/simple/".to_string()),
                extras_to_remap: remap,
            },
            "Args and config should merge correctly when partially provided"
        );
    }
}

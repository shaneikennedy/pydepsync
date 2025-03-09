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
            config.exclude_dirs.unwrap_or_else(|| Vec::new())
        },
        extra_indexes: if !args.extra_indexes.is_empty() {
            args.extra_indexes
        } else {
            config.extra_indexes.unwrap_or_else(|| Vec::new())
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
        Err(e) => panic!("Failed to write deps to pyproject.toml: {:?}", e),
    };
    Ok(())
}

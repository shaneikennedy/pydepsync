use std::path::PathBuf;

use clap::Parser;
use engine::{DetectEngineError, EngineOptions};
use log::info;
use simple_logger::SimpleLogger;

mod dependency;
mod engine;
mod pyproject;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// List of directories to ignore, we ignore .venv and .git by default
    #[arg(long)]
    exclude_dirs: Vec<String>,

    /// List of extra package indexes pydepsync should check when resolving dependencies. We check https://pypi.org/simple by default.
    #[arg(long)]
    extra_indexes: Vec<String>,

    /// The index pydepsync should check first when resolving packages
    #[arg(long)]
    prefered_index: Option<String>,

    /// List of key-value pairs in the format 'key=value'
    #[arg(
        short,
        long,
        value_name = "KEY=VALUE",
        value_parser = remap_parser,
        number_of_values = 1,
        action = clap::ArgAction::Append
    )]
    remap: Vec<(String, String)>,
}

fn remap_parser(s: &str) -> Result<(String, String), String> {
    match s.split_once('=') {
        Some((key, value)) => {
            if key.is_empty() {
                Err("Key cannot be empty".to_string())
            } else {
                Ok((key.to_string(), value.to_string()))
            }
        }
        None => Err("Invalid key-value pair format. Use 'key=value'".to_string()),
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
    let options = EngineOptions {
        exclude_dirs: args.exclude_dirs,
        extra_indexes: args.extra_indexes,
        preferred_index: args.prefered_index,
        extras_to_remap: args.remap.into_iter().collect(),
    };
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

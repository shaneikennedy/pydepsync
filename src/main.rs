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

    /// List of extra package indexes pydepsync should check when resolving dependencies. We check https://pypi.org/pypi by default.
    #[arg(long)]
    extra_indexes: Vec<String>,

    /// The index pydepsync should check first when resolving packages
    #[arg(long)]
    prefered_index: Option<String>,
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
    };
    let pyproject_path = PathBuf::from("./pyproject.toml");
    let pyproject = pyproject::read(&pyproject_path).unwrap();
    let engine = engine::DetectEngine::new(pyproject.clone(), options);
    let deps = engine.detect_dependencies(PathBuf::from("."))?;

    if deps.is_empty() {
        info!("No dependencies detected, nothing to do");
        return Ok(());
    }

    match pyproject::write(&pyproject_path, pyproject, deps) {
        Ok(_) => info!("Updated pyproject.toml"),
        Err(e) => panic!("Failed to write deps to pyproject.toml: {:?}", e),
    };
    Ok(())
}

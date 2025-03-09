use clap::Parser;

#[derive(PartialEq, Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// List of directories to ignore, we ignore .venv and .git by default
    #[arg(long)]
    pub exclude_dirs: Vec<String>,

    /// List of extra package indexes pydepsync should check when resolving dependencies. We check https://pypi.org/simple by default.
    #[arg(long)]
    pub extra_indexes: Vec<String>,

    /// The index pydepsync should check first when resolving packages
    #[arg(long)]
    pub preferred_index: Option<String>,

    /// List of key-value pairs in the format 'key=value'
    #[arg(
        short,
        long,
        value_name = "KEY=VALUE",
        value_parser = remap_parser,
        number_of_values = 1,
        action = clap::ArgAction::Append
    )]
    pub remap: Vec<(String, String)>,
}

pub fn remap_parser(s: &str) -> Result<(String, String), String> {
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

# pydepsync

Detect external package dependencies in your code and add the missing ones to pyproject.toml

A Rust port of the excellent package [pipreqs](https://github.com/bndr/pipreqs), but for pyproject.toml

## Usage

Run `pydepsync` in the root of a project with a `pyproject.toml` file to scan your code and update dependencies

## Use-cases

- Vibe coding and claude keeps adding dependencies everywhere? pydepsync
- Migrating from poetry, pipenv, or just plain pip? pydepsync

## Installation

### From Binary Releases

Download the latest binary for your platform from the [releases page](https://github.com/shaneikennedy/pydepsync/releases/latest), extract it, and place it in your PATH.

### From Source

Install from source using Cargo (requires Rust to be installed):

```bash
cargo install --git https://github.com/shaneikennedy/pydepsync
```

## Usage

```sh
uvproject git:main
❯ pydepsync --help
Usage: pydepsync [OPTIONS]

Options:
      --exclude-dirs <EXCLUDE_DIRS>
          List of directories to ignore, we ignore .venv and .git by default
      --extra-indexes <EXTRA_INDEXES>
          List of extra package indexes pydepsync should check when resolving dependencies. We check https://pypi.org/simple by default
      --preferred-index <PREFERRED_INDEX>
          The index pydepsync should check first when resolving packages
  -r, --remap <KEY=VALUE>
          List of key-value pairs in the format 'key=value'
  -h, --help
          Print help
  -V, --version
          Print version
```

## Configuration

To avoid repeating CLI arguments, especially for private indexes or remapped packages, create a `.pydepsync.toml` file in your project root (next to `pyproject.toml`). CLI arguments override these settings.

Example:

```toml
# .pydepsync.toml

# Directories to exclude (array of strings)
# .venv and .git are ignored by default; no need to list them unless overriding
exclude_dirs = ["build", "dist"]

# Extra package indexes to check (array of strings)
extra_indexes = ["https://test.pypi.org/simple/", "https://mycompany.pypi.org/simple/"]

# Preferred index to check first (optional string)
# Defaults to https://pypi.org/simple/ if omitted
preferred_index = "https://pypi.org/simple/"

# Remappings for import-to-package-name mismatches
[remap]
"rest_framework" = "djangorestframework"  # Built-in for 1000+ public packages
"how_its_imported" = "WhatItsNamedOnIndex"
```

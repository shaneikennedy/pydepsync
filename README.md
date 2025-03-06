# pydepsync

Detect external package dependencies in your code and add the missing ones to pyproject.toml

A rust port of the excellent package [pipreqs](https://github.com/bndr/pipreqs), but for pyproject.toml

## Usage

`pydepsync` in the root of a pyproject.toml managed project

## Use-cases

- Vibe coding and claude keeps adding dependencies everywhere? pydepsync
- Migrating from poetry, pipenv, or just plain pip? pydepsync

## Installation

### From Binary Releases

Download the latest binary for your platform from the [releases page](https://github.com/shaneikennedy/pydepsync/releases/latest).

### From Source
```bash
cargo install --git https://github.com/shaneikennedy/pydepsync
```

## Demo

```sh
pydepsync git:main
❯ pwd
/Users/shane.kennedy/dev/shane/pydepsync

pydepsync git:main
❯ cargo build
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.14s

pydepsync git:main
❯ target/debug/pydepsync --help
Usage: pydepsync [OPTIONS]

Options:
      --exclude-dirs <EXCLUDE_DIRS>
          List of directories to ignore, we ignore .venv and .git by default
      --extra-indexes <EXTRA_INDEXES>
          List of extra package indexes pydepsync should check when resolving dependencies. We check https://pypi.org/pypi by default
      --prefered-index <PREFERED_INDEX>
          The index pydepsync should check first when resolving packages
  -h, --help
          Print help
  -V, --version
          Print version
pydepsync git:main
❯ cd example_app

pydepsync/example_app git:main
❯ cat pyproject.toml
[project]
name = "example"
version = "0.1.0"
description = "Add your description here"
readme = "README.md"
requires-python = ">=3.13"
dependencies = []

pydepsync/example_app git:main
❯ ../target/debug/pydepsync
INFO  [pydepsync::engine] Reading your code...
INFO  [pydepsync::engine] Parsing imports...
INFO  [pydepsync::engine] Evaluating candidates...
INFO  [pydepsync::engine] Resolving packages...
INFO  [pydepsync::pyproject] Adding: Django~=5.1.6
INFO  [pydepsync::pyproject] Adding: djangorestframework~=3.15.2
INFO  [pydepsync] Updated pyproject.toml

pydepsync/example_app git:main
❯ cat pyproject.toml
[project]
name = "example"
version = "0.1.0"
description = "Add your description here"
readme = "README.md"
requires-python = ">=3.13"
dependencies = [
    "Django~=5.1.6",
    "djangorestframework~=3.15.2"
]
```

### ToDos

- [x] Make it configurable via cli flags
- [x] Resolve against package registries for proper version speccing
- [x] Don't overwrite current contents
- [x] Real logging
- [x] Handle optional deps, don't try to figure out which option they are but don't overwrite or redeclare them when they exist already
- [x] Allow extra package indexes
- [x] Allow "preferable" package index, i.e check there first
- [ ] Add a dotfile for saving configurattion
- [x] Do better than panicing everywhere
- [x] Make it fast

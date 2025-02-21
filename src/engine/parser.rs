use rustpython_parser::parse;
use std::io;

pub fn extract_dependencies(py_code: &str) -> Result<Vec<String>, io::Error> {
    let ast = parse(py_code, rustpython_parser::Mode::Module, "<embedded>").unwrap();
    let module = ast.module();
    let mut imports = match &module {
        Some(m) => m
            .body
            .iter()
            .filter(|&s| s.clone().import_stmt().is_some())
            .flat_map(|i| i.clone().import_stmt().unwrap().names)
            .map(|i| i.name.to_string())
            .collect(),
        None => Vec::new(),
    };
    let from_imports = match &module {
        Some(m) => m
            .body
            .iter()
            .filter(|&s| s.clone().import_from_stmt().is_some())
            .flat_map(|i| i.clone().import_from_stmt().unwrap().module)
            .map(|i| i.to_string())
            .collect(),
        None => Vec::new(),
    };
    imports.extend(from_imports);
    Ok(imports)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finds_both_types_of_imports() -> Result<(), io::Error> {
        let code = r#"
from django import db
import os
def is_odd(i):
  return bool(i & 1)
"#;
        let imports = extract_dependencies(code).unwrap();
        assert_eq!(imports.len(), 2); // Should only find test1.py
        assert!(imports.contains(&String::from("django")));
        assert!(imports.contains(&String::from("os")));
        Ok(())
    }
}

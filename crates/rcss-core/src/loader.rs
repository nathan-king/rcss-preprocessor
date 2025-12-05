use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Load an RCSS file with recursive imports inlined.
pub fn load_with_imports(path: &Path) -> Result<String, String> {
    let mut visited = HashSet::new();
    let mut stack = HashSet::new();
    load_recursive(path, &mut visited, &mut stack)
}

fn load_recursive(
    path: &Path,
    visited: &mut HashSet<PathBuf>,
    stack: &mut HashSet<PathBuf>,
) -> Result<String, String> {
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve {}: {}", path.display(), e))?;

    if stack.contains(&canonical) {
        return Err(format!(
            "Recursive import detected: {}",
            canonical.display()
        ));
    }

    if visited.contains(&canonical) {
        return Ok(String::new());
    }

    stack.insert(canonical.clone());

    let data = fs::read_to_string(&canonical)
        .map_err(|e| format!("Failed to read {}: {}", canonical.display(), e))?;

    let mut out = String::new();
    for line in data.lines() {
        if let Some(target) = parse_import_line(line) {
            let import_path = canonical
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(target);
            let imported = load_recursive(&import_path, visited, stack)?;
            out.push_str(&imported);
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }

    stack.remove(&canonical);
    visited.insert(canonical);
    Ok(out)
}

fn parse_import_line(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if !trimmed.starts_with("@import") {
        return None;
    }
    let rest = trimmed["@import".len()..].trim_start();
    if rest.is_empty() || !rest.ends_with(';') {
        return None;
    }
    let path_literal = rest[..rest.len() - 1].trim();
    if path_literal.len() < 2 {
        return None;
    }

    let bytes = path_literal.as_bytes();
    let first = bytes[0] as char;
    let last = bytes[bytes.len() - 1] as char;
    if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
        Some(&path_literal[1..path_literal.len() - 1])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/imports")
    }

    #[test]
    fn inline_nested_imports_once() {
        let path = fixture_dir().join("b.rcss");
        let combined = load_with_imports(&path).expect("load imports");
        assert!(combined.contains(".a"));
        assert!(combined.contains(".b"));
        assert!(combined.contains(".c"));
        assert!(!combined.contains("import"));
    }

    #[test]
    fn skip_duplicate_imports() {
        let path = fixture_dir().join("b-dup.rcss");
        let combined = load_with_imports(&path).expect("load imports");
        assert_eq!(combined.matches(".a {").count(), 1);
    }

    #[test]
    fn detect_cycles() {
        let path = fixture_dir().join("cycle-a.rcss");
        let err = load_with_imports(&path).unwrap_err();
        assert!(err.contains("Recursive import detected"));
    }

    #[test]
    fn missing_file_error() {
        let path = fixture_dir().join("missing.rcss");
        let err = load_with_imports(&path).unwrap_err();
        assert!(err.contains("Failed to read") || err.contains("Failed to resolve"));
    }
}

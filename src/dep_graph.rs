use crate::error::PipelineError;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Dependency graph with adjacency edges and cycle flag.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DepGraph {
    pub edges: HashMap<String, Vec<String>>,
    pub has_cycle: bool,
}

/// Build a dependency graph by scanning `@./relative@` and `@path@` USD asset references
/// found in `.usda` files under `root_path`.
pub fn build_graph(root_path: &str) -> Result<DepGraph, PipelineError> {
    let mut edges: HashMap<String, Vec<String>> = HashMap::new();

    let root = std::path::Path::new(root_path);
    let files: Vec<_> = if root.is_file() {
        vec![root.to_path_buf()]
    } else {
        walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s == "usda")
                    .unwrap_or(false)
            })
            .map(|e| e.path().to_path_buf())
            .collect()
    };

    for file in &files {
        let content = std::fs::read_to_string(file)?;
        let key = file.to_string_lossy().into_owned();
        let refs = extract_refs(&content);
        edges.insert(key, refs);
    }

    let has_cycle = detect_cycle(&edges);
    Ok(DepGraph { edges, has_cycle })
}

/// Extract USD asset reference paths from file content.
/// Matches patterns like `@./path/to/file.usda@` or `@path/to/file.usda@`.
fn extract_refs(content: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut chars = content.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '@' {
            let mut inner = String::new();
            for nc in chars.by_ref() {
                if nc == '@' {
                    break;
                }
                inner.push(nc);
            }
            let trimmed = inner.trim().to_string();
            if !trimmed.is_empty() {
                refs.push(trimmed);
            }
        }
    }
    refs
}

/// DFS cycle detection on the adjacency map.
fn detect_cycle(edges: &HashMap<String, Vec<String>>) -> bool {
    let mut visited = HashSet::new();
    let mut stack = HashSet::new();

    for node in edges.keys() {
        if dfs(node, edges, &mut visited, &mut stack) {
            return true;
        }
    }
    false
}

fn dfs<'a>(
    node: &'a str,
    edges: &'a HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    stack: &mut HashSet<String>,
) -> bool {
    if stack.contains(node) {
        return true;
    }
    if visited.contains(node) {
        return false;
    }
    visited.insert(node.to_owned());
    stack.insert(node.to_owned());

    if let Some(neighbors) = edges.get(node) {
        for n in neighbors {
            if dfs(n, edges, visited, stack) {
                return true;
            }
        }
    }
    stack.remove(node);
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_refs_finds_paths() {
        let content = r#"references = [@./chars/hero.usda@, @./envs/set.usda@]"#;
        let refs = extract_refs(content);
        assert_eq!(refs.len(), 2);
        assert!(refs.iter().any(|r| r.contains("hero.usda")));
    }

    #[test]
    fn test_extract_refs_empty() {
        let refs = extract_refs("def Xform \"Root\" {}");
        assert!(refs.is_empty());
    }

    #[test]
    fn test_detect_cycle_none() {
        let mut edges = HashMap::new();
        edges.insert("a".to_owned(), vec!["b".to_owned()]);
        edges.insert("b".to_owned(), vec!["c".to_owned()]);
        edges.insert("c".to_owned(), vec![]);
        assert!(!detect_cycle(&edges));
    }

    #[test]
    fn test_detect_cycle_found() {
        let mut edges = HashMap::new();
        edges.insert("a".to_owned(), vec!["b".to_owned()]);
        edges.insert("b".to_owned(), vec!["c".to_owned()]);
        edges.insert("c".to_owned(), vec!["a".to_owned()]); // cycle
        assert!(detect_cycle(&edges));
    }

    #[test]
    fn test_build_graph_single_file() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(f, r#"references = [@./sub.usda@]"#).unwrap();
        let path = f.path().to_str().unwrap();
        let graph = build_graph(path).unwrap();
        assert!(graph.edges.contains_key(path));
    }
}

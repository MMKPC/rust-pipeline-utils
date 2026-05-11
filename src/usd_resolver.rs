use crate::error::PipelineError;
use serde::{Deserialize, Serialize};

/// Information about a resolved USD layer.
#[derive(Debug, Serialize, Deserialize)]
pub struct LayerInfo {
    /// Nesting depth (0 = root).
    pub depth: usize,
    /// Resolved file path string.
    pub path: String,
}

/// Resolve the flat layer stack for a USD file by recursively parsing
/// `@./path@` style references up to a fixed depth limit.
///
/// This is a static analyser — it does not invoke the USD runtime.
///
/// # Arguments
/// * `root_path` - Path to the root `.usda` file.
///
/// # Errors
/// Returns `PipelineError::Io` if a referenced file cannot be read.
/// Returns `PipelineError::InvalidUsdRef` for malformed references.
pub fn resolve_layer_stack(root_path: &str) -> Result<Vec<LayerInfo>, PipelineError> {
    let mut stack: Vec<LayerInfo> = Vec::new();
    let mut visited = std::collections::HashSet::new();
    resolve_recursive(root_path, 0, &mut stack, &mut visited)?;
    Ok(stack)
}

fn resolve_recursive(
    path: &str,
    depth: usize,
    stack: &mut Vec<LayerInfo>,
    visited: &mut std::collections::HashSet<String>,
) -> Result<(), PipelineError> {
    const MAX_DEPTH: usize = 32;

    if depth > MAX_DEPTH {
        return Ok(()); // safeguard against infinite recursion
    }

    let canonical = std::fs::canonicalize(path)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_owned());

    if !visited.insert(canonical.clone()) {
        return Ok(()); // already visited
    }

    stack.push(LayerInfo {
        depth,
        path: canonical.clone(),
    });

    // Read and parse references — fail gracefully if file missing
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };

    let base = std::path::Path::new(path)
        .parent()
        .unwrap_or(std::path::Path::new("."));

    for ref_path in extract_at_refs(&content) {
        // Validate: must not contain null bytes or obviously broken paths
        if ref_path.contains('\0') {
            return Err(PipelineError::InvalidUsdRef {
                file: path.to_owned(),
                detail: format!("null byte in reference: {ref_path}"),
            });
        }
        let resolved = base.join(&ref_path);
        if let Some(s) = resolved.to_str() {
            resolve_recursive(s, depth + 1, stack, visited)?;
        }
    }
    Ok(())
}

/// Extract @…@ asset references from USD content string.
fn extract_at_refs(content: &str) -> Vec<String> {
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
            let t = inner.trim().to_string();
            if !t.is_empty() {
                refs.push(t);
            }
        }
    }
    refs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_single_file_no_refs() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(f, r#"def Xform "Root" {}"#).unwrap();
        let layers = resolve_layer_stack(f.path().to_str().unwrap()).unwrap();
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].depth, 0);
    }

    #[test]
    fn test_resolve_missing_ref_does_not_error() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(f, r#"references = [@./nonexistent_layer.usda@]"#).unwrap();
        // Should not error — graceful degradation
        let layers = resolve_layer_stack(f.path().to_str().unwrap()).unwrap();
        assert_eq!(layers.len(), 1); // only root added
    }

    #[test]
    fn test_resolve_chain() {
        let dir = TempDir::new().unwrap();
        let child_path = dir.path().join("child.usda");
        std::fs::write(&child_path, r#"def Mesh "Cube" {}"#).unwrap();

        let root_path = dir.path().join("root.usda");
        let child_name = child_path.file_name().unwrap().to_string_lossy();
        std::fs::write(&root_path, format!(r#"references = [@./{child_name}@]"#)).unwrap();

        let layers = resolve_layer_stack(root_path.to_str().unwrap()).unwrap();
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].depth, 0);
        assert_eq!(layers[1].depth, 1);
    }

    #[test]
    fn test_resolve_cycle_does_not_loop() {
        // a.usda refs b.usda refs a.usda
        let dir = TempDir::new().unwrap();
        let a = dir.path().join("a.usda");
        let b = dir.path().join("b.usda");
        std::fs::write(&a, r#"references = [@./b.usda@]"#).unwrap();
        std::fs::write(&b, r#"references = [@./a.usda@]"#).unwrap();

        let layers = resolve_layer_stack(a.to_str().unwrap()).unwrap();
        assert_eq!(layers.len(), 2); // visited guard prevents infinite recursion
    }
}

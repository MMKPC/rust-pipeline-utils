use crate::error::PipelineError;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

/// Result of processing a single file.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessResult {
    pub source: String,
    pub dest: String,
    pub bytes: u64,
}

/// Process all files with a given extension in `input_dir`, copying them to `output_dir`.
///
/// Uses Rayon for parallel file I/O.
///
/// # Arguments
/// * `input_dir`  - Source directory path.
/// * `output_dir` - Destination directory path (created if absent).
/// * `ext`        - File extension filter (without leading dot, e.g. `"usda"`).
///
/// # Errors
/// Returns `PipelineError::Io` on filesystem errors.
pub fn process_directory(
    input_dir: &str,
    output_dir: &str,
    ext: &str,
) -> Result<Vec<ProcessResult>, PipelineError> {
    let out = Path::new(output_dir);
    std::fs::create_dir_all(out)?;

    // Collect matching files
    let entries: Vec<_> = WalkDir::new(input_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case(ext))
                    .unwrap_or(false)
        })
        .collect();

    let results: Vec<Result<ProcessResult, PipelineError>> = entries
        .par_iter()
        .map(|entry| {
            let src = entry.path();
            let filename = src.file_name().unwrap_or_default();
            let dst = out.join(filename);
            let bytes = std::fs::metadata(src)?.len();
            std::fs::copy(src, &dst)?;
            Ok(ProcessResult {
                source: src.to_string_lossy().into_owned(),
                dest: dst.to_string_lossy().into_owned(),
                bytes,
            })
        })
        .collect();

    let mut ok = Vec::new();
    for r in results {
        ok.push(r?);
    }
    Ok(ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn make_dir_with_files(ext: &str, count: usize) -> TempDir {
        let dir = TempDir::new().unwrap();
        for i in 0..count {
            let path = dir.path().join(format!("asset_{i}.{ext}"));
            let mut f = std::fs::File::create(path).unwrap();
            write!(f, "content {i}").unwrap();
        }
        dir
    }

    #[test]
    fn test_process_copies_matching_files() {
        let src = make_dir_with_files("usda", 3);
        let dst = TempDir::new().unwrap();
        let results = process_directory(
            src.path().to_str().unwrap(),
            dst.path().to_str().unwrap(),
            "usda",
        )
        .unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_process_filters_by_extension() {
        let src = make_dir_with_files("fbx", 2);
        let dst = TempDir::new().unwrap();
        // Request .usda but dir has .fbx — should return 0
        let results = process_directory(
            src.path().to_str().unwrap(),
            dst.path().to_str().unwrap(),
            "usda",
        )
        .unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_process_creates_output_dir() {
        let src = make_dir_with_files("usda", 1);
        let dst_path = TempDir::new().unwrap().path().join("subdir/output");
        let _ = process_directory(
            src.path().to_str().unwrap(),
            dst_path.to_str().unwrap(),
            "usda",
        )
        .unwrap();
        assert!(dst_path.exists());
    }
}

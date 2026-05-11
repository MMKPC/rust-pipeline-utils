use crate::error::PipelineError;
use std::fs;
use std::io::Read;

/// Generate a BLAKE3 hex digest for a file.
///
/// # Arguments
/// * `path` - Filesystem path to the file.
///
/// # Errors
/// Returns `PipelineError::Io` if the file cannot be read.
pub fn generate(path: &str) -> Result<String, PipelineError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

/// Verify that a file matches the expected BLAKE3 hex digest.
///
/// # Arguments
/// * `path`     - Filesystem path.
/// * `expected` - Expected hex digest string.
///
/// # Returns
/// `Ok(true)` if the digest matches, `Ok(false)` otherwise.
pub fn verify(path: &str, expected: &str) -> Result<bool, PipelineError> {
    let actual = generate(path)?;
    Ok(actual.eq_ignore_ascii_case(expected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_generate_returns_hex_string() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"hello pipeline").unwrap();
        let hash = generate(f.path().to_str().unwrap()).unwrap();
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_verify_correct_hash() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"mmkpc").unwrap();
        let hash = generate(f.path().to_str().unwrap()).unwrap();
        let ok = verify(f.path().to_str().unwrap(), &hash).unwrap();
        assert!(ok);
    }

    #[test]
    fn test_verify_wrong_hash() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"mmkpc").unwrap();
        let ok = verify(f.path().to_str().unwrap(), "deadbeef").unwrap();
        assert!(!ok);
    }

    #[test]
    fn test_generate_missing_file_returns_error() {
        let result = generate("/nonexistent/path/file.usda");
        assert!(result.is_err());
    }

    #[test]
    fn test_deterministic() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"deterministic content").unwrap();
        let p = f.path().to_str().unwrap();
        let h1 = generate(p).unwrap();
        let h2 = generate(p).unwrap();
        assert_eq!(h1, h2);
    }
}

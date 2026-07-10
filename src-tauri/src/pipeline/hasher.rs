use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::AppError;

/// Compute a stable SHA-256 hex digest of a file's content.
///
/// Returns a 64-character lowercase hex string. Identical file content will
/// always produce the same hash; any byte change produces a different hash.
pub fn content_hash(path: &Path) -> Result<String, AppError> {
    let mut file = std::fs::File::open(path).map_err(AppError::from)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 4096];

    loop {
        let n = file.read(&mut buffer).map_err(AppError::from)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_hash_returns_64_char_hex() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "some content").unwrap();
        let hash = content_hash(f.path()).unwrap();
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_identical_content_identical_hash() {
        let mut f1 = NamedTempFile::new().unwrap();
        write!(f1, "same content").unwrap();
        let mut f2 = NamedTempFile::new().unwrap();
        write!(f2, "same content").unwrap();
        assert_eq!(content_hash(f1.path()).unwrap(), content_hash(f2.path()).unwrap());
    }

    #[test]
    fn test_different_content_different_hash() {
        let mut f1 = NamedTempFile::new().unwrap();
        write!(f1, "content A").unwrap();
        let mut f2 = NamedTempFile::new().unwrap();
        write!(f2, "content B").unwrap();
        assert_ne!(content_hash(f1.path()).unwrap(), content_hash(f2.path()).unwrap());
    }

    #[test]
    fn test_hash_is_deterministic() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "deterministic test").unwrap();
        let h1 = content_hash(f.path()).unwrap();
        let h2 = content_hash(f.path()).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_missing_file_returns_io_error() {
        let err = content_hash(Path::new("/tmp/does_not_exist_cortex_test.bin")).unwrap_err();
        match err {
            AppError::Io(_) => {}
            other => panic!("Expected Io error, got {:?}", other),
        }
    }
}

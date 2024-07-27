use crate::error::AybError;
use blake3::Hasher;
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

pub fn hash_db_directory(path: &Path) -> Result<String, AybError> {
    let mut paths = fs::read_dir(path)?
        .map(|entry| {
            let entry_path = entry?.path();
            if entry_path.is_file() {
                Ok(entry_path)
            } else {
                Err(AybError::SnapshotError {
                    message: format!(
                        "Unexpected non-file path in database directory: {}",
                        entry_path.display()
                    ),
                })
            }
        })
        .collect::<Result<Vec<PathBuf>, AybError>>()?;
    // Sort alphabetically to ensure determinism.
    paths.sort();

    let mut hasher = Hasher::new();
    for entry in paths {
        let file = fs::File::open(entry)?;
        let mut reader = BufReader::new(file);
        let mut buffer = [0; 4096];
        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
    }
    Ok(hasher.finalize().to_hex().to_string())
}

use crate::error::AybError;
use blake3::Hasher;
use std::fs;
use std::io::{BufReader, Read};
use std::ops::Range;
use std::path::{Path, PathBuf};

/// Hashes the files in a snapshot directory. Bytes in `ignored_bytes`
/// (offsets within each file) are hashed as zeros.
pub fn hash_db_directory(path: &Path, ignored_bytes: &[Range<usize>]) -> Result<String, AybError> {
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
    // Sort alphabetically to ensure hash is deterministic.
    paths.sort();

    let mut hasher = Hasher::new();
    for entry in paths {
        let file = fs::File::open(entry)?;
        let mut reader = BufReader::new(file);
        let mut buffer = [0; 4096];
        let mut offset = 0;
        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            let chunk = &mut buffer[..bytes_read];
            for range in ignored_bytes {
                let start = range.start.clamp(offset, offset + bytes_read) - offset;
                let end = range.end.clamp(offset, offset + bytes_read) - offset;
                chunk[start..end].fill(0);
            }
            hasher.update(chunk);
            offset += bytes_read;
        }
    }
    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::hash_db_directory;
    use crate::ayb_db::models::DBType;
    use crate::hosted_db::{engine_for, QueryMode};
    use std::fs;
    use std::path::{Path, PathBuf};

    // Snapshots `db_path` into a new directory `name` (next to it) and
    // returns that directory.
    fn snapshot(db_type: &DBType, db_path: &Path, name: &str) -> PathBuf {
        let directory = db_path.parent().unwrap().join(name);
        fs::create_dir(&directory).unwrap();
        engine_for(db_type)
            .create_snapshot(db_path, &directory.join("snapshot"))
            .unwrap();
        directory
    }

    // Restoring a snapshot and snapshotting the result shouldn't produce a
    // snapshot with a new ID unless the contents changed.
    fn check_restored_snapshot_keeps_its_id(db_type: DBType, file_name: &str) {
        let engine = engine_for(&db_type);
        let ignored_bytes = engine.snapshot_hash_ignored_bytes();
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join(file_name);
        for query in ["CREATE TABLE t(x INTEGER)", "INSERT INTO t VALUES (1), (2)"] {
            engine.query(&db_path, query, QueryMode::ReadWrite).unwrap();
        }

        let first = snapshot(&db_type, &db_path, "first");
        // A restore makes the snapshot file the current database.
        fs::copy(first.join("snapshot"), &db_path).unwrap();
        let after_restore = snapshot(&db_type, &db_path, "after_restore");
        assert_eq!(
            hash_db_directory(&first, ignored_bytes).unwrap(),
            hash_db_directory(&after_restore, ignored_bytes).unwrap()
        );

        engine
            .query(&db_path, "INSERT INTO t VALUES (3)", QueryMode::ReadWrite)
            .unwrap();
        let changed = snapshot(&db_type, &db_path, "changed");
        assert_ne!(
            hash_db_directory(&first, ignored_bytes).unwrap(),
            hash_db_directory(&changed, ignored_bytes).unwrap()
        );
    }

    #[test]
    fn sqlite_restored_snapshot_keeps_its_id() {
        check_restored_snapshot_keeps_its_id(DBType::Sqlite, "test.sqlite");
    }

    #[test]
    fn duckdb_restored_snapshot_keeps_its_id() {
        check_restored_snapshot_keeps_its_id(DBType::Duckdb, "test.duckdb");
    }

    #[test]
    fn ignored_bytes_are_left_out_of_the_hash() {
        let dir = tempfile::tempdir().unwrap();
        let hash = |contents: &[u8]| {
            fs::write(dir.path().join("file"), contents).unwrap();
            // The second range runs past the end of the file.
            hash_db_directory(dir.path(), &[2..4, 5..100]).unwrap()
        };
        assert_eq!(hash(b"abcdef"), hash(b"abXYeZ"));
        assert_ne!(hash(b"abcdef"), hash(b"Zbcdef"));
    }
}

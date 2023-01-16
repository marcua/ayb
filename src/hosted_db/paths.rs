use crate::error::StacksError;
use std::fs;
use std::path::PathBuf;

pub fn database_path(entity_slug: &str, database_slug: &str) -> Result<PathBuf, StacksError> {
    // TODO(marcua): make the path relate to some
    // persistent storage (with high availability, etc.)
    let mut path: PathBuf = ["/tmp", "stacks", entity_slug].iter().collect();
    if let Err(e) = fs::create_dir_all(&path) {
        return Err(StacksError {
            error_string: format!("Unable to crate entity path for {}: {}", entity_slug, e),
        });
    }
    path.push(database_slug);
    Ok(path)
}

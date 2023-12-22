use crate::error::AybError;
use std::fs;
use std::path::PathBuf;

pub fn database_path(
    entity_slug: &str,
    database_slug: &str,
    data_path: &str,
    create_database: bool,
) -> Result<PathBuf, AybError> {
    let mut path: PathBuf = [data_path, entity_slug].iter().collect();
    if create_database {
        if let Err(e) = fs::create_dir_all(&path) {
            return Err(AybError {
                message: format!("Unable to create entity path for {}: {}", entity_slug, e),
            });
        }
    }

    path.push(database_slug);

    if create_database && !path.exists() {
        fs::File::create(path.clone())?;
    }

    Ok(path)
}

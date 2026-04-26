use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{APIToken, DBType, InstantiatedEntity};
use crate::error::AybError;
use crate::hosted_db::paths::current_database_path;
use crate::hosted_db::sqlite::query_sqlite;
use crate::hosted_db::QueryMode;
use crate::http::structs::EntityDatabasePath;
use crate::server::config::AybConfig;
use crate::server::permissions::highest_query_access_level;
use crate::server::utils::unwrap_authenticated_entity;
use actix_files::NamedFile;
use actix_web::http::header::{ContentDisposition, DispositionParam, DispositionType};
use actix_web::mime;
use actix_web::{get, web, HttpRequest, HttpResponse};
use std::fs;
use uuid::{timestamp::context::ContextV7, Timestamp, Uuid};

const EXPORTS_DIR: &str = "exports";

#[get("/{entity}/{database}/export")]
async fn export(
    req: HttpRequest,
    path: web::Path<EntityDatabasePath>,
    ayb_db: web::Data<Box<dyn AybDb>>,
    ayb_config: web::Data<AybConfig>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
    api_token: Option<web::ReqData<APIToken>>,
) -> Result<HttpResponse, AybError> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database;
    let database = ayb_db.get_database(entity_slug, database_slug).await?;
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;
    let token_ref = api_token.as_ref().map(|t| t.clone().into_inner());
    let token = token_ref.as_ref();

    // Read access (read-only or read-write) is sufficient to export.
    let access_level =
        highest_query_access_level(&authenticated_entity, &database, token, &ayb_db).await?;
    if access_level.is_none() {
        return Err(AybError::Other {
            message: format!(
                "Authenticated entity {} can't export database {}/{}",
                authenticated_entity.slug, entity_slug, database_slug
            ),
        });
    }

    let db_type = DBType::try_from(database.db_type)?;
    let db_path = current_database_path(entity_slug, database_slug, &ayb_config.data_path)?;
    let temp_dir = make_export_temp_dir(&ayb_config.data_path)?;
    let temp_path = temp_dir.join(database_slug);

    match dump_database(&db_type, &db_path, &temp_path) {
        Ok(()) => stream_and_clean_up(&req, &temp_path, &temp_dir, database_slug),
        Err(err) => {
            let _ = fs::remove_dir_all(&temp_dir);
            Err(err)
        }
    }
}

fn make_export_temp_dir(data_path: &str) -> Result<std::path::PathBuf, AybError> {
    let uuid = Uuid::new_v7(Timestamp::now(ContextV7::new()));
    let path: std::path::PathBuf = [data_path, EXPORTS_DIR, &uuid.to_string()].iter().collect();
    fs::create_dir_all(&path)?;
    Ok(fs::canonicalize(path)?)
}

fn dump_database(
    db_type: &DBType,
    src: &std::path::Path,
    dest: &std::path::Path,
) -> Result<(), AybError> {
    match db_type {
        DBType::Sqlite => {
            // VACUUM INTO produces a single-file, transactionally
            // consistent copy that is automatically defragmented. It
            // runs while writers continue (WAL mode) and matches the
            // method used by snapshots.
            let result = query_sqlite(
                &src.to_path_buf(),
                &format!("VACUUM INTO \"{}\"", dest.display()),
                true,
                QueryMode::ReadOnly,
            )?;
            if !result.rows.is_empty() {
                return Err(AybError::Other {
                    message: format!("Unexpected VACUUM INTO result: {result:?}"),
                });
            }
            Ok(())
        }
        _ => Err(AybError::Other {
            message: format!("Export not supported for {} databases", db_type.to_str()),
        }),
    }
}

fn stream_and_clean_up(
    req: &HttpRequest,
    file_path: &std::path::Path,
    temp_dir: &std::path::Path,
    download_name: &str,
) -> Result<HttpResponse, AybError> {
    let file = std::fs::File::open(file_path)?;

    // Unlink the file (Unix: handle keeps inode alive until streamed
    // out; on other platforms the file lingers until next restart).
    let _ = fs::remove_file(file_path);
    let _ = fs::remove_dir(temp_dir);

    let named_file = NamedFile::from_file(file, download_name)?
        .set_content_type(mime::APPLICATION_OCTET_STREAM)
        .set_content_disposition(ContentDisposition {
            disposition: DispositionType::Attachment,
            parameters: vec![DispositionParam::Filename(download_name.to_string())],
        });
    Ok(named_file.into_response(req))
}

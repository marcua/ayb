use crate::hosted_db::{run_query, QueryResult};
use crate::http::structs::{Error, Owner, OwnerDatabase};
use crate::http::utils::get_header;
use crate::stacks_db::crud::{
    create_database as create_database_crud, create_owner as create_owner_crud,
    get_database as get_database_crud, get_owner as get_owner_crud,
};
use crate::stacks_db::models::{
    DBType, Database, DatabaseOwner, InstantiatedDatabase, InstantiatedDatabaseOwner,
};
use actix_web::{post, web, HttpRequest};
use sqlx;

#[post("/v1/{owner}/{database}")]
async fn create_database(
    path: web::Path<OwnerDatabase>,
    req: HttpRequest,
    db_pool: web::Data<&sqlx::PgPool>,
) -> Result<web::Json<InstantiatedDatabase>, Error> {
    let owner_slug = &path.owner;
    match get_owner_crud(owner_slug, &db_pool).await {
        Ok(owner) => {
            let db_type = get_header(req, "db-type");
            match db_type {
                Ok(db_type) => {
                    let database = Database {
                        owner_id: owner.id,
                        slug: path.database.clone(),
                        db_type: DBType::from_str(&db_type) as i16,
                    };

                    match create_database_crud(&database, &db_pool).await {
                        Ok(result) => Ok(web::Json(result)),
                        Err(err) => Err(Error { error_string: err }),
                    }
                }
                Err(err) => Err(Error {
                    error_string: err.to_string(),
                }),
            }
        }
        Err(err) => Err(Error { error_string: err }),
    }
}

#[post("/v1/{owner}")]
async fn create_owner(
    path: web::Path<Owner>,
    // req: HttpRequest // tried this
    db_pool: web::Data<&sqlx::PgPool>,
) -> Result<web::Json<InstantiatedDatabaseOwner>, Error> {
    let owner = DatabaseOwner {
        slug: path.owner.clone(),
    };

    match create_owner_crud(&owner, &db_pool).await {
        Ok(result) => Ok(web::Json(result)),
        Err(err) => Err(Error { error_string: err }),
    }
}

#[post("/v1/{owner}/{database}/query")]
async fn query(
    path: web::Path<OwnerDatabase>,
    query: String,
    db_pool: web::Data<&sqlx::PgPool>,
) -> Result<web::Json<QueryResult>, Error> {
    let owner_slug = &path.owner;
    let database_slug = &path.database;
    match get_database_crud(owner_slug, database_slug, &db_pool).await {
        Ok(database) => {
            let db_type = DBType::from_i16(database.db_type);
            // TODO(marcua): make the path relate to some
            // persistent storage (with high availability, etc.)
            let path = ["/tmp", owner_slug, database_slug].iter().collect();
            match run_query(&path, &query, &db_type) {
                Ok(result) => Ok(web::Json(result)),
                Err(err) => Err(Error { error_string: err }),
            }
        }
        Err(err) => Err(Error { error_string: err }),
    }
}

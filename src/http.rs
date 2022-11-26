use crate::hosted_db::{run_query, QueryResult};
use crate::stacks_db::crud::{
    create_database as create_database_crud, create_owner as create_owner_crud,
    get_database as get_database_crud, get_owner as get_owner_crud,
};
use crate::stacks_db::models::{
    DBType, Database, DatabaseOwner, InstantiatedDatabase, InstantiatedDatabaseOwner,
};
use actix_web::{error, middleware, post, web, App, HttpServer};
use derive_more::{Display, Error};
use dotenvy;
use serde::{Deserialize, Serialize};
use sqlx;

#[derive(Serialize, Deserialize)]
pub struct OwnerDatabase {
    owner: String,
    database: String,
}

#[derive(Serialize, Deserialize)]
pub struct Owner {
    owner: String,
}

#[derive(Debug, Display, Error)]
#[display(fmt = "{}", error_string)]
struct Error {
    error_string: String,
}

impl error::ResponseError for Error {}

#[post("/v1/{owner}/{database}")]
async fn create_database(
    path: web::Path<OwnerDatabase>,
    // req: HttpRequest // tried this
    db_pool: web::Data<&sqlx::PgPool>,
) -> Result<web::Json<InstantiatedDatabase>, Error> {
    let owner_slug = &path.owner;
    match get_owner_crud(owner_slug, &db_pool).await {
        Ok(owner) => {
            // TODO(marcua): Read db type from header
            // let db_type = req.headers().get("db-type")?.to_str().ok(); //DBType::Sqlite;
            let database = Database {
                owner_id: owner.id,
                slug: path.database.clone(),
                db_type: DBType::Sqlite as i16, // TODO(marcua): actual db type from headers.
            };

            match create_database_crud(&database, &db_pool).await {
                Ok(result) => Ok(web::Json(result)),
                Err(err) => Err(Error { error_string: err }),
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

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(create_database);
    cfg.service(create_owner);
    cfg.service(query);
}

// TODO(marcua): Understand tokio::main vs actix_web::main
#[actix_web::main]
pub async fn run_server(host: &str, port: &u16) -> std::io::Result<()> {
    let database_url = dotenvy::var("DATABASE_URL").expect("Provide a DATABASE_URL");

    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await
        .expect("Failed to connect to DATABASE_URL");

    sqlx::migrate!()
        .run(&db)
        .await
        .expect("Unable to run migration");

    println!("Starting server {}:{}...", host, port);
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .configure(config)
            .app_data(web::Data::new(db.clone()))
    })
    .bind((host, *port))?
    .run()
    .await
}

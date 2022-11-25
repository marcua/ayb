use crate::hosted_db::{run_query, QueryResult};
use crate::stacks_db::crud::{create_database, DatabaseCreationResult};
use crate::stacks_db::models::{Database, DBType};
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

#[derive(Debug, Display, Error)]
#[display(fmt = "{}", error_string)]
struct Error {
    error_string: String,
}

impl error::ResponseError for Error {}

#[post("/v1/{owner}/{database}/query")]
async fn query(
    path: web::Path<OwnerDatabase>,
    query: String,
) -> Result<web::Json<QueryResult>, Error> {
    let owner = &path.owner;
    let database = &path.database;
    // TODO(marcua): derive database type from DB (requires creation
    // endpoint).
    let db_type = DBType::Sqlite;
    // TODO(marcua): make the path relate to some
    // persistent storage (with high availability, etc.)
    let path = ["/tmp", owner, database].iter().collect();
    match run_query(&path, &query, &db_type) {
        Ok(result) => Ok(web::Json(result)),
        Err(err) => Err(Error { error_string: err }),
    }
}

#[post("/v1/{owner}/{database}")]
async fn create(
    path: web::Path<OwnerDatabase>,
    // req: HttpRequest // tried this
    db_pool: web::Data<&sqlx::PgPool>, 
) -> Result<web::Json<DatabaseCreationResult>, Error> {
    // let owner = &path.owner;

    // TODO(marcua): Read db type from header
    // let db_type = req.headers().get("db-type")?.to_str().ok(); //DBType::Sqlite;
    let database = Database {
        owner_id: 0, // TODO(marcua): actual owner.
        slug: path.database.clone(),
        db_type: DBType::Sqlite // TODO(marcua): actual db type from headers.
    };
    
    match create_database(&database, &db_pool).await {
        Ok(result) => Ok(web::Json(result)),
        Err(err) => Err(Error { error_string: err }),
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(query);
    //cfg.service(create);
}

// TODO(marcua): Understand tokio::main vs actix_web::main
#[actix_web::main]
pub async fn run_server(host: &str, port: &u16) -> std::io::Result<()> {
    let database_url = dotenvy::var("DATABASE_URL")
        .expect("Provide a DATABASE_URL");

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

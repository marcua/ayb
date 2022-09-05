use crate::databases::{run_query, DBType, QueryResult};
use actix_web::{error, middleware, post, web, App, HttpServer};
use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct EntityDatabase {
    entity: String,
    database: String,
}

#[derive(Debug, Display, Error)]
#[display(fmt = "{}", error_string)]
struct QueryError {
    error_string: String,
}

impl error::ResponseError for QueryError {}

#[post("/v1/{entity}/{database}")]
async fn query(
    path: web::Path<EntityDatabase>,
    query: String,
) -> Result<web::Json<QueryResult>, QueryError> {
    let entity = &path.entity;
    let database = &path.database;
    // TODO(marcua): derive database type from DB (requires creation
    // endpoint).
    let db_type = DBType::Sqlite;
    // TODO(marcua): make the path relate to some
    // persistent storage (with high availability, etc.)
    let path = ["/tmp", entity, database].iter().collect();
    match run_query(&path, &query, &db_type) {
        Ok(result) => Ok(web::Json(result)),
        Err(err) => Err(QueryError { error_string: err }),
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(query);
}

// TODO(marcua): Understand tokio::main vs actix_web::main
#[actix_web::main]
pub async fn run_server(host: &str, port: &u16) -> std::io::Result<()> {
    println!("Starting server {}:{}...", host, port);
    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Compress::default())
            .configure(config)
    })
    .bind((host, *port))?
    .run()
    .await
}

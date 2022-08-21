use actix_web::{post, web, App, HttpServer, Responder};
use serde::{Deserialize, Serialize};
// use crate::databases::{run_query, DBType};

#[derive(Serialize, Deserialize)]
pub struct EntityDatabase {
    pub entity: String,
    pub database: String,
}

#[post("/v1/{entity}/{database}")]
async fn query(path: web::Path<EntityDatabase>) -> impl Responder {
    let entity = &path.entity;
    let database = &path.database;
    format!("Hello {entity}/{database}!")
}

// TODO(marcua): Understand tokio::main vs actix_web::main
#[actix_web::main]
pub async fn run_server(host: &str, port: &u16) -> std::io::Result<()> {
    println!("Starting server {}:{}...", host, port);
    HttpServer::new(|| App::new().service(query))
        .bind((host, *port))?
        .run()
        .await
}

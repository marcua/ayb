use crate::http::endpoints::{create_database, create_entity, query};
use actix_web::{middleware, web, App, HttpServer};
use serde::{Serialize, Deserialize};
use sqlx::migrate;
use sqlx::postgres::PgPoolOptions;
use std::fs;
use std::path::PathBuf;
use toml;

#[derive(Serialize, Deserialize)]
struct Config {
    host: String,
    port: u16,
    database_url: String,
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(create_database);
    cfg.service(create_entity);
    cfg.service(query);
}

pub async fn run_server(config_path: &PathBuf) -> std::io::Result<()> {
    env_logger::init();
    
    let contents = fs::read_to_string(config_path)?;
    let conf: Config = toml::from_str(&contents).unwrap();
    
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&conf.database_url)
        .await
        .expect("Unable to connect to database");

    migrate!()
        .run(&pool)
        .await
        .expect("Unable to run migrations");

    println!("Starting server {}:{}...", conf.host, conf.port);
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .configure(config)
            .app_data(web::Data::new(pool.clone()))
    })
    .bind((conf.host, conf.port))?
    .run()
    .await
}

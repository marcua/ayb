use crate::http::endpoints::{create_database, query, register};
use crate::http::structs::AybConfig;
use actix_web::{middleware, web, App, HttpServer};
use sqlx::migrate;
use sqlx::postgres::PgPoolOptions;
use std::fs;
use std::path::PathBuf;
use toml;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(create_database);
    cfg.service(query);
    cfg.service(register);
}

pub async fn run_server(config_path: &PathBuf) -> std::io::Result<()> {
    env_logger::init();

    let contents = fs::read_to_string(config_path)?;
    let ayb_conf: AybConfig = toml::from_str(&contents).unwrap();
    let ayb_conf_for_server = ayb_conf.clone();

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&ayb_conf.database_url)
        .await
        .expect("Unable to connect to database");

    migrate!()
        .run(&pool)
        .await
        .expect("Unable to run migrations");

    println!("Starting server {}:{}...", ayb_conf.host, ayb_conf.port);
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .configure(config)
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(ayb_conf_for_server.clone()))
    })
    .bind((ayb_conf.host, ayb_conf.port))?
    .run()
    .await
}

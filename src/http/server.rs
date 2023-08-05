use crate::ayb_db::db_interfaces::connect_to_ayb_db;
use crate::http::endpoints::{confirm, create_database, query, register};
use crate::http::structs::AybConfig;
use actix_web::{middleware, web, App, HttpServer};
use dyn_clone::clone_box;
use std::fs;
use std::path::PathBuf;
use toml;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(create_database);
    cfg.service(query);
    cfg.service(register);
    cfg.service(confirm);
}

pub async fn run_server(config_path: &PathBuf) -> std::io::Result<()> {
    env_logger::init();

    let contents = fs::read_to_string(config_path)?;
    let ayb_conf: AybConfig = toml::from_str(&contents).unwrap();
    let ayb_conf_for_server = ayb_conf.clone();
    fs::create_dir_all(&ayb_conf.data_path).expect("Unable to create data directory");
    let ayb_db = connect_to_ayb_db(ayb_conf.database_url).await.unwrap();

    println!("Starting server {}:{}...", ayb_conf.host, ayb_conf.port);
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .configure(config)
            .app_data(web::Data::new(clone_box(&*ayb_db)))
            .app_data(web::Data::new(ayb_conf_for_server.clone()))
    })
    .bind((ayb_conf.host, ayb_conf.port))?
    .run()
    .await
}

use crate::ayb_db::db_interfaces::connect_to_ayb_db;
use crate::ayb_db::db_interfaces::AybDb;
use crate::error::AybError;
use crate::http::endpoints::{confirm, create_database, log_in, query, register};
use crate::http::structs::AybConfig;
use crate::http::tokens::retrieve_and_validate_api_token;
use actix_web::dev::ServiceRequest;
use actix_web::{middleware, web, App, Error, HttpMessage, HttpServer};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use actix_web_httpauth::middleware::HttpAuthentication;
use dyn_clone::clone_box;
use std::fs;
use std::path::PathBuf;
use toml;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(confirm);
    cfg.service(log_in);
    cfg.service(register);
    cfg.service(
        web::scope("")
            .wrap(HttpAuthentication::bearer(entity_validator))
            .service(create_database)
            .service(query),
    );
}

async fn entity_validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    match req.app_data::<web::Data<Box<dyn AybDb>>>() {
        Some(ayb_db) => {
            let api_token = retrieve_and_validate_api_token(credentials.token(), ayb_db).await;
            match api_token {
                Ok(api_token) => {
                    let entity = ayb_db.get_entity_by_id(api_token.entity_id).await;
                    match entity {
                        Ok(entity) => {
                            req.extensions_mut().insert(entity);
                            Ok(req)
                        }
                        Err(e) => Err((e.into(), req)),
                    }
                }
                Err(e) => Err((e.into(), req)),
            }
        }
        None => Err((
            AybError {
                message: "Misconfigured server: no database".to_string(),
            }
            .into(),
            req,
        )),
    }
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

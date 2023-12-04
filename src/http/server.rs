use crate::ayb_db::db_interfaces::connect_to_ayb_db;
use crate::ayb_db::db_interfaces::AybDb;
use crate::error::AybError;
use crate::http::config::read_config;
use crate::http::endpoints::{
    confirm_endpoint, create_db_endpoint, log_in_endpoint, query_endpoint, register_endpoint,
};
use crate::http::tokens::retrieve_and_validate_api_token;
use actix_web::dev::ServiceRequest;
use actix_web::{middleware, web, App, Error, HttpMessage, HttpServer};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use actix_web_httpauth::middleware::HttpAuthentication;
use dyn_clone::clone_box;
use std::fs;
use std::path::PathBuf;
use actix_cors::Cors;
use crate::http::structs::AybConfigCors;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(confirm_endpoint);
    cfg.service(log_in_endpoint);
    cfg.service(register_endpoint);
    cfg.service(
        web::scope("")
            .wrap(HttpAuthentication::bearer(entity_validator))
            .service(create_db_endpoint)
            .service(query_endpoint),
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

fn build_cors(ayb_cors: Option<AybConfigCors>) -> Cors {
    let mut cors = Cors::default()
        .allow_any_header()
        .allow_any_method();

    if ayb_cors.as_ref().is_some_and(|conf| conf.origin.trim() == "*") || ayb_cors.is_none() {
        cors = cors.allow_any_origin()
    } else {
        cors = cors.allowed_origin(ayb_cors.as_ref().unwrap().origin.trim());
    }

    cors
}

pub async fn run_server(config_path: &PathBuf) -> std::io::Result<()> {
    env_logger::init();

    let ayb_conf = read_config(config_path).unwrap();
    let ayb_conf_for_server = ayb_conf.clone();
    fs::create_dir_all(&ayb_conf.data_path).expect("Unable to create data directory");
    let ayb_db = connect_to_ayb_db(ayb_conf.database_url).await.unwrap();

    println!("Starting server {}:{}...", ayb_conf.host, ayb_conf.port);
    HttpServer::new(move || {
        let cors = build_cors(ayb_conf.cors.clone());

        App::new()
            .wrap(middleware::Compress::default())
            .wrap(cors)
            .configure(config)
            .app_data(web::Data::new(clone_box(&*ayb_db)))
            .app_data(web::Data::new(ayb_conf_for_server.clone()))
    })
    .bind((ayb_conf.host, ayb_conf.port))?
    .run()
    .await
}

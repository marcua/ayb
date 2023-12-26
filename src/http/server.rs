use crate::ayb_db::db_interfaces::connect_to_ayb_db;
use crate::ayb_db::db_interfaces::AybDb;
use crate::error::AybError;
use crate::http::config::read_config;
use crate::http::endpoints::{
    confirm_endpoint, create_db_endpoint, entity_details_endpoint, log_in_endpoint, query_endpoint,
    register_endpoint,
};
use crate::http::structs::AybConfigCors;
use crate::http::tokens::retrieve_and_validate_api_token;
use crate::http::web_frontend::WebFrontendDetails;
use actix_cors::Cors;
use actix_web::dev::ServiceRequest;
use actix_web::{middleware, web, App, Error, HttpMessage, HttpServer};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use actix_web_httpauth::middleware::HttpAuthentication;
use dyn_clone::clone_box;
use std::fs;
use std::path::PathBuf;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(confirm_endpoint);
    cfg.service(log_in_endpoint);
    cfg.service(register_endpoint);
    cfg.service(
        web::scope("")
            .wrap(HttpAuthentication::bearer(entity_validator))
            .service(create_db_endpoint)
            .service(query_endpoint)
            .service(entity_details_endpoint),
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
            AybError::Other {
                message: "Misconfigured server: no database".to_string(),
            }
            .into(),
            req,
        )),
    }
}

fn build_cors(ayb_cors: AybConfigCors) -> Cors {
    let mut cors = Cors::default().allow_any_header().allow_any_method();

    if ayb_cors.origin.trim() == "*" {
        cors = cors.allow_any_origin()
    } else {
        cors = cors.allowed_origin(ayb_cors.origin.trim());
    }

    cors
}

pub async fn run_server(config_path: &PathBuf) -> std::io::Result<()> {
    env_logger::init();

    let ayb_conf = read_config(config_path).unwrap();
    let ayb_conf_for_server = ayb_conf.clone();
    fs::create_dir_all(&ayb_conf.data_path).expect("Unable to create data directory");
    let ayb_db = connect_to_ayb_db(ayb_conf.database_url).await.unwrap();
    let web_details = if let Some(web_conf) = ayb_conf.web {
        Some(
            WebFrontendDetails::from_url(&web_conf.info_url)
                .await
                .expect("failed to retrieve information from the web frontend"),
        )
    } else {
        None
    };

    println!("Starting server {}:{}...", ayb_conf.host, ayb_conf.port);
    if ayb_conf.isolation.is_none() {
        println!("Note: Server is running without full isolation. Read more about isolating users from one-another: https://github.com/marcua/ayb/#isolation");
    }
    HttpServer::new(move || {
        let cors = build_cors(ayb_conf.cors.clone());

        App::new()
            .wrap(middleware::Compress::default())
            .wrap(cors)
            .configure(config)
            .app_data(web::Data::new(web_details.clone()))
            .app_data(web::Data::new(clone_box(&*ayb_db)))
            .app_data(web::Data::new(ayb_conf_for_server.clone()))
    })
    .bind((ayb_conf.host, ayb_conf.port))?
    .run()
    .await
}

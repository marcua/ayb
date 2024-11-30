use crate::ayb_db::db_interfaces::connect_to_ayb_db;
use crate::ayb_db::db_interfaces::AybDb;
use crate::error::AybError;
use crate::server::config::read_config;
use crate::server::config::AybConfigCors;
use crate::server::endpoints::{
    confirm_endpoint, create_db_endpoint, entity_details_endpoint, list_snapshots_endpoint,
    log_in_endpoint, query_endpoint, register_endpoint, restore_snapshot_endpoint, share_endpoint,
    update_db_endpoint, update_profile_endpoint,
};
use crate::server::snapshots::execution::schedule_periodic_snapshots;
use crate::server::tokens::retrieve_and_validate_api_token;
use crate::server::web_frontend::WebFrontendDetails;
use actix_cors::Cors;
use actix_web::dev::ServiceRequest;
use actix_web::{middleware, web, App, Error, HttpMessage, HttpServer};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use actix_web_httpauth::middleware::HttpAuthentication;
use dyn_clone::clone_box;
use std::fs;
use std::path::{Path, PathBuf};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(confirm_endpoint);
    cfg.service(log_in_endpoint);
    cfg.service(register_endpoint);
    cfg.service(
        web::scope("")
            .wrap(HttpAuthentication::bearer(entity_validator))
            .service(create_db_endpoint)
            .service(update_db_endpoint)
            .service(query_endpoint)
            .service(entity_details_endpoint)
            .service(update_profile_endpoint)
            .service(list_snapshots_endpoint)
            .service(restore_snapshot_endpoint)
            .service(share_endpoint),
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

    let ayb_conf = read_config(config_path).expect("unable to find an ayb.toml configuration file");
    let ayb_conf_for_server = ayb_conf.clone();
    fs::create_dir_all(&ayb_conf.data_path).expect("unable to create data directory");
    let ayb_db = connect_to_ayb_db(ayb_conf.database_url)
        .await
        .expect("unable to connect to ayb database");
    let web_details = if let Some(web_conf) = ayb_conf.web {
        Some(
            WebFrontendDetails::from_url(&web_conf.info_url)
                .await
                .expect("failed to retrieve information from the web frontend"),
        )
    } else {
        None
    };
    schedule_periodic_snapshots(ayb_conf_for_server.clone(), ayb_db.clone())
        .await
        .expect("unable to start periodic snapshot scheduler");

    println!("Starting server {}:{}...", ayb_conf.host, ayb_conf.port);
    if ayb_conf.isolation.is_none() {
        println!("Note: Server is running without full isolation. Read more about isolating users from one-another: https://github.com/marcua/ayb/#isolation");
    } else {
        let isolation = ayb_conf.isolation.unwrap();
        let nsjail_path = Path::new(&isolation.nsjail_path);
        if !nsjail_path.exists() {
            panic!("nsjail path {} does not exist", nsjail_path.display());
        }
    }
    HttpServer::new(move || {
        let cors = build_cors(ayb_conf.cors.clone());

        App::new()
            .wrap(middleware::Logger::default())
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

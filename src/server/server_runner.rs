use crate::ayb_db::db_interfaces::connect_to_ayb_db;
use crate::ayb_db::db_interfaces::AybDb;
use crate::email::create_email_backends;
use crate::error::AybError;
use crate::hosted_db::daemon_registry::DaemonRegistry;
use crate::server::config::read_config;
use crate::server::config::{AybConfig, AybConfigCors, WebHostingMethod};
use crate::server::snapshots::execution::schedule_periodic_snapshots;
use crate::server::tokens::retrieve_and_validate_api_token;
use crate::server::web_frontend::WebFrontendDetails;
use crate::server::{api_endpoints, ui_endpoints};
use actix_cors::Cors;
use actix_web::dev::ServiceRequest;
use actix_web::{middleware, web, App, Error, HttpMessage, HttpServer};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use actix_web_httpauth::middleware::HttpAuthentication;
use dyn_clone::clone_box;
use std::env::consts::OS;
use std::fs;
use std::path::Path;

pub fn config(cfg: &mut web::ServiceConfig, ayb_config: &AybConfig) {
    // Unauthenticated API endpoints
    cfg.service(api_endpoints::health_endpoint)
        .service(api_endpoints::confirm_endpoint)
        .service(api_endpoints::log_in_endpoint)
        .service(api_endpoints::register_endpoint);

    // Authenticated API endpoints
    cfg.service(
        web::scope("/v1")
            .wrap(HttpAuthentication::bearer(entity_validator))
            .service(api_endpoints::create_database_endpoint)
            .service(api_endpoints::database_details_endpoint)
            .service(api_endpoints::update_database_endpoint)
            .service(api_endpoints::query_endpoint)
            .service(api_endpoints::entity_details_endpoint)
            .service(api_endpoints::update_profile_endpoint)
            .service(api_endpoints::list_snapshots_endpoint)
            .service(api_endpoints::restore_snapshot_endpoint)
            .service(api_endpoints::share_endpoint)
            .service(api_endpoints::list_database_permissions_endpoint)
            .service(api_endpoints::list_tokens_endpoint)
            .service(api_endpoints::revoke_token_endpoint),
    );

    // Only add UI routes if web frontend is configured for local serving
    if let Some(web_config) = &ayb_config.web {
        if web_config.hosting_method == WebHostingMethod::Local {
            cfg.service(ui_endpoints::log_in_endpoint)
                .service(ui_endpoints::log_in_submit_endpoint)
                .service(ui_endpoints::log_out_endpoint)
                .service(ui_endpoints::register_endpoint)
                .service(ui_endpoints::register_submit_endpoint)
                .service(ui_endpoints::confirm_endpoint)
                .service(ui_endpoints::entity_tokens_endpoint)
                .service(ui_endpoints::revoke_token_endpoint)
                .service(ui_endpoints::entity_details_endpoint)
                .service(ui_endpoints::create_database_endpoint)
                .service(ui_endpoints::update_profile_endpoint)
                .service(ui_endpoints::database_endpoint)
                .service(ui_endpoints::query_endpoint)
                .service(ui_endpoints::update_public_sharing_endpoint)
                .service(ui_endpoints::share_with_entity_endpoint)
                .service(ui_endpoints::database_permissions_endpoint)
                .service(ui_endpoints::database_snapshots_endpoint)
                .service(ui_endpoints::restore_snapshot_endpoint);
        }
    }
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
                            req.extensions_mut().insert(api_token);
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

pub async fn run_server(config_path: &Path) -> std::io::Result<()> {
    env_logger::init();

    let ayb_conf = read_config(config_path)
        .unwrap_or_else(|e| panic!("unable to read ayb.toml configuration file: {e}"));
    let mut ayb_conf_for_server = ayb_conf.clone();
    fs::create_dir_all(&ayb_conf.data_path).expect("unable to create data directory");
    let ayb_db = connect_to_ayb_db(ayb_conf.database_url)
        .await
        .expect("unable to connect to ayb database");
    let web_details = WebFrontendDetails::load(ayb_conf_for_server.clone())
        .await
        .expect("failed to load web frontend details");
    let email_backends = create_email_backends(&ayb_conf.email);

    // Create the daemon registry for managing persistent query runner processes
    let daemon_registry = DaemonRegistry::new();
    // Clone for cleanup handler before moving into closure
    let cleanup_daemon_registry = daemon_registry.clone();

    schedule_periodic_snapshots(ayb_conf_for_server.clone(), ayb_db.clone())
        .await
        .expect("unable to start periodic snapshot scheduler");

    println!("Starting server {}:{}...", ayb_conf.host, ayb_conf.port);
    if ayb_conf.isolation.is_none() {
        println!("Note: Server is running without full isolation. Read more about isolating users from one-another: https://github.com/marcua/ayb/#isolation");
    } else if OS != "linux" {
        println!(
            "Warning: nsjail isolation is only supported on Linux. Running without isolation on {OS}"
        );
        ayb_conf_for_server.isolation = None;
    } else {
        let isolation = ayb_conf.isolation.unwrap();
        let nsjail_path = Path::new(&isolation.nsjail_path);
        if !nsjail_path.exists() {
            panic!("nsjail path {} does not exist", nsjail_path.display());
        }
    }

    let server = HttpServer::new(move || {
        let cors = build_cors(ayb_conf.cors.clone());

        App::new()
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .wrap(cors)
            .app_data(web::Data::new(web_details.clone()))
            .app_data(web::Data::new(clone_box(&*ayb_db)))
            .app_data(web::Data::new(ayb_conf_for_server.clone()))
            .app_data(web::Data::new(email_backends.clone()))
            .app_data(web::Data::new(daemon_registry.clone()))
            .configure(|cfg| config(cfg, &ayb_conf_for_server.clone()))
    })
    .bind((ayb_conf.host, ayb_conf.port))?
    .run();

    let server_handle = server.handle();

    // Spawn a task to handle shutdown and clean up daemons
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("Shutting down server and cleaning up daemons...");
        cleanup_daemon_registry.shut_down_all().await;
        server_handle.stop(true).await;
    });

    server.await
}

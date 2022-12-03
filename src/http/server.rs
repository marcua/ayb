use crate::http::endpoints::{create_database, create_entity, query};
use actix_web::{middleware, web, App, HttpServer};
use dotenvy;
use sqlx;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(create_database);
    cfg.service(create_entity);
    cfg.service(query);
}

// TODO(marcua): Understand tokio::main vs actix_web::main
#[actix_web::main]
pub async fn run_server(host: &str, port: &u16) -> std::io::Result<()> {
    let database_url = dotenvy::var("DATABASE_URL").expect("Provide a DATABASE_URL");

    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await
        .expect("Failed to connect to DATABASE_URL");

    sqlx::migrate!()
        .run(&db)
        .await
        .expect("Unable to run migration");

    println!("Starting server {}:{}...", host, port);
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .configure(config)
            .app_data(web::Data::new(db.clone()))
    })
    .bind((host, *port))?
    .run()
    .await
}

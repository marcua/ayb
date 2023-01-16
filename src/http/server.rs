use crate::http::endpoints::{create_database, create_entity, query};
use actix_web::{middleware, web, App, HttpServer};
use dotenvy;
use sqlx::migrate;
use sqlx::postgres::PgPoolOptions;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(create_database);
    cfg.service(create_entity);
    cfg.service(query);
}

pub async fn run_server(host: &str, port: &u16) -> std::io::Result<()> {
    env_logger::init();
    let database_url = dotenvy::var("DATABASE_URL").expect("Provide a DATABASE_URL");

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await
        .expect("Unable to connect to database");

    migrate!()
        .run(&pool)
        .await
        .expect("Unable to run migrations");

    println!("Starting server {}:{}...", host, port);
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .configure(config)
            .app_data(web::Data::new(pool.clone()))
    })
    .bind((host, *port))?
    .run()
    .await
}

use crate::ayb_db::db_interfaces::{AybDb, connect_to_ayb_db};
use crate::http::structs::{AybConfig, AybConfigAuthentication, AybConfigEmail};

pub async fn test_ayb_database() -> Box<dyn AybDb> {
    connect_to_ayb_db("sqlite://:memory:".into()).await.unwrap()
}
pub fn test_ayb_conf() -> AybConfig {
    AybConfig {
        host: "0.0.0.0".into(),
        port: 5433,
        e2e_testing: Some(true),
        database_url: "sqlite://:memory:".into(),
        data_path: "./ayb_data".into(),
        authentication: AybConfigAuthentication {
            fernet_key: "QRibF1t12YQAwtCucF8RbBB_RHp9g92j1-wjxYJXiBc=".into(),
            token_expiration_seconds: 31536000,
        },
        email: AybConfigEmail {
            from: "".into(),
            reply_to: "".into(),
            smtp_host: "".into(),
            smtp_port: 0,
            smtp_username: "".into(),
            smtp_password: "".into(),
        },
    }
}
use crate::ayb_db::models::{DBType, EntityType};
use crate::error::AybError;
use crate::hosted_db::QueryResult;
use crate::http::structs::{APIToken, Database, EmptyResponse};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::de::DeserializeOwned;

pub struct AybClient {
    pub base_url: String,
}

impl AybClient {
    fn make_url(&self, endpoint: String) -> String {
        format!("{}/v1/{}", self.base_url, endpoint)
    }

    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
        expected_status: reqwest::StatusCode,
    ) -> Result<T, AybError> {
        match response.status() {
            status if status == expected_status => response.json::<T>().await.or_else(|err| {
                Err(AybError {
                    message: format!("Unable to parse successful response: {}", err),
                })
            }),
            _other => {
                let error = response.json::<AybError>().await;
                match error {
                    Ok(ayb_error) => Err(ayb_error),
                    Err(error) => Err(AybError {
                        message: format!("Unable to parse error response: {:#?}", error),
                    }),
                }
            }
        }
    }

    pub async fn confirm(&self, authentication_token: &str) -> Result<APIToken, AybError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("authentication-token"),
            HeaderValue::from_str(authentication_token).unwrap(),
        );

        let response = reqwest::Client::new()
            .post(self.make_url("confirm".to_owned()))
            .headers(headers)
            .send()
            .await?;

        self.handle_response(response, reqwest::StatusCode::OK)
            .await
    }

    pub async fn create_database(
        &self,
        entity: &str,
        database: &str,
        db_type: &DBType,
    ) -> Result<Database, AybError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("db-type"),
            HeaderValue::from_str(db_type.to_str()).unwrap(),
        );

        let response = reqwest::Client::new()
            .post(self.make_url(format!("{}/{}/create", entity, database)))
            .headers(headers)
            .send()
            .await?;

        self.handle_response(response, reqwest::StatusCode::CREATED)
            .await
    }

    pub async fn log_in(&self, entity: &str) -> Result<EmptyResponse, AybError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("entity"),
            HeaderValue::from_str(entity).unwrap(),
        );

        let response = reqwest::Client::new()
            .post(self.make_url("log_in".to_owned()))
            .headers(headers)
            .send()
            .await?;

        self.handle_response(response, reqwest::StatusCode::OK)
            .await
    }

    pub async fn query(
        &self,
        entity: &str,
        database: &str,
        query: &str,
    ) -> Result<QueryResult, AybError> {
        let response = reqwest::Client::new()
            .post(self.make_url(format!("{}/{}/query", entity, database)))
            .body(query.to_owned())
            .send()
            .await?;

        self.handle_response(response, reqwest::StatusCode::OK)
            .await
    }

    pub async fn register(
        &self,
        entity: &str,
        email_address: &str,
        entity_type: &EntityType,
    ) -> Result<EmptyResponse, AybError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("entity"),
            HeaderValue::from_str(entity).unwrap(),
        );
        headers.insert(
            HeaderName::from_static("email-address"),
            HeaderValue::from_str(email_address).unwrap(),
        );
        headers.insert(
            HeaderName::from_static("entity-type"),
            HeaderValue::from_str(entity_type.to_str()).unwrap(),
        );

        let response = reqwest::Client::new()
            .post(self.make_url(format!("register")))
            .headers(headers)
            .send()
            .await?;

        self.handle_response(response, reqwest::StatusCode::OK)
            .await
    }
}

use crate::ayb_db::models::{DBType, EntityType};
use crate::error::AybError;
use crate::hosted_db::QueryResult;
use crate::http::structs::{APIToken, Database, EmptyResponse, EntityQueryResponse};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::de::DeserializeOwned;

pub struct AybClient {
    pub base_url: String,
    pub api_token: Option<String>,
}

impl AybClient {
    fn make_url(&self, endpoint: String) -> String {
        format!("{}/v1/{}", self.base_url, endpoint)
    }

    fn add_bearer_token(&self, headers: &mut HeaderMap) -> Result<(), AybError> {
        if let Some(api_token) = &self.api_token {
            headers.insert(
                HeaderName::from_static("authorization"),
                HeaderValue::from_str(format!("Bearer {}", api_token).as_str()).unwrap(),
            );
            Ok(())
        } else {
            Err(AybError::Other {
                message: "Calling endpoint that requires client API token, but none provided"
                    .to_string(),
            })
        }
    }

    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
        expected_status: reqwest::StatusCode,
    ) -> Result<T, AybError> {
        let status = response.status();
        if status == expected_status {
            response.json::<T>().await.map_err(|err| AybError::Other {
                message: format!("Unable to parse successful response: {}", err),
            })
        } else {
            response
                .json::<AybError>()
                .await
                .map(|v| Err(v))
                .map_err(|error| AybError::Other {
                    message: format!(
                        "Unable to parse error response: {:#?}, response code: {}",
                        error, status
                    ),
                })?
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
        self.add_bearer_token(&mut headers)?;

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

    pub async fn list_databases(&self, entity: &str) -> Result<EntityQueryResponse, AybError> {
        let mut headers = HeaderMap::new();
        self.add_bearer_token(&mut headers)?;

        let response = reqwest::Client::new()
            .get(self.make_url(format!("entity/{}", entity)))
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
        let mut headers = HeaderMap::new();
        self.add_bearer_token(&mut headers)?;

        let response = reqwest::Client::new()
            .post(self.make_url(format!("{}/{}/query", entity, database)))
            .headers(headers)
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
            .post(self.make_url("register".to_string()))
            .headers(headers)
            .send()
            .await?;

        self.handle_response(response, reqwest::StatusCode::OK)
            .await
    }
}

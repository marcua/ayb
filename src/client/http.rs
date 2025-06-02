use crate::ayb_db::models::{DBType, EntityDatabaseSharingLevel, EntityType, PublicSharingLevel};
use crate::error::AybError;
use crate::hosted_db::QueryResult;
use crate::http::structs::{
    APIToken, Database, DatabaseDetails, EmptyResponse, EntityQueryResponse, ShareList,
    SnapshotList,
};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::de::DeserializeOwned;
use std::collections::HashMap;

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

    async fn handle_empty_response(
        &self,
        response: reqwest::Response,
        expected_status: reqwest::StatusCode,
    ) -> Result<(), AybError> {
        let status = response.status();
        if status == expected_status {
            Ok(())
        } else {
            response
                .json::<AybError>()
                .await
                .map(Err)
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
        public_sharing_level: &PublicSharingLevel,
    ) -> Result<Database, AybError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("db-type"),
            HeaderValue::from_str(db_type.to_str()).unwrap(),
        );
        headers.insert(
            HeaderName::from_static("public-sharing-level"),
            HeaderValue::from_str(public_sharing_level.to_str()).unwrap(),
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

    pub async fn list_snapshots(
        &self,
        entity: &str,
        database: &str,
    ) -> Result<SnapshotList, AybError> {
        let mut headers = HeaderMap::new();
        self.add_bearer_token(&mut headers)?;

        let response = reqwest::Client::new()
            .get(self.make_url(format!("{}/{}/list_snapshots", entity, database)))
            .headers(headers)
            .send()
            .await?;

        self.handle_response(response, reqwest::StatusCode::OK)
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

    pub async fn entity_details(&self, entity: &str) -> Result<EntityQueryResponse, AybError> {
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

    pub async fn restore_snapshot(
        &self,
        entity: &str,
        database: &str,
        snapshot_id: &str,
    ) -> Result<(), AybError> {
        let mut headers = HeaderMap::new();
        self.add_bearer_token(&mut headers)?;

        let response = reqwest::Client::new()
            .post(self.make_url(format!("{}/{}/restore_snapshot", entity, database)))
            .headers(headers)
            .body(snapshot_id.to_owned())
            .send()
            .await?;

        self.handle_empty_response(response, reqwest::StatusCode::OK)
            .await
    }

    pub async fn update_profile(
        &self,
        entity: &str,
        profile_update: &HashMap<String, Option<String>>,
    ) -> Result<(), AybError> {
        let mut headers = HeaderMap::new();
        self.add_bearer_token(&mut headers)?;

        headers.insert(
            "Content-Type",
            "application/json"
                .parse()
                .expect("const value must be valid"),
        );

        let response = reqwest::Client::new()
            .patch(self.make_url(format!("entity/{}", entity)))
            .headers(headers)
            .body(serde_json::to_string(profile_update)?)
            .send()
            .await?;

        self.handle_empty_response(response, reqwest::StatusCode::OK)
            .await
    }

    pub async fn database_details(
        &self,
        entity: &str,
        database: &str,
    ) -> Result<DatabaseDetails, AybError> {
        let mut headers = HeaderMap::new();
        self.add_bearer_token(&mut headers)?;

        let response = reqwest::Client::new()
            .get(self.make_url(format!("{}/{}/details", entity, database)))
            .headers(headers)
            .send()
            .await?;

        self.handle_response(response, reqwest::StatusCode::OK)
            .await
    }

    pub async fn update_database(
        &self,
        entity: &str,
        database: &str,
        public_sharing_level: &PublicSharingLevel,
    ) -> Result<(), AybError> {
        let mut headers = HeaderMap::new();
        self.add_bearer_token(&mut headers)?;

        headers.insert(
            HeaderName::from_static("public-sharing-level"),
            HeaderValue::from_str(public_sharing_level.to_str()).unwrap(),
        );

        let response = reqwest::Client::new()
            .patch(self.make_url(format!("{}/{}/update", entity, database)))
            .headers(headers)
            .send()
            .await?;

        self.handle_empty_response(response, reqwest::StatusCode::OK)
            .await
    }

    pub async fn share(
        &self,
        entity_for_database: &str,
        database: &str,
        entity_for_permission: &str,
        sharing_level: &EntityDatabaseSharingLevel,
    ) -> Result<(), AybError> {
        let mut headers = HeaderMap::new();
        self.add_bearer_token(&mut headers)?;

        headers.insert(
            HeaderName::from_static("entity-for-permission"),
            HeaderValue::from_str(entity_for_permission).unwrap(),
        );

        headers.insert(
            HeaderName::from_static("sharing-level"),
            HeaderValue::from_str(sharing_level.to_str()).unwrap(),
        );

        let response = reqwest::Client::new()
            .post(self.make_url(format!("{}/{}/share", entity_for_database, database)))
            .headers(headers)
            .send()
            .await?;

        self.handle_empty_response(response, reqwest::StatusCode::NO_CONTENT)
            .await
    }

    pub async fn share_list(
        &self,
        entity: &str,
        database: &str,
    ) -> Result<ShareList, AybError> {
        let mut headers = HeaderMap::new();
        self.add_bearer_token(&mut headers)?;

        let response = reqwest::Client::new()
            .get(self.make_url(format!("{}/{}/share_list", entity, database)))
            .headers(headers)
            .send()
            .await?;

        self.handle_response(response, reqwest::StatusCode::OK)
            .await
    }
}

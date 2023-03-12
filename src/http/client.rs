use crate::error::StacksError;
use crate::hosted_db::QueryResult;
use crate::stacks_db::models::{DBType, EntityType, InstantiatedDatabase, InstantiatedEntity};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::de::DeserializeOwned;

pub struct StacksClient {
    pub base_url: String,
}

impl StacksClient {
    fn make_url(&self, endpoint: String) -> String {
        format!("{}/v1/{}", self.base_url, endpoint)
    }

    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, StacksError> {
        match response.status() {
            reqwest::StatusCode::OK => response.json::<T>().await.or_else(|err| {
                Err(StacksError {
                    message: format!("Unable to parse successful response: {}", err),
                })
            }),
            _other => {
                let error = response.json::<StacksError>().await;
                match error {
                    Ok(stacks_error) => Err(stacks_error),
                    Err(error) => Err(StacksError {
                        message: format!("Unable to parse error response: {:#?}", error),
                    }),
                }
            }
        }
    }

    pub async fn create_database(
        &self,
        entity: &str,
        database: &str,
        db_type: &DBType,
    ) -> Result<InstantiatedDatabase, StacksError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("db-type"),
            HeaderValue::from_str(db_type.to_str()).unwrap(),
        );

        let response = reqwest::Client::new()
            .post(self.make_url(format!("{}/{}", entity, database)))
            .headers(headers)
            .send()
            .await?;

        self.handle_response(response).await
    }

    pub async fn create_entity(
        &self,
        entity: &str,
        entity_type: &EntityType,
    ) -> Result<InstantiatedEntity, StacksError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("entity-type"),
            HeaderValue::from_str(entity_type.to_str()).unwrap(),
        );

        let response = reqwest::Client::new()
            .post(self.make_url(entity.to_owned()))
            .headers(headers)
            .send()
            .await?;

        self.handle_response(response).await
    }

    pub async fn query(
        &self,
        entity: &str,
        database: &str,
        query: &str,
    ) -> Result<QueryResult, StacksError> {
        let response = reqwest::Client::new()
            .post(self.make_url(format!("{}/{}/query", entity, database)))
            .body(query.to_owned())
            .send()
            .await?;

        self.handle_response(response).await
    }
}

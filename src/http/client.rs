use crate::error::StacksError;
use crate::stacks_db::models::{DBType, EntityType};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

pub struct StacksClient {
    pub base_url: String,
}

impl StacksClient {
    fn make_url(&self, endpoint: String) -> String {
        format!("{}/v1/{}", self.base_url, endpoint)
    }

    async fn handle_response(&self, response: reqwest::Response) -> Result<String, StacksError> {
        match response.status() {
            reqwest::StatusCode::OK => {
                println!("Success! {:?}", response);
                Ok("some success".to_owned())
                /*
                        match response.json::<APIResponse>().await {
                        Ok(parsed) => println!("Success! {:?}", parsed),
                        Err(_) => Err(Error {error_string: format!("Non-JSON response: {}", other.text)})
                };
                     */
            }
            other => Err(StacksError {
                error_string: format!(
                    "Response code: {}, text: {:?}",
                    other,
                    response.text().await?
                ),
            }),
        }
    }

    pub async fn create_database(
        &self,
        entity: &str,
        database: &str,
        db_type: &DBType,
    ) -> Result<String, StacksError> {
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
    ) -> Result<String, StacksError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("entity-type"),
            HeaderValue::from_str(entity_type.to_str()).unwrap(),
        );

        let response = reqwest::Client::new()
            .post(self.make_url(format!("{}", entity)))
            .headers(headers)
            .send()
            .await?;

        self.handle_response(response).await
    }

    pub fn query(&self) {}
}

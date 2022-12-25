use crate::http::structs::Error;
use crate::stacks_db::models::DBType;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

pub struct StacksClient {
    pub base_url: String,
}

impl StacksClient {
    fn make_url(&self, endpoint: String) -> String {
        format!("{}/v1/{}", self.base_url, endpoint)
    }

    pub async fn create_database(
        &self,
        entity: &str,
        database: &str,
        db_type: &DBType,
    ) -> Result<String, Error> {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("db-type"),
            HeaderValue::from_str(db_type.to_str()).unwrap(),
        );

        let response = reqwest::Client::new()
            .post(self.make_url(format!("{}/{}", entity, database)))
            .headers(headers)
            .send()
            .await;
        // TODO(marcua): Share this logic amongst the various calls.
        match response {
            Ok(response) => {
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
                    other => Err(Error {
                        error_string: format!(
                            "Response code: {}, text: {:?}",
                            other,
                            response.text().await
                        ),
                    }),
                }
            }
            Err(err) => Err(Error {
                error_string: err.to_string(),
            }),
        }
    }

    pub fn create_entity(&self) {}

    pub fn query(&self) {}
}

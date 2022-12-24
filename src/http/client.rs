use awc::Client;
use crate::http::structs::Error;
use crate::stacks_db::models::DBType;


pub struct StacksClient {
    pub base_url: String,
}

impl StacksClient {
    fn make_url(&self, endpoint: String) -> String {
        format!("{}/v1/{}", self.base_url, endpoint)
    }
    
    pub async fn create_database(&self, entity: &str, database: &str, db_type: &DBType) -> Result<String, Error> {
        let client = Client::default();
        
        let response = client
            .post(self.make_url(format!("{}/{}", entity, database)))
            .insert_header(("db-type", db_type.to_str()))
            .send()
            .await;

        Ok(format!("{:?}", response))
    }

    pub fn create_entity(&self) {

    }

    pub fn query(&self) {

    }
}

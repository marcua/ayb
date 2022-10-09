#[derive(Serialize, Deserialize)]
pub struct Database {
    pub entity: String,
    pub db_name: String,
    pub db_type: DBType,
    pub auth_tokens: Vec<String>
}

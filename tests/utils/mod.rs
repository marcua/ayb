use ayb::error::AybError;
use quoted_printable;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize)]
pub struct EmailEntry {
    from: String,
    to: String,
    reply_to: String,
    subject: String,
    content_type: String,
    content_transfer_encoding: String,
    date: String,
    content: Vec<String>,
}

pub fn extract_token(email: &EmailEntry) -> Result<String, AybError> {
    let prefix = "\tayb client confirm ";
    assert_eq!(email.subject, "Your login credentials");
    for line in &email.content {
        if line.starts_with(prefix) && line.len() > prefix.len() {
            return Ok(String::from_utf8(quoted_printable::decode(
                line[prefix.len()..].to_owned(),
                quoted_printable::ParseMode::Robust,
            )?)?);
        }
    }
    return Err(AybError {
        message: "No token found in email".to_owned(),
    });
}

pub fn parse_smtp_log(file_path: &str) -> Result<Vec<EmailEntry>, serde_json::Error> {
    let mut entries = Vec::new();
    for line in fs::read_to_string(file_path).unwrap().lines() {
        entries.push(serde_json::from_str(line)?);
    }
    return Ok(entries);
}

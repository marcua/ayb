use ayb::error::AybError;

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Output;

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

pub fn extract_api_key(output: &Output) -> Result<String, AybError> {
    let output_str = std::str::from_utf8(&output.stdout)?;
    let re = Regex::new(r"^Successfully authenticated and saved token (\S+)\n").unwrap();
    if re.is_match(output_str) {
        let captures = re.captures(output_str).unwrap();
        Ok(captures.get(1).map_or("", |m| m.as_str()).to_string())
    } else {
        Err(AybError::Other {
            message: "No API key".to_string(),
        })
    }
}

pub fn extract_token(email: &EmailEntry) -> Result<String, AybError> {
    let prefix = "\tayb client confirm ";
    assert_eq!(email.subject, "Your login credentials");
    for line in &email.content {
        if line.starts_with(prefix) && line.len() > prefix.len() {
            return Ok(String::from_utf8(quoted_printable::decode(
                &line[prefix.len()..],
                quoted_printable::ParseMode::Robust,
            )?)?);
        }
    }
    Err(AybError::Other {
        message: "No token found in email".to_string(),
    })
}

pub fn parse_smtp_log(file_path: &str) -> Result<Vec<EmailEntry>, serde_json::Error> {
    let mut entries = Vec::new();
    for line in fs::read_to_string(file_path).unwrap().lines() {
        entries.push(serde_json::from_str(line)?);
    }
    Ok(entries)
}

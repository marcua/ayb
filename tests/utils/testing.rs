use assert_cmd::prelude::*;
use ayb::error::AybError;

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::process::{Child, Command, Output};


// ayb_cmd!("value1", value2; {
//     "ENV_VAR" => env_value
// })
macro_rules! ayb_cmd {
    ($($value:expr),+; { $($env_left:literal => $env_right:expr),* $(,)? }) => {
        Command::cargo_bin("ayb")?
                .args([$($value,)*])
                $(.env($env_left, $env_right))*
    }
}

pub struct Cleanup;

impl Drop for Cleanup {
    fn drop(&mut self) {
        if let Err(err) = fs::remove_dir_all("/tmp/ayb/e2e") {
            assert_eq!(format!("{}", err), "No such file or directory (os error 2)")
        }
    }
}

pub struct AybServer(Child);
impl AybServer {
    pub fn run(db_type: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self(
            ayb_cmd!("server", "--config", &format!("tests/test-server-config-{}.toml", db_type); {
                "RUST_LOG" => "actix_web=debug",
                "RUST_BACKTRACE" => "1"
            })
            .spawn()?,
        ))
    }
}

impl Drop for AybServer {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}

pub struct SmtpServer(Child);

impl SmtpServer {
    pub fn run(smtp_port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(SmtpServer(
            Command::new("tests/smtp_server.sh")
                .args([&*format!("{}", smtp_port)])
                .spawn()?,
        ))
    }
}

impl Drop for SmtpServer {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}

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
    let re = Regex::new(r"^Successfully authenticated (\S+) and saved token (\S+)\n").unwrap();
    if re.is_match(output_str) {
        let captures = re.captures(output_str).unwrap();
        Ok(captures.get(2).map_or("", |m| m.as_str()).to_string())
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

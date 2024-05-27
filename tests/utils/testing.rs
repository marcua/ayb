use assert_cmd::prelude::*;
use std::fs;
use std::process::{Child, Command};

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

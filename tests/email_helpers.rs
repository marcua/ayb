use ayb::email::backend::EmailEntry;
use std::path::Path;

pub fn parse_email_file<P: AsRef<Path>>(
    path: P,
) -> Result<Vec<EmailEntry>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let mut emails = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if !line.is_empty() {
            emails.push(serde_json::from_str(line)?);
        }
    }

    Ok(emails)
}

pub fn extract_token_from_emails(emails: &[EmailEntry]) -> Option<String> {
    for email in emails {
        for line in &email.content {
            if line.starts_with('\t') {
                if let Some(token_part) = line.split("ayb client confirm ").nth(1) {
                    return Some(token_part.trim().to_string());
                }
            }
        }
    }
    None
}

const SQLITE_EMAIL_FILE: &str = "tests/ayb_data_sqlite/emails.jsonl";
const POSTGRES_EMAIL_FILE: &str = "tests/ayb_data_postgres/emails.jsonl";
const BROWSER_SQLITE_EMAIL_FILE: &str = "tests/ayb_data_browser_sqlite/emails.jsonl";

pub fn get_email_file_for_test_type(
    test_type: &str,
) -> Result<&'static str, Box<dyn std::error::Error>> {
    match test_type {
        "postgres" => Ok(POSTGRES_EMAIL_FILE),
        "browser_sqlite" => Ok(BROWSER_SQLITE_EMAIL_FILE),
        "sqlite" => Ok(SQLITE_EMAIL_FILE),
        _ => Err(format!("Unknown test type: {}", test_type).into()),
    }
}

pub fn clear_email_data(test_type: &str) -> Result<(), Box<dyn std::error::Error>> {
    let email_file = get_email_file_for_test_type(test_type)?;
    if Path::new(email_file).exists() {
        std::fs::remove_file(email_file)?;
    }
    Ok(())
}

pub fn get_emails_for_recipient(
    test_type: &str,
    recipient: &str,
) -> Result<Vec<EmailEntry>, Box<dyn std::error::Error>> {
    let email_file = get_email_file_for_test_type(test_type)?;
    let emails = parse_email_file(email_file)?;

    let filtered_emails = emails
        .into_iter()
        .filter(|email| email.to == recipient)
        .collect();

    Ok(filtered_emails)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_parse_email_file() {
        let test_file_path = "/tmp/test_parse_email_file.jsonl";
        let email_content = r#"{"from":"test@example.com","to":"user@example.com","reply_to":"noreply@example.com","subject":"Test Subject","content_type":"text/plain","content_transfer_encoding":"7bit","date":"Mon, 1 Jan 2024 00:00:00 +0000","content":["Test email body"]}
{"from":"test2@example.com","to":"user2@example.com","reply_to":"noreply@example.com","subject":"Test Subject 2","content_type":"text/plain","content_transfer_encoding":"7bit","date":"Mon, 1 Jan 2024 00:00:00 +0000","content":["Test email body 2"]}"#;

        fs::write(test_file_path, email_content).unwrap();

        let emails = parse_email_file(test_file_path).unwrap();
        assert_eq!(emails.len(), 2);
        assert_eq!(emails[0].from, "test@example.com");
        assert_eq!(emails[1].from, "test2@example.com");

        // Clean up
        let _ = fs::remove_file(test_file_path);
    }

    #[test]
    fn test_extract_token_from_emails() {
        let emails = vec![EmailEntry {
            from: "test@example.com".to_string(),
            to: "user@example.com".to_string(),
            reply_to: "noreply@example.com".to_string(),
            subject: "Your login credentials".to_string(),
            content_type: "text/plain".to_string(),
            content_transfer_encoding: "7bit".to_string(),
            date: "Mon, 1 Jan 2024 00:00:00 +0000".to_string(),
            content: vec![
                "To complete your registration, type".to_string(),
                "\tayb client confirm abc123token".to_string(),
            ],
        }];

        let token = extract_token_from_emails(&emails);
        assert_eq!(token, Some("abc123token".to_string()));
    }
}

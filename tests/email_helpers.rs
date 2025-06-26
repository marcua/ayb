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

pub fn clear_email_file<P: AsRef<Path>>(path: P) -> Result<(), std::io::Error> {
    if path.as_ref().exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_email_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let email_content = r#"{"from":"test@example.com","to":"user@example.com","reply_to":"noreply@example.com","subject":"Test Subject","content_type":"text/plain","content_transfer_encoding":"7bit","date":"Mon, 1 Jan 2024 00:00:00 +0000","content":["Test email body"]}
{"from":"test2@example.com","to":"user2@example.com","reply_to":"noreply@example.com","subject":"Test Subject 2","content_type":"text/plain","content_transfer_encoding":"7bit","date":"Mon, 1 Jan 2024 00:00:00 +0000","content":["Test email body 2"]}"#;

        fs::write(temp_file.path(), email_content).unwrap();

        let emails = parse_email_file(temp_file.path()).unwrap();
        assert_eq!(emails.len(), 2);
        assert_eq!(emails[0].from, "test@example.com");
        assert_eq!(emails[1].from, "test2@example.com");
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

    #[test]
    fn test_clear_email_file() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), "test content").unwrap();
        assert!(temp_file.path().exists());

        clear_email_file(temp_file.path()).unwrap();
        assert!(!temp_file.path().exists());
    }
}

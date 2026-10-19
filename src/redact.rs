pub fn redact_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if let Some(rest) = line[i..].strip_prefix("Bearer ") {
            out.push_str("Bearer [redacted]");
            i += 7;
            let skip = rest.find(char::is_whitespace).unwrap_or(rest.len());
            i += skip;
            continue;
        }
        let mut skipped = false;
        for key in ["client_secret", "access_token", "refresh_token"] {
            let pattern = format!("\"{key}\"");
            if line[i..].starts_with(&pattern) {
                out.push_str(&pattern);
                out.push_str(":[redacted]");
                i += pattern.len();
                let rest = &line[i..];
                if let Some(start) = rest.find('"') {
                    if let Some(end) = rest[start + 1..].find('"') {
                        i += start + 1 + end + 1;
                    }
                }
                skipped = true;
                break;
            }
        }
        if skipped {
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_bearer_and_json_secrets() {
        let line = r#"Authorization: Bearer super-secret-value {"client_secret":"abc","access_token":"zzz"}"#;
        let clean = redact_line(line);
        assert!(!clean.contains("super-secret-value"));
        assert!(!clean.contains("abc"));
        assert!(!clean.contains("zzz"));
        assert!(clean.contains("[redacted]"));
    }
}

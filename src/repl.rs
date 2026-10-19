//! Shell-style tokenizer shared by the REPL. Clap parses the resulting arguments.

use crate::error::Error;

pub fn tokenize(input: &str) -> Result<Vec<String>, Error> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_arg = false;
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                in_arg = true;
                match chars.next() {
                    Some(next) => current.push(next),
                    None => return Err(Error::UnclosedQuote),
                }
            }
            '"' | '\'' => {
                in_arg = true;
                let quote = ch;
                let mut closed = false;
                while let Some(next) = chars.next() {
                    if next == '\\' && quote == '"' {
                        match chars.next() {
                            Some(escaped) => current.push(escaped),
                            None => return Err(Error::UnclosedQuote),
                        }
                        continue;
                    }
                    if next == quote {
                        closed = true;
                        break;
                    }
                    current.push(next);
                }
                if !closed {
                    return Err(Error::UnclosedQuote);
                }
            }
            c if c.is_whitespace() => {
                if in_arg {
                    args.push(std::mem::take(&mut current));
                    in_arg = false;
                }
            }
            other => {
                in_arg = true;
                current.push(other);
            }
        }
    }
    if in_arg {
        args.push(current);
    }
    Ok(args)
}

pub fn sensitive(line: &str) -> bool {
    let lower = line.to_ascii_lowercase().replace('-', "_");
    lower.contains("client_secret")
        || lower.contains("refresh_token")
        || lower.contains("access_token")
        || lower.contains("bearer ")
}

pub fn complete(line: &str) -> Option<String> {
    let matches: Vec<&str> = COMMANDS
        .iter()
        .copied()
        .filter(|cmd| cmd.starts_with(line))
        .collect();
    match matches.len() {
        0 => None,
        1 => Some(matches[0].to_string()),
        _ => Some(common_prefix(&matches).unwrap_or_else(|| line.to_string())),
    }
}

fn common_prefix(items: &[&str]) -> Option<String> {
    let first = items.first()?;
    let mut prefix = first.to_string();
    for item in items.iter().skip(1) {
        while !item.starts_with(&prefix) {
            prefix.pop()?;
        }
    }
    if prefix.is_empty() {
        None
    } else {
        Some(prefix)
    }
}

const COMMANDS: &[&str] = &[
    "auth configure",
    "auth status",
    "doctor",
    "help",
    "job create --input ",
    "job checksum ",
    "job threads ",
    "job drop ",
    "job export ",
    "job list",
    "job merge ",
    "job pause ",
    "job prepare ",
    "job requeue-unknown ",
    "job resume ",
    "job retry-failed ",
    "job run ",
    "job status ",
    "tui",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_and_spaces() {
        let args = tokenize(r#"job create --input "my file.csv""#).unwrap();
        assert_eq!(args, vec!["job", "create", "--input", "my file.csv"]);
    }

    #[test]
    fn unclosed_quote_is_an_error() {
        assert!(tokenize("job create --input \"abc").is_err());
    }

    #[test]
    fn random_inputs_do_not_panic() {
        let mut seed = 0xdecafu64;
        for _ in 0..300 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let len = (seed % 40) as usize;
            let mut text = String::new();
            for i in 0..len {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let byte = (seed % 96) as u8 + 32;
                if i == 3 && seed.is_multiple_of(5) {
                    text.push('"');
                }
                text.push(byte as char);
            }
            let _ = tokenize(&text);
        }
    }

    #[test]
    fn completion_and_redaction() {
        assert_eq!(complete("job l").as_deref(), Some("job list"));
        assert!(sensitive("auth configure client_secret=abc"));
        assert!(!sensitive("job list"));
    }
}

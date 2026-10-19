use crate::engine::{App, MAX_THREADS};
use crate::error::Error;
use crate::jobcfg::JobConfig;

pub(super) struct ExportForm {
    pub job: String,
    pub format_csv: bool,
    pub partial: bool,
    pub focus: usize,
}
impl ExportForm {
    pub fn key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab | KeyCode::BackTab | KeyCode::Up | KeyCode::Down => {
                self.focus = 1 - self.focus
            }
            KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right => {
                if self.focus == 0 {
                    self.format_csv = !self.format_csv;
                } else {
                    self.partial = !self.partial;
                }
            }
            _ => {}
        }
    }
}
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

pub(super) struct ThreadForm {
    pub job: Option<String>,
    pub value: String,
    pub previous: u32,
    pub cursor: Option<usize>,
}

impl ThreadForm {
    pub fn validate(&self) -> Result<u32, Error> {
        self.value
            .parse::<u32>()
            .ok()
            .filter(|n| (1..=MAX_THREADS).contains(n))
            .ok_or_else(|| {
                Error::Msg(format!(
                    "Threads must be an integer from 1 to {MAX_THREADS}. No changes saved."
                ))
            })
    }
}

#[derive(Clone)]
pub(super) struct ImportForm {
    pub path: String,
    pub threads: String,
    pub validation: usize,
    pub focus: usize,
    cursors: [Option<usize>; 2],
}

impl ImportForm {
    pub fn paste(&mut self, value: &str) {
        if self.focus < 2 {
            let text = if self.focus == 0 {
                &mut self.path
            } else {
                &mut self.threads
            };
            paste(text, &mut self.cursors[self.focus], value, 4096);
        }
    }
    pub fn cursor(&self) -> Option<usize> {
        if self.focus < 2 {
            self.cursors[self.focus]
        } else {
            None
        }
    }
    pub fn new(threads: u32) -> Self {
        Self {
            path: String::new(),
            threads: threads.to_string(),
            validation: 1,
            focus: 0,
            cursors: [None, None],
        }
    }
    pub fn from_defaults(settings: &crate::engine::ProcessingDefaults) -> Self {
        let mut form = Self::new(settings.threads);
        form.validation = match settings.validation.as_str() {
            "length" => 0,
            "none" => 2,
            _ => 1,
        };
        form
    }
    pub fn validation_name(&self) -> &'static str {
        ["length", "checksum", "none"][self.validation % 3]
    }
    pub fn key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab | KeyCode::Down => self.focus = (self.focus + 1) % 3,
            KeyCode::BackTab | KeyCode::Up => self.focus = (self.focus + 2) % 3,
            KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right if self.focus == 2 => {
                self.validation = (self.validation + 1) % 3;
            }
            _ if self.focus < 2 => {
                let text = if self.focus == 0 {
                    &mut self.path
                } else {
                    &mut self.threads
                };
                edit(text, &mut self.cursors[self.focus], key, 4096);
            }
            _ => {}
        }
    }
    pub fn config(&self, app: &App) -> Result<(PathBuf, JobConfig), Error> {
        let path = PathBuf::from(self.path.trim());
        if self.path.trim().is_empty() {
            return Err(Error::Msg("Enter the full CSV path.".into()));
        }
        if !std::fs::metadata(&path)?.is_file() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "CSV missing",
            )));
        }
        let threads = ThreadForm {
            job: None,
            value: self.threads.clone(),
            previous: 8,
            cursor: None,
        }
        .validate()?;
        let mut config = crate::jobcfg::default_tuning();
        let defaults = app.processing_defaults()?;
        config.rps = defaults.rps;
        config.batch_size = defaults.batch_size;
        let saved = app.api_config()?;
        config.endpoint = saved.process_url;
        config.token_url = saved.token_url;
        config.client_id = saved.client_id;
        config.concurrency = threads;
        config.validation = self.validation_name().into();
        Ok((path, config))
    }
}

/// Byte cursor with Unicode boundaries; None follows the end while typing.
pub(crate) fn paste(text: &mut String, cursor: &mut Option<usize>, value: &str, limit: usize) {
    let mut at = cursor.unwrap_or(text.len()).min(text.len());
    while !text.is_char_boundary(at) {
        at = at.saturating_sub(1);
    }
    let mut literal = String::new();
    for ch in value.chars().take(limit) {
        if ch.is_control() {
            continue;
        }
        if text.len() + literal.len() + ch.len_utf8() > limit {
            break;
        }
        literal.push(ch);
    }
    text.insert_str(at, &literal);
    at += literal.len();
    *cursor = (at < text.len()).then_some(at);
}

pub(crate) fn edit(text: &mut String, cursor: &mut Option<usize>, key: KeyEvent, limit: usize) {
    let mut at = cursor.unwrap_or(text.len()).min(text.len());
    while !text.is_char_boundary(at) {
        at = at.saturating_sub(1);
    }
    let before = || {
        text[..at]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0)
    };
    match key.code {
        KeyCode::Left => *cursor = Some(before()),
        KeyCode::Right => {
            *cursor = text[at..]
                .chars()
                .next()
                .map(|ch| at + ch.len_utf8())
                .filter(|i| *i < text.len())
        }
        KeyCode::Home => *cursor = Some(0),
        KeyCode::End => *cursor = None,
        KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
            text.clear();
            *cursor = None;
        }
        KeyCode::Backspace if at > 0 => {
            let start = before();
            text.replace_range(start..at, "");
            *cursor = (start < text.len()).then_some(start);
        }
        KeyCode::Delete if at < text.len() => {
            let end = at + text[at..].chars().next().unwrap().len_utf8();
            text.replace_range(at..end, "");
            *cursor = Some(at);
        }
        KeyCode::Char(ch)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                && !ch.is_control()
                && text.len() + ch.len_utf8() <= limit =>
        {
            text.insert(at, ch);
            at += ch.len_utf8();
            *cursor = (at < text.len()).then_some(at);
        }
        _ => {}
    }
}

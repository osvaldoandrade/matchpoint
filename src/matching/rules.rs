use super::{bad, hex};
use crate::error::Error;
use csv::{Reader, ReaderBuilder, StringRecord};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{borrow::Cow, fs::File, path::Path};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Transform {
    #[default]
    None,
    Sha256,
    Sha256Raw,
}
impl Transform {
    pub fn bytes(self, key: &str) -> Cow<'_, [u8]> {
        match self {
            Self::None => Cow::Borrowed(key.as_bytes()),
            Self::Sha256 => Cow::Owned(hex(&Sha256::digest(key.as_bytes())).into_bytes()),
            Self::Sha256Raw => Cow::Owned(Sha256::digest(key.as_bytes()).to_vec()),
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Sha256 => "SHA-256 hex",
            Self::Sha256Raw => "SHA-256 raw",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::None => Self::Sha256,
            Self::Sha256 => Self::Sha256Raw,
            Self::Sha256Raw => Self::None,
        }
    }
    pub(crate) fn code(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Sha256 => 1,
            Self::Sha256Raw => 2,
        }
    }
    pub(crate) fn from_code(code: u32) -> Result<Self, Error> {
        match code {
            0 => Ok(Self::None),
            1 => Ok(Self::Sha256),
            2 => Ok(Self::Sha256Raw),
            _ => Err(bad("Unknown DAT key transform.")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Rule {
    pub column: String,
    pub headers: bool,
    pub delimiter: u8,
    pub transform: Transform,
    pub trim: bool,
}
impl Default for Rule {
    fn default() -> Self {
        Self {
            column: "#1".into(),
            headers: true,
            delimiter: b',',
            transform: Transform::None,
            trim: false,
        }
    }
}
impl Rule {
    pub fn validate(&self) -> Result<(), Error> {
        if self.column.is_empty()
            || self.column.len() > 256
            || self.column.chars().any(char::is_control)
        {
            return Err(bad(
                "Choose a column name or a one-based column such as #1.",
            ));
        }
        if self.column.starts_with('#')
            && self.column[1..]
                .parse::<usize>()
                .ok()
                .filter(|i| *i > 0)
                .is_none()
        {
            return Err(bad("Column indexes start at #1."));
        }
        if !self.delimiter.is_ascii() || matches!(self.delimiter, 0 | b'\r' | b'\n' | b'"') {
            return Err(bad("Choose a single ASCII CSV delimiter."));
        }
        Ok(())
    }
    pub fn lookup<'a>(&self, original: &'a str) -> &'a str {
        if self.trim {
            original.trim()
        } else {
            original
        }
    }
}

pub struct CsvKeys {
    reader: Reader<File>,
    index: usize,
    record: StringRecord,
    row: u64,
}
impl CsvKeys {
    pub fn open(path: &Path, rule: &Rule) -> Result<Self, Error> {
        rule.validate()?;
        if !rule.headers && !rule.column.starts_with('#') {
            return Err(bad("Headerless CSV requires a column index such as #1."));
        }
        let mut reader = ReaderBuilder::new()
            .has_headers(rule.headers)
            .delimiter(rule.delimiter)
            .from_path(path)
            .map_err(|_| bad("Cannot open the CSV input."))?;
        let index = if let Some(number) = rule.column.strip_prefix('#') {
            number
                .parse::<usize>()
                .map_err(|_| bad("Invalid column index."))?
                - 1
        } else {
            let header = reader
                .headers()
                .map_err(|_| bad("Cannot read CSV headers."))?;
            let positions = header
                .iter()
                .enumerate()
                .filter(|(_, name)| *name == rule.column)
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            if positions.len() != 1 {
                return Err(bad("Selected CSV header is missing or duplicated."));
            }
            positions[0]
        };
        if rule.headers
            && index
                >= reader
                    .headers()
                    .map_err(|_| bad("Cannot read CSV headers."))?
                    .len()
        {
            return Err(bad("Selected CSV column is outside the header."));
        }
        Ok(Self {
            reader,
            index,
            record: StringRecord::new(),
            row: 0,
        })
    }
    pub fn next_key(&mut self) -> Result<Option<String>, Error> {
        if !self
            .reader
            .read_record(&mut self.record)
            .map_err(|_| bad(&format!("Invalid CSV near row {}.", self.row + 1)))?
        {
            return Ok(None);
        }
        self.row += 1;
        let key = self
            .record
            .get(self.index)
            .ok_or_else(|| bad("Selected column is missing from a CSV record."))?;
        if key.is_empty() || key.len() > 4096 {
            return Err(bad(&format!(
                "CSV key at row {} must have 1–4096 UTF-8 bytes.",
                self.row
            )));
        }
        Ok(Some(key.to_string()))
    }
    pub(crate) fn position(&self) -> csv::Position {
        self.reader.position().clone()
    }
    pub(crate) fn seek(&mut self, byte: u64, line: u64, record: u64) -> Result<(), Error> {
        let mut position = csv::Position::new();
        position.set_byte(byte).set_line(line).set_record(record);
        self.reader
            .seek(position)
            .map_err(|_| bad("Invalid CSV checkpoint."))?;
        self.row = record;
        Ok(())
    }
}

pub(crate) fn output_key(key: &str) -> String {
    if key.trim() != key
        || key
            .chars()
            .any(|c| c.is_control() || matches!(c, ':' | '\\' | '"'))
    {
        serde_json::to_string(key).expect("serializing a string cannot fail")
    } else {
        key.into()
    }
}

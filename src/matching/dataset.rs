//! Bounded readers: CSV records or a single projected Parquet column per batch.
use super::{bad, rules::CsvKeys, Rule};
use crate::error::Error;
use arrow_array::{Array, RecordBatch};
use arrow_schema::DataType;
use parquet::arrow::{
    arrow_reader::{ParquetRecordBatchReader, ParquetRecordBatchReaderBuilder},
    ProjectionMask,
};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};

pub enum Dataset {
    Csv(Box<CsvKeys>),
    Parquet(Box<ParquetKeys>),
}
impl Dataset {
    pub fn open(path: &Path, rule: &Rule) -> Result<Self, Error> {
        let mut magic = [0; 4];
        let n = File::open(path)?.read(&mut magic)?;
        if n == 4 && &magic == b"PAR1" {
            Ok(Self::Parquet(Box::new(ParquetKeys::open(path, rule, 0)?)))
        } else {
            Ok(Self::Csv(Box::new(CsvKeys::open(path, rule)?)))
        }
    }
    pub fn next_key(&mut self) -> Result<Option<String>, Error> {
        match self {
            Self::Csv(reader) => reader.next_key(),
            Self::Parquet(reader) => reader.next_key(),
        }
    }
    pub(crate) fn position(&self) -> csv::Position {
        match self {
            Self::Csv(reader) => reader.position(),
            Self::Parquet(reader) => {
                let mut position = csv::Position::new();
                position.set_record(reader.row);
                position
            }
        }
    }
    pub(crate) fn seek(&mut self, byte: u64, line: u64, record: u64) -> Result<(), Error> {
        match self {
            Self::Csv(reader) => reader.seek(byte, line, record),
            Self::Parquet(reader) => {
                **reader = ParquetKeys::open(&reader.path, &reader.rule, record)?;
                Ok(())
            }
        }
    }
}
pub struct ParquetKeys {
    path: PathBuf,
    rule: Rule,
    reader: ParquetRecordBatchReader,
    batch: Option<RecordBatch>,
    at: usize,
    row: u64,
}
impl ParquetKeys {
    fn open(path: &Path, rule: &Rule, offset: u64) -> Result<Self, Error> {
        rule.validate()?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(File::open(path)?)
            .map_err(|_| bad("Invalid Parquet metadata."))?;
        let schema = builder.schema();
        let index = if let Some(number) = rule.column.strip_prefix('#') {
            number
                .parse::<usize>()
                .map_err(|_| bad("Invalid column index."))?
                - 1
        } else {
            let found = schema
                .fields()
                .iter()
                .enumerate()
                .filter(|(_, field)| field.name() == &rule.column)
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            if found.len() != 1 {
                return Err(bad("Selected Parquet column is missing or duplicated."));
            }
            found[0]
        };
        let field = schema
            .fields()
            .get(index)
            .ok_or_else(|| bad("Selected column is outside the Parquet schema."))?;
        if !matches!(
            field.data_type(),
            DataType::Utf8
                | DataType::LargeUtf8
                | DataType::Utf8View
                | DataType::Int8
                | DataType::Int16
                | DataType::Int32
                | DataType::Int64
                | DataType::UInt8
                | DataType::UInt16
                | DataType::UInt32
                | DataType::UInt64
        ) {
            return Err(bad("Parquet keys must be UTF-8 strings or integers. Store identifiers as strings to preserve leading zeros."));
        }
        let projection = ProjectionMask::roots(builder.parquet_schema(), [index]);
        let reader = builder
            .with_projection(projection)
            .with_batch_size(8192)
            .with_offset(
                usize::try_from(offset)
                    .map_err(|_| bad("Parquet checkpoint exceeds this platform."))?,
            )
            .build()
            .map_err(|_| bad("Cannot initialize Parquet reader."))?;
        Ok(Self {
            path: path.into(),
            rule: rule.clone(),
            reader,
            batch: None,
            at: 0,
            row: offset,
        })
    }
    fn next_key(&mut self) -> Result<Option<String>, Error> {
        while self
            .batch
            .as_ref()
            .is_none_or(|batch| self.at >= batch.num_rows())
        {
            self.batch = match self.reader.next() {
                Some(result) => Some(result.map_err(|_| bad("Invalid Parquet data page."))?),
                None => return Ok(None),
            };
            self.at = 0;
        }
        let array = self.batch.as_ref().unwrap().column(0);
        if array.is_null(self.at) {
            return Err(bad(&format!("Null Parquet key at row {}.", self.row + 1)));
        }
        let key = value(array, self.at)?;
        self.at += 1;
        self.row += 1;
        if key.is_empty() || key.len() > 4096 {
            return Err(bad(&format!(
                "Parquet key at row {} must have 1–4096 UTF-8 bytes.",
                self.row
            )));
        }
        Ok(Some(key))
    }
}
fn value(array: &Arc<dyn Array>, at: usize) -> Result<String, Error> {
    macro_rules! downcast {($($kind:ty),*)=>{$(if let Some(a)=array.as_any().downcast_ref::<$kind>() {return Ok(a.value(at).to_string());})*};}
    downcast!(
        arrow_array::StringArray,
        arrow_array::LargeStringArray,
        arrow_array::StringViewArray,
        arrow_array::Int8Array,
        arrow_array::Int16Array,
        arrow_array::Int32Array,
        arrow_array::Int64Array,
        arrow_array::UInt8Array,
        arrow_array::UInt16Array,
        arrow_array::UInt32Array,
        arrow_array::UInt64Array
    );
    Err(bad("Unsupported Parquet key representation."))
}

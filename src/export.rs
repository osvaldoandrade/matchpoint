use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use arrow_array::{BooleanArray, Int32Array, Int64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;

use crate::disk::publish_file;
use crate::error::Error;
use crate::prepare::sha256_file;
use crate::store::{Db, ExportRow};

pub fn export_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("row_id", DataType::Int64, false),
        Field::new("cpf", DataType::Utf8, false),
        Field::new("status", DataType::Utf8, false),
        Field::new("is_customer", DataType::Boolean, true),
        Field::new("checked_at", DataType::Utf8, true),
        Field::new("attempts", DataType::Int64, false),
        Field::new("http_status", DataType::Int32, true),
        Field::new("error_code", DataType::Utf8, true),
        Field::new("request_id", DataType::Utf8, true),
        Field::new("evidence_source", DataType::Utf8, false),
    ]))
}

pub fn write_parquet(
    db: &Db,
    final_path: &Path,
    fail_before_rename: bool,
) -> Result<(u64, String), Error> {
    if let Some(parent) = final_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = temp_path(final_path);
    let file = File::create(&tmp)?;
    let props = WriterProperties::builder()
        .set_compression(Compression::SNAPPY)
        .set_max_row_group_row_count(Some(10_000))
        .build();
    let mut writer = ArrowWriter::try_new(file, export_schema(), Some(props))
        .map_err(|e| Error::Msg(format!("parquet: {e}")))?;
    let mut after = 0_i64;
    let mut rows = 0_u64;
    let mut previous = 0_i64;
    loop {
        let page = db.export_page(after, 10_000)?;
        if page.is_empty() {
            break;
        }
        for row in &page {
            if previous != 0 && row.row_id != previous + 1 {
                return Err(Error::RowGap { row_id: row.row_id });
            }
            if previous == 0 && row.row_id != 1 && rows == 0 {
                return Err(Error::RowGap { row_id: row.row_id });
            }
            previous = row.row_id;
            rows += 1;
        }
        let batch = to_batch(&page)?;
        writer
            .write(&batch)
            .map_err(|e| Error::Msg(format!("parquet: {e}")))?;
        after = page.last().map(|r| r.row_id).unwrap_or(after);
    }
    writer
        .close()
        .map_err(|e| Error::Msg(format!("parquet: {e}")))?;
    if rows > 0 && previous != rows as i64 {
        return Err(Error::RowGap { row_id: previous });
    }
    if fail_before_rename {
        return Err(Error::DiskFull);
    }
    publish_file(&tmp, final_path)?;
    let hash = sha256_file(final_path)?;
    Ok((rows, hash))
}

pub fn write_csv(
    db: &Db,
    final_path: &Path,
    fail_before_rename: bool,
) -> Result<(u64, String), Error> {
    if let Some(parent) = final_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = temp_path(final_path);
    let mut file = File::create(&tmp)?;
    let mut writer = csv::Writer::from_writer(&mut file);
    writer
        .write_record([
            "row_id",
            "cpf",
            "status",
            "is_customer",
            "checked_at",
            "attempts",
            "http_status",
            "error_code",
            "request_id",
            "evidence_source",
        ])
        .map_err(|e| Error::Csv(e.to_string()))?;
    let mut after = 0_i64;
    let mut rows = 0_u64;
    let mut previous = 0_i64;
    loop {
        let page = db.export_page(after, 10_000)?;
        if page.is_empty() {
            break;
        }
        for row in &page {
            if rows == 0 && row.row_id != 1 {
                return Err(Error::RowGap { row_id: row.row_id });
            }
            if previous != 0 && row.row_id != previous + 1 {
                return Err(Error::RowGap { row_id: row.row_id });
            }
            previous = row.row_id;
            rows += 1;
            writer
                .write_record([
                    row.row_id.to_string(),
                    row.cpf.clone(),
                    row.state.clone(),
                    match row.is_customer {
                        Some(true) => "true".into(),
                        Some(false) => "false".into(),
                        None => String::new(),
                    },
                    row.checked_at.clone().unwrap_or_default(),
                    row.attempts.to_string(),
                    row.http_status.map(|v| v.to_string()).unwrap_or_default(),
                    row.error_code.clone().unwrap_or_default(),
                    row.request_id.clone().unwrap_or_default(),
                    row.evidence_source.clone(),
                ])
                .map_err(|e| Error::Csv(e.to_string()))?;
        }
        after = page.last().map(|r| r.row_id).unwrap_or(after);
    }
    writer.flush().map_err(|e| Error::Csv(e.to_string()))?;
    drop(writer);
    file.sync_all()?;
    if rows > 0 && previous != rows as i64 {
        return Err(Error::RowGap { row_id: previous });
    }
    if fail_before_rename {
        return Err(Error::DiskFull);
    }
    publish_file(&tmp, final_path)?;
    Ok((rows, sha256_file(final_path)?))
}

fn to_batch(page: &[ExportRow]) -> Result<RecordBatch, Error> {
    let row_id: Vec<i64> = page.iter().map(|r| r.row_id).collect();
    let cpf: Vec<&str> = page.iter().map(|r| r.cpf.as_str()).collect();
    let status: Vec<&str> = page.iter().map(|r| r.state.as_str()).collect();
    let customer: Vec<Option<bool>> = page.iter().map(|r| r.is_customer).collect();
    let checked: Vec<Option<&str>> = page.iter().map(|r| r.checked_at.as_deref()).collect();
    let attempts: Vec<i64> = page.iter().map(|r| r.attempts).collect();
    let http: Vec<Option<i32>> = page
        .iter()
        .map(|r| r.http_status.and_then(|v| i32::try_from(v).ok()))
        .collect();
    let error: Vec<Option<&str>> = page.iter().map(|r| r.error_code.as_deref()).collect();
    let request: Vec<Option<&str>> = page.iter().map(|r| r.request_id.as_deref()).collect();
    let evidence: Vec<&str> = page.iter().map(|r| r.evidence_source.as_str()).collect();
    RecordBatch::try_new(
        export_schema(),
        vec![
            Arc::new(Int64Array::from(row_id)),
            Arc::new(StringArray::from(cpf)),
            Arc::new(StringArray::from(status)),
            Arc::new(BooleanArray::from(customer)),
            Arc::new(StringArray::from(checked)),
            Arc::new(Int64Array::from(attempts)),
            Arc::new(Int32Array::from(http)),
            Arc::new(StringArray::from(error)),
            Arc::new(StringArray::from(request)),
            Arc::new(StringArray::from(evidence)),
        ],
    )
    .map_err(|e| Error::Msg(format!("arrow: {e}")))
}

fn temp_path(final_path: &Path) -> std::path::PathBuf {
    let mut name = final_path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    final_path.with_file_name(name)
}

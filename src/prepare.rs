use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use csv::{Reader, ReaderBuilder, StringRecord};
use sha2::{Digest, Sha256};

use crate::domain::{validate_cpf, Validation};
use crate::error::Error;
use crate::store::{Checkpoint, Db, NewRow};
use crate::timeutil::{format_rfc3339, WallClock};

pub fn copy_and_hash(src: &Path, dst: &Path) -> Result<String, Error> {
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let partial = dst.with_extension("csv.partial");
    let mut input = File::open(src)?;
    let mut output = File::create(&partial)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0_u8; 1024 * 1024];
    loop {
        let n = input.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        output.write_all(&buf[..n])?;
    }
    output.sync_all()?;
    drop(output);
    std::fs::rename(&partial, dst)?;
    if let Some(parent) = dst.parent() {
        File::open(parent)?.sync_all()?;
    }
    Ok(hex(&hasher.finalize()))
}

pub fn sha256_file(path: &Path) -> Result<String, Error> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0_u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

struct Batch {
    id: i64,
    rows: Vec<NewRow>,
}

pub fn import_csv(
    db: &mut Db,
    csv_path: &Path,
    batch_size: u32,
    validation: Validation,
    clock: &dyn WallClock,
    fail_before_commit_batch: Option<u32>,
) -> Result<i64, Error> {
    let batch_size = batch_size.max(1) as i64;
    let mut ckpt = db.checkpoint()?;
    if ckpt.complete {
        return Ok(ckpt.rows_imported);
    }
    let file = File::open(csv_path)?;
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .flexible(false)
        .from_reader(file);
    if ckpt.byte_offset > 0 || ckpt.record_index > 0 {
        seek_to(&mut reader, &ckpt)?;
    }
    let mut column = db
        .meta_get("cpf_column")?
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut buffered: Option<StringRecord> = None;
    if !ckpt.header_done {
        let mut record = StringRecord::new();
        if !reader
            .read_record(&mut record)
            .map_err(|e| Error::Csv(e.to_string()))?
        {
            ckpt.header_done = true;
            ckpt.complete = true;
            capture_pos(&mut ckpt, &reader);
            db.insert_prepared(1, &[], &ckpt, &now(clock), false)?;
            return Ok(0);
        }
        if record.iter().any(|cell| cell.eq_ignore_ascii_case("cpf")) {
            column = record
                .iter()
                .position(|cell| cell.eq_ignore_ascii_case("cpf"))
                .unwrap_or(0);
            db.meta_set("cpf_column", &column.to_string())?;
            db.meta_set("has_header", "1")?;
            ckpt.header_done = true;
            capture_pos(&mut ckpt, &reader);
            db.insert_prepared(1, &[], &ckpt, &now(clock), false)?;
        } else if record.len() != 1 {
            return Err(Error::Csv(
                "expected one column or a header named cpf".into(),
            ));
        } else {
            db.meta_set("cpf_column", "0")?;
            db.meta_set("has_header", "0")?;
            ckpt.header_done = true;
            buffered = Some(record);
        }
    }
    let mut batch = Batch {
        id: (ckpt.rows_imported / batch_size) + 1,
        rows: Vec::new(),
    };
    let mut committed_batches = 0_u32;
    if let Some(record) = buffered {
        committed_batches += push(
            db,
            &reader,
            &record,
            column,
            validation,
            &mut ckpt,
            batch_size,
            clock,
            &mut batch,
            fail_before_commit_batch,
            &mut committed_batches,
        )?;
    }
    let mut record = StringRecord::new();
    loop {
        if !reader
            .read_record(&mut record)
            .map_err(|e| Error::Csv(e.to_string()))?
        {
            break;
        }
        if record
            .iter()
            .any(|field| field.contains('\n') || field.contains('\r'))
        {
            return Err(Error::Csv("unterminated quote or multiline field".into()));
        }
        if db.meta_get("has_header")?.as_deref() != Some("1") && record.len() != 1 {
            return Err(Error::Csv(
                "expected one column or a header named cpf".into(),
            ));
        }
        committed_batches += push(
            db,
            &reader,
            &record,
            column,
            validation,
            &mut ckpt,
            batch_size,
            clock,
            &mut batch,
            fail_before_commit_batch,
            &mut committed_batches,
        )?;
    }
    if !batch.rows.is_empty() {
        capture_pos(&mut ckpt, &reader);
        let fail = fail_before_commit_batch == Some(committed_batches + 1);
        ckpt.rows_imported += batch.rows.len() as i64;
        db.insert_prepared(batch.id, &batch.rows, &ckpt, &now(clock), fail)?;
    }
    ckpt.complete = true;
    capture_pos(&mut ckpt, &reader);
    db.insert_prepared(batch.id, &[], &ckpt, &now(clock), false)?;
    Ok(ckpt.rows_imported)
}

#[allow(clippy::too_many_arguments)]
fn push(
    db: &mut Db,
    reader: &Reader<File>,
    record: &StringRecord,
    column: usize,
    validation: Validation,
    ckpt: &mut Checkpoint,
    batch_size: i64,
    clock: &dyn WallClock,
    batch: &mut Batch,
    fail_before_commit_batch: Option<u32>,
    committed_batches: &mut u32,
) -> Result<u32, Error> {
    let row_id = ckpt.rows_imported + batch.rows.len() as i64 + 1;
    let cpf = record.get(column).unwrap_or("").to_string();
    batch.rows.push(NewRow {
        row_id,
        cpf: cpf.clone(),
        rejected: validate_cpf(validation, &cpf),
    });
    if batch.rows.len() as i64 >= batch_size {
        capture_pos(ckpt, reader);
        *committed_batches += 1;
        let fail = fail_before_commit_batch == Some(*committed_batches);
        ckpt.rows_imported += batch.rows.len() as i64;
        db.insert_prepared(batch.id, &batch.rows, ckpt, &now(clock), fail)?;
        batch.rows.clear();
        batch.id += 1;
        return Ok(0);
    }
    Ok(0)
}

fn seek_to(reader: &mut Reader<File>, ckpt: &Checkpoint) -> Result<(), Error> {
    let mut pos = csv::Position::new();
    pos.set_byte(ckpt.byte_offset);
    pos.set_line(ckpt.line_no);
    pos.set_record(ckpt.record_index);
    reader.seek(pos).map_err(|e| Error::Csv(e.to_string()))
}

fn capture_pos(ckpt: &mut Checkpoint, reader: &Reader<File>) {
    let pos = reader.position();
    ckpt.byte_offset = pos.byte();
    ckpt.line_no = pos.line();
    ckpt.record_index = pos.record();
}

fn now(clock: &dyn WallClock) -> String {
    format_rfc3339(clock.now_ms())
}

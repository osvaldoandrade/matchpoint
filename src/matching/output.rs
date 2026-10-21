//! Absolute primary/export destinations and atomic, no-overwrite publication.
use super::bad;
use crate::error::Error;
use std::path::{Path, PathBuf};

pub(super) fn target(id: &str, requested: Option<&Path>) -> Result<PathBuf, Error> {
    let id = uuid::Uuid::parse_str(id).map_err(|_| bad("Invalid matching job ID."))?;
    let requested = requested
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut target = std::path::absolute(requested)?;
    if target.is_dir() {
        target.push(format!("result-{id}.txt"));
    }
    Ok(target)
}

pub(super) fn copy(source: &Path, target: &Path) -> Result<(), Error> {
    if source == target && source.is_file() {
        return Ok(());
    }
    if target.exists() {
        return Err(bad("Output already exists; choose a new path."));
    }
    let tmp = target.with_extension(format!("{}.partial", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut output = crate::disk::create_private_file(&tmp)?;
        crate::error::restrict_file(&tmp)?;
        std::io::copy(&mut std::fs::File::open(source)?, &mut output)?;
        output.sync_all()?;
        drop(output);
        super::publish_new(&tmp, target)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(tmp);
    }
    result
}

pub(super) fn publish(source: &Path, target: &Path) -> Result<(), Error> {
    match super::publish_new(source, target) {
        Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::CrossesDevices => {
            copy(source, target)?;
            std::fs::remove_file(source)?;
            Ok(())
        }
        result => result,
    }
}

use std::path::Path;

use nix::fcntl::{Flock, FlockArg};
use nix::sys::statfs;

use crate::error::Error;

/// Holds an exclusive non-blocking lock for the lifetime of the value.
/// The file descriptor stays open, which is what keeps the lock.
pub struct JobLock {
    _lock: Flock<std::fs::File>,
}

pub fn acquire(path: &Path) -> Result<JobLock, Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?;
    match Flock::lock(file, FlockArg::LockExclusiveNonblock) {
        Ok(lock) => Ok(JobLock { _lock: lock }),
        Err((_, err))
            if err == nix::errno::Errno::EWOULDBLOCK || err == nix::errno::Errno::EAGAIN =>
        {
            Err(Error::Locked)
        }
        Err((_, err)) => Err(Error::Msg(format!("lock failed: {err}"))),
    }
}

pub fn ensure_local_fs(path: &Path) -> Result<(), Error> {
    let probe = if path.exists() {
        path.to_path_buf()
    } else {
        path.parent().unwrap_or(path).to_path_buf()
    };
    if !probe.exists() {
        if let Some(parent) = probe.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::create_dir_all(&probe)?;
    }
    let stat = statfs::statfs(&probe).map_err(|e| Error::Msg(format!("statfs: {e}")))?;
    #[cfg(not(target_os = "linux"))]
    let kind = stat.filesystem_type_name().to_string();
    #[cfg(target_os = "linux")]
    let kind = linux_filesystem_name(stat.filesystem_type()).to_string();
    let blocked = ["nfs", "smbfs", "cifs", "afpfs", "webdav", "fuse"];
    if blocked.iter().any(|name| kind.eq_ignore_ascii_case(name)) {
        return Err(Error::NetworkFs { kind });
    }
    Ok(())
}

pub fn free_bytes(path: &Path) -> Result<u64, Error> {
    let probe = if path.exists() {
        path.to_path_buf()
    } else {
        path.parent().unwrap_or(path).to_path_buf()
    };
    let stat = statfs::statfs(&probe).map_err(|e| Error::Msg(format!("statfs: {e}")))?;
    Ok(stat.blocks_available() * stat.block_size() as u64)
}

#[cfg(target_os = "linux")]
fn linux_filesystem_name(kind: statfs::FsType) -> &'static str {
    match kind {
        statfs::NFS_SUPER_MAGIC => "nfs",
        statfs::SMB_SUPER_MAGIC => "smbfs",
        statfs::FUSE_SUPER_MAGIC => "fuse",
        // Linux UAPI magic.h names these CIFS_SUPER_MAGIC and SMB2_SUPER_MAGIC.
        statfs::FsType(0xff534d42 | 0xfe534d42) => "cifs",
        _ => "local",
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn linux_network_filesystems_keep_the_same_rejection_policy() {
        for (kind, name) in [
            (statfs::NFS_SUPER_MAGIC, "nfs"),
            (statfs::SMB_SUPER_MAGIC, "smbfs"),
            (statfs::FUSE_SUPER_MAGIC, "fuse"),
            (statfs::FsType(0xff534d42), "cifs"),
            (statfs::FsType(0xfe534d42), "cifs"),
            (statfs::EXT4_SUPER_MAGIC, "local"),
        ] {
            assert_eq!(linux_filesystem_name(kind), name);
        }
    }
}

pub fn publish_file(tmp: &Path, final_path: &Path) -> Result<(), Error> {
    {
        let file = std::fs::OpenOptions::new().write(true).open(tmp)?;
        file.sync_all()?;
    }
    std::fs::rename(tmp, final_path)?;
    if let Some(parent) = final_path.parent() {
        let dir = std::fs::File::open(parent)?;
        dir.sync_all()?;
    }
    Ok(())
}

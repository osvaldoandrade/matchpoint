use std::path::Path;

use nix::fcntl::{Flock, FlockArg};
use nix::sys::statfs;

use crate::error::Error;

/// create_private_file prevents a permissive interval before any later chmod.
pub(crate) fn create_private_file(path: &Path) -> std::io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

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

#[cfg(all(test, unix))]
mod private_file_tests {
    use super::create_private_file;
    use std::{io::ErrorKind, os::unix::fs::PermissionsExt, process::Command};

    fn assert_creation_mode(mask: u16) {
        let root = tempfile::tempdir().unwrap();
        let previous = nix::sys::stat::umask(nix::sys::stat::Mode::from_bits_truncate(mask as _));
        let file = create_private_file(&root.path().join("private.txt")).unwrap();
        assert_eq!(
            file.metadata().unwrap().permissions().mode() & 0o777,
            0o600 & !u32::from(mask)
        );
        nix::sys::stat::umask(previous);
    }

    #[test]
    fn files_are_private_at_creation() {
        const MASK: &str = "MATCHPOINT_TEST_CREATION_UMASK";
        if let Ok(mask) = std::env::var(MASK) {
            assert_creation_mode(mask.parse().unwrap());
            return;
        }
        // A subprocess isolates the process-wide umask from concurrent tests.
        for mask in [0o000, 0o002, 0o022, 0o077, 0o777] {
            let output = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "disk::private_file_tests::files_are_private_at_creation",
                ])
                .env(MASK, mask.to_string())
                .output()
                .unwrap();
            assert!(output.status.success(), "umask {mask:o}: {output:?}");
            assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed;"));
        }
    }

    #[test]
    fn existing_files_are_preserved() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("existing.txt");
        std::fs::write(&path, "preserve").unwrap();
        assert_eq!(
            create_private_file(&path).unwrap_err().kind(),
            ErrorKind::AlreadyExists
        );
        assert_eq!(std::fs::read_to_string(path).unwrap(), "preserve");
    }

    #[test]
    fn symbolic_links_are_preserved_without_following_them() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("target.txt");
        let link = root.path().join("link.txt");
        std::fs::write(&target, "preserve").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert_eq!(
            create_private_file(&link).unwrap_err().kind(),
            ErrorKind::AlreadyExists
        );
        assert!(link.is_symlink());
        assert_eq!(std::fs::read_to_string(target).unwrap(), "preserve");
    }
}

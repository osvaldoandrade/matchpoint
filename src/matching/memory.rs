//! Anonymous pages: lock before use, zeroize while locked, then unmap on drop.
use super::bad;
use crate::error::Error;
use memmap2::MmapMut;
use std::{
    io::Read,
    ops::{Deref, DerefMut},
};
use zeroize::Zeroize;

pub(crate) struct LockedBytes(MmapMut);
impl LockedBytes {
    pub fn new(size: usize) -> Result<Self, Error> {
        if size == 0 {
            return Err(bad("DAT allocation must not be empty."));
        }
        let map =
            MmapMut::map_anon(size).map_err(|_| bad("Cannot allocate anonymous DAT memory."))?;
        #[cfg(unix)]
        map.lock().map_err(|_| bad("Cannot lock DAT pages in RAM; increase the OS locked-memory limit. No disk/swap fallback is allowed."))?;
        #[cfg(not(unix))]
        return Err(bad(
            "Strict resident DAT memory requires a supported Unix platform.",
        ));
        Ok(Self(map))
    }
}
impl Deref for LockedBytes {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.0
    }
}
impl DerefMut for LockedBytes {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}
impl AsRef<[u8]> for LockedBytes {
    fn as_ref(&self) -> &[u8] {
        self
    }
}
impl<'a> IntoIterator for &'a LockedBytes {
    type Item = &'a u8;
    type IntoIter = std::slice::Iter<'a, u8>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl Zeroize for LockedBytes {
    fn zeroize(&mut self) {
        self.0[..].zeroize();
    }
}
impl Drop for LockedBytes {
    fn drop(&mut self) {
        self.zeroize(); /* munmap releases the lock after wiping. */
    }
}

const CHUNK: usize = 64 * 1024;
pub(crate) struct Download {
    chunks: Vec<LockedBytes>,
    length: usize,
    position: usize,
    limit: usize,
}
impl Download {
    pub fn len(&self) -> usize {
        self.length
    }
    /// Range parts are MiB-aligned; transfer locked pages without copying them.
    pub fn join(&mut self, mut part: Self) -> Result<(), Error> {
        if self.position != 0
            || part.position != 0
            || !self.length.is_multiple_of(CHUNK)
            || part.length > self.limit.saturating_sub(self.length)
        {
            return Err(bad("Invalid DAT part assembly."));
        }
        self.chunks
            .try_reserve(part.chunks.len())
            .map_err(|_| bad("Insufficient memory for DAT parts."))?;
        self.length += part.length;
        self.chunks.append(&mut part.chunks);
        Ok(())
    }
    pub fn new(limit: usize) -> Self {
        Self {
            chunks: Vec::new(),
            length: 0,
            position: 0,
            limit,
        }
    }
    pub fn into_locked(mut self) -> Result<LockedBytes, Error> {
        let mut bytes = LockedBytes::new(self.length - self.position)?;
        self.read_exact(&mut bytes)?;
        Ok(bytes)
    }
    pub fn append(&mut self, mut bytes: &[u8]) -> Result<(), Error> {
        if bytes.len() > self.limit.saturating_sub(self.length) {
            return Err(bad("DAT download exceeds compressed limit."));
        }
        while !bytes.is_empty() {
            let at = self.length % CHUNK;
            if at == 0 {
                self.chunks
                    .try_reserve(1)
                    .map_err(|_| bad("Insufficient memory for DAT download."))?;
                self.chunks.push(LockedBytes::new(CHUNK)?);
            }
            let n = bytes.len().min(CHUNK - at);
            self.chunks.last_mut().unwrap()[at..at + n].copy_from_slice(&bytes[..n]);
            self.length += n;
            bytes = &bytes[n..];
        }
        Ok(())
    }
}
impl Read for Download {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        let n = output.len().min(self.length - self.position);
        let mut done = 0;
        while done < n {
            let at = self.position % CHUNK;
            let count = (n - done).min(CHUNK - at);
            output[done..done + count]
                .copy_from_slice(&self.chunks[self.position / CHUNK][at..at + count]);
            self.position += count;
            done += count;
        }
        Ok(n)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn locked_buffers_wipe_and_bounded_download_streams_across_chunks() {
        assert!(LockedBytes::new(0).is_err());
        let mut buffer = LockedBytes::new(4096).unwrap();
        buffer.fill(173);
        buffer.zeroize();
        assert!(buffer.iter().all(|b| *b == 0));
        let mut wire = Download::new(CHUNK * 2);
        let bytes = vec![91; CHUNK + 17];
        wire.append(&bytes).unwrap();
        assert!(wire.append(&vec![0; CHUNK]).is_err());
        let mut out = Vec::new();
        wire.read_to_end(&mut out).unwrap();
        assert_eq!(out, bytes);
    }
    #[test]
    fn range_assembly_refuses_unaligned_consumed_or_oversized_parts() {
        let mut wire = Download::new(CHUNK * 2);
        wire.append(&[1]).unwrap();
        assert!(wire.join(Download::new(CHUNK)).is_err());
        let mut wire = Download::new(CHUNK);
        let mut part = Download::new(CHUNK * 2);
        part.append(&vec![0; CHUNK + 1]).unwrap();
        assert!(wire.join(part).is_err());
        let mut part = Download::new(CHUNK);
        part.append(&[1, 2]).unwrap();
        part.read_exact(&mut [0]).unwrap();
        assert!(wire.join(part).is_err());
    }
}

//! PGMBLOOM v1 codec. Limits are checked before bit-array allocation.
use super::{bad, dataset::Dataset, hex, Rule, Transform};
use crate::error::Error;
use flate2::{bufread::GzDecoder, write::GzEncoder, Compression};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{BufReader, Cursor, Read, Write},
    path::Path,
};

pub const HEADER_BYTES: usize = 128;
pub const TARGET_FPP: f64 = 0.001;
const DOMAIN: &[u8] = b"PGMBLOOM/v1";
mod provider;

#[derive(Clone, Debug, Serialize)]
pub struct Info {
    pub wire_format: &'static str,
    pub version: u16,
    pub bits: u64,
    pub hashes: u32,
    pub capacity: u64,
    pub inserted: u64,
    pub target_fpp: f64,
    pub transform: Transform,
    pub trim: bool,
    pub bytes: u64,
    pub sha256: String,
}
pub struct Bloom {
    info: Info,
    seed: [u8; 16],
    bits: super::memory::LockedBytes,
    provider: bool,
}
impl Bloom {
    pub fn new(capacity: u64, rule: &Rule) -> Result<Self, Error> {
        if capacity == 0 {
            return Err(bad("Cannot build an empty Bloom filter."));
        }
        rule.validate()?;
        let count =
            (-(capacity as f64) * (TARGET_FPP * 0.8).ln() / std::f64::consts::LN_2.powi(2)).ceil();
        if !count.is_finite() || count > 512.0 * 1024.0 * 1024.0 * 8.0 {
            return Err(bad("Bloom builder exceeds the 512 MiB bit-array limit."));
        }
        // Tiny filters otherwise amplify correlations between double-hash probes.
        let bits = (count as u64).max(4096).div_ceil(64) * 64;
        let hashes = ((bits as f64 / capacity as f64) * std::f64::consts::LN_2)
            .round()
            .clamp(1.0, 32.0) as u32;
        let info = Info {
            wire_format: "pgmbloom-native-v1",
            version: 1,
            bits,
            hashes,
            capacity,
            inserted: 0,
            target_fpp: TARGET_FPP,
            transform: rule.transform,
            trim: rule.trim,
            bytes: bits / 8,
            sha256: String::new(),
        };
        Ok(Self {
            bits: allocate(info.bytes)?,
            info,
            seed: *uuid::Uuid::new_v4().as_bytes(),
            provider: false,
        })
    }
    pub fn info(&self) -> &Info {
        &self.info
    }
    pub fn compatible(&self, rule: &Rule) -> Result<(), Error> {
        if self.provider {
            // This header declares digest probes, not an input extraction/normalization rule.
            return rule.validate();
        }
        if self.info.transform != rule.transform || self.info.trim != rule.trim {
            return Err(bad("Dataset transformation differs from the DAT; use the producer's transform and trim rule."));
        }
        Ok(())
    }
    pub fn insert(&mut self, key: &str) -> Result<(), Error> {
        if self.provider {
            return Err(bad(
                "Provider filters are immutable; preserve the original DAT.",
            ));
        }
        if self.info.inserted >= self.info.capacity {
            return Err(bad("Bloom capacity exceeded; rebuild a larger filter."));
        }
        let probes = self.probes(key);
        for bit in probes {
            self.bits[(bit / 8) as usize] |= 1 << (bit % 8);
        }
        self.info.inserted += 1;
        Ok(())
    }
    /// Provider input defaults to an already encoded SHA-256 hex digest.
    /// Complexity: O(key bytes + probes) time, O(1) provider working space.
    pub fn contains(&self, key: &str) -> Result<bool, Error> {
        if self.provider {
            return provider::query(&self.info, &self.bits, key, Transform::None);
        }
        Ok(self
            .probes(key)
            .all(|bit| self.bits[(bit / 8) as usize] & (1 << (bit % 8)) != 0))
    }
    /// Apply the job's explicit rule; malformed provider digests are errors, never negatives.
    /// Complexity: O(key bytes + probes) time, O(1) provider working space.
    pub fn contains_with_rule(&self, key: &str, rule: &Rule) -> Result<bool, Error> {
        if self.provider {
            return provider::query(&self.info, &self.bits, rule.lookup(key), rule.transform);
        }
        self.compatible(rule)?;
        self.contains(key)
    }
    fn probes(&self, key: &str) -> impl Iterator<Item = u64> {
        let key = if self.info.trim { key.trim() } else { key };
        let bytes = self.info.transform.bytes(key);
        let mut hash = Sha256::new();
        hash.update(DOMAIN);
        hash.update(self.seed);
        hash.update(bytes.as_ref());
        let digest = hash.finalize();
        let h1 = u64::from_le_bytes(digest[..8].try_into().unwrap()) as u128;
        let h2 = (u64::from_le_bytes(digest[8..16].try_into().unwrap()) | 1) as u128;
        let m = self.info.bits as u128;
        (0..self.info.hashes).map(move |i| ((h1 + i as u128 * h2) % m) as u64)
    }
    fn header(&self) -> [u8; HEADER_BYTES] {
        let mut h = [0; HEADER_BYTES];
        h[..8].copy_from_slice(b"PGMBLOOM");
        h[8..10].copy_from_slice(&1u16.to_le_bytes());
        h[10..12].copy_from_slice(&128u16.to_le_bytes());
        h[12..16].copy_from_slice(&1u32.to_le_bytes());
        h[16..24].copy_from_slice(&self.info.bits.to_le_bytes());
        h[24..28].copy_from_slice(&self.info.hashes.to_le_bytes());
        h[28..32].copy_from_slice(&self.info.transform.code().to_le_bytes());
        h[32..40].copy_from_slice(&self.info.capacity.to_le_bytes());
        h[40..48].copy_from_slice(&self.info.inserted.to_le_bytes());
        h[48..56].copy_from_slice(&self.info.target_fpp.to_le_bytes());
        h[56..72].copy_from_slice(&self.seed);
        h[72..80].copy_from_slice(&self.info.bytes.to_le_bytes());
        h[112..116].copy_from_slice(&u32::from(self.info.trim).to_le_bytes());
        let checksum = checksum(&h, &self.bits);
        h[80..112].copy_from_slice(&checksum);
        h
    }
    pub fn write(&self, path: &Path) -> Result<(), Error> {
        if self.provider {
            return Err(bad("Provider filters cannot be written with the native encoder; preserve the original DAT."));
        }
        if path.exists() {
            return Err(bad("DAT output already exists; choose a new path."));
        }
        let tmp = path.with_extension(format!("{}.partial", uuid::Uuid::new_v4()));
        let result = (|| {
            let file = File::create(&tmp)?;
            crate::error::restrict_file(&tmp)?;
            let mut gzip = GzEncoder::new(file, Compression::fast());
            gzip.write_all(&self.header())?;
            gzip.write_all(&self.bits)?;
            gzip.finish()?.sync_all()?;
            super::publish_new(&tmp, path)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(tmp);
        }
        result
    }
    pub fn read(path: &Path, max_bytes: u64) -> Result<Self, Error> {
        Self::decode(File::open(path)?, max_bytes)
    }
    pub fn decode(mut input: impl Read, max_bytes: u64) -> Result<Self, Error> {
        let mut gz = [0; 10];
        input
            .read_exact(&mut gz)
            .map_err(|_| bad("Truncated GZip header."))?;
        if &gz[..8] == b"PGMBLOOM" {
            let mut reader = Cursor::new(gz).chain(input);
            let bloom = Self::decode_payload(&mut reader, max_bytes)?;
            let mut extra = [0];
            if reader.read(&mut extra)? != 0 {
                return Err(bad("Unexpected trailing DAT bytes."));
            }
            return Ok(bloom);
        }
        if gz[..3] != [0x1f, 0x8b, 8] || gz[3] != 0 {
            return Err(bad(
                "DAT requires a plain GZip envelope without optional headers.",
            ));
        }
        let mut decoder = GzDecoder::new(BufReader::new(Cursor::new(gz).chain(input)));
        let bloom = Self::decode_payload(&mut decoder, max_bytes)?;
        let mut extra = [0];
        if decoder
            .read(&mut extra)
            .map_err(|_| bad("GZip CRC/trailer verification failed."))?
            != 0
        {
            return Err(bad("Unexpected bytes after DAT bit array."));
        }
        if decoder.into_inner().read(&mut extra)? != 0 {
            return Err(bad("Unexpected trailing GZip members or bytes."));
        }
        Ok(bloom)
    }
    fn decode_payload(input: &mut impl Read, max_bytes: u64) -> Result<Self, Error> {
        let mut prefix = [0; 12];
        input
            .read_exact(&mut prefix)
            .map_err(|_| bad("Invalid or truncated DAT header."))?;
        if &prefix[..8] != b"PGMBLOOM" {
            return Err(bad("Unsupported DAT magic."));
        }
        if prefix[8..12] == 1u32.to_be_bytes() {
            return provider::read(input, prefix, max_bytes);
        }
        let mut h = [0; HEADER_BYTES];
        h[..12].copy_from_slice(&prefix);
        input
            .read_exact(&mut h[12..])
            .map_err(|_| bad("Invalid or truncated DAT header."))?;
        let (info, seed) = parse_header(&h, max_bytes)?;
        let mut bits = allocate(info.bytes)?;
        input
            .read_exact(&mut bits)
            .map_err(|_| bad("Invalid or truncated DAT payload."))?;
        if checksum(&h, &bits).as_slice() != &h[80..112] {
            return Err(bad("DAT SHA-256 mismatch."));
        }
        Ok(Self {
            info,
            seed,
            bits,
            provider: false,
        })
    }
}
fn allocate(bytes: u64) -> Result<super::memory::LockedBytes, Error> {
    let size = usize::try_from(bytes).map_err(|_| bad("DAT cannot fit this platform."))?;
    super::memory::LockedBytes::new(size)
}
fn checksum(h: &[u8; 128], bits: &[u8]) -> Vec<u8> {
    let mut sha = Sha256::new();
    sha.update(&h[..80]);
    sha.update(&h[112..]);
    sha.update(bits);
    sha.finalize().to_vec()
}
fn parse_header(h: &[u8; 128], max_bytes: u64) -> Result<(Info, [u8; 16]), Error> {
    let u32at = |i| u32::from_le_bytes(h[i..i + 4].try_into().unwrap());
    let u64at = |i| u64::from_le_bytes(h[i..i + 8].try_into().unwrap());
    if &h[..8] != b"PGMBLOOM"
        || h[8..10] != 1u16.to_le_bytes()
        || h[10..12] != 128u16.to_le_bytes()
        || u32at(12) != 1
        || u32at(112) > 1
        || h[116..].iter().any(|b| *b != 0)
    {
        return Err(bad("Unsupported DAT magic, version, algorithm or flags."));
    }
    let info = Info {
        wire_format: "pgmbloom-native-v1",
        version: 1,
        bits: u64at(16),
        hashes: u32at(24),
        transform: Transform::from_code(u32at(28))?,
        capacity: u64at(32),
        inserted: u64at(40),
        target_fpp: f64::from_le_bytes(h[48..56].try_into().unwrap()),
        trim: u32at(112) == 1,
        bytes: u64at(72),
        sha256: hex(&h[80..112]),
    };
    if info.bits == 0
        || !info.bits.is_multiple_of(64)
        || info.bytes != info.bits / 8
        || info.bytes > max_bytes
        || !(1..=32).contains(&info.hashes)
        || info.capacity == 0
        || info.inserted > info.capacity
        || !info.target_fpp.is_finite()
        || !(0.0..=TARGET_FPP).contains(&info.target_fpp)
        || info.target_fpp == 0.0
    {
        return Err(bad(
            "Invalid DAT size/capacity/probes/FPP or memory limit exceeded.",
        ));
    }
    let estimated = (1.0 - (-(info.hashes as f64) * info.capacity as f64 / info.bits as f64).exp())
        .powi(info.hashes as i32);
    if estimated > info.target_fpp {
        return Err(bad(
            "DAT parameters exceed the declared false-positive target.",
        ));
    }
    Ok((info, h[56..72].try_into().unwrap()))
}
pub fn build_dataset(input: &Path, output: &Path, rule: &Rule) -> Result<Info, Error> {
    let before = crate::prepare::sha256_file(input)?;
    let mut reader = Dataset::open(input, rule)?;
    let mut count = 0u64;
    while reader.next_key()?.is_some() {
        count = count
            .checked_add(1)
            .ok_or_else(|| bad("Dataset row count overflow."))?;
    }
    let mut bloom = Bloom::new(count, rule)?;
    let mut reader = Dataset::open(input, rule)?;
    while let Some(key) = reader.next_key()? {
        if rule.lookup(&key).is_empty() {
            return Err(bad("Dataset contains an empty transformed key."));
        }
        bloom.insert(&key)?;
    }
    if bloom.info.inserted != count || crate::prepare::sha256_file(input)? != before {
        return Err(bad("Dataset changed while building the DAT."));
    }
    bloom.write(output)?;
    Ok(Bloom::read(output, 512 * 1024 * 1024)?.info)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn encode(payload: &[u8]) -> Vec<u8> {
        let mut gz = GzEncoder::new(Vec::new(), Compression::fast());
        gz.write_all(payload).unwrap();
        gz.finish().unwrap()
    }
    fn sample() -> Bloom {
        let mut b = Bloom::new(20, &Rule::default()).unwrap();
        b.seed = [7; 16];
        for i in 0..20 {
            b.insert(&format!("key-{i}")).unwrap();
        }
        b
    }
    #[test]
    fn independent_python_vectors_are_readable_for_every_transform() {
        for (tid, transform) in [Transform::None, Transform::Sha256, Transform::Sha256Raw]
            .into_iter()
            .enumerate()
        {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("tests/fixtures/bloom/python-v1-{tid}.dat"));
            let b = Bloom::read(&path, 512).unwrap();
            assert_eq!(b.info.transform, transform);
            for key in ["00012345678", "partner-abc", "a:b", "ç"] {
                assert!(b.contains(key).unwrap());
            }
            assert!(!b.contains("not-in-independent-vector").unwrap());
        }
    }
    #[test]
    fn roundtrip_transforms_trim_capacity_and_rule_mismatch() {
        for transform in [Transform::None, Transform::Sha256, Transform::Sha256Raw] {
            for trim in [false, true] {
                let rule = Rule {
                    transform,
                    trim,
                    ..Rule::default()
                };
                let mut b = Bloom::new(1, &rule).unwrap();
                b.insert(" 0001 ").unwrap();
                assert!(b.insert("excess").is_err());
                let mut decoded = b.header().to_vec();
                decoded.extend(&b.bits);
                let loaded = Bloom::decode(encode(&decoded).as_slice(), 512).unwrap();
                assert!(loaded.contains(" 0001 ").unwrap());
                assert_eq!(loaded.contains("0001").unwrap(), trim);
                assert!(loaded.compatible(&rule).is_ok());
                assert!(loaded
                    .compatible(&Rule {
                        trim: !trim,
                        ..rule
                    })
                    .is_err());
            }
        }
        assert!(Bloom::new(0, &Rule::default()).is_err());
        assert!(Bloom::new(u64::MAX, &Rule::default()).is_err());
    }
    #[test]
    fn corrupt_headers_lengths_checksum_crc_and_trailers_fail_closed() {
        let b = sample();
        let mut payload = b.header().to_vec();
        payload.extend(&b.bits);
        let good = encode(&payload);
        for offset in [0, 8, 10, 12, 16, 24, 28, 32, 40, 48, 72, 80, 112, 116, 128] {
            let mut bytes = payload.clone();
            bytes[offset] ^= 0xff;
            assert!(
                Bloom::decode(encode(&bytes).as_slice(), 512).is_err(),
                "offset {offset}"
            );
        }
        for cut in [0, 1, 9, 10, 20, good.len() - 1, good.len() - 8] {
            assert!(Bloom::decode(&good[..cut], 512).is_err());
        }
        let mut corrupt = good.clone();
        let crc = corrupt.len() - 8;
        corrupt[crc] ^= 1;
        assert!(Bloom::decode(corrupt.as_slice(), 512).is_err());
        let mut extra = good.clone();
        extra.extend(&good);
        assert!(Bloom::decode(extra.as_slice(), 512).is_err());
        payload.push(0);
        assert!(Bloom::decode(encode(&payload).as_slice(), 512).is_err());
        assert!(Bloom::decode(good.as_slice(), 511).is_err());
        let mut optional = good;
        optional[3] = 8;
        assert!(Bloom::decode(optional.as_slice(), 512).is_err());
    }
    #[test]
    fn seeded_no_false_negatives_and_empirical_false_positives() {
        for capacity in [1, 20, 10_000] {
            let mut b = Bloom::new(capacity, &Rule::default()).unwrap();
            b.seed = [42; 16];
            for i in 0..capacity {
                b.insert(&format!("included-{i:011}")).unwrap();
            }
            for i in 0..capacity {
                assert!(b.contains(&format!("included-{i:011}")).unwrap());
            }
            let positives = (0..200_000)
                .filter(|i| b.contains(&format!("excluded-{i:011}")).unwrap())
                .count();
            assert!(
                positives <= 200,
                "capacity={capacity}, false positives={positives}/200000"
            );
        }
    }
    #[test]
    fn bounded_adversarial_parser_smoke_never_panics() {
        let b = sample();
        let mut payload = b.header().to_vec();
        payload.extend(&b.bits);
        let mut seed = 1u64;
        for i in 0..3000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let mut bytes = payload.clone();
            let at = seed as usize % bytes.len();
            bytes[at] ^= (i % 255 + 1) as u8;
            let compressed = encode(&bytes);
            let _ = Bloom::decode(compressed.as_slice(), 1024);
            let random = seed.to_le_bytes();
            let _ = Bloom::decode(random.as_slice(), 1024);
        }
    }
}

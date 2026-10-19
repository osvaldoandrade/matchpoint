//! Provider PGMBLOOM: big-endian 64-byte header, digest probes, SHA-256 trailer.
use super::{allocate, bad, hex, Bloom, Digest, Error, Info, Read, Sha256, Transform, TARGET_FPP};

pub(super) fn read(
    input: &mut impl Read,
    prefix: [u8; 12],
    max_bytes: u64,
) -> Result<Bloom, Error> {
    let mut header = [0u8; 64];
    header[..12].copy_from_slice(&prefix);
    input
        .read_exact(&mut header[12..])
        .map_err(|_| bad("Truncated provider DAT header."))?;
    let u32at = |i| u32::from_be_bytes(header[i..i + 4].try_into().unwrap());
    let u64at = |i| u64::from_be_bytes(header[i..i + 8].try_into().unwrap());
    if u32at(8) != 1 || u32at(12) != 1 || u32at(20) != 0 || u64at(56) != 0 {
        return Err(bad(
            "Unsupported provider DAT version, algorithm or reserved fields.",
        ));
    }
    let bits = u64at(24);
    let hashes = u32at(16);
    let capacity = u64at(32);
    let inserted = u64at(40);
    let target_fpp = f64::from_bits(u64at(48));
    if bits == 0
        || !bits.is_multiple_of(8)
        || bits / 8 > max_bytes
        || !(1..=32).contains(&hashes)
        || capacity == 0
        || inserted > capacity
        || !target_fpp.is_finite()
        || target_fpp <= 0.0
        || target_fpp > TARGET_FPP
    {
        return Err(bad(
            "Invalid provider DAT size/capacity/probes/FPP or memory limit exceeded.",
        ));
    }
    let estimated =
        (1.0 - (-(hashes as f64) * capacity as f64 / bits as f64).exp()).powi(hashes as i32);
    if estimated > target_fpp * 1.01 || estimated > TARGET_FPP {
        return Err(bad(
            "Provider DAT parameters exceed the false-positive bound.",
        ));
    }
    let mut payload = allocate(bits / 8)?;
    input
        .read_exact(&mut payload)
        .map_err(|_| bad("Truncated provider DAT bit array."))?;
    let mut trailer = [0u8; 32];
    input
        .read_exact(&mut trailer)
        .map_err(|_| bad("Truncated provider DAT checksum."))?;
    let mut sha = Sha256::new();
    sha.update(header);
    sha.update(&payload);
    if sha.finalize().as_slice() != trailer {
        return Err(bad("Provider DAT SHA-256 mismatch."));
    }
    Ok(Bloom {
        info: Info {
            wire_format: "pgmbloom-provider-v1",
            version: 1,
            bits,
            hashes,
            capacity,
            inserted,
            target_fpp,
            transform: Transform::None,
            trim: false,
            bytes: bits / 8,
            sha256: hex(&trailer),
        },
        seed: [0; 16],
        bits: payload,
        provider: true,
    })
}

pub(super) fn query(
    info: &Info,
    bits: &[u8],
    key: &str,
    transform: Transform,
) -> Result<bool, Error> {
    if key.is_empty() {
        return Err(bad("Provider lookup key is empty after normalization."));
    }
    let digest: [u8; 32] = match transform {
        Transform::None => {
            if key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(bad("Provider DAT expects a 64-character SHA-256 hex key. Use None for existing hashes, or SHA-256 for raw identifiers."));
            }
            let mut digest = [0; 32];
            let digit = |b: u8| {
                if b.is_ascii_digit() {
                    b - b'0'
                } else {
                    b.to_ascii_lowercase() - b'a' + 10
                }
            };
            for (out, pair) in digest.iter_mut().zip(key.as_bytes().chunks_exact(2)) {
                *out = (digit(pair[0]) << 4) | digit(pair[1]);
            }
            digest
        }
        Transform::Sha256 | Transform::Sha256Raw => Sha256::digest(key.as_bytes()).into(),
    };
    let h1 = u64::from_be_bytes(digest[..8].try_into().unwrap());
    let h2 = u64::from_be_bytes(digest[8..16].try_into().unwrap()) | 1;
    Ok((0..info.hashes).all(|i| {
        let bit = h1.wrapping_add(u64::from(i).wrapping_mul(h2)) % info.bits;
        bits[(bit / 8) as usize] & (1 << (bit % 8)) != 0
    }))
}

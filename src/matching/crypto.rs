//! Authenticate an entire downloaded DAT before GZip/Bloom decoding.
use super::{
    bad,
    memory::{Download, LockedBytes},
    Service,
};
use crate::error::Error;
use aws_lc_rs::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use base64::{engine::general_purpose::STANDARD, Engine};
use sha2::{Digest, Sha256};
use std::{io::Read, sync::Arc};

const HEADER_BYTES: usize = 32;
const CIPHERTEXT_AT: usize = 44;
const ENVELOPE_OVERHEAD: u64 = 92;

struct Envelope {
    header: [u8; HEADER_BYTES],
    nonce: [u8; 12],
    trailer_at: usize,
}
impl Envelope {
    fn verify(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < ENVELOPE_OVERHEAD as usize {
            return Err(bad(
                "Truncated PGMENC01 envelope; expected header, nonce, GCM tag and SHA-256 trailer.",
            ));
        }
        let header: [u8; HEADER_BYTES] = bytes[..HEADER_BYTES].try_into().unwrap();
        if &header[..8] != b"PGMENC01"
            || u32::from_be_bytes(header[8..12].try_into().unwrap()) != 1
            || u32::from_be_bytes(header[12..16].try_into().unwrap()) != 1
            || header[24..32] != [0; 8]
        {
            return Err(bad(
                "Unsupported PGMENC01 magic, version, algorithm or reserved fields.",
            ));
        }
        let plaintext_bytes = u64::from_be_bytes(header[16..24].try_into().unwrap());
        if plaintext_bytes.checked_add(ENVELOPE_OVERHEAD) != Some(bytes.len() as u64) {
            return Err(bad(
                "PGMENC01 size mismatch; body length must equal declared plaintext length + 92.",
            ));
        }
        let trailer_at = bytes.len() - 32;
        if Sha256::digest(&bytes[..trailer_at]).as_slice() != &bytes[trailer_at..] {
            return Err(bad(
                "PGMENC01 SHA-256 checksum mismatch; no DAT was decrypted.",
            ));
        }
        Ok(Self {
            header,
            nonce: bytes[32..CIPHERTEXT_AT].try_into().unwrap(),
            trailer_at,
        })
    }
}

/// Session-only 256-bit key. No Debug/serialization/clone of raw key material.
pub struct DatKey(LockedBytes);
impl DatKey {
    /// Require canonical padded standard Base64 encoding exactly 32 bytes.
    pub fn from_base64(encoded: &str) -> Result<Self, Error> {
        let encoded = encoded.trim();
        if encoded.len() != 44 {
            return Err(bad("Enter a Base64 AES-256 key encoding exactly 32 bytes."));
        }
        let mut key = LockedBytes::new(32)?;
        if STANDARD.decode_slice(encoded, &mut key[..]).ok() != Some(32) {
            return Err(bad(
                "Invalid Base64 AES-256 key; expected exactly 32 bytes.",
            ));
        }
        Ok(Self(key))
    }
    pub(super) fn decrypt(&self, wire: Download) -> Result<Plaintext, Error> {
        let mut bytes = wire.into_locked()?;
        let envelope = Envelope::verify(&bytes)?;
        let key = LessSafeKey::new(
            UnboundKey::new(&AES_256_GCM, &self.0)
                .map_err(|_| bad("Cannot initialize AES-256-GCM decryption."))?,
        );
        // The source supplies the nonce. Decryption never generates/reuses encryption nonces.
        let length = key.open_in_place(Nonce::assume_unique_for_key(envelope.nonce), Aad::from(envelope.header), &mut bytes[CIPHERTEXT_AT..envelope.trailer_at])
            .map_err(|_| bad("DAT authentication failed. Check the AES key and encrypted source; no filter was loaded."))?.len();
        Ok(Plaintext {
            bytes,
            position: CIPHERTEXT_AT,
            end: CIPHERTEXT_AT + length,
        })
    }
}
pub(super) struct Plaintext {
    bytes: LockedBytes,
    position: usize,
    end: usize,
}
impl Read for Plaintext {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let n = out.len().min(self.end - self.position);
        out[..n].copy_from_slice(&self.bytes[self.position..self.position + n]);
        self.position += n;
        Ok(n)
    }
}
impl Service {
    /// Install a login key in RAM only, after joining work and authenticating the user.
    pub fn set_dat_key(&self, key: DatKey) -> Result<(), Error> {
        let profile = self.resident_identity(&self.settings()?)?;
        self.clear_filters()?;
        self.resident
            .lock()
            .map_err(|_| bad("Resident DAT registry unavailable."))?
            .dat_key = Some((profile, Arc::new(key)));
        Ok(())
    }
    /// Release session ownership; active joined decoder references finish before final wipe.
    pub fn clear_dat_key(&self) -> Result<(), Error> {
        self.resident
            .lock()
            .map_err(|_| bad("Resident DAT registry unavailable."))?
            .dat_key = None;
        Ok(())
    }
    pub(super) fn dat_key(&self) -> Result<Option<Arc<DatKey>>, Error> {
        let profile = self.resident_identity(&self.settings()?)?;
        let state = self
            .resident
            .lock()
            .map_err(|_| bad("Resident DAT registry unavailable."))?;
        match &state.dat_key {
            Some((owner, key)) if owner == &profile => Ok(Some(Arc::clone(key))),
            Some(_) => Err(bad(
                "The DAT key belongs to another source profile. Sign in again.",
            )),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key() -> DatKey {
        DatKey::from_base64(&STANDARD.encode([0x42; 32])).unwrap()
    }
    fn wire(bytes: &[u8]) -> Download {
        let mut wire = Download::new(1024 * 1024);
        wire.append(bytes).unwrap();
        wire
    }
    fn checksum(bytes: &mut [u8]) {
        let end = bytes.len() - 32;
        let digest = Sha256::digest(&bytes[..end]);
        bytes[end..].copy_from_slice(&digest);
    }
    fn seal(plain: &[u8], nonce: [u8; 12], aad: Option<&[u8]>) -> Vec<u8> {
        let mut header = [0; 32];
        header[..8].copy_from_slice(b"PGMENC01");
        header[8..12].copy_from_slice(&1u32.to_be_bytes());
        header[12..16].copy_from_slice(&1u32.to_be_bytes());
        header[16..24].copy_from_slice(&(plain.len() as u64).to_be_bytes());
        let key = LessSafeKey::new(UnboundKey::new(&AES_256_GCM, &[0x42; 32]).unwrap());
        let mut ciphertext = plain.to_vec();
        key.seal_in_place_append_tag(
            Nonce::assume_unique_for_key(nonce),
            Aad::from(aad.unwrap_or(&header)),
            &mut ciphertext,
        )
        .unwrap();
        let mut body = [header.as_slice(), &nonce, &ciphertext, &[0; 32]].concat();
        checksum(&mut body);
        body
    }
    #[test]
    fn key_parser_requires_canonical_base64_of_exactly_256_bits() {
        for encoded in [
            String::new(),
            "password".into(),
            "!".repeat(44),
            STANDARD.encode([0; 16]),
            STANDARD.encode([0; 24]),
            STANDARD.encode([0; 31]),
            STANDARD.encode([0; 33]),
            "A".repeat(43),
            "A".repeat(45),
        ] {
            let error = DatKey::from_base64(&encoded).err().unwrap().to_string();
            assert!(!error.contains("password"));
        }
        let encoded = STANDARD.encode([0x42; 32]);
        assert!(DatKey::from_base64(&format!(" {encoded} ")).is_ok());
    }
    #[test]
    fn independent_empty_envelope_authenticates_but_is_not_a_valid_bloom() {
        let bytes = include_bytes!("../../tests/fixtures/bloom/empty-gcm.enc");
        let mut plaintext = key().decrypt(wire(bytes)).unwrap();
        assert_eq!(plaintext.read(&mut [0; 16]).unwrap(), 0);
        assert!(super::super::format::Bloom::decode(plaintext, 1024).is_err());
    }
    #[test]
    fn independent_python_vectors_decrypt_before_raw_or_gzip_bloom_decode() {
        for (encrypted, expected) in [
            (
                include_bytes!("../../tests/fixtures/bloom/native-gcm.enc").as_slice(),
                include_bytes!("../../tests/fixtures/bloom/python-v1-0.dat").as_slice(),
            ),
            (
                include_bytes!("../../tests/fixtures/bloom/provider-raw-gcm.enc").as_slice(),
                include_bytes!("../../tests/fixtures/bloom/provider-v1.dat").as_slice(),
            ),
            (
                include_bytes!("../../tests/fixtures/bloom/provider-gzip-gcm.enc").as_slice(),
                include_bytes!("../../tests/fixtures/bloom/provider-v1.dat.gz").as_slice(),
            ),
        ] {
            let mut decoded = Vec::new();
            key()
                .decrypt(wire(encrypted))
                .unwrap()
                .read_to_end(&mut decoded)
                .unwrap();
            assert_eq!(decoded, expected);
            let bloom = super::super::format::Bloom::decode(
                key().decrypt(wire(encrypted)).unwrap(),
                1024 * 1024,
            )
            .unwrap();
            if bloom.info().wire_format == "pgmbloom-provider-v1" {
                let vectors: serde_json::Value = serde_json::from_str(include_str!(
                    "../../tests/fixtures/bloom/provider-v1-vectors.json"
                ))
                .unwrap();
                for entry in vectors["vectors"].as_array().unwrap() {
                    let expected = entry["expected"].as_bool().unwrap();
                    assert_eq!(
                        bloom.contains(entry["sha256"].as_str().unwrap()).unwrap(),
                        expected
                    );
                    assert_eq!(
                        bloom
                            .contains_with_rule(
                                entry["raw"].as_str().unwrap(),
                                &super::super::Rule {
                                    transform: super::super::Transform::Sha256,
                                    ..Default::default()
                                }
                            )
                            .unwrap(),
                        expected
                    );
                }
            }
        }
    }
    #[test]
    fn wrong_key_nonce_ciphertext_tag_truncation_and_plaintext_never_authenticate() {
        let bytes = include_bytes!("../../tests/fixtures/bloom/native-gcm.enc");
        let wrong = DatKey::from_base64(&STANDARD.encode([0x43; 32])).unwrap();
        assert!(wrong.decrypt(wire(bytes)).is_err());
        assert!(key()
            .decrypt(wire(include_bytes!(
                "../../tests/fixtures/bloom/python-v1-0.dat"
            )))
            .is_err());
        for n in 0..bytes.len() {
            assert!(key().decrypt(wire(&bytes[..n])).is_err());
        }
        for i in 0..bytes.len() {
            let mut corrupt = bytes.to_vec();
            corrupt[i] ^= 1;
            assert!(key().decrypt(wire(&corrupt)).is_err());
        }
    }
    #[test]
    fn large_plaintext_crosses_locked_transfer_chunks_and_aad_is_not_ignored() {
        for (i, aad) in [
            None,
            Some(b"".as_slice()),
            Some(b"unexpected aad".as_slice()),
        ]
        .into_iter()
        .enumerate()
        {
            let body = seal(&vec![0x5a; 150000], [0x27 + i as u8; 12], aad);
            if aad.is_none() {
                let mut plain = key().decrypt(wire(&body)).unwrap();
                let mut result = Vec::new();
                plain.read_to_end(&mut result).unwrap();
                assert_eq!(result, vec![0x5a; 150000]);
            } else {
                assert!(key().decrypt(wire(&body)).is_err());
            }
        }
    }
    #[test]
    fn envelope_rejects_bad_be_header_reserved_fields_and_size_overflow() {
        let valid = include_bytes!("../../tests/fixtures/bloom/native-gcm.enc");
        for (offset, value) in [
            (0, b"WRONGENC".as_slice()),
            (8, 1u32.to_le_bytes().as_slice()),
            (8, 2u32.to_be_bytes().as_slice()),
            (12, 0u32.to_be_bytes().as_slice()),
            (12, 2u32.to_be_bytes().as_slice()),
            (24, 1u64.to_be_bytes().as_slice()),
        ] {
            let mut invalid = valid.to_vec();
            invalid[offset..offset + value.len()].copy_from_slice(value);
            checksum(&mut invalid);
            let error = key().decrypt(wire(&invalid)).err().unwrap().to_string();
            assert!(error.contains("Unsupported PGMENC01"));
        }
        let length = (valid.len() - 92) as u64;
        for declared in [
            0,
            1,
            length - 1,
            length + 1,
            u64::MAX,
            u64::MAX - 91,
            length.swap_bytes(),
        ] {
            let mut invalid = valid.to_vec();
            invalid[16..24].copy_from_slice(&declared.to_be_bytes());
            checksum(&mut invalid);
            let error = key().decrypt(wire(&invalid)).err().unwrap().to_string();
            assert!(error.contains("size mismatch"));
        }
        let mut extra = valid.to_vec();
        extra.push(0);
        assert!(key().decrypt(wire(&extra)).is_err());
    }
    #[test]
    fn checksum_precedes_authentication_and_recomputed_sha_cannot_bypass_gcm() {
        let valid = include_bytes!("../../tests/fixtures/bloom/native-gcm.enc");
        for at in [32, 43, 44, valid.len() - 33, valid.len() - 1] {
            let mut corrupt = valid.to_vec();
            corrupt[at] ^= 1;
            assert!(key()
                .decrypt(wire(&corrupt))
                .err()
                .unwrap()
                .to_string()
                .contains("checksum mismatch"));
            if at < valid.len() - 32 {
                checksum(&mut corrupt);
                assert!(key()
                    .decrypt(wire(&corrupt))
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("authentication failed"));
            }
        }
        let empty_aad = seal(b"test plaintext", [0x67; 12], Some(b""));
        assert!(key()
            .decrypt(wire(&empty_aad))
            .err()
            .unwrap()
            .to_string()
            .contains("authentication failed"));
        let obsolete = &empty_aad[32..empty_aad.len() - 32];
        assert!(key().decrypt(wire(obsolete)).is_err());
    }
    #[test]
    fn bounded_malformed_envelope_harness_never_panics() {
        let mut seed = 17u64;
        for n in 0..1024 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let mut candidate = vec![(seed >> 32) as u8; n];
            if n >= 92 {
                candidate[..8].copy_from_slice(b"PGMENC01");
                candidate[8..12].copy_from_slice(&1u32.to_be_bytes());
                candidate[12..16].copy_from_slice(&1u32.to_be_bytes());
                candidate[16..24].copy_from_slice(&((n - 92) as u64).to_be_bytes());
                candidate[24..32].fill(0);
                checksum(&mut candidate);
            }
            assert!(key().decrypt(wire(&candidate)).is_err());
        }
    }
    #[test]
    fn session_key_is_shared_profile_bound_and_released_by_sign_out() {
        let root = tempfile::tempdir().unwrap();
        let service = Service::open(
            root.path().into(),
            Arc::new(crate::secrets::MemorySecrets::new()),
        )
        .unwrap();
        service.set_dat_key(key()).unwrap();
        let resident = service.dat_key().unwrap().unwrap();
        let weak = Arc::downgrade(&resident);
        drop(resident);
        assert!(service.clone().dat_key().unwrap().is_some());
        let mut cfg = service.settings().unwrap();
        cfg.url = "https://example.test/dat".into();
        service.save_settings(&cfg).unwrap();
        assert!(service.dat_key().is_err());
        assert!(!std::fs::read_to_string(service.root.join("settings.json"))
            .unwrap()
            .contains(&STANDARD.encode([0x42; 32])));
        service.forget_access().unwrap();
        assert!(weak.upgrade().is_none());
        assert!(service.dat_key().unwrap().is_none());
    }
}

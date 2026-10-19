"""Independent public synthetic PGMENC01 vectors; never use this test key live."""
import base64
import hashlib
import json
from pathlib import Path
import struct

from cryptography.hazmat.primitives.ciphers.aead import AESGCM

root = Path(__file__).resolve().parent
key = bytes([0x42]) * 32
cases = []
for i, (name, source) in enumerate([
    ("native-gcm", "python-v1-0.dat"),
    ("provider-raw-gcm", "provider-v1.dat"),
    ("provider-gzip-gcm", "provider-v1.dat.gz"),
    ("native-gcm-fresh-nonce", "python-v1-0.dat"),
    ("empty-gcm", None),
]):
    plaintext = (root / source).read_bytes() if source else b""
    header = struct.pack(">8sIIQ8s", b"PGMENC01", 1, 1, len(plaintext), bytes(8))
    nonce = bytes(range(i * 12, (i + 1) * 12))
    body = header + nonce + AESGCM(key).encrypt(nonce, plaintext, header)
    envelope = body + hashlib.sha256(body).digest()
    assert len(envelope) == len(plaintext) + 92
    (root / (name + ".enc")).write_bytes(envelope)
    cases.append({
        "file": name + ".enc", "plaintext": source,
        "nonce_hex": nonce.hex(), "bytes": len(envelope),
        "plaintext_bytes": len(plaintext), "aad_hex": header.hex(),
        "trailer_sha256": envelope[-32:].hex(),
        "whole_file_sha256": hashlib.sha256(envelope).hexdigest(),
    })
(root / "aes-gcm-vectors.json").write_text(json.dumps({
    "purpose": "Public synthetic test vectors; never use this key/nonces outside tests",
    "generator": "Python cryptography AESGCM + struct/hashlib",
    "key_base64": base64.b64encode(key).decode(),
    "layout": "PGMENC01 header[32] || nonce[12] || ciphertext || tag[16] || SHA256[32]",
    "aad": "exact 32-byte header", "cases": cases,
}, indent=2) + "\n")

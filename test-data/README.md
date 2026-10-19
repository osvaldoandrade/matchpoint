# Synthetic CPF hash fixture

`cpfs_sha256_1m.csv`: UTF-8/LF, one header `cpf_sha256`, exactly 1,000,000 records. Each record is the lowercase SHA-256 hex digest of a unique synthetic 11-digit, check-digit-valid CPF, without punctuation, salt or trailing newline. No real-person source is used.

Metadata records the deterministic generation, samples, size and file checksum. Independent validation recomputes every digest, checks the CPF check digits using a second formula, and verifies row count/shape/uniqueness.

For matching already-hashed input, choose key `cpf_sha256` and transformation `None`, with a DAT built for these hash strings and `transform=none`. Applying SHA-256 again hashes the digest text and produces different lookup keys. A DAT declaring a different transformation must use the corresponding compatible input/rule; the application rejects mismatched rules.

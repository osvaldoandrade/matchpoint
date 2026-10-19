Parter Get Parter

Match CSV or Parquet identifier columns against a GZip-compressed Bloom DAT.
Download the filter once; all matching is local and in memory.
Output: original_key: true / false, one line per row.
A true result is a possible match (target false-positive rate 0.1%).

Run pgm-data tui, then Enter to open Home.
Settings: DAT source, key rules, optional Keychain bearer, appearance.
New Job: dataset path, key column, optional transformation. Enter runs it.
Jobs: R run/resume, P pause, E export, K checksum.
Use string identifiers to preserve leading zeros. CSV and Parquet are streamed.
Parquet codecs: uncompressed, Snappy, GZip, LZ4, Zstd and Brotli.

CLI help: pgm-data --help
Source: pgm-data bloom configure --url https://example.com/partners.dat
Load: pgm-data bloom download
Match: pgm-data match --input partners.csv --key identifier --output results.txt

Data is kept in ~/Library/Application Support/pgm-data (PGM_DATA_HOME overrides).
Matching uses the matching/ subdirectory; historical jobs are preserved.
No identifiers are sent to the API. Do not treat partial files as complete output.
The uninstall script removes only installation-owned paths and preserves data.
This local build is not a signed/notarized production release.

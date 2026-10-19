# ADR 0005 — Dependências fixadas

- Status: accepted
- Date: 2026-10-13

## Context

O pedido lista clap, ratatui, crossterm, tokio, reqwest, serde, csv, rusqlite, tracing, Arrow/Parquet e Keychain. Rust estável verificado nesta máquina: 1.94.0 (2026-03-02).

## Decision

Versões exatas no `Cargo.toml`, lockfile versionado, toolchain `1.94.0`. TLS via rustls com verificador da plataforma, mínimo TLS 1.2, HTTP/2 negociado por ALPN, HTTP/1.1 permanece aceito. Parquet usa Arrow e Snappy, sem o conjunto default de compressores. SQLite vendored (`bundled`). Keychain via `security-framework` só no macOS.

Não entrou: crate de TOML, `dirs`, `keyring`, `wiremock`, `axum`. Configuração é JSON. O servidor de teste é HTTP/1.1 mínimo no próprio binário de teste.

## Alternatives

- reqwest 0.12. A versão estável atual é 0.13.5, com rustls por feature `rustls`.
- Arrow completo com todos os codecs. Rejeitado pelo tempo de compilação e pelo disco.

## Consequences

`cargo audit` é o gate de vulnerabilidade. Licenças esperadas: MIT, Apache-2.0 ou BSD. O lockfile é a fonte da árvore fechada.

## Rollback

Reverter `Cargo.toml` e `Cargo.lock` juntos.

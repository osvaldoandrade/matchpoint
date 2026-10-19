# ADR 0006 — Pacote macOS sem afirmar Gatekeeper

- Status: accepted
- Date: 2026-10-13

## Context

A máquina tem Developer ID Application e `notarytool`. Não há identidade Developer ID Installer (há 3rd Party Mac Developer Installer, que não assina PKG fora da App Store). Só o alvo `aarch64-apple-darwin` está instalado. Disco livre perto de 19 GiB no início do trabalho.

## Decision

Artefato nomeado `pgm-data-aarch64-apple-darwin`. Sem `lipo` automático: baixar o alvo x86_64 não faz parte de `make test`. PKG instala em `/usr/local/pgm-data`. `preinstall` recusa sobrescrever `/usr/local/bin/pgm-data` se não for o nosso symlink. Dados ficam fora do payload.

`make sign` assina só o binário, sem `--deep`. `make notarize` não roda sem `NOTARY_PROFILE`. Ausência de certificado preserva o artefato local e reporta pendência. Não se afirma aprovação do Gatekeeper.

## Consequences

Um DMG produzido aqui pode ser bloqueado pelo Gatekeeper. Isso é o estado honesto.

## Rollback

`make uninstall` remove o binário do prefixo do usuário. O script de pacote não apaga Application Support sem `--delete-data`.

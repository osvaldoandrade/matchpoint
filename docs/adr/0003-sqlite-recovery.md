# ADR 0003 — SQLite por job, commit em grupo

- Status: accepted
- Date: 2026-10-13

## Context

20 milhões de linhas exigem um índice de claim que não varra a tabela e uma escrita que não marque sucesso antes do disco confirmar.

## Decision

Um diretório por job em Application Support: `input.csv`, `job.sqlite`, `exports/`, `run.lock`. WAL e `synchronous=FULL`. Filesystem de rede (`nfs`, `smbfs`, `cifs`, `afpfs`, `webdav`, `fuse`) é recusado.

Uma thread (`pgm-sqlite`) é a única escritora. Commits de resposta agrupam 500 linhas ou 250 ms. O claim de `IN_FLIGHT` commita antes do HTTP. Checkpoint de importação e as linhas do lote entram na mesma transação.

Índices:

- `idx_records_pending (row_id) WHERE state='PENDING'` — claim em ordem.
- `idx_records_retry (next_attempt_at, row_id) WHERE state='RETRY_WAIT'`.
- `idx_records_batch_state (batch_id, state)` — reconciliação, não o hot path.
- `idx_attempts_row (row_id)`.

Schema versão 1, com `0001_up.sql` e `0001_down.sql`. `user_version` maior que 1 é recusado. Down só existe para teste; não roda em job real.

`IN_FLIGHT` na abertura da execução vira `UNKNOWN`. Não há reenvio automático.

## Consequences

Durabilidade de resposta = commit SQLite, não o arquivo de exportação. Exportação é reconstruível. Falta de espaço ou falha de commit interrompe novos envios; o último commit permanece.

## Rollback

Restaurar o diretório do job. Não há migração destrutiva em produção nesta versão.

# ADR 0002 — Contrato PicPay pendente

- Status: accepted
- Date: 2026-10-13

## Context

O POST documentado é `https://api.picpay.com/v1/partner/account-opening-intentions` com `{"cpf":"..."}`. O nome indica intenção de abertura de conta, não uma consulta pura. Não há, neste repositório, semântica de sucesso, regra de cliente, limite de taxa, idempotência nem política de repetição depois de timeout.

## Decision

Dois modos:

- `pending` (padrão fora de teste): HTTP 2xx com corpo não interpretado vira `UNKNOWN`, `is_customer` fica nulo, `evidence_source=http_only`. Merge normal recusa `UNKNOWN`.
- `simulated-fixture-v1`: só em loopback com `--test-mode`. O corpo tem de ser o fixture versionado. `is_customer` só muda se `customer_evidence` for `customer` ou `not_customer`.

Host que não é loopback é recusado em `job run`, antes de abrir conexão. HTTP sem TLS só é aceito em loopback com `--test-mode`. Redirects não são seguidos. Corpo acima de 64 KiB não é sucesso.

Não há chave de idempotência no modo `pending`. No modo simulado, a chave é `{job_id}:{row_id}` e não contém o CPF.

## Alternatives

- Tratar todo 2xx como cliente. Rejeitado: inventa um booleano.
- Chamar a API real durante o desenvolvimento. Rejeitado pelo pedido.

## Consequences

Um job contra o fixture pode completar. Um job `pending` com respostas HTTP recebidas termina `BLOCKED_UNKNOWN` até existir contrato. Isso é intencional.

## Rollback

Não há chamada de produção para desligar. O portão é o código de `endpoint_allowed`.

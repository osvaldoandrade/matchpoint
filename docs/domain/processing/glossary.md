# Glossário — processamento

Dono: operador local do `pgm-data`. Revisão: a cada mudança de estado ou de contrato.

| Termo | Definição | Não usamos |
| --- | --- | --- |
| AccountOpeningIntention | O POST para `/v1/partner/account-opening-intentions`. Pode ter efeito no provedor. | "consulta de cliente" |
| Job | Um CSV importado, um SQLite, uma configuração congelada. | lote (lote é batch) |
| Record | Uma linha do CSV, com `row_id` estável na ordem original. Duplicata permanece. | deduplicação |
| Batch | Grupo de até 10 mil `row_id` consecutivos. | página de API |
| `is_customer` | Booleano só quando o contrato ativo traz prova. Nulo não é falso. | inferir de HTTP 2xx |
| UNKNOWN | Não há resultado confiável. Bloqueia merge normal. | erro de validação |
| FAILED_FINAL | Falha técnica ou validação local, sem nova tentativa automática. | "não é cliente" |
| SUCCEEDED | Troca HTTP concluída segundo o contrato ativo. No modo pending isso não ocorre para 2xx opaco. | sucesso de negócio inventado |
| Evidence | `none`, `http_only` ou `simulated_fixture`. Diz de onde veio `is_customer`. | tratar o corpo cru como verdade |
| Generation | Número da exportação. `retry-failed` incrementa e não reescreve a geração anterior. | sobrescrever o Parquet publicado |

Invariantes do registro:

- Um `row_id` tem um estado.
- Transição ilegal devolve erro.
- `SUCCEEDED` não volta sozinho.
- `FAILED_FINAL` e `UNKNOWN` só voltam a `PENDING` por comando explícito.
- CPF é texto. Zero à esquerda permanece. Vazio é `validation:empty` e não é enviado.

Invariantes do job:

- `batch_size`, endpoint, checksum e modo de contrato não mudam depois da criação.
- Dois runners não seguram `run.lock` ao mesmo tempo.
- Merge normal exige zero linhas abertas e zero `UNKNOWN`.

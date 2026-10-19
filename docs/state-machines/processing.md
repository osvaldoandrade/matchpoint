# Record

Estados: `PENDING`, `IN_FLIGHT`, `RETRY_WAIT`, `SUCCEEDED`, `FAILED_FINAL`, `UNKNOWN`.

Terminais: `SUCCEEDED`, `FAILED_FINAL`, `UNKNOWN`. Compensação explícita: `Requeue` de `FAILED_FINAL` ou `UNKNOWN` para `PENDING`.

| De | Evento | Para |
| --- | --- | --- |
| PENDING | Claim | IN_FLIGHT |
| PENDING | RejectLocal | FAILED_FINAL |
| RETRY_WAIT | Claim | IN_FLIGHT |
| IN_FLIGHT | Succeed | SUCCEEDED |
| IN_FLIGHT | FailFinal | FAILED_FINAL |
| IN_FLIGHT | ScheduleRetry | RETRY_WAIT |
| IN_FLIGHT | LoseOutcome | UNKNOWN |
| IN_FLIGHT | ReleaseUnsent | RETRY_WAIT |
| FAILED_FINAL | Requeue | PENDING |
| UNKNOWN | Requeue | PENDING |

`ReleaseUnsent` só ocorre quando o processo ainda não enviou o POST (auth indisponível ou cancelamento antes do envio). Cancelamento durante o HTTP deixa `IN_FLIGHT`. Na próxima execução, `IN_FLIGHT` vira `UNKNOWN` (`LoseOutcome`).

# Job

| De | Evento | Para |
| --- | --- | --- |
| PREPARING | PreparedOk | PREPARED |
| PREPARING | FailPrepare | PREPARE_FAILED |
| PREPARING | Interrupt | PREPARING |
| PREPARE_FAILED | Resume | PREPARING |
| PREPARED | Start | RUNNING |
| RUNNING | Pause | PAUSED |
| RUNNING | Interrupt | INTERRUPTED |
| RUNNING | Complete | COMPLETED |
| RUNNING | CompleteWithErrors | COMPLETED_WITH_ERRORS |
| RUNNING | BlockUnknown | BLOCKED_UNKNOWN |
| PAUSED | Resume | RUNNING |
| INTERRUPTED | Resume | RUNNING |
| COMPLETED_WITH_ERRORS | RetryFailed | PREPARED |
| BLOCKED_UNKNOWN | RequeueUnknown | PREPARED |

`COMPLETED` não tem saída.

# Batch

`PENDING`, `RUNNING`, `COMPLETED`, `COMPLETED_WITH_ERRORS`. Derivado dos contadores. `UNKNOWN` ou falha no lote, sem linha aberta, é `COMPLETED_WITH_ERRORS`. O job correspondente com `UNKNOWN` fica `BLOCKED_UNKNOWN`.

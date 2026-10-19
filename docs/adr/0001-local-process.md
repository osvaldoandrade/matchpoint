# ADR 0001 — Um processo local, seis contextos

- Status: accepted
- Date: 2026-10-13

## Context

O operador processa até 20 milhões de CPFs numa máquina macOS, com retomada depois de queda, sem daemon. Autenticação, disco e a API externa falham de formas diferentes.

## Decision

Um binário. Contextos separados em módulos: domínio (sem I/O), SQLite, OAuth/Keychain, HTTP, agendamento, exportação, CLI/TUI. A TUI e o REPL chamam o mesmo `App` dos subcomandos. O adaptador PicPay é um ACL: o domínio não vê JSON do provedor.

O grafo de tarefas é um pool fixo (`concurrency` workers), uma ponte para uma thread SQLite e filas limitadas a 1024 comandos. Não há uma task por CPF.

## Alternatives

- Um serviço com fila externa. Rejeitado: a versão não tem daemon e o disco local já é a fila.
- Carregar o CSV inteiro. Rejeitado: 20 milhões de linhas não cabem no orçamento de memória.

## Consequences

Queda do processo perde no máximo o grupo de respostas ainda não commitado (500 linhas ou 250 ms). Essas linhas ficam `IN_FLIGHT` e, na sessão seguinte, `UNKNOWN`.

## Rollback

Apagar o diretório do job (`job drop --yes`) ou substituir o binário. Os dados em Application Support sobrevivem à troca do binário.

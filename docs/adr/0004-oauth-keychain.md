# ADR 0004 — Token e Keychain

- Status: accepted
- Date: 2026-10-13

## Context

O provedor pediu JSON, não form-urlencoded, para `client_credentials` e para `refresh_token`. O segredo não pode aparecer em argv, log, SQLite, histórico ou no DMG.

## Decision

`TokenGate` guarda o access token só em memória. `client_secret` e `refresh_token` vão ao Keychain (serviço `pgm-data`) ou, em teste, a um mapa em memória. `client_id` fica no `config.json` com modo 0600.

Uma renovação por vez; as outras esperam. Renovação antecipada usa `expires_in` menos a margem (60 s), com trava de 1 s para não entrar em loop quando o token nasce dentro da margem. `token_type` tem de ser Bearer. `expires_in` aceito: 1 a 604800 segundos.

401 invalida só a geração usada naquela chamada. Há uma nova tentativa autenticada. 401/403 depois disso não renovam de novo. `invalid_grant` no refresh cai uma vez para `client_credentials`. Falha de auth pausa claims novos.

`Debug` de segredo imprime `[redacted]`. O writer de log apaga `Bearer`, `client_secret`, `access_token` e `refresh_token`. O doctor não lê o Keychain, para não abrir prompt.

## Consequences

Binário sem assinatura estável pode levar o macOS a pedir acesso ao Keychain na primeira leitura real. Os testes não fazem essa leitura.

## Rollback

`auth configure` de novo substitui os itens. Apagar os itens no Acesso às Chaves remove o segredo sem tocar nos jobs.

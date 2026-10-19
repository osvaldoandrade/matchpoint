# Operação

## Minuto 0 — subir

`pgm-data doctor`. Confira disco local, espaço (≥ 256 MiB) e se o `client_id` existe. O doctor não mostra segredo.

`pgm-data auth configure --allow-remote` numa sessão de terminal, ou `/api` / Ctrl+O na TUI. Confirme as URLs, informe client-id e client-secret e habilite o envio. O segredo fica no Keychain. `pgm-data auth status` mostra somente o perfil. Para bloquear novos envios sem alterar credenciais: `pgm-data auth configure --disable-remote`.

## Durante o job

A TUI inicia na home; selecione **Jobs** para operar. **N** cria um job, **R** executa ou retoma, **P** pausa o job ativo, **S** abre Settings, **T** configura threads, **K** verifica SHA-256 e **E** exporta. **H** volta à home. ↑/↓ seleciona o job, Tab troca painel e PgUp/PgDn rola conteúdo. **?** abre ajuda. O prompt avançado está em **/**; Ctrl+L abre os eventos completos. Durante uma operação local, a saída aguarda sua gravação.

Threads aceitam 1–256 workers e preservam o RPS. Altere jobs ativos só depois de PAUSADO. **Settings → Processing** persiste threads, RPS, tamanho de lote e validação dos novos jobs. `job threads <id> --count 16` e `job threads --default --count 8` oferecem o mesmo controle na CLI.

As barras distinguem percentual real de animação de atividade. Registros tratados incluem UNKNOWN; os incertos aparecem separadamente e nunca equivalem a cliente confirmado. A animação pode ser desligada com `/motion`.

No Ghostty, `./packaging/ghostty/run-ansi.command` abre o perfil ANSI com fonte IBM VGA 8x16. Cmd +/− ajusta o tamanho; Cmd 0 restaura. **Settings → Appearance** ajusta fonte, tamanho e animação; a fonte é aplicada em uma nova janela. **D** abre diagnóstico e **?** abre ajuda em quatro páginas, navegáveis com Tab/Shift+Tab.

`job pause` grava `desired=pause`. O runner interrompe novos envios e espera as chamadas em curso até o prazo (15 s no comando), antes de gravar `PAUSED`. As respostas recebidas são preservadas; chamadas ainda sem resultado ao fim do prazo viram `UNKNOWN`. `job resume` continua as linhas pendentes sem reenviar resultados terminais.

## Minuto 5 — auth falhou

Sintoma: eventos `auth_paused`, linhas em `RETRY_WAIT` com `unauthorized`. Não há loop de refresh. Corrija o Keychain com `auth configure` e rode de novo. Se a linha esgotou tentativas, ela está `FAILED_FINAL`; `job retry-failed` abre outra geração de exportação.

## Minuto 15 — disco ou UNKNOWN

Sintoma: estado `INTERRUPTED` e erro de disco, ou `BLOCKED_UNKNOWN`. Não reenvie `UNKNOWN` no escuro. `job requeue-unknown` é a ação explícita e também incrementa a geração. Merge normal continua recusado até não haver `UNKNOWN`, salvo `--allow-unknown`, que marca `partial: true`.

## Minuto 60 — retomar depois de queda

O processo não fica no ar. Abra de novo, `job status`, `job run`. Checksum, schema e endpoint têm de bater. Linha já `SUCCEEDED` ou `FAILED_FINAL` não é reenviada.

## Integridade

**K** ou `job checksum <id>` compara SHA-256 da cópia `input.csv` com o hash da importação e das exportações com os respectivos manifestos. Confira esperado, obtido e momento da verificação. Arquivo ausente ou alterado gera DIVERGENTE. Não reenvie o CSV alterado: restaure a cópia original ou crie outro job.

## Exportação

`job merge --format parquet` ou `csv`. Arquivos em `exports/gen-NNNN/`. Interrupção no rename deixa o SQLite intacto; rode o merge outra vez. Não concatene Parquet byte a byte.

## Remoção

Prefixo de desenvolvimento: `make uninstall` (só `PREFIX/bin/pgm-data`). Pacote: `/usr/local/pgm-data/uninstall.sh`. Dados só com `--delete-data`.

## O que esta versão não faz

Os testes não chamam a API PicPay real. O contrato pendente não permite interpretar 2xx como cliente; 5xx também fica UNKNOWN e não dispara repetição automática. O envio remoto só usa URLs HTTPS e client-id iguais ao perfil explicitamente habilitado. Não promete exactly-once remoto. Tem navegação de ajuda, lotes e eventos na TUI. Não tem notarização automática.

# STRIDE — processador local

Ativo: CSV de CPF, segredo OAuth, access token, SQLite com resultado. Adversário: outro processo no mesmo Mac, um servidor que recebe o POST, ou um arquivo CSV hostil. Não há multitenant.

| Limite | Ameaça | Controle |
| --- | --- | --- |
| Operador → CLI | Segredo em argv ou histórico | Não há flag de segredo. Prompt sem eco. Histórico descarta linhas com `client_secret`, `refresh_token`, `access_token`, `Bearer`. |
| Processo → Keychain | Prompt ou leitura por outro binário | Itens genéricos no serviço `pgm-data`. Doctor não lê o segredo. Testes usam memória. |
| Processo → PicPay | Token em log, redirect com Authorization | Sem log do corpo. Redirect desligado. TLS 1.2+, certificado verificado. Host remoto recusado enquanto o contrato estiver pendente. |
| CSV → importação | Campo malformado, aspas, duplicata | Parser incremental. Aspas não fechadas viram erro e o lote anterior permanece. Duplicata não é apagada. |
| Dois processos → um job | Corrida de escrita | `flock` exclusivo. SQLite com um escritor. |
| Disco cheio | Estado pela metade | Commit é a fronteira. Falha para novos envios. Último commit fica. |
| Exportação | Arquivo parcial lido como final | Escreve em `.tmp`, `fsync`, rename, `fsync` do diretório. Manifesto é a publicação. |
| DMG | Credencial embutida | O pacote não contém segredo nem SQLite de cliente. |

Aceito: um processo do mesmo usuário com acesso ao Keychain e ao diretório de dados lê CPF e segredo. Isso é o modelo de ameaça de uma ferramenta local de um operador.

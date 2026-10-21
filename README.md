# MatchPoint

Matching local de datasets CSV e Parquet com referências DAT.

**macOS e Linux** — execute como usuário normal:

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/osvaldoandrade/matchpoint-install/main/install.sh | bash
```

**Windows (PowerShell)** — instala e usa Ubuntu no WSL2:

```powershell
curl.exe --proto '=https' --tlsv1.2 -fSL https://raw.githubusercontent.com/osvaldoandrade/matchpoint-install/main/install.ps1 -o install-matchpoint.ps1
if ($LASTEXITCODE -eq 0) { powershell -NoProfile -ExecutionPolicy Bypass -File .\install-matchpoint.ps1 }
```

O instalador prepara as ferramentas, instala Rust se necessário, clona o código e compila com `cargo build --locked --release`. O código está disponível publicamente no GitHub. No primeiro uso, macOS pode pedir as ferramentas Apple; Windows pode exigir ativação do WSL, reinício e criação do usuário Linux. Conclua a etapa indicada e execute novamente.

Abra um novo terminal (Ubuntu no Windows) e execute:

```sh
matchpoint tui
```

**[Documentação completa no Wiki do projeto](https://github.com/osvaldoandrade/matchpoint/wiki)**

[Instalação e atualização](https://github.com/osvaldoandrade/matchpoint/wiki/Instalacao) · [Primeiros passos](https://github.com/osvaldoandrade/matchpoint/wiki/Primeiros-passos) · [CLI](https://github.com/osvaldoandrade/matchpoint/wiki/Referencia-CLI)

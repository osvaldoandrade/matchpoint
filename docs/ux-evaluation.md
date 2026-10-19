# Avaliações independentes da TUI

Objetivo: estética BBS ANSI dos anos 90, paleta uniforme, home/splash distinta,
progresso animado com contagens reais, configuração de threads, erros acionáveis
e verificação SHA-256. A correção do usuário rejeitou o primeiro wordmark e a
mistura de onboarding, jobs vazios e comandos slash; menus e formulários passam
a ser o percurso principal, com prompt avançado opcional.

Fitness: cada avaliador atribui 0–10 a cada uma das [10 heurísticas de
Nielsen](https://www.nngroup.com/articles/ten-usability-heuristics/), com pesos
iguais. A nota do avaliador é sua média; a rodada é a média dos três avaliadores.
As notas são avaliações subjetivas apoiadas em evidência, não métricas de usuários
reais. Aprovação exige média de cada avaliador estritamente superior a 9,8 na mesma versão;
nenhum requisito explícito pode permanecer incompleto. Notas não são solicitadas
para atingir uma meta. Recomendações e impactos pertencem aos respectivos
avaliadores e podem se sobrepor.

## Cenários fixos

1. Home/primeiro uso sem jobs/API e menu principal.
2. Importar CSV (caminho com espaços, ausente, validação, cancelamento).
3. Múltiplos jobs, seleção visível e nomes dos arquivos.
4. Executar, observar progresso em dois instantes, pausar e retomar.
5. Threads válidas, inválidas, persistência e tentativa durante execução.
6. Erros, recuperação, detalhes navegáveis e preservação da entrada.
7. Checksum íntegro, entrada alterada, exportação e hashes completos.
8. Navegação principal, ajuda, prompt avançado e histórico.
9. 120×40, 100×30, 80×24, 40×18, e comportamento abaixo do mínimo.
10. Sigilo de segredos, acesso remoto explícito e saída aguardando operações.

## Rodada 0 — estado inicial

Snapshot fonte: /tmp/pgm-ui-baseline-20261014. Avaliadores A e B observaram
PTY real com dados isolados; C inspecionou fonte, testes e preview com limitações
explicitadas. Nenhum deles alterou código nem usou Keychain ou API real.

| Heurística | A | B | C |
| --- | ---: | ---: | ---: |
| H1 Estado visível | 5,8 | 6,7 | 6,0 |
| H2 Linguagem do operador | 6,4 | 7,1 | 6,5 |
| H3 Controle/liberdade | 7,2 | 7,3 | 7,0 |
| H4 Consistência | 6,4 | 6,8 | 5,5 |
| H5 Prevenção de erros | 7,3 | 7,5 | 6,5 |
| H6 Reconhecimento | 5,5 | 6,8 | 6,0 |
| H7 Eficiência | 6,2 | 6,9 | 5,5 |
| H8 Estética/minimalismo | 5,3 | 3,8 | 3,0 |
| H9 Recuperação de erros | 5,2 | 5,9 | 4,0 |
| H10 Ajuda | 6,2 | 6,7 | 6,0 |
| Média individual | **6,15** | **6,55** | **5,60** |

Média da rodada: **6,10/10**. Reprovada.

### Recomendações de A

| Recomendação | Severidade / H | Impacto potencial na média |
| --- | --- | --- |
| Barra em todas as larguras; preparação responsiva | Alta / H1,H3,H7 | +0,55–0,80 |
| Distinguir ativo/selecionado e alvo da pausa | Alta / H1,H3,H5 | +0,30–0,50 |
| Threads com limites, valor atual e entrada preservada | Alta / H2,H5,H6,H7 | +0,45–0,70 |
| Erros em português, causa e ação contextual | Alta / H2,H6,H9 | +0,45–0,70 |
| Checksum com esperado/obtido/arquivo/destino | Alta / H1,H5,H6,H9 | +0,35–0,55 |
| Limpar ajuda, rolar conteúdo e preservar ações em 40×18 | Média / H4,H6,H8,H10 | +0,35–0,55 |
| Paleta uniforme e arte ANSI original | Média / H4,H8 | +0,30–0,45 |
| Nome do arquivo, seleção e listas longas | Média / H1,H2,H6,H7 | +0,30–0,50 |
| Resultados completos navegáveis | Média / H1,H6,H7,H9 | +0,30–0,45 |

### Recomendações de B

| Recomendação | Severidade / H | Impacto potencial na média |
| --- | --- | --- |
| Identidade ANSI BBS e paleta principal | Alta / H4,H8 | +0,60–0,90 |
| Barra e resumo em 80×24/40×18 | Alta / H1,H6,H7,H8 | +0,45–0,65 |
| Threads descobríveis, faixa e bloqueio explicado | Alta / H3,H5,H6,H7,H10 | +0,45–0,70 |
| Erros operacionais traduzidos e entrada preservada | Alta / H2,H4,H9 | +0,40–0,60 |
| Verificação explícita da entrada/exportação | Alta / H1,H5,H6,H9,H10 | +0,35–0,55 |
| Ajuda sem resíduos e leitura completa | Média / H4,H8,H10 | +0,15–0,25 |
| Arquivos/seleção/rolagem | Média / H1,H6,H7 | +0,20–0,35 |
| Animação distinta do percentual e fases da pausa | Média / H1,H3 | +0,15–0,30 |

### Recomendações de C

| Recomendação | Severidade / H | Impacto potencial na média |
| --- | --- | --- |
| Paleta ANSI, bordas quadradas e arte compacta | Alta / H4,H8 | +0,70–1,00 |
| Barra por job, seleção e rolagem | Alta / H1,H3,H6,H7 | +0,60–0,90 |
| Threads explícitas, validação/cancelamento/bloqueio | Alta / H1,H5,H6,H7,H10 | +0,50–0,80 |
| Erros em português, entrada corrigível e atividade | Alta / H1,H3,H9 | +0,50–0,80 |
| SHA-256 esperado/obtido/instante/destino | Alta / H1,H5,H6,H9 | +0,40–0,70 |
| Importação por formulário com validação | Média / H5,H6,H7 | +0,30–0,50 |
| Ajuda navegável e edição previsível | Média / H3,H6,H7,H10 | +0,30–0,50 |

## Referências visuais consultadas

- [DMA, pack dma-vlfd, VL!INFO.ANS](https://16colo.rs/pack/dma-vlfd/VL%21INFO.ANS):
  volumes e sombras construídos por blocos, meios blocos e caracteres de densidade.
- [ACiD, Golgotha, Covert Action 2](https://16colo.rs/pack/acdu0893/GL-CVT2.ANS):
  cena figurativa e composição vertical, em vez de um wordmark simples.

A primeira cena do operador era original e foi posteriormente rejeitada pelo
usuário. A versão atual usa geometria numérica original e cores ANSI diretamente;
não depende de imagem raster ou cópia dos packs.

## Evidência reproduzível

`cargo run --offline --example ui_review -- target/ux-review/<rodada>` gera
renderizações TXT, ANSI e SVG da TUI real nos quatro tamanhos e executa os fluxos
com CSVs sintéticos, MemorySecrets e fixture HTTP de loopback.
`cargo run --offline --example ui_review -- --live` abre a mesma TUI num ambiente
sintético isolado, sem Keychain. Capturas não substituem a validação do motor;
as evidências de execução e os testes são registrados separadamente.

## Rodada 1 — versão e evidência

Fonte congelada: `216c2525fdb1c63c000ac423eb83a057c5d71552db3592738716b3b42e91da39`.
Manifesto e 88 telas em `target/ux-review/round-1/`. A cena da home foi inspecionada
visualmente no navegador; o fluxo sintético executou pausa do ativo após mudança
de seleção, retomada até COMPLETED, exportação pelo formulário, hashes iguais e
entrada alterada divergente. A pausa anterior abandonava workers; agora o writer
aguarda os resultados ou o prazo de drenagem, antes de publicar o estado final.

Validação desta versão: **86 testes passaram**, inclusive regressão com 16 workers
na pausa e navegação da home; `cargo clippy --offline --all-targets --all-features
-- -D warnings` passou. Os três avaliadores receberam a mesma fonte e renders,
com instrução de fazer avaliação independente e testar o PTY isolado.

## Rodada 1 — resultado independente

| Heurística | A | B | C |
| --- | ---: | ---: | ---: |
| H1 | 8,8 | 8,5 | 8,3 |
| H2 | 8,8 | 9,0 | 8,9 |
| H3 | 8,4 | 8,1 | 8,3 |
| H4 | 8,3 | 8,6 | 8,5 |
| H5 | 9,2 | 9,2 | 9,2 |
| H6 | 8,3 | 8,7 | 8,6 |
| H7 | 8,8 | 8,4 | 8,0 |
| H8 | 8,1 | 8,6 | 8,3 |
| H9 | 8,5 | 8,0 | 7,8 |
| H10 | 8,6 | 8,6 | 8,5 |
| Média | **8,58** | **8,57** | **8,44** |

Média da rodada: **8,53/10**, reprovada. As três avaliações observaram fonte
sem divergência do manifesto, renderizações coloridas e PTYs independentes,
sem dados reais ou interferência na sessão do usuário.

### Recomendações e impactos estimados

| Avaliador | Recomendação | Severidade | Impacto na média |
| --- | --- | --- | --- |
| A | Ações bloqueadas não podem restaurar comandos internos no prompt | Alta | +0,20–0,35 |
| A | Banner com causa específica e recuperação de threads | Alta | +0,15–0,25 |
| A | Resumo compacto preservando o denominador em 40×18 | Alta | +0,08–0,15 |
| A | Ajuda prioriza menu/letras | Média | +0,12–0,22 |
| A | Reduzir instruções permanentes | Média | +0,10–0,20 |
| A | Campos com cursor e explicações compactas | Média | +0,08–0,16 |
| A | Diagnóstico visível e pausa de job parado explicada | Média | +0,08–0,15 |
| A | Moldura, arte animada e tipografia consistente | Média | +0,07–0,15 |
| B | Manter o modo de menu após erros | Alta | +0,30–0,45 |
| B | Paginar conforme itens efetivamente visíveis | Alta | +0,15–0,25 |
| B | Avisos vinculados ao job de origem | Alta | +0,15–0,25 |
| B | Mostrar causa e ação no bloqueio de threads | Alta | +0,15–0,25 |
| B | Distinguir destino do job do perfil global | Média | +0,10–0,15 |
| B | Instruções específicas e cursor por formulário | Média | +0,10–0,20 |
| B | Menos texto, moldura e arte animada | Média | +0,10–0,20 |
| C | Paginação sem pular lotes | Maior | +0,25–0,45 |
| C | Vincular/limpar aviso ao trocar seleção | Maior | +0,15–0,30 |
| C | Recuperação direta do bloqueio de threads | Maior | +0,20–0,35 |
| C | Cursor, viewport e instruções específicas | Moderada | +0,20–0,35 |
| C | Números completos em 40×18 | Moderada | +0,10–0,20 |
| C | Moldura, animação e fonte apropriada | Moderada | +0,15–0,30 |
| C | Reduzir texto e priorizar menus na ajuda | Moderada | +0,15–0,30 |
| C | Medir latência com muitos jobs | Moderada | +0,10–0,20 |

Os impactos são sobrepostos. Achados críticos reproduzidos: T durante execução
inseria threads-form no prompt e P deixava de pausar; PgDn pulava lotes em telas
curtas; aviso SHA-256 DIVERGENTE permanecia após selecionar outro job. A rodada 2
corrige esses fluxos e incorpora as solicitações de fonte Ghostty, bordas,
menos texto e animação ANSI da arte.

## Rodada 2 — versão anterior à substituição numérica

Fonte congelada: `4f331b17785bb0f545018ad3ac9a7d6f49a7e46c72f5bfafc4eaaa019adc12b3`.
Manifesto e 96 telas nos quatro tamanhos em `target/ux-review/round-2/`;
snapshot em `/tmp/pgm-ui-round-2-source-20261014`. A home usa moldura dupla,
menu compacto e animação da cena ANSI. O menu Aparência oferece IBM VGA 8x16
ou Menlo, tamanho e velocidade; o perfil Ghostty próprio tem zoom por teclado.

Validação: **92 testes passaram**, clippy sem avisos e harness sintético
completo aprovado. O Ghostty 1.2.3 reconheceu a fonte por `+list-fonts` e
aceitou o perfil por `+validate-config`. O launcher foi verificado com
`open` substituído por um gravador de argumentos, preservando caminhos com
espaços e `PGM_DATA_HOME`, sem abrir interface durante esse teste.

As PNGs são a renderização real do TestBackend com a fonte VGA, rasterizada
com fontconfig local; não são screenshots nativas. A ferramenta de captura
negou acesso ao app Ghostty, portanto não se afirma aceitação visual da janela
nativa. A fonte é distribuída sem alterações, com licença e atribuição de VileR.

Latência medida: 150 jobs sintéticos, 12 amostras de seleção, refresh SQLite
e renderização em 100×30, **p95 = 84,83 ms**. O JSON conserva todas as amostras;
esta medição não representa outros equipamentos nem arquivos de qualquer tamanho.

Os três avaliadores receberam a mesma versão e evidências para nova avaliação
independente. O resultado está abaixo; essa arte foi rejeitada posteriormente
pelo usuário e as notas não aprovam a substituição numérica.

## Rodada 2 — resultado independente

| Heurística | A | B | C |
| --- | ---: | ---: | ---: |
| H1 | 9,3 | 9,5 | 9,6 |
| H2 | 9,6 | 9,4 | 9,5 |
| H3 | 9,5 | 9,7 | 9,5 |
| H4 | 9,6 | 9,6 | 9,4 |
| H5 | 9,6 | 9,8 | 9,7 |
| H6 | 9,7 | 9,6 | 9,4 |
| H7 | 9,5 | 9,5 | 9,5 |
| H8 | 9,6 | 9,7 | 9,3 |
| H9 | 9,4 | 9,5 | 9,4 |
| H10 | 9,6 | 9,6 | 9,5 |
| Média | **9,54** | **9,59** | **9,48** |

Média exata da rodada: **286,1 ÷ 30 = 9,536666…**, reprovada.

Recomendações históricas confirmadas pelos próprios avaliadores, sem reavaliação:

| Avaliador | Recomendação | Severidade | Impacto na média |
| --- | --- | --- | --- |
| A | Capturar ID ativo antes de limpar e vincular conclusão/pausa/erro à origem | Alta | +0,10–0,18 |
| A | Causa e ação completas em 40×18; não sugerir abrir o formulário já aberto | Média | +0,04–0,08 |
| A | Remover/atualizar erro do campo quando corrigido | Baixa | +0,02–0,04 |
| A | Preservar rótulos completos K/Q no footer compacto | Baixa | +0,01–0,03 |
| B | Identificar o job nas notificações finais de pausa/conclusão | Moderada | +0,08–0,15 |
| B | Diagnóstico em linguagem do operador: resultado/consequência/ação | Moderada | +0,08–0,15 |
| B | Fazer instruções/mensagens caberem integralmente em 40 colunas | Baixa | +0,03–0,07 |
| B | Aceitação visual nativa da fonte e perfil Ghostty | Lacuna de evidência | Sem ganho numérico presumido |
| C | Distinguir registros “Em curso” de chamadas/workers simultâneos | 2 | +0,05–0,12 |
| C | Preservar Q:sair integralmente em 40 colunas | 1 | +0,03–0,07 |
| C | Priorizar ação por letra/menu na recuperação | 1 | +0,05–0,10 |
| C | Traduzir diagnóstico genérico/auth mantendo detalhe em Eventos | 1 | +0,03–0,08 |
| C | Aceitação visual nativa Ghostty/VGA/zoom/animação | Lacuna de evidência | Sem ganho automático |

## Substituição numérica — versões e evidência

O usuário pediu interface 100% em inglês, arte geek/math e Settings com tabs,
depois rejeitou o torus como um “O girando”. A implementação atual remove o
torus: **tesseract numérico em rotação 4D** por padrão e **lattice numérico 3D**
alternativo. Home contém Jobs/New Job/Settings; Jobs reúne seleção e detalhes
com tabs Overview/Batches/Events/Integrity; Settings reúne API/Secrets/Processing/
Appearance. As versões antigas permanecem no histórico, sem carregar notas.

| Versão | Fonte agregada SHA-256 | Situação |
| --- | --- | --- |
| numeric-2 | `f2848f8d40159472c62e13102ddb057e989981eed266ce4760a38fcc2fcb53ed` | Primeiro snapshot avaliado da substituição |
| numeric-7 | `ef394fbc73930f7c22a103fb41224c18d10f0cfa610f5b67a2ee6063ecbacfd3` | Drafts, Save por grupo, contexto e recuperação; avaliações A/C |
| numeric-8 | `03416a417287206312441ed2c4f50dd025b034ee6951eec93e4be3cf36231a02` | Contraste de títulos/bordas corrigido; avaliação C |
| numeric-9 | `7ca9b913770e0224f8ae48d9887f24059ca8c253e1be01da1db49f86ac49bd6d` | Warning de defaults inválidos e confirmação final comum |

numeric-3/4/5/6 foram verificações intermediárias de implementação, sem rodada
completa de três notas. Não são aprovações. Freeze numeric-2 tinha 87 arquivos
e omitia dois SQLs existentes: a execução da PTY de B usou uma cópia separada de
build com os mesmos 87 arquivos e os dois SQLs suplementados e hashados; o freeze
original ficou intacto. numeric-7/8/9 contêm 89 arquivos, incluindo esses SQLs.

### numeric-2 — resultados disponíveis

| Heurística | A | C |
| --- | ---: | ---: |
| H1 | 9,0 | 9,7 |
| H2 | 9,3 | 9,4 |
| H3 | 9,2 | 9,6 |
| H4 | 9,3 | 9,3 |
| H5 | 9,7 | 9,7 |
| H6 | 9,3 | 9,1 |
| H7 | 9,4 | 9,6 |
| H8 | 9,6 | 9,4 |
| H9 | 9,3 | 9,5 |
| H10 | 9,4 | 9,5 |
| Média | **9,35** | **9,48** |

B confirmou que numeric-2 ficou incompleta, sem nota final; não há média de
rodada de três avaliadores. A e C inspecionaram
fonte congelada, renders e evidência sintética; não executaram PTY própria dessa
versão. C executou testes pertinentes antes da divergência posterior da árvore
viva. A build/PTY independente redigida por B foi executada pelo executor na
cópia de build numeric-2: 42 capturas aprovadas. Isso é execução do executor,
não uma execução de ferramenta própria de B. Falhas anteriores de bind/SQL são
limites do ambiente/pacote de evidência e não recebem nota de defeito do produto.

| Avaliador | Recomendação | Impacto na média |
| --- | --- | --- |
| A | Sinal visível de drafts não salvos | +0,12–0,20 |
| A | Explicar Save compartilhado API + Secrets | +0,05–0,09 |
| A | Vincular notice/error ao grupo de origem | +0,05–0,09 |
| A | Contraste e contador de campos em tela compacta | +0,07–0,12 |
| A | Preview usa o rascunho de Appearance | +0,03–0,06 |
| C | Mensagem UNKNOWN de recuperação em inglês | +0,04–0,10 |
| C | Indicador campos visíveis/total | +0,08–0,15 |
| C | Contraste do campo em edição | +0,06–0,12 |
| C | Footer compacto completo | +0,05–0,10 |
| C | Home compacto e filename legível | +0,03–0,07 |
| C | Clareza do grupo salvo | +0,03–0,07 |

### numeric-7 — avaliações parciais

| Heurística | A | C |
| --- | ---: | ---: |
| H1 | 9,7 | 9,7 |
| H2 | 9,9 | 9,9 |
| H3 | 9,9 | 9,8 |
| H4 | 9,8 | 9,5 |
| H5 | 9,9 | 9,9 |
| H6 | 9,5 | 9,5 |
| H7 | 9,9 | 9,9 |
| H8 | 9,6 | 9,4 |
| H9 | 9,8 | 9,8 |
| H10 | 9,9 | 9,9 |
| Média | **9,79** | **9,73** |

Sem média de rodada nem aprovação. Ambos confirmaram o contraste verde sobre
verde do título/borda focados em SVG e PNG, apesar do texto existir no TXT.
A recomendou inverter somente o conteúdo editável (+0,07–0,12) e avisar sobre
defaults Processing substitutos inválidos (+0,01–0,03). C recomendou corrigir
o contraste (+0,12–0,25). Sem PTY própria nova; fonte/render/evidência fornecida.

### numeric-8 — avaliação parcial

C: H1 9,90; H2 9,90; H3 9,85; H4 9,90; H5 9,90; H6 9,90; H7 9,90;
H8 9,85; H9 9,90; H10 9,90. **98,90 ÷ 10 = 9,89**, sem arredondamento.
Nenhuma recomendação corretiva material; impacto incremental estimado zero.
Confirmou títulos verde sobre preto e inversão limitada ao conteúdo. Conferiu
freeze antes/depois; sem novo build/PTY próprio. A árvore viva já tinha correção
posterior, portanto essa aprovação individual vale apenas para numeric-8.
Não há aprovação de rodada a partir da nota de um único avaliador.

### numeric-9 — verificação final comum

Snapshot completo: `/tmp/pgm-ui-numeric-9-source-20261014`. Artefatos atuais:
`target/ux-review/numeric-9/`. **110 testes**, fmt/check/clippy/build, secret scan,
perfil Ghostty e PTY sintética do executor passaram. 132 PNGs VGA, quatro tamanhos.
Latência em 64 amostras por cenário: navegação com SQLite p95 60,720/61,498 ms;
home CPU + TestBackend p95 0,234–0,798 ms para ambos os tipos de geometria.
Essas medições não observam pintura nativa Ghostty. Regras, comandos e limites
de verificação completos estão no [workflow ledger](workflow-ledger.md).

Os três avaliadores receberam a mesma fonte final e evidência atual, sem editar
código, com hash antes/depois e notas independentes. Resultados abaixo.
Só documentação de evidência é acrescentada à árvore viva após o congelamento.
Aceitação visual do usuário, API real e release permanecem separadas da fitness.


### numeric-9 — resultado independente

| Heurística | A | B | C |
| --- | ---: | ---: | ---: |
| H1 | 9,9 | 9,4 | 9,90 |
| H2 | 9,8 | 9,8 | 9,90 |
| H3 | 9,9 | 9,9 | 9,90 |
| H4 | 9,9 | 9,8 | 9,90 |
| H5 | 9,9 | 9,9 | 9,90 |
| H6 | 9,9 | 9,5 | 9,90 |
| H7 | 9,8 | 9,8 | 9,90 |
| H8 | 9,8 | 9,9 | 9,85 |
| H9 | 9,9 | 9,8 | 9,90 |
| H10 | 9,9 | 9,8 | 9,90 |
| Média | **9,87** | **9,76** | **9,895** |

Média agregada: **295,25 ÷ 30 = 9,841666…**. Rodada **reprovada**: B não
superou 9,8 e encontrou contagens cortadas em 40×18. A média agregada não
apaga o achado nem substitui a aprovação dos três avaliadores.

A executou cinco regressões próprias: contraste, Processing inválido, API
inválida, drafts/Save por grupo e ações da ajuda; todas passaram. Relatório
preservado em `target/ux-review/numeric-9/nielsen-a-tests.json`. B escreveu e
executou um harness sem rede ligado à fonte congelada, usando as mesmas versões
runtime do lock. Quatro estados × quatro tamanhos: Running passou em todos;
Completed, Completed with errors e Uncertain results cortaram o denominador de
`1000000/1000000` em 40×18. Capturas/harness original em
`target/ux-review/numeric-9/nielsen-b-counter-harness/` (cópia da evidência
original de B). C inspecionou fonte/renders/evidência fornecida,
sem novo build/PTY próprio. Os 89 arquivos congelados ficaram íntegros antes/depois
nas três avaliações. Nenhum avaliador afirmou observar Ghostty nativo.

| Avaliador | Recomendação | Severidade | Impacto na média |
| --- | --- | --- | --- |
| A | Resumo compacto do perfil API vinculado em New Job | Baixa, opcional | +0,01–0,03 |
| A | Observar fonte/redimensionamento/fluidez em Ghostty nativo | Validação complementar | Sem aumento automático |
| B | Separar ID/estado/contagem conforme largura e recalcular paginação pela altura efetiva | Moderada, H1/H6 | +0,06–0,12 |
| B | Observar fonte/foco/animação em Ghostty nativo | Lacuna de evidência | Sem ganho presumido |
| C | Nenhuma recomendação corretiva material | — | 0 |

### numeric-10 — correção de contagens em telas estreitas

O resumo passa a medir o texto real: conserva uma linha quando cabe e separa
identidade/estado da contagem quando necessário. Valores com 19 dígitos têm
linhas Done/Total para preservar os números. A paginação de lotes usa essa
mesma altura efetiva. Duas regressões verificam estados/contagens de um milhão,
20 milhões e o limite inteiro em todos os quatro tamanhos, além da visita de
cada lote sem saltos quando o resumo ocupa mais linhas. O harness de revisão
inclui agora os quatro estados com contagens grandes como fixtures visuais
sintéticos; os fluxos reais de processamento continuam verificados separadamente.

Esta versão requer nova confirmação independente dos três, sem reutilizar
notas de numeric-9. Fonte e resultados ainda serão registrados abaixo.


### numeric-10 — resultado independente

Fonte `0eabbae5306ea7d4526ff896b93d38fec8d691042f0a4b019f32a7b47a1456a2`,
89 arquivos íntegros antes/depois nas três avaliações.

| Heurística | A | B | C |
| --- | ---: | ---: | ---: |
| H1 | 9,9 | 9,9 | 9,80 |
| H2 | 9,7 | 9,7 | 9,70 |
| H3 | 9,9 | 9,9 | 9,90 |
| H4 | 9,7 | 9,7 | 9,60 |
| H5 | 9,9 | 9,9 | 9,90 |
| H6 | 9,9 | 9,9 | 9,90 |
| H7 | 9,8 | 9,8 | 9,90 |
| H8 | 9,8 | 9,9 | 9,80 |
| H9 | 9,8 | 9,8 | 9,90 |
| H10 | 9,8 | 9,7 | 9,80 |
| Média | **9,82** | **9,82** | **9,82** |

Média da rodada: **9,82**, acima do limiar. A entrega ainda foi **reprovada pelo
requisito explícito English incompleto**: o título do prompt mantinha COMANDO /
Enter executar e o launcher mantinha uma mensagem em português. As notas não
substituem os requisitos do usuário.

A executou cinco regressões próprias e inspecionou 26 PNGs; resultados originais
em `/var/folders/14/52y2hz4j46s7_h04n92vcrx00000gn/T/pgm-nielsen-a-numeric10-tests-10pe__30/results.json`.
B executou o harness dos quatro estados/tamanhos (16/16) e dez testes TUI sem
rede, todos aprovados. C executou um probe isolado: a hipótese inicial de
Events com conteúdo zero em 1M + erro foi **desconfirmada**, não entrou como
defeito. Confirmou, porém, que i64::MAX + banner em 40×18 escondia Total enquanto
Done e uma linha de Events permaneciam. Esse extremo é sintético e pouco plausível.
Artefatos originais de C: `/tmp/pgm-review-c-numeric10/`.

| Avaliador | Recomendação | Severidade | Impacto na média |
| --- | --- | --- | --- |
| A | Traduzir prompt/launcher e verificar Home/Jobs com/sem sugestões | Baixa; bloqueante para English | +0,03–0,06 |
| A | Explicar conflito de fonte e identificar arquivos | Baixa | +0,01–0,02 |
| A | Perfil API compacto em New Job | Baixa, opcional | +0,01–0,03 |
| A | Observar Ghostty nativo | Validação complementar | Sem aumento automático |
| B | Traduzir título e instrução do prompt | Baixa; requisito explícito | +0,02–0,04 |
| B | Observar fonte/animação Ghostty nativo | Lacuna de evidência | Sem ganho presumido |
| C | Traduzir título do prompt nos quatro tamanhos | Menor; H2/H4/H10 | +0,03–0,07 |
| C | Preservar Total em 19 dígitos + erro compacto | Cosmética; H1 | +0,01–0,03 |

### numeric-11 — verificação intermediária

Freeze `40f4934fd22076067c9b9c6b60bad87e5d44841d9a74fc3b2d2c6113449aca0a`.
O prompt e o erro do launcher foram traduzidos e uma regressão cobre Home/Jobs,
com/sem sugestões e os quatro tamanhos. **113 testes**, fmt/check/clippy/builds,
fluxo e latência passaram nessa fonte. Não houve nota ou aprovação de rodada:
antes de iniciar a confirmação dos três, foi incorporado o limite com banner
identificado por C em numeric-10. O freeze 11 permanece histórico e intacto.

### numeric-12 — confirmação final

Esta revisão inclui o idioma corrigido e reserva conteúdo útil quando o banner
compete com contagens longas. Resumo, render e paginação partilham o cálculo de
áreas; espaçamento/barra cedem altura antes dos números. No extremo mais apertado,
a borda do painel e o footer identificam a tab ativa. O scroll de detalhes usa
até cinco linhas, limitado às linhas efetivamente visíveis, evitando saltos em
Events compactos. Uma regressão verifica Done/Total, todos os dez eventos e todos
os 25 lotes sob erro nos quatro tamanhos, inclusive no limite inteiro.

Os fixtures visuais adicionam Events/Batches sob erro extremo; seguem sintéticos,
separados do fluxo real loopback. Os três avaliadores precisam confirmar a mesma
fonte final, sem carregar pontuação anterior. Hash, verificações e resultados
serão acrescentados após o congelamento desta implementação.

Fonte final completa numeric-12: `4cf43d9234bea2ea24639e92983a4f462914293bff9785602995cdd414f77ee9`, 89 arquivos. **114 testes** (74 unitários + 40 integração), fmt/check/clippy, debug/release builds, fluxo loopback, duas PTYs do executor e secret scan passam. 156 PNGs VGA atuais. Logs por comando, manifestos e scripts/checks estão em `target/ux-review/numeric-12/`. Latência atual em 64 amostras: navegação p95 **60,938/63,729 ms**; home CPU + TestBackend p95 **0,234–0,787 ms**. O perfil Ghostty mantém o hash previamente validado; a janela nativa não foi observada. Apenas documentação de evidência muda após o freeze.


### numeric-12 — resultado independente final

| Heurística | A | B | C |
| --- | ---: | ---: | ---: |
| H1 | 9,9 | 9,9 | 9,90 |
| H2 | 9,9 | 9,8 | 9,90 |
| H3 | 9,9 | 9,9 | 9,90 |
| H4 | 9,9 | 9,9 | 9,90 |
| H5 | 9,9 | 9,9 | 9,90 |
| H6 | 9,9 | 9,8 | 9,90 |
| H7 | 9,8 | 9,8 | 9,90 |
| H8 | 9,8 | 9,9 | 9,85 |
| H9 | 9,8 | 9,8 | 9,90 |
| H10 | 9,9 | 9,7 | 9,90 |
| Soma | 98,7 | 98,4 | 98,95 |
| Média exata | **9,87** | **9,84** | **9,895** |

**296,05 ÷ 30 = 9,868333…**. Os três avaliadores ultrapassaram individualmente
9,8 sem arredondamento, na mesma fonte. Não identificaram bloqueador funcional
nos cenários observados. Os requisitos explícitos de arte numérica distinta,
English, home, Settings com tabs, Jobs/Batches, fontes, animação, threads, erros
e checksum estão implementados. A rodada conclui o objetivo local; a nota
heurística não representa aceite visual do usuário ou certificação de produção.

A: 18 regressões próprias aprovadas, 30 PNGs inspecionadas e comparação dos inputs
do harness isolado com o freeze antes/depois. B: 50 testes TUI próprios aprovados
e harness independente de 16 renders. C: probe próprio compilado em target
isolado confirmou 1M/20M/i64::MAX × quatro tamanhos, todos os dez Events/25 Batches
alcançados via PageDown e matriz English do prompt em Home/Jobs. Os registros
originais foram preservados em `target/ux-review/numeric-12/`: `nielsen-a-tests.json`,
`nielsen-b-evaluation.json`, `nielsen-c-probe/` e `nielsen-final.json`.

Os três conferiram os 89 arquivos congelados antes/depois, sem divergência.
114 testes completos, fluxo real loopback, builds debug/release, PTYs e latência
são evidências do executor, distintas das execuções próprias acima. Nenhum
avaliador observou Ghostty nativo, API real ou operação de Keychain. Código,
assets e inputs executáveis continuam idênticos à fonte avaliada; somente estes
relatórios/ledger/plan são acrescentados depois do freeze.

| Avaliador | Recomendação restante | Severidade | Impacto na média |
| --- | --- | --- | --- |
| A | Identificar arquivos e orientar conflito de fonte instalada | Baixa | +0,01–0,02 |
| A | Perfil API compacto no formulário New Job | Baixa, opcional | +0,01–0,03 |
| A | Observar fonte/redimensionamento/fluidez no Ghostty nativo | Validação complementar | Sem aumento automático |
| B | Dica `? help` na Home | Baixa; H6/H10 | +0,01–0,03 |
| B | Observar fonte/animação/redimensionamento/interação nativos | Lacuna de evidência | Sem ganho presumido |
| C | Nenhuma recomendação funcional pendente | — | 0 |

Essas melhorias pequenas permanecem como recomendações após o limiar, não como
bloqueadores ou ganhos automaticamente somados. Não foram usados valores
sugeridos para fabricar notas. Limites de cobertura/mutação, aceitação nativa,
provider e publicação permanecem explicitados no [workflow ledger](workflow-ledger.md).

<p align="center"><img src="media/icon.png" width="128" height="128" alt="A Survivor to the Past"></p>
<h1 align="center">A Survivor to the Past</h1>
<p align="center"><i>Um survivors-like no mundo de <b>A Link to the Past</b>: resista às hordas de monstros de Hyrule.</i></p>
<p align="center"><a href="https://discord.gg/N8f62M2eaK"><img alt="Discord" src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white"></a> <a href="https://github.com/BigFatDeuce/A-Survivor-to-the-Past/releases"><img alt="version" src="https://img.shields.io/badge/alpha-0.2.0-e0a030"></a> <a href="LICENSE"><img alt="GPL-3.0" src="https://img.shields.io/badge/code-GPL--3.0-3a7bd5"></a> <img alt="Rust" src="https://img.shields.io/badge/Rust-wgpu-b7410e?logo=rust&logoColor=white"></p>
<p align="center"><a href="https://github.com/BigFatDeuce/A-Survivor-to-the-Past/releases/latest"><b>⬇ Baixar para Windows</b></a></p>
<p align="center"><a href="README.md">English</a> · <a href="README.fr.md">Français</a> · <a href="README.de.md">Deutsch</a> · <a href="README.es.md">Español</a> · <a href="README.it.md">Italiano</a> · <b>Português</b> · <a href="README.ja.md">日本語</a></p>

<p align="center"><img src="media/horde.png" alt="" width="100%"></p>

## O que é?

**A Survivor to the Past** é um fangame não oficial, feito por diversão. Ele mistura a fórmula dos *survivors-like* (suas armas atacam sozinhas, as ondas nunca param, cada nível traz novos poderes) com os sprites, músicas e sons de *The Legend of Zelda: A Link to the Past* (Super Nintendo, 1991).

Guie Link no meio de hordas cada vez maiores, colete rupias para subir de nível, escolha uma melhoria entre três, abra os baús deixados pelas elites e evolua suas armas para as formas lendárias. Um chefe aparece aos 5:00.

## Conteúdo

- **8 armas**, cada uma com sua **evolução**, e **8 itens passivos** — 6 espaços de armas e 6 de itens: cada escolha conta
- **15 tipos de inimigos** de Hyrule, **elites** que deixam baús e um **chefe** (Hinox) aos 5:00
- Ataques automáticos ou **mira manual** a qualquer momento (teclado, mouse ou controle)
- **Gráficos, músicas e sons** originais de A Link to the Past (Super Nintendo)
- **Filtros de vídeo retrô**: SNES (padrão), CRT, Scanlines, Game Boy Advance, Game Boy, além de um modo **4:3** — e **filtros de áudio** combinando
- **7 idiomas**: English, Français, Deutsch, Español, Italiano, Português, 日本語
- **Motor próprio em Rust** (wgpu): milhares de inimigos na tela, sempre fluido

## Armas e evoluções

| Arma | + Item (qualquer nível) | Evolução (arma no nível máximo) |
|---|---|---|
| Espada do guerreiro | Recipiente de coração | **Espada Mestra** |
| Arco | Aljava | **Flechas de prata** |
| Bumerangue | Botas de Pégaso | **Bumerangue mágico** |
| Bastão de fogo | Lamparina | **Medalhão Bombos** |
| Bastão de gelo | Meia magia | **Medalhão Éter** |
| Martelo mágico | Luva de poder | **Medalhão Tremor** |
| Bombas | Escudo espelhado | **Superbomba** |
| Cetro de Byrna | Rede de insetos | **Roda de fadas** |

## Imagens

<table>
<tr>
<td width="50%" align="center"><img src="media/title.png" alt="Tela de título"><br><sub>Tela de título</sub></td>
<td width="50%" align="center"><img src="media/levelup.png" alt="Subir de nível: escolha 1 de 3"><br><sub>Subir de nível: escolha 1 de 3</sub></td>
</tr>
<tr>
<td width="50%" align="center"><img src="media/evolution.png" alt="Baú de evolução"><br><sub>Baú de evolução</sub></td>
<td width="50%" align="center"><img src="media/boss.png" alt="Chefe: Hinox"><br><sub>Chefe: Hinox</sub></td>
</tr>
<tr>
<td width="50%" align="center"><img src="media/options.png" alt="Opções (idioma, filtros, volumes)"><br><sub>Opções (idioma, filtros, volumes)</sub></td>
<td width="50%" align="center"><img src="media/about.png" alt="Sobre"><br><sub>Sobre</sub></td>
</tr>
</table>

<p align="center"><img src="media/filters.png" alt="" width="100%"><br><sub>Filtros de vídeo: nenhum · CRT / Game Boy Advance · Game Boy / Scanlines · CRT em 4:3</sub></p>

## Jogar

O jogo está em **alfa** (versão 0.2.0). **Windows**: baixe a versão mais recente na [página de versões](https://github.com/BigFatDeuce/A-Survivor-to-the-Past/releases/latest), extraia o zip inteiro e abra `A Survivor to the Past.exe`. O Windows pode avisar que o jogo não é assinado: clique em *Mais informações* → *Executar assim mesmo*. Depois, o jogo **se atualiza sozinho**: ao iniciar, ele verifica no GitHub se há uma nova versão, baixa em segundo plano e instala no próximo início (Opções → Atualizações: Automáticas / Perguntar / Não). Suas configurações são mantidas e só o GitHub é contatado, sem nenhum dado pessoal.

As versões para Linux, macOS, Android e navegador virão quando o jogo estiver pronto. Enquanto isso, você pode compilá-lo a partir do código-fonte com [Rust](https://rustup.rs) (stable):

```sh
git clone https://github.com/BigFatDeuce/A-Survivor-to-the-Past.git
cd A-Survivor-to-the-Past
cargo run --release -p game
```

No Linux, instale também os pacotes de desenvolvimento do ALSA e do udev (`libasound2-dev`, `libudev-dev`).

## Controles

| Ação | Teclado / mouse | Controle |
|---|---|---|
| Mover | WASD / setas | Analógico esquerdo / direcional |
| Atacar (modo manual) | J / clique esquerdo | X / gatilho direito |
| Investida | Espaço / clique direito | A / gatilho esquerdo |
| Ataque automático ou manual | Tab / R | Y |
| Pausa | Esc / P | Start |
| Tela cheia · info · captura | F11 · F3 · F12 |  |

## Próximos passos

- Mapas de verdade (primeiro o Mundo da Luz) e mais personagens jogáveis
- Ranking online para cada mapa, com o apelido que você quiser
- Bestiário, arsenal e estatísticas
- Exportar / importar saves e atualizações automáticas
- Versões para Windows, Linux, macOS, Android e navegador

## Comunidade

É um projeto feito por diversão. Para outros projetos, compartilhar, ideias, sugestões ou bugs, venha para o **[Discord](https://discord.gg/N8f62M2eaK)**.

Site: [a-survivor-to-the-past.holy-c.men](https://a-survivor-to-the-past.holy-c.men/)

---

## Aviso legal

**A Survivor to the Past** é um **fangame não oficial, gratuito e sem fins comerciais**, feito por diversão. **É proibido vendê-lo, no todo ou em parte.**

*The Legend of Zelda*, *A Link to the Past*, seus personagens, gráficos, músicas e efeitos sonoros são marcas e obras protegidas da **Nintendo**. Todos os recursos da Nintendo usados aqui pertencem à Nintendo. Este projeto não é afiliado, patrocinado nem endossado pela Nintendo.

A [licença GPL-3.0](LICENSE) cobre apenas o código-fonte escrito para este projeto; ela não concede nenhum direito sobre os recursos da Nintendo. As folhas de sprites vêm das extrações compartilhadas pela comunidade do *The Spriters Resource* — obrigado aos seus autores.

Detentores de direitos: para remover um conteúdo, abram uma issue ou falem conosco no [Discord](https://discord.gg/N8f62M2eaK).

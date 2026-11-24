<p align="center"><img src="media/icon.png" width="128" height="128" alt="A Survivor to the Past"></p>
<h1 align="center">A Survivor to the Past</h1>
<p align="center"><i>Un survivors-like nel mondo di <b>A Link to the Past</b>: resisti alle orde di mostri di Hyrule.</i></p>
<p align="center"><a href="https://discord.gg/N8f62M2eaK"><img alt="Discord" src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white"></a> <a href="https://github.com/BigFatDeuce/A-Survivor-to-the-Past/releases"><img alt="version" src="https://img.shields.io/badge/alpha-0.2.0-e0a030"></a> <a href="LICENSE"><img alt="GPL-3.0" src="https://img.shields.io/badge/code-GPL--3.0-3a7bd5"></a> <img alt="Rust" src="https://img.shields.io/badge/Rust-wgpu-b7410e?logo=rust&logoColor=white"></p>
<p align="center"><a href="README.md">English</a> · <a href="README.fr.md">Français</a> · <a href="README.de.md">Deutsch</a> · <a href="README.es.md">Español</a> · <b>Italiano</b> · <a href="README.pt.md">Português</a> · <a href="README.ja.md">日本語</a></p>

<p align="center"><img src="media/horde.png" alt="" width="100%"></p>

## Che cos'è?

**A Survivor to the Past** è un fan game non ufficiale, fatto per divertimento. Unisce la formula dei *survivors-like* (le tue armi attaccano da sole, le ondate non finiscono mai, ogni livello porta nuovi poteri) agli sprite, alle musiche e ai suoni di *The Legend of Zelda: A Link to the Past* (Super Nintendo, 1991).

Guida Link tra orde sempre più grandi, raccogli rupie per salire di livello, scegli un potenziamento su tre, apri i forzieri lasciati dalle élite e fai evolvere le tue armi nelle loro forme leggendarie. Un boss arriva a 5:00.

## Contenuti

- **8 armi**, ognuna con la sua **evoluzione**, e **8 oggetti passivi** — 6 slot per le armi e 6 per gli oggetti: ogni scelta conta
- **15 tipi di nemici** di Hyrule, **élite** che lasciano forzieri e un **boss** (Hinox) a 5:00
- Attacchi automatici o **mira manuale** in qualsiasi momento (tastiera, mouse o controller)
- **Grafica, musiche e suoni** originali di A Link to the Past (Super Nintendo)
- **Filtri video retrò**: SNES (predefinito), CRT, Scanlines, Game Boy Advance, Game Boy, più una modalità **4:3** — e **filtri audio** abbinati
- **7 lingue**: English, Français, Deutsch, Español, Italiano, Português, 日本語
- **Motore fatto in casa in Rust** (wgpu): migliaia di nemici sullo schermo, sempre fluido

## Armi ed evoluzioni

| Arma | + Oggetto (qualsiasi livello) | Evoluzione (arma al livello massimo) |
|---|---|---|
| Spada del guerriero | Portacuore | **Spada Suprema** |
| Arco | Faretra | **Frecce d'argento** |
| Boomerang | Stivali di Pegaso | **Boomerang magico** |
| Verga di fuoco | Lanterna | **Medaglione Bombos** |
| Verga di ghiaccio | Mezza magia | **Medaglione Etere** |
| Martello magico | Guanto del potere | **Medaglione Sisma** |
| Bombe | Scudo specchio | **Superbomba** |
| Bastone di Byrna | Retino | **Girotondo di fate** |

## Immagini

<table>
<tr>
<td width="50%" align="center"><img src="media/title.png" alt="Schermata del titolo"><br><sub>Schermata del titolo</sub></td>
<td width="50%" align="center"><img src="media/levelup.png" alt="Livello superiore: scegli 1 su 3"><br><sub>Livello superiore: scegli 1 su 3</sub></td>
</tr>
<tr>
<td width="50%" align="center"><img src="media/evolution.png" alt="Forziere dell'evoluzione"><br><sub>Forziere dell'evoluzione</sub></td>
<td width="50%" align="center"><img src="media/boss.png" alt="Boss: Hinox"><br><sub>Boss: Hinox</sub></td>
</tr>
<tr>
<td width="50%" align="center"><img src="media/options.png" alt="Opzioni (lingua, filtri, volumi)"><br><sub>Opzioni (lingua, filtri, volumi)</sub></td>
<td width="50%" align="center"><img src="media/about.png" alt="Informazioni"><br><sub>Informazioni</sub></td>
</tr>
</table>

<p align="center"><img src="media/filters.png" alt="" width="100%"><br><sub>Filtri video: nessuno · CRT / Game Boy Advance · Game Boy / Scanlines · CRT in 4:3</sub></p>

## Giocare

Il gioco è in **alfa** (versione 0.2.0). Non esiste ancora una versione pubblica: i download compariranno nella [pagina delle release](https://github.com/BigFatDeuce/A-Survivor-to-the-Past/releases) — prima Windows, poi Linux, macOS, Android e browser quando il gioco sarà pronto.

Nel frattempo puoi compilarlo dai sorgenti con [Rust](https://rustup.rs) (stable):

```sh
git clone https://github.com/BigFatDeuce/A-Survivor-to-the-Past.git
cd A-Survivor-to-the-Past
cargo run --release -p game
```

Su Linux installa anche i pacchetti di sviluppo di ALSA e udev (`libasound2-dev`, `libudev-dev`).

## Comandi

| Azione | Tastiera / mouse | Controller |
|---|---|---|
| Muoversi | WASD / frecce | Levetta sinistra / croce |
| Attaccare (modalità manuale) | J / clic sinistro | X / grilletto destro |
| Scatto | Spazio / clic destro | A / grilletto sinistro |
| Attacco automatico o manuale | Tab / R | Y |
| Pausa | Esc / P | Start |
| Schermo intero · info · cattura | F11 · F3 · F12 |  |

## Tabella di marcia

- Mappe vere (prima il Mondo della Luce) e altri personaggi giocabili
- Classifica online per ogni mappa, con il nickname che preferisci
- Bestiario, armeria e statistiche
- Esportazione / importazione dei salvataggi e aggiornamenti automatici
- Versioni per Windows, Linux, macOS, Android e browser

## Comunità

È un progetto fatto per divertimento. Per altri progetti, condivisioni, idee, proposte o bug, vieni sul **[Discord](https://discord.gg/N8f62M2eaK)**.

Sito: [a-survivor-to-the-past.holy-c.men](https://a-survivor-to-the-past.holy-c.men/)

---

## Note legali

**A Survivor to the Past** è un **fan game non ufficiale, gratuito e non commerciale**, fatto per divertimento. **È vietato venderlo, in tutto o in parte.**

*The Legend of Zelda*, *A Link to the Past*, i loro personaggi, la grafica, le musiche e gli effetti sonori sono marchi e opere protette di **Nintendo**. Tutte le risorse Nintendo usate qui appartengono a Nintendo. Questo progetto non è affiliato, sponsorizzato né approvato da Nintendo.

La [licenza GPL-3.0](LICENSE) copre solo il codice sorgente scritto per questo progetto; non concede alcun diritto sulle risorse di Nintendo. Gli sprite sheet provengono dai rip condivisi dalla comunità di *The Spriters Resource* — grazie ai loro autori.

Titolari dei diritti: per far rimuovere un contenuto, aprite una issue o contattateci su [Discord](https://discord.gg/N8f62M2eaK).

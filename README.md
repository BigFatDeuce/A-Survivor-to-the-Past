<p align="center"><img src="media/icon.png" width="128" height="128" alt="A Survivor to the Past"></p>
<h1 align="center">A Survivor to the Past</h1>
<p align="center"><i>A survivors-like in the world of <b>A Link to the Past</b>: hold out against hordes of Hyrule's monsters.</i></p>
<p align="center"><a href="https://discord.gg/N8f62M2eaK"><img alt="Discord" src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white"></a> <a href="https://github.com/BigFatDeuce/A-Survivor-to-the-Past/releases"><img alt="version" src="https://img.shields.io/badge/alpha-0.3.0-e0a030"></a> <a href="LICENSE"><img alt="GPL-3.0" src="https://img.shields.io/badge/code-GPL--3.0-3a7bd5"></a> <img alt="Rust" src="https://img.shields.io/badge/Rust-wgpu-b7410e?logo=rust&logoColor=white"></p>
<p align="center"><a href="https://github.com/BigFatDeuce/A-Survivor-to-the-Past/releases/latest"><b>⬇ Download for Windows</b></a> · <a href="https://a-survivor-to-the-past.holy-c.men/play/"><b>▶ Play in your browser</b></a></p>
<p align="center"><b>English</b> · <a href="README.fr.md">Français</a> · <a href="README.de.md">Deutsch</a> · <a href="README.es.md">Español</a> · <a href="README.it.md">Italiano</a> · <a href="README.pt.md">Português</a> · <a href="README.ja.md">日本語</a></p>

<p align="center"><img src="media/horde.png" alt="" width="100%"></p>

## What is it?

**A Survivor to the Past** is an unofficial fan game made for fun. It mixes the formula of *survivors-like* games (your weapons attack on their own, the waves never stop, every level offers new powers) with the sprites, music and sounds of *The Legend of Zelda: A Link to the Past* (SNES, 1991).

Move Link through ever-growing hordes, pick up rupees to level up, choose one upgrade out of three, open the chests dropped by elites and evolve your weapons into their legendary forms. Each stage lasts about ten minutes: a mid-boss arrives at 4:30, the final boss at 9:00, and beating it opens the way out. Between runs, spend your rupees on permanent upgrades.

## Features

- **5 stages** unlocked one after the other — Hyrule Field, Lost Woods, Desert of Mystery, Frozen Lake and Pyramid of Power — each with its own weather, music, mid-boss and final boss, up to Agahnim and Ganon
- **8 weapons**, each with an **evolution**, and **8 passive items** — 6 weapon slots and 6 item slots, so every choice matters
- **15 enemy types** from Hyrule, **7 bosses** and **elites** that drop chests
- **Progression between runs**: the Kakariko shop (permanent upgrades), the Great Fairy's blessings, a **relic** won on each stage, 4 **difficulties** and Ganon's **seals** to earn more rupees
- **Profile**: bestiary, armory, statistics, and save export / import as a text code
- Automatic attacks, or **manual aiming** at any time — keyboard, mouse or gamepad, and **every key can be remapped**
- Original SNES **graphics, music and sounds** of A Link to the Past
- **Retro video filters**: SNES, CRT, Scanlines, Game Boy Advance, Game Boy, plus a **4:3** mode — and matching **audio filters**
- **7 languages**: English, Français, Deutsch, Español, Italiano, Português, 日本語
- Home-made **Rust engine** (wgpu): thousands of enemies on screen, on Windows and **in the browser**

## Weapons and evolutions

| Weapon | + Item (any level) | Evolution (weapon at max level) |
|---|---|---|
| Fighter's Sword | Heart Container | **Master Sword** |
| Bow | Quiver | **Silver Arrows** |
| Boomerang | Pegasus Boots | **Magical Boomerang** |
| Fire Rod | Lamp | **Bombos Medallion** |
| Ice Rod | Half Magic | **Ether Medallion** |
| Magic Hammer | Power Glove | **Quake Medallion** |
| Bombs | Mirror Shield | **Super Bomb** |
| Cane of Byrna | Bug-Catching Net | **Fairy Ring** |

## Screenshots

<table>
<tr>
<td width="50%" align="center"><img src="media/title.png" alt="Title screen"><br><sub>Title screen</sub></td>
<td width="50%" align="center"><img src="media/stages.png" alt="Stage select"><br><sub>Stage select</sub></td>
</tr>
<tr>
<td width="50%" align="center"><img src="media/levelup.png" alt="Level up: choose 1 of 3"><br><sub>Level up: choose 1 of 3</sub></td>
<td width="50%" align="center"><img src="media/evolution.png" alt="Evolution chest"><br><sub>Evolution chest</sub></td>
</tr>
<tr>
<td width="50%" align="center"><img src="media/boss.png" alt="Boss: Hinox"><br><sub>Boss: Hinox</sub></td>
<td width="50%" align="center"><img src="media/shop.png" alt="Kakariko shop: permanent upgrades"><br><sub>Kakariko shop: permanent upgrades</sub></td>
</tr>
<tr>
<td width="50%" align="center"><img src="media/options.png" alt="Options (language, filters, volumes)"><br><sub>Options (language, filters, volumes)</sub></td>
<td width="50%" align="center"><img src="media/about.png" alt="About"><br><sub>About</sub></td>
</tr>
</table>

<p align="center"><img src="media/filters.png" alt="" width="100%"><br><sub>Video filters: none · CRT / Game Boy Advance · Game Boy / Scanlines · CRT in 4:3</sub></p>

## Play

The game is in **early alpha** (version 0.3.0). **Windows**: download the latest version from the [Releases page](https://github.com/BigFatDeuce/A-Survivor-to-the-Past/releases/latest), extract the whole zip and launch `A Survivor to the Past.exe`. Windows may warn that the game is not signed: click *More info* → *Run anyway*. The game then **updates itself**: at launch it checks GitHub for a new version, downloads it in the background and installs it at the next launch (Options → Updates: Automatic / Ask me / Off). Your settings are kept, and only GitHub is contacted, without any personal data.

**Browser**: [play directly on the website](https://a-survivor-to-the-past.holy-c.men/play/), nothing to install. It works best on a computer, in Chrome, Edge or Firefox, and your progress is saved in the browser. Linux, macOS and Android versions will come later. Meanwhile you can build the game from source with [Rust](https://rustup.rs) (stable):

```sh
git clone https://github.com/BigFatDeuce/A-Survivor-to-the-Past.git
cd A-Survivor-to-the-Past
cargo run --release -p game
```

On Linux, also install the ALSA and udev development packages (`libasound2-dev`, `libudev-dev`).

## Controls

| Action | Keyboard / mouse | Gamepad |
|---|---|---|
| Move | WASD / arrows | Left stick / D-pad |
| Attack (manual mode) | J / left click | X / right trigger |
| Dash | Space / right click | A / left trigger |
| Auto or manual attack | Tab / R | Y |
| Pause | Esc / P | Start |
| Fullscreen · info · screenshot | F11 · F3 · F12 |  |

## Roadmap

- Real hand-made maps and more playable characters
- Online leaderboard for each stage, with a nickname of your choice
- More bosses with their own attacks and animations
- Touch controls for phones and tablets
- Builds for Linux, macOS and Android

## Community

This is a project made for fun. For other projects, sharing, ideas, suggestions or bug reports, come and talk on **[Discord](https://discord.gg/N8f62M2eaK)**.

Website: [a-survivor-to-the-past.holy-c.men](https://a-survivor-to-the-past.holy-c.men/)

---

## Legal

**A Survivor to the Past** is an **unofficial, free and non-commercial fan game**, made for fun. **Selling it, or any part of it, is forbidden.**

*The Legend of Zelda*, *A Link to the Past*, their characters, graphics, music and sound effects are trademarks and copyrighted works of **Nintendo**. All Nintendo assets used here belong to Nintendo. This project is not affiliated with, sponsored or endorsed by Nintendo.

The [GPL-3.0 license](LICENSE) only covers the source code written for this project; it grants no rights over Nintendo's assets. Sprite sheets come from the rips shared by the community of *The Spriters Resource* — thanks to their authors.

Rights holders: to have content removed, open an issue or contact us on [Discord](https://discord.gg/N8f62M2eaK).

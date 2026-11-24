<p align="center"><img src="media/icon.png" width="128" height="128" alt="A Survivor to the Past"></p>
<h1 align="center">A Survivor to the Past</h1>
<p align="center"><i>Un survivors-like en el mundo de <b>A Link to the Past</b>: resiste a las hordas de monstruos de Hyrule.</i></p>
<p align="center"><a href="https://discord.gg/N8f62M2eaK"><img alt="Discord" src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white"></a> <a href="https://github.com/BigFatDeuce/A-Survivor-to-the-Past/releases"><img alt="version" src="https://img.shields.io/badge/alpha-0.2.0-e0a030"></a> <a href="LICENSE"><img alt="GPL-3.0" src="https://img.shields.io/badge/code-GPL--3.0-3a7bd5"></a> <img alt="Rust" src="https://img.shields.io/badge/Rust-wgpu-b7410e?logo=rust&logoColor=white"></p>
<p align="center"><a href="https://github.com/BigFatDeuce/A-Survivor-to-the-Past/releases/latest"><b>⬇ Descargar para Windows</b></a></p>
<p align="center"><a href="README.md">English</a> · <a href="README.fr.md">Français</a> · <a href="README.de.md">Deutsch</a> · <b>Español</b> · <a href="README.it.md">Italiano</a> · <a href="README.pt.md">Português</a> · <a href="README.ja.md">日本語</a></p>

<p align="center"><img src="media/horde.png" alt="" width="100%"></p>

## ¿Qué es?

**A Survivor to the Past** es un fangame no oficial, hecho por diversión. Mezcla la fórmula de los *survivors-like* (tus armas atacan solas, las oleadas nunca se detienen, cada nivel trae nuevos poderes) con los sprites, la música y los sonidos de *The Legend of Zelda: A Link to the Past* (Super Nintendo, 1991).

Guía a Link entre hordas cada vez más grandes, recoge rupias para subir de nivel, elige una mejora entre tres, abre los cofres que sueltan las élites y haz evolucionar tus armas a sus formas legendarias. Un jefe aparece a los 5:00.

## Contenido

- **8 armas**, cada una con su **evolución**, y **8 objetos pasivos** — 6 huecos de armas y 6 de objetos: cada elección cuenta
- **15 tipos de enemigos** de Hyrule, **élites** que sueltan cofres y un **jefe** (Hinox) a los 5:00
- Ataques automáticos o **puntería manual** en cualquier momento (teclado, ratón o mando)
- **Gráficos, música y sonidos** originales de A Link to the Past (Super Nintendo)
- **Filtros de vídeo retro**: SNES (por defecto), CRT, Scanlines, Game Boy Advance, Game Boy, además de un modo **4:3** — y **filtros de audio** a juego
- **7 idiomas**: English, Français, Deutsch, Español, Italiano, Português, 日本語
- **Motor propio en Rust** (wgpu): miles de enemigos en pantalla, siempre fluido

## Armas y evoluciones

| Arma | + Objeto (cualquier nivel) | Evolución (arma al nivel máximo) |
|---|---|---|
| Espada de luchador | Contenedor de corazón | **Espada Maestra** |
| Arco | Carcaj | **Flechas de plata** |
| Bumerán | Botas de Pegaso | **Bumerán mágico** |
| Vara de fuego | Lámpara | **Medallón Bombos** |
| Vara de hielo | Media magia | **Medallón Éter** |
| Martillo mágico | Guante de poder | **Medallón Temblor** |
| Bombas | Escudo espejo | **Superbomba** |
| Cetro de Byrna | Red cazabichos | **Corro de hadas** |

## Capturas

<table>
<tr>
<td width="50%" align="center"><img src="media/title.png" alt="Pantalla de título"><br><sub>Pantalla de título</sub></td>
<td width="50%" align="center"><img src="media/levelup.png" alt="Subida de nivel: elige 1 de 3"><br><sub>Subida de nivel: elige 1 de 3</sub></td>
</tr>
<tr>
<td width="50%" align="center"><img src="media/evolution.png" alt="Cofre de evolución"><br><sub>Cofre de evolución</sub></td>
<td width="50%" align="center"><img src="media/boss.png" alt="Jefe: Hinox"><br><sub>Jefe: Hinox</sub></td>
</tr>
<tr>
<td width="50%" align="center"><img src="media/options.png" alt="Opciones (idioma, filtros, volumen)"><br><sub>Opciones (idioma, filtros, volumen)</sub></td>
<td width="50%" align="center"><img src="media/about.png" alt="Acerca de"><br><sub>Acerca de</sub></td>
</tr>
</table>

<p align="center"><img src="media/filters.png" alt="" width="100%"><br><sub>Filtros de vídeo: ninguno · CRT / Game Boy Advance · Game Boy / Scanlines · CRT en 4:3</sub></p>

## Jugar

El juego está en **alfa** (versión 0.2.0). **Windows**: descarga la última versión en la [página de versiones](https://github.com/BigFatDeuce/A-Survivor-to-the-Past/releases/latest), extrae todo el zip y ejecuta `A Survivor to the Past.exe`. Windows puede avisar de que el juego no está firmado: haz clic en *Más información* → *Ejecutar de todas formas*. Después, el juego **se actualiza solo**: al iniciarse comprueba en GitHub si hay una versión nueva, la descarga en segundo plano y la instala en el siguiente inicio (Opciones → Actualizaciones: Automáticas / Preguntar / No). Tus ajustes se conservan y solo se contacta con GitHub, sin ningún dato personal.

Las versiones para Linux, macOS, Android y navegador llegarán cuando el juego esté listo. Mientras tanto, puedes compilarlo desde el código fuente con [Rust](https://rustup.rs) (stable):

```sh
git clone https://github.com/BigFatDeuce/A-Survivor-to-the-Past.git
cd A-Survivor-to-the-Past
cargo run --release -p game
```

En Linux, instala también los paquetes de desarrollo de ALSA y udev (`libasound2-dev`, `libudev-dev`).

## Controles

| Acción | Teclado / ratón | Mando |
|---|---|---|
| Moverse | WASD / flechas | Stick izquierdo / cruceta |
| Atacar (modo manual) | J / clic izquierdo | X / gatillo derecho |
| Carrera | Espacio / clic derecho | A / gatillo izquierdo |
| Ataque automático o manual | Tab / R | Y |
| Pausa | Esc / P | Start |
| Pantalla completa · info · captura | F11 · F3 · F12 |  |

## Hoja de ruta

- Mapas de verdad (primero el Mundo de la Luz) y más personajes jugables
- Clasificación en línea para cada mapa, con el apodo que elijas
- Bestiario, armería y estadísticas
- Exportar / importar partidas y actualizaciones automáticas
- Versiones para Windows, Linux, macOS, Android y navegador

## Comunidad

Es un proyecto hecho por diversión. Para otros proyectos, compartir, ideas, propuestas o errores, ven al **[Discord](https://discord.gg/N8f62M2eaK)**.

Web: [a-survivor-to-the-past.holy-c.men](https://a-survivor-to-the-past.holy-c.men/)

---

## Aviso legal

**A Survivor to the Past** es un **fangame no oficial, gratuito y sin ánimo de lucro**, hecho por diversión. **Está prohibido venderlo, total o parcialmente.**

*The Legend of Zelda*, *A Link to the Past*, sus personajes, gráficos, música y efectos de sonido son marcas y obras protegidas de **Nintendo**. Todos los recursos de Nintendo usados aquí pertenecen a Nintendo. Este proyecto no está afiliado, patrocinado ni respaldado por Nintendo.

La [licencia GPL-3.0](LICENSE) solo cubre el código fuente escrito para este proyecto; no otorga ningún derecho sobre los recursos de Nintendo. Las hojas de sprites proceden de las extracciones compartidas por la comunidad de *The Spriters Resource* — gracias a sus autores.

Titulares de derechos: para retirar un contenido, abrid una issue o contactad con nosotros en [Discord](https://discord.gg/N8f62M2eaK).

"""README du depot en 7 langues (README.md = anglais, README.<lang>.md pour les autres).

Les noms d'armes et d'objets viennent des fichiers de langue du jeu (data/lang) : le README
reste coherent avec le jeu. Usage : python research/tools/make_readme.py
"""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
REPO = "https://github.com/BigFatDeuce/A-Survivor-to-the-Past"
SITE = "https://a-survivor-to-the-past.holy-c.men/"
DISCORD = "https://discord.gg/N8f62M2eaK"
VERSION = re.search(r'version\s*=\s*"([^"]+)"', (ROOT / "Cargo.toml").read_text(encoding="utf-8")).group(1)

# arme -> (objet requis, evolution), dans l'ordre de data/weapons.ron
EVOS = [("sword", "heart_container", "master_sword"), ("bow", "quiver", "silver_bow"),
        ("boomerang", "pegasus_boots", "magic_boomerang"), ("fire_rod", "lamp", "bombos"),
        ("ice_rod", "half_magic", "ether"), ("hammer", "power_glove", "quake"),
        ("bombs", "mirror_shield", "super_bomb"), ("byrna", "bug_net", "fairy_ring")]

LANGS = [("en", "English"), ("fr", "Français"), ("de", "Deutsch"), ("es", "Español"),
         ("it", "Italiano"), ("pt", "Português"), ("ja", "日本語")]

T = {
    "en": dict(
        tagline="A survivors-like in the world of <b>A Link to the Past</b>: hold out against hordes of Hyrule's monsters.",
        status="alpha",
        what_h="What is it?",
        what="""**A Survivor to the Past** is an unofficial fan game made for fun. It mixes the formula of *survivors-like* games (your weapons attack on their own, the waves never stop, every level offers new powers) with the sprites, music and sounds of *The Legend of Zelda: A Link to the Past* (SNES, 1991).

Move Link through ever-growing hordes, pick up rupees to level up, choose one upgrade out of three, open the chests dropped by elites and evolve your weapons into their legendary forms. A boss shows up at 5:00.""",
        feat_h="Features",
        feats=[
            "**8 weapons**, each with an **evolution**, and **8 passive items** — 6 weapon slots and 6 item slots, so every choice matters",
            "**15 enemy types** from Hyrule, **elites** that drop chests, and a **boss** (Hinox) at 5:00",
            "Automatic attacks, or **manual aiming** at any time (keyboard, mouse or gamepad)",
            "Original SNES **graphics, music and sounds** of A Link to the Past",
            "**Retro video filters**: SNES (default), CRT, Scanlines, Game Boy Advance, Game Boy, plus a **4:3** mode — and matching **audio filters**",
            "**7 languages**: English, Français, Deutsch, Español, Italiano, Português, 日本語",
            "Home-made **Rust engine** (wgpu): thousands of enemies on screen at a smooth frame rate",
        ],
        evo_h="Weapons and evolutions",
        evo_cols=("Weapon", "+ Item (any level)", "Evolution (weapon at max level)"),
        shots_h="Screenshots",
        caps=dict(title="Title screen", levelup="Level up: choose 1 of 3", evolution="Evolution chest",
                  boss="Boss: Hinox", options="Options (language, filters, volumes)", about="About"),
        filters_cap="Video filters: none · CRT / Game Boy Advance · Game Boy / Scanlines · CRT in 4:3",
        play_h="Play",
        play=f"""The game is in **early alpha** (version {VERSION}). **Windows**: download the latest version from the [Releases page]({REPO}/releases/latest), extract the whole zip and launch `A Survivor to the Past.exe`. Windows may warn that the game is not signed: click *More info* → *Run anyway*. The game then **updates itself**: at launch it checks GitHub for a new version, downloads it in the background and installs it at the next launch (Options → Updates: Automatic / Ask me / Off). Your settings are kept, and only GitHub is contacted, without any personal data.

Linux, macOS, Android and browser versions will come when the game is ready. Meanwhile you can build it from source with [Rust](https://rustup.rs) (stable):""",
        linux="On Linux, also install the ALSA and udev development packages (`libasound2-dev`, `libudev-dev`).",
        ctrl_h="Controls",
        ctrl_cols=("Action", "Keyboard / mouse", "Gamepad"),
        ctrl=[("Move", "WASD / arrows", "Left stick / D-pad"),
              ("Attack (manual mode)", "J / left click", "X / right trigger"),
              ("Dash", "Space / right click", "A / left trigger"),
              ("Auto or manual attack", "Tab / R", "Y"),
              ("Pause", "Esc / P", "Start"),
              ("Fullscreen · info · screenshot", "F11 · F3 · F12", "")],
        road_h="Roadmap",
        road=["Real maps (Light World first) and more playable characters",
              "Online leaderboard for each map, with a nickname of your choice",
              "Bestiary, armory and statistics",
              "Save export / import and automatic updates",
              "Builds for Windows, Linux, macOS, Android and the browser"],
        comm_h="Community",
        comm=f"""This is a project made for fun. For other projects, sharing, ideas, suggestions or bug reports, come and talk on **[Discord]({DISCORD})**.

Website: [{SITE.rstrip('/').split('//')[1]}]({SITE})""",
        legal_h="Legal",
        legal=f"""**A Survivor to the Past** is an **unofficial, free and non-commercial fan game**, made for fun. **Selling it, or any part of it, is forbidden.**

*The Legend of Zelda*, *A Link to the Past*, their characters, graphics, music and sound effects are trademarks and copyrighted works of **Nintendo**. All Nintendo assets used here belong to Nintendo. This project is not affiliated with, sponsored or endorsed by Nintendo.

The [GPL-3.0 license](LICENSE) only covers the source code written for this project; it grants no rights over Nintendo's assets. Sprite sheets come from the rips shared by the community of *The Spriters Resource* — thanks to their authors.

Rights holders: to have content removed, open an issue or contact us on [Discord]({DISCORD}).""",
    ),
    "fr": dict(
        tagline="Un survivors-like dans l'univers de <b>A Link to the Past</b> : tenez face aux hordes de monstres d'Hyrule.",
        status="alpha",
        what_h="Qu'est-ce que c'est ?",
        what="""**A Survivor to the Past** est un fan game non officiel, fait pour le plaisir. Il mélange la formule des *survivors-like* (vos armes attaquent seules, les vagues ne s'arrêtent jamais, chaque niveau apporte de nouveaux pouvoirs) avec les sprites, musiques et sons de *The Legend of Zelda: A Link to the Past* (Super Nintendo, 1991).

Guidez Link au milieu de hordes toujours plus grandes, ramassez des rubis pour monter de niveau, choisissez une amélioration parmi trois, ouvrez les coffres lâchés par les élites et faites évoluer vos armes vers leurs formes légendaires. Un boss arrive à 5:00.""",
        feat_h="Contenu",
        feats=[
            "**8 armes**, chacune avec son **évolution**, et **8 objets passifs** — 6 emplacements d'armes et 6 d'objets : chaque choix compte",
            "**15 types d'ennemis** d'Hyrule, des **élites** qui lâchent des coffres et un **boss** (Hinox) à 5:00",
            "Attaques automatiques, ou **visée manuelle** à tout moment (clavier, souris ou manette)",
            "**Graphismes, musiques et sons** d'origine de A Link to the Past (Super Nintendo)",
            "**Filtres vidéo rétro** : SNES (par défaut), CRT, Scanlines, Game Boy Advance, Game Boy, plus un mode **4:3** — et des **filtres audio** assortis",
            "**7 langues** : English, Français, Deutsch, Español, Italiano, Português, 日本語",
            "**Moteur maison en Rust** (wgpu) : des milliers d'ennemis à l'écran, toujours fluide",
        ],
        evo_h="Armes et évolutions",
        evo_cols=("Arme", "+ Objet (n'importe quel niveau)", "Évolution (arme au niveau max)"),
        shots_h="Captures d'écran",
        caps=dict(title="Écran titre", levelup="Niveau supérieur : 1 choix parmi 3", evolution="Coffre d'évolution",
                  boss="Boss : Hinox", options="Options (langue, filtres, volumes)", about="À propos"),
        filters_cap="Filtres vidéo : aucun · CRT / Game Boy Advance · Game Boy / Scanlines · CRT en 4:3",
        play_h="Jouer",
        play=f"""Le jeu est en **alpha** (version {VERSION}). **Windows** : téléchargez la dernière version sur la [page des versions]({REPO}/releases/latest), extrayez tout le zip et lancez `A Survivor to the Past.exe`. Windows peut prévenir que le jeu n'est pas signé : cliquez sur *Informations complémentaires* → *Exécuter quand même*. Ensuite, le jeu **se met à jour tout seul** : au lancement, il vérifie sur GitHub s'il existe une nouvelle version, la télécharge en arrière-plan et l'installe au lancement suivant (Options → Mises à jour : Automatiques / Me demander / Non). Vos réglages sont conservés, et seul GitHub est contacté, sans aucune donnée personnelle.

Les versions Linux, macOS, Android et navigateur viendront quand le jeu sera prêt. En attendant, vous pouvez le compiler depuis les sources avec [Rust](https://rustup.rs) (stable) :""",
        linux="Sous Linux, installez aussi les paquets de développement ALSA et udev (`libasound2-dev`, `libudev-dev`).",
        ctrl_h="Commandes",
        ctrl_cols=("Action", "Clavier / souris", "Manette"),
        ctrl=[("Se déplacer", "ZQSD / flèches", "Stick gauche / croix"),
              ("Attaquer (mode manuel)", "J / clic gauche", "X / gâchette droite"),
              ("Dash", "Espace / clic droit", "A / gâchette gauche"),
              ("Attaque auto ou manuelle", "Tab / R", "Y"),
              ("Pause", "Échap / P", "Start"),
              ("Plein écran · infos · capture", "F11 · F3 · F12", "")],
        road_h="Feuille de route",
        road=["De vraies cartes (le Monde de la Lumière d'abord) et d'autres personnages jouables",
              "Classement en ligne pour chaque carte, avec le pseudo de votre choix",
              "Bestiaire, armurerie et statistiques",
              "Export / import des sauvegardes et mises à jour automatiques",
              "Versions Windows, Linux, macOS, Android et navigateur"],
        comm_h="Communauté",
        comm=f"""C'est un projet fait pour le plaisir. Pour d'autres projets, partager, proposer des idées ou signaler des bugs, rendez-vous sur le **[Discord]({DISCORD})**.

Site : [{SITE.rstrip('/').split('//')[1]}]({SITE})""",
        legal_h="Mentions légales",
        legal=f"""**A Survivor to the Past** est un **fan game non officiel, gratuit et non commercial**, fait pour le plaisir. **Sa revente, totale ou partielle, est interdite.**

*The Legend of Zelda*, *A Link to the Past*, leurs personnages, graphismes, musiques et effets sonores sont des marques et des œuvres protégées de **Nintendo**. Tous les assets Nintendo utilisés ici appartiennent à Nintendo. Ce projet n'est ni affilié à Nintendo, ni sponsorisé, ni approuvé par Nintendo.

La [licence GPL-3.0](LICENSE) ne couvre que le code source écrit pour ce projet ; elle ne donne aucun droit sur les assets de Nintendo. Les planches de sprites proviennent des extractions partagées par la communauté de *The Spriters Resource* — merci à leurs auteurs.

Ayants droit : pour faire retirer un contenu, ouvrez une issue ou contactez-nous sur le [Discord]({DISCORD}).""",
    ),
    "de": dict(
        tagline="Ein Survivors-like in der Welt von <b>A Link to the Past</b>: Halte den Monsterhorden Hyrules stand.",
        status="alpha",
        what_h="Worum geht es?",
        what="""**A Survivor to the Past** ist ein inoffizielles Fan-Spiel, aus Spaß gemacht. Es verbindet das Prinzip der *Survivors-likes* (deine Waffen greifen von selbst an, die Wellen hören nie auf, jede Stufe bringt neue Kräfte) mit den Sprites, der Musik und den Klängen von *The Legend of Zelda: A Link to the Past* (SNES, 1991).

Steuere Link durch immer größere Horden, sammle Rubine, um aufzusteigen, wähle eine von drei Verbesserungen, öffne die Truhen der Elite-Gegner und entwickle deine Waffen zu ihren legendären Formen. Bei 5:00 erscheint ein Boss.""",
        feat_h="Inhalt",
        feats=[
            "**8 Waffen**, jede mit einer **Entwicklung**, und **8 passive Gegenstände** — 6 Waffen- und 6 Gegenstandsplätze: Jede Wahl zählt",
            "**15 Gegnertypen** aus Hyrule, **Elite-Gegner**, die Truhen fallen lassen, und ein **Boss** (Hinox) bei 5:00",
            "Automatische Angriffe oder jederzeit **manuelles Zielen** (Tastatur, Maus oder Controller)",
            "Originale **Grafik, Musik und Klänge** von A Link to the Past (SNES)",
            "**Retro-Videofilter**: SNES (Standard), CRT, Scanlines, Game Boy Advance, Game Boy, dazu ein **4:3**-Modus — und passende **Audiofilter**",
            "**7 Sprachen**: English, Français, Deutsch, Español, Italiano, Português, 日本語",
            "Eigene **Rust-Engine** (wgpu): Tausende Gegner auf dem Bildschirm, stets flüssig",
        ],
        evo_h="Waffen und Entwicklungen",
        evo_cols=("Waffe", "+ Gegenstand (beliebige Stufe)", "Entwicklung (Waffe auf Höchststufe)"),
        shots_h="Bilder",
        caps=dict(title="Titelbildschirm", levelup="Stufenaufstieg: 1 aus 3 wählen", evolution="Entwicklungstruhe",
                  boss="Boss: Hinox", options="Optionen (Sprache, Filter, Lautstärke)", about="Über das Spiel"),
        filters_cap="Videofilter: keiner · CRT / Game Boy Advance · Game Boy / Scanlines · CRT in 4:3",
        play_h="Spielen",
        play=f"""Das Spiel ist in der **Alpha-Phase** (Version {VERSION}). **Windows**: Lade die neueste Version von der [Release-Seite]({REPO}/releases/latest) herunter, entpacke das ganze Zip und starte `A Survivor to the Past.exe`. Windows warnt eventuell, dass das Spiel nicht signiert ist: Klicke auf *Weitere Informationen* → *Trotzdem ausführen*. Danach **aktualisiert sich das Spiel selbst**: Beim Start prüft es auf GitHub, ob es eine neue Version gibt, lädt sie im Hintergrund herunter und installiert sie beim nächsten Start (Optionen → Updates: Automatisch / Nachfragen / Aus). Deine Einstellungen bleiben erhalten, und es wird nur GitHub kontaktiert, ohne persönliche Daten.

Versionen für Linux, macOS, Android und Browser folgen, sobald das Spiel fertig ist. Bis dahin kannst du es mit [Rust](https://rustup.rs) (stable) aus dem Quellcode bauen:""",
        linux="Unter Linux werden außerdem die Entwicklungspakete für ALSA und udev benötigt (`libasound2-dev`, `libudev-dev`).",
        ctrl_h="Steuerung",
        ctrl_cols=("Aktion", "Tastatur / Maus", "Controller"),
        ctrl=[("Bewegen", "WASD / Pfeiltasten", "Linker Stick / Steuerkreuz"),
              ("Angreifen (manuell)", "J / Linksklick", "X / rechter Trigger"),
              ("Sprint", "Leertaste / Rechtsklick", "A / linker Trigger"),
              ("Auto- oder manueller Angriff", "Tab / R", "Y"),
              ("Pause", "Esc / P", "Start"),
              ("Vollbild · Info · Bildschirmfoto", "F11 · F3 · F12", "")],
        road_h="Fahrplan",
        road=["Echte Karten (zuerst die Lichtwelt) und weitere spielbare Figuren",
              "Online-Bestenliste für jede Karte, mit frei wählbarem Spitznamen",
              "Bestiarium, Waffenkammer und Statistiken",
              "Export / Import von Spielständen und automatische Updates",
              "Versionen für Windows, Linux, macOS, Android und Browser"],
        comm_h="Community",
        comm=f"""Dies ist ein Projekt aus Spaß. Für andere Projekte, Austausch, Ideen, Vorschläge oder Fehlerberichte: komm auf den **[Discord]({DISCORD})**.

Webseite: [{SITE.rstrip('/').split('//')[1]}]({SITE})""",
        legal_h="Rechtliches",
        legal=f"""**A Survivor to the Past** ist ein **inoffizielles, kostenloses und nicht kommerzielles Fan-Spiel**, aus Spaß gemacht. **Der Verkauf des Spiels oder von Teilen davon ist verboten.**

*The Legend of Zelda*, *A Link to the Past*, ihre Figuren, Grafiken, Musik und Soundeffekte sind Marken und urheberrechtlich geschützte Werke von **Nintendo**. Alle hier verwendeten Nintendo-Assets gehören Nintendo. Dieses Projekt steht in keiner Verbindung zu Nintendo und wird von Nintendo weder gesponsert noch unterstützt.

Die [GPL-3.0-Lizenz](LICENSE) gilt nur für den für dieses Projekt geschriebenen Quellcode; sie gewährt keinerlei Rechte an Nintendos Assets. Die Sprite-Sheets stammen aus den Rips der Community von *The Spriters Resource* — danke an ihre Autoren.

Rechteinhaber: Um Inhalte entfernen zu lassen, eröffne ein Issue oder kontaktiere uns auf [Discord]({DISCORD}).""",
    ),
    "es": dict(
        tagline="Un survivors-like en el mundo de <b>A Link to the Past</b>: resiste a las hordas de monstruos de Hyrule.",
        status="alpha",
        what_h="¿Qué es?",
        what="""**A Survivor to the Past** es un fangame no oficial, hecho por diversión. Mezcla la fórmula de los *survivors-like* (tus armas atacan solas, las oleadas nunca se detienen, cada nivel trae nuevos poderes) con los sprites, la música y los sonidos de *The Legend of Zelda: A Link to the Past* (Super Nintendo, 1991).

Guía a Link entre hordas cada vez más grandes, recoge rupias para subir de nivel, elige una mejora entre tres, abre los cofres que sueltan las élites y haz evolucionar tus armas a sus formas legendarias. Un jefe aparece a los 5:00.""",
        feat_h="Contenido",
        feats=[
            "**8 armas**, cada una con su **evolución**, y **8 objetos pasivos** — 6 huecos de armas y 6 de objetos: cada elección cuenta",
            "**15 tipos de enemigos** de Hyrule, **élites** que sueltan cofres y un **jefe** (Hinox) a los 5:00",
            "Ataques automáticos o **puntería manual** en cualquier momento (teclado, ratón o mando)",
            "**Gráficos, música y sonidos** originales de A Link to the Past (Super Nintendo)",
            "**Filtros de vídeo retro**: SNES (por defecto), CRT, Scanlines, Game Boy Advance, Game Boy, además de un modo **4:3** — y **filtros de audio** a juego",
            "**7 idiomas**: English, Français, Deutsch, Español, Italiano, Português, 日本語",
            "**Motor propio en Rust** (wgpu): miles de enemigos en pantalla, siempre fluido",
        ],
        evo_h="Armas y evoluciones",
        evo_cols=("Arma", "+ Objeto (cualquier nivel)", "Evolución (arma al nivel máximo)"),
        shots_h="Capturas",
        caps=dict(title="Pantalla de título", levelup="Subida de nivel: elige 1 de 3", evolution="Cofre de evolución",
                  boss="Jefe: Hinox", options="Opciones (idioma, filtros, volumen)", about="Acerca de"),
        filters_cap="Filtros de vídeo: ninguno · CRT / Game Boy Advance · Game Boy / Scanlines · CRT en 4:3",
        play_h="Jugar",
        play=f"""El juego está en **alfa** (versión {VERSION}). **Windows**: descarga la última versión en la [página de versiones]({REPO}/releases/latest), extrae todo el zip y ejecuta `A Survivor to the Past.exe`. Windows puede avisar de que el juego no está firmado: haz clic en *Más información* → *Ejecutar de todas formas*. Después, el juego **se actualiza solo**: al iniciarse comprueba en GitHub si hay una versión nueva, la descarga en segundo plano y la instala en el siguiente inicio (Opciones → Actualizaciones: Automáticas / Preguntar / No). Tus ajustes se conservan y solo se contacta con GitHub, sin ningún dato personal.

Las versiones para Linux, macOS, Android y navegador llegarán cuando el juego esté listo. Mientras tanto, puedes compilarlo desde el código fuente con [Rust](https://rustup.rs) (stable):""",
        linux="En Linux, instala también los paquetes de desarrollo de ALSA y udev (`libasound2-dev`, `libudev-dev`).",
        ctrl_h="Controles",
        ctrl_cols=("Acción", "Teclado / ratón", "Mando"),
        ctrl=[("Moverse", "WASD / flechas", "Stick izquierdo / cruceta"),
              ("Atacar (modo manual)", "J / clic izquierdo", "X / gatillo derecho"),
              ("Carrera", "Espacio / clic derecho", "A / gatillo izquierdo"),
              ("Ataque automático o manual", "Tab / R", "Y"),
              ("Pausa", "Esc / P", "Start"),
              ("Pantalla completa · info · captura", "F11 · F3 · F12", "")],
        road_h="Hoja de ruta",
        road=["Mapas de verdad (primero el Mundo de la Luz) y más personajes jugables",
              "Clasificación en línea para cada mapa, con el apodo que elijas",
              "Bestiario, armería y estadísticas",
              "Exportar / importar partidas y actualizaciones automáticas",
              "Versiones para Windows, Linux, macOS, Android y navegador"],
        comm_h="Comunidad",
        comm=f"""Es un proyecto hecho por diversión. Para otros proyectos, compartir, ideas, propuestas o errores, ven al **[Discord]({DISCORD})**.

Web: [{SITE.rstrip('/').split('//')[1]}]({SITE})""",
        legal_h="Aviso legal",
        legal=f"""**A Survivor to the Past** es un **fangame no oficial, gratuito y sin ánimo de lucro**, hecho por diversión. **Está prohibido venderlo, total o parcialmente.**

*The Legend of Zelda*, *A Link to the Past*, sus personajes, gráficos, música y efectos de sonido son marcas y obras protegidas de **Nintendo**. Todos los recursos de Nintendo usados aquí pertenecen a Nintendo. Este proyecto no está afiliado, patrocinado ni respaldado por Nintendo.

La [licencia GPL-3.0](LICENSE) solo cubre el código fuente escrito para este proyecto; no otorga ningún derecho sobre los recursos de Nintendo. Las hojas de sprites proceden de las extracciones compartidas por la comunidad de *The Spriters Resource* — gracias a sus autores.

Titulares de derechos: para retirar un contenido, abrid una issue o contactad con nosotros en [Discord]({DISCORD}).""",
    ),
    "it": dict(
        tagline="Un survivors-like nel mondo di <b>A Link to the Past</b>: resisti alle orde di mostri di Hyrule.",
        status="alpha",
        what_h="Che cos'è?",
        what="""**A Survivor to the Past** è un fan game non ufficiale, fatto per divertimento. Unisce la formula dei *survivors-like* (le tue armi attaccano da sole, le ondate non finiscono mai, ogni livello porta nuovi poteri) agli sprite, alle musiche e ai suoni di *The Legend of Zelda: A Link to the Past* (Super Nintendo, 1991).

Guida Link tra orde sempre più grandi, raccogli rupie per salire di livello, scegli un potenziamento su tre, apri i forzieri lasciati dalle élite e fai evolvere le tue armi nelle loro forme leggendarie. Un boss arriva a 5:00.""",
        feat_h="Contenuti",
        feats=[
            "**8 armi**, ognuna con la sua **evoluzione**, e **8 oggetti passivi** — 6 slot per le armi e 6 per gli oggetti: ogni scelta conta",
            "**15 tipi di nemici** di Hyrule, **élite** che lasciano forzieri e un **boss** (Hinox) a 5:00",
            "Attacchi automatici o **mira manuale** in qualsiasi momento (tastiera, mouse o controller)",
            "**Grafica, musiche e suoni** originali di A Link to the Past (Super Nintendo)",
            "**Filtri video retrò**: SNES (predefinito), CRT, Scanlines, Game Boy Advance, Game Boy, più una modalità **4:3** — e **filtri audio** abbinati",
            "**7 lingue**: English, Français, Deutsch, Español, Italiano, Português, 日本語",
            "**Motore fatto in casa in Rust** (wgpu): migliaia di nemici sullo schermo, sempre fluido",
        ],
        evo_h="Armi ed evoluzioni",
        evo_cols=("Arma", "+ Oggetto (qualsiasi livello)", "Evoluzione (arma al livello massimo)"),
        shots_h="Immagini",
        caps=dict(title="Schermata del titolo", levelup="Livello superiore: scegli 1 su 3", evolution="Forziere dell'evoluzione",
                  boss="Boss: Hinox", options="Opzioni (lingua, filtri, volumi)", about="Informazioni"),
        filters_cap="Filtri video: nessuno · CRT / Game Boy Advance · Game Boy / Scanlines · CRT in 4:3",
        play_h="Giocare",
        play=f"""Il gioco è in **alfa** (versione {VERSION}). **Windows**: scarica l'ultima versione dalla [pagina delle release]({REPO}/releases/latest), estrai tutto lo zip e avvia `A Survivor to the Past.exe`. Windows potrebbe avvisare che il gioco non è firmato: fai clic su *Ulteriori informazioni* → *Esegui comunque*. Poi il gioco **si aggiorna da solo**: all'avvio controlla su GitHub se c'è una nuova versione, la scarica in background e la installa all'avvio successivo (Opzioni → Aggiornamenti: Automatici / Chiedi / No). Le tue impostazioni restano e viene contattato solo GitHub, senza alcun dato personale.

Le versioni per Linux, macOS, Android e browser arriveranno quando il gioco sarà pronto. Nel frattempo puoi compilarlo dai sorgenti con [Rust](https://rustup.rs) (stable):""",
        linux="Su Linux installa anche i pacchetti di sviluppo di ALSA e udev (`libasound2-dev`, `libudev-dev`).",
        ctrl_h="Comandi",
        ctrl_cols=("Azione", "Tastiera / mouse", "Controller"),
        ctrl=[("Muoversi", "WASD / frecce", "Levetta sinistra / croce"),
              ("Attaccare (modalità manuale)", "J / clic sinistro", "X / grilletto destro"),
              ("Scatto", "Spazio / clic destro", "A / grilletto sinistro"),
              ("Attacco automatico o manuale", "Tab / R", "Y"),
              ("Pausa", "Esc / P", "Start"),
              ("Schermo intero · info · cattura", "F11 · F3 · F12", "")],
        road_h="Tabella di marcia",
        road=["Mappe vere (prima il Mondo della Luce) e altri personaggi giocabili",
              "Classifica online per ogni mappa, con il nickname che preferisci",
              "Bestiario, armeria e statistiche",
              "Esportazione / importazione dei salvataggi e aggiornamenti automatici",
              "Versioni per Windows, Linux, macOS, Android e browser"],
        comm_h="Comunità",
        comm=f"""È un progetto fatto per divertimento. Per altri progetti, condivisioni, idee, proposte o bug, vieni sul **[Discord]({DISCORD})**.

Sito: [{SITE.rstrip('/').split('//')[1]}]({SITE})""",
        legal_h="Note legali",
        legal=f"""**A Survivor to the Past** è un **fan game non ufficiale, gratuito e non commerciale**, fatto per divertimento. **È vietato venderlo, in tutto o in parte.**

*The Legend of Zelda*, *A Link to the Past*, i loro personaggi, la grafica, le musiche e gli effetti sonori sono marchi e opere protette di **Nintendo**. Tutte le risorse Nintendo usate qui appartengono a Nintendo. Questo progetto non è affiliato, sponsorizzato né approvato da Nintendo.

La [licenza GPL-3.0](LICENSE) copre solo il codice sorgente scritto per questo progetto; non concede alcun diritto sulle risorse di Nintendo. Gli sprite sheet provengono dai rip condivisi dalla comunità di *The Spriters Resource* — grazie ai loro autori.

Titolari dei diritti: per far rimuovere un contenuto, aprite una issue o contattateci su [Discord]({DISCORD}).""",
    ),
    "pt": dict(
        tagline="Um survivors-like no mundo de <b>A Link to the Past</b>: resista às hordas de monstros de Hyrule.",
        status="alpha",
        what_h="O que é?",
        what="""**A Survivor to the Past** é um fangame não oficial, feito por diversão. Ele mistura a fórmula dos *survivors-like* (suas armas atacam sozinhas, as ondas nunca param, cada nível traz novos poderes) com os sprites, músicas e sons de *The Legend of Zelda: A Link to the Past* (Super Nintendo, 1991).

Guie Link no meio de hordas cada vez maiores, colete rupias para subir de nível, escolha uma melhoria entre três, abra os baús deixados pelas elites e evolua suas armas para as formas lendárias. Um chefe aparece aos 5:00.""",
        feat_h="Conteúdo",
        feats=[
            "**8 armas**, cada uma com sua **evolução**, e **8 itens passivos** — 6 espaços de armas e 6 de itens: cada escolha conta",
            "**15 tipos de inimigos** de Hyrule, **elites** que deixam baús e um **chefe** (Hinox) aos 5:00",
            "Ataques automáticos ou **mira manual** a qualquer momento (teclado, mouse ou controle)",
            "**Gráficos, músicas e sons** originais de A Link to the Past (Super Nintendo)",
            "**Filtros de vídeo retrô**: SNES (padrão), CRT, Scanlines, Game Boy Advance, Game Boy, além de um modo **4:3** — e **filtros de áudio** combinando",
            "**7 idiomas**: English, Français, Deutsch, Español, Italiano, Português, 日本語",
            "**Motor próprio em Rust** (wgpu): milhares de inimigos na tela, sempre fluido",
        ],
        evo_h="Armas e evoluções",
        evo_cols=("Arma", "+ Item (qualquer nível)", "Evolução (arma no nível máximo)"),
        shots_h="Imagens",
        caps=dict(title="Tela de título", levelup="Subir de nível: escolha 1 de 3", evolution="Baú de evolução",
                  boss="Chefe: Hinox", options="Opções (idioma, filtros, volumes)", about="Sobre"),
        filters_cap="Filtros de vídeo: nenhum · CRT / Game Boy Advance · Game Boy / Scanlines · CRT em 4:3",
        play_h="Jogar",
        play=f"""O jogo está em **alfa** (versão {VERSION}). **Windows**: baixe a versão mais recente na [página de versões]({REPO}/releases/latest), extraia o zip inteiro e abra `A Survivor to the Past.exe`. O Windows pode avisar que o jogo não é assinado: clique em *Mais informações* → *Executar assim mesmo*. Depois, o jogo **se atualiza sozinho**: ao iniciar, ele verifica no GitHub se há uma nova versão, baixa em segundo plano e instala no próximo início (Opções → Atualizações: Automáticas / Perguntar / Não). Suas configurações são mantidas e só o GitHub é contatado, sem nenhum dado pessoal.

As versões para Linux, macOS, Android e navegador virão quando o jogo estiver pronto. Enquanto isso, você pode compilá-lo a partir do código-fonte com [Rust](https://rustup.rs) (stable):""",
        linux="No Linux, instale também os pacotes de desenvolvimento do ALSA e do udev (`libasound2-dev`, `libudev-dev`).",
        ctrl_h="Controles",
        ctrl_cols=("Ação", "Teclado / mouse", "Controle"),
        ctrl=[("Mover", "WASD / setas", "Analógico esquerdo / direcional"),
              ("Atacar (modo manual)", "J / clique esquerdo", "X / gatilho direito"),
              ("Investida", "Espaço / clique direito", "A / gatilho esquerdo"),
              ("Ataque automático ou manual", "Tab / R", "Y"),
              ("Pausa", "Esc / P", "Start"),
              ("Tela cheia · info · captura", "F11 · F3 · F12", "")],
        road_h="Próximos passos",
        road=["Mapas de verdade (primeiro o Mundo da Luz) e mais personagens jogáveis",
              "Ranking online para cada mapa, com o apelido que você quiser",
              "Bestiário, arsenal e estatísticas",
              "Exportar / importar saves e atualizações automáticas",
              "Versões para Windows, Linux, macOS, Android e navegador"],
        comm_h="Comunidade",
        comm=f"""É um projeto feito por diversão. Para outros projetos, compartilhar, ideias, sugestões ou bugs, venha para o **[Discord]({DISCORD})**.

Site: [{SITE.rstrip('/').split('//')[1]}]({SITE})""",
        legal_h="Aviso legal",
        legal=f"""**A Survivor to the Past** é um **fangame não oficial, gratuito e sem fins comerciais**, feito por diversão. **É proibido vendê-lo, no todo ou em parte.**

*The Legend of Zelda*, *A Link to the Past*, seus personagens, gráficos, músicas e efeitos sonoros são marcas e obras protegidas da **Nintendo**. Todos os recursos da Nintendo usados aqui pertencem à Nintendo. Este projeto não é afiliado, patrocinado nem endossado pela Nintendo.

A [licença GPL-3.0](LICENSE) cobre apenas o código-fonte escrito para este projeto; ela não concede nenhum direito sobre os recursos da Nintendo. As folhas de sprites vêm das extrações compartilhadas pela comunidade do *The Spriters Resource* — obrigado aos seus autores.

Detentores de direitos: para remover um conteúdo, abram uma issue ou falem conosco no [Discord]({DISCORD}).""",
    ),
    "ja": dict(
        tagline="<b>神々のトライフォース</b>の世界で生き残れ。ハイラルの魔物の大群に立ち向かうサバイバーライク。",
        status="alpha",
        what_h="どんなゲーム？",
        what="""**A Survivor to the Past** は、楽しみのために作られた非公式のファンゲームです。*サバイバーライク*のルール（武器は自動で攻撃し、敵の波は止まらず、レベルアップのたびに新しい力が手に入る）と、*ゼルダの伝説 神々のトライフォース*（スーパーファミコン、1991年）のスプライト・音楽・効果音を組み合わせています。

増え続ける大群の中でリンクを動かし、ルピーを集めてレベルアップ。3つの強化から1つを選び、エリートが落とす宝箱を開けて、武器を伝説の姿へ進化させましょう。5:00 にはボスが現れます。""",
        feat_h="特徴",
        feats=[
            "**8種類の武器**（それぞれに**進化**あり）と**8種類のパッシブアイテム** — 武器枠6・アイテム枠6なので、選択が大切",
            "ハイラルの**15種類の敵**、宝箱を落とす**エリート**、そして 5:00 に現れる**ボス**（ヒノックス）",
            "自動攻撃、またはいつでも**手動エイム**に切り替え可能（キーボード・マウス・ゲームパッド）",
            "神々のトライフォースのオリジナルの**グラフィック・音楽・効果音**",
            "**レトロ映像フィルター**：SNES（標準）、CRT、スキャンライン、ゲームボーイアドバンス、ゲームボーイ、**4:3** モード — 対応する**音声フィルター**も",
            "**7言語**：English、Français、Deutsch、Español、Italiano、Português、日本語",
            "**Rust 製の自作エンジン**（wgpu）：何千体もの敵が画面に出ても滑らか",
        ],
        evo_h="武器と進化",
        evo_cols=("武器", "＋ アイテム（レベル不問）", "進化（武器が最大レベル）"),
        shots_h="スクリーンショット",
        caps=dict(title="タイトル画面", levelup="レベルアップ：3つから1つを選ぶ", evolution="進化の宝箱",
                  boss="ボス：ヒノックス", options="オプション（言語・フィルター・音量）", about="このゲームについて"),
        filters_cap="映像フィルター：なし · CRT / ゲームボーイアドバンス · ゲームボーイ / スキャンライン · CRT（4:3）",
        play_h="遊ぶ",
        play=f"""このゲームは**アルファ版**（バージョン {VERSION}）です。**Windows**：[リリースページ]({REPO}/releases/latest)から最新版をダウンロードし、zip をすべて展開して `A Survivor to the Past.exe` を起動してください。署名がないため Windows に警告された場合は、*詳細情報* → *実行* をクリックします。その後は**自動でアップデート**されます。起動時に GitHub で新しいバージョンを確認し、バックグラウンドでダウンロードして次回起動時にインストールします（オプション → アップデート：じどう / かくにん する / オフ）。設定はそのまま残り、通信先は GitHub だけで、個人情報は送信されません。

Linux・macOS・Android・ブラウザ版はゲームの完成後に予定しています。それまでは [Rust](https://rustup.rs)（stable）でソースからビルドできます：""",
        linux="Linux では ALSA と udev の開発パッケージ（`libasound2-dev`、`libudev-dev`）も必要です。",
        ctrl_h="操作方法",
        ctrl_cols=("アクション", "キーボード / マウス", "ゲームパッド"),
        ctrl=[("移動", "WASD / 矢印キー", "左スティック / 十字キー"),
              ("攻撃（手動モード）", "J / 左クリック", "X / 右トリガー"),
              ("ダッシュ", "スペース / 右クリック", "A / 左トリガー"),
              ("自動・手動攻撃の切り替え", "Tab / R", "Y"),
              ("ポーズ", "Esc / P", "Start"),
              ("全画面 · 情報 · スクリーンショット", "F11 · F3 · F12", "")],
        road_h="今後の予定",
        road=["本格的なマップ（まずは光の世界）と新しいプレイアブルキャラクター",
              "マップごとのオンラインランキング（好きなニックネームで登録）",
              "モンスター図鑑・武器庫・統計",
              "セーブデータのエクスポート / インポートと自動アップデート",
              "Windows・Linux・macOS・Android・ブラウザ版"],
        comm_h="コミュニティ",
        comm=f"""楽しみのためのプロジェクトです。ほかの作品、共有、アイデア、提案、バグ報告は **[Discord]({DISCORD})** へどうぞ。

サイト：[{SITE.rstrip('/').split('//')[1]}]({SITE})""",
        legal_h="法的事項",
        legal=f"""**A Survivor to the Past** は、楽しみのために作られた**非公式・無料・非営利のファンゲーム**です。**本作の全部または一部の販売を禁止します。**

*ゼルダの伝説*、*神々のトライフォース*、およびそのキャラクター・グラフィック・音楽・効果音は**任天堂**の商標および著作物です。本作で使用している任天堂の素材はすべて任天堂に帰属します。本プロジェクトは任天堂とは一切関係がなく、任天堂の後援・承認を受けたものではありません。

[GPL-3.0 ライセンス](LICENSE)は本プロジェクトのために書かれたソースコードのみに適用され、任天堂の素材に関する権利を与えるものではありません。スプライトシートは *The Spriters Resource* のコミュニティが共有したものです。作者の皆さんに感謝します。

権利者の方へ：コンテンツの削除をご希望の場合は、Issue を作成するか [Discord]({DISCORD}) でご連絡ください。""",
    ),
}

DL = {
    "en": "Download for Windows",
    "fr": "Télécharger pour Windows",
    "de": "Für Windows herunterladen",
    "es": "Descargar para Windows",
    "it": "Scarica per Windows",
    "pt": "Baixar para Windows",
    "ja": "Windows 版をダウンロード",
}

PAT = re.compile(r'"([^"]+)":\s*"((?:[^"\\]|\\.)*)"')


def lang_names(code):
    d = dict(PAT.findall((ROOT / f"data/lang/{code}.ron").read_text(encoding="utf-8")))
    return lambda kind, id: d.get(f"{kind}.{id}.name", id)


def readme(code):
    t = T[code]
    name = lang_names(code)
    files = {c: ("README.md" if c == "en" else f"README.{c}.md") for c, _ in LANGS}
    langs = " · ".join(f"<b>{n}</b>" if c == code else f'<a href="{files[c]}">{n}</a>' for c, n in LANGS)
    badges = " ".join([
        f'<a href="{DISCORD}"><img alt="Discord" src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white"></a>',
        f'<a href="{REPO}/releases"><img alt="version" src="https://img.shields.io/badge/{t["status"]}-{VERSION}-e0a030"></a>',
        '<a href="LICENSE"><img alt="GPL-3.0" src="https://img.shields.io/badge/code-GPL--3.0-3a7bd5"></a>',
        '<img alt="Rust" src="https://img.shields.io/badge/Rust-wgpu-b7410e?logo=rust&logoColor=white">',
    ])
    out = [
        '<p align="center"><img src="media/icon.png" width="128" height="128" alt="A Survivor to the Past"></p>',
        '<h1 align="center">A Survivor to the Past</h1>',
        f'<p align="center"><i>{t["tagline"]}</i></p>',
        f'<p align="center">{badges}</p>',
        f'<p align="center"><a href="{REPO}/releases/latest"><b>⬇ {DL[code]}</b></a></p>',
        f'<p align="center">{langs}</p>',
        "",
        '<p align="center"><img src="media/horde.png" alt="" width="100%"></p>',
        "",
        f"## {t['what_h']}",
        "",
        t["what"],
        "",
        f"## {t['feat_h']}",
        "",
        *[f"- {f}" for f in t["feats"]],
        "",
        f"## {t['evo_h']}",
        "",
        f"| {t['evo_cols'][0]} | {t['evo_cols'][1]} | {t['evo_cols'][2]} |",
        "|---|---|---|",
        *[f"| {name('weapon', w)} | {name('passive', p)} | **{name('weapon', e)}** |" for w, p, e in EVOS],
        "",
        f"## {t['shots_h']}",
        "",
        "<table>",
    ]
    caps = t["caps"]
    for a, b in (("title", "levelup"), ("evolution", "boss"), ("options", "about")):
        out.append("<tr>")
        for k in (a, b):
            out.append(f'<td width="50%" align="center"><img src="media/{k}.png" alt="{caps[k]}"><br><sub>{caps[k]}</sub></td>')
        out.append("</tr>")
    out += [
        "</table>",
        "",
        f'<p align="center"><img src="media/filters.png" alt="" width="100%"><br><sub>{t["filters_cap"]}</sub></p>',
        "",
        f"## {t['play_h']}",
        "",
        t["play"],
        "",
        "```sh",
        f"git clone {REPO}.git",
        "cd A-Survivor-to-the-Past",
        "cargo run --release -p game",
        "```",
        "",
        t["linux"],
        "",
        f"## {t['ctrl_h']}",
        "",
        f"| {' | '.join(t['ctrl_cols'])} |",
        "|---|---|---|",
        *[f"| {a} | {k} | {p} |" for a, k, p in t["ctrl"]],
        "",
        f"## {t['road_h']}",
        "",
        *[f"- {r}" for r in t["road"]],
        "",
        f"## {t['comm_h']}",
        "",
        t["comm"],
        "",
        "---",
        "",
        f"## {t['legal_h']}",
        "",
        t["legal"],
        "",
    ]
    (ROOT / files[code]).write_text("\n".join(out), encoding="utf-8", newline="\n")
    return files[code]


if __name__ == "__main__":
    for c, _ in LANGS:
        print("ok :", readme(c))

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

# arme -> (objet requis, evolution), lu dans data/weapons.ron (ordre du fichier)
def read_evos():
    text = (ROOT / "data" / "weapons.ron").read_text(encoding="utf-8")
    out = []
    ids = list(re.finditer(r'^    "(\w+)": \(', text, re.M))
    for k, m in enumerate(ids):
        block = text[m.end(): ids[k + 1].start() if k + 1 < len(ids) else len(text)]
        e = re.search(r'evolution: Some\(\(passive: "(\w+)", into: "(\w+)"\)\)', block)
        if e:
            out.append((m.group(1), e.group(1), e.group(2)))
    return out


EVOS = read_evos()

LANGS = [("en", "English"), ("fr", "Français"), ("de", "Deutsch"), ("es", "Español"),
         ("it", "Italiano"), ("pt", "Português"), ("ja", "日本語")]

T = {
    "en": dict(
        tagline="A survivors-like in the world of <b>A Link to the Past</b>: hold out against hordes of Hyrule's monsters.",
        status="alpha",
        what_h="What is it?",
        what="""**A Survivor to the Past** is an unofficial fan game made for fun. It mixes the formula of *survivors-like* games (your weapons attack on their own, the waves never stop, every level offers new powers) with the sprites, music and sounds of *The Legend of Zelda: A Link to the Past* (SNES, 1991).

Move Link through ever-growing hordes, pick up rupees to level up, choose one upgrade out of three, open the chests dropped by elites and evolve your weapons into their legendary forms. Each stage lasts about ten minutes: a mid-boss arrives at 4:30, the final boss at 9:00, and beating it opens the way out. Between runs, spend your rupees on permanent upgrades.""",
        feat_h="Features",
        feats=[
            '**6 stages** unlocked one after the other — Hyrule Field, Lost Woods, Desert of Mystery, Frozen Lake, Pyramid of Power and the bonus Death Mountain — each with its own weather, music, mid-boss and final boss, up to Agahnim and Ganon',
            '**12 playable characters** to unlock — Link, Minish Link, Capless Link, Sonic, Princess Zelda, NES Link, Four Swords Link (4 colors), *Link\'s Awakening* Link, Adult Link, Deku Link, Shy Guy and Shadow Link (defeat Ganon) — each with strengths, weaknesses and a starting weapon',
            'A rare **Bob-omb** that blows away every enemy on screen, and **breakable scenery** hiding rupees, hearts, fairies or the Ether medallion',
            'Random **encounters**: a wandering champion waits somewhere, an arrow shows the way — beat it for a blessing, a special weapon upgrade or a relic you keep if you win',
            "A **thief Pikit** that gobbles the rupees left on the ground — catch it to get them all back",
            '**12 weapons**, each with an **evolution**, and **12 passive items** — 6 weapon slots and 6 item slots, so every choice matters',
            '**18 enemy types** from Hyrule, **7 bosses** and **elites** that drop chests',
            "**Progression between runs**: the Kakariko shop (permanent upgrades), the Great Fairy's blessings, a **relic** won on each stage, 4 **difficulties** and Ganon's **seals** to earn more rupees",
            '**Profile**: bestiary, armory, statistics, and save export / import as a text code',
            "**5 players** on the same computer, each with a name, an avatar and their own progress; a **score** for every run and **leaderboards**: best scores per stage, between the players at home and a **world top 100** per game version",
            "An **instruction booklet** in the style of the original one (PDF, English and French)",
            'Automatic attacks, or **manual aiming** at any time — keyboard, mouse or gamepad, and **every key can be remapped**',
            'Original SNES **graphics, music and sounds** of A Link to the Past',
            '**Retro video filters**: SNES, CRT, Scanlines, Game Boy Advance, Game Boy, plus a **4:3** mode — and matching **audio filters**',
            '**7 languages**: English, Français, Deutsch, Español, Italiano, Português, 日本語',
            'Home-made **Rust engine** (wgpu): thousands of enemies on screen, on Windows and **in the browser**',
        ],
        evo_h="Weapons and evolutions",
        evo_cols=("Weapon", "+ Item (any level)", "Evolution (weapon at max level)"),
        shots_h="Screenshots",
        caps=dict(chars='Character select', mountain='Death Mountain (bonus stage) and its Lynels',
                  stages='Stage select', shop='Kakariko shop: permanent upgrades',
                  title="Title screen", levelup="Level up: choose 1 of 3", evolution="Evolution chest",
                  boss="Boss: Hinox", options="Options (language, filters, volumes)", about="About"),
        filters_cap="Video filters: none · CRT / Game Boy Advance · Game Boy / Scanlines · CRT in 4:3",
        play_h="Play",
        play=f"""The game is in **early alpha** (version {VERSION}). **Windows**: download the latest version from the [Releases page]({REPO}/releases/latest), extract the whole zip and launch `A Survivor to the Past.exe`. Windows may warn that the game is not signed: click *More info* → *Run anyway*. The game then **updates itself**: at launch it checks GitHub for a new version, downloads it in the background and installs it at the next launch (Options → Updates: Automatic / Ask me / Off). Your settings are kept and no personal data is sent: updates only contact GitHub, and the world leaderboard only contacts the game's website when you open it or publish a score (nickname and score).

**Browser**: [play directly on the website]({SITE}play/), nothing to install. It works best on a computer, in Chrome, Edge or Firefox, and your progress is saved in the browser. Linux, macOS and Android versions will come later. Meanwhile you can build the game from source with [Rust](https://rustup.rs) (stable):""",
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
        road=[
            'Real hand-made maps and more playable characters',
            'More bosses with their own attacks and animations',
            'Touch controls for phones and tablets',
            'Builds for Linux, macOS and Android',
        ],
        comm_h="Community",
        comm=f"""This is a project made for fun. For other projects, sharing, ideas, suggestions or bug reports, come and talk on **[Discord]({DISCORD})**.

Website: [{SITE.rstrip('/').split('//')[1]}]({SITE})""",
        legal_h="Legal",
        legal=f"""**A Survivor to the Past** is an **unofficial, free and non-commercial fan game**, made for fun. **Selling it, or any part of it, is forbidden.**

*The Legend of Zelda*, *A Link to the Past*, *The Minish Cap*, their characters, graphics, music and sound effects are trademarks and copyrighted works of **Nintendo**. All Nintendo assets used here belong to Nintendo. This project is not affiliated with, sponsored or endorsed by Nintendo.

The [GPL-3.0 license](LICENSE) only covers the source code written for this project; it grants no rights over Nintendo's assets. Sprite sheets come from the rips shared by the community of *The Spriters Resource* — thanks to their authors.

The name of every sprite ripper and artist (Barack Obama, Mister Man, Nemau, Aztec Warrior, Deathbringer, Barubary, Vaati, Mister Mike, Ragnarockerbunny and many more) is listed in **[CREDITS.md](CREDITS.md)**. Shy Guy also belongs to Nintendo; *Sonic the Hedgehog* is a trademark of **SEGA**.

Rights holders: to have content removed, open an issue or contact us on [Discord]({DISCORD}).""",
    ),
    "fr": dict(
        tagline="Un survivors-like dans l'univers de <b>A Link to the Past</b> : tenez face aux hordes de monstres d'Hyrule.",
        status="alpha",
        what_h="Qu'est-ce que c'est ?",
        what="""**A Survivor to the Past** est un fan game non officiel, fait pour le plaisir. Il mélange la formule des *survivors-like* (vos armes attaquent seules, les vagues ne s'arrêtent jamais, chaque niveau apporte de nouveaux pouvoirs) avec les sprites, musiques et sons de *The Legend of Zelda: A Link to the Past* (Super Nintendo, 1991).

Guidez Link au milieu de hordes toujours plus grandes, ramassez des rubis pour monter de niveau, choisissez une amélioration parmi trois, ouvrez les coffres lâchés par les élites et faites évoluer vos armes vers leurs formes légendaires. Chaque stage dure une dizaine de minutes : un boss intermédiaire arrive à 4:30, le boss final à 9:00, et le vaincre ouvre la sortie. Entre deux parties, dépensez vos rubis en améliorations permanentes.""",
        feat_h="Contenu",
        feats=[
            "**6 stages** à débloquer l'un après l'autre — Plaine d'Hyrule, Bois Perdus, Désert du Mystère, Lac gelé, Pyramide de la Puissance et la Montagne de la Mort en bonus — chacun avec sa météo, sa musique, son boss intermédiaire et son boss final, jusqu'à Agahnim et Ganon",
            '**12 personnages jouables** à débloquer — Link, Link Minish, Link sans bonnet, Sonic, la princesse Zelda, le Link de la NES, le Link de *Four Swords* (4 couleurs), celui de *Link\'s Awakening*, Link adulte, Link Mojo, Maskass et Shadow Link (en battant Ganon) — chacun avec ses forces, ses faiblesses et son arme de départ',
            "Une **Bob-omb** rare qui souffle tous les ennemis à l'écran, et un **décor cassable** qui cache rubis, cœurs, fées ou médaillon d'Éther",
            "Des **rencontres** aléatoires : un champion errant attend quelque part, une flèche indique où aller — battez-le pour une bénédiction, une amélioration spéciale d'arme ou une relique gardée si vous gagnez",
            "Un **Pikit voleur** qui dévore les rubis restés au sol — rattrapez-le pour tout récupérer",
            "**12 armes**, chacune avec une **évolution**, et **12 objets passifs** — 6 emplacements d'armes et 6 d'objets : chaque choix compte",
            "**18 types d'ennemis** d'Hyrule, **7 boss** et des **élites** qui lâchent des coffres",
            '**Progression entre les parties** : la boutique de Cocorico (améliorations permanentes), les bénédictions de la Grande Fée, une **relique** gagnée sur chaque stage, 4 **difficultés** et les **sceaux** de Ganon pour gagner plus de rubis',
            '**Profil** : bestiaire, armurerie, statistiques, et export / import de la sauvegarde sous forme de code texte',
            "**5 joueurs** sur le même ordinateur, chacun avec son nom, son avatar et sa progression ; un **score** à chaque partie et des **classements** : meilleurs scores par stage, entre les joueurs de la maison et **top 100 mondial** par version du jeu",
            "Un **livret d'instructions** à la manière de celui d'origine (PDF, français et anglais)",
            'Attaques automatiques, ou **visée manuelle** à tout moment — clavier, souris ou manette, et **toutes les touches sont reconfigurables**',
            '**Graphismes, musiques et sons** originaux de A Link to the Past sur SNES',
            '**Filtres vidéo rétro** : SNES, CRT, Scanlines, Game Boy Advance, Game Boy, plus un mode **4:3** — et des **filtres audio** assortis',
            '**7 langues** : English, Français, Deutsch, Español, Italiano, Português, 日本語',
            "**Moteur maison en Rust** (wgpu) : des milliers d'ennemis à l'écran, sur Windows et **dans le navigateur**",
        ],
        evo_h="Armes et évolutions",
        evo_cols=("Arme", "+ Objet (n'importe quel niveau)", "Évolution (arme au niveau max)"),
        shots_h="Captures d'écran",
        caps=dict(chars='Choix du personnage', mountain='Montagne de la Mort (stage bonus) et ses Lynels',
                  stages='Choix du stage', shop='Boutique de Cocorico : améliorations permanentes',
                  title="Écran titre", levelup="Niveau supérieur : 1 choix parmi 3", evolution="Coffre d'évolution",
                  boss="Boss : Hinox", options="Options (langue, filtres, volumes)", about="À propos"),
        filters_cap="Filtres vidéo : aucun · CRT / Game Boy Advance · Game Boy / Scanlines · CRT en 4:3",
        play_h="Jouer",
        play=f"""Le jeu est en **alpha** (version {VERSION}). **Windows** : téléchargez la dernière version sur la [page des versions]({REPO}/releases/latest), extrayez tout le zip et lancez `A Survivor to the Past.exe`. Windows peut prévenir que le jeu n'est pas signé : cliquez sur *Informations complémentaires* → *Exécuter quand même*. Ensuite, le jeu **se met à jour tout seul** : au lancement, il vérifie sur GitHub s'il existe une nouvelle version, la télécharge en arrière-plan et l'installe au lancement suivant (Options → Mises à jour : Automatiques / Me demander / Non). Vos réglages sont conservés et aucune donnée personnelle n'est envoyée : les mises à jour ne contactent que GitHub, et le classement mondial ne contacte le site du jeu que lorsque vous l'ouvrez ou publiez un score (pseudo et score).

**Navigateur** : [jouez directement sur le site]({SITE}play/), sans rien installer. Idéal sur ordinateur, dans Chrome, Edge ou Firefox ; votre progression est enregistrée dans le navigateur. Les versions Linux, macOS et Android viendront plus tard. En attendant, vous pouvez compiler le jeu depuis les sources avec [Rust](https://rustup.rs) (stable) :""",
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
        road=[
            "De vraies cartes dessinées à la main et d'autres personnages jouables",
            "D'autres boss avec leurs propres attaques et animations",
            'Commandes tactiles pour téléphones et tablettes',
            'Versions Linux, macOS et Android',
        ],
        comm_h="Communauté",
        comm=f"""C'est un projet fait pour le plaisir. Pour d'autres projets, partager, proposer des idées ou signaler des bugs, rendez-vous sur le **[Discord]({DISCORD})**.

Site : [{SITE.rstrip('/').split('//')[1]}]({SITE})""",
        legal_h="Mentions légales",
        legal=f"""**A Survivor to the Past** est un **fan game non officiel, gratuit et non commercial**, fait pour le plaisir. **Sa revente, totale ou partielle, est interdite.**

*The Legend of Zelda*, *A Link to the Past*, *The Minish Cap*, leurs personnages, graphismes, musiques et effets sonores sont des marques et des œuvres protégées de **Nintendo**. Tous les assets Nintendo utilisés ici appartiennent à Nintendo. Ce projet n'est ni affilié à Nintendo, ni sponsorisé, ni approuvé par Nintendo.

La [licence GPL-3.0](LICENSE) ne couvre que le code source écrit pour ce projet ; elle ne donne aucun droit sur les assets de Nintendo. Les planches de sprites proviennent des extractions partagées par la communauté de *The Spriters Resource* — merci à leurs auteurs.

Le nom de chaque auteur d'extraction ou de sprite (Barack Obama, Mister Man, Nemau, Aztec Warrior, Deathbringer, Barubary, Vaati, Mister Mike, Ragnarockerbunny et bien d'autres) figure dans **[CREDITS.md](CREDITS.md)**. Maskass appartient lui aussi à Nintendo ; *Sonic the Hedgehog* est une marque de **SEGA**.

Ayants droit : pour faire retirer un contenu, ouvrez une issue ou contactez-nous sur le [Discord]({DISCORD}).""",
    ),
    "de": dict(
        tagline="Ein Survivors-like in der Welt von <b>A Link to the Past</b>: Halte den Monsterhorden Hyrules stand.",
        status="alpha",
        what_h="Worum geht es?",
        what="""**A Survivor to the Past** ist ein inoffizielles Fan-Spiel, aus Spaß gemacht. Es verbindet das Prinzip der *Survivors-likes* (deine Waffen greifen von selbst an, die Wellen hören nie auf, jede Stufe bringt neue Kräfte) mit den Sprites, der Musik und den Klängen von *The Legend of Zelda: A Link to the Past* (SNES, 1991).

Steuere Link durch immer größere Horden, sammle Rubine, um aufzusteigen, wähle eine von drei Verbesserungen, öffne die Truhen der Elite-Gegner und entwickle deine Waffen zu ihren legendären Formen. Jede Stage dauert etwa zehn Minuten: Bei 4:30 kommt ein Zwischenboss, bei 9:00 der Endboss, und sein Sieg öffnet den Ausgang. Zwischen den Runden gibst du deine Rubine für dauerhafte Verbesserungen aus.""",
        feat_h="Inhalt",
        feats=[
            '**6 Stages**, die nacheinander freigeschaltet werden — Hyrule-Ebene, Verlorene Wälder, Wüste der Mysterien, Gefrorener See, Pyramide der Macht und der Bonus-Stage Todesberg — jede mit eigenem Wetter, eigener Musik, Zwischenboss und Endboss, bis zu Agahnim und Ganon',
            '**12 spielbare Figuren** zum Freischalten — Link, Minish-Link, Link ohne Mütze, Sonic, Prinzessin Zelda, NES-Link, Four-Swords-Link (4 Farben), der Link aus *Link\'s Awakening*, Erwachsener Link, Deku-Link, Shy Guy und Shadow Link (Ganon besiegen) — jede mit Stärken, Schwächen und eigener Startwaffe',
            'Ein seltener **Bob-omb**, der alle Gegner auf dem Bildschirm wegsprengt, und **zerstörbare Umgebung** mit Rubinen, Herzen, Feen oder dem Äther-Medaillon',
            'Zufällige **Begegnungen**: Ein wandernder Champion wartet irgendwo, ein Pfeil zeigt den Weg — besiege ihn für einen Segen, eine Spezialverbesserung oder ein Relikt, das bleibt, wenn du gewinnst',
            "Ein **diebischer Pikit**, der die Rubine am Boden frisst — fang ihn, um alle zurückzubekommen",
            '**12 Waffen**, jede mit einer **Entwicklung**, und **12 passive Gegenstände** — 6 Waffen- und 6 Gegenstandsplätze, jede Wahl zählt',
            '**18 Gegnertypen** aus Hyrule, **7 Bosse** und **Elite-Gegner**, die Truhen fallen lassen',
            '**Fortschritt zwischen den Runden**: der Laden von Kakariko (dauerhafte Verbesserungen), die Segen der Großen Fee, eine **Reliquie** pro Stage, 4 **Schwierigkeitsgrade** und Ganons **Siegel** für mehr Rubine',
            '**Profil**: Bestiarium, Waffenkammer, Statistiken sowie Export / Import des Spielstands als Textcode',
            "**5 Spieler** auf demselben Computer, jeder mit Namen, Avatar und eigenem Fortschritt; eine **Punktzahl** für jede Runde und **Ranglisten**: Bestwerte pro Stage, unter den Spielern zu Hause und eine **Welt-Top-100** pro Spielversion",
            "Eine **Spielanleitung** im Stil des Originalhefts (PDF, Englisch und Französisch)",
            'Automatische Angriffe oder jederzeit **manuelles Zielen** — Tastatur, Maus oder Controller, und **alle Tasten sind frei belegbar**',
            'Originale SNES-**Grafiken, Musik und Sounds** aus A Link to the Past',
            '**Retro-Videofilter**: SNES, CRT, Scanlines, Game Boy Advance, Game Boy, dazu ein **4:3**-Modus — und passende **Audiofilter**',
            '**7 Sprachen**: English, Français, Deutsch, Español, Italiano, Português, 日本語',
            'Eigene **Rust-Engine** (wgpu): Tausende Gegner auf dem Bildschirm, unter Windows und **im Browser**',
        ],
        evo_h="Waffen und Entwicklungen",
        evo_cols=("Waffe", "+ Gegenstand (beliebige Stufe)", "Entwicklung (Waffe auf Höchststufe)"),
        shots_h="Bilder",
        caps=dict(chars='Figurenauswahl', mountain='Todesberg (Bonus-Stage) und seine Leunen',
                  stages='Stage-Auswahl', shop='Laden von Kakariko: dauerhafte Verbesserungen',
                  title="Titelbildschirm", levelup="Stufenaufstieg: 1 aus 3 wählen", evolution="Entwicklungstruhe",
                  boss="Boss: Hinox", options="Optionen (Sprache, Filter, Lautstärke)", about="Über das Spiel"),
        filters_cap="Videofilter: keiner · CRT / Game Boy Advance · Game Boy / Scanlines · CRT in 4:3",
        play_h="Spielen",
        play=f"""Das Spiel ist in der **Alpha-Phase** (Version {VERSION}). **Windows**: Lade die neueste Version von der [Release-Seite]({REPO}/releases/latest) herunter, entpacke das ganze Zip und starte `A Survivor to the Past.exe`. Windows warnt eventuell, dass das Spiel nicht signiert ist: Klicke auf *Weitere Informationen* → *Trotzdem ausführen*. Danach **aktualisiert sich das Spiel selbst**: Beim Start prüft es auf GitHub, ob es eine neue Version gibt, lädt sie im Hintergrund herunter und installiert sie beim nächsten Start (Optionen → Updates: Automatisch / Nachfragen / Aus). Deine Einstellungen bleiben erhalten und es werden keine persönlichen Daten gesendet: Updates kontaktieren nur GitHub, und die Weltrangliste kontaktiert die Website des Spiels nur, wenn du sie öffnest oder eine Punktzahl veröffentlichst (Spitzname und Punktzahl).

**Browser**: [direkt auf der Website spielen]({SITE}play/), ohne Installation. Am besten am Computer, in Chrome, Edge oder Firefox; dein Fortschritt wird im Browser gespeichert. Versionen für Linux, macOS und Android folgen später. Bis dahin kannst du das Spiel mit [Rust](https://rustup.rs) (stable) aus dem Quellcode bauen:""",
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
        road=[
            'Echte, handgemachte Karten und weitere spielbare Figuren',
            'Weitere Bosse mit eigenen Angriffen und Animationen',
            'Touch-Steuerung für Smartphones und Tablets',
            'Versionen für Linux, macOS und Android',
        ],
        comm_h="Community",
        comm=f"""Dies ist ein Projekt aus Spaß. Für andere Projekte, Austausch, Ideen, Vorschläge oder Fehlerberichte: komm auf den **[Discord]({DISCORD})**.

Webseite: [{SITE.rstrip('/').split('//')[1]}]({SITE})""",
        legal_h="Rechtliches",
        legal=f"""**A Survivor to the Past** ist ein **inoffizielles, kostenloses und nicht kommerzielles Fan-Spiel**, aus Spaß gemacht. **Der Verkauf des Spiels oder von Teilen davon ist verboten.**

*The Legend of Zelda*, *A Link to the Past*, *The Minish Cap*, ihre Figuren, Grafiken, Musik und Soundeffekte sind Marken und urheberrechtlich geschützte Werke von **Nintendo**. Alle hier verwendeten Nintendo-Assets gehören Nintendo. Dieses Projekt steht in keiner Verbindung zu Nintendo und wird von Nintendo weder gesponsert noch unterstützt.

Die [GPL-3.0-Lizenz](LICENSE) gilt nur für den für dieses Projekt geschriebenen Quellcode; sie gewährt keinerlei Rechte an Nintendos Assets. Die Sprite-Sheets stammen aus den Rips der Community von *The Spriters Resource* — danke an ihre Autoren.

Die Namen aller Sprite-Ripper und Künstler (Barack Obama, Mister Man, Nemau, Aztec Warrior, Deathbringer, Barubary, Vaati, Mister Mike, Ragnarockerbunny und viele mehr) stehen in **[CREDITS.md](CREDITS.md)**. Shy Guy gehört ebenfalls Nintendo; *Sonic the Hedgehog* ist eine Marke von **SEGA**.

Rechteinhaber: Um Inhalte entfernen zu lassen, eröffne ein Issue oder kontaktiere uns auf [Discord]({DISCORD}).""",
    ),
    "es": dict(
        tagline="Un survivors-like en el mundo de <b>A Link to the Past</b>: resiste a las hordas de monstruos de Hyrule.",
        status="alpha",
        what_h="¿Qué es?",
        what="""**A Survivor to the Past** es un fangame no oficial, hecho por diversión. Mezcla la fórmula de los *survivors-like* (tus armas atacan solas, las oleadas nunca se detienen, cada nivel trae nuevos poderes) con los sprites, la música y los sonidos de *The Legend of Zelda: A Link to the Past* (Super Nintendo, 1991).

Guía a Link entre hordas cada vez más grandes, recoge rupias para subir de nivel, elige una mejora entre tres, abre los cofres que sueltan las élites y haz evolucionar tus armas a sus formas legendarias. Cada fase dura unos diez minutos: un jefe intermedio llega a los 4:30, el jefe final a los 9:00, y vencerlo abre la salida. Entre partidas, gasta tus rupias en mejoras permanentes.""",
        feat_h="Contenido",
        feats=[
            '**6 fases** que se desbloquean una tras otra — Llanura de Hyrule, Bosque Perdido, Desierto del Misterio, Lago helado, Pirámide del Poder y la Montaña de la Muerte como extra — cada una con su clima, su música, su jefe intermedio y su jefe final, hasta Agahnim y Ganon',
            '**12 personajes jugables** por desbloquear — Link, Link Minish, Link sin gorro, Sonic, la princesa Zelda, el Link de NES, el Link de *Four Swords* (4 colores), el de *Link\'s Awakening*, Link adulto, Link Deku, Shy Guy y Shadow Link (derrotando a Ganon) — cada uno con sus puntos fuertes, sus debilidades y su arma inicial',
            'Una **Bob-omb** rara que barre a todos los enemigos de la pantalla, y **decorado destructible** que esconde rupias, corazones, hadas o el medallón de Éter',
            '**Encuentros** aleatorios: un campeón errante espera en algún lugar y una flecha indica el camino — derrótalo para obtener una bendición, una mejora especial de arma o una reliquia que conservas si ganas',
            "Un **Pikit ladrón** que devora las rupias del suelo — atrápalo para recuperarlas todas",
            '**12 armas**, cada una con una **evolución**, y **12 objetos pasivos** — 6 huecos de armas y 6 de objetos: cada elección cuenta',
            '**18 tipos de enemigos** de Hyrule, **7 jefes** y **élites** que sueltan cofres',
            '**Progresión entre partidas**: la tienda de Kakariko (mejoras permanentes), las bendiciones de la Gran Hada, una **reliquia** por fase, 4 **dificultades** y los **sellos** de Ganon para ganar más rupias',
            '**Perfil**: bestiario, armería, estadísticas, y exportar / importar la partida como código de texto',
            "**5 jugadores** en el mismo ordenador, cada uno con su nombre, su avatar y su progreso; una **puntuación** en cada partida y **clasificaciones**: mejores puntuaciones por fase, entre los jugadores de casa y un **top 100 mundial** por versión del juego",
            "Un **manual de instrucciones** al estilo del original (PDF, inglés y francés)",
            'Ataques automáticos o **apuntado manual** en cualquier momento — teclado, ratón o mando, y **todas las teclas se pueden reasignar**',
            '**Gráficos, música y sonidos** originales de A Link to the Past en SNES',
            '**Filtros de vídeo retro**: SNES, CRT, Scanlines, Game Boy Advance, Game Boy, más un modo **4:3** — y **filtros de audio** a juego',
            '**7 idiomas**: English, Français, Deutsch, Español, Italiano, Português, 日本語',
            '**Motor propio en Rust** (wgpu): miles de enemigos en pantalla, en Windows y **en el navegador**',
        ],
        evo_h="Armas y evoluciones",
        evo_cols=("Arma", "+ Objeto (cualquier nivel)", "Evolución (arma al nivel máximo)"),
        shots_h="Capturas",
        caps=dict(chars='Selección de personaje', mountain='Montaña de la Muerte (fase extra) y sus Lynels',
                  stages='Selección de fase', shop='Tienda de Kakariko: mejoras permanentes',
                  title="Pantalla de título", levelup="Subida de nivel: elige 1 de 3", evolution="Cofre de evolución",
                  boss="Jefe: Hinox", options="Opciones (idioma, filtros, volumen)", about="Acerca de"),
        filters_cap="Filtros de vídeo: ninguno · CRT / Game Boy Advance · Game Boy / Scanlines · CRT en 4:3",
        play_h="Jugar",
        play=f"""El juego está en **alfa** (versión {VERSION}). **Windows**: descarga la última versión en la [página de versiones]({REPO}/releases/latest), extrae todo el zip y ejecuta `A Survivor to the Past.exe`. Windows puede avisar de que el juego no está firmado: haz clic en *Más información* → *Ejecutar de todas formas*. Después, el juego **se actualiza solo**: al iniciarse comprueba en GitHub si hay una versión nueva, la descarga en segundo plano y la instala en el siguiente inicio (Opciones → Actualizaciones: Automáticas / Preguntar / No). Tus ajustes se conservan y no se envía ningún dato personal: las actualizaciones solo contactan con GitHub, y la clasificación mundial solo contacta con la web del juego cuando la abres o publicas una puntuación (apodo y puntuación).

**Navegador**: [juega directamente en la web]({SITE}play/), sin instalar nada. Funciona mejor en ordenador, en Chrome, Edge o Firefox, y tu progreso se guarda en el navegador. Las versiones para Linux, macOS y Android llegarán más adelante. Mientras tanto, puedes compilar el juego desde el código fuente con [Rust](https://rustup.rs) (stable):""",
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
        road=[
            'Mapas de verdad hechos a mano y más personajes jugables',
            'Más jefes con sus propios ataques y animaciones',
            'Controles táctiles para móviles y tabletas',
            'Versiones para Linux, macOS y Android',
        ],
        comm_h="Comunidad",
        comm=f"""Es un proyecto hecho por diversión. Para otros proyectos, compartir, ideas, propuestas o errores, ven al **[Discord]({DISCORD})**.

Web: [{SITE.rstrip('/').split('//')[1]}]({SITE})""",
        legal_h="Aviso legal",
        legal=f"""**A Survivor to the Past** es un **fangame no oficial, gratuito y sin ánimo de lucro**, hecho por diversión. **Está prohibido venderlo, total o parcialmente.**

*The Legend of Zelda*, *A Link to the Past*, *The Minish Cap*, sus personajes, gráficos, música y efectos de sonido son marcas y obras protegidas de **Nintendo**. Todos los recursos de Nintendo usados aquí pertenecen a Nintendo. Este proyecto no está afiliado, patrocinado ni respaldado por Nintendo.

La [licencia GPL-3.0](LICENSE) solo cubre el código fuente escrito para este proyecto; no otorga ningún derecho sobre los recursos de Nintendo. Las hojas de sprites proceden de las extracciones compartidas por la comunidad de *The Spriters Resource* — gracias a sus autores.

El nombre de cada autor de extracciones o sprites (Barack Obama, Mister Man, Nemau, Aztec Warrior, Deathbringer, Barubary, Vaati, Mister Mike, Ragnarockerbunny y muchos más) aparece en **[CREDITS.md](CREDITS.md)**. Shy Guy también pertenece a Nintendo; *Sonic the Hedgehog* es una marca de **SEGA**.

Titulares de derechos: para retirar un contenido, abrid una issue o contactad con nosotros en [Discord]({DISCORD}).""",
    ),
    "it": dict(
        tagline="Un survivors-like nel mondo di <b>A Link to the Past</b>: resisti alle orde di mostri di Hyrule.",
        status="alpha",
        what_h="Che cos'è?",
        what="""**A Survivor to the Past** è un fan game non ufficiale, fatto per divertimento. Unisce la formula dei *survivors-like* (le tue armi attaccano da sole, le ondate non finiscono mai, ogni livello porta nuovi poteri) agli sprite, alle musiche e ai suoni di *The Legend of Zelda: A Link to the Past* (Super Nintendo, 1991).

Guida Link tra orde sempre più grandi, raccogli rupie per salire di livello, scegli un potenziamento su tre, apri i forzieri lasciati dalle élite e fai evolvere le tue armi nelle loro forme leggendarie. Ogni stage dura una decina di minuti: un boss intermedio arriva a 4:30, il boss finale a 9:00, e sconfiggerlo apre l'uscita. Tra una partita e l'altra, spendi le rupie in potenziamenti permanenti.""",
        feat_h="Contenuti",
        feats=[
            "**6 stage** da sbloccare uno dopo l'altro — Piana di Hyrule, Boschi Perduti, Deserto del Mistero, Lago ghiacciato, Piramide del Potere e il Monte Morte come bonus — ognuno con il suo meteo, la sua musica, un boss intermedio e un boss finale, fino ad Agahnim e Ganon",
            '**12 personaggi giocabili** da sbloccare — Link, Link Minish, Link senza berretto, Sonic, la principessa Zelda, il Link del NES, il Link di *Four Swords* (4 colori), quello di *Link\'s Awakening*, Link adulto, Link Deku, Tipo Timido e Shadow Link (sconfiggendo Ganon) — ognuno con punti di forza, debolezze e la sua arma iniziale',
            "Una **Bob-omb** rara che spazza via tutti i nemici sullo schermo, e **scenari distruttibili** che nascondono rupie, cuori, fate o il medaglione dell'Etere",
            '**Incontri** casuali: un campione errante aspetta da qualche parte, una freccia indica la strada — sconfiggilo per una benedizione, un potenziamento speciale o una reliquia che resta se vinci',
            "Un **Pikit ladro** che divora le rupie rimaste a terra — prendilo per recuperarle tutte",
            "**12 armi**, ognuna con un'**evoluzione**, e **12 oggetti passivi** — 6 slot per le armi e 6 per gli oggetti: ogni scelta conta",
            '**18 tipi di nemici** di Hyrule, **7 boss** ed **élite** che lasciano forzieri',
            '**Progressi tra le partite**: il negozio di Kakariko (potenziamenti permanenti), le benedizioni della Grande Fata, una **reliquia** per ogni stage, 4 **difficoltà** e i **sigilli** di Ganon per guadagnare più rupie',
            '**Profilo**: bestiario, armeria, statistiche, ed esportazione / importazione del salvataggio come codice di testo',
            "**5 giocatori** sullo stesso computer, ognuno con nome, avatar e progressi propri; un **punteggio** per ogni partita e **classifiche**: migliori punteggi per livello, tra i giocatori di casa e una **top 100 mondiale** per versione del gioco",
            "Un **libretto di istruzioni** nello stile dell'originale (PDF, inglese e francese)",
            'Attacchi automatici o **mira manuale** in qualsiasi momento — tastiera, mouse o controller, e **tutti i tasti sono riassegnabili**',
            '**Grafica, musica e suoni** originali di A Link to the Past per SNES',
            '**Filtri video retrò**: SNES, CRT, Scanlines, Game Boy Advance, Game Boy, più una modalità **4:3** — e **filtri audio** abbinati',
            '**7 lingue**: English, Français, Deutsch, Español, Italiano, Português, 日本語',
            '**Motore fatto in casa in Rust** (wgpu): migliaia di nemici sullo schermo, su Windows e **nel browser**',
        ],
        evo_h="Armi ed evoluzioni",
        evo_cols=("Arma", "+ Oggetto (qualsiasi livello)", "Evoluzione (arma al livello massimo)"),
        shots_h="Immagini",
        caps=dict(chars='Scelta del personaggio', mountain='Monte Morte (stage bonus) e i suoi Lynel',
                  stages='Scelta dello stage', shop='Negozio di Kakariko: potenziamenti permanenti',
                  title="Schermata del titolo", levelup="Livello superiore: scegli 1 su 3", evolution="Forziere dell'evoluzione",
                  boss="Boss: Hinox", options="Opzioni (lingua, filtri, volumi)", about="Informazioni"),
        filters_cap="Filtri video: nessuno · CRT / Game Boy Advance · Game Boy / Scanlines · CRT in 4:3",
        play_h="Giocare",
        play=f"""Il gioco è in **alfa** (versione {VERSION}). **Windows**: scarica l'ultima versione dalla [pagina delle release]({REPO}/releases/latest), estrai tutto lo zip e avvia `A Survivor to the Past.exe`. Windows potrebbe avvisare che il gioco non è firmato: fai clic su *Ulteriori informazioni* → *Esegui comunque*. Poi il gioco **si aggiorna da solo**: all'avvio controlla su GitHub se c'è una nuova versione, la scarica in background e la installa all'avvio successivo (Opzioni → Aggiornamenti: Automatici / Chiedi / No). Le tue impostazioni restano e non viene inviato alcun dato personale: gli aggiornamenti contattano solo GitHub, e la classifica mondiale contatta il sito del gioco solo quando la apri o pubblichi un punteggio (soprannome e punteggio).

**Browser**: [gioca direttamente sul sito]({SITE}play/), senza installare nulla. Meglio su computer, con Chrome, Edge o Firefox; i progressi sono salvati nel browser. Le versioni per Linux, macOS e Android arriveranno più avanti. Nel frattempo puoi compilare il gioco dai sorgenti con [Rust](https://rustup.rs) (stable):""",
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
        road=[
            'Mappe vere fatte a mano e altri personaggi giocabili',
            'Altri boss con attacchi e animazioni propri',
            'Comandi touch per telefoni e tablet',
            'Versioni per Linux, macOS e Android',
        ],
        comm_h="Comunità",
        comm=f"""È un progetto fatto per divertimento. Per altri progetti, condivisioni, idee, proposte o bug, vieni sul **[Discord]({DISCORD})**.

Sito: [{SITE.rstrip('/').split('//')[1]}]({SITE})""",
        legal_h="Note legali",
        legal=f"""**A Survivor to the Past** è un **fan game non ufficiale, gratuito e non commerciale**, fatto per divertimento. **È vietato venderlo, in tutto o in parte.**

*The Legend of Zelda*, *A Link to the Past*, *The Minish Cap*, i loro personaggi, la grafica, le musiche e gli effetti sonori sono marchi e opere protette di **Nintendo**. Tutte le risorse Nintendo usate qui appartengono a Nintendo. Questo progetto non è affiliato, sponsorizzato né approvato da Nintendo.

La [licenza GPL-3.0](LICENSE) copre solo il codice sorgente scritto per questo progetto; non concede alcun diritto sulle risorse di Nintendo. Gli sprite sheet provengono dai rip condivisi dalla comunità di *The Spriters Resource* — grazie ai loro autori.

Il nome di ogni autore di rip o di sprite (Barack Obama, Mister Man, Nemau, Aztec Warrior, Deathbringer, Barubary, Vaati, Mister Mike, Ragnarockerbunny e molti altri) è elencato in **[CREDITS.md](CREDITS.md)**. Anche Tipo Timido appartiene a Nintendo; *Sonic the Hedgehog* è un marchio di **SEGA**.

Titolari dei diritti: per far rimuovere un contenuto, aprite una issue o contattateci su [Discord]({DISCORD}).""",
    ),
    "pt": dict(
        tagline="Um survivors-like no mundo de <b>A Link to the Past</b>: resista às hordas de monstros de Hyrule.",
        status="alpha",
        what_h="O que é?",
        what="""**A Survivor to the Past** é um fangame não oficial, feito por diversão. Ele mistura a fórmula dos *survivors-like* (suas armas atacam sozinhas, as ondas nunca param, cada nível traz novos poderes) com os sprites, músicas e sons de *The Legend of Zelda: A Link to the Past* (Super Nintendo, 1991).

Guie Link no meio de hordas cada vez maiores, colete rupias para subir de nível, escolha uma melhoria entre três, abra os baús deixados pelas elites e evolua suas armas para as formas lendárias. Cada fase dura uns dez minutos: um chefe intermediário chega aos 4:30, o chefe final aos 9:00, e vencê-lo abre a saída. Entre as partidas, gaste suas rupias em melhorias permanentes.""",
        feat_h="Conteúdo",
        feats=[
            '**6 fases** desbloqueadas uma após a outra — Campo de Hyrule, Bosque Perdido, Deserto do Mistério, Lago congelado, Pirâmide do Poder e a Montanha da Morte como bônus — cada uma com seu clima, sua música, um chefe intermediário e um chefe final, até Agahnim e Ganon',
            '**12 personagens jogáveis** para desbloquear — Link, Link Minish, Link sem gorro, Sonic, a princesa Zelda, o Link do NES, o Link de *Four Swords* (4 cores), o de *Link\'s Awakening*, Link adulto, Link Deku, Shy Guy e Shadow Link (derrotando Ganon) — cada um com pontos fortes, fraquezas e sua arma inicial',
            'Uma **Bob-omb** rara que varre todos os inimigos da tela, e **cenário destrutível** que esconde rupias, corações, fadas ou o medalhão do Éter',
            '**Encontros** aleatórios: um campeão errante espera em algum lugar e uma seta mostra o caminho — derrote-o para ganhar uma bênção, uma melhoria especial de arma ou uma relíquia mantida se você vencer',
            "Um **Pikit ladrão** que devora as rupias deixadas no chão — pegue-o para recuperar todas",
            '**12 armas**, cada uma com uma **evolução**, e **12 itens passivos** — 6 espaços de armas e 6 de itens: cada escolha conta',
            '**18 tipos de inimigos** de Hyrule, **7 chefes** e **elites** que deixam baús',
            '**Progressão entre partidas**: a loja de Kakariko (melhorias permanentes), as bênçãos da Grande Fada, uma **relíquia** por fase, 4 **dificuldades** e os **selos** de Ganon para ganhar mais rupias',
            '**Perfil**: bestiário, arsenal, estatísticas, e exportar / importar o progresso como código de texto',
            "**5 jogadores** no mesmo computador, cada um com nome, avatar e progresso próprios; uma **pontuação** em cada partida e **rankings**: melhores pontuações por fase, entre os jogadores da casa e um **top 100 mundial** por versão do jogo",
            "Um **manual de instruções** no estilo do original (PDF, inglês e francês)",
            'Ataques automáticos ou **mira manual** a qualquer momento — teclado, mouse ou controle, e **todas as teclas podem ser reconfiguradas**',
            '**Gráficos, músicas e sons** originais de A Link to the Past no SNES',
            '**Filtros de vídeo retrô**: SNES, CRT, Scanlines, Game Boy Advance, Game Boy, mais um modo **4:3** — e **filtros de áudio** combinando',
            '**7 idiomas**: English, Français, Deutsch, Español, Italiano, Português, 日本語',
            '**Motor próprio em Rust** (wgpu): milhares de inimigos na tela, no Windows e **no navegador**',
        ],
        evo_h="Armas e evoluções",
        evo_cols=("Arma", "+ Item (qualquer nível)", "Evolução (arma no nível máximo)"),
        shots_h="Imagens",
        caps=dict(chars='Seleção de personagem', mountain='Montanha da Morte (fase bônus) e seus Lynels',
                  stages='Seleção de fase', shop='Loja de Kakariko: melhorias permanentes',
                  title="Tela de título", levelup="Subir de nível: escolha 1 de 3", evolution="Baú de evolução",
                  boss="Chefe: Hinox", options="Opções (idioma, filtros, volumes)", about="Sobre"),
        filters_cap="Filtros de vídeo: nenhum · CRT / Game Boy Advance · Game Boy / Scanlines · CRT em 4:3",
        play_h="Jogar",
        play=f"""O jogo está em **alfa** (versão {VERSION}). **Windows**: baixe a versão mais recente na [página de versões]({REPO}/releases/latest), extraia o zip inteiro e abra `A Survivor to the Past.exe`. O Windows pode avisar que o jogo não é assinado: clique em *Mais informações* → *Executar assim mesmo*. Depois, o jogo **se atualiza sozinho**: ao iniciar, ele verifica no GitHub se há uma nova versão, baixa em segundo plano e instala no próximo início (Opções → Atualizações: Automáticas / Perguntar / Não). Suas configurações são mantidas e nenhum dado pessoal é enviado: as atualizações só contatam o GitHub, e o ranking mundial só contata o site do jogo quando você o abre ou publica uma pontuação (apelido e pontuação).

**Navegador**: [jogue direto no site]({SITE}play/), sem instalar nada. Funciona melhor no computador, no Chrome, Edge ou Firefox, e seu progresso fica salvo no navegador. As versões para Linux, macOS e Android virão depois. Enquanto isso, você pode compilar o jogo a partir do código-fonte com [Rust](https://rustup.rs) (stable):""",
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
        road=[
            'Mapas de verdade feitos à mão e mais personagens jogáveis',
            'Mais chefes com ataques e animações próprios',
            'Controles de toque para celulares e tablets',
            'Versões para Linux, macOS e Android',
        ],
        comm_h="Comunidade",
        comm=f"""É um projeto feito por diversão. Para outros projetos, compartilhar, ideias, sugestões ou bugs, venha para o **[Discord]({DISCORD})**.

Site: [{SITE.rstrip('/').split('//')[1]}]({SITE})""",
        legal_h="Aviso legal",
        legal=f"""**A Survivor to the Past** é um **fangame não oficial, gratuito e sem fins comerciais**, feito por diversão. **É proibido vendê-lo, no todo ou em parte.**

*The Legend of Zelda*, *A Link to the Past*, *The Minish Cap*, seus personagens, gráficos, músicas e efeitos sonoros são marcas e obras protegidas da **Nintendo**. Todos os recursos da Nintendo usados aqui pertencem à Nintendo. Este projeto não é afiliado, patrocinado nem endossado pela Nintendo.

A [licença GPL-3.0](LICENSE) cobre apenas o código-fonte escrito para este projeto; ela não concede nenhum direito sobre os recursos da Nintendo. As folhas de sprites vêm das extrações compartilhadas pela comunidade do *The Spriters Resource* — obrigado aos seus autores.

O nome de cada autor de extrações ou sprites (Barack Obama, Mister Man, Nemau, Aztec Warrior, Deathbringer, Barubary, Vaati, Mister Mike, Ragnarockerbunny e muitos outros) está em **[CREDITS.md](CREDITS.md)**. Shy Guy também pertence à Nintendo; *Sonic the Hedgehog* é uma marca da **SEGA**.

Detentores de direitos: para remover um conteúdo, abram uma issue ou falem conosco no [Discord]({DISCORD}).""",
    ),
    "ja": dict(
        tagline="<b>神々のトライフォース</b>の世界で生き残れ。ハイラルの魔物の大群に立ち向かうサバイバーライク。",
        status="alpha",
        what_h="どんなゲーム？",
        what="""**A Survivor to the Past** は、楽しみのために作られた非公式のファンゲームです。*サバイバーライク*のルール（武器は自動で攻撃し、敵の波は止まらず、レベルアップのたびに新しい力が手に入る）と、*ゼルダの伝説 神々のトライフォース*（スーパーファミコン、1991年）のスプライト・音楽・効果音を組み合わせています。

増え続ける大群の中でリンクを動かし、ルピーを集めてレベルアップ。3つの強化から1つを選び、エリートが落とす宝箱を開けて、武器を伝説の姿へ進化させましょう。各ステージは約10分。4:30 に中ボス、9:00 にラスボスが現れ、倒すと出口が開きます。プレイの合間には、ルピーで永続的な強化を買いましょう。""",
        feat_h="特徴",
        feats=[
            '順番に解放される**6つのステージ** — ハイラル平原、迷いの森、幻の砂漠、凍った湖、力のピラミッド、ボーナスのデスマウンテン。それぞれ天候、音楽、中ボス、ラスボスが異なり、最後はアグニムとガノンが待ち受けます',
            '解放できる**12人のキャラクター** — リンク、ミニッシュリンク、帽子なしのリンク、ソニック、ゼルダ姫、ファミコン版リンク、『4つの剣』のリンク（4色）、『夢をみる島』のリンク、大人のリンク、デクリンク、ヘイホー、シャドウリンク（ガノンを倒すと解放）。それぞれ長所と短所、最初の武器が異なります',
            '画面の敵をすべて吹き飛ばすレアな**ボム兵**と、ルピー・ハート・妖精・エーテルのメダルが隠れた**壊せる背景**',
            'ランダムな**遭遇戦**：どこかで待つ放浪のチャンピオンを矢印が示します。倒すと祝福、武器の特別強化、勝利すれば持ち帰れるレリックが手に入ります',
            "地面に残ったルピーを食べてしまう**泥棒ピキット** — 捕まえればすべて取り戻せます",
            '**12種類の武器**（それぞれ**進化**あり）と**12種類のパッシブアイテム** — 武器枠6つ、アイテム枠6つで、選択が重要です',
            'ハイラルの**18種類の敵**、**7体のボス**、宝箱を落とす**エリート**',
            '**プレイをまたぐ成長**：カカリコのショップ（永続強化）、大妖精の祝福、ステージごとの**レリック**、4段階の**難易度**、ルピーが増えるガノンの**封印**',
            '**プロフィール**：モンスター図鑑、武器図鑑、統計、テキストコードによるセーブのエクスポート / インポート',
            "同じパソコンで**5人のプレイヤー**（名前・アバター・進行状況）。毎回の**スコア**と**ランキング**：ステージごとの自己ベスト、家族のプレイヤー同士、バージョンごとの**世界トップ100**",
            "オリジナルの説明書を模した**取扱説明書**（PDF、英語とフランス語）",
            '自動攻撃、またはいつでも**手動照準** — キーボード、マウス、コントローラーに対応し、**すべてのキーを変更可能**',
            'SNES 版『神々のトライフォース』のオリジナル**グラフィック・音楽・効果音**',
            '**レトロ映像フィルター**：SNES、CRT、スキャンライン、ゲームボーイアドバンス、ゲームボーイ、**4:3** モード — 対応する**音声フィルター**付き',
            '**7言語**：English、Français、Deutsch、Español、Italiano、Português、日本語',
            '自作の **Rust エンジン**（wgpu）：画面上に数千体の敵、Windows でも**ブラウザ**でも',
        ],
        evo_h="武器と進化",
        evo_cols=("武器", "＋ アイテム（レベル不問）", "進化（武器が最大レベル）"),
        shots_h="スクリーンショット",
        caps=dict(chars='キャラクター選択', mountain='デスマウンテン（ボーナスステージ）とライネル',
                  stages='ステージ選択', shop='カカリコのショップ：永続強化',
                  title="タイトル画面", levelup="レベルアップ：3つから1つを選ぶ", evolution="進化の宝箱",
                  boss="ボス：ヒノックス", options="オプション（言語・フィルター・音量）", about="このゲームについて"),
        filters_cap="映像フィルター：なし · CRT / ゲームボーイアドバンス · ゲームボーイ / スキャンライン · CRT（4:3）",
        play_h="遊ぶ",
        play=f"""このゲームは**アルファ版**（バージョン {VERSION}）です。**Windows**：[リリースページ]({REPO}/releases/latest)から最新版をダウンロードし、zip をすべて展開して `A Survivor to the Past.exe` を起動してください。署名がないため Windows に警告された場合は、*詳細情報* → *実行* をクリックします。その後は**自動でアップデート**されます。起動時に GitHub で新しいバージョンを確認し、バックグラウンドでダウンロードして次回起動時にインストールします（オプション → アップデート：じどう / かくにん する / オフ）。設定はそのまま残り、個人情報は送信されません。アップデートの通信先は GitHub だけで、世界ランキングは開いたときやスコアを公開したときだけゲームのサイトに接続します（ニックネームとスコア）。

**ブラウザ**：[サイト上で直接プレイ]({SITE}play/)でき、インストールは不要です。パソコンの Chrome、Edge、Firefox がおすすめで、進行状況はブラウザに保存されます。Linux・macOS・Android 版は今後予定しています。それまでは [Rust](https://rustup.rs)（stable）でソースからビルドできます：""",
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
        road=[
            '手作りの本格的なマップと新しいプレイアブルキャラクター',
            '独自の攻撃とアニメーションを持つボスの追加',
            'スマートフォン・タブレット向けのタッチ操作',
            'Linux・macOS・Android 版',
        ],
        comm_h="コミュニティ",
        comm=f"""楽しみのためのプロジェクトです。ほかの作品、共有、アイデア、提案、バグ報告は **[Discord]({DISCORD})** へどうぞ。

サイト：[{SITE.rstrip('/').split('//')[1]}]({SITE})""",
        legal_h="法的事項",
        legal=f"""**A Survivor to the Past** は、楽しみのために作られた**非公式・無料・非営利のファンゲーム**です。**本作の全部または一部の販売を禁止します。**

*ゼルダの伝説*、*神々のトライフォース*、*ふしぎのぼうし*、およびそのキャラクター・グラフィック・音楽・効果音は**任天堂**の商標および著作物です。本作で使用している任天堂の素材はすべて任天堂に帰属します。本プロジェクトは任天堂とは一切関係がなく、任天堂の後援・承認を受けたものではありません。

[GPL-3.0 ライセンス](LICENSE)は本プロジェクトのために書かれたソースコードのみに適用され、任天堂の素材に関する権利を与えるものではありません。スプライトシートは *The Spriters Resource* のコミュニティが共有したものです。作者の皆さんに感謝します。

スプライトを抽出・制作した方々（Barack Obama、Mister Man、Nemau、Aztec Warrior、Deathbringer、Barubary、Vaati、Mister Mike、Ragnarockerbunny ほか）の名前は **[CREDITS.md](CREDITS.md)** に掲載しています。ヘイホーも任天堂のキャラクターです。*Sonic the Hedgehog* は **SEGA** の商標です。

権利者の方へ：コンテンツの削除をご希望の場合は、Issue を作成するか [Discord]({DISCORD}) でご連絡ください。""",
    ),
}

WEB = {
    "en": "Play in your browser",
    "fr": "Jouer dans le navigateur",
    "de": "Im Browser spielen",
    "es": "Jugar en el navegador",
    "it": "Gioca nel browser",
    "pt": "Jogar no navegador",
    "ja": "ブラウザでプレイ",
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
        f'<p align="center"><a href="{REPO}/releases/latest"><b>⬇ {DL[code]}</b></a> · '
        f'<a href="{SITE}play/"><b>▶ {WEB[code]}</b></a></p>',
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
    for a, b in (("title", "stages"), ("chars", "mountain"), ("levelup", "evolution"), ("boss", "shop"), ("options", "about")):
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

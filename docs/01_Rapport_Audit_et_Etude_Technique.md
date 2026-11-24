# A Survivor to the Past — Rapport n°1 : audit des assets & étude technique

*24 novembre 2026 — Étapes 1 (Audit) et 2 (Étude technique), avec une première proposition pour l'étape 3 (Conception).*
*Aucun code de jeu n'a été écrit : seulement des outils d'analyse et des prototypes de benchmark (`research/`).*

---

## 1. Inventaire et analyse des assets

Dossier analysé : planches et sons fournis pour le projet (28 Mo, 144 fichiers), copiés dans `assets/source/`.

| Catégorie | Contenu | Format |
|---|---|---|
| Planches de sprites ALttP (SNES) | 64 PNG : Link, 12 boss, 11 planches d'ennemis, 5 planches de PNJ, effets, objets, HUD, polices | Planches « Spriters Resource » |
| Décors | Light World complet (4110×5136), Dark World complet (4240×4240), Pyramide du Pouvoir, Château d'Hyrule, 5 grottes/intérieurs | Rendus de cartes entières (pas des tilesets) |
| Effets visuels | Bombes/explosions, buissons/pots, morts d'ennemis, brouillard, canopée des Bois Perdus, pluie + halo de lumière | PNG |
| Sons | 129 effets sonores WAV (archive ZIP) | 44,1 kHz, 16 bits, stéréo, 156 s au total |
| Personnages additionnels | 3 planches dans `AdditionalChars` | Styles variés (voir §2) |
| Divers | Tutoriel HTML de ripping SNES, 2 ZIP de modèles 3D (Triforce, cristal) | Non exploitables tels quels |
| **Musiques** | **Aucune** | — |

### Format technique des planches
- **Fond magenta `#FF00FF`** sur la quasi-totalité des planches ; vert (`#009E00`, `#11FF11`) ou sarcelle (Link) sur quelques-unes ; transparent sur les persos additionnels.
- Cadres rose clair `#F880F8` autour de certains groupes, **texte d'annotation** (« Moving – loop », « reverse »…), **nuanciers de palettes** et crédits des rippers incrustés.
- Les annotations sont précieuses : elles indiquent l'ordre des frames, les boucles et les animations inversées.
- **Palettes alternatives fournies** : 11 palettes pour les chevaliers (or, rouge, vert, bleu, glace…), **palettes de « hit flash »** officielles pour les ennemis et boss, 3 tuniques de Link (verte/bleue/rouge) + forme lapin.
- Tailles : Link ≈ 16×24 px par frame ; ennemis 16×16 à 32×32 ; boss de 32×32 à ~96×96 une fois assemblés.

### Découpage automatique — testé
J'ai écrit un prototype de découpeur (`research/tools/autoslice.py`) : détection du fond, isolement des sprites par composantes connexes, rejet du texte et des nuanciers, export d'un atlas + JSON.
- Sur la planche « Minor Light World Enemies » : **64 sprites propres extraits sans intervention**, transparence comprise.
- Sur l'ensemble des planches de personnages/ennemis/boss : **≈ 6 900 sprites détectés automatiquement**.
- Limites : les **noms d'animations** (marche bas/haut/côté, attaque…) et l'**assemblage des boss multi-pièces** (Trinexx, Helmasaur King, Vitreous, Kholdstare, Moldorm, Lanmola) demandent une passe de curation une fois pour toutes, stockée dans des fichiers de données.

### Décors : rendus de cartes → tuiles réutilisables
Les cartes ne sont pas des tilesets, mais elles sont **parfaitement alignées sur la grille SNES** :
- Dark World : **3 183 tuiles 8×8 uniques / 2 264 tuiles 16×16 uniques** sur 70 000 cellules (grille alignée en 0,0).
- Light World : composé de zones à des décalages différents → extraction zone par zone (outil à faire, faisable).
- Conséquence : on peut **reconstruire n'importe quelle zone d'Hyrule en tilemap** (rendu par blocs, collisions par tuile, variations), et la géographie Light World / Dark World est **identique** → mécanique de bascule possible (voir §8).

### Sons
- 129 WAV de très bonne couverture : épée (4 niveaux), rayon d'épée, attaque tournoyante, flèches, bombes, baguettes de feu/glace, marteau, grappin, coups/morts d'ennemis et de boss, rubis, cœurs, fées, coffres, « item get », menus, curseur, erreurs, téléportations, tonnerre, séisme, pluie d'ambiance…
- À traiter automatiquement : **silences en fin de fichier** (ex. « fighter sword 1 » : son utile 0,14 s sur 0,6 s), stéréo inutile (sources mono), conversion OGG (~27 Mo → ~2-3 Mo), normalisation du volume.

---

## 2. Personnages et ennemis identifiés

### Personnages jouables
| Personnage | Source | Animations | Adéquation SNES |
|---|---|---|---|
| **Link** (principal) | ALttP SNES | Marche 4 directions, poses d'épée, attaque tournoyante, nage, chute, mort, objet brandi, blessure, forme lapin ; **3 tuniques** ; équipement séparé (4 épées × angles de coup, 3 boucliers, arc, boomerangs, grappin, baguettes, cannes, marteau, médaillons…) | Parfaite |
| Princesse Zelda | Four Swords (GBA) | Marche 4 directions, idle, quelques actions — pas d'attaque | Très bonne (style GBA proche du SNES) |
| Sonic « style Zelda Game Boy » | Custom | Marche, sprint, boule, attaques, 3 variantes de palette | Moyenne : style Game Boy, plus plat |
| Link BotW « style Zelda 1 » | Custom NES | Marche 4 directions, attaque, objets | Faible : style NES 8 bits, palette très réduite |

**Point clé d'architecture** : dans un Survivors, l'attaque visible est l'effet de l'**arme**, pas le corps. Le minimum pour un personnage est donc un **cycle de marche** (4 directions, ou 1 direction + miroir). C'est ce qui rend l'intégration automatique réaliste.

### Ennemis (~70 types, sans compter les variantes de palette)
- **Soldats d'Hyrule** : épée, lance, arc, bombes, fléau, trident — 11 palettes (base idéale pour élites et variantes de difficulté).
- **Light World** : Octorok, Buzz Blob, Cukeman, Corbeau, Rope, Rat, Keese, Tektites, Leevers, Geldman, Vautour, Abeilles, Armos, Poe, Zol, Hoarder, Kodongo, Toppo…
- **Dark World** : Moblin, Taros, Lynel, Hinox, Eyegore, Zazak, Goriya, Wizzrobes, Gibo, Chain Chomp, Freezor, Pengator, Zora, Kyameron, Bari, Ropa, Stalfos (5 variantes), Chevalier Stalfos, Gibdo, Terrorpin, Snap Dragon, Slarok, Helmasaur…
- **Pièges/objets** : Beamos, pièges à pointes, barres de feu, etc.
- **Boss (12)** : Armos Knights, Lanmola, Moldorm, Agahnim, Helmasaur King, Arrghus, Mothula, Blind, Kholdstare, Vitreous, Trinexx, Ganon — avec projectiles, phases et palettes de hit flash.
- **PNJ** (Zelda, fées, Grande Fée, animaux dont les **Cuccos**, habitants) : décor vivant, événements, marchands.

---

## 3. Ressources graphiques, animations, musiques et sons — usages prévus

| Asset ALttP | Usage dans le jeu |
|---|---|
| Rubis vert / bleu / rouge | **Gemmes d'expérience** (1 / 5 / 20 XP) |
| Cœurs, fées, flacons | Soin, résurrection, consommables |
| Petits/grands coffres + son « item get » + pose « objet brandi » de Link | **Ouverture de coffre** spectaculaire |
| Équipement de Link (épées, arc, boomerang, baguettes, cannes, marteau, médaillons) | **Armes automatiques** et icônes d'amélioration |
| Pendentifs, cristaux, Triforce | Progression méta / déblocage de la map finale / **porte de sortie** |
| HUD (cœurs, jauge de magie, compteurs) | Interface en jeu (jauge de magie → barre d'XP) |
| Polices **dont la police française accentuée** (é, è, à, ç, ô…) | Toute l'interface, en français, dans la police d'origine |
| Écran de statut, écran de saisie du nom | Menus, sélection, **emplacements de sauvegarde** |
| Écran titre (Triforce, épée) | Base de l'écran titre (le logo « Zelda » sera remplacé) |
| Pluie, brouillard, canopée, halo de lumière | **Météo et ambiances de map**, zones sombres à la lanterne |
| Buissons, pots, explosions, morts d'ennemis | Destructibles avec butin, retours visuels de combat |

**Musique : aucun fichier fourni.** C'est le manque principal (voir §4 et question n°2).

---

## 4. Éléments manquants

| Manque | Gravité | Solution proposée |
|---|---|---|
| **Musiques** | Bloquant pour l'ambiance | Tu fournis les pistes (OGG/MP3/WAV/FLAC, ou fichiers **SPC** — ffmpeg les convertit avec points de boucle). Le moteur gère intro + boucle. |
| Logo du titre « A Survivor to the Past » | Moyen | À créer dans le style ALttP (exception signalée) |
| Effets des médaillons (Bombos, Ether, Quake) et grosses attaques de fin de partie | Moyen | Composer à partir des effets existants (explosions, flammes, éclairs) + shaders rétro ; quelques frames à dessiner dans le style si nécessaire (exception signalée) |
| Tilesets d'intérieurs de donjons (hors château et grottes) | Faible | Les 5 maps proposées utilisent l'extérieur + château + pyramide |
| Portraits de personnages | Faible | Sprite animé agrandi en pixel-perfect sur fond de cadre ALttP |
| Cadres d'interface (cartes d'amélioration, boutons) | Faible | Construits en 9-slice à partir des cadres de texte et de l'écran de statut |
| Assemblage des boss multi-pièces | Travail, pas un manque | Fichiers de définition de boss (segments, articulations, phases) |
| Cohérence de style des persos additionnels | À décider | Voir question n°4 |

**Note légale (sans jugement)** : les sprites, sons et musiques sont des rips de jeux Nintendo. Pour un projet **personnel et privé**, aucun souci pratique ; une **diffusion publique** (y compris une version Web ouverte) serait risquée — Nintendo fait régulièrement retirer les fan games. D'où ma recommandation : version Web future **strictement privée**, et un écran de crédits pour les rippers.

---

## 5. Comparatif des moteurs — avec benchmarks réels

### Méthode
Même scénario partout : N ennemis qui poursuivent le joueur avec **séparation** (anti-chevauchement via grille spatiale), **N/2 projectiles** testés contre les ennemis, morts et réapparitions. Puis rendu des ennemis avec de **vrais sprites ALttP animés**, tous visibles à l'écran (pire cas), en 1920×1080.
Machine de test : PC de bureau récent (processeur 8 cœurs, carte graphique haut de gamme). Code dans `research/benchmarks/`.

### Simulation seule (1 cœur, sans rendu) — ms par frame (budget à 60 fps : 16,6 ms)
| Ennemis + projectiles | Rust natif | Rust → WebAssembly | JavaScript (TypedArrays) |
|---|---|---|---|
| 5 000 + 2 500 | 0,35 | 0,37 | 0,49 |
| 10 000 + 5 000 | **0,91** | 0,95 | 1,23 |
| 20 000 + 10 000 | 2,34 | 2,39 | 3,05 |
| 50 000 + 25 000 | 5,78 | 5,97 | 7,08 |
| 100 000 + 50 000 | 9,94 | 10,45 | 11,93 |

→ Avec une conception orientée données, **le langage compte moins que l'architecture**. La différence se fera sur le multithreading (facile en Rust), l'absence de ramasse-miettes et surtout le **rendu**.

### Simulation + rendu complet — ms par frame
| Ennemis | **Rust sur mesure (wgpu, 1 appel de dessin instancié)** | Bevy 0.20 (1 entité par sprite, usage standard) | WebGL2 + JS dans le navigateur |
|---|---|---|---|
| 5 000 | **0,51** (≈1 900 fps) | 3,42 (293 fps) | 0,94 |
| 10 000 | **1,07** (≈930 fps) | 5,64 (177 fps) | 2,50 |
| 25 000 | **3,37** (≈300 fps) | 12,68 (79 fps) | 9,69 |
| 50 000 | **6,53** (153 fps) | 25,02 (40 fps) | 23,85 |
| 100 000 | **11,36** (88 fps) | 53,87 (19 fps) | 49,89 |

*Rust/wgpu et WebGL mesurés en mode « synchronisé GPU » (pessimiste) ; Bevy en fenêtre sans vsync (avantagé par son rendu en pipeline).*

**Lecture** : un moteur générique qui traite chaque ennemi comme un objet (entité Bevy, nœud Godot, GameObject Unity) paie un coût fixe par objet. Un rendu instancié sur mesure dessine 10 000 sprites en ~0,15 ms : la simulation devient le seul vrai coût, et elle se parallélise.

### Godot et Unity (non mesurés localement)
Ils ne sont pas installés ; je ne les ai pas téléchargés sans ton accord. Synthèse factuelle :
- **Godot 4** : *Halls of Torment* est fait avec Godot ([vitrine Godot](https://godotengine.org/showcase/halls-of-torment/), [conférence GodotFest 2025](https://media.ccc.de/v/godotfest2025-a-peek-under-the-hood-technical-learnings-from-halls-of-torment)) — preuve qu'il suffit pour quelques centaines à quelques milliers d'ennemis. Au-delà, la [documentation officielle](https://docs.godotengine.org/en/4.5/tutorials/performance/using_multimesh.html) recommande MultiMesh + accès direct aux serveurs, voire une extension C++/Rust : on retombe alors sur l'approche « sur mesure », mais coupée en deux langages, et l'export Web avec extension native reste délicat.
- **Unity 6** : *Vampire Survivors* a quitté Phaser (JS) pour Unity pour des raisons de performance ([PC Gamer](https://www.pcgamer.com/vampire-survivors-was-such-a-big-hit-its-getting-a-whole-new-engine), [GamingOnLinux](https://www.gamingonlinux.com/2023/07/vampire-survivors-switching-to-new-game-engine-on-august-17)). DOTS/Burst est très rapide, mais le 2D en ECS est à construire soi-même, le flux de travail est centré sur l'éditeur (difficile à piloter efficacement depuis le code) et l'outil est lourd.
- Si tu veux, je télécharge Godot (~70 Mo, site officiel) pour mesurer les mêmes scénarios avant de trancher.

### Grille de comparaison
| Critère | Rust sur mesure | Bevy | Godot 4 (+ ext. Rust) | Unity 6 DOTS | TS / WebGL |
|---|---|---|---|---|---|
| Ennemis simultanés | ★★★★★ 50 000+ mesuré | ★★★ ~10 000 mesuré | ★★★ nœuds / ★★★★ MultiMesh | ★★★★★ (non mesuré) | ★★★ ~10–20 000 mesuré |
| CPU / GPU | ★★★★★ | ★★★ | ★★★ | ★★★★★ | ★★★ |
| Sprites et animations 2D | ★★★★ (à écrire, contrôle total) | ★★★★ | ★★★★★ éditeur | ★★★ | ★★★★ |
| Collisions à grande échelle | ★★★★★ grille SoA multithread | ★★★★ | ★★ physique native / ★★★★ ext. | ★★★★★ | ★★★ mono-thread |
| Projectiles et effets | ★★★★★ instanciés | ★★★ | ★★★ | ★★★★ | ★★★ |
| Batching | ★★★★★ 1 appel de dessin | ★★★ | ★★★ (pas de culling par instance) | ★★★★ | ★★★★ |
| Développement et maintenance | ★★★ plus de socle à écrire, aucune dépendance d'éditeur | ★★★ compilation 3–4 min, API qui change tous les ~4 mois | ★★★★ mais 2 langages | ★★ | ★★★★ |
| Compatibilité assets | ★★★★★ pipeline maison | ★★★★ | ★★★★ | ★★★★ | ★★★★ |
| Application locale | ★★★★★ un .exe léger | ★★★★★ | ★★★★★ | ★★★★ | ★★★ (Tauri/Electron) |
| Web futur | ★★★★ wgpu → WebGPU/WebGL2 | ★★★★ | ★★★ (ext. native délicate) | ★★★ | ★★★★★ |

---

## 6. Recommandation : moteur sur mesure en Rust (wgpu)

**Classement** : 1) Rust sur mesure · 2) Godot 4 + extension Rust (si tu tiens à un éditeur visuel) · 3) Bevy avec rendu de horde sur mesure · 4) TypeScript/WebGL · 5) Unity.

**Pourquoi** :
1. **Performances mesurées** : 4 à 5× Bevy, ~2× le Web, marge énorme (50 000 ennemis à 150 fps). On vise 5 000–10 000 ennemis + des dizaines de milliers de projectiles, gemmes et particules sans ralentissement, même sur une machine moyenne.
2. **Contrôle total du rendu SNES** : rendu en basse résolution puis agrandissement entier, positions au pixel près, **échange de palettes par shader** (hit flash officiels, élites), effets façon SNES (mosaïque, distorsion « HDMA », transitions façon Mode 7), filtre CRT optionnel.
3. **Entièrement piloté par les données** : personnages, ennemis, armes, vagues, maps et difficultés en fichiers lisibles, rechargés à chaud pendant le développement.
4. **Web possible plus tard** : wgpu cible WebGPU/WebGL2 ; la simulation compile déjà en WASM (testé : seulement +5 % de temps).
5. **Pas de dépendance à un éditeur ni aux changements d'API d'un gros moteur** : un .exe léger, du code qui reste stable.

**Compromis honnêtes** :
- Pas d'éditeur visuel → compensé par des **outils de développement intégrés** (panneau de debug : apparition d'ennemis, accélération du temps, visualisation des vagues, compteurs de performance) et des fichiers de données.
- Plus de socle à construire au départ (rendu, interface, audio) : environ les premières étapes de développement, avant le contenu. Je prends ce coût en charge.

**Briques retenues** (bibliothèques mûres, pas un moteur) : `wgpu` (rendu), `winit` (fenêtre, entrées), `gilrs` (manettes), `kira` (audio : bus, boucles, fondus), `serde` + **RON** (données lisibles avec commentaires), `rayon` (multithread), `egui` (outils de debug uniquement).

---

## 7. Architecture technique envisagée

```
a-survivor-to-the-past/
├─ crates/
│  ├─ engine/      fenêtre, entrées, rendu wgpu (sprites instanciés, tilemaps par blocs,
│  │               palettes, post-effets), audio kira, chargement d'assets, temps fixe 60 Hz
│  ├─ sim/         gameplay pur, sans rendu : horde en tableaux SoA, grille spatiale,
│  │               flow-field (contournement des obstacles), armes, projectiles, dégâts,
│  │               statuts, IA paramétrable, vagues, boss, RNG à graine (rejouable)
│  ├─ content/     schémas de données + validation avec messages clairs
│  ├─ ui/          pile d'écrans, widgets pixel-art (9-slice), police bitmap FR, transitions
│  └─ game/        assemblage : états (menu → préparation → partie → fin), sauvegarde
├─ tools/asset-forge/   CLI : découpe des planches → atlas + manifestes, extraction de tuiles,
│                       import de personnages, conversion audio (rognage, OGG, normalisation)
├─ assets/  raw/ (planches d'origine, intouchées) · generated/ (atlas, tilemaps, audio OGG)
├─ data/    characters/ enemies/ weapons/ passives/ maps/ waves/ bosses/
│           difficulty.ron  balance.ron  audio.ron
└─ AdditionalChars/   dossier surveillé
```

**Principes**
- **Séparation stricte** : `sim` ne sait rien du rendu ni de l'interface → testable, rapide, portable Web.
- Simulation à **pas fixe de 60 Hz** (comportement identique quelle que soit la machine), rendu interpolé à la fréquence de l'écran (165 Hz chez toi).
- **Résolution logique 640×360** rendue puis agrandie en entier : ×3 en 1080p, **×4 en 1440p**, ×6 en 4K. Champ de vision 40×22 tuiles (contre 16×14 sur SNES) : nécessaire pour lire une horde.
- **Tout est donnée** — exemple :

```ron
// data/enemies/soldat_vert.ron
Enemy(
    id: "soldat_vert",
    sprite: "knights/green_sword",
    hp: 12, speed: 38, contact_damage: 5, mass: 1.0,
    behavior: Chaser,
    drop: Rupee(Green),
    variants: [ (palette: "gold", tag: Elite, hp_mult: 4.0, drop: Chest) ],
)
```
```ron
// data/waves/plaine_hyrule.ron (extrait)
At("02:30", Spawn(["soldat_vert" @ 3.0/s, "archer_bleu" @ 0.5/s], pattern: Ring)),
At("05:00", MiniBoss("chevalier_fleau")),
At("06:00", Event(Orage)),
At("09:00", Boss("armos_knights")),
```

### Personnages additionnels : intégration sans code
1. Tu déposes une planche (PNG) dans `AdditionalChars/`.
2. Au lancement, **asset-forge** détecte le fond (transparent ou couleur unie), retire textes/crédits, découpe les frames, regroupe par rangées, repère les variantes de palette côte à côte, devine les directions (symétrie gauche/droite, ressemblance des frames) et génère un **brouillon** `character.ron` + un atlas.
3. Le personnage apparaît dans la sélection avec un **profil par défaut** (Équilibré) et une arme de départ.
4. Si la détection hésite, un **Atelier de personnage** intégré au jeu permet d'associer les rangées aux animations et de choisir profil/arme **en cliquant** — sans toucher au code.
5. Un fichier facultatif à côté de la planche (`Sonic.toml` : nom, stats, arme) prend le dessus sur les valeurs devinées.

Honnêtement : les planches simples seront 100 % automatiques ; les planches complexes donneront un brouillon jouable immédiatement, à affiner en une minute dans l'Atelier.

---

## 8. Première proposition de gameplay et des 5 maps

### Boucle d'une partie (10 minutes)
- Déplacement libre ; **coup d'épée automatique** de Link comme arme de base.
- Ennemis tués → **rubis** (XP). Montée de niveau → **choix 1 parmi 3** (4 avec la Chance), avec relance / passer / bannir.
- **6 armes + 6 objets passifs**. Armes niveau 1→8. **Évolution** = arme niveau max + passif requis + coffre de boss.
- **Mini-boss vers 5:00**, **boss final vers 9:00**. À sa mort, la **porte de sortie** apparaît (portail Triforce + son de téléportation) : l'atteindre = victoire. Si le boss vit encore à 11:00 → **Fureur** (boss enragé + élites en continu).
- Buissons et pots destructibles avec butin, coffres d'élites, fées, cœurs.

### Armes (tirées de l'arsenal de Link) — exemples
| Arme | Comportement | Évolution (avec…) |
|---|---|---|
| Épée | Coup en arc devant soi | **Épée de Légende** (Pendentif du Courage) : rayons d'épée + attaque tournoyante auto |
| Arc | Flèches vers l'ennemi le plus proche | **Flèches d'argent** (Carquois) : perforantes, dégâts ×3 sur boss |
| Boomerang | Aller-retour, étourdit | **Boomerang magique** (Gants) : multiples, ricochets |
| Bombes | Lancées en arc, zone | **Bombos** (Médaillon) : colonnes de feu à l'écran |
| Baguette de feu / de glace | Projectile + brûlure / gel | Combinables : **Vapeur** (dégâts sur ennemis gelés en feu) |
| Canne de Byrna | Orbes en orbite | **Barrière de Byrna** : aura continue |
| Marteau | Onde de choc au sol | **Quake** : séisme qui retourne tous les ennemis |
| Grappin | Attire et transperce | Chaînes en éventail |
| Lanterne | Cercle de flammes | Éclaire les zones sombres + brûlure |

**Passifs** : Bottes de Pégase (vitesse), Gants de puissance (dégâts/recul), Tunique bleue/rouge (armure), Bouclier (blocage), Demi-magie (recharge), Quart de cœur (PV max), Perle de Lune, Loupe (chance), Filet (aimant à rubis), Pendentifs…

### Idées originales à valider
1. **Bascule Monde de Lumière ↔ Monde des Ténèbres** : nous avons les deux cartes avec **la même géographie**. Pendant certaines parties, le Miroir Magique fait basculer la map (transition mosaïque SNES) : décor, palette et ennemis passent en version Ténèbres, plus dangereux mais plus rentables. Sans **Perle de Lune**, Link se transforme brièvement en lapin (clin d'œil, désactivable en facile).
2. **Sanctuaires de Grande Fée** : rester 5 s dans le cercle pendant que la horde arrive → choisir une bénédiction rare.
3. **La vengeance des Cuccos** : frapper un Cucco déclenche un essaim invincible pendant 20 s (événement rare et drôle).
4. **Pendentifs et cristaux** comme fil conducteur : chaque map vaincue rapporte un pendentif ; les trois ouvrent la Pyramide.
5. **Élites et palettes** : soldats dorés, glacés, etc., grâce aux 11 palettes fournies.

### Les 5 maps (zones réelles d'Hyrule reconstruites en tuiles, arènes bornées de ~3 000 px de côté)
| # | Map | Identité | Ennemis principaux | Événements | Mini-boss → Boss |
|---|---|---|---|---|---|
| 1 | **Plaine d'Hyrule — Nuit d'orage** | Pluie, éclairs, nuit qui laisse place à l'aube en fin de partie | Soldats verts/bleus, archers, Octoroks, Buzz Blobs, Corbeaux, Keese | Alerte du château (escouades en formation), éclairs | Chevalier au fléau → **Armos Knights** |
| 2 | **Bois Perdus** | Canopée, brouillard, lumière filtrée | Moblins, Cukemans, Hoarders, Poes, essaims d'abeilles, voleurs | Brouillard (vision réduite), voleur de rubis | Hinox → **Mothula** |
| 3 | **Désert du Mystère** | Chaleur (distorsion d'image), sable | Leevers, Geldmans, Vautours, Hokkubokku, Lynels tardifs | Tempête de sable (poussée), sables mouvants | Moldorm → **Lanmola** (trois vers qui plongent) |
| 4 | **Lac Hylia gelé** | Glace, blizzard, eau | Zoras, Tektites d'eau, Kyamerons, Pengators, Freezors, Baris | Sol glissant, blizzard | Arrghus → **Kholdstare** |
| 5 | **Pyramide du Pouvoir** (Monde des Ténèbres, débloquée par les 3 pendentifs) | Ciel crépusculaire, tout en Ténèbres | Lynels, Chevaliers Stalfos, Wizzrobes, Gibos, Taros, élites | Obscurité (halo de lanterne), vagues d'élites | Helmasaur King, Blind, Trinexx → **Agahnim puis Ganon** (2 phases) |

Vitreous reste en réserve pour les ajouts futurs.

### Difficultés (centralisées dans `difficulty.ron`)
| | Apprenti | Chevalier | Héros | Légende |
|---|---|---|---|---|
| Fréquence d'apparition | ×0,7 | ×1 | ×1,4 | ×1,8 |
| PV / dégâts ennemis | ×0,7 / ×0,6 | ×1 / ×1 | ×1,5 / ×1,4 | ×2,2 / ×2 |
| Vitesse ennemis | ×0,9 | ×1 | ×1,1 | ×1,2 |
| Chance d'élites | faible | normale | élevée | très élevée + boss avec phase bonus |
| Récompenses (rubis méta) | ×0,75 | ×1 | ×1,5 | ×2,5 |

**Proposition en plus : les Sceaux de Ganon** — modificateurs cumulables choisis avant la partie (ennemis plus rapides, pas de soin, boss enragés…), chacun augmentant le multiplicateur de récompense. Ça donne une rejouabilité quasi infinie avec le même contenu.

### Progression entre les parties (méta)
- Les rubis gardés alimentent la **Boutique de Kakariko** (améliorations permanentes) et la **Fontaine de la Grande Fée** (déblocages).
- Option à discuter : des **reliques** trouvées dans les coffres et ramenées pour être équipées avant une partie (façon *Halls of Torment*).

---

## 9. Questions qui ont besoin de ton avis

1. **Moteur** : valides-tu le **moteur sur mesure en Rust (wgpu)** ? Ou préfères-tu que je mesure d'abord Godot (téléchargement ~70 Mo) ?
2. **Musiques** : il n'y en a aucune dans le dossier. Peux-tu fournir les pistes (OGG/MP3/FLAC ou SPC) ? Sinon, préfères-tu des musiques libres de droits au style SNES, ou un premier prototype sans musique ?
3. **Usage** : confirmes-tu un projet **strictement privé** (y compris la future version Web) ? Ça conditionne la gestion des crédits et l'hébergement Web.
4. **Personnages additionnels** : les styles NES/Game Boy jurent avec le SNES. On les accepte tels quels, on applique une **harmonisation automatique** (palette et contour ajustés au style SNES), ou on se limite aux sprites de style SNES/GBA ?
5. **Les 5 maps et leurs boss** : la répartition te convient-elle ?
6. **Bascule Lumière/Ténèbres** : est-ce qu'on en fait une mécanique centrale (dans toutes les maps), un événement ponctuel, ou on la garde pour plus tard ?
7. **Compétence active** : jeu 100 % automatique comme *Vampire Survivors*, ou une compétence manuelle optionnelle (attaque tournoyante, sprint des Bottes de Pégase) ?
8. **Méta-progression** : boutique permanente façon *Vampire Survivors*, reliques façon *Halls of Torment*, ou les deux ?
9. **Cadrage** : 640×360 (champ large, lisibilité de la horde — ma recommandation) ou un cadrage plus serré, plus proche du SNES (≈ 427×240) ?

---

### Fichiers produits pour cette étude
- `research/benchmarks/sim_core/` — simulation de horde en Rust (natif + WASM)
- `research/benchmarks/js_sim/` — même simulation en JavaScript
- `research/benchmarks/wgpu_render/` — rendu instancié sur mesure (wgpu)
- `research/benchmarks/bevy_render/` — rendu Bevy (avec l'atlas d'ennemis généré)
- `research/benchmarks/webgl_render/` — rendu WebGL2 dans le navigateur
- `research/tools/autoslice.py`, `research/tools/count_sprites.py` — prototypes de découpe automatique

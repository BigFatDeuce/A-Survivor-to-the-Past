# A Survivor to the Past — Feuille de route de développement

*Validée le 24/11/2026 : moteur Rust sur mesure, gameplay §8 du rapport n°1, 5 maps, bascule Lumière/Ténèbres,
dash + attaque manuelle/automatique (bascule par touche), méta-progression boutique + reliques, 640×360 propre en 4K
et à haut framerate, persos NES/Game Boy gardés tels quels (« persos rétro »), usage privé avec accès Web privé plus tard.*

## Décisions techniques prises (petites décisions, pas besoin de validation)
- **Rendu « pixels nets, mouvement fluide »** : la scène est dessinée directement à la résolution de l'écran, chaque
  pixel du jeu devenant un carré de N×N pixels (N entier : ×3 en 1080p, ×4 en 1440p, ×6 en 4K). Les positions sont
  calées sur les pixels de l'écran, pas sur la grille 640×360 : déplacements et caméra parfaitement fluides à 144/165/240 Hz,
  sans flou ni pixels déformés. Rotations et zooms d'effets sont calculés dans la grille du sprite (aspect SNES conservé).
- **Écrans larges** : hauteur logique 360, largeur adaptée au format (pas de bandes noires en 21:9).
- **Simulation à pas fixe 60 Hz** + interpolation de rendu : même comportement à 60 ou 240 Hz.
- **Données en RON** (lisible, commentaires autorisés) dans `data/`.
- **Musiques** : FLAC → OGG, points de boucle détectés automatiquement (`research/music_loops.json`), ajustables à la main
  dans `data/audio.ron`.

## Jalons
| # | Jalon | Contenu | Critère de fin |
|---|---|---|---|
| J1 | **Socle jouable** | Moteur (fenêtre, plein écran, rendu net 4K/240 Hz, audio, clavier/manette), pipeline d'assets, Link (marche, dash, épée manuelle/auto), horde, collisions, rubis/XP, premiers effets de « juice » | Une arène où l'on fauche des centaines de soldats à 165 fps |
| J2 | Combat complet | Armes et passifs en données, montée de niveau (choix 1/3), évolutions, coffres, ennemis variés (comportements), élites (palettes), boss générique | 8 armes, 8 passifs, 15 ennemis jouables |
| J3 | Map 1 complète | Tuiles extraites des cartes, collisions, vagues chronométrées, mini-boss + Armos Knights, porte de sortie, bascule Lumière/Ténèbres, météo | Une partie de 10 min de bout en bout |
| J4 | Interface | Titre, menus, sélection perso/map/difficulté, préparation, pause, fin de partie, statistiques, options, sauvegarde | Parcours complet sans clavier ni souris (manette) |
| J5 | Méta-progression | Boutique de Kakariko, Grande Fée, reliques, pendentifs, Sceaux de Ganon, 4 difficultés | Progression cohérente sur plusieurs parties |
| J6 | Maps 2 à 5 | Bois Perdus, Désert, Lac gelé, Pyramide (Agahnim → Ganon) | 5 maps jouables |
| J7 | Personnages | Import automatique `AdditionalChars` + Atelier, Zelda/Sonic/Link rétro | Déposer une planche = personnage jouable |
| J8 | Finition | Équilibrage, passe de juice, optimisation, tests de charge, build final | Version locale stable |

Chaque jalon se termine par un point avec toi (démo + questions éventuelles).

## État d'avancement

### J1 — Socle jouable : terminé (24/11/2026)
- **Lancer** : `cargo run --release -p game` (ou `target/release/a_survivor_to_the_past.exe`).
  Options : `--fullscreen`, `--novsync`, `--bench 3000` (test de charge), `--demo`, `--shot fichier.png 5`.
- **Commandes** : ZQSD/flèches/stick gauche ; attaque Espace/clic gauche/X/gâchette droite ; dash Maj/clic droit/A ;
  bascule auto/manuel Tab/R/Y ; pause Échap/Start ; plein écran F11 ou Alt+Entrée ; infos F3 ; capture F12.
- **Contenu** : arène sans fin (`data/maps/arena_test.ron`), Link (marche, dash avec invulnérabilité, épée en arc),
  6 ennemis (soldats vert/bleu, serpent, rat, Keese, Tektite) avec 3 comportements, rubis = expérience, petits cœurs,
  bonus automatiques à chaque niveau + onde de choc (les choix 1 parmi 3 arrivent au J2).
- **Juice** : flash blanc + écrasement des ennemis touchés, recul, étincelles orientées, nuages de mort ALttP, chiffres
  de dégâts (critiques en jaune), tremblement d'écran, arrêts sur image (coup reçu, niveau), images rémanentes du dash,
  gamme montante des rubis ramassés en chaîne, onde de choc et annonce de niveau, bip de vie basse, cœurs qui tremblent.
- **Mesures (PC de bureau récent)** : 3000 ennemis à ~1700 i/s en 1080p (simulation 0,26 ms/pas) ; 5000 ennemis à
  ~940 i/s en 4K (×6) ; plein écran 2560×1440 verrouillé à 165 Hz. Pixels nets à toutes les échelles.
- **Non vérifié** : manette (aucune branchée pendant les essais), écoute des boucles musicales, écran de Game Over.

### J2 — Combat complet : terminé (24/11/2026)
- **Commandes (changées)** : dash sur **Espace** (aussi K, clic droit, A, gâchette gauche), recharge 1,8 s ;
  attaque manuelle J, clic gauche, X, gâchette droite. Toutes les touches sont décrites dans `data/input.ron`
  (prêtes pour l'écran de réattribution, prévu avec les menus du J4).
- **Armes (8 + 8 évolutions)**, `data/weapons.ron` : Épée → Épée de légende (rayons) avec Réceptacle de cœur ;
  Arc → Flèches d'argent (Carquois) ; Boomerang → Boomerang magique (Bottes de Pégase) ; Baguette de feu → Médaillon
  Bombos (Lanterne) ; Baguette de glace → Médaillon Éther (Demi-magie) ; Marteau → Médaillon Séisme (Gant de puissance) ;
  Bombes → Super bombe (Bouclier miroir) ; Canne de Byrna → Ronde des fées (Filet à papillons).
  Une évolution = arme au niveau max + objet possédé, révélée dans un coffre.
- **Objets passifs (8)**, `data/passives.ron` : bonus par niveau (puissance, zone, recharge, quantité, durée, vitesse,
  course, cœurs, armure, attraction, recharge du dash, régénération).
- **Niveaux** : choix 1 parmi 3 (cartes : nom, niveau, effet exact du niveau, partenaire d'évolution), 6 armes et
  6 objets maximum (cases vides visibles), clavier/manette/souris, touches 1-3, délai anti-erreur de 0,35 s.
- **Ennemis (15 + boss)**, `data/enemies.ron` : poursuite, vol en zigzag, bonds, flânerie, charge annoncée, tir à
  distance (Octorok, archer, Moblin). Élites (contour doré, ×12 PV) qui lâchent un coffre. Boss générique Hinox à 5:00
  (bombes en cloche, charges, renforts), affiché ×2, barre de vie, musique dédiée, grand coffre.
- **Langues** : anglais par défaut ; français, allemand, espagnol, italien, portugais, japonais (`data/lang/*.ron`,
  régénérables avec `research/tools/gen_lang.py`). Police unique tirée des planches FR/DE/JP du jeu.
- **Options** (Échap → Options, enregistrées dans `settings.ron`) : langue, filtre sonore rétro (Aucun / Doux / SNES,
  SNES par défaut), volumes général/musique/effets, plein écran.
- **Retours J1 appliqués** : épée seule moins forte (portée courte, 3 cibles par coup, dégâts réduits), dégâts au contact
  relevés, onde de niveau sans dégâts, son adouci (volumes, compresseur, filtre), interface agrandie, vie en cases sous Link
  (+ jauge de dash pendant la recharge).
- **Juice** : directeur de ralentis (grosse vague vaincue, gros impact, boss) avec délai minimal et budget de 0,55 s de
  ralenti par ~30 s ; quand il refuse, les effets restent mais le temps n'est pas touché. Explosions, colonnes de feu,
  ondes du marteau, gel, brûlure, étourdissement, annonces d'attaque ennemie, flèches vers les coffres hors écran.
- **Options d'essai** : `--demo` (pilote automatique), `--time 290`, `--give bow,quiver,chest`, `--menu options`,
  `--lang ja`.

### Publication et confort : fait le 24/11/2026 (hors jalons)
- **Écran titre** : fond `menubg` agrandi/rogné pour toutes les résolutions, logo au centre-droit, menu à gauche
  (Jouer, Options, À propos, Quitter), étincelles sur le lac et l'épée, filtre SNES. Retour au titre depuis la pause
  et l'écran de fin.
- **À propos** : fan game non officiel, gratuit, revente interdite, assets de Nintendo, projet fait pour le plaisir ;
  liens Discord, GitHub, site (`data/links.ron`).
- **Filtres vidéo** (options, `settings.ron`) : Aucun, SNES (défaut : flou composite léger + fuite de chroma +
  scanlines douces), CRT (courbure, masque, halo), Scanlines, GBA (grille LCD + couleurs), Game Boy (4 verts +
  tramage) ; **format 4:3** (bandes latérales). **Filtres audio** : Aucun, Doux, SNES, GBA, Game Boy
  (rééchantillonnage, réduction de bits, mono).
- **Vérification automatique de l'interface** (`--uicheck`) : 7 langues × 6 formats de vue × tous les écrans ; textes
  hors écran, chevauchements, débordements de cadre, caractères absents. État : 0 problème sur 882 écrans.
  Majuscules accentuées (É È À Á…) composées dans la police.
- **Icône B** (Link et la horde) : fenêtre, exe Windows, favicon (`research/tools/export_icon.py`).
- **Dépôt GitHub public** avec README en 7 langues et captures (`research/tools/make_readme.py`) ; **site** en
  7 langues sur le sous-domaine holy-c.men (`site/`, `research/tools/make_site.py`, déploiement `npx wrangler deploy`).

- **Version Windows jouable (alpha 0.2.0)** publiée sur GitHub Releases (`research/tools/make_release.py` :
  dossier « A Survivor to the Past » avec l'exe, `data/`, `assets/gen`, `assets/audio`, licence, lisez-moi FR/EN).
- **Mises à jour intégrées** (`crates/game/src/update.rs`) : au lancement, lecture de la dernière version publiée ;
  mode Automatiques (défaut) = téléchargement en arrière-plan puis installation au lancement suivant ou tout de
  suite (« Redémarrer et installer » sur le titre) ; mode Me demander ; mode Non (aucune connexion). Archive
  contrôlée (dépôt officiel uniquement, taille, empreinte SHA-256 de GitHub, chemins de l'archive vérifiés),
  installation jeu fermé avec retour arrière en cas d'erreur, réglages du joueur conservés. Essai local de bout
  en bout : `--update-source dossier` (latest.json + archive).
- **Plantages** : `crash.log` + message au joueur ; message clair si le jeu est lancé sans ses fichiers (zip non extrait).

### Version 0.3.0 : publiée le 24/11/2026 (Windows, navigateur, site)
- **5 stages** (`data/maps/*.ron`) débloqués l'un après l'autre : Plaine d'Hyrule, Bois Perdus, Désert du Mystère,
  Lac gelé, Pyramide de la Puissance. Chacun a sa météo (pluie, brume, tempête de sable, blizzard, ténèbres), ses
  musiques, un boss de mi-parcours à 4:30 et un boss final à 9:00 (Armos, Mothula, Lanmola, Kholdstare, Agahnim puis
  Ganon) ; la victoire ouvre une porte de sortie. Écran de choix du stage avec aperçu, boss et meilleur temps.
  **Écart** : le sol est procédural avec des décors (pas encore les tuiles extraites des vraies cartes) ; Lanmola
  n'a qu'un sprite ; trois boss de mi-parcours réutilisent les boss finaux d'autres stages.
- **Méta-progression** (`data/meta.ron`, `crates/game/src/meta.rs`, `shop.rs`) : rubis mis en banque à la fin de
  chaque partie ; boutique de Cocorico (9 bonus permanents), bénédictions de la Grande Fée, une relique par stage
  (2 équipées au maximum), 4 difficultés, 5 Sceaux de Ganon (défis qui augmentent les rubis gagnés).
- **Profil** (`records.rs`) : bestiaire, armurerie (recettes d'évolution découvertes), statistiques, export / import
  de la sauvegarde en code texte (presse-papiers ; boîte de saisie dans le navigateur).
- **Commandes** (`controls.rs`) : réaffectation de chaque action au clavier et à la manette, réinitialisation.
  **Écart** : le nom d'une touche suit la disposition QWERTY tant qu'elle n'a pas été pressée une fois.
- **Écran titre** : pluie et orage à la place du filtre SNES, longue musique en boucle avec fondu enchaîné ;
  en jeu, aucun filtre vidéo par défaut. **Écart** : le filtre audio reste « SNES » par défaut.
- **Version navigateur** (`research/tools/make_web.py`, `site/play/`) : WebAssembly, WebGPU avec repli WebGL2,
  données du jeu dans un seul paquet (`game.pak`), musiques lues en flux par la page, sauvegarde dans le navigateur.
  **Écart** : dans le navigateur, la musique ne passe pas par le filtre audio. Page servie avec sa propre politique
  de sécurité (`'wasm-unsafe-eval'`, tout reste sur le domaine).
- **Site et README** : bouton « Jouer dans le navigateur », nouvelles captures (choix du stage, boutique), textes à
  jour en 7 langues. Vérification de l'interface : 0 problème sur 1764 écrans.

### Version 0.4.0 : publiée le 25/11/2026 (Windows, navigateur, site)
- **Menus** : la validation joue un tintement court (`message_finish`) ; la fanfare reste réservée au choix
  d'amélioration en partie.
- **Personnages** (`data/characters/*.ron`, `crates/game/src/chars.rs`) : écran de choix avant le stage (portrait
  animé, cœurs, vitesse, arme de départ, bonus). Link ; Link Minish (Minish Cap, débloqué en gagnant la Plaine
  d'Hyrule : chance, vitesse, boomerang) ; Link sans bonnet (3000 ennemis vaincus : force, zone, armure, bombes).
- **8 armes** : Grappin → Super grappin (Palmes de Zora), Poudre magique → Poudre de sorcière (Champignon),
  Canne de Somaria → Grande Somaria (Livre de Mudora), Abeilles → Abeilles dorées (Cape magique).
  **4 objets** et **2 statistiques** : Chance (coups critiques, objets du décor) et Croissance (expérience), aussi
  à la boutique. **Écart** : le bloc de Somaria est le bloc de la Tour de la Tortue, ses rayons utilisent l'Étincelle.
- **Bob-omb** (Minish Cap) : ennemi rare (une à la fois, toutes les 60 à 100 s), halo, étincelles, flèche quand elle
  est hors de l'écran ; la vaincre souffle tous les ennemis à l'écran sauf les boss (flash, explosions, rubis).
- **Décor cassable** : buissons (moins nombreux sur la Plaine), plantes du désert, pierres de la Montagne ; tirage
  au sort : rien, rubis, cœur, ou rarement filet (aspire les rubis), fée (soin complet), médaillon d'Éther (gel).
  **Écart** : le filet utilise l'icône du filet à papillons ; pas de jarres (aucun sprite de jarre au sol).
- **Stage 6 bonus : Montagne de la Mort** (ouverte par la victoire à la Pyramide) : sol en tuiles de la vraie carte,
  rochers, Lynels (nouvel ennemi, boules de feu), Lanmola en mi-parcours, Agahnim au sommet.
- **Juice** : corps projetés, poussière de pas, bord rouge quand la vie est basse, nuage de poudre avec sa portée,
  chaîne du grappin, éclats du décor. Traductions des 7 langues ; vérification de l'interface : 0 problème
  sur 2142 écrans.

## Idées du 24/11/2026 : propositions (à valider)
| Idée | Proposition | Jalon proposé |
|---|---|---|
| **Classement en ligne** par map, avec pseudo | Petit service Cloudflare (Worker + base D1) sur le même domaine. La simulation est déterministe : le jeu envoie la graine et les entrées de la partie, le serveur rejoue la partie (simulation compilée en WASM) et n'enregistre que les scores vérifiés. Pseudo filtré, limite d'envois. | J8 |
| **Export / import des sauvegardes** | Profil dans un seul fichier ; boutons Exporter / Importer (fichier, ou code à copier-coller pour le navigateur et Android). | **Fait (0.3.0)** |
| **Bestiaire, armurerie, statistiques** | Bestiaire (ennemis vus/vaincus, description, sprite animé), armurerie (armes, évolutions découvertes et leur recette), statistiques (parties, temps, ennemis vaincus, records par map). | **Fait (0.3.0)** |
| **Toutes les plateformes** | Windows, Linux, macOS, Android, navigateur : **uniquement quand tu diras que le jeu est prêt**. Points à prévoir : commandes tactiles (Android), musiques en flux (navigateur, 50 Mo), signature macOS. | Windows et **navigateur faits (0.3.0)** ; Linux, macOS, Android sur ton signal |
| Remappage des touches | Écran de réaffectation (les liaisons sont déjà en données). | **Fait (0.3.0)** |

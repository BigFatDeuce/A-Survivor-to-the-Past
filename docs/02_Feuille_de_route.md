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

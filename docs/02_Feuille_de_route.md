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

### Version 0.5.0 : publiée le 25/11/2026
- **Décor cohérent** : la tuile de sol « buisson peint » (identique au buisson cassable) est remplacée par des fleurs
  (Plaine, Bois Perdus) ; moins de buissons, mais tous se coupent. Bob-omb encore plus rare (170 à 240 s).
- **HUD** : compteur de rubis rouges (rubis mis en banque si la partie s'arrêtait maintenant, plus en cas de
  victoire) ; icône de l'épée à côté d'Auto/Manuel (le mode ne concerne que l'épée), icône d'épée lame vers le haut ;
  textes remontés dans les cadres (`nudge` de police) et dans les cartes de choix.
- **Personnages** : Sonic (vainqueur du Désert : très rapide, dash quasi sans attente, épée, mais fragile et zone
  réduite), Zelda (5 boss : canne de Somaria, durée, expérience, recharge, mais lente et moins puissante), Link NES
  (niveau 25 : arc, une flèche de plus, mais moins de dégâts et de zone). Chaque personnage a maintenant des défauts
  (Link : peu de chance ; Minish : moins de dégâts ; sans bonnet : recharge plus lente). Icône de l'arme de départ
  dans l'écran de choix. **Écart** : Sonic © SEGA, sprites personnalisés crédités dans le README.
- **Rencontres** (`crates/sim/src/encounter.rs`, `crates/game/src/reward.rs`, `rules.ron` `encounter`) : vers 1:15
  puis toutes les 100 à 150 s, un champion errant (élite violette, vie x3) attend hors de l'écran ; annonce, flèche
  au bord de l'écran avec distance et temps restant (70 s). Vaincu : choix entre amélioration spéciale d'une arme
  (gardée à l'évolution), relique de rencontre (effet immédiat, gardée si la partie est gagnée) et bénédiction.
- **Reliques** : 15 au total (Miroir magique pour la Montagne de la Mort, 8 reliques de rencontre) ;
  **écart** : 3 emplacements au lieu de 2.
- **Six personnages de plus (12 au total)**, planches de `AdditionalChars` / `MoreSprites` découpées par
  `research/tools/gen_chars.py` (planches « Derived » réalignées) : Link de *Four Swords* (un seul personnage,
  4 couleurs au choix avec gauche / droite, gardées dans le profil), Link de *Link's Awakening*,
  Link adulte (style Chrono Trigger, ombre retirée), Link Mojo, Maskass, et **Shadow Link débloqué en battant
  Ganon** (`UnlockDef::Slain`). Liste des personnages qui défile. **Écarts** : Link enfant non repris (planche
  sans cycle de marche) ; Shadow Link est
  un personnage à part (même planche que *Four Swords*), pas une couleur.
- **Flèches d'indication** (champion, Bob-omb, coffres hors écran) : sprite de flèche de la police d'*A Link to the
  Past*, tourné par quarts de tour, au lieu d'une flèche dessinée.
- **0.5.1** : Link de *Zelda II* retiré (planche de profil seulement) ; écran de chargement web avec un seul
  message traduit (« No Nintendo lawyers approved this game! »).
- **Crédits** : `CREDITS.md` liste l'auteur de chaque planche (nom inscrit sur la planche), lien dans les README.
- Traductions des 7 langues (`lang_v05.py`, `lang_v05b.py`) ; tests 10/10 ; vérification de l'interface :
  0 problème sur 2940 écrans.

### Version 0.7.0 (26/11/2026) : tous les lots « 0.6.1 » à « 0.6.5 » + toutes les plateformes
- La 0.7.0 regroupe les sept lots de retours sur la 0.6.0 ci-dessous (site, commit et release GitHub).
- **Plateformes** : Windows x64 (compilé ici), **Linux x64**, **Linux ARM64**, **macOS** universel (Apple Silicon +
  Intel, application `.app` signée ad hoc, non notariée : `xattr -cr` expliqué dans le lisez-moi) et **consoles
  portables** par PortMaster (Anbernic RG40XX H et autres consoles Linux ARM64), **expérimental** : pas testé sur
  une vraie console. Les exécutables Linux / macOS sont compilés par GitHub Actions (`.github/workflows/builds.yml`,
  lancé à la main : `gh workflow run builds.yml` puis `gh run download <id> -D dist/bin`), les paquets sont faits ici
  par `make_release.py --platform windows-x64|linux-x64|linux-arm64|macos|portmaster` (livrets PDF compris).
- **Mise à jour automatique** sur Linux et macOS (archive de la plateforme, droit d'exécution gardé ; sur macOS la
  racine du jeu est `.app/Contents/MacOS`). Désactivée par `ASTP_NO_UPDATE` (lanceur PortMaster).
- **Consoles** : le lanceur monte le module Westonpack de PortMaster (Weston + Xwayland, EGL / OpenGL ES de Crusty) et
  lance le jeu en X11 avec `WGPU_BACKEND=gl` (le moteur lit maintenant `WGPU_BACKEND`). Boutons -> clavier par
  gptokeyb (`asurvivortothepast.gptk`). Risques : pilote GL de Crusty avec wgpu, 1 Go de mémoire, atlas 4096 px.

### Retours sur la 0.6.0, septième lot « 0.6.5 » (26/11/2026) : en local seulement (rien de publié)
- **350 personnages** (choix validés : tous jouables, remplacements, couleurs des palettes, stats automatiques, un
  tiers en boutique) : 328 nouveaux venus des sprites du *A Link to the Past Randomizer* (`.zspr`, snesrev). Ils
  sont dessinés avec les poses de marche de Link, retrouvées bloc par bloc dans les tuiles (`zspr_poses.py`, 0 pixel
  manquant, lapin compris) ; `gen_zspr_chars.py` écrit les planches (`assets/source/zspr/`), le manifeste, les
  personnages (`data/zspr_characters.ron`), les textes (`lang_zspr.py`) et les crédits (CREDITS.md, entre
  marqueurs). Jusqu'à 4 couleurs chacun : cottes verte / bleue / rouge sans les doublons (nommées d'après la
  teinte dominante) et forme du Monde des Ténèbres (« Ténèbres », propre à chaque sprite).
- **Remplacés** (même identifiant, progression gardée) : Link Minish, Link NES, Maskass, Vivi, Sonic, Crewmate,
  Ganondorf, Zelda, Link Four Swords, Shadow Link, Link's Awakening, Mario, Zora des mers. Les anciennes
  planches et 7 tenues de la garde-robe 0.6.4 qui en dépendaient (Link NES vert / rouge, Luigi, Maskass x3, Super
  Sonic) sont retirées ; Luigi est maintenant un personnage à part entière.
- **Stats** tirées d'un hachage de l'identifiant (2 avantages, 1 défaut, arme d'après le nom quand il évoque
  quelque chose). **Déblocages** : 105 en vente (onglet **Personnages** de la boutique, 1000 à 20000 rubis rouges),
  les autres par exploits (ennemis, boss, niveau, ennemi vaincu - son propre sprite quand il existe -, stage
  gagné). Ganondorf demande toujours la troupe d'origine seulement. Écran des déblocages : 16 au plus à la fois.
- **Écran des personnages** : six univers (Zelda, Nintendo, RPG, Jeux vidéo, Séries et films, Communauté), choisis
  au-dessus de la grille (gauche / droite, clic sur les flèches) ; la grille défile ; fiche avec l'auteur du sprite.
- **Atlas** : images identiques rangées une seule fois, atlas rectangulaire coupé au contenu (4096 x 2924, PNG en
  compression maximale : 3,8 Mo, contre 1,2 Mo avant). Trophée des personnages : 10 / 50 / tous.
- Tests 26/26, vérification de l'interface : 0 problème sur 18942 écrans, Clippy sans avertissement. Risque : ces
  sprites n'ont pas de licence (crédit de chaque auteur, retrait sur demande) et beaucoup ne sont pas de Nintendo.

### Retours sur la 0.6.0, sixième lot « 0.6.4 » (26/11/2026) : en local seulement (rien de publié)
- **Annonces** : bandeau centré sur l'encre des majuscules, lue dans la police (décalage `nudge` et hauteur de
  la lettre A) : le texte touchait le bord du haut. Mesuré au pixel à quatre formats d'écran (5 px au-dessus et
  au-dessous, 24 px à gauche et à droite) ; capture `--menu banner`.
- **Rencontres** : le Bob-omb et le voleur de rubis ont trois fois la vie moyenne d'un ennemi ordinaire du stage
  (avant : un tiers du boss de mi-parcours) et une barre de vie (orange pour le Bob-omb). Le voleur est maintenant
  un **Boo** (planche Boo Buddy de JulioTanaka) ; profils migrés (version 4 : victoires et figurine du Pikit).
- **Garde-robe** : 12 nouvelles tenues : Link lapin et l'anneau rouge du Link NES (couleurs des planches) et
  10 recolorations (`research/tools/gen_wardrobe_v064.py` : tuniques Goron et Zora, Link NES vert, Toon Link rouge
  et bleu, Luigi (Mario en vert), Maskass bleu / vert / jaune, Super Sonic). Les lignes sans icône montrent la tenue.
- **Boutique** : la Grande Fée est le premier onglet, la sorcière en tête (portrait, nom en rose chatoyant).
- **Menu** (choix validé) : Profil remplacé par **Recueil** (bestiaire, armes, objets, fusions) et **Exploits**
  (trophées, statistiques) ; la sauvegarde passe dans les Options (depuis l'écran titre seulement).
- **Trophées** (choix validé : bronze / argent / or) : 21 familles, 59 trophées calculés d'après le profil (les
  parties déjà jouées comptent), `crates/game/src/trophies.rs` ; capture `--menu trophies`.
- Tests 26/26 (dont trophées et vie du voleur), Worker 11/11, vérification de l'interface : 0 problème sur 4704
  écrans, Clippy sans avertissement. Manuel (`manual_text.py`) et README : Boo et nouveau menu (PDF à régénérer
  à la prochaine release).

### Retours sur la 0.6.0, cinquième lot « 0.6.3 » (26/11/2026) : en test sur le site seulement (ni commit ni release, version 0.6.0)
- **10 stages dans l'ordre du jeu** (plaine, désert, Montagne de la Mort, Château d'Hyrule, Palais des Ténèbres,
  Grand Marais, Bois Perdus, Village des Voleurs, Lac gelé, Pyramide) : 4 nouveaux stages (sols, météo, musiques,
  bestiaire de la zone), boss de mi-parcours et boss final fixés par stage (`SpawnAt.hp`), difficulté croissante
  (`MapDef.power` : vie et dégâts des ennemis). Ganon est le dernier et le plus fort (test). 14 ennemis ajoutés
  (soldat rouge, Kodongo, Eyegore, Zazak, Gibdo, Snapdragon, Zol, Ropa, Bari, Zora, Pengator, Stalfos Knight,
  Lynel géant, chevalier au fléau). Un stage déjà gagné reste ouvert malgré le nouvel ordre ; les anciens scores
  en ligne ne sont plus comparables. Tuiles de sol vérifiées sans coutures visibles (`check_tiles.py`).
- **Master Sword, fusion ultime** : débloquée en battant Ganon (annonce et animation), Épée + Bouclier miroir au
  maximum, environ 25 % au-dessus des meilleures fusions. L'ancienne fusion Épée + Réceptacle devient l'Épée de
  combat ; profils migrés (version 3). Les fusions apparaissent aussi dans les choix de niveau quand les deux
  objets sont au maximum (plus rares). Bestiaire du plus faible au plus fort ; armurerie en onglets Armes /
  Objets / Fusions.
- **Diseuse de bonne aventure** à la place du grand coffre du boss final (sprite recomposé à partir des morceaux
  de la planche, `gen_npcs_v063.py`) : jeu des coffres (mise de banque, x3 / x1 / rien) et quitte ou double sur
  la banque de la partie (x2 ou x1/2), une fois chacun ; partie et chrono en pause.
- **Sorcière** (Grande Fée, 10 000 rubis) : passe une fois par partie entre 2:00 et 5:00, reste 30 s, bulle « ! »
  et flèche ; trois potions (1 ou 2 bonus et un malus, valeurs tirées dans des fourchettes) pour toute la partie,
  payées en rubis de la partie : prix selon la force du tirage, le stage, la difficulté et l'heure (x1,6 à 2:00).
  Potion bue : animation et son.
- **Rubis rouges après tout débloqué** (aucun effet en partie) : garde-robe (cottes de mailles bleue et rouge de
  Link, palettes du jeu, `gen_wardrobe_v063.py`), juke-box (28 musiques, choix de la musique du titre), offrandes
  à la Grande Fée (3 titres, pseudo en couleur au classement en ligne), figurines du bestiaire (socle doré).
- **Classement en ligne** : champ `title` côté jeu et Worker, colonne en base. À la mise en ligne : exécuter
  `worker/migrations/0063_title.sql` sur la base AVANT `npx wrangler deploy` (le nouveau Worker lit la colonne) ; fait le 26/11/2026.
- **Interface** : fin de partie lisible (notes groupées, deux colonnes si besoin), choix des personnages en grille
  (tous, verrouillés en gris, personnage animé au survol, couleur avec Q / E), nouveau contrôle `mal centre` dans
  la vérification des écrans, annonces plus courtes et animées en douceur, Bob-omb et Pikit voleur plus gros avec
  une aura et un tiers de la vie d'un boss de mi-parcours.
- Tests 25/25, Worker 11/11, vérification de l'interface : 0 problème sur 4662 écrans ; compilation web vérifiée.
  Clippy : 0 avertissement (`to_title` renommé `quit_to_title`, alias de type dans `online.rs`, `allow` habituel
  sur les fonctions de dessin à nombreux paramètres).

### Audit technique (26/11/2026) : en test sur le site seulement
- **Fenêtre fermée en pleine partie** : la partie était perdue (rubis en banque, bestiaire, statistiques) ;
  elle compte maintenant comme un abandon, comme « Écran titre » (`Game::exit`). Les sessions pilotées par
  script (`--seconds`, captures) n'enregistrent rien (`Options::scripted`).
- **Classement en ligne** : une partie qui ne resterait pas au classement (le pseudo a déjà 3 parties au moins
  aussi bonnes) n'est plus écrite en base. Avant, elle était insérée puis effacée aussitôt, donc jamais comptée
  par la limite d'envois : des envois sans fin faisaient autant d'écritures. La lecture refuse aussi les stages
  inconnus (chaque nom inventé contournait le cache). Vérifié sur un serveur local (`wrangler dev --local`).
- **Brûlure** : une brûlure terminée gardait sa force pour la suivante (remise à zéro seulement si elle finissait
  sur un pas de dégâts) ; une brûlure faible héritait d'une forte. Test ajouté.
- **Réglages illisibles** : copie dans `settings.broken.ron` avant le retour aux réglages par défaut.
- **Mises à jour** : test de l'extraction (chemins `..` refusés, chemin absolu ramené dans le dossier).
- Lint (clippy) nettoyé ; restent 4 avertissements de style assumés (fonctions de dessin à nombreux paramètres).
- **Multiplicateur de score** : le serveur refuse au-delà du maximum réel du jeu (`MAX_MULT`, difficulté la plus
  payante x tous les Sceaux = 4,05), recalculé depuis `data/meta.ron` par `make_site.py` à chaque mise en ligne.
- **Onglet fermé en pleine partie** (navigateur, aucun événement de fermeture) : la partie est enregistrée comme
  un abandon toutes les 10 s et quand la page perd le focus, sans toucher au profil en mémoire ; la vraie fin de
  partie remplace ensuite cet enregistrement. Vérifié : une seule partie comptée.
- **« Recommencer » depuis la pause** : l'écran des déblocages s'affiche avant la nouvelle partie.
- `ui_check` (contrôle des écrans) sorti de `game.rs` dans `game/uicheck_screens.rs`.
- Reste volontairement large : la borne de durée (2 h) et de nombre d'ennemis du classement.

### Retours sur la 0.6.0, quatrième lot (26/11/2026) : en test sur le site seulement
- **Fusion mise en scène** : quand un coffre fait évoluer une arme, le coffre tremble et s'ouvre, l'arme et son
  objet partenaire en sortent en spirale, des éclairs les relient, puis flash, anneaux et gerbe d'étincelles :
  l'arme évoluée apparaît en grand, avec son nom, « Fusion : arme + objet » et ses statistiques avant / après
  (dégâts, projectiles, recharge, zone). De retour en jeu, l'arme éclate autour de Link et sa case du HUD brille en
  rose quelques secondes. Un coffre qui contient une fusion tombe avec des anneaux roses.
- **Noms mis en valeur dans les textes** : objets, armes et boss en or, fusions en rose qui chatoie, nouveautés en
  vert, avec un reflet qui parcourt le mot et quelques étincelles (choix de la fusion, cartes de niveau, recueil,
  conditions de déblocage, écran de victoire). Balises invisibles dans les chaînes (`hud::hl`, `hl_fusion`,
  `hl_new`), refermées et rouvertes quand une ligne est coupée.
- **Écran des déblocages** en fin de partie (victoire, défaite ou retour au titre) : **tous** les personnages et
  armes débloqués apparaissent l'un après l'autre, avec gerbe, rayons et le personnage qui marche ; un appui
  affiche tout d'un coup. Remplace l'ancienne bannière qui ne montrait que le premier personnage.
- **Armes à mériter** : les six armes de la 0.6.2 ne sont proposées qu'après avoir vaincu une première fois un
  mini-boss (lance-pierre : Chevalier Armos, fanal : Blind, plume de Roc : Lanmola, jarre magique : Arrghus,
  ocarina : Moldorm, Feu de Din : Agahnim ; champ `unlock` de `data/weapons.ron`). Annonce en jeu « Nouvelle arme
  débloquée ! Elle peut maintenant apparaître dans toutes les parties. », arme disponible dès la même partie,
  enregistrée dans le profil (`weapons`) ; le recueil indique le mini-boss à vaincre.
- **Personnages nouveaux** : nom en vert qui scintille dans le choix des personnages tant qu'on ne les a pas
  joués (profil `new_chars`).
- **Menu pause** : « Fusions possibles », une carte par arme en main qui peut évoluer (arme + objet partenaire,
  arme évoluée en silhouette tant qu'elle n'a jamais été obtenue ; rose qui pulse = prête au prochain coffre,
  partenaire déjà utilisé = barré), à la place de l'aide des commandes.
- **Son de la fusion** : la fin du jingle *Unsealing the Master Sword* (piste 22, 5 s, `master_sword_fusion.ogg`).
- Choix validés : armes des mini-boss telles quelles, déblocage non rétroactif, arme proposable dès la partie
  où le mini-boss tombe.
- **Vivi** : la marche de profil utilisait deux poses de combat (il glissait) ; remplacée par le vrai cycle de 5
  images de la planche.

### Retours sur la 0.6.0, troisième lot (26/11/2026) : en test sur le site seulement
- **Personnages** (22 en tout) : Sheik (planche d'Oldskoolmario complétée : dos et profil dessinés pour le jeu,
  copie ajoutée au dossier AdditionalChars ; débloquée par Vitreous), Mario façon *Link's Awakening* (avance
  bouclier levé, part avec les bombes ; 15 boss), Skull Kid (Mothula), Simon Belmont (Agahnim), Vivi
  (Kholdstare), Ganondorf (**toujours le dernier** : nouvelle condition « tous les autres personnages
  débloqués », qui reste vraie quand on en ajoute). Planches : `research/tools/gen_chars_v062.py`.
- **Link de Link's Awakening** : deuxième apparence « Game Boy » (planche de *Super Mario Bros. Crossover*).
- **Canne de Somaria** : la statue Méduse (qui crache des boules de feu dans les donjons) remplace le bloc marron.
- **Souris dans les menus** : survoler une ligne la choisit sans faire défiler la liste (avant, la liste se
  recentrait et partait toute seule) ; la **molette** fait défiler le choix des personnages et des stages et les
  listes du hub ; au clavier et à la manette, la liste ne bouge que pour garder la sélection visible.
- Skul abandonné (planche de jeu de plateforme, profil seulement).
- **Six nouvelles armes** (pas d'armes tirées des planches de personnages : fouet et lames jumelles écartés),
  chacune avec son évolution : lance-pierre (graines en éventail ; + carquois = hyper lance-pierre), plume de Roc
  (saut invincible et onde de choc à l'atterrissage, nouvelle famille `Jump` ; + bottes de Pégase = cape de Roc),
  fanal (halo de feu ; + demi-magie = fanal bleu), jarre magique (souffle qui repousse ; + cape magique = jarre de
  Vaati), ocarina des fées (tornades qui errent, nouvelle famille `Tornado` ; + livre de Mudora = ocarina du
  temps), Feu de Din (explosion de feu ; + lanterne = courroux de Din, flammes au sol). Icônes : planche « Items
  (The Minish Cap-Style) » de ChaosMiles07 (`research/tools/gen_items_v062.py`). Le fanal s'appelle ainsi pour ne
  pas être confondu avec l'objet Lanterne.
- **Une seule fusion par objet partenaire** : six objets servent maintenant à deux armes, mais chacun ne fait
  évoluer qu'une arme par partie (la première prête ; si plusieurs sont prêtes en même temps, le coffre laisse le joueur choisir celle qui évolue) ;
  les cartes n'affichent plus un partenaire déjà utilisé.
- **Relancer / Passer** à la montée de niveau : 2 de chaque par partie (boutons sous les cartes, avec le nombre
  restant). Relancer = trois nouveaux choix ; Passer = le niveau sans rien prendre. En plus : boutique (« Regard
  neuf » et « Patience », 3 niveaux, +1 par niveau, hors Avarice) et deux reliques de rencontre (Monocle de
  Vérité +1 relance, Fragment de Kinstone +1 passe).
- **Lac gelé** : plus d'arbres, de souches ni de buissons ; à la place, le décor du vrai Lac de Glace (blocs de
  glace flottants et pieux en bois découpés dans la carte du monde des ténèbres, blocs du Palais de Glace
  cassables qui volent en éclats ; `research/tools/gen_ice_props_v062.py`). Icônes des nouvelles armes : la
  grille de la planche est décalée d'un pixel en bas, d'où une barre sombre sur le fanal et la jarre (corrigé).

### Retours sur la 0.6.0, deuxième lot (25/11/2026) : en test sur le site seulement
Mis en ligne sur le site pour essai, **sans release GitHub** ; le numéro reste 0.6.0.
- **Musique comme dans le jeu** (étude de `ZeldaAlttPSNESRecomp` : le pilote N-SPC joue l'intro une fois puis
  boucle sans couture ; les pistes MSU-1 ont un point de boucle à l'échantillon près ; fondu de sortie d'environ
  1,7 s ; volume baissé sur la carte ; même morceau demandé = pas de redémarrage). Une piste par contexte :
  Plaine = monde de la lumière, Bois perdus = forêt mystérieuse, Désert = donjons du monde de la lumière, Lac gelé
  = donjons du monde des ténèbres, Montagne = mont de la Mort, Pyramide = monde des ténèbres ; boss = colère des
  gardiens, Agahnim et Ganon ont leur thème. Fichiers « prêts à boucler » (`research/tools/music_loopify.py`) :
  le morceau s'arrête au point de boucle, ses 60 dernières ms sont fondues vers le début de la boucle, plus une
  demi-seconde de marge ; `loop_start` / `loop_end` dans `data/audio.ron` à l'échantillon près (32 kHz). En natif
  : région de boucle de kira ; dans le navigateur : `AudioBufferSourceNode` (loopStart / loopEnd) au lieu de
  l'élément audio, dont le saut à l'image près faisait un accroc. Changement : fondu de sortie de 1,7 s, le
  suivant démarre juste après ; musique baissée pendant la pause. Les flux vidéo (pochettes) des .ogg ont été retirés.
- **Un boss différent par zone**, les 12 boss d'*A Link to the Past* : Plaine (Chevaliers Armos puis Roi
  Helmasaur), Bois perdus (Blind puis Mothula), Désert (Lanmola puis Vitreous), Lac gelé (Arrghus puis
  Kholdstare), Montagne (Moldorm puis Trinexx), Pyramide (Agahnim puis Ganon). Le Hinox n'est plus un boss de
  stage (`bestiary: false` : absent du bestiaire tant qu'il n'a pas été vaincu).
- **Quatre personnages** (16 en tout) : Toon Link façon *Mario & Luigi* (grappin, rapide, 5 cœurs ; vaincre
  Blind), Link d'*Oracle of Ages* (baguette de feu, zone + ; gagner la Montagne), Zora des mers (canne de Byrna,
  armure + mais lent ; vaincre Arrghus), Crewmate d'*Among Us* (bombes, chance + ; 8000 ennemis) en 8 couleurs
  (échange de palette avec les couleurs du jeu d'origine). Planches : `research/tools/gen_chars_v061.py`.
- **Sceaux** : icônes des médaillons des sages (façon *Ocarina of Time*) : Force = Esprit, Nuée = Forêt,
  Fureur = Feu, Hâte = Eau, Famine = Ombre.
- **Bestiaire** : A ou un clic sur un ennemi déjà vaincu ouvre sa fiche (sprite animé à sa taille réelle en
  partie, qui se tourne dans les quatre directions, PV, dégâts, vitesse) ; gauche / droite passe au suivant.
- **Reliques actives** affichées pendant la partie, sous les objets (cases dorées).
- **Fusion** : seulement quand l'arme **et** son objet partenaire sont au niveau maximum ; la carte d'amélioration
  indique le niveau du partenaire possédé (ex. « Abeilles 3/8 »).
- **Mes parties** : les parties quittées et celles de moins de 30 s ne sont plus gardées (les anciennes sont
  retirées au chargement).
- **Clignotement rétro** des projectiles et des flammes qui vont disparaître.
- **Boutique** : retour à la ligne avant « Reliques équipées : » dans la fiche d'une relique.

### Retours sur la 0.6.0 (25/11/2026) : en test sur le site seulement
Mis en ligne sur le site (version navigateur) pour essai, **sans release GitHub** ; le numéro reste 0.6.0.
- **Musique** : listes de lecture par stage (fondus enchaînés de 5 s). **Remplacé** au lot suivant par une
  musique par contexte, bouclée comme dans le jeu (voir ci-dessus).
- **Flèches seules** au bord de l'écran (champion violet, Pikit vert, Bob-omb orange, coffres or) : plus de sprite
  ni de distance.
- **Menus à onglets** : gâchettes et boutons de tranche de la manette (Q / E ou Page préc. / suiv. au clavier)
  changent d'onglet ; le curseur peut monter sur la rangée des onglets (gauche / droite pour choisir).
- **Cadre de sélection** bien plus visible partout (double bordure dorée qui pulse, halo, coins).
- « Auto » et l'icône de l'épée alignés ; **Select** de la manette = plein écran ; curseur de la souris = la
  **fée** de l'écran de sélection d'*A Link to the Past* (curseur du système caché, la fée n'apparaît qu'à la souris).
- **Sons des choix** : aucun son pour un choix ordinaire ; arpège joyeux (`heart_container_1`) pour un choix
  important (fusion, niveau maximal, butin de champion).
- **Rubis verts** (ramassés pendant la partie) et **rubis rouges** (en banque, monnaie de la boutique) nommés
  partout dans les 7 langues (`lang_v061.py`) et dans le livret. **Écart** : les rubis bleus et rouges ramassés
  en partie comptent toujours dans le compteur vert (valeurs 5 et 20) : à revoir si cela gêne.
- **Reliques** : carte dorée, halo, reflet, quatre étincelles et bandeau « Gardée après la partie ».
- **Fusion du Réceptacle de cœur** : non reproduit (la carte est bien rose avec l'Épée). La carte « Bénédiction de
  fée » (soin) avait une icône de cœur : elle prend la fée, pour ne plus la confondre avec le Réceptacle.
- **Filtres sonores** : dans le navigateur, la musique passait par le lecteur de la page **sans aucun filtre** ;
  elle passe maintenant par un graphe Web Audio (passe-haut, passe-bas, réduction de résolution, mono). Réglages
  beaucoup plus marqués (la musique SNES n'a presque rien au-dessus de 6 kHz) : Doux 4,2 kHz, Rétro 2,6 kHz,
  GBA 7,5 kHz + grain, Game Boy 3 kHz, mono, 5 bits, petit haut-parleur.
- **Classements** : nom en vert pour une partie gagnée, en rouge pour une partie perdue (légende sous le stage).
- Capture `--menu reward` (butin de champion avec une relique).

### Version 0.6.0 : publiée le 25/11/2026
- **Sons plus fidèles** : la fanfare d'objet bien connue (`item_get_4`, reconnue à la hauteur des notes) à
  l'ouverture des coffres, après le bruit du coffre ; sons d'origine pour les tirs d'Agahnim (et sa charge), des
  archers, Lynels, Ganon et Kholdstare, l'évolution, les reliques, les récompenses et le déblocage d'un personnage.
- **Fusions mises en évidence** : dans les choix de niveau, une carte qui peut fusionner avec une arme ou un objet
  déjà possédé est rose (bordure qui pulse, halo, deux étincelles), « Fusionne avec » et l'icône du partenaire qui
  sautille. **Fusions revues** (choix validés) : Boomerang + Palmes de Zora, Grappin + Bottes de Pégase,
  Abeilles + Filet à papillons, Canne de Byrna + Cape magique.
- **5 joueurs** (`players.rs`, `profile.rs`) : un fichier par joueur (`profile_1.ron` à `profile_5.ron`, joueur
  actif dans `profiles.ron`), nom (12 caractères, clavier direct ou grille de lettres à la manette / souris) et
  avatar parmi 20 sprites d'*A Link to the Past* ; créer, changer, renommer, supprimer (double confirmation).
  L'ancien `profile.ron` devient le premier joueur (copie `profile.old.ron`). Le joueur actif et son avatar sont
  dans le menu titre (mise en page du menu adaptée aux menus longs).
- **Link (Four Swords)** : bouton de couleur sous le portrait qui « respire » (fondu entre les 4 couleurs).
- **Barres de vie** sous les mini-boss, champions errants et le Pikit (le boss final garde sa barre en haut).
- **Tirs ennemis** 22 % plus lents (`rules.ron` `shot_speed`), halo rouge pulsé, traînée et contour sombre ; zone
  de contact = 90 % de la taille visible du sprite (`shot_hitbox`, calculée depuis l'atlas).
- **Pikit voleur** (`crates/sim/src/thief.rs`, choix validé) : vers 1:50 puis toutes les 150 à 190 s (2 à 3 fois
  par partie), un Pikit arrive hors de l'écran (annonce, flèche avec distance et temps), fuit en sautant ; vaincu,
  tous les rubis au sol reviennent au joueur ; ignoré, il repart au bout de 45 s.
- **Score** (choix validé) : ennemis x10 + secondes x10 + boss x1000 + victoire 5000, multiplié par la difficulté et
  les Sceaux ; affiché en fin de partie avec « Nouveau record ! » ; 5 meilleures parties par stage et par joueur.
- **Classements** (`scores.rs`, `online.rs`) : mes parties, joueurs de cet ordinateur, monde (top 100 par stage et
  par version, la dernière par défaut). Publication avec un pseudo. Serveur : Cloudflare Worker (`worker/index.js`)
  + base D1 `astp-scores` (`worker/schema.sql`) sur le domaine du site ; **anti-triche par vraisemblance** (choix
  validé, pas de rejeu) : score recalculé, limites de temps / ennemis / niveau / boss / multiplicateur, 12 envois
  par heure et 40 par jour par adresse (empreinte salée, effacée au bout de 30 jours), pseudos filtrés, 3 parties
  gardées par pseudo. Suppression manuelle : `npx wrangler d1 execute astp-scores --remote --command "DELETE ..."`.
  `make_site.py` met à jour `LATEST` et `STAGES` dans `wrangler.jsonc`. **À la 1.0 : les scores des 0.x sont
  masqués par le serveur et à effacer** (`DELETE FROM scores WHERE version LIKE '0.%'`).
- **Manuel** (`research/tools/make_manual.py`, textes `manual_text.py`) sur le modèle du livret d'*A Link to the
  Past* : couverture, légende, 11 chapitres, listes tirées des données du jeu, captures prises par le jeu dans un
  dossier temporaire. **À régénérer à chaque version.** **Écart** : français et anglais seulement (pas 7 langues).
- **Écart** : les caractères `&` et `_` n'existent pas dans la police : la grille de noms propose `+` et `:`.
- Traductions des 7 langues (`lang_v06.py`) ; tests 15/15 (+ essais du serveur `node --test worker/test.mjs`) ;
  vérification de l'interface : 0 problème sur 3192 écrans.

## Idées du 24/11/2026 : propositions (à valider)
| Idée | Proposition | Jalon proposé |
|---|---|---|
| **Classement en ligne** par map, avec pseudo | Petit service Cloudflare (Worker + base D1) sur le même domaine, pseudo filtré, limite d'envois. Rejeu des parties côté serveur écarté au profit de contrôles de vraisemblance (choix du 25/11/2026). | **Fait (0.6.0)** |
| **Export / import des sauvegardes** | Profil dans un seul fichier ; boutons Exporter / Importer (fichier, ou code à copier-coller pour le navigateur et Android). | **Fait (0.3.0)** |
| **Bestiaire, armurerie, statistiques** | Bestiaire (ennemis vus/vaincus, description, sprite animé), armurerie (armes, évolutions découvertes et leur recette), statistiques (parties, temps, ennemis vaincus, records par map). | **Fait (0.3.0)** |
| **Toutes les plateformes** | Windows, Linux, macOS, Android, navigateur : **uniquement quand tu diras que le jeu est prêt**. Points à prévoir : commandes tactiles (Android), musiques en flux (navigateur, 50 Mo), signature macOS. | Windows et **navigateur faits (0.3.0)** ; Linux, macOS, Android sur ton signal |
| Remappage des touches | Écran de réaffectation (les liaisons sont déjà en données). | **Fait (0.3.0)** |

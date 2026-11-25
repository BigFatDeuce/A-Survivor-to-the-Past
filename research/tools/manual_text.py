"""Textes du manuel (livret facon notice d'A Link to the Past), en francais et en anglais.
Importe par make_manual.py. Les noms d'armes, d'objets, de personnages, de stages et d'ennemis ne sont
pas ici : ils viennent des traductions du jeu (data/lang), comme les listes du manuel.
A relire a chaque version : ce qui change dans le jeu doit changer ici (voir la feuille de route).
Marques : **gras**, *italique*."""

T = {
    "fr": dict(
        file="Manuel",
        booklet="LIVRET D'INSTRUCTIONS",
        cover_note="Fan game non officiel, gratuit et non commercial",
        version="Version {v}",
        notice_h="Avant de jouer",
        notice=[
            "Merci d'avoir choisi **A Survivor to the Past**. Ce livret vous explique tout ce qu'il faut savoir pour "
            "survivre aux hordes d'Hyrule. Lisez-le avant de jouer, et gardez-le sous la main : les monstres, eux, "
            "n'attendront pas.",
            "**A Survivor to the Past** est un fan game **non officiel**, gratuit et non commercial, fait par des fans "
            "pour le plaisir. Il n'est ni produit, ni approuvé, ni soutenu par Nintendo. *The Legend of Zelda* et "
            "*A Link to the Past* sont des marques de Nintendo ; les sprites, musiques et sons d'origine leur "
            "appartiennent. Les auteurs des planches de sprites sont crédités dans le fichier CREDITS.md.",
            "Ce livret suit la version **{v}** du jeu. Il est mis à jour à chaque nouvelle version.",
        ],
        needs_h="Ce qu'il vous faut",
        needs=[
            "**Windows 10 ou 11** (64 bits) avec une carte graphique compatible Vulkan, DirectX 12 ou DirectX 11, "
            "ou un **navigateur** récent pour la version web.",
            "Un clavier (et une souris), ou une **manette** (Xbox, PlayStation, Switch Pro et la plupart des manettes USB).",
        ],
        toc_h="Sommaire",
        legend_h="La légende d'Hyrule",
        legend=[
            "Il y a fort longtemps, le royaume d'Hyrule fut sauvé par un jeune héros vêtu de vert. Le sorcier Agahnim "
            "fut vaincu, Ganon scellé dans le Monde des Ténèbres, et la paix revint sur la Triforce.",
            "Mais un sceau n'est jamais éternel. Une nuit d'orage, les portes du Monde des Ténèbres se sont "
            "entrouvertes, et une marée de monstres s'est déversée sur Hyrule : soldats maudits, Moblins, Lynels, "
            "et des créatures que personne n'avait vues depuis des siècles.",
            "Il n'y a plus de donjon à explorer, plus d'énigme à résoudre : seulement des vagues, toujours plus "
            "nombreuses. Pour refermer chaque porte, il faut tenir bon jusqu'à ce que le maître des lieux se montre, "
            "le vaincre, puis franchir la sortie qu'il gardait.",
            "Vous êtes ce survivant. Chaque rubis ramassé vous rend plus fort, chaque coffre peut changer une arme "
            "en légende. Et si vous tombez, ce que vous aurez gagné servira à celui qui reprendra l'épée.",
        ],
        legend_quote="Survivre, encore et encore... jusqu'à ce que l'aube revienne sur Hyrule.",
        # 1. Avant de commencer
        ch_start="1. Avant de commencer",
        install_h="Installer et lancer le jeu",
        install=[
            "**Windows** : extrayez **toute** l'archive dans un dossier (Documents, Bureau...), puis lancez "
            "« A Survivor to the Past.exe ». Le jeu n'étant pas signé, Windows peut afficher « Windows a protégé "
            "votre ordinateur » : cliquez sur « Informations complémentaires », puis « Exécuter quand même ».",
            "**Navigateur** : ouvrez le site du jeu et choisissez « Jouer dans le navigateur ». Vos réglages et vos "
            "joueurs sont gardés dans ce navigateur.",
            "**Mises à jour** : au lancement, le jeu regarde s'il existe une nouvelle version. En mode "
            "« Automatiques », il la télécharge et l'installe au lancement suivant. Vos joueurs et vos réglages sont "
            "toujours gardés.",
        ],
        title_h="L'écran titre",
        title_txt="Sous l'orage, au bord du lac d'Hyrule, l'écran titre propose :",
        menu=[
            ("ui.play", "Choisir un personnage puis un stage, et partir à l'aventure."),
            ("ui.player_item", "Le joueur actif (nom et avatar). Ouvre l'écran des joueurs."),
            ("ui.shop", "Dépenser les rubis en banque : améliorations, bénédictions, reliques, défis."),
            ("ui.profile", "Bestiaire, armurerie, statistiques et sauvegarde du joueur actif."),
            ("ui.scores", "Les meilleurs scores de chaque stage : les vôtres, ceux des autres joueurs, le monde."),
            ("ui.options", "Son, image, filtres rétro, langue, commandes et mises à jour."),
            ("ui.about", "Crédits et liens (site, Discord, code source)."),
        ],
        players_h="L'écran des joueurs",
        players=[
            "Jusqu'à **cinq joueurs** peuvent partager le jeu sur un même ordinateur. Chacun a son nom, son avatar "
            "(vingt personnages et monstres d'A Link to the Past) et toute sa progression : stages gagnés, rubis en "
            "banque, achats, reliques, bestiaire et meilleurs scores.",
            "Choisissez un emplacement libre pour **créer** un joueur, ou un autre joueur pour **jouer avec lui**. "
            "Sur le joueur actif, validez pour le **renommer** et utilisez Gauche/Droite pour changer d'**avatar**.",
            "**Supprimer** efface le joueur actif et toute sa progression, après une seconde confirmation. "
            "Attention : c'est définitif.",
        ],
        name_h="Saisir un nom",
        name=[
            "Au clavier, tapez simplement le nom (12 caractères au plus), Retour arrière pour effacer, Entrée pour "
            "valider, Échap pour annuler. À la manette ou à la souris, choisissez les lettres dans la grille, "
            "puis « Valider ».",
        ],
        # 2. Commandes
        ch_controls="2. Les commandes",
        controls_intro="Le jeu se joue au clavier, au clavier et à la souris, ou à la manette. Toutes les touches de jeu "
                       "peuvent être changées dans Options > Commandes.",
        controls_cols=("Action", "Clavier", "Souris", "Manette"),
        controls=[
            ("Se déplacer", "Z Q S D / flèches", "", "Stick gauche / croix"),
            ("Viser et attaquer", "J", "Clic gauche (vise le curseur)", "X / gâchette droite"),
            ("Dash (esquive rapide)", "Espace / K", "Clic droit", "A / gâchette gauche"),
            ("Attaque automatique ou manuelle", "Tab / R", "", "Y"),
            ("Pause", "Échap / P", "", "Start"),
            ("Menus : valider", "Entrée / Espace / J", "Clic gauche", "A"),
            ("Menus : retour", "Échap / Retour arrière", "Clic droit", "B"),
        ],
        controls_note="Les lettres sont celles d'un clavier AZERTY (WASD sur un clavier QWERTY). Par défaut, vos armes "
                      "attaquent seules la cible la plus proche ; en mode manuel, elles visent là où vous visez.",
        # 3. Ecran de jeu
        ch_screen="3. L'écran de jeu",
        screen_intro="Tout ce qu'il faut surveiller pendant une partie :",
        hud=[
            ("Cœurs", "Votre vie. À zéro, la partie est perdue (sauf si une fée vous ranime)."),
            ("Barre d'expérience", "Se remplit avec les rubis ramassés. Pleine : niveau supérieur et un choix d'amélioration."),
            ("Temps", "Le temps de la partie. Le boss du milieu arrive vers 4:30, le boss final à 9:00."),
            ("Rubis", "Les rubis ramassés, et ceux qui iront en banque à la fin de la partie."),
            ("Armes et objets", "Vos 6 emplacements d'armes et vos 6 emplacements d'objets, avec leur niveau."),
            ("Barres de vie", "En haut de l'écran pour les boss ; sous les mini-boss, les champions et le Pikit voleur."),
            ("Flèches au bord", "Elles montrent les coffres, les champions errants et le Pikit, avec leur distance."),
        ],
        # 4. Deroulement
        ch_run="4. Déroulement d'une partie",
        run=[
            "Une partie dure une dizaine de minutes. Les vagues de monstres grossissent sans cesse ; vos armes "
            "frappent toutes seules, à vous de bouger, d'esquiver et de bien choisir vos améliorations.",
            "Vers **4:30**, un boss du milieu se montre. À **9:00**, le **boss final** s'éveille : vainquez-le et la "
            "**porte de sortie** s'ouvre. Franchissez-la pour gagner le stage.",
        ],
        levelup_h="Monter de niveau",
        levelup=[
            "Chaque niveau propose **trois choix** : une nouvelle arme, un nouvel objet, ou l'amélioration de ce que "
            "vous avez déjà (jusqu'au niveau 8 pour les armes, 5 pour les objets).",
            "Les choix **en rose** peuvent **fusionner** avec une arme ou un objet que vous possédez déjà : c'est le "
            "chemin vers une évolution. L'objet partenaire que vous avez déjà sautille sur la carte.",
        ],
        chest_h="Coffres et évolutions",
        chest=[
            "Les ennemis **élites** (contour doré) laissent un coffre. Il contient des améliorations, ou une "
            "**évolution** : une arme au niveau maximum, accompagnée de son objet partenaire, se transforme en sa "
            "forme légendaire. Les boss laissent un grand coffre.",
        ],
        drops_h="Ce que l'on trouve au sol",
        drops=[
            ("rupee", "**Rubis** vert, bleu ou rouge : expérience, et rubis pour la banque."),
            ("heart", "**Cœur** : rend un peu de vie."),
            ("fairy", "**Fée** : rend toute la vie."),
            ("magnet", "**Filet à papillons** : attire tous les rubis de l'écran."),
            ("medallion", "**Médaillon d'Éther** : gèle les ennemis à l'écran."),
            ("bomb", "**Bob-omb** (rare) : souffle tous les ennemis à l'écran."),
        ],
        drops_note="Les buissons, pots et autres décors se cassent et cachent parfois rubis, cœurs, fées ou médaillons.",
        shots_h="Les tirs ennemis",
        shots="Les archers, Lynels, sorciers et boss tirent. Leurs projectiles sont entourés d'un halo rouge, et leur "
              "zone de contact est un peu plus petite que leur dessin : un frôlement ne compte pas.",
        encounters_h="Rencontres",
        champion="**Champions errants** : un ennemi plus fort que les autres attend quelque part ; une flèche montre "
                 "le chemin. Vaincu, il vous offre une récompense au choix : bénédiction, amélioration spéciale "
                 "d'une arme, ou relique gardée si vous gagnez le stage.",
        thief="**Pikit voleur** : deux à trois fois par partie, un Pikit surgit et dévore les rubis restés au sol. "
              "Il fuit en sautant quand vous approchez et repart au bout de 45 secondes. Rattrapez-le et vainquez-le : "
              "sa langue vous rapporte **tous les rubis** encore au sol !",
        # 5. Equipement
        ch_gear="5. Armes et objets",
        gear_intro="Vous partez avec l'arme de votre personnage. Les autres arrivent en montant de niveau. Six armes et six "
                   "objets au plus : chaque choix compte.",
        weapons_h="Les armes",
        passives_h="Les objets",
        evos_h="Évolutions",
        evos_intro="Une arme au **niveau 8** et son objet partenaire (à n'importe quel niveau) : le prochain coffre la "
                   "fait évoluer.",
        evos_cols=("Arme", "Objet", "Évolution"),
        # 6. Personnages
        ch_chars="6. Les personnages",
        chars_intro="Chaque personnage a ses forces, ses faiblesses et son arme de départ. Les autres se débloquent en jouant.",
        unlocked="Disponible dès le début.",
        # 7. Stages
        ch_stages="7. Les stages",
        stages_intro="Les stages s'ouvrent l'un après l'autre : gagnez-en un pour ouvrir le suivant. Chacun a son climat, "
                     "sa musique, son boss du milieu et son boss final.",
        mid_boss="Boss du milieu",
        final_boss="Boss final",
        relic="Relique",
        # 8. Entre les parties
        ch_meta="8. Entre les parties",
        meta_intro="À la fin d'une partie, une partie des rubis ramassés va **en banque** (plus si vous gagnez). Ils se "
                   "dépensent depuis l'écran titre, dans la Boutique.",
        shop_h="Boutique de Kakariko",
        shop_intro="Des améliorations permanentes, achetées niveau par niveau.",
        fairy_h="La Grande Fée",
        fairy_intro="Des bénédictions achetées une fois pour toutes.",
        relics_h="Reliques",
        relics_intro="La première victoire de chaque stage donne sa relique ; les champions en proposent d'autres. "
                     "Trois reliques peuvent être équipées à la fois.",
        seals_h="Défis : les Sceaux de Ganon",
        seals_intro="Des défis optionnels : chaque sceau rend les parties plus dures, mais rapporte plus de rubis et de points.",
        diff_h="Difficultés",
        diff_intro="Quatre difficultés, choisies dans la Boutique. Plus c'est dur, plus la banque et le score grimpent.",
        profile_h="Profil et sauvegarde",
        profile="Le Profil montre le bestiaire, l'armurerie et les statistiques du joueur actif. L'onglet Sauvegarde "
                "exporte toute la progression sous forme de code texte, à recoller sur un autre ordinateur ou dans "
                "le navigateur.",
        # 9. Score et classements
        ch_scores="9. Score et classements",
        score_h="Le score",
        score=[
            "Chaque partie, gagnée ou perdue, reçoit un score :",
            "**10 points** par ennemi vaincu, **10 points** par seconde de survie, **1 000 points** par boss et "
            "**5 000 points** pour une victoire, le tout multiplié par la **difficulté** et les **Sceaux de Ganon** actifs.",
            "Les cinq meilleures parties de chaque stage sont gardées pour chaque joueur.",
        ],
        boards_h="L'écran Classements",
        boards=[
            ("ui.scores_mine", "Vos cinq meilleures parties sur le stage choisi. Validez une partie pour la publier."),
            ("ui.scores_players", "Le meilleur score de chaque joueur de cet ordinateur : qui est le meilleur de la maison ?"),
            ("ui.scores_world", "Le top 100 mondial du stage, pour la dernière version du jeu (Gauche/Droite sur « Version » "
                                "pour voir les anciennes)."),
        ],
        publish_h="Publier un score",
        publish=[
            "Choisissez un **pseudo** (12 caractères au plus), puis validez : le score part dans le classement mondial. "
            "Le serveur vérifie que la partie est vraisemblable ; les pseudos injurieux sont refusés.",
            "Seuls le pseudo et la partie sont envoyés (stage, version, score, temps, ennemis, niveau, personnage, "
            "difficulté). Aucune donnée personnelle n'est demandée. Le classement n'est contacté que quand vous "
            "ouvrez l'onglet Monde ou publiez un score.",
            "Les classements des versions 0.x seront remis à zéro à la sortie de la version 1.0.",
        ],
        # 10. Bestiaire
        ch_bestiary="10. Bestiaire d'Hyrule",
        bestiary_intro="Les créatures que vous croiserez. Méfiez-vous des chiffres : la difficulté et les Sceaux les rendent plus "
                       "robustes et plus dangereux.",
        stat_hp="PV",
        stat_dmg="Dégâts",
        stat_speed="Vitesse",
        boss_tag="Boss",
        # 11. Conseils
        ch_tips="11. Conseils de survivant",
        tips=[
            "Ne restez jamais immobile : tournez autour de la horde plutôt que de la traverser.",
            "Le dash rend invulnérable un court instant : gardez-le pour sortir d'un encerclement.",
            "Cherchez les fusions en rose : une arme évoluée change une partie.",
            "Les élites lâchent des coffres : ne les laissez pas filer.",
            "Quand le Pikit voleur apparaît, suivez la flèche : tous vos rubis au sol sont en jeu.",
            "Un mauvais départ ? Les rubis mis en banque ne sont jamais perdus : chaque partie vous rend plus fort.",
            "Les Sceaux de Ganon multiplient le score : idéal pour grimper au classement mondial.",
        ],
        back_h="A Survivor to the Past",
        back=[
            "Fan game non officiel, gratuit et non commercial.",
            "Site : {site}",
            "Discord : {discord}",
            "Code source : {repo}",
        ],
        back_credits="Sprites, musiques et sons : *The Legend of Zelda: A Link to the Past* (Nintendo, 1991). "
                     "Planches de sprites : voir CREDITS.md. Moteur et jeu : projet libre A Survivor to the Past.",
        page="Page",
    ),
    "en": dict(
        file="Manual",
        booklet="INSTRUCTION BOOKLET",
        cover_note="Unofficial, free and non-commercial fan game",
        version="Version {v}",
        notice_h="Before you play",
        notice=[
            "Thank you for choosing **A Survivor to the Past**. This booklet explains everything you need to know to "
            "survive the hordes of Hyrule. Read it before playing and keep it close by: the monsters won't wait.",
            "**A Survivor to the Past** is an **unofficial**, free and non-commercial fan game, made by fans for fun. "
            "It is not produced, approved or endorsed by Nintendo. *The Legend of Zelda* and *A Link to the Past* "
            "are trademarks of Nintendo; the original sprites, music and sounds belong to them. The authors of the "
            "sprite sheets are credited in the CREDITS.md file.",
            "This booklet matches version **{v}** of the game. It is updated with every new version.",
        ],
        needs_h="What you need",
        needs=[
            "**Windows 10 or 11** (64-bit) with a graphics card supporting Vulkan, DirectX 12 or DirectX 11, "
            "or a recent **web browser** for the browser version.",
            "A keyboard (and mouse), or a **gamepad** (Xbox, PlayStation, Switch Pro and most USB controllers).",
        ],
        toc_h="Contents",
        legend_h="The Legend of Hyrule",
        legend=[
            "Long ago, the kingdom of Hyrule was saved by a young hero dressed in green. The wizard Agahnim was "
            "defeated, Ganon was sealed away in the Dark World, and peace returned to the Triforce.",
            "But no seal lasts forever. One stormy night, the gates of the Dark World cracked open, and a tide of "
            "monsters poured into Hyrule: cursed soldiers, Moblins, Lynels, and creatures no one had seen for "
            "centuries.",
            "There are no dungeons left to explore, no puzzles left to solve: only waves, ever larger. To close each "
            "gate, you must hold out until its master shows itself, defeat it, then walk through the exit it guarded.",
            "You are that survivor. Every rupee you pick up makes you stronger, every chest may turn a weapon into "
            "a legend. And if you fall, what you earned will help whoever takes up the sword next.",
        ],
        legend_quote="Survive, again and again... until dawn returns to Hyrule.",
        ch_start="1. Before You Start",
        install_h="Installing and launching",
        install=[
            "**Windows**: extract the **whole** archive into a folder (Documents, Desktop...), then launch "
            "“A Survivor to the Past.exe”. The game is not signed, so Windows may say “Windows protected "
            "your PC”: click “More info”, then “Run anyway”.",
            "**Browser**: open the game's website and choose “Play in your browser”. Your settings and players "
            "are kept in that browser.",
            "**Updates**: at launch the game checks for a new version. In “Automatic” mode it downloads it and "
            "installs it at the next launch. Your players and settings are always kept.",
        ],
        title_h="The title screen",
        title_txt="Under the storm, by the lake of Hyrule, the title screen offers:",
        menu=[
            ("ui.play", "Choose a character, then a stage, and set off."),
            ("ui.player_item", "The active player (name and avatar). Opens the players screen."),
            ("ui.shop", "Spend your banked rupees: upgrades, blessings, relics, challenges."),
            ("ui.profile", "Bestiary, armory, statistics and save of the active player."),
            ("ui.scores", "The best scores of every stage: yours, the other players', the world's."),
            ("ui.options", "Sound, video, retro filters, language, controls and updates."),
            ("ui.about", "Credits and links (website, Discord, source code)."),
        ],
        players_h="The players screen",
        players=[
            "Up to **five players** can share the game on the same computer. Each one has a name, an avatar "
            "(twenty characters and monsters from A Link to the Past) and their own progress: stages won, banked "
            "rupees, purchases, relics, bestiary and best scores.",
            "Pick an empty slot to **create** a player, or another player to **play as them**. On the active player, "
            "confirm to **rename** and use Left/Right to change the **avatar**.",
            "**Delete** erases the active player and all their progress, after a second confirmation. Careful: "
            "there is no way back.",
        ],
        name_h="Entering a name",
        name=[
            "With a keyboard, just type the name (12 characters at most), Backspace to erase, Enter to confirm, "
            "Esc to cancel. With a gamepad or mouse, pick letters in the grid, then “Confirm”.",
        ],
        ch_controls="2. Controls",
        controls_intro="Play with a keyboard, keyboard and mouse, or a gamepad. Every gameplay key can be changed in "
                       "Options > Controls.",
        controls_cols=("Action", "Keyboard", "Mouse", "Gamepad"),
        controls=[
            ("Move", "W A S D / arrows", "", "Left stick / D-pad"),
            ("Aim and attack", "J", "Left click (aims at the cursor)", "X / right trigger"),
            ("Dash (quick dodge)", "Space / K", "Right click", "A / left trigger"),
            ("Automatic or manual attacks", "Tab / R", "", "Y"),
            ("Pause", "Esc / P", "", "Start"),
            ("Menus: confirm", "Enter / Space / J", "Left click", "A"),
            ("Menus: back", "Esc / Backspace", "Right click", "B"),
        ],
        controls_note="Letters are those of a QWERTY keyboard (ZQSD on an AZERTY keyboard). By default your weapons attack "
                      "the nearest enemy on their own; in manual mode they aim where you aim.",
        ch_screen="3. The Game Screen",
        screen_intro="Everything to keep an eye on during a run:",
        hud=[
            ("Hearts", "Your life. At zero the run is lost (unless a fairy revives you)."),
            ("Experience bar", "Fills up with the rupees you collect. When full: level up and a choice of upgrade."),
            ("Time", "The run's clock. The mid-boss arrives around 4:30, the final boss at 9:00."),
            ("Rupees", "Rupees collected, and those that will go to the bank at the end of the run."),
            ("Weapons and items", "Your 6 weapon slots and 6 item slots, with their levels."),
            ("Health bars", "At the top of the screen for bosses; under mini-bosses, champions and the thief Pikit."),
            ("Edge arrows", "They point to chests, wandering champions and the Pikit, with their distance."),
        ],
        ch_run="4. How a Run Goes",
        run=[
            "A run lasts about ten minutes. The waves of monsters keep growing; your weapons strike on their own, "
            "it's up to you to move, dodge and choose your upgrades well.",
            "Around **4:30** a mid-boss shows up. At **9:00** the **final boss** awakens: defeat it and the **exit door** "
            "opens. Walk through it to win the stage.",
        ],
        levelup_h="Leveling up",
        levelup=[
            "Each level offers **three choices**: a new weapon, a new item, or an upgrade of what you already have "
            "(up to level 8 for weapons, 5 for items).",
            "Choices shown **in pink** can **fuse** with a weapon or item you already own: that's the road to an "
            "evolution. The partner you already have hops on the card.",
        ],
        chest_h="Chests and evolutions",
        chest=[
            "**Elite** enemies (golden outline) drop a chest. It holds upgrades, or an **evolution**: a weapon at its "
            "maximum level, together with its partner item, turns into its legendary form. Bosses drop a big chest.",
        ],
        drops_h="What lies on the ground",
        drops=[
            ("rupee", "**Rupee** (green, blue or red): experience, and rupees for the bank."),
            ("heart", "**Heart**: restores a little life."),
            ("fairy", "**Fairy**: restores all your life."),
            ("magnet", "**Bug-Catching Net**: pulls every rupee on screen."),
            ("medallion", "**Ether Medallion**: freezes the enemies on screen."),
            ("bomb", "**Bob-omb** (rare): blows away every enemy on screen."),
        ],
        drops_note="Bushes, pots and other scenery break, and sometimes hide rupees, hearts, fairies or medallions.",
        shots_h="Enemy shots",
        shots="Archers, Lynels, wizards and bosses shoot. Their projectiles glow with a red halo, and their hitbox is a "
              "little smaller than their drawing: a graze doesn't count.",
        encounters_h="Encounters",
        champion="**Wandering champions**: a stronger enemy waits somewhere; an arrow shows the way. Beat it for a reward "
                 "of your choice: a blessing, a special weapon upgrade, or a relic you keep if you win the stage.",
        thief="**Thief Pikit**: two or three times per run, a Pikit shows up and gobbles the rupees left on the ground. "
              "It hops away when you get close and leaves after 45 seconds. Catch it and defeat it: its tongue brings "
              "back **every rupee** still on the ground!",
        ch_gear="5. Weapons and Items",
        gear_intro="You start with your character's weapon. The others come as you level up. Six weapons and six items at "
                   "most: every choice matters.",
        weapons_h="Weapons",
        passives_h="Items",
        evos_h="Evolutions",
        evos_intro="A weapon at **level 8** plus its partner item (any level): the next chest evolves it.",
        evos_cols=("Weapon", "Item", "Evolution"),
        ch_chars="6. Characters",
        chars_intro="Each character has strengths, weaknesses and a starting weapon. The others unlock as you play.",
        unlocked="Available from the start.",
        ch_stages="7. Stages",
        stages_intro="Stages open one after the other: win one to open the next. Each has its own weather, music, mid-boss "
                     "and final boss.",
        mid_boss="Mid-boss",
        final_boss="Final boss",
        relic="Relic",
        ch_meta="8. Between Runs",
        meta_intro="At the end of a run, part of the rupees you collected goes **to the bank** (more if you win). Spend them "
                   "from the title screen, in the Shop.",
        shop_h="Kakariko Shop",
        shop_intro="Permanent upgrades, bought level by level.",
        fairy_h="The Great Fairy",
        fairy_intro="Blessings bought once and for all.",
        relics_h="Relics",
        relics_intro="The first win on each stage grants its relic; champions offer others. Three relics can be equipped at "
                     "once.",
        seals_h="Challenges: Ganon's Seals",
        seals_intro="Optional challenges: each seal makes runs harder, but earns more rupees and points.",
        diff_h="Difficulties",
        diff_intro="Four difficulties, chosen in the Shop. The harder, the higher the bank and the score.",
        profile_h="Profile and save",
        profile="The Profile shows the active player's bestiary, armory and statistics. The Save tab exports all progress "
                "as a text code, to paste on another computer or in the browser.",
        ch_scores="9. Score and Leaderboards",
        score_h="The score",
        score=[
            "Every run, won or lost, gets a score:",
            "**10 points** per enemy defeated, **10 points** per second survived, **1,000 points** per boss and "
            "**5,000 points** for a victory, all multiplied by the **difficulty** and the active **Ganon's Seals**.",
            "Each player keeps their five best runs on every stage.",
        ],
        boards_h="The Leaderboards screen",
        boards=[
            ("ui.scores_mine", "Your five best runs on the chosen stage. Confirm a run to publish it."),
            ("ui.scores_players", "The best score of each player on this computer: who's the best in the house?"),
            ("ui.scores_world", "The world top 100 of the stage, for the latest version of the game (Left/Right on "
                                "“Version” to see older ones)."),
        ],
        publish_h="Publishing a score",
        publish=[
            "Choose a **name** (12 characters at most), then confirm: the score goes to the world leaderboard. The "
            "server checks that the run is plausible; offensive names are refused.",
            "Only the name and the run are sent (stage, version, score, time, enemies, level, character, difficulty). "
            "No personal data is asked for. The leaderboard is only contacted when you open the World tab or publish "
            "a score.",
            "The leaderboards of versions 0.x will be reset when version 1.0 comes out.",
        ],
        ch_bestiary="10. Bestiary of Hyrule",
        bestiary_intro="The creatures you will meet. Don't trust the numbers too much: difficulty and Seals make them tougher "
                       "and more dangerous.",
        stat_hp="HP",
        stat_dmg="Damage",
        stat_speed="Speed",
        boss_tag="Boss",
        ch_tips="11. Survivor's Tips",
        tips=[
            "Never stand still: circle around the horde rather than cutting through it.",
            "The dash makes you invulnerable for a moment: keep it to escape when surrounded.",
            "Look for pink fusions: an evolved weapon changes a run.",
            "Elites drop chests: don't let them slip away.",
            "When the thief Pikit appears, follow the arrow: all your rupees on the ground are at stake.",
            "Bad start? Banked rupees are never lost: every run makes you stronger.",
            "Ganon's Seals multiply the score: ideal for climbing the world leaderboard.",
        ],
        back_h="A Survivor to the Past",
        back=[
            "Unofficial, free and non-commercial fan game.",
            "Website: {site}",
            "Discord: {discord}",
            "Source code: {repo}",
        ],
        back_credits="Sprites, music and sounds: *The Legend of Zelda: A Link to the Past* (Nintendo, 1991). "
                     "Sprite sheets: see CREDITS.md. Engine and game: the open-source A Survivor to the Past project.",
        page="Page",
    ),
}

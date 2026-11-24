"""Textes de la version 0.4 : personnages, Bob-omb, decor cassable et objets rares, nouvelles armes et
objets, statistiques Chance et Croissance. Importe par gen_lang.py.
Japonais en kana uniquement (police du jeu) ; pas de guillemets doubles ni de chevrons."""

UI = ["ui.characters", "ui.char_health", "ui.char_speed", "ui.char_weapon", "ui.unlock_win", "ui.unlock_kills",
      "ui.unlock_bosses", "ui.unlock_level", "ui.new_character", "ui.bomb_spawned", "ui.screen_clear",
      "ui.pickup_net", "ui.pickup_fairy", "ui.pickup_medallion", "stat.luck", "stat.growth",
      "shop.luck.name", "shop.luck.desc", "shop.growth.name", "shop.growth.desc", "enemy.bobomb"]
CHARS = ["link", "link_minish", "link_capless"]
WEAPONS = ["hookshot", "longshot", "magic_powder", "witch_powder", "cane_of_somaria", "great_somaria", "bees",
           "golden_bees"]
PASSIVES = ["book_of_mudora", "mushroom", "magic_cape", "flippers"]

KEYS = (UI + [f"char.{c}.{f}" for c in CHARS for f in ("name", "desc")]
        + [f"weapon.{w}.{f}" for w in WEAPONS for f in ("name", "desc")]
        + [f"passive.{p}.{f}" for p in PASSIVES for f in ("name", "desc")])


def table(ui, chars, weapons, passives):
    assert len(ui) == len(UI) and len(chars) == len(CHARS) and len(weapons) == len(WEAPONS) and len(passives) == len(PASSIVES)
    t = dict(zip(UI, ui))
    for ids, prefix, rows in (("c", "char", chars), ("w", "weapon", weapons), ("p", "passive", passives)):
        names = {"c": CHARS, "w": WEAPONS, "p": PASSIVES}[ids]
        for i, (n, d) in zip(names, rows):
            t[f"{prefix}.{i}.name"] = n
            t[f"{prefix}.{i}.desc"] = d
    return t


EXTRA = {
    "en": table(
        ["Characters", "Hearts", "Speed", "Starting weapon", "Win {s} to unlock.",
         "Defeat {n} enemies in total to unlock.", "Defeat {n} bosses in total to unlock.",
         "Reach level {n} in a run to unlock.", "New character: {s}", "A Bob-omb is coming!",
         "Bob-omb! {n} enemies blown away", "Bug net: every rupee comes to you!", "A fairy heals you completely!",
         "Ether medallion: enemies frozen!", "Luck", "Growth",
         "Lucky mushroom", "More critical hits and more items in bushes.",
         "Sahasrahla's lessons", "Gain experience faster.", "Bob-omb"],
        [("Link", "The hero of Hyrule. Balanced, sword in hand."),
         ("Minish Link", "Light and lucky: more critical hits and items. Starts with the boomerang."),
         ("Capless Link", "Sturdy and strong: more damage, area and armor, but slower. Starts with bombs.")],
        [("Hookshot", "Shoots a chained hook that pierces and stuns."),
         ("Longshot", "Three long chains that stun for a long time."),
         ("Magic Powder", "A sparkling cloud around you hurts nearby enemies."),
         ("Witch's Powder", "A huge cloud that burns everything around you."),
         ("Cane of Somaria", "Drops a block on an enemy: it bursts into 4 beams."),
         ("Great Somaria", "Three blocks that burst into 8 beams."),
         ("Bees", "Bees chase and sting the nearest enemies."),
         ("Golden Bees", "A swarm of golden bees that never stops stinging.")],
        [("Book of Mudora", "You learn faster: more experience."),
         ("Mushroom", "More luck: critical hits and items from bushes."),
         ("Magic Cape", "Your effects last longer."),
         ("Zora's Flippers", "Faster projectiles, and you move a little faster.")],
    ),
    "fr": table(
        ["Personnages", "Coeurs", "Vitesse", "Arme de départ", "Gagnez {s} pour le débloquer.",
         "Vainquez {n} ennemis au total pour le débloquer.", "Vainquez {n} boss au total pour le débloquer.",
         "Atteignez le niveau {n} en partie pour le débloquer.", "Nouveau personnage : {s}", "Une Bob-omb approche !",
         "Bob-omb ! {n} ennemis soufflés", "Filet : tous les rubis viennent à vous !", "Une fée vous soigne entièrement !",
         "Médaillon d'Éther : ennemis gelés !", "Chance", "Croissance",
         "Champignon porte-bonheur", "Plus de coups critiques et d'objets dans les buissons.",
         "Leçons de Sahasrahla", "Gagnez de l'expérience plus vite.", "Bob-omb"],
        [("Link", "Le héros d'Hyrule. Équilibré, l'épée à la main."),
         ("Link (Minish)", "Léger et chanceux : plus de critiques et d'objets. Part avec le boomerang."),
         ("Link sans bonnet", "Robuste et puissant : plus de dégâts, de zone et d'armure, mais plus lent. Part avec les bombes.")],
        [("Grappin", "Lance un crochet enchaîné qui transperce et étourdit."),
         ("Super grappin", "Trois longues chaînes qui étourdissent longtemps."),
         ("Poudre magique", "Un nuage scintillant autour de toi blesse les ennemis proches."),
         ("Poudre de la sorcière", "Un immense nuage qui brûle tout autour de toi."),
         ("Canne de Somaria", "Pose un bloc sur un ennemi : il éclate en 4 rayons."),
         ("Grande Somaria", "Trois blocs qui éclatent en 8 rayons."),
         ("Abeilles", "Des abeilles poursuivent et piquent les ennemis proches."),
         ("Abeilles dorées", "Un essaim d'abeilles dorées qui pique sans relâche.")],
        [("Livre de Mudora", "Tu apprends plus vite : plus d'expérience."),
         ("Champignon", "Plus de chance : critiques et objets dans les buissons."),
         ("Cape magique", "Tes effets durent plus longtemps."),
         ("Palmes de Zora", "Projectiles plus rapides, et tu te déplaces un peu plus vite.")],
    ),
    "de": table(
        ["Figuren", "Herzen", "Tempo", "Startwaffe", "Gewinne {s} zum Freischalten.",
         "Besiege insgesamt {n} Gegner zum Freischalten.", "Besiege insgesamt {n} Bosse zum Freischalten.",
         "Erreiche Stufe {n} in einem Lauf zum Freischalten.", "Neue Figur: {s}", "Ein Bob-omb naht!",
         "Bob-omb! {n} Gegner weggefegt", "Kescher: alle Rubine fliegen zu dir!", "Eine Fee heilt dich vollständig!",
         "Äther-Medaillon: Gegner eingefroren!", "Glück", "Wachstum",
         "Glückspilz", "Mehr kritische Treffer und mehr Funde in Büschen.",
         "Sahasrahlas Lehren", "Sammle schneller Erfahrung.", "Bob-omb"],
        [("Link", "Der Held Hyrules. Ausgewogen, mit Schwert in der Hand."),
         ("Minish-Link", "Leicht und glücklich: mehr Kritische und Funde. Startet mit dem Bumerang."),
         ("Link ohne Mütze", "Robust und stark: mehr Schaden, Fläche und Rüstung, aber langsamer. Startet mit Bomben.")],
        [("Enterhaken", "Schießt einen Kettenhaken, der durchbohrt und betäubt."),
         ("Lang-Enterhaken", "Drei lange Ketten, die lange betäuben."),
         ("Zauberpulver", "Eine funkelnde Wolke um dich verletzt nahe Gegner."),
         ("Hexenpulver", "Eine riesige Wolke, die alles um dich verbrennt."),
         ("Somaria-Stab", "Setzt einen Block auf einen Gegner: er platzt in 4 Strahlen."),
         ("Große Somaria", "Drei Blöcke, die in 8 Strahlen platzen."),
         ("Bienen", "Bienen jagen und stechen die nächsten Gegner."),
         ("Goldbienen", "Ein Schwarm Goldbienen, der unablässig sticht.")],
        [("Buch Mudora", "Du lernst schneller: mehr Erfahrung."),
         ("Pilz", "Mehr Glück: Kritische und Funde in Büschen."),
         ("Zauberumhang", "Deine Effekte halten länger."),
         ("Zora-Flossen", "Schnellere Geschosse, und du bist etwas flinker.")],
    ),
    "es": table(
        ["Personajes", "Corazones", "Velocidad", "Arma inicial", "Gana {s} para desbloquearlo.",
         "Derrota {n} enemigos en total para desbloquearlo.", "Derrota {n} jefes en total para desbloquearlo.",
         "Alcanza el nivel {n} en una partida para desbloquearlo.", "Nuevo personaje: {s}", "¡Se acerca un Bob-omb!",
         "¡Bob-omb! {n} enemigos barridos", "Red: ¡todas las rupias vienen a ti!", "¡Un hada te cura por completo!",
         "Medallón de Éter: ¡enemigos congelados!", "Suerte", "Crecimiento",
         "Champiñón de la suerte", "Más golpes críticos y más objetos en los arbustos.",
         "Lecciones de Sahasrahla", "Gana experiencia más rápido.", "Bob-omb"],
        [("Link", "El héroe de Hyrule. Equilibrado, espada en mano."),
         ("Link Minish", "Ligero y afortunado: más críticos y objetos. Empieza con el bumerán."),
         ("Link sin gorro", "Robusto y fuerte: más daño, área y armadura, pero más lento. Empieza con bombas.")],
        [("Gancho", "Lanza un garfio encadenado que atraviesa y aturde."),
         ("Supergancho", "Tres cadenas largas que aturden mucho tiempo."),
         ("Polvo mágico", "Una nube brillante a tu alrededor hiere a los enemigos cercanos."),
         ("Polvo de la bruja", "Una nube enorme que quema todo a tu alrededor."),
         ("Bastón de Somaria", "Pone un bloque sobre un enemigo: estalla en 4 rayos."),
         ("Gran Somaria", "Tres bloques que estallan en 8 rayos."),
         ("Abejas", "Las abejas persiguen y pican a los enemigos cercanos."),
         ("Abejas doradas", "Un enjambre de abejas doradas que pica sin parar.")],
        [("Libro de Mudora", "Aprendes más rápido: más experiencia."),
         ("Champiñón", "Más suerte: críticos y objetos en los arbustos."),
         ("Capa mágica", "Tus efectos duran más."),
         ("Aletas de Zora", "Proyectiles más rápidos, y te mueves un poco más rápido.")],
    ),
    "it": table(
        ["Personaggi", "Cuori", "Velocità", "Arma iniziale", "Vinci {s} per sbloccarlo.",
         "Sconfiggi {n} nemici in totale per sbloccarlo.", "Sconfiggi {n} boss in totale per sbloccarlo.",
         "Raggiungi il livello {n} in una partita per sbloccarlo.", "Nuovo personaggio: {s}", "Arriva una Bob-omba!",
         "Bob-omba! {n} nemici spazzati via", "Retino: tutte le rupie vengono da te!", "Una fata ti cura del tutto!",
         "Medaglione dell'Etere: nemici congelati!", "Fortuna", "Crescita",
         "Fungo portafortuna", "Più colpi critici e più oggetti nei cespugli.",
         "Lezioni di Sahasrahla", "Guadagna esperienza più in fretta.", "Bob-omba"],
        [("Link", "L'eroe di Hyrule. Equilibrato, spada in pugno."),
         ("Link Minish", "Leggero e fortunato: più critici e oggetti. Parte con il boomerang."),
         ("Link senza berretto", "Robusto e forte: più danni, area e armatura, ma più lento. Parte con le bombe.")],
        [("Rampino", "Lancia un uncino a catena che trafigge e stordisce."),
         ("Super rampino", "Tre lunghe catene che stordiscono a lungo."),
         ("Polvere magica", "Una nube scintillante intorno a te ferisce i nemici vicini."),
         ("Polvere della strega", "Una nube enorme che brucia tutto intorno a te."),
         ("Bastone di Somaria", "Posa un blocco su un nemico: esplode in 4 raggi."),
         ("Grande Somaria", "Tre blocchi che esplodono in 8 raggi."),
         ("Api", "Le api inseguono e pungono i nemici vicini."),
         ("Api dorate", "Uno sciame di api dorate che punge senza sosta.")],
        [("Libro di Mudora", "Impari più in fretta: più esperienza."),
         ("Fungo", "Più fortuna: critici e oggetti nei cespugli."),
         ("Mantello magico", "I tuoi effetti durano di più."),
         ("Pinne di Zora", "Proiettili più veloci, e ti muovi un po' più in fretta.")],
    ),
    "pt": table(
        ["Personagens", "Corações", "Velocidade", "Arma inicial", "Vença {s} para desbloquear.",
         "Derrote {n} inimigos no total para desbloquear.", "Derrote {n} chefes no total para desbloquear.",
         "Alcance o nível {n} numa partida para desbloquear.", "Novo personagem: {s}", "Um Bob-omb se aproxima!",
         "Bob-omb! {n} inimigos varridos", "Rede: todas as rupias vêm até você!", "Uma fada cura você por completo!",
         "Medalhão do Éter: inimigos congelados!", "Sorte", "Crescimento",
         "Cogumelo da sorte", "Mais acertos críticos e mais itens nos arbustos.",
         "Lições de Sahasrahla", "Ganhe experiência mais rápido.", "Bob-omb"],
        [("Link", "O herói de Hyrule. Equilibrado, de espada em punho."),
         ("Link Minish", "Leve e sortudo: mais críticos e itens. Começa com o bumerangue."),
         ("Link sem gorro", "Robusto e forte: mais dano, área e armadura, mas mais lento. Começa com bombas.")],
        [("Gancho", "Lança um gancho com corrente que atravessa e atordoa."),
         ("Supergancho", "Três correntes longas que atordoam por muito tempo."),
         ("Pó mágico", "Uma nuvem cintilante ao seu redor fere os inimigos próximos."),
         ("Pó da bruxa", "Uma nuvem enorme que queima tudo ao seu redor."),
         ("Bastão de Somaria", "Coloca um bloco sobre um inimigo: ele explode em 4 raios."),
         ("Grande Somaria", "Três blocos que explodem em 8 raios."),
         ("Abelhas", "Abelhas perseguem e picam os inimigos próximos."),
         ("Abelhas douradas", "Um enxame de abelhas douradas que pica sem parar.")],
        [("Livro de Mudora", "Você aprende mais rápido: mais experiência."),
         ("Cogumelo", "Mais sorte: críticos e itens nos arbustos."),
         ("Capa mágica", "Seus efeitos duram mais."),
         ("Nadadeiras de Zora", "Projéteis mais rápidos, e você anda um pouco mais rápido.")],
    ),
    "ja": table(
        ["キャラクター", "ハート", "はやさ", "さいしょの ぶき", "「{s}」に かつと つかえる",
         "てきを ぜんぶで {n}たい たおすと つかえる", "ボスを ぜんぶで {n}たい たおすと つかえる",
         "ひとつの ゲームで レベル{n}に なると つかえる", "あたらしい キャラクター: {s}", "ボムへいが ちかづいて くる!",
         "ボムへい! てきを {n}たい ふきとばした", "むしとりあみ: ルピーが ぜんぶ あつまる!", "ようせいが ぜんかいふく して くれた!",
         "エーテルの メダル: てきが こおった!", "ラッキー", "せいちょう",
         "ラッキーキノコ", "クリティカルと しげみの アイテムが ふえる。",
         "サハスラーラの おしえ", "けいけんちが はやく たまる。", "ボムへい"],
        [("リンク", "ハイラルの ゆうしゃ。 けんを もつ バランスがた。"),
         ("ミニッシュの リンク", "かるくて ラッキー。 クリティカルと アイテムが ふえる。 ブーメランで はじまる。"),
         ("ぼうしの ない リンク", "がんじょうで ちからもち。 ダメージ、 はんい、 ぼうぎょが あがるが おそい。 バクダンで はじまる。")],
        [("フックショット", "くさりの かぎを うつ。 つらぬいて しびれさせる。"),
         ("ロングフック", "ながい くさりを 3ほん。 ながく しびれさせる。"),
         ("まほうの こな", "まわりの キラキラの くもが ちかくの てきを きずつける。"),
         ("まじょの こな", "まわりの すべてを もやす おおきな くも。"),
         ("ソマリアの つえ", "てきの うえに ブロックを おく。 4ほうこうに ビームが でる。"),
         ("グレート ソマリア", "ブロック 3こが 8ほうこうに はじける。"),
         ("ハチ", "ハチが ちかくの てきを おいかけて さす。"),
         ("ゴールドの ハチ", "ゴールドの ハチの むれが さしつづける。")],
        [("ムドラの しょ", "はやく まなぶ: けいけんちが ふえる。"),
         ("キノコ", "ラッキーが あがる: クリティカルと しげみの アイテム。"),
         ("まほうの マント", "こうかが ながく つづく。"),
         ("ゾーラの フィン", "うつ ものが はやくなり、 すこし はやく あるける。")],
    ),
}

# Stage 6 (bonus) : Montagne de la Mort, et son ennemi le Lynel.
STAGE = ["map.death_mountain.name", "map.death_mountain.desc", "enemy.lynel"]
KEYS += STAGE
_STAGE = {
    "en": ["Death Mountain", "Mist on the peaks, Lynels on the slopes, and Ganon's Tower at the summit.", "Lynel"],
    "fr": ["Montagne de la Mort", "La brume sur les cimes, des Lynels sur les pentes et la Tour de Ganon au sommet.", "Lynel"],
    "de": ["Todesberg", "Nebel auf den Gipfeln, Leunen an den Hängen und Ganons Turm auf dem Gipfel.", "Leune"],
    "es": ["Montaña de la Muerte", "Niebla en las cumbres, Lynels en las laderas y la Torre de Ganon en la cima.", "Lynel"],
    "it": ["Monte Morte", "Nebbia sulle vette, Lynel sui pendii e la Torre di Ganon in cima.", "Lynel"],
    "pt": ["Montanha da Morte", "Névoa nos picos, Lynels nas encostas e a Torre de Ganon no topo.", "Lynel"],
    "ja": ["デスマウンテン", "きりの やまに ライネルが さまよい、 ちょうじょうには ガノンの とうが ある。", "ライネル"],
}
for _code, _v in _STAGE.items():
    EXTRA[_code].update(zip(STAGE, _v))

"""Textes de la version 0.6.2 : personnages ajoutes (Sheik, Mario, Skull Kid, Simon Belmont, Vivi, Ganondorf),
deblocage de Ganondorf (tous les autres personnages), deux apparences du Link de Link's Awakening, statue de
la canne de Somaria. Importe par gen_lang.py. Japonais en kana uniquement (police du jeu) ; pas de guillemets
doubles ni de chevrons, pas de point-virgule.
KEYS ne liste que les cles nouvelles ; EXTRA remplace aussi des textes existants (Link's Awakening, Somaria)."""

UI = ["ui.unlock_all_others", "skin.dx", "skin.gb"]
CHARS = ["sheik", "mario", "skull_kid", "simon", "vivi", "ganondorf"]
OVERRIDE = ["char.link_awakening.desc", "weapon.cane_of_somaria.desc", "weapon.great_somaria.desc"]

KEYS = UI + [f"char.{c}.{f}" for c in CHARS for f in ("name", "desc")]


def table(ui, chars, override):
    assert len(ui) == len(UI) and len(chars) == len(CHARS) and len(override) == len(OVERRIDE)
    t = dict(zip(UI, ui))
    for i, (n, d) in zip(CHARS, chars):
        t[f"char.{i}.name"] = n
        t[f"char.{i}.desc"] = d
    t.update(zip(OVERRIDE, override))
    return t


EXTRA = {
    "fr": table(
        ["Débloquez tous les autres personnages.", "Game Boy Color", "Game Boy"],
        [("Sheik", "Très rapide et son dash revient vite, mais fragile et frappe moins fort. Part avec l'arc (ses aiguilles)."),
         ("Mario", "Avance bouclier levé : armure et explosions plus larges, mais moins chanceux. Part avec les bombes."),
         ("Skull Kid", "Farceur chanceux, ses effets durent plus longtemps, mais fragile. Part avec la poudre magique."),
         ("Simon Belmont", "Chasseur de vampires : 7 coeurs et plus de dégâts, mais lent. Part avec le boomerang (sa croix)."),
         ("Vivi", "Mage noir : sorts plus larges et plus fréquents, mais seulement 4 coeurs. Part avec la baguette de feu."),
         ("Ganondorf", "Le roi du mal : 10 coeurs, armure et énormes dégâts, mais lent. Part avec l'épée.")],
        ["Chanceux, attire les objets de loin, mais moins de dégâts. Part avec la poudre magique.",
         "Pose une statue sur un ennemi : elle éclate en 4 rayons.",
         "Trois statues qui éclatent en 8 rayons."],
    ),
    "en": table(
        ["Unlock every other character.", "Game Boy Color", "Game Boy"],
        [("Sheik", "Very fast and her dash comes back quickly, but frail and hits softer. Starts with the bow (her needles)."),
         ("Mario", "Walks shield up: armor and wider blasts, but less lucky. Starts with bombs."),
         ("Skull Kid", "Lucky trickster with longer effects, but frail. Starts with magic powder."),
         ("Simon Belmont", "Vampire hunter: 7 hearts and more damage, but slow. Starts with the boomerang (his cross)."),
         ("Vivi", "Black mage: wider and more frequent spells, but only 4 hearts. Starts with the Fire Rod."),
         ("Ganondorf", "The King of Evil: 10 hearts, armor and huge damage, but slow. Starts with the sword.")],
        ["Lucky and pulls items from afar, but deals less damage. Starts with magic powder.",
         "Drops a statue on an enemy: it bursts into 4 beams.",
         "Three statues that burst into 8 beams."],
    ),
    "de": table(
        ["Schalte alle anderen Figuren frei.", "Game Boy Color", "Game Boy"],
        [("Shiek", "Sehr schnell, ihr Sprint lädt schnell, aber zerbrechlich und schwächer. Startet mit dem Bogen (ihre Nadeln)."),
         ("Mario", "Geht mit erhobenem Schild: Rüstung und größere Explosionen, aber weniger Glück. Startet mit Bomben."),
         ("Horror Kid", "Glücklicher Schelm mit längeren Effekten, aber zerbrechlich. Startet mit Zauberpulver."),
         ("Simon Belmont", "Vampirjäger: 7 Herzen und mehr Schaden, aber langsam. Startet mit dem Bumerang (sein Kreuz)."),
         ("Vivi", "Schwarzmagier: größere und häufigere Zauber, aber nur 4 Herzen. Startet mit dem Feuerstab."),
         ("Ganondorf", "Der König des Bösen: 10 Herzen, Rüstung und riesiger Schaden, aber langsam. Startet mit dem Schwert.")],
        ["Glücklich und zieht Gegenstände von weit her an, aber weniger Schaden. Startet mit Zauberpulver.",
         "Setzt eine Statue auf einen Gegner: sie platzt in 4 Strahlen.",
         "Drei Statuen, die in 8 Strahlen platzen."],
    ),
    "es": table(
        ["Desbloquea a todos los demás personajes.", "Game Boy Color", "Game Boy"],
        [("Sheik", "Muy rápida y su carrera vuelve pronto, pero frágil y golpea menos. Empieza con el arco (sus agujas)."),
         ("Mario", "Avanza con el escudo en alto: armadura y explosiones más amplias, pero menos suerte. Empieza con las bombas."),
         ("Skull Kid", "Bromista afortunado con efectos más largos, pero frágil. Empieza con los polvos mágicos."),
         ("Simon Belmont", "Cazador de vampiros: 7 corazones y más daño, pero lento. Empieza con el bumerán (su cruz)."),
         ("Vivi", "Mago negro: hechizos más amplios y frecuentes, pero solo 4 corazones. Empieza con la vara de fuego."),
         ("Ganondorf", "El rey del mal: 10 corazones, armadura y daño enorme, pero lento. Empieza con la espada.")],
        ["Afortunado y atrae objetos de lejos, pero hace menos daño. Empieza con los polvos mágicos.",
         "Pone una estatua sobre un enemigo: estalla en 4 rayos.",
         "Tres estatuas que estallan en 8 rayos."],
    ),
    "it": table(
        ["Sblocca tutti gli altri personaggi.", "Game Boy Color", "Game Boy"],
        [("Sheik", "Velocissima e il suo scatto torna presto, ma fragile e colpisce meno. Parte con l'arco (i suoi aghi)."),
         ("Mario", "Avanza con lo scudo alzato: armatura ed esplosioni più ampie, ma meno fortuna. Parte con le bombe."),
         ("Skull Kid", "Burlone fortunato con effetti più lunghi, ma fragile. Parte con la polvere magica."),
         ("Simon Belmont", "Cacciatore di vampiri: 7 cuori e più danni, ma lento. Parte con il boomerang (la sua croce)."),
         ("Vivi", "Mago nero: incantesimi più ampi e frequenti, ma solo 4 cuori. Parte con la verga di fuoco."),
         ("Ganondorf", "Il re del male: 10 cuori, armatura e danni enormi, ma lento. Parte con la spada.")],
        ["Fortunato e attira gli oggetti da lontano, ma fa meno danni. Parte con la polvere magica.",
         "Posa una statua su un nemico: esplode in 4 raggi.",
         "Tre statue che esplodono in 8 raggi."],
    ),
    "pt": table(
        ["Desbloqueie todos os outros personagens.", "Game Boy Color", "Game Boy"],
        [("Sheik", "Muito rápida e sua corrida volta logo, mas frágil e bate mais fraco. Começa com o arco (suas agulhas)."),
         ("Mario", "Anda de escudo erguido: armadura e explosões maiores, mas menos sorte. Começa com as bombas."),
         ("Skull Kid", "Brincalhão sortudo com efeitos mais longos, mas frágil. Começa com o pó mágico."),
         ("Simon Belmont", "Caçador de vampiros: 7 corações e mais dano, mas lento. Começa com o bumerangue (sua cruz)."),
         ("Vivi", "Mago negro: feitiços maiores e mais frequentes, mas só 4 corações. Começa com o bastão de fogo."),
         ("Ganondorf", "O rei do mal: 10 corações, armadura e dano enorme, mas lento. Começa com a espada.")],
        ["Sortudo e atrai itens de longe, mas causa menos dano. Começa com o pó mágico.",
         "Põe uma estátua sobre um inimigo: ela estoura em 4 raios.",
         "Três estátuas que estouram em 8 raios."],
    ),
    "ja": table(
        ["ほかの キャラを すべて かいほうする。", "ゲームボーイカラー", "ゲームボーイ"],
        [("シーク", "とても はやく ダッシュも すぐ もどるが もろく こうげきは よわめ。 ゆみで はじまる。"),
         ("マリオ", "たてを かまえて あるく。 まもりと ばくはつが つよいが うんは さがる。 バクダンで はじまる。"),
         ("スタルキッド", "うんが よく こうかが ながく つづくが もろい。 まほうの こなで はじまる。"),
         ("シモン ベルモンド", "バンパイアハンター。 ハート7こで こうげきも つよいが おそい。 ブーメランで はじまる。"),
         ("ビビ", "くろまどうし。 じゅもんが ひろく はやいが ハートは 4こ だけ。 ファイアロッドで はじまる。"),
         ("ガノンドロフ", "まおう。 ハート10こ まもりと すごい こうげき。 でも おそい。 けんで はじまる。")],
        ["ラッキーで とおくの アイテムも ひきよせるが ダメージが ひくい。 まほうの こなで はじまる。",
         "てきの うえに せきぞうを おく。 4ほうこうに ビームが でる。",
         "せきぞう 3こが 8ほうこうに はじける。"],
    ),
}
